use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use tokio::io;
use tokio_util::sync::CancellationToken;
use url::Url;

use funnel_core::protocol::frame;
use funnel_core::protocol::handshake::{
    AccessControl, Handshake, HandshakeResult, TunnelSpec, TunnelStatus, TunnelType,
};
use funnel_core::protocol::request::{DataHeader, HttpRequest};
use funnel_core::protocol::{PROTOCOL_VERSION, QUIC_ALPN};
use funnel_core::relay;
use funnel_core::tunnel::id::TunnelId;

use super::display::{RequestResult, TunnelDisplay};
use super::forwarder::{ForwardResult, ForwardUpgradeResult, Forwarder};

/// handshake was rejected with an error the server will never change its
/// mind about. reconnecting will produce the same rejection.
#[derive(Debug)]
pub struct PermanentError {
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for PermanentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for PermanentError {}

/// errors from connect() that the runner can inspect to decide whether to retry.
#[derive(Debug)]
pub enum ConnectError {
    Permanent(PermanentError),
    Transient(anyhow::Error),
}

impl std::fmt::Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Permanent(e) => write!(f, "{e}"),
            Self::Transient(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ConnectError {}

impl From<PermanentError> for ConnectError {
    fn from(e: PermanentError) -> Self {
        Self::Permanent(e)
    }
}

/// tunnel registration result returned from connect().
pub struct ConnectResult {
    pub conn: quinn::Connection,
    /// server-allocated remote port for stream tunnels.
    pub remote_port: Option<u16>,
}

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_CONCURRENT_REQUESTS: usize = 128;

/// configuration for a single tunnel connection.
pub struct TunnelOptions {
    pub tunnel_id: TunnelId,
    pub local_addr: String,
    pub tunnel_type: TunnelType,
    pub token: Option<String>,
    pub quic_port: u16,
    pub insecure: bool,
    pub team: Option<String>,
    pub remote_port: Option<u16>,
    pub access: Option<AccessControl>,
}

pub struct TunnelClient {
    pub tunnel_id: TunnelId,
    local_addr: String,
    tunnel_type: TunnelType,
    token: Option<String>,
    quic_addr: SocketAddr,
    host: String,
    endpoint: quinn::Endpoint,
    team: Option<String>,
    remote_port: Option<u16>,
    access: Option<AccessControl>,
}

impl TunnelClient {
    pub fn new(server_url: &str, opts: TunnelOptions) -> anyhow::Result<Self> {
        let url = Url::parse(server_url)?;
        let host = url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("no host in server url"))?
            .to_string();

        let quic_addr = resolve_quic_addr(&host, opts.quic_port)?;
        let skip_verify = opts.insecure || is_loopback(&host, &quic_addr);
        let client_config = build_client_config(skip_verify)?;

        // bind address must match the remote address family
        let bind_addr: SocketAddr = if quic_addr.is_ipv6() {
            SocketAddr::from((std::net::Ipv6Addr::UNSPECIFIED, 0))
        } else {
            SocketAddr::from((std::net::Ipv4Addr::UNSPECIFIED, 0))
        };

        let mut endpoint = quinn::Endpoint::client(bind_addr)?;
        endpoint.set_default_client_config(client_config);

        Ok(Self {
            tunnel_id: opts.tunnel_id,
            local_addr: opts.local_addr,
            tunnel_type: opts.tunnel_type,
            token: opts.token,
            quic_addr,
            host,
            endpoint,
            team: opts.team,
            remote_port: opts.remote_port,
            access: opts.access,
        })
    }

    /// connect to the server via quic and run the tunnel until it disconnects.
    pub async fn run(
        &self,
        cancel: CancellationToken,
        display: &Arc<TunnelDisplay>,
    ) -> Result<(), ConnectError> {
        let result = self.connect().await?;
        let conn = result.conn;

        if let Some(port) = result.remote_port {
            display.println(&format!("  remote port {port} (allocated)"));
        }
        let forwarder = Arc::new(Forwarder::new(self.local_addr.clone()));
        let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_REQUESTS));

        display.set_message("waiting for requests...");

        loop {
            tokio::select! {
                bi = conn.accept_bi() => {
                    let (send, recv) = match bi {
                        Ok(pair) => pair,
                        Err(e) => {
                            tracing::debug!(error = %e, "connection closed");
                            break;
                        }
                    };

                    let fwd = Arc::clone(&forwarder);
                    let sem = Arc::clone(&semaphore);
                    let cancel = cancel.clone();
                    let display = Arc::clone(display);

                    tokio::spawn(async move {
                        let _permit = tokio::select! {
                            permit = sem.acquire() => match permit {
                                Ok(p) => p,
                                Err(_) => return,
                            },
                            () = cancel.cancelled() => return,
                        };

                        match handle_stream(send, recv, &fwd).await {
                            Ok(result) => display.log_request(&result),
                            Err(e) => display.println(&format!("stream error: {e}")),
                        }
                    });
                }
                () = cancel.cancelled() => break,
            }
        }

        conn.close(quinn::VarInt::from_u32(0), b"client shutdown");
        Ok(())
    }

    async fn connect(&self) -> Result<ConnectResult, ConnectError> {
        tracing::debug!(addr = %self.quic_addr, host = %self.host, "connecting via quic");

        let connecting = self
            .endpoint
            .connect(self.quic_addr, &self.host)
            .map_err(|e| ConnectError::Transient(e.into()))?;

        let conn = tokio::time::timeout(HANDSHAKE_TIMEOUT, connecting)
            .await
            .map_err(|_| {
                ConnectError::Transient(anyhow::anyhow!(
                    "quic handshake timed out after {HANDSHAKE_TIMEOUT:?}"
                ))
            })
            .and_then(|r| r.map_err(|e| ConnectError::Transient(e.into())))?;

        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| ConnectError::Transient(e.into()))?;

        let handshake = Handshake {
            version: PROTOCOL_VERSION,
            token: self.token.clone(),
            tunnels: vec![TunnelSpec {
                id: self.tunnel_id.clone(),
                tunnel_type: self.tunnel_type.clone(),
                team: self.team.clone(),
                local_port: None,
                routing: None,
                remote_port: self.remote_port,
                access: self.access.clone(),
            }],
        };

        frame::write_meta(&mut send, &handshake)
            .await
            .map_err(|e| ConnectError::Transient(e.into()))?;

        let result: HandshakeResult = frame::read_meta(&mut recv)
            .await
            .map_err(|e| ConnectError::Transient(e.into()))?;

        let tunnel_result = result
            .tunnels
            .iter()
            .find(|t| t.id == self.tunnel_id)
            .ok_or_else(|| {
                ConnectError::Transient(anyhow::anyhow!(
                    "server did not return result for tunnel {}",
                    self.tunnel_id
                ))
            })?;

        if tunnel_result.status == TunnelStatus::Error {
            let code = tunnel_result
                .error_code
                .map_or_else(|| "unknown".to_string(), |c| c.as_str().to_string());
            let msg = tunnel_result
                .error_message
                .clone()
                .unwrap_or_else(|| "unknown error".to_string());

            return Err(ConnectError::Permanent(PermanentError {
                code,
                message: msg,
            }));
        }

        let remote_port = tunnel_result.remote_port;

        tracing::info!(tunnel_id = %self.tunnel_id, "quic tunnel connected");

        // keep the control stream alive in the background; dropping send
        // would finish the stream, which the server interprets as disconnect
        tokio::spawn(async move {
            let _send = send;
            let mut buf = [0u8; 1];
            let _ = recv.read(&mut buf).await;
        });

        Ok(ConnectResult { conn, remote_port })
    }
}

async fn handle_stream(
    send: quinn::SendStream,
    mut recv: quinn::RecvStream,
    forwarder: &Forwarder,
) -> anyhow::Result<RequestResult> {
    let start = Instant::now();
    let header: DataHeader = frame::read_meta(&mut recv).await?;

    match header {
        DataHeader::Http(meta) => handle_http_stream(send, recv, forwarder, meta, start).await,
        DataHeader::Stream(header) => handle_tcp_stream(send, recv, forwarder, header, start).await,
    }
}

async fn handle_http_stream(
    mut send: quinn::SendStream,
    mut recv: quinn::RecvStream,
    forwarder: &Forwarder,
    meta: HttpRequest,
    start: Instant,
) -> anyhow::Result<RequestResult> {
    let method = meta.method.clone();
    let path = meta.path.clone();

    if meta.upgrade {
        return handle_upgrade_stream(send, recv, forwarder, &meta, &method, &path, start).await;
    }

    let body = recv.read_to_end(64 * 1024 * 1024).await?;
    let body = Bytes::from(body);

    match forwarder.forward(meta, body).await {
        ForwardResult::Success {
            meta: resp_meta,
            body: incoming,
            conn,
        } => {
            frame::write_meta(&mut send, &resp_meta).await?;
            stream_body_to_quic(incoming, &mut send).await?;
            send.finish()?;
            forwarder.release(conn);
            Ok(RequestResult {
                method,
                path,
                status: resp_meta.status,
                duration: start.elapsed(),
            })
        }
        ForwardResult::LocalError {
            meta: resp_meta,
            body: resp_body,
        } => {
            frame::write_meta(&mut send, &resp_meta).await?;
            send.write_all(&resp_body).await?;
            send.finish()?;
            Ok(RequestResult {
                method,
                path,
                status: resp_meta.status,
                duration: start.elapsed(),
            })
        }
    }
}

async fn handle_tcp_stream(
    mut send: quinn::SendStream,
    mut recv: quinn::RecvStream,
    forwarder: &Forwarder,
    header: funnel_core::protocol::request::StreamHeader,
    start: Instant,
) -> anyhow::Result<RequestResult> {
    let local_addr = forwarder.local_addr();
    let tcp_stream = match tokio::net::TcpStream::connect(local_addr).await {
        Ok(s) => s,
        Err(e) => {
            tracing::debug!(error = %e, addr = %local_addr, "failed to connect to local service");
            send.reset(quinn::VarInt::from_u32(0x05))?;
            anyhow::bail!("local service unreachable: {e}");
        }
    };

    let (mut tcp_read, mut tcp_write) = io::split(tcp_stream);

    let _ =
        relay::copy_bidirectional_split(&mut recv, &mut send, &mut tcp_read, &mut tcp_write).await;

    let _ = send.finish();

    Ok(RequestResult {
        method: "TCP".to_string(),
        path: header.remote_addr,
        status: 0,
        duration: start.elapsed(),
    })
}

async fn stream_body_to_quic(
    mut body: Incoming,
    send: &mut quinn::SendStream,
) -> anyhow::Result<()> {
    while let Some(frame) = body.frame().await {
        let frame = frame?;
        if let Some(data) = frame.data_ref() {
            send.write_all(data).await?;
        }
    }
    Ok(())
}

async fn handle_upgrade_stream(
    mut send: quinn::SendStream,
    mut recv: quinn::RecvStream,
    forwarder: &Forwarder,
    meta: &HttpRequest,
    method: &str,
    path: &str,
    start: Instant,
) -> anyhow::Result<RequestResult> {
    let result = forwarder.forward_upgrade(meta).await?;

    match result {
        ForwardUpgradeResult::Upgraded(upgrade) => {
            frame::write_meta(&mut send, &upgrade.meta).await?;

            let status = upgrade.meta.status;

            let upgraded_io = TokioIo::new(upgrade.upgraded);
            let (mut local_read, mut local_write) = io::split(upgraded_io);

            let quic_to_local = io::copy(&mut recv, &mut local_write);
            let local_to_quic = io::copy(&mut local_read, &mut send);

            tokio::select! {
                r = quic_to_local => {
                    if let Err(e) = r {
                        tracing::debug!(error = %e, "quic to local copy ended");
                    }
                }
                r = local_to_quic => {
                    if let Err(e) = r {
                        tracing::debug!(error = %e, "local to quic copy ended");
                    }
                }
            }

            Ok(RequestResult {
                method: method.to_string(),
                path: path.to_string(),
                status,
                duration: start.elapsed(),
            })
        }
        ForwardUpgradeResult::Rejected(resp_meta, resp_body) => {
            frame::write_meta(&mut send, &resp_meta).await?;
            send.write_all(&resp_body).await?;
            send.finish()?;

            Ok(RequestResult {
                method: method.to_string(),
                path: path.to_string(),
                status: resp_meta.status,
                duration: start.elapsed(),
            })
        }
    }
}

fn is_loopback(host: &str, addr: &SocketAddr) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1") || addr.ip().is_loopback()
}

fn resolve_quic_addr(host: &str, port: u16) -> anyhow::Result<SocketAddr> {
    let addr_str = format!("{host}:{port}");
    addr_str.parse().or_else(|_| {
        use std::net::ToSocketAddrs;
        addr_str
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow::anyhow!("failed to resolve host: {host}"))
    })
}

fn build_client_config(insecure: bool) -> anyhow::Result<quinn::ClientConfig> {
    let mut crypto = if insecure {
        rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SkipVerification))
            .with_no_client_auth()
    } else {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth()
    };

    crypto.alpn_protocols = vec![QUIC_ALPN.to_vec()];

    let quic_config = quinn::crypto::rustls::QuicClientConfig::try_from(crypto)?;
    Ok(quinn::ClientConfig::new(Arc::new(quic_config)))
}

/// certificate verifier that accepts any certificate (for dev/insecure mode)
#[derive(Debug)]
struct SkipVerification;

impl rustls::client::danger::ServerCertVerifier for SkipVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

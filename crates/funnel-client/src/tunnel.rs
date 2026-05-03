use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio_util::io::ReaderStream;
use tokio_util::sync::CancellationToken;
use url::Url;

use funnel_core::protocol::frame;
use funnel_core::protocol::handshake::{Handshake, HandshakeResponse};
use funnel_core::protocol::request::RequestMeta;
use funnel_core::protocol::QUIC_ALPN;
use funnel_core::tunnel::id::TunnelId;

use crate::forwarder::Forwarder;

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_CONCURRENT_REQUESTS: usize = 128;

pub struct TunnelClient {
    pub tunnel_id: TunnelId,
    local_addr: String,
    token: Option<String>,
    quic_addr: SocketAddr,
    host: String,
    endpoint: quinn::Endpoint,
}

impl TunnelClient {
    pub fn new(
        tunnel_id: TunnelId,
        server_url: String,
        local_addr: String,
        token: Option<String>,
        quic_port: u16,
        insecure: bool,
    ) -> anyhow::Result<Self> {
        let url = Url::parse(&server_url)?;
        let host = url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("no host in server url"))?
            .to_string();

        let quic_addr = resolve_quic_addr(&host, quic_port)?;
        let client_config = build_client_config(insecure)?;

        // bind address must match the remote address family
        let bind_addr: SocketAddr = if quic_addr.is_ipv6() {
            SocketAddr::from((std::net::Ipv6Addr::UNSPECIFIED, 0))
        } else {
            SocketAddr::from((std::net::Ipv4Addr::UNSPECIFIED, 0))
        };

        let mut endpoint = quinn::Endpoint::client(bind_addr)?;
        endpoint.set_default_client_config(client_config);

        Ok(Self {
            tunnel_id,
            local_addr,
            token,
            quic_addr,
            host,
            endpoint,
        })
    }

    /// connect to the server via quic and run the tunnel until it disconnects.
    pub async fn run(&self, cancel: CancellationToken) -> anyhow::Result<()> {
        let conn = self.connect().await?;
        let forwarder = Arc::new(Forwarder::new(self.local_addr.clone()));
        let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_REQUESTS));

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

                    tokio::spawn(async move {
                        let _permit = tokio::select! {
                            permit = sem.acquire() => match permit {
                                Ok(p) => p,
                                Err(_) => return,
                            },
                            _ = cancel.cancelled() => return,
                        };

                        if let Err(e) = handle_stream(send, recv, &fwd).await {
                            tracing::debug!(error = %e, "stream handler error");
                        }
                    });
                }
                _ = cancel.cancelled() => break,
            }
        }

        conn.close(quinn::VarInt::from_u32(0), b"client shutdown");
        Ok(())
    }

    async fn connect(&self) -> anyhow::Result<quinn::Connection> {
        tracing::debug!(addr = %self.quic_addr, host = %self.host, "connecting via quic");

        let connecting = self.endpoint.connect(self.quic_addr, &self.host)?;

        let conn = tokio::time::timeout(HANDSHAKE_TIMEOUT, connecting)
            .await
            .map_err(|_| anyhow::anyhow!("connection timed out"))
            .and_then(|r| r.map_err(Into::into))?;

        let (mut send, mut recv) = conn.open_bi().await?;

        let handshake = Handshake {
            tunnel_id: self.tunnel_id.clone(),
            token: self.token.clone(),
        };

        frame::write_meta(&mut send, &handshake).await?;

        let resp: HandshakeResponse = frame::read_meta(&mut recv).await?;
        resp.into_result()?;

        tracing::info!(tunnel_id = %self.tunnel_id, "quic tunnel connected");

        // keep the control stream alive in the background; dropping send
        // would finish the stream, which the server interprets as disconnect
        tokio::spawn(async move {
            let _send = send;
            let mut buf = [0u8; 1];
            let _ = recv.read(&mut buf).await;
        });

        Ok(conn)
    }
}

async fn handle_stream(
    mut send: quinn::SendStream,
    mut recv: quinn::RecvStream,
    forwarder: &Forwarder,
) -> anyhow::Result<()> {
    let meta: RequestMeta = frame::read_meta(&mut recv).await?;

    tracing::debug!(
        method = %meta.method,
        path = %meta.path,
        "received request"
    );

    // stream body from quic directly to local service (no buffer cap)
    let body_stream = ReaderStream::new(recv);
    let body = reqwest::Body::wrap_stream(body_stream);

    let (resp_meta, resp_body) = forwarder.forward(meta, body).await;

    frame::write_meta(&mut send, &resp_meta).await?;
    send.write_all(&resp_body).await?;
    send.finish()?;

    Ok(())
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

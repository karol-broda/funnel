use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use funnel_core::protocol::QUIC_ALPN;

use crate::app::AppState;

const KEEP_ALIVE: Duration = Duration::from_secs(15);
const IDLE_TIMEOUT: Duration = Duration::from_mins(1);

pub fn spawn_listener(
    host: &str,
    port: u16,
    state: Arc<AppState>,
) -> anyhow::Result<tokio::task::JoinHandle<()>> {
    let server_config = build_self_signed_config()?;
    let endpoint = build_endpoint(server_config, host, port)?;

    Ok(tokio::spawn(async move {
        if let Err(e) = super::listener::run(endpoint, state).await {
            tracing::error!(error = %e, "quic listener failed");
        }
    }))
}

fn build_self_signed_config() -> anyhow::Result<quinn::ServerConfig> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
    let cert_der = cert.cert.der().clone();
    let key_der = rustls::pki_types::PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der());

    let mut server_crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![cert_der],
            rustls::pki_types::PrivateKeyDer::Pkcs8(key_der),
        )?;

    server_crypto.alpn_protocols = vec![QUIC_ALPN.to_vec()];

    let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)?,
    ));

    let mut transport = quinn::TransportConfig::default();
    transport.keep_alive_interval(Some(KEEP_ALIVE));
    transport.max_idle_timeout(Some(quinn::IdleTimeout::try_from(IDLE_TIMEOUT)?));

    server_config.transport_config(Arc::new(transport));

    Ok(server_config)
}

/// build a quic endpoint, using a dual stack ipv6 socket when the host is a
/// wildcard address so that both ipv4 and ipv6 clients can connect.
fn build_endpoint(
    config: quinn::ServerConfig,
    host: &str,
    port: u16,
) -> anyhow::Result<quinn::Endpoint> {
    let addr: SocketAddr = format!("{host}:{port}").parse()?;

    // when binding to 0.0.0.0, also bind the ipv6 wildcard so dual stack works
    if addr == SocketAddr::from(([0, 0, 0, 0], port)) {
        let v6_addr: SocketAddr = format!("[::]:{port}").parse()?;
        let socket = std::net::UdpSocket::bind(v6_addr)?;
        let runtime = quinn::default_runtime()
            .ok_or_else(|| anyhow::anyhow!("no async runtime available for quic endpoint"))?;
        Ok(quinn::Endpoint::new(
            quinn::EndpointConfig::default(),
            Some(config),
            socket,
            runtime,
        )?)
    } else {
        Ok(quinn::Endpoint::server(config, addr)?)
    }
}

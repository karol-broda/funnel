use std::net::SocketAddr;
use std::ops::RangeInclusive;
use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};

use dashmap::DashMap;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use funnel_core::protocol::request::StreamHeader;
use funnel_core::relay;
use funnel_core::tunnel::id::TunnelId;

use crate::tunnel::connection::ActiveTunnel;

const DEFAULT_PORT_MIN: u16 = 10000;
const DEFAULT_PORT_MAX: u16 = 60000;

pub struct StreamListenerManager {
    port_min: u16,
    port_max: u16,
    next_port: AtomicU16,
    listeners: DashMap<TunnelId, ListenerHandle>,
}

struct ListenerHandle {
    cancel: CancellationToken,
}

impl StreamListenerManager {
    pub fn new(port_min: u16, port_max: u16) -> Self {
        let port_min = if port_min == 0 {
            DEFAULT_PORT_MIN
        } else {
            port_min
        };
        let port_max = if port_max == 0 {
            DEFAULT_PORT_MAX
        } else {
            port_max
        };
        Self {
            port_min,
            port_max,
            next_port: AtomicU16::new(port_min),
            listeners: DashMap::new(),
        }
    }

    const fn port_range(&self) -> RangeInclusive<u16> {
        self.port_min..=self.port_max
    }

    /// bind a TCP listener on the given or next available port.
    /// returns the listener and the allocated port.
    /// the caller should then call `run` to start accepting connections.
    pub async fn bind(
        &self,
        requested_port: Option<u16>,
        host: &str,
    ) -> Result<(TcpListener, u16), StreamListenerError> {
        match requested_port {
            Some(p) if p != 0 => {
                if !self.port_range().contains(&p) {
                    return Err(StreamListenerError::PortOutOfRange(p));
                }
                let listener = try_bind(host, p).await?;
                Ok((listener, p))
            }
            _ => self.bind_next_available(host).await,
        }
    }

    /// start accepting TCP connections on the given listener and relay
    /// them through the tunnel's QUIC connection.
    pub fn run(
        &self,
        tunnel_id: TunnelId,
        tunnel: Arc<ActiveTunnel>,
        listener: TcpListener,
        port: u16,
    ) {
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let tunnel_id_clone = tunnel_id.clone();
        tokio::spawn(async move {
            run_stream_listener(listener, tunnel, tunnel_id_clone, port, cancel_clone).await;
        });

        self.listeners.insert(tunnel_id, ListenerHandle { cancel });
    }

    /// stop the TCP listener for a tunnel and free its port.
    pub fn stop(&self, tunnel_id: &TunnelId) {
        if let Some((_, handle)) = self.listeners.remove(tunnel_id) {
            handle.cancel.cancel();
        }
    }

    /// try binding ports starting from the atomic counter.
    /// the OS bind call is the real mutual exclusion, no check-then-act.
    async fn bind_next_available(
        &self,
        host: &str,
    ) -> Result<(TcpListener, u16), StreamListenerError> {
        let range_size = (self.port_max - self.port_min + 1) as u32;
        for _ in 0..range_size {
            let port = self.next_port.fetch_add(1, Ordering::Relaxed);

            // wrap around when we exceed the range
            if port > self.port_max {
                self.next_port.store(self.port_min, Ordering::Relaxed);
                continue;
            }

            match try_bind(host, port).await {
                Ok(listener) => return Ok((listener, port)),
                Err(StreamListenerError::PortInUse(_)) => {}
                Err(e) => return Err(e),
            }
        }
        Err(StreamListenerError::NoPortsAvailable)
    }
}

async fn try_bind(host: &str, port: u16) -> Result<TcpListener, StreamListenerError> {
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .map_err(|_| StreamListenerError::BindFailed(port))?;

    TcpListener::bind(addr)
        .await
        .map_err(|_| StreamListenerError::PortInUse(port))
}

async fn run_stream_listener(
    listener: TcpListener,
    tunnel: Arc<ActiveTunnel>,
    tunnel_id: TunnelId,
    server_port: u16,
    cancel: CancellationToken,
) {
    loop {
        let (tcp_stream, remote_addr) = tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok(pair) => pair,
                    Err(e) => {
                        tracing::debug!(error = %e, tunnel_id = %tunnel_id, "tcp accept error");
                        continue;
                    }
                }
            }
            () = cancel.cancelled() => break,
        };

        let tunnel = Arc::clone(&tunnel);
        let tunnel_id = tunnel_id.clone();

        tokio::spawn(async move {
            if let Err(e) =
                relay_stream_connection(tcp_stream, &tunnel, &tunnel_id, remote_addr, server_port)
                    .await
            {
                tracing::debug!(
                    error = %e,
                    tunnel_id = %tunnel_id,
                    remote_addr = %remote_addr,
                    "stream relay error"
                );
            }
        });
    }
}

async fn relay_stream_connection(
    tcp_stream: tokio::net::TcpStream,
    tunnel: &ActiveTunnel,
    tunnel_id: &TunnelId,
    remote_addr: SocketAddr,
    server_port: u16,
) -> anyhow::Result<()> {
    let header = StreamHeader {
        tunnel_id: tunnel_id.clone(),
        remote_addr: remote_addr.to_string(),
        server_port,
        sni: None,
    };

    let (mut quic_send, mut quic_recv) = tunnel.send_stream_request(header).await?;

    let (mut tcp_read, mut tcp_write) = tokio::io::split(tcp_stream);

    let stats = relay::copy_bidirectional_split(
        &mut tcp_read,
        &mut tcp_write,
        &mut quic_recv,
        &mut quic_send,
    )
    .await;

    let _ = quic_send.finish();

    if let Ok(ref stats) = stats {
        tunnel.record_stream_bytes(stats.a_to_b, stats.b_to_a);
    }

    stats?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum StreamListenerError {
    #[error("port {0} is already in use")]
    PortInUse(u16),

    #[error("port {0} is outside the allowed range")]
    PortOutOfRange(u16),

    #[error("failed to bind port {0}")]
    BindFailed(u16),

    #[error("no ports available in the configured range")]
    NoPortsAvailable,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn default_port_range() {
        let mgr = StreamListenerManager::new(0, 0);
        assert_eq!(mgr.port_min, DEFAULT_PORT_MIN);
        assert_eq!(mgr.port_max, DEFAULT_PORT_MAX);
    }

    #[test]
    fn custom_port_range() {
        let mgr = StreamListenerManager::new(20000, 30000);
        assert_eq!(mgr.port_min, 20000);
        assert_eq!(mgr.port_max, 30000);
    }

    #[tokio::test]
    async fn bind_specific_port() {
        let mgr = StreamListenerManager::new(30000, 30010);
        let (listener, port) = mgr.bind(Some(30005), "127.0.0.1").await.unwrap();
        assert_eq!(port, 30005);
        assert_eq!(listener.local_addr().unwrap().port(), 30005);
    }

    #[tokio::test]
    async fn bind_auto_allocates() {
        let mgr = StreamListenerManager::new(30020, 30030);
        let (_, port) = mgr.bind(None, "127.0.0.1").await.unwrap();
        assert!((30020..=30030).contains(&port));
    }

    #[tokio::test]
    async fn bind_zero_means_auto() {
        let mgr = StreamListenerManager::new(30040, 30050);
        let (_, port) = mgr.bind(Some(0), "127.0.0.1").await.unwrap();
        assert!((30040..=30050).contains(&port));
    }

    #[tokio::test]
    async fn bind_rejects_port_out_of_range() {
        let mgr = StreamListenerManager::new(30060, 30070);
        let result = mgr.bind(Some(9999), "127.0.0.1").await;
        assert!(matches!(
            result,
            Err(StreamListenerError::PortOutOfRange(9999))
        ));
    }

    #[tokio::test]
    async fn bind_rejects_duplicate_port() {
        let mgr = StreamListenerManager::new(30080, 30090);
        let (_listener, port) = mgr.bind(Some(30085), "127.0.0.1").await.unwrap();
        // try to bind the same port again while the first listener is alive
        let result = mgr.bind(Some(port), "127.0.0.1").await;
        assert!(matches!(result, Err(StreamListenerError::PortInUse(_))));
    }

    #[tokio::test]
    async fn bind_exhaustion_returns_error() {
        // range of 1 port, bind it, then auto-allocate should fail
        let mgr = StreamListenerManager::new(30095, 30095);
        let _first = mgr.bind(Some(30095), "127.0.0.1").await.unwrap();
        let result = mgr.bind(None, "127.0.0.1").await;
        assert!(matches!(result, Err(StreamListenerError::NoPortsAvailable)));
    }

    #[tokio::test]
    async fn stop_frees_tunnel_entry() {
        let mgr = StreamListenerManager::new(30100, 30110);
        let id = TunnelId::new("test-stop").unwrap();
        let (listener, port) = mgr.bind(None, "127.0.0.1").await.unwrap();

        // we can't call run() without a real ActiveTunnel, but we can test stop
        let cancel = CancellationToken::new();
        mgr.listeners.insert(id.clone(), ListenerHandle { cancel });
        assert_eq!(mgr.listeners.len(), 1);

        mgr.stop(&id);
        assert_eq!(mgr.listeners.len(), 0);

        // stop on non-existent tunnel is a no-op
        mgr.stop(&id);
        drop((listener, port));
    }

    #[tokio::test]
    async fn stop_nonexistent_is_noop() {
        let mgr = StreamListenerManager::new(30120, 30130);
        let id = TunnelId::new("nonexistent").unwrap();
        mgr.stop(&id); // should not panic
    }
}

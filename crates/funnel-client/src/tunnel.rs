use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use url::Url;

use funnel_core::protocol::TunnelMessage;
use funnel_core::tunnel::TunnelId;

use crate::forwarder::Forwarder;

const CHANNEL_BUFFER: usize = 128;
const PING_INTERVAL: Duration = Duration::from_secs(30);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_CONCURRENT_REQUESTS: usize = 128;

pub struct TunnelClient {
    pub tunnel_id: TunnelId,
    server_url: String,
    local_addr: String,
    token: Option<String>,
}

impl TunnelClient {
    pub fn new(
        tunnel_id: TunnelId,
        server_url: String,
        local_addr: String,
        token: Option<String>,
    ) -> Self {
        Self {
            tunnel_id,
            server_url,
            local_addr,
            token,
        }
    }

    /// connect to the server and run the tunnel until it disconnects.
    pub async fn run(&self, cancel: CancellationToken) -> anyhow::Result<()> {
        let ws = self.connect().await?;
        let (ws_sink, ws_stream) = ws.split();

        let (outgoing_tx, outgoing_rx) = mpsc::channel::<TunnelMessage>(CHANNEL_BUFFER);
        let forwarder = Arc::new(Forwarder::new(self.local_addr.clone()));
        let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_REQUESTS));

        let read_cancel = cancel.clone();
        let read_tx = outgoing_tx.clone();
        let read_forwarder = Arc::clone(&forwarder);
        let read_semaphore = Arc::clone(&semaphore);
        let read_tunnel_id = self.tunnel_id.clone();
        let read_handle = tokio::spawn(async move {
            read_loop(
                ws_stream,
                read_tx,
                read_forwarder,
                read_semaphore,
                read_tunnel_id,
                read_cancel,
            )
            .await;
        });

        let write_cancel = cancel.clone();
        let write_handle = tokio::spawn(async move {
            write_loop(ws_sink, outgoing_rx, write_cancel).await;
        });

        let ping_cancel = cancel.clone();
        let ping_tx = outgoing_tx.clone();
        let ping_handle = tokio::spawn(async move {
            heartbeat_loop(ping_tx, ping_cancel).await;
        });

        tokio::select! {
            _ = read_handle => {},
            _ = write_handle => {},
            _ = ping_handle => {},
            _ = cancel.cancelled() => {},
        }

        Ok(())
    }

    async fn connect(
        &self,
    ) -> anyhow::Result<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    > {
        let mut url = Url::parse(&self.server_url)?;

        let ws_scheme = if url.scheme() == "https" { "wss" } else { "ws" };
        url.set_scheme(ws_scheme)
            .map_err(|_| anyhow::anyhow!("failed to set websocket scheme on url: {url}"))?;

        url.set_path("/ws");
        url.query_pairs_mut()
            .append_pair("id", self.tunnel_id.as_ref());

        use tokio_tungstenite::tungstenite::client::IntoClientRequest;

        let mut request = url.as_str().into_client_request()?;

        if let Some(token) = &self.token {
            request.headers_mut().insert(
                "Authorization",
                format!("Bearer {token}").parse()?,
            );
        }

        tracing::debug!(url = %url, "connecting to server");

        let (ws, _response) =
            tokio::time::timeout(HANDSHAKE_TIMEOUT, tokio_tungstenite::connect_async(request))
                .await
                .map_err(|_| anyhow::anyhow!("connection timed out"))??;

        tracing::info!(tunnel_id = %self.tunnel_id, "websocket connected");
        Ok(ws)
    }
}

async fn read_loop(
    mut stream: futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    outgoing_tx: mpsc::Sender<TunnelMessage>,
    forwarder: Arc<Forwarder>,
    semaphore: Arc<tokio::sync::Semaphore>,
    tunnel_id: TunnelId,
    cancel: CancellationToken,
) {
    loop {
        let msg = tokio::select! {
            msg = stream.next() => msg,
            _ = cancel.cancelled() => break,
        };

        let Some(result) = msg else { break };

        let frame = match result {
            Ok(f) => f,
            Err(e) => {
                tracing::debug!(error = %e, "websocket read error");
                break;
            }
        };

        match frame {
            Message::Text(text) => {
                handle_text_message(
                    &text,
                    &outgoing_tx,
                    &forwarder,
                    &semaphore,
                    &tunnel_id,
                    &cancel,
                );
            }
            Message::Close(_) => break,
            Message::Ping(_) => {
                // tungstenite handles pong automatically
            }
            _ => {}
        }
    }

    tracing::debug!("read loop exiting");
    cancel.cancel();
}

fn handle_text_message(
    text: &str,
    outgoing_tx: &mpsc::Sender<TunnelMessage>,
    forwarder: &Arc<Forwarder>,
    semaphore: &Arc<tokio::sync::Semaphore>,
    tunnel_id: &TunnelId,
    cancel: &CancellationToken,
) {
    let msg: TunnelMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, "malformed message from server");
            return;
        }
    };

    match msg {
        TunnelMessage::Request {
            request_id,
            payload,
            ..
        } => {
            tracing::debug!(
                request_id = %request_id,
                method = %payload.method,
                path = %payload.path,
                "received request"
            );

            let tx = outgoing_tx.clone();
            let fwd = Arc::clone(forwarder);
            let sem = Arc::clone(semaphore);
            let cancel = cancel.clone();
            let tid = tunnel_id.clone();

            tokio::spawn(async move {
                let _permit = tokio::select! {
                    permit = sem.acquire() => match permit {
                        Ok(p) => p,
                        Err(_) => return,
                    },
                    _ = cancel.cancelled() => return,
                };

                let response = fwd.forward(payload).await;

                let msg = TunnelMessage::Response {
                    request_id,
                    payload: response,
                };

                if tx.send(msg).await.is_err() {
                    tracing::debug!(tunnel_id = %tid, "outgoing channel closed");
                }
            });
        }
        TunnelMessage::RequestCancel { request_id } => {
            tracing::debug!(request_id = %request_id, "request cancelled by server");
            // cancellation of individual in flight requests would require
            // tracking cancel tokens per request. for now we just log it;
            // the forwarded request will complete and its response will be
            // silently dropped by the server since the pending entry is gone.
        }
        TunnelMessage::Pong => {
            tracing::debug!("pong received");
        }
        other => {
            tracing::debug!(msg_type = ?other, "unhandled message type");
        }
    }
}

async fn write_loop(
    mut sink: futures::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    mut rx: mpsc::Receiver<TunnelMessage>,
    cancel: CancellationToken,
) {
    loop {
        let msg = tokio::select! {
            msg = rx.recv() => msg,
            _ = cancel.cancelled() => break,
        };

        let Some(msg) = msg else { break };

        let json = match serde_json::to_string(&msg) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!(error = %e, "failed to serialize message");
                continue;
            }
        };

        if sink.send(Message::text(json)).await.is_err() {
            break;
        }
    }

    tracing::debug!("write loop exiting");
    cancel.cancel();
}

async fn heartbeat_loop(tx: mpsc::Sender<TunnelMessage>, cancel: CancellationToken) {
    let mut interval = tokio::time::interval(PING_INTERVAL);
    interval.tick().await; // skip first immediate tick

    loop {
        tokio::select! {
            _ = interval.tick() => {},
            _ = cancel.cancelled() => break,
        }

        if tx.send(TunnelMessage::Ping).await.is_err() {
            break;
        }
    }

    tracing::debug!("heartbeat loop exiting");
}

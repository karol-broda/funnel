use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use funnel_core::protocol::{ResponsePayload, TunnelMessage};
use funnel_core::tunnel::TunnelId;

use super::stats::TunnelStats;

const CHANNEL_BUFFER: usize = 128;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub struct ActiveTunnel {
    id: TunnelId,
    outgoing_tx: mpsc::Sender<TunnelMessage>,
    pending_requests: DashMap<Uuid, oneshot::Sender<ResponsePayload>>,
    stats: TunnelStats,
    connected_at: tokio::time::Instant,
    cancel: CancellationToken,
}

impl ActiveTunnel {
    pub fn id(&self) -> &TunnelId {
        &self.id
    }

    pub fn stats(&self) -> super::stats::TunnelStatsSnapshot {
        self.stats.snapshot()
    }

    pub fn connected_at(&self) -> tokio::time::Instant {
        self.connected_at
    }

    pub fn is_alive(&self) -> bool {
        !self.cancel.is_cancelled()
    }

    /// wait until the tunnel connection has been closed.
    pub async fn cancelled(&self) {
        self.cancel.cancelled().await;
    }

    /// send a request through the tunnel and wait for the client's response.
    pub async fn send_request(
        &self,
        payload: funnel_core::protocol::RequestPayload,
    ) -> Result<ResponsePayload, SendError> {
        let request_id = Uuid::now_v7();
        let (tx, rx) = oneshot::channel();

        self.pending_requests.insert(request_id, tx);
        self.stats.inc_requests();

        let msg = TunnelMessage::Request {
            tunnel_id: self.id.clone(),
            request_id,
            payload,
        };

        if self.outgoing_tx.send(msg).await.is_err() {
            self.pending_requests.remove(&request_id);
            return Err(SendError::TunnelClosed);
        }

        let result = tokio::time::timeout(REQUEST_TIMEOUT, rx).await;

        // clean up regardless of outcome
        self.pending_requests.remove(&request_id);

        match result {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(SendError::TunnelClosed),
            Err(_) => {
                // send cancel so the client can stop working on this request
                let _ = self.outgoing_tx.send(TunnelMessage::RequestCancel { request_id }).await;
                Err(SendError::Timeout)
            }
        }
    }

    /// shut down the tunnel connection.
    pub fn close(&self) {
        self.cancel.cancel();
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SendError {
    #[error("tunnel connection closed")]
    TunnelClosed,
    #[error("request timed out")]
    Timeout,
}

/// spawn the read/write tasks for a websocket connection and return the active tunnel.
/// the returned tunnel is live immediately; the background tasks run until the
/// connection drops or `close()` is called.
pub fn spawn(id: TunnelId, ws: WebSocket) -> Arc<ActiveTunnel> {
    let (ws_sink, ws_stream) = ws.split();
    let (outgoing_tx, outgoing_rx) = mpsc::channel(CHANNEL_BUFFER);
    let cancel = CancellationToken::new();

    let tunnel = Arc::new(ActiveTunnel {
        id: id.clone(),
        outgoing_tx,
        pending_requests: DashMap::new(),
        stats: TunnelStats::new(),
        connected_at: tokio::time::Instant::now(),
        cancel: cancel.clone(),
    });

    let read_tunnel = Arc::clone(&tunnel);
    let read_cancel = cancel.clone();
    tokio::spawn(async move {
        read_loop(ws_stream, &read_tunnel, read_cancel).await;
    });

    let write_cancel = cancel.clone();
    tokio::spawn(async move {
        write_loop(ws_sink, outgoing_rx, write_cancel).await;
    });

    tunnel
}

async fn read_loop(
    mut stream: futures::stream::SplitStream<WebSocket>,
    tunnel: &ActiveTunnel,
    cancel: CancellationToken,
) {
    loop {
        let msg = tokio::select! {
            msg = stream.next() => msg,
            _ = cancel.cancelled() => break,
        };

        let Some(result) = msg else { break };

        let frame = match result {
            Ok(frame) => frame,
            Err(e) => {
                tracing::debug!(tunnel_id = %tunnel.id, error = %e, "websocket read error");
                break;
            }
        };

        match frame {
            Message::Text(text) => {
                tunnel.stats.add_bytes_in(text.len() as u64);
                handle_incoming_text(tunnel, &text);
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    tracing::debug!(tunnel_id = %tunnel.id, "read loop exiting");
    cancel.cancel();
}

fn handle_incoming_text(tunnel: &ActiveTunnel, text: &str) {
    let msg: TunnelMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(tunnel_id = %tunnel.id, error = %e, "malformed message from client");
            return;
        }
    };

    match msg {
        TunnelMessage::Response {
            request_id,
            payload,
        } => {
            if let Some((_, tx)) = tunnel.pending_requests.remove(&request_id) {
                let _ = tx.send(payload);
            }
        }
        TunnelMessage::Pong => {}
        other => {
            tracing::debug!(
                tunnel_id = %tunnel.id,
                msg_type = ?other,
                "unexpected message type from client"
            );
        }
    }
}

async fn write_loop(
    mut sink: futures::stream::SplitSink<WebSocket, Message>,
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
                tracing::error!(error = %e, "failed to serialize tunnel message");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_error_display() {
        assert_eq!(SendError::TunnelClosed.to_string(), "tunnel connection closed");
        assert_eq!(SendError::Timeout.to_string(), "request timed out");
    }
}

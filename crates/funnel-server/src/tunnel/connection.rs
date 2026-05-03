use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use http_body_util::BodyExt;
use tokio::io::{AsyncRead, ReadBuf};

use funnel_core::protocol::{self, FrameError, RequestMeta, ResponseMeta};
use funnel_core::tunnel::TunnelId;

use super::stats::TunnelStats;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub struct ActiveTunnel {
    id: TunnelId,
    conn: quinn::Connection,
    stats: Arc<TunnelStats>,
    connected_at: tokio::time::Instant,
}

impl ActiveTunnel {
    pub fn new(id: TunnelId, conn: quinn::Connection) -> Self {
        Self {
            id,
            conn,
            stats: Arc::new(TunnelStats::new()),
            connected_at: tokio::time::Instant::now(),
        }
    }

    pub fn id(&self) -> &TunnelId {
        &self.id
    }

    pub fn stats(&self) -> super::stats::TunnelStatsSnapshot {
        self.stats.snapshot()
    }

    pub fn connected_at(&self) -> tokio::time::Instant {
        self.connected_at
    }

    /// open a quic bidirectional stream, send the request, and return the
    /// response metadata along with a counted recv stream for body streaming.
    pub async fn send_request(
        &self,
        meta: RequestMeta,
        body: axum::body::Body,
    ) -> Result<(ResponseMeta, CountedRecvStream), SendError> {
        self.stats.inc_requests();

        let start = std::time::Instant::now();

        let result = tokio::time::timeout(REQUEST_TIMEOUT, async {
            let (mut send, mut recv) = self
                .conn
                .open_bi()
                .await
                .map_err(SendError::OpenStream)?;

            protocol::write_meta(&mut send, &meta)
                .await
                .map_err(SendError::SendMeta)?;

            let mut body = body;
            let mut bytes_sent: u64 = 0;
            while let Some(frame) = body.frame().await {
                let frame = frame.map_err(SendError::ReadBody)?;
                if let Ok(data) = frame.into_data() {
                    bytes_sent += data.len() as u64;
                    send.write_all(&data)
                        .await
                        .map_err(SendError::WriteBody)?;
                }
            }

            send.finish().map_err(SendError::FinishStream)?;

            self.stats.add_bytes_out(bytes_sent);
            metrics::counter!("funnel_bytes_out_total").increment(bytes_sent);
            metrics::histogram!("funnel_request_body_bytes").record(bytes_sent as f64);

            let resp_meta: ResponseMeta = protocol::read_meta(&mut recv)
                .await
                .map_err(SendError::ReadResponse)?;

            let counted_recv = CountedRecvStream::new(recv, Arc::clone(&self.stats));
            Ok::<_, SendError>((resp_meta, counted_recv))
        })
        .await;

        let duration = start.elapsed().as_secs_f64();
        metrics::histogram!("funnel_request_duration_seconds").record(duration);

        match result {
            Ok(Ok(response)) => {
                metrics::counter!("funnel_requests_total", "outcome" => "success").increment(1);
                Ok(response)
            }
            Ok(Err(e)) => {
                metrics::counter!("funnel_requests_total", "outcome" => e.outcome_label()).increment(1);
                Err(e)
            }
            Err(_) => {
                metrics::counter!("funnel_requests_total", "outcome" => "timeout").increment(1);
                Err(SendError::Timeout)
            }
        }
    }

    pub fn close(&self) {
        self.conn
            .close(quinn::VarInt::from_u32(0), b"tunnel closed");
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SendError {
    #[error("failed to open tunnel stream")]
    OpenStream(#[source] quinn::ConnectionError),

    #[error("failed to send request metadata")]
    SendMeta(#[source] FrameError),

    #[error("failed to read request body")]
    ReadBody(#[source] axum::Error),

    #[error("failed to write body to tunnel")]
    WriteBody(#[source] quinn::WriteError),

    #[error("failed to finish send stream")]
    FinishStream(#[source] quinn::ClosedStream),

    #[error("failed to read response metadata")]
    ReadResponse(#[source] FrameError),

    #[error("request timed out")]
    Timeout,
}

impl SendError {
    pub fn outcome_label(&self) -> &'static str {
        match self {
            Self::OpenStream(_) => "stream_open_failed",
            Self::SendMeta(_) => "send_meta_failed",
            Self::ReadBody(_) => "read_body_failed",
            Self::WriteBody(_) => "write_body_failed",
            Self::FinishStream(_) => "finish_stream_failed",
            Self::ReadResponse(_) => "read_response_failed",
            Self::Timeout => "timeout",
        }
    }

}

/// wraps a quinn::RecvStream and tracks bytes read for metrics and per tunnel stats.
/// on drop, records funnel_bytes_in_total and funnel_response_body_bytes.
pub struct CountedRecvStream {
    inner: quinn::RecvStream,
    bytes_read: u64,
    stats: Arc<TunnelStats>,
}

impl CountedRecvStream {
    fn new(inner: quinn::RecvStream, stats: Arc<TunnelStats>) -> Self {
        Self {
            inner,
            bytes_read: 0,
            stats,
        }
    }
}

impl AsyncRead for CountedRecvStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        let before = buf.filled().len();
        let result = Pin::new(&mut this.inner).poll_read(cx, buf);
        let after = buf.filled().len();
        this.bytes_read += (after - before) as u64;
        result
    }
}

impl Drop for CountedRecvStream {
    fn drop(&mut self) {
        self.stats.add_bytes_in(self.bytes_read);
        metrics::counter!("funnel_bytes_in_total").increment(self.bytes_read);
        metrics::histogram!("funnel_response_body_bytes").record(self.bytes_read as f64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_error_display() {
        assert_eq!(SendError::Timeout.to_string(), "request timed out");
    }

    #[test]
    fn send_error_outcome_labels() {
        assert_eq!(SendError::Timeout.outcome_label(), "timeout");
    }

}

use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use url::Url;

use funnel_core::tunnel::id::TunnelId;

use super::client::{ConnectError, TunnelClient};
use super::display::TunnelDisplay;

const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(30);

pub async fn run(client: &TunnelClient, shutdown: CancellationToken, display: &Arc<TunnelDisplay>) {
    let mut attempt: u32 = 0;

    loop {
        if shutdown.is_cancelled() {
            break;
        }

        display.set_message("connecting to server...");

        let run_cancel = shutdown.child_token();
        match client.run(run_cancel, display).await {
            Ok(()) => {
                attempt = 0;
                display.println("connection lost, reconnecting...");
            }
            Err(ConnectError::Permanent(e)) => {
                display.println(&format!("error: [{}] {}", e.code, e.message));
                break;
            }
            Err(ConnectError::Transient(e)) => {
                display.println(&format!("connection failed: {e}"));
            }
        }

        let delay = backoff_delay(attempt);
        attempt = attempt.saturating_add(1);

        display.set_message(&format!("reconnecting in {}s...", delay.as_secs()));

        tokio::select! {
            () = tokio::time::sleep(delay) => {},
            () = shutdown.cancelled() => break,
        }
    }
}

fn backoff_delay(attempt: u32) -> Duration {
    let secs = 2u64.saturating_pow(attempt);
    let delay = Duration::from_secs(secs).min(MAX_BACKOFF);
    delay.max(INITIAL_BACKOFF)
}

pub fn build_public_url(server_url: &str, tunnel_id: &TunnelId) -> Option<String> {
    let url = Url::parse(server_url).ok()?;
    let host = url.host_str()?;
    let scheme = if url.scheme() == "https" {
        "https"
    } else {
        "http"
    };

    match url.port() {
        Some(port) => Some(format!("{scheme}://{tunnel_id}.{host}:{port}")),
        None => Some(format!("{scheme}://{tunnel_id}.{host}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_increases_exponentially() {
        assert_eq!(backoff_delay(0), Duration::from_secs(1));
        assert_eq!(backoff_delay(1), Duration::from_secs(2));
        assert_eq!(backoff_delay(2), Duration::from_secs(4));
        assert_eq!(backoff_delay(3), Duration::from_secs(8));
    }

    #[test]
    fn backoff_caps_at_max() {
        assert_eq!(backoff_delay(10), MAX_BACKOFF);
        assert_eq!(backoff_delay(20), MAX_BACKOFF);
    }

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn public_url_http() -> TestResult {
        let id = TunnelId::new("my-tunnel")?;
        let url = build_public_url("http://tunnel.example.com", &id);
        assert_eq!(url.as_deref(), Some("http://my-tunnel.tunnel.example.com"));
        Ok(())
    }

    #[test]
    fn public_url_https() -> TestResult {
        let id = TunnelId::new("my-tunnel")?;
        let url = build_public_url("https://tunnel.example.com", &id);
        assert_eq!(url.as_deref(), Some("https://my-tunnel.tunnel.example.com"));
        Ok(())
    }

    #[test]
    fn public_url_with_port() -> TestResult {
        let id = TunnelId::new("abc")?;
        let url = build_public_url("http://localhost:8080", &id);
        assert_eq!(url.as_deref(), Some("http://abc.localhost:8080"));
        Ok(())
    }
}

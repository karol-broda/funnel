use std::sync::Arc;

use crate::app::AppState;

use super::handler;

pub async fn run(endpoint: quinn::Endpoint, state: Arc<AppState>) -> anyhow::Result<()> {
    tracing::info!(
        addr = %endpoint.local_addr()?,
        "quic endpoint listening"
    );

    while let Some(incoming) = endpoint.accept().await {
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            match incoming.await {
                Ok(conn) => {
                    if let Err(e) = handler::handle_connection(conn, state).await {
                        tracing::debug!(error = %e, "connection ended");
                    }
                }
                Err(e) => {
                    tracing::debug!(error = %e, "failed to accept connection");
                }
            }
        });
    }

    Ok(())
}

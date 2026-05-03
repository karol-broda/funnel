use serde::{Deserialize, Serialize};

use crate::tunnel::TunnelId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handshake {
    pub tunnel_id: TunnelId,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResponse {
    success: bool,
    error: Option<String>,
}

impl HandshakeResponse {
    pub fn ok() -> Self {
        Self {
            success: true,
            error: None,
        }
    }

    pub fn rejected(reason: impl Into<String>) -> Self {
        Self {
            success: false,
            error: Some(reason.into()),
        }
    }

    pub fn into_result(self) -> Result<(), HandshakeRejected> {
        if self.success {
            Ok(())
        } else {
            Err(HandshakeRejected {
                reason: self.error.unwrap_or_else(|| "unknown error".to_string()),
            })
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("handshake rejected: {reason}")]
pub struct HandshakeRejected {
    pub reason: String,
}

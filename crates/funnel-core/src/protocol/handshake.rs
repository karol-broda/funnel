use serde::{Deserialize, Serialize};

use crate::tunnel::id::TunnelId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handshake {
    pub tunnel_id: TunnelId,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum HandshakeResponse {
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "rejected")]
    Rejected { reason: String },
}

impl HandshakeResponse {
    pub fn ok() -> Self {
        Self::Ok
    }

    pub fn rejected(reason: impl Into<String>) -> Self {
        Self::Rejected {
            reason: reason.into(),
        }
    }

    pub fn into_result(self) -> Result<(), HandshakeRejected> {
        match self {
            Self::Ok => Ok(()),
            Self::Rejected { reason } => Err(HandshakeRejected { reason }),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("handshake rejected: {reason}")]
pub struct HandshakeRejected {
    pub reason: String,
}

pub mod frame;
pub mod handshake;
pub mod request;

pub use frame::{read_frame, read_meta, write_frame, write_meta, FrameError};
pub use handshake::{Handshake, HandshakeRejected, HandshakeResponse};
pub use request::{RequestMeta, ResponseMeta};

/// application level protocol name used for QUIC ALPN negotiation
pub const QUIC_ALPN: &[u8] = b"funnel/1";

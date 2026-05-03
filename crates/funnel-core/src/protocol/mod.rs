pub mod frame;
pub mod handshake;
pub mod request;

/// application level protocol name used for QUIC ALPN negotiation
pub const QUIC_ALPN: &[u8] = b"funnel/1";

pub mod error_codes;
pub mod frame;
pub mod handshake;
pub mod request;

pub const PROTOCOL_VERSION: u32 = 1;

pub const QUIC_ALPN: &[u8] = b"funnel";

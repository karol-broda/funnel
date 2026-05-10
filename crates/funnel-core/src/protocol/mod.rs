pub mod frame;
pub mod handshake;
pub mod request;

/// unified version number shared across protocol, schema, and API
pub const PROTOCOL_VERSION: u32 = 1;

/// application level protocol name used for QUIC ALPN negotiation
pub static QUIC_ALPN: std::sync::LazyLock<Vec<u8>> =
    std::sync::LazyLock::new(|| format!("funnel/{PROTOCOL_VERSION}").into_bytes());

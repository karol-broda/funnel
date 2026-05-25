use std::collections::HashMap;

use bytes::Bytes;
use serde::Serialize;

pub const BODY_PREVIEW_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct CapturedExchange {
    pub id: String,
    pub sequence: u64,
    pub source: ExchangeSource,
    pub timestamp_ms: u128,
    pub remote_addr: String,
    pub method: String,
    pub path: String,
    pub request_headers: HashMap<String, Vec<String>>,
    pub request_body: BodyPreview,
    pub status: u16,
    pub response_headers: HashMap<String, Vec<String>>,
    pub response_body: BodyPreview,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExchangeSource {
    Tunnel,
    Replay,
}

#[derive(Debug, Clone, Serialize)]
pub struct BodyPreview {
    pub text: String,
    pub bytes: usize,
    pub truncated: bool,
    pub binary: bool,
    #[serde(skip)]
    pub preview_bytes: Vec<u8>,
}

impl BodyPreview {
    pub fn empty() -> Self {
        Self {
            text: String::new(),
            bytes: 0,
            truncated: false,
            binary: false,
            preview_bytes: Vec::new(),
        }
    }
}

pub fn body_preview(bytes: &Bytes) -> BodyPreview {
    let truncated = bytes.len() > BODY_PREVIEW_BYTES;
    let slice_len = bytes.len().min(BODY_PREVIEW_BYTES);
    let slice = &bytes[..slice_len];
    let binary = !is_probably_text(slice);
    let text = if binary && !bytes.is_empty() {
        binary_label(bytes.len())
    } else {
        String::from_utf8_lossy(slice).to_string()
    };

    BodyPreview {
        text,
        bytes: bytes.len(),
        truncated,
        binary,
        preview_bytes: slice.to_vec(),
    }
}

pub fn append_preview(preview: &mut BodyPreview, chunk: &[u8]) {
    let remaining_bytes = BODY_PREVIEW_BYTES.saturating_sub(preview.preview_bytes.len());
    if remaining_bytes > 0 {
        preview
            .preview_bytes
            .extend_from_slice(&chunk[..chunk.len().min(remaining_bytes)]);
    }
    preview.bytes += chunk.len();
    if preview.binary {
        preview.text = binary_label(preview.bytes);
        return;
    }
    if !is_probably_text(chunk) {
        preview.binary = true;
        preview.text = binary_label(preview.bytes);
        preview.truncated = preview.preview_bytes.len() < preview.bytes;
        return;
    }
    if preview.text.len() >= BODY_PREVIEW_BYTES {
        preview.truncated = preview.truncated || !chunk.is_empty();
        return;
    }

    let remaining = BODY_PREVIEW_BYTES - preview.text.len();
    let take = chunk.len().min(remaining);
    preview
        .text
        .push_str(&String::from_utf8_lossy(&chunk[..take]));
    preview.truncated = preview.truncated || take < chunk.len();
}

fn binary_label(bytes: usize) -> String {
    format!("(binary body, {bytes} bytes)")
}

fn is_probably_text(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return true;
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let control_count = text
        .chars()
        .filter(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
        .count();
    control_count * 100 / text.chars().count().max(1) < 5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_stream_under_preview_limit_is_not_truncated() {
        let mut preview = BodyPreview::empty();
        append_preview(&mut preview, b"\0woff2-like-binary");

        assert!(preview.binary);
        assert_eq!(preview.bytes, 18);
        assert_eq!(preview.preview_bytes.len(), preview.bytes);
        assert!(!preview.truncated);
    }
}

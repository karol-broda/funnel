use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("frame too large: {size} bytes (max {max})")]
    TooLarge { size: u32, max: u32 },

    #[error("empty frame")]
    Empty,

    #[error("decode error: {0}")]
    Decode(#[from] rmp_serde::decode::Error),

    #[error("encode error: {0}")]
    Encode(#[from] rmp_serde::encode::Error),
}

pub async fn write_frame(
    writer: &mut (impl AsyncWrite + Unpin),
    data: &[u8],
) -> Result<(), FrameError> {
    let len = data.len() as u32;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(data).await?;
    Ok(())
}

pub async fn read_frame(
    reader: &mut (impl AsyncRead + Unpin),
    max_size: u32,
) -> Result<Vec<u8>, FrameError> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf);

    if len == 0 {
        return Err(FrameError::Empty);
    }

    if len > max_size {
        return Err(FrameError::TooLarge {
            size: len,
            max: max_size,
        });
    }

    let mut buf = vec![0u8; len as usize];
    reader.read_exact(&mut buf).await?;
    Ok(buf)
}

pub async fn write_meta(
    writer: &mut (impl AsyncWrite + Unpin),
    value: &impl Serialize,
) -> Result<(), FrameError> {
    let data = rmp_serde::to_vec_named(value)?;
    write_frame(writer, &data).await
}

const MAX_META_SIZE: u32 = 1024 * 1024;

pub async fn read_meta<T: for<'de> Deserialize<'de>>(
    reader: &mut (impl AsyncRead + Unpin),
) -> Result<T, FrameError> {
    let data = read_frame(reader, MAX_META_SIZE).await?;
    Ok(rmp_serde::from_slice(&data)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[tokio::test]
    async fn frame_roundtrip() -> TestResult {
        let data = b"hello world";
        let mut buf = Vec::new();
        write_frame(&mut buf, data).await?;

        let mut cursor = std::io::Cursor::new(buf);
        let result = read_frame(&mut cursor, 1024).await?;
        assert_eq!(result, data);
        Ok(())
    }

    #[tokio::test]
    async fn frame_too_large() -> TestResult {
        let data = vec![0u8; 100];
        let mut buf = Vec::new();
        write_frame(&mut buf, &data).await?;

        let mut cursor = std::io::Cursor::new(buf);
        let result = read_frame(&mut cursor, 50).await;
        assert!(matches!(
            result,
            Err(FrameError::TooLarge { size: 100, max: 50 })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn meta_roundtrip() -> TestResult {
        use std::collections::HashMap;

        let original: HashMap<String, String> =
            std::iter::once(("key".into(), "value".into())).collect();

        let mut buf = Vec::new();
        write_meta(&mut buf, &original).await?;

        let mut cursor = std::io::Cursor::new(buf);
        let decoded: HashMap<String, String> = read_meta(&mut cursor).await?;
        assert_eq!(decoded, original);
        Ok(())
    }
}

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const MAX_META_SIZE: u32 = 1_048_576; // 1mb

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("frame too large: {size} bytes (max {max})")]
    TooLarge { size: u32, max: u32 },

    #[error("encode error: {0}")]
    Encode(#[from] rmp_serde::encode::Error),

    #[error("decode error: {0}")]
    Decode(#[from] rmp_serde::decode::Error),
}

/// write a length prefixed frame: [4 bytes big endian u32 length][data]
pub async fn write_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    data: &[u8],
) -> Result<(), FrameError> {
    let len = data.len() as u32;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(data).await?;
    Ok(())
}

/// read a length prefixed frame, rejecting frames larger than max_size
pub async fn read_frame<R: AsyncRead + Unpin>(
    reader: &mut R,
    max_size: u32,
) -> Result<Vec<u8>, FrameError> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf);

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

/// serialize T as msgpack and write as a length prefixed frame
pub async fn write_meta<W: AsyncWrite + Unpin, T: serde::Serialize>(
    writer: &mut W,
    meta: &T,
) -> Result<(), FrameError> {
    let data = rmp_serde::to_vec_named(meta)?;
    write_frame(writer, &data).await
}

/// read a length prefixed frame and deserialize as msgpack
pub async fn read_meta<R: AsyncRead + Unpin, T: serde::de::DeserializeOwned>(
    reader: &mut R,
) -> Result<T, FrameError> {
    let data = read_frame(reader, MAX_META_SIZE).await?;
    let value = rmp_serde::from_slice(&data)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn frame_roundtrip() {
        let data = b"hello world";
        let mut buf = Vec::new();
        write_frame(&mut buf, data).await.unwrap();

        let mut cursor = std::io::Cursor::new(buf);
        let result = read_frame(&mut cursor, 1024).await.unwrap();
        assert_eq!(result, data);
    }

    #[tokio::test]
    async fn frame_too_large() {
        let data = vec![0u8; 100];
        let mut buf = Vec::new();
        write_frame(&mut buf, &data).await.unwrap();

        let mut cursor = std::io::Cursor::new(buf);
        let result = read_frame(&mut cursor, 50).await;
        assert!(matches!(result, Err(FrameError::TooLarge { size: 100, max: 50 })));
    }

    #[tokio::test]
    async fn meta_roundtrip() {
        use std::collections::HashMap;

        let original: HashMap<String, String> =
            [("key".into(), "value".into())].into_iter().collect();

        let mut buf = Vec::new();
        write_meta(&mut buf, &original).await.unwrap();

        let mut cursor = std::io::Cursor::new(buf);
        let decoded: HashMap<String, String> = read_meta(&mut cursor).await.unwrap();
        assert_eq!(decoded, original);
    }
}

use std::sync::atomic::{AtomicU64, Ordering};

use tokio::io::{self, AsyncRead, AsyncWrite, AsyncWriteExt};

/// result of a bidirectional relay between two streams.
pub struct RelayStats {
    /// bytes copied from side A to side B.
    pub a_to_b: u64,
    /// bytes copied from side B to side A.
    pub b_to_a: u64,
}

/// copy bytes bidirectionally between two streams with proper half-close.
///
/// when one direction reaches EOF, its write side is shut down while the
/// other direction continues until it also reaches EOF or errors. this
/// preserves TCP half-close semantics end-to-end.
///
/// returns the total bytes transferred in each direction.
pub async fn copy_bidirectional<A, B>(a: &mut A, b: &mut B) -> io::Result<RelayStats>
where
    A: AsyncRead + AsyncWrite + Unpin,
    B: AsyncRead + AsyncWrite + Unpin,
{
    let (mut a_read, mut a_write) = io::split(a);
    let (mut b_read, mut b_write) = io::split(b);

    let a_to_b_bytes = AtomicU64::new(0);
    let b_to_a_bytes = AtomicU64::new(0);

    let a_to_b = async {
        let result = io::copy(&mut a_read, &mut b_write).await;
        if let Ok(n) = &result {
            a_to_b_bytes.store(*n, Ordering::Relaxed);
        }
        let _ = b_write.shutdown().await;
        result
    };

    let b_to_a = async {
        let result = io::copy(&mut b_read, &mut a_write).await;
        if let Ok(n) = &result {
            b_to_a_bytes.store(*n, Ordering::Relaxed);
        }
        let _ = a_write.shutdown().await;
        result
    };

    // run both directions to completion, not racing them.
    // tokio::join! waits for both futures to finish.
    let (a_result, b_result) = tokio::join!(a_to_b, b_to_a);

    // if either direction had an error, return it, but prefer the first
    a_result?;
    b_result?;

    Ok(RelayStats {
        a_to_b: a_to_b_bytes.load(Ordering::Relaxed),
        b_to_a: b_to_a_bytes.load(Ordering::Relaxed),
    })
}

/// copy bytes bidirectionally between split read/write halves with proper
/// half-close. designed for cases where the two sides are already split
/// (e.g. QUIC SendStream/RecvStream + TCP read/write halves).
///
/// when one direction reaches EOF, the corresponding write side is shut down.
/// both directions run to completion.
pub async fn copy_bidirectional_split<AR, AW, BR, BW>(
    a_read: &mut AR,
    a_write: &mut AW,
    b_read: &mut BR,
    b_write: &mut BW,
) -> io::Result<RelayStats>
where
    AR: AsyncRead + Unpin,
    AW: AsyncWrite + Unpin,
    BR: AsyncRead + Unpin,
    BW: AsyncWrite + Unpin,
{
    let a_to_b_bytes = AtomicU64::new(0);
    let b_to_a_bytes = AtomicU64::new(0);

    let a_to_b = async {
        let result = io::copy(a_read, b_write).await;
        if let Ok(n) = &result {
            a_to_b_bytes.store(*n, Ordering::Relaxed);
        }
        let _ = b_write.shutdown().await;
        result
    };

    let b_to_a = async {
        let result = io::copy(b_read, a_write).await;
        if let Ok(n) = &result {
            b_to_a_bytes.store(*n, Ordering::Relaxed);
        }
        let _ = a_write.shutdown().await;
        result
    };

    let (a_result, b_result) = tokio::join!(a_to_b, b_to_a);

    a_result?;
    b_result?;

    Ok(RelayStats {
        a_to_b: a_to_b_bytes.load(Ordering::Relaxed),
        b_to_a: b_to_a_bytes.load(Ordering::Relaxed),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use tokio::io::duplex;

    type TestResult = Result<(), Box<dyn Error>>;

    #[tokio::test]
    async fn bidirectional_copy_both_directions() -> TestResult {
        let (mut left_client, mut left_server) = duplex(1024);
        let (mut right_client, mut right_server) = duplex(1024);

        // write data into both sides before starting relay
        let left_task = tokio::spawn(async move {
            tokio::io::AsyncWriteExt::write_all(&mut left_client, b"hello from left").await?;
            tokio::io::AsyncWriteExt::shutdown(&mut left_client).await?;

            let mut buf = Vec::new();
            tokio::io::AsyncReadExt::read_to_end(&mut left_client, &mut buf).await?;
            Ok::<_, io::Error>(buf)
        });

        let right_task = tokio::spawn(async move {
            tokio::io::AsyncWriteExt::write_all(&mut right_client, b"hello from right").await?;
            tokio::io::AsyncWriteExt::shutdown(&mut right_client).await?;

            let mut buf = Vec::new();
            tokio::io::AsyncReadExt::read_to_end(&mut right_client, &mut buf).await?;
            Ok::<_, io::Error>(buf)
        });

        let stats = copy_bidirectional(&mut left_server, &mut right_server).await?;

        let left_received = left_task.await??;
        let right_received = right_task.await??;

        assert_eq!(right_received, b"hello from left");
        assert_eq!(left_received, b"hello from right");
        assert_eq!(stats.a_to_b, 15);
        assert_eq!(stats.b_to_a, 16);
        Ok(())
    }

    #[tokio::test]
    async fn half_close_one_direction_first() -> TestResult {
        let (mut left_client, mut left_server) = duplex(1024);
        let (mut right_client, mut right_server) = duplex(1024);

        // left sends data then closes, right reads it, sends response, then closes
        let left_task = tokio::spawn(async move {
            tokio::io::AsyncWriteExt::write_all(&mut left_client, b"request").await?;
            tokio::io::AsyncWriteExt::shutdown(&mut left_client).await?;

            // should still be able to read the response after closing write side
            let mut buf = Vec::new();
            tokio::io::AsyncReadExt::read_to_end(&mut left_client, &mut buf).await?;
            Ok::<_, io::Error>(buf)
        });

        let right_task = tokio::spawn(async move {
            let mut buf = vec![0u8; 1024];
            let n = tokio::io::AsyncReadExt::read(&mut right_client, &mut buf).await?;
            assert_eq!(&buf[..n], b"request");

            // read EOF from left
            let n2 = tokio::io::AsyncReadExt::read(&mut right_client, &mut buf).await?;
            assert_eq!(n2, 0);

            // now send response and close
            tokio::io::AsyncWriteExt::write_all(&mut right_client, b"response").await?;
            tokio::io::AsyncWriteExt::shutdown(&mut right_client).await?;
            Ok::<_, io::Error>(())
        });

        let stats = copy_bidirectional(&mut left_server, &mut right_server).await?;

        let left_received = left_task.await??;
        right_task.await??;

        assert_eq!(left_received, b"response");
        assert_eq!(stats.a_to_b, 7); // "request"
        assert_eq!(stats.b_to_a, 8); // "response"
        Ok(())
    }

    #[tokio::test]
    async fn empty_transfer() -> TestResult {
        let (mut left_client, mut left_server) = duplex(1024);
        let (mut right_client, mut right_server) = duplex(1024);

        let left_task = tokio::spawn(async move {
            tokio::io::AsyncWriteExt::shutdown(&mut left_client).await?;
            let mut buf = Vec::new();
            tokio::io::AsyncReadExt::read_to_end(&mut left_client, &mut buf).await?;
            Ok::<_, io::Error>(())
        });

        let right_task = tokio::spawn(async move {
            tokio::io::AsyncWriteExt::shutdown(&mut right_client).await?;
            let mut buf = Vec::new();
            tokio::io::AsyncReadExt::read_to_end(&mut right_client, &mut buf).await?;
            Ok::<_, io::Error>(())
        });

        let stats = copy_bidirectional(&mut left_server, &mut right_server).await?;
        left_task.await??;
        right_task.await??;

        assert_eq!(stats.a_to_b, 0);
        assert_eq!(stats.b_to_a, 0);
        Ok(())
    }
}

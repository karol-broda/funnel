use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::harness::TcpTestEnv;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test(flavor = "multi_thread")]
async fn tcp_echo_basic() -> TestResult {
    let env = TcpTestEnv::start().await?;

    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", env.remote_port)).await?;

    stream.write_all(b"hello tcp").await?;
    stream.shutdown().await?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await?;

    assert_eq!(buf, b"hello tcp");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn tcp_large_payload() -> TestResult {
    let env = TcpTestEnv::start().await?;

    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", env.remote_port)).await?;

    let payload = vec![0xAB; 500_000];
    stream.write_all(&payload).await?;
    stream.shutdown().await?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await?;

    assert_eq!(buf.len(), payload.len());
    assert_eq!(buf, payload);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn tcp_multiple_connections() -> TestResult {
    let env = TcpTestEnv::start().await?;

    let mut set = tokio::task::JoinSet::new();
    for i in 0u8..5 {
        let port = env.remote_port;
        set.spawn(async move {
            let mut stream = TcpStream::connect(format!("127.0.0.1:{port}"))
                .await
                .unwrap();
            let msg = vec![i; 100];
            stream.write_all(&msg).await.unwrap();
            stream.shutdown().await.unwrap();

            let mut buf = Vec::new();
            stream.read_to_end(&mut buf).await.unwrap();
            assert_eq!(buf, msg);
        });
    }

    while let Some(result) = set.join_next().await {
        result?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn tcp_empty_payload() -> TestResult {
    let env = TcpTestEnv::start().await?;

    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", env.remote_port)).await?;
    stream.shutdown().await?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await?;
    assert!(buf.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn tcp_connection_after_disconnect() -> TestResult {
    let env = TcpTestEnv::start().await?;

    // first connection
    {
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", env.remote_port)).await?;
        stream.write_all(b"first").await?;
        stream.shutdown().await?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await?;
        assert_eq!(buf, b"first");
    }

    // second connection on same tunnel should work
    {
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", env.remote_port)).await?;
        stream.write_all(b"second").await?;
        stream.shutdown().await?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await?;
        assert_eq!(buf, b"second");
    }

    Ok(())
}

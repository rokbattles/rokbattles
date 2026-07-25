//! TCP listener and connection forwarding.

use std::{io, sync::Arc};

use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, TcpStream},
};
use tracing::{debug, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TransferStats {
    client_to_upstream_bytes: u64,
    upstream_to_client_bytes: u64,
}

#[derive(Debug, thiserror::Error)]
enum ConnectionError {
    #[error("failed to configure the client socket: {0}")]
    ConfigureClient(#[source] io::Error),
    #[error("failed to connect to the upstream server: {0}")]
    ConnectUpstream(#[source] io::Error),
    #[error("failed to configure the upstream socket: {0}")]
    ConfigureUpstream(#[source] io::Error),
    #[error("failed while forwarding bytes: {0}")]
    Forward(#[source] io::Error),
}

/// Accept connections from `listener` and forward each one to `upstream_addr`.
///
/// A connection failure is isolated to that client. The listener continues
/// accepting other clients until accepting itself fails or the task is
/// cancelled.
///
/// # Errors
///
/// Returns an I/O error if accepting a new client connection fails.
pub async fn serve(listener: TcpListener, upstream_addr: String) -> io::Result<()> {
    let upstream_addr: Arc<str> = upstream_addr.into();

    loop {
        let (client, client_addr) = listener.accept().await?;
        let upstream_addr = Arc::clone(&upstream_addr);

        std::mem::drop(tokio::spawn(async move {
            match relay_connection(client, &upstream_addr).await {
                Ok(stats) => {
                    debug!(
                        %client_addr,
                        client_to_upstream_bytes = stats.client_to_upstream_bytes,
                        upstream_to_client_bytes = stats.upstream_to_client_bytes,
                        "TCP relay connection closed"
                    );
                }
                Err(error) => {
                    warn!(%client_addr, %error, "TCP relay connection failed");
                }
            }
        }));
    }
}

async fn relay_connection(
    mut client: TcpStream,
    upstream_addr: &str,
) -> Result<TransferStats, ConnectionError> {
    client.set_nodelay(true).map_err(ConnectionError::ConfigureClient)?;

    let mut upstream =
        TcpStream::connect(upstream_addr).await.map_err(ConnectionError::ConnectUpstream)?;
    upstream.set_nodelay(true).map_err(ConnectionError::ConfigureUpstream)?;

    let (client_to_upstream_bytes, upstream_to_client_bytes) =
        copy_bidirectional(&mut client, &mut upstream).await.map_err(ConnectionError::Forward)?;

    Ok(TransferStats { client_to_upstream_bytes, upstream_to_client_bytes })
}

#[cfg(test)]
mod tests {
    use std::{future::Future, net::SocketAddr, time::Duration};

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        task::JoinHandle,
        time::timeout,
    };

    use super::*;

    const TEST_TIMEOUT: Duration = Duration::from_secs(3);

    async fn within<T>(future: impl Future<Output = T>) -> T {
        timeout(TEST_TIMEOUT, future).await.expect("socket operation should not time out")
    }

    async fn connected_streams()
    -> (TcpStream, TcpStream, JoinHandle<Result<TransferStats, ConnectionError>>) {
        let upstream_listener =
            TcpListener::bind("127.0.0.1:0").await.expect("upstream listener should bind");
        let upstream_addr =
            upstream_listener.local_addr().expect("upstream address should be available");
        let relay_listener =
            TcpListener::bind("127.0.0.1:0").await.expect("relay listener should bind");
        let relay_addr = relay_listener.local_addr().expect("relay address should be available");

        let relay_task = tokio::spawn(async move {
            let (client, _) =
                relay_listener.accept().await.expect("relay should accept the client");
            relay_connection(client, &upstream_addr.to_string()).await
        });

        let client =
            TcpStream::connect(relay_addr).await.expect("client should connect to the relay");
        let (upstream, _) =
            upstream_listener.accept().await.expect("relay should connect upstream");

        (client, upstream, relay_task)
    }

    #[tokio::test]
    async fn connection_should_forward_exact_bytes_in_both_directions() {
        let (mut client, mut upstream, relay_task) = connected_streams().await;
        let client_payload = b"client-to-upstream";
        let upstream_payload = b"upstream-to-client";

        client.write_all(client_payload).await.expect("client payload should write");
        let mut received_upstream = vec![0; client_payload.len()];
        upstream
            .read_exact(&mut received_upstream)
            .await
            .expect("upstream should receive the client payload");

        upstream.write_all(upstream_payload).await.expect("upstream payload should write");
        let mut received_client = vec![0; upstream_payload.len()];
        client
            .read_exact(&mut received_client)
            .await
            .expect("client should receive the upstream payload");

        client.shutdown().await.expect("client write side should close");
        upstream.shutdown().await.expect("upstream write side should close");
        let stats = within(relay_task)
            .await
            .expect("relay task should not panic")
            .expect("relay connection should finish");

        assert_eq!(
            (received_upstream, received_client, stats),
            (
                client_payload.to_vec(),
                upstream_payload.to_vec(),
                TransferStats {
                    client_to_upstream_bytes: client_payload.len() as u64,
                    upstream_to_client_bytes: upstream_payload.len() as u64,
                },
            )
        );
    }

    #[tokio::test]
    async fn connection_should_forward_payloads_larger_than_copy_buffers() {
        let (mut client, mut upstream, relay_task) = connected_streams().await;
        let client_payload = vec![0x5a; 64 * 1024];
        let upstream_payload = vec![0xa5; 96 * 1024];

        let client_exchange = async {
            client.write_all(&client_payload).await.expect("client payload should write");
            client.shutdown().await.expect("client write side should close");
            let mut response = Vec::new();
            client.read_to_end(&mut response).await.expect("client response should read");
            response
        };
        let upstream_exchange = async {
            let mut request = Vec::new();
            upstream.read_to_end(&mut request).await.expect("upstream request should read");
            upstream.write_all(&upstream_payload).await.expect("upstream payload should write");
            upstream.shutdown().await.expect("upstream write side should close");
            request
        };

        let (received_client, received_upstream) =
            within(async { tokio::join!(client_exchange, upstream_exchange) }).await;
        let stats = within(relay_task)
            .await
            .expect("relay task should not panic")
            .expect("relay connection should finish");

        assert_eq!(
            (received_upstream, received_client, stats),
            (
                client_payload,
                upstream_payload,
                TransferStats {
                    client_to_upstream_bytes: (64 * 1024) as u64,
                    upstream_to_client_bytes: (96 * 1024) as u64,
                },
            )
        );
    }

    #[tokio::test]
    async fn client_half_close_should_allow_upstream_response() {
        let (mut client, mut upstream, relay_task) = connected_streams().await;
        let client_payload = b"request before half-close";
        let upstream_payload = b"response after client EOF";

        client.write_all(client_payload).await.expect("client payload should write");
        client.shutdown().await.expect("client write side should close");

        let mut received_upstream = Vec::new();
        within(upstream.read_to_end(&mut received_upstream))
            .await
            .expect("upstream should observe client EOF");
        upstream.write_all(upstream_payload).await.expect("upstream payload should write");
        upstream.shutdown().await.expect("upstream write side should close");

        let mut received_client = Vec::new();
        within(client.read_to_end(&mut received_client))
            .await
            .expect("client should observe upstream EOF");
        within(relay_task)
            .await
            .expect("relay task should not panic")
            .expect("relay connection should finish");

        assert_eq!(
            (received_upstream, received_client),
            (client_payload.to_vec(), upstream_payload.to_vec())
        );
    }

    #[tokio::test]
    async fn upstream_half_close_should_allow_remaining_client_bytes() {
        let (mut client, mut upstream, relay_task) = connected_streams().await;
        let upstream_payload = b"response before half-close";
        let client_payload = b"request after upstream EOF";

        upstream.write_all(upstream_payload).await.expect("upstream payload should write");
        upstream.shutdown().await.expect("upstream write side should close");

        let mut received_client = Vec::new();
        within(client.read_to_end(&mut received_client))
            .await
            .expect("client should observe upstream EOF");
        client.write_all(client_payload).await.expect("client payload should write");
        client.shutdown().await.expect("client write side should close");

        let mut received_upstream = Vec::new();
        within(upstream.read_to_end(&mut received_upstream))
            .await
            .expect("upstream should observe client EOF");
        within(relay_task)
            .await
            .expect("relay task should not panic")
            .expect("relay connection should finish");

        assert_eq!(
            (received_client, received_upstream),
            (upstream_payload.to_vec(), client_payload.to_vec())
        );
    }

    #[tokio::test]
    async fn listener_should_handle_concurrent_connections() {
        let upstream_listener =
            TcpListener::bind("127.0.0.1:0").await.expect("upstream listener should bind");
        let upstream_addr =
            upstream_listener.local_addr().expect("upstream address should be available");
        let relay_listener =
            TcpListener::bind("127.0.0.1:0").await.expect("relay listener should bind");
        let relay_addr = relay_listener.local_addr().expect("relay address should be available");
        let relay_task = tokio::spawn(serve(relay_listener, upstream_addr.to_string()));

        let upstream_task = tokio::spawn(async move {
            let (first, _) =
                upstream_listener.accept().await.expect("first upstream should accept");
            let (second, _) =
                upstream_listener.accept().await.expect("second upstream should accept");
            tokio::join!(echo(first), echo(second));
        });

        let first = exchange_with_relay(relay_addr, b"first connection");
        let second = exchange_with_relay(relay_addr, b"second connection");
        let (first_response, second_response) = within(async { tokio::join!(first, second) }).await;
        within(upstream_task).await.expect("upstream task should not panic");
        relay_task.abort();

        assert_eq!(
            (first_response, second_response),
            (b"first connection".to_vec(), b"second connection".to_vec())
        );
    }

    #[tokio::test]
    async fn failed_upstream_connection_should_not_stop_listener() {
        let unavailable_listener =
            TcpListener::bind("127.0.0.1:0").await.expect("port reservation should bind");
        let upstream_addr =
            unavailable_listener.local_addr().expect("reserved address should be available");
        drop(unavailable_listener);

        let relay_listener =
            TcpListener::bind("127.0.0.1:0").await.expect("relay listener should bind");
        let relay_addr = relay_listener.local_addr().expect("relay address should be available");
        let relay_task = tokio::spawn(serve(relay_listener, upstream_addr.to_string()));

        let mut failed_client =
            TcpStream::connect(relay_addr).await.expect("first client should connect");
        let mut failed_response = Vec::new();
        within(failed_client.read_to_end(&mut failed_response))
            .await
            .expect("failed connection should close");

        let upstream_listener =
            TcpListener::bind(upstream_addr).await.expect("upstream listener should recover");
        let upstream_task = tokio::spawn(async move {
            let (upstream, _) =
                upstream_listener.accept().await.expect("recovered upstream should accept");
            echo(upstream).await;
        });
        let response = within(exchange_with_relay(relay_addr, b"listener survived")).await;
        within(upstream_task).await.expect("upstream task should not panic");
        relay_task.abort();

        assert_eq!((failed_response, response), (Vec::new(), b"listener survived".to_vec()));
    }

    async fn exchange_with_relay(relay_addr: SocketAddr, payload: &[u8]) -> Vec<u8> {
        let mut client =
            TcpStream::connect(relay_addr).await.expect("client should connect to relay");
        client.write_all(payload).await.expect("client payload should write");
        client.shutdown().await.expect("client write side should close");

        let mut response = Vec::new();
        client.read_to_end(&mut response).await.expect("relay response should read");
        response
    }

    async fn echo(mut stream: TcpStream) {
        let mut payload = Vec::new();
        stream.read_to_end(&mut payload).await.expect("echo payload should read");
        stream.write_all(&payload).await.expect("echo payload should write");
        stream.shutdown().await.expect("echo write side should close");
    }
}

//! ClamAV scanner client using the `zINSTREAM` command.
//!
//! The `z` selects NUL-terminated framing, not compression. Payload bytes are sent
//! unchanged, and only a complete, bounded scan response is accepted.

use std::time::Duration;

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    time::timeout,
};

/// A completed scan; scanner failures are returned as [`ScanError`] instead.
#[derive(Debug)]
pub enum ScanStatus {
    Clean,
    /// The full detection response, without the terminating NUL.
    Infected(String),
}

/// Errors produced while scanning with ClamAV.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("scan timed out")]
    Timeout,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("unexpected response: {0}")]
    UnexpectedResponse(String),
}

const CHUNK_SIZE: usize = 1024 * 1024;
const MAX_RESPONSE_BYTES: u64 = 4096;

/// Scans the original payload using INSTREAM with NUL-terminated command framing.
///
/// The deadline covers connection, upload, and response reading. I/O failures,
/// timeouts, and malformed responses return errors instead of a clean result.
pub async fn scan_instream(
    payload: &[u8],
    addr: &str,
    timeout_duration: Duration,
) -> Result<ScanStatus, ScanError> {
    let response = timeout(timeout_duration, async move {
        let mut stream = TcpStream::connect(addr).await?;
        stream.write_all(b"zINSTREAM\0").await?;

        for chunk in payload.chunks(CHUNK_SIZE) {
            let len = u32::try_from(chunk.len()).map_err(std::io::Error::other)?;
            stream.write_all(&len.to_be_bytes()).await?;
            stream.write_all(chunk).await?;
        }

        stream.write_all(&0u32.to_be_bytes()).await?;
        stream.flush().await?;

        // Read through NUL rather than waiting for the scanner to close the connection.
        let mut response = Vec::new();
        BufReader::new(stream.take(MAX_RESPONSE_BYTES)).read_until(0, &mut response).await?;

        Ok::<Vec<u8>, std::io::Error>(response)
    })
    .await
    .map_err(|_error| ScanError::Timeout)??;

    parse_response(&response)
}

fn parse_response(response: &[u8]) -> Result<ScanStatus, ScanError> {
    // EOF or the byte limit can end the read without a complete response.
    let Some(record) = response.strip_suffix(b"\0") else {
        return Err(ScanError::UnexpectedResponse("unterminated response".to_string()));
    };

    let response_str = String::from_utf8_lossy(record);
    if response_str == "stream: OK" {
        return Ok(ScanStatus::Clean);
    }

    if let Some(signature) =
        response_str.strip_prefix("stream: ").and_then(|result| result.strip_suffix(" FOUND"))
        && !signature.is_empty()
        && !signature.contains(['\0', '\n', '\r'])
    {
        return Ok(ScanStatus::Infected(response_str.into_owned()));
    }

    Err(ScanError::UnexpectedResponse(response_str.to_string()))
}

#[cfg(test)]
mod tests {
    use tokio::net::TcpListener;

    use super::*;

    #[tokio::test]
    async fn sends_original_payload_in_length_prefixed_chunks() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind scanner");
        let addr = listener.local_addr().expect("scanner address").to_string();
        let payload = vec![0x42; CHUNK_SIZE + 17];

        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept scan");
            let mut command = [0; 10];
            stream.read_exact(&mut command).await.expect("read command");
            assert_eq!(&command, b"zINSTREAM\0");

            let mut received = Vec::new();
            let mut lengths = Vec::new();
            loop {
                let length = stream.read_u32().await.expect("read chunk length") as usize;
                if length == 0 {
                    break;
                }

                assert!(length <= CHUNK_SIZE);
                let mut chunk = vec![0; length];
                stream.read_exact(&mut chunk).await.expect("read chunk");
                lengths.push(length);
                received.extend_from_slice(&chunk);
            }

            stream.write_all(b"stream: OK\0").await.expect("write response");
            (received, lengths)
        });

        let result =
            scan_instream(&payload, &addr, Duration::from_secs(5)).await.expect("scan payload");
        assert!(matches!(result, ScanStatus::Clean));

        let (received, lengths) = server.await.expect("scanner task");
        assert_eq!(received.len(), payload.len());
        assert_eq!(received, payload);
        assert_eq!(lengths, [CHUNK_SIZE, 17]);
    }

    #[test]
    fn rejects_error_response_containing_ok() {
        parse_response(b"stream: unable to open OK-file ERROR\0")
            .expect_err("an error containing OK must not pass the scan");
    }

    #[test]
    fn rejects_truncated_response() {
        parse_response(b"stream: OK").expect_err("a response must be NUL-terminated");
    }

    #[tokio::test]
    async fn empty_payload_reads_fragmented_response_without_waiting_for_eof() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind scanner");
        let addr = listener.local_addr().expect("scanner address").to_string();

        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept scan");
            let mut command = [0; 10];
            stream.read_exact(&mut command).await.expect("read command");
            assert_eq!(&command, b"zINSTREAM\0");
            assert_eq!(stream.read_u32().await.expect("read empty chunk"), 0);

            stream.write_all(b"stream: ").await.expect("write response prefix");
            tokio::task::yield_now().await;
            stream.write_all(b"OK\0").await.expect("write response suffix");

            let mut byte = [0];
            assert_eq!(stream.read(&mut byte).await.expect("wait for client close"), 0);
        });

        let result =
            scan_instream(b"", &addr, Duration::from_secs(5)).await.expect("scan empty payload");
        assert!(matches!(result, ScanStatus::Clean));
        server.await.expect("scanner task");
    }

    #[tokio::test]
    async fn times_out_when_scanner_does_not_reply() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind scanner");
        let addr = listener.local_addr().expect("scanner address").to_string();

        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.expect("accept scan");
            std::future::pending::<()>().await;
        });

        let result = scan_instream(b"mail", &addr, Duration::from_millis(50)).await;
        server.abort();

        assert!(matches!(result, Err(ScanError::Timeout)));
    }

    #[tokio::test]
    async fn rejects_oversized_response() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind scanner");
        let addr = listener.local_addr().expect("scanner address").to_string();

        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept scan");
            let mut request = [0; 14];
            stream.read_exact(&mut request).await.expect("read empty scan");

            let response = vec![b'x'; MAX_RESPONSE_BYTES as usize];
            stream.write_all(&response).await.expect("write oversized response");

            let mut byte = [0];
            assert_eq!(stream.read(&mut byte).await.expect("wait for client close"), 0);
        });

        let result = scan_instream(b"", &addr, Duration::from_secs(5)).await;
        server.await.expect("scanner task");

        assert!(matches!(result, Err(ScanError::UnexpectedResponse(_))));
    }

    #[test]
    fn rejects_malformed_scan_results() {
        for response in [
            b"stream: NOT OK\0".as_slice(),
            b"stream: OKAY\0",
            b"stream: FOUND in error text ERROR\0",
            b"INSTREAM size limit exceeded. ERROR\0",
            b"stream: \xff OK\0",
            b"\0",
        ] {
            parse_response(response).expect_err("only a complete scan result is accepted");
        }
    }

    #[test]
    fn parses_clean_response() {
        let response = b"stream: OK\0";
        assert!(matches!(parse_response(response).unwrap(), ScanStatus::Clean));
    }

    #[test]
    fn parses_infected_response() {
        let response = b"stream: Eicar-Test-Signature FOUND\0";
        match parse_response(response).unwrap() {
            ScanStatus::Infected(message) => {
                assert!(message.contains("FOUND"));
            }
            _ => panic!("expected infected"),
        }
    }

    #[test]
    fn parses_error_response() {
        let response = b"stream: ERROR\0";
        assert!(matches!(parse_response(response), Err(ScanError::UnexpectedResponse(_))));
    }
}

//! Minimal ClamAV zINSTREAM scanner client.

use std::io::Write;
use std::time::Duration;

use flate2::{Compression, write::ZlibEncoder};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Debug)]
pub enum ScanStatus {
    Clean,
    Infected(String),
}

/// Errors produced while scanning with ClamAV.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("connection timed out")]
    Timeout,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unexpected response: {0}")]
    UnexpectedResponse(String),
    #[error("compression error: {0}")]
    Compression(String),
}

const CHUNK_SIZE: usize = 1024 * 1024;

/// Scan a payload using ClamAV's zINSTREAM protocol (zlib-compressed stream).
pub async fn scan_zstream(
    payload: &[u8],
    addr: &str,
    timeout_duration: Duration,
) -> Result<ScanStatus, ScanError> {
    let compressed = compress_zlib(payload)?;
    let response = timeout(timeout_duration, async move {
        let mut stream = TcpStream::connect(addr).await?;
        stream.write_all(b"zINSTREAM\0").await?;

        for chunk in compressed.chunks(CHUNK_SIZE) {
            let len = u32::try_from(chunk.len()).unwrap_or(u32::MAX);
            stream.write_all(&len.to_be_bytes()).await?;
            stream.write_all(chunk).await?;
        }

        stream.write_all(&0u32.to_be_bytes()).await?;
        stream.flush().await?;

        let mut response = Vec::new();
        stream.read_to_end(&mut response).await?;
        Ok::<Vec<u8>, std::io::Error>(response)
    })
    .await
    .map_err(|_| ScanError::Timeout)??;

    parse_response(&response)
}

fn compress_zlib(payload: &[u8]) -> Result<Vec<u8>, ScanError> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    encoder
        .write_all(payload)
        .map_err(|error| ScanError::Compression(error.to_string()))?;
    encoder
        .finish()
        .map_err(|error| ScanError::Compression(error.to_string()))
}

fn parse_response(response: &[u8]) -> Result<ScanStatus, ScanError> {
    let trimmed = response
        .iter()
        .copied()
        .take_while(|byte| *byte != 0)
        .collect::<Vec<u8>>();
    let response_str = String::from_utf8_lossy(&trimmed);
    let response_str = response_str.trim();

    if response_str.contains("FOUND") {
        return Ok(ScanStatus::Infected(response_str.to_string()));
    }
    if response_str.contains("OK") {
        return Ok(ScanStatus::Clean);
    }
    if response_str.contains("ERROR") {
        return Err(ScanError::UnexpectedResponse(response_str.to_string()));
    }

    Err(ScanError::UnexpectedResponse(response_str.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_clean_response() {
        let response = b"stream: OK\0";
        assert!(matches!(
            parse_response(response).unwrap(),
            ScanStatus::Clean
        ));
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
        assert!(matches!(
            parse_response(response),
            Err(ScanError::UnexpectedResponse(_))
        ));
    }
}

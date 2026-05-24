//! Shared stream ids and constants.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// Default TCP port for the remote service.
pub const CLIENT_PORT: u16 = 3101;

/// Direction of a packet relative to the remote service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Local client to remote service.
    ClientToServer,
    /// Remote service to local client.
    ServerToClient,
}

/// Stable identity for one TCP connection.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StreamId {
    /// Local client IP address from packet capture.
    pub client_addr: IpAddr,
    /// Client-side ephemeral TCP port.
    pub client_port: u16,
    /// Remote service IP address.
    pub server_addr: IpAddr,
    /// Remote service TCP port, normally [`CLIENT_PORT`].
    pub server_port: u16,
}

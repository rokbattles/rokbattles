//! IPv4 NAT forwarding with passive collection of Rise of Kingdoms mail.
//!
//! This Linux service delegates forwarding to nftables and connection tracking
//! (conntrack). Userspace receives packet copies through NFLOG, reconstructs
//! the server stream, and uploads decoded mail to the relay ingress endpoint.
//! Capture does not supply packet verdicts: a stalled or stopped collector does
//! not stop connections already forwarded by the kernel.
//!
//! # Startup and privileges
//!
//! The binary disables core dumps, reads [`config`], and resolves the upstream.
//! While privileged, it opens [`capture::Capture`] and installs the rules through
//! [`service::install`]. It then drops its identity and capabilities before
//! loading protocol descriptors, decoding payloads, or starting worker threads.
//! These process-wide operations assume a dedicated gateway node; see [`service`].
//!
//! # Collection pipeline
//!
//! [`capture`] validates netlink framing and direction metadata. [`packet`]
//! borrows IPv4/TCP fields, [`reassembly`] orders server bytes, and [`observer`]
//! feeds the shared protocol decoder. [`spool`] buffers and persists completed
//! batches independently of each connection's lifetime.
//!
//! Observation requires a client SYN and matching server SYN/ACK. Restarting the
//! collector preserves kernel connections but loses their decoder state; those
//! connections are observable again only after a new handshake. Capture loss
//! also discards decoder state rather than guessing missing cipher bytes.
//!
//! # Delivery limits
//!
//! Queue acceptance is not a disk commit. Queued batches can be lost if the
//! process exits before they are written. Committed batches survive restarts
//! and are retried after upload failures; acknowledged batches are removed even
//! when ingress rejects individual entries. Memory, disk, and capture buffers
//! are bounded, so overload or storage failures can still lose observations.
//!
//! Pure parsers and rule renderers can be exercised without privileges; see the
//! examples on [`config::Config::from_lookup`] and [`reassembly::Reassembly`].

#![cfg(target_os = "linux")]
#![deny(missing_docs)]

pub mod capture;
pub mod config;
pub mod observer;
pub mod packet;
pub mod reassembly;
pub mod rules;
pub mod service;
pub mod spool;

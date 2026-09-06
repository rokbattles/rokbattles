//! Application payload decoders enabled by the `schemas` feature.
//!
//! Each schema ID identifies one wire layout. Decoders validate its fields and
//! consume the complete payload.

pub mod territory;

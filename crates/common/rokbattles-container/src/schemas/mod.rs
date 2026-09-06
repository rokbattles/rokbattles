//! Application payload schemas available with the `schemas` feature.
//!
//! Schema IDs identify complete wire layouts. The territory layouts use unsigned
//! base-128 varints, zigzag signed values, and quantized coordinates. Decoders
//! consume the entire payload and reject unsupported tags or arithmetic overflow.

pub mod territory;

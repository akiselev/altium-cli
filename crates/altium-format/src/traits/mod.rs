//! Core traits for Altium value conversion.
//!
//! This module provides traits for converting between Rust types and Altium parameter values.

mod conversion;

pub use conversion::{FromParamList, FromParamValue, ToParamList, ToParamValue};

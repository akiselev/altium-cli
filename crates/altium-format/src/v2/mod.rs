//! V2 Altium format implementation ported from decompiled C#.
//!
//! This module runs in parallel with the existing (v1) code and fixes
//! several critical bugs found by comparing decompiled C# against v1:
//!
//! - Coordinate system: uses correct 100,000 units/mil (v1 uses 10,000)
//! - Binary pin format: writes OwnerIndex, not record_type=2
//! - No phantom byte in binary pin serialization
//! - Correct FormalType export (actual value, not always 0)
//! - SwapIdPin field name (v1 incorrectly uses SwapIdGroup)
//! - Section key truncation: 30 chars with collision avoidance (v1 uses 31)
//! - Pin extended data streams (PinFrac, PinWideText, PinTextData, etc.)
//!
//! # Architecture
//!
//! - [`coord`] — V2Coord with 100K units/mil
//! - [`types`] — Shared enums (ObjectId, PinElectrical, etc.)
//! - [`consts`] — Parameter name constants from FileFormatConsts.cs
//! - [`serializer`] — SchSerializer trait + ASCII/Binary implementations
//! - [`fields`] — Per-record data structs and format functions
//! - [`io`] — SchLib/SchDoc file I/O

pub mod consts;
pub mod coord;
pub mod fields;
pub mod io;
pub mod pcb;
pub mod serializer;
pub mod types;

pub use coord::{V2Coord, V2Point};
pub use types::ObjectId;

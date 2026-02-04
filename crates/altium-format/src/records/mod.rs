//! Record types for Altium schematic and PCB primitives.
//!
//! **DEPRECATED**: V1 record types are being replaced by v2 implementations.
//! V1 has coordinate scale bugs and field type mismatches.
//!
//! # Migration Map
//!
//! | V1 Type | V2 Replacement |
//! |---------|----------------|
//! | `sch::SchPin` | `v2::fields::PinData` |
//! | `sch::SchComponent` | `v2::fields::ComponentData` |
//! | `sch::SchRecord` | `v2::fields::TypedRecord` |
//! | `sch::SchWire` | `v2::fields::WireData` |
//! | `sch::SchLabel` | `v2::fields::LabelData` |
//! | `pcb::PcbPad` | `v2::pcb::PcbPadV2` |
//! | `pcb::PcbTrack` | `v2::pcb::PcbTrackV2` |
//! | `pcb::PcbVia` | `v2::pcb::PcbViaV2` |
//! | `pcb::PcbRecord` | v2::pcb types directly |
//!
//! # Modules Still Using V1 Types
//!
//! The following modules still reference V1 types and need migration:
//! - `edit/` - Edit session, library editing, layout, routing
//! - `query/` - Query engine and selectors
//! - `footprint/` - Footprint builder and renderer
//! - `ops/` - Some queries and transforms
//! - `dump/` - DumpTree implementations
//!
//! These modules are marked with `#[allow(deprecated)]` and will be migrated
//! in future milestones.

#![allow(deprecated)]

pub mod pcb;
pub mod sch;

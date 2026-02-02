//! PCB binary format support (PcbDoc/PcbLib).
//!
//! Implements Ghidra-verified, byte-accurate PCB binary record layouts.
//! PCB coordinates use 10,000 internal units per mil (distinct from SchLib's 100,000).

pub mod arc;
pub mod board;
pub mod class;
pub mod component;
pub mod component_body;
pub mod connection;
pub mod constants;
pub mod coord;
pub mod dimension;
pub mod enums;
pub mod fill;
pub mod io;
pub mod net;
pub mod pad;
pub mod polygon;
pub mod primitive;
pub mod region;
pub mod rule;
pub mod text;
pub mod track;
pub mod via;

pub use coord::{PcbCoord, PcbPoint};
pub use enums::*;
pub use primitive::{PcbCommonHeader, PcbObjectId, PcbTrailingFields};

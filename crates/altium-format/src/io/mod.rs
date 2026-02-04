//! I/O utilities for reading and writing Altium files.
//!
//! This module provides:
//! - `prjpcb` - Project file (.PrjPcb) reading/writing
//! - `reader` - Low-level binary block reading utilities
//! - `writer` - Low-level binary block writing utilities
//!
//! For file I/O, use the V2 API:
//! - `v2::io::schlib::SchLibV2` for SchLib files
//! - `v2::io::schdoc::SchDocV2` for SchDoc files
//! - `v2::pcb::io::pcblib::PcbLib` for PcbLib files
//! - `v2::pcb::io::pcbdoc::PcbDoc` for PcbDoc files

pub mod prjpcb;
pub mod reader;
pub mod writer;

pub use prjpcb::*;
pub use reader::*;
pub use writer::*;

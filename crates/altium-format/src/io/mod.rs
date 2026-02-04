//! I/O utilities for reading and writing Altium files.
//!
//! **DEPRECATED**: V1 I/O types have fundamental bugs (coordinate scales, field types).
//! Migration map to v2:
//! - `SchLib` -> `v2::io::schlib::SchLibV2`
//! - `SchDoc` -> `v2::io::schdoc::SchDocV2`
//! - `PcbLib` -> `v2::pcb::io::pcblib::PcbLibV2`
//! - `PcbDoc` -> `v2::pcb::io::pcbdoc::PcbDocV2`
//! - `IntLib` -> embedded SchLib/PcbLib use v2 equivalents
//!
//! Altium files use COM/OLE Compound Storage (CFB) format.
//! Project files (.PrjPcb) use INI-style text format.

// Deprecated modules contain stubs to force migration. Impl blocks allow deprecated internally.
pub mod intlib;
pub mod pcbdoc;
pub mod pcblib;
pub mod prjpcb;
pub mod reader;
pub mod schdoc;
pub mod schlib;
pub mod writer;

// Re-exports expose deprecated types - consumers will see warnings on use
#[allow(deprecated)]
pub use intlib::*;
#[allow(deprecated)]
pub use pcbdoc::*;
#[allow(deprecated)]
pub use pcblib::*;
pub use prjpcb::*;
pub use reader::*;
#[allow(deprecated)]
pub use schdoc::*;
#[allow(deprecated)]
pub use schlib::*;
pub use writer::*;

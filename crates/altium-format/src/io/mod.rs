//! I/O utilities for reading and writing Altium files.
//!
//! Altium files use COM/OLE Compound Storage (CFB) format.
//! Project files (.PrjPcb) use INI-style text format.

pub mod intlib;
pub mod pcbdoc;
pub mod pcblib;
pub mod prjpcb;
pub mod reader;
pub mod schdoc;
pub mod schlib;
pub mod writer;

pub use intlib::*;
pub use pcbdoc::*;
pub use pcblib::*;
pub use prjpcb::*;
pub use reader::*;
pub use schdoc::*;
pub use schlib::*;
pub use writer::*;

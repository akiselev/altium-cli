//! Document types for Altium file I/O.
//!
//! Each document type represents a complete Altium file (SchLib, SchDoc, PcbLib)
//! using the v2 backing-store architecture. Documents handle reading from and
//! writing to CFB (Compound File Binary) format.

pub mod section_keys;
pub mod schlib;
pub mod schdoc;
pub mod pcblib;

pub use section_keys::SectionKeyList;
pub use schlib::{SchLib, SchLibHeader, SchLibComponentEntry};
pub use schdoc::SchDoc;
pub use pcblib::PcbLib;

//! Document types for Altium file I/O.
//!
//! Each document type represents a complete Altium file (SchLib, SchDoc, PcbLib)
//! using the v2 backing-store architecture. Documents handle reading from and
//! writing to CFB (Compound File Binary) format.

pub(crate) mod encoding;
pub mod pcblib;
pub mod schdoc;
pub mod schlib;
pub mod section_keys;

pub use pcblib::PcbLib;
pub use schdoc::SchDoc;
pub use schlib::{SchLib, SchLibComponentEntry, SchLibHeader};
pub use section_keys::SectionKeyList;

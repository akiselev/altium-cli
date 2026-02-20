//! Document types for Altium file I/O.
//!
//! Each document type represents a complete Altium file (SchLib, SchDoc, PcbLib)
//! using the v2 backing-store architecture. Documents handle reading from and
//! writing to CFB (Compound File Binary) format.

pub(crate) mod encoding;
pub mod pcbdoc;
pub mod pcbdoc_streams;
pub mod pcblib;
pub mod pcblib_streams;
pub mod schdoc;
pub mod schdoc_streams;
pub mod schlib;
pub mod schlib_streams;
pub mod section_keys;

pub use pcbdoc::PcbDoc;
pub use pcblib::PcbLib;
pub use schdoc::SchDoc;
pub use schlib::{SchLib, SchLibComponentEntry, SchLibHeader};
pub use section_keys::SectionKeyList;

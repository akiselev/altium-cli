// Schematic file format constants from Altium Designer 26.
//
// Sourced from `Altium.Sch.DataModel.FileFormats.FileFormatConsts`.
// Every constant here is the authoritative key name or value used in the on-disk
// parametric format. String values are case-sensitive and preserve Altium's
// original spelling (including intentional typos like "IsNotAccesible").
//
// Organization mirrors the logical grouping of the C# source, split into
// domain-specific modules for discoverability.

pub mod component;
pub mod electrical;
pub mod file_headers;
pub mod harness;
pub mod locking;
pub mod model;
pub mod parsing;
pub mod pin;
pub mod record_structure;
pub mod reuse;
pub mod sheet;
pub mod streams;
pub mod text;
pub mod vault;
pub mod visual;

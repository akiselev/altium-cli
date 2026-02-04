//! SchDoc reader/writer for Altium schematic document files.
//!
//! **DEPRECATED**: V1 IO is replaced by v2 with proper field deserialization.
//! V1 has coordinate scale bugs and unsafe field parsing. Use v2::io::schdoc::SchDocV2.

#![allow(unused_imports)]
#![allow(dead_code)]

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use cfb::CompoundFile;
use std::fs::File;
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;

use crate::dump::{DumpTree, TreeBuilder};
use crate::error::{AltiumError, Result};
use crate::format::SIZE_FLAG_MASK;
use crate::io::reader::{decode_windows_1252, read_parameters_block};
use crate::io::writer::{write_block, write_parameters};
use crate::records::sch::{SchPrimitive, SchRecord, SchSheetHeader};
use crate::types::ParameterCollection;

/// A schematic document containing primitives.
///
/// **DEPRECATED**: Use `v2::io::schdoc::SchDocV2` instead.
/// V1 has coordinate scale bugs and field type mismatches.
#[deprecated(note = "Use v2::io::schdoc::SchDocV2")]
#[derive(Debug, Default)]
pub struct SchDoc {
    /// All primitives in the document.
    pub primitives: Vec<SchRecord>,
    /// Optional document name (typically the filename without extension).
    /// This is used as the sheet name in queries, not the Title parameter.
    pub document_name: Option<String>,
}

#[allow(deprecated)]
impl SchDoc {
    /// Open and read a SchDoc file.
    ///
    /// **DEPRECATED**: Use `v2::io::schdoc::SchDocV2::open()` instead.
    #[deprecated(note = "Use v2::io::schdoc::SchDocV2::open()")]
    pub fn open<R: Read + Seek>(_reader: R) -> Result<Self> {
        unimplemented!("Use v2::io::schdoc::SchDocV2::open() - v1 API has been deprecated")
    }

    /// Open and read a SchDoc file from a path.
    ///
    /// **DEPRECATED**: Use `v2::io::schdoc::SchDocV2::open_file()` instead.
    #[deprecated(note = "Use v2::io::schdoc::SchDocV2::open_file()")]
    pub fn open_file<P: AsRef<Path>>(_path: P) -> Result<Self> {
        unimplemented!("Use v2::io::schdoc::SchDocV2::open_file() - v1 API has been deprecated")
    }

    /// Save the SchDoc to a file.
    ///
    /// **DEPRECATED**: Use `v2::io::schdoc::SchDocV2::write()` instead.
    #[deprecated(note = "Use v2::io::schdoc::SchDocV2::write()")]
    pub fn save<W: Read + Write + Seek>(&self, _writer: W) -> Result<()> {
        unimplemented!("Use v2::io::schdoc::SchDocV2::write() - v1 API has been deprecated")
    }

    /// Save the SchDoc to a file path.
    ///
    /// **DEPRECATED**: Use `v2::io::schdoc::SchDocV2::write_to_file()` instead.
    #[deprecated(note = "Use v2::io::schdoc::SchDocV2::write_to_file()")]
    pub fn save_to_file<P: AsRef<Path>>(&self, _path: P) -> Result<()> {
        unimplemented!("Use v2::io::schdoc::SchDocV2::write_to_file() - v1 API has been deprecated")
    }

    // Internal methods stubbed to prevent accidental usage.

    fn read_file_header<R: Read + Seek>(&mut self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::io::schdoc::SchDocV2")
    }

    fn read_record<R: Read>(&self, _reader: &mut R) -> Result<SchRecord> {
        unimplemented!("Replaced by v2::io::schdoc::SchDocV2")
    }

    fn write_storage<F: Read + Write + Seek>(&self, _cf: &mut CompoundFile<F>) -> Result<()> {
        unimplemented!("Replaced by v2::io::schdoc::SchDocV2")
    }

    fn write_file_header<F: Read + Write + Seek>(&self, _cf: &mut CompoundFile<F>) -> Result<()> {
        unimplemented!("Replaced by v2::io::schdoc::SchDocV2")
    }

    fn write_additional<F: Read + Write + Seek>(&self, _cf: &mut CompoundFile<F>) -> Result<()> {
        unimplemented!("Replaced by v2::io::schdoc::SchDocV2")
    }

    fn write_record<W: Write>(&self, _writer: &mut W, _record: &SchRecord) -> Result<()> {
        unimplemented!("Replaced by v2::io::schdoc::SchDocV2")
    }

    /// Get the sheet header if present.
    ///
    /// NOTE: With v1 IO stubbed, returns None since primitives Vec is never populated.
    /// Callers must use v2::io::schdoc::SchDocV2 to obtain actual sheet metadata.
    pub fn sheet_header(&self) -> Option<&SchSheetHeader> {
        self.primitives.iter().find_map(|r| {
            if let SchRecord::SheetHeader(h) = r {
                Some(h)
            } else {
                None
            }
        })
    }

    /// Get all components in the document.
    ///
    /// NOTE: With v1 IO stubbed, this always yields nothing since primitives Vec is never populated.
    pub fn components(&self) -> impl Iterator<Item = &crate::records::sch::SchComponent> {
        self.primitives.iter().filter_map(|r| {
            if let SchRecord::Component(c) = r {
                Some(c)
            } else {
                None
            }
        })
    }

    /// Get all wires in the document.
    ///
    /// NOTE: With v1 IO stubbed, this always yields nothing since primitives Vec is never populated.
    pub fn wires(&self) -> impl Iterator<Item = &crate::records::sch::SchWire> {
        self.primitives.iter().filter_map(|r| {
            if let SchRecord::Wire(w) = r {
                Some(w)
            } else {
                None
            }
        })
    }

    /// Get the number of primitives.
    ///
    /// NOTE: With v1 IO stubbed, this always returns 0 since primitives Vec is never populated.
    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
    }
}

#[allow(deprecated)]
impl SchRecord {
    /// Export record to parameters.
    pub fn export_to_params(&self) -> ParameterCollection {
        match self {
            SchRecord::Component(r) => r.export_to_params(),
            SchRecord::Pin(r) => r.export_to_params(),
            SchRecord::Symbol(r) => r.export_to_params(),
            SchRecord::Label(r) => r.export_to_params(),
            SchRecord::Bezier(r) => r.export_to_params(),
            SchRecord::Polyline(r) => r.export_to_params(),
            SchRecord::Polygon(r) => r.export_to_params(),
            SchRecord::Ellipse(r) => r.export_to_params(),
            SchRecord::Pie(r) => r.export_to_params(),
            SchRecord::EllipticalArc(r) => r.export_to_params(),
            SchRecord::Arc(r) => r.export_to_params(),
            SchRecord::Line(r) => r.export_to_params(),
            SchRecord::Rectangle(r) => r.export_to_params(),
            SchRecord::PowerObject(r) => r.export_to_params(),
            SchRecord::Port(r) => r.export_to_params(),
            SchRecord::NoErc(r) => r.export_to_params(),
            SchRecord::NetLabel(r) => r.export_to_params(),
            SchRecord::Bus(r) => r.export_to_params(),
            SchRecord::Wire(r) => r.export_to_params(),
            SchRecord::TextFrame(r) => r.export_to_params(),
            SchRecord::TextFrameVariant(r) => r.export_to_params(),
            SchRecord::Junction(r) => r.export_to_params(),
            SchRecord::Image(r) => r.export_to_params(),
            SchRecord::SheetHeader(r) => r.export_to_params(),
            SchRecord::Designator(r) => r.export_to_params(),
            SchRecord::BusEntry(r) => r.export_to_params(),
            SchRecord::Parameter(r) => r.export_to_params(),
            SchRecord::WarningSign(r) => r.export_to_params(),
            SchRecord::ImplementationList(r) => r.export_to_params(),
            SchRecord::Implementation(r) => r.export_to_params(),
            SchRecord::MapDefinerList(r) => r.export_to_params(),
            SchRecord::MapDefiner(r) => r.export_to_params(),
            SchRecord::ImplementationParameters(r) => r.export_to_params(),
            SchRecord::Unknown { record_id, params } => {
                let mut p = params.clone();
                p.add_int("RECORD", *record_id);
                p
            }
        }
    }
}

#[allow(deprecated)]
impl DumpTree for SchDoc {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.root(&format!("SchDoc ({} primitives)", self.primitives.len()));

        // Count by type
        let mut component_count = 0;
        let mut wire_count = 0;
        let mut other_count = 0;

        for prim in &self.primitives {
            match prim {
                SchRecord::Component(_) => component_count += 1,
                SchRecord::Wire(_) => wire_count += 1,
                _ => other_count += 1,
            }
        }

        // Summary
        tree.push(true);
        tree.add_leaf(
            "Summary",
            &[
                ("components", format!("{}", component_count)),
                ("wires", format!("{}", wire_count)),
                ("other", format!("{}", other_count)),
            ],
        );
        tree.pop();

        // Show primitives
        tree.push(false);
        tree.begin_node(&format!("Primitives ({})", self.primitives.len()));
        for (i, prim) in self.primitives.iter().enumerate() {
            tree.push(i < self.primitives.len() - 1);
            prim.dump(tree);
            tree.pop();
        }
        tree.pop();
    }
}

//! IntLib reader/writer for Altium Integrated Library files.
//!
//! **DEPRECATED**: V1 IO is replaced by v2. The embedded SchLib and PcbLib
//! use v2 equivalents. IntLib files are CFB containers that bundle:
//! - Embedded SchLib (zlib-compressed CFB)
//! - Embedded PcbLib (zlib-compressed CFB)
//! - Cross-reference mapping components to symbols and footprints
//! - Consolidated component parameters

#![allow(unused_imports)]
#![allow(dead_code)]

use cfb::CompoundFile;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;

use crate::error::{AltiumError, Result};
use crate::io::{PcbLib, SchLib};
use crate::types::ParameterCollection;

/// An integrated library containing schematic symbols and PCB footprints.
///
/// **DEPRECATED**: The embedded SchLib and PcbLib should use v2 equivalents.
/// Use v2::io::schlib::SchLibV2 and v2::pcb::io::pcblib::PcbLibV2 for the
/// embedded libraries.
#[deprecated(note = "Embedded SchLib/PcbLib use v2 equivalents")]
#[derive(Debug, Default)]
pub struct IntLib {
    /// Version of the IntLib format.
    pub version: u32,
    /// Embedded schematic library.
    #[allow(deprecated)]
    pub schlib: SchLib,
    /// Embedded PCB footprint library.
    #[allow(deprecated)]
    pub pcblib: PcbLib,
    /// Cross-reference entries mapping components to their symbols and footprints.
    pub cross_refs: Vec<CrossReference>,
    /// Component parameters (BOM data).
    pub parameters: Vec<ComponentParameters>,
}

/// Cross-reference entry linking a component to its symbol and footprint.
#[derive(Debug, Clone, Default)]
pub struct CrossReference {
    /// Component name.
    pub name: String,
    /// Schematic symbol library path (relative within IntLib).
    pub schlib_path: String,
    /// Description from the schematic symbol.
    pub description: String,
    /// Original source path.
    pub source_path: String,
    /// PCB footprint name.
    pub footprint: String,
    /// PCB library type (e.g., "PCBLIB").
    pub pcblib_type: String,
    /// PCB library path (relative within IntLib).
    pub pcblib_path: String,
    /// Original PCB library source path.
    pub pcblib_source_path: String,
}

/// Parameters for a component (BOM data).
#[derive(Debug, Clone)]
pub struct ComponentParameters {
    /// Component name.
    pub name: String,
    /// Key-value parameters.
    pub params: ParameterCollection,
}

#[allow(deprecated)]
impl IntLib {
    /// Open and read an IntLib file.
    ///
    /// **DEPRECATED**: Use v2 SchLib/PcbLib types for the embedded libraries.
    #[deprecated(note = "Embedded SchLib/PcbLib use v2 equivalents")]
    pub fn open<R: Read + Seek>(_reader: R) -> Result<Self> {
        unimplemented!(
            "Use v2::io::schlib::SchLibV2 and v2::pcb::io::pcblib::PcbLibV2 for embedded libraries - v1 API has been deprecated"
        )
    }

    /// Open and read an IntLib file from a path.
    ///
    /// **DEPRECATED**: Use v2 SchLib/PcbLib types for the embedded libraries.
    #[deprecated(note = "Embedded SchLib/PcbLib use v2 equivalents")]
    pub fn open_file<P: AsRef<Path>>(_path: P) -> Result<Self> {
        unimplemented!(
            "Use v2::io::schlib::SchLibV2 and v2::pcb::io::pcblib::PcbLibV2 for embedded libraries - v1 API has been deprecated"
        )
    }

    /// Save the IntLib to a file.
    ///
    /// **DEPRECATED**: Use v2 types instead.
    #[deprecated(note = "Embedded SchLib/PcbLib use v2 equivalents")]
    pub fn save<W: Read + Write + Seek>(&self, _writer: W) -> Result<()> {
        unimplemented!("Use v2 types - v1 API has been deprecated")
    }

    /// Save the IntLib to a file path.
    ///
    /// **DEPRECATED**: Use v2 types instead.
    #[deprecated(note = "Embedded SchLib/PcbLib use v2 equivalents")]
    pub fn save_to_file<P: AsRef<Path>>(&self, _path: P) -> Result<()> {
        unimplemented!("Use v2 types - v1 API has been deprecated")
    }

    // Internal methods stubbed

    fn read_version<R: Read + Seek>(&mut self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2 implementation")
    }

    fn write_version<F: Read + Write + Seek>(&self, _cf: &mut CompoundFile<F>) -> Result<()> {
        unimplemented!("Replaced by v2 implementation")
    }

    fn read_cross_refs<R: Read + Seek>(&mut self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2 implementation")
    }

    fn parse_cross_refs(&mut self, _data: &[u8]) -> Result<()> {
        unimplemented!("Replaced by v2 implementation")
    }

    fn read_block_string<R: Read>(_reader: &mut R) -> Result<String> {
        unimplemented!("Replaced by v2 implementation")
    }

    fn read_cross_ref_entry<R: Read>(&self, _reader: &mut R) -> Result<CrossReference> {
        unimplemented!("Replaced by v2 implementation")
    }

    fn write_cross_refs<F: Read + Write + Seek>(&self, _cf: &mut CompoundFile<F>) -> Result<()> {
        unimplemented!("Replaced by v2 implementation")
    }

    fn write_block_string<W: Write>(_writer: &mut W, _s: &str) -> Result<()> {
        unimplemented!("Replaced by v2 implementation")
    }

    fn write_cross_ref_entry<W: Write>(
        &self,
        _writer: &mut W,
        _entry: &CrossReference,
    ) -> Result<()> {
        unimplemented!("Replaced by v2 implementation")
    }

    fn read_parameters<R: Read + Seek>(&mut self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2 implementation")
    }

    fn parse_parameters(&mut self, _data: &[u8]) -> Result<()> {
        unimplemented!("Replaced by v2 implementation")
    }

    fn read_parameter_entry<R: Read>(&self, _reader: &mut R) -> Result<ComponentParameters> {
        unimplemented!("Replaced by v2 implementation")
    }

    fn write_parameters<F: Read + Write + Seek>(&self, _cf: &mut CompoundFile<F>) -> Result<()> {
        unimplemented!("Replaced by v2 implementation")
    }

    fn read_schlib<R: Read + Seek>(&mut self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2 implementation")
    }

    fn write_schlib<F: Read + Write + Seek>(&self, _cf: &mut CompoundFile<F>) -> Result<()> {
        unimplemented!("Replaced by v2 implementation")
    }

    fn read_pcblib<R: Read + Seek>(&mut self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2 implementation")
    }

    fn write_pcblib<F: Read + Write + Seek>(&self, _cf: &mut CompoundFile<F>) -> Result<()> {
        unimplemented!("Replaced by v2 implementation")
    }

    // Simple accessor methods - kept functional

    /// Get the number of schematic components.
    ///
    /// NOTE: With v1 IO stubbed, this always returns 0.
    pub fn schematic_component_count(&self) -> usize {
        self.schlib.component_count()
    }

    /// Get the number of PCB footprints.
    ///
    /// NOTE: With v1 IO stubbed, this always returns 0.
    pub fn footprint_count(&self) -> usize {
        self.pcblib.component_count()
    }

    /// Get cross-reference for a component by name.
    pub fn get_cross_ref(&self, name: &str) -> Option<&CrossReference> {
        self.cross_refs.iter().find(|r| r.name == name)
    }

    /// Get parameters for a component by name.
    pub fn get_parameters(&self, name: &str) -> Option<&ComponentParameters> {
        self.parameters.iter().find(|p| p.name == name)
    }

    /// Get a mapping of component names to their footprints.
    pub fn component_footprint_map(&self) -> HashMap<String, String> {
        self.cross_refs
            .iter()
            .map(|r| (r.name.clone(), r.footprint.clone()))
            .collect()
    }
}

// DumpTree implementation
use crate::dump::{DumpTree, TreeBuilder};

#[allow(deprecated)]
impl DumpTree for IntLib {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.root(&format!(
            "IntLib (v{}, {} symbols, {} footprints)",
            self.version,
            self.schematic_component_count(),
            self.footprint_count()
        ));

        // Cross-references summary
        tree.push(true);
        tree.begin_node(&format!("Cross-References ({})", self.cross_refs.len()));
        for (i, xref) in self.cross_refs.iter().enumerate() {
            tree.push(i < self.cross_refs.len() - 1);
            let props = vec![
                ("symbol", xref.name.clone()),
                ("footprint", xref.footprint.clone()),
                ("description", xref.description.clone()),
            ];
            tree.add_leaf(&xref.name, &props);
            tree.pop();
        }
        tree.pop();

        // SchLib section
        tree.push(true);
        tree.begin_node(&format!(
            "SchLib ({} components)",
            self.schlib.component_count()
        ));
        for (i, comp) in self.schlib.iter().enumerate() {
            tree.push(i < self.schlib.component_count() - 1);
            let props = vec![
                ("name", comp.name().to_string()),
                ("pins", format!("{}", comp.pin_count())),
            ];
            tree.add_leaf(comp.name(), &props);
            tree.pop();
        }
        tree.pop();

        // PcbLib section
        tree.push(false);
        tree.begin_node(&format!(
            "PcbLib ({} footprints)",
            self.pcblib.component_count()
        ));
        for (i, comp) in self.pcblib.iter().enumerate() {
            tree.push(i < self.pcblib.component_count() - 1);
            let props = vec![
                ("name", comp.pattern.clone()),
                ("pads", format!("{}", comp.pad_count())),
            ];
            tree.add_leaf(&comp.pattern, &props);
            tree.pop();
        }
        tree.pop();
    }
}

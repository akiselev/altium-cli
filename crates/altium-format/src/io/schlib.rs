//! SchLib reader/writer for Altium schematic library files.
//!
//! **DEPRECATED**: V1 IO is replaced by v2 with proper field deserialization.
//! V1 has coordinate scale bugs and unsafe field parsing. Use v2::io::schlib::SchLibV2.

#![allow(unused_imports)]
#![allow(dead_code)]

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use cfb::CompoundFile;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;

use crate::error::{AltiumError, Result};
use crate::format::SIZE_FLAG_MASK;
use crate::io::reader::{
    decode_windows_1252, read_parameters_block, read_pascal_short_string, read_string_block,
};
use crate::io::writer::{write_block, write_parameters, write_pascal_short_string};
use crate::records::sch::{
    PinConglomerateFlags, PinElectricalType, PinSymbol, SchComponent, SchGraphicalBase, SchPin,
    SchRecord, coord_to_dxp_frac, dxp_frac_to_coord,
};
use crate::types::ParameterCollection;

/// A schematic library containing components.
///
/// **DEPRECATED**: Use `v2::io::schlib::SchLibV2` instead.
/// V1 has coordinate scale bugs (uses 10K units/mil instead of 100K) and field type mismatches.
#[deprecated(note = "Use v2::io::schlib::SchLibV2")]
#[derive(Debug, Default)]
pub struct SchLib {
    /// Section keys mapping LIBREF to storage path.
    section_keys: HashMap<String, String>,
    /// Components in the library.
    pub components: Vec<SchLibComponent>,
    /// Raw FileHeader parameters (fonts, grid, sheet settings).
    /// Preserved for round-trip fidelity. Excludes HEADER, WEIGHT,
    /// COMPCOUNT, LIBREF*, PARTCOUNT* (which are regenerated on save).
    pub header_params: ParameterCollection,
}

/// A component in the schematic library.
///
/// **DEPRECATED**: Use `v2::io::schlib::SchLibComponent` instead.
#[deprecated(note = "Use v2::io::schlib::SchLibComponent")]
#[derive(Debug)]
pub struct SchLibComponent {
    /// Component data record.
    pub component: SchComponent,
    /// All primitives belonging to this component.
    pub primitives: Vec<SchRecord>,
}

#[allow(deprecated)]
impl SchLib {
    /// Open and read a SchLib file.
    ///
    /// **DEPRECATED**: Use `v2::io::schlib::SchLibV2::open()` instead.
    #[deprecated(note = "Use v2::io::schlib::SchLibV2::open()")]
    pub fn open<R: Read + Seek>(_reader: R) -> Result<Self> {
        unimplemented!("Use v2::io::schlib::SchLibV2::open() - v1 API has been deprecated")
    }

    /// Open and read a SchLib file from a path.
    ///
    /// **DEPRECATED**: Use `v2::io::schlib::SchLibV2::open_file()` instead.
    #[deprecated(note = "Use v2::io::schlib::SchLibV2::open_file()")]
    pub fn open_file<P: AsRef<Path>>(_path: P) -> Result<Self> {
        unimplemented!("Use v2::io::schlib::SchLibV2::open_file() - v1 API has been deprecated")
    }

    /// Save the SchLib to a file.
    ///
    /// **DEPRECATED**: Use `v2::io::schlib::SchLibV2::write()` instead.
    #[deprecated(note = "Use v2::io::schlib::SchLibV2::write()")]
    pub fn save<W: Read + Write + Seek>(&self, _writer: W) -> Result<()> {
        unimplemented!("Use v2::io::schlib::SchLibV2::write() - v1 API has been deprecated")
    }

    /// Save the SchLib to a file path.
    ///
    /// **DEPRECATED**: Use `v2::io::schlib::SchLibV2::write_to_file()` instead.
    #[deprecated(note = "Use v2::io::schlib::SchLibV2::write_to_file()")]
    pub fn save_to_file<P: AsRef<Path>>(&self, _path: P) -> Result<()> {
        unimplemented!("Use v2::io::schlib::SchLibV2::write_to_file() - v1 API has been deprecated")
    }

    // Internal methods stubbed to prevent accidental usage.
    // V2 implementation handles section key mapping and component parsing.

    fn write_storage<F: Read + Write + Seek>(&self, _cf: &mut CompoundFile<F>) -> Result<()> {
        unimplemented!("Replaced by v2::io::schlib::SchLibV2")
    }

    fn write_file_header<F: Read + Write + Seek>(&self, _cf: &mut CompoundFile<F>) -> Result<()> {
        unimplemented!("Replaced by v2::io::schlib::SchLibV2")
    }

    fn write_section_keys<F: Read + Write + Seek>(&self, _cf: &mut CompoundFile<F>) -> Result<()> {
        unimplemented!("Replaced by v2::io::schlib::SchLibV2")
    }

    fn write_alias_redirections<F: Read + Write + Seek>(
        &self,
        _cf: &mut CompoundFile<F>,
    ) -> Result<()> {
        unimplemented!("Replaced by v2::io::schlib::SchLibV2")
    }

    fn needs_section_key(_name: &str) -> bool {
        unimplemented!("Replaced by v2::io::schlib::SchLibV2")
    }

    fn get_section_key_for(_name: &str) -> String {
        unimplemented!("Replaced by v2::io::schlib::SchLibV2")
    }

    fn write_component<F: Read + Write + Seek>(
        &self,
        _cf: &mut CompoundFile<F>,
        _comp: &SchLibComponent,
    ) -> Result<()> {
        unimplemented!("Replaced by v2::io::schlib::SchLibV2")
    }

    fn write_record<W: Write>(&self, _writer: &mut W, _record: &SchRecord) -> Result<()> {
        unimplemented!("Replaced by v2::io::schlib::SchLibV2")
    }

    fn write_binary_pin<W: Write>(&self, _writer: &mut W, _pin: &SchPin) -> Result<()> {
        unimplemented!("Replaced by v2::io::schlib::SchLibV2")
    }

    fn get_section_key(&self, _ref_name: &str) -> String {
        unimplemented!("Replaced by v2::io::schlib::SchLibV2")
    }

    fn read_section_keys<R: Read + Seek>(&mut self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::io::schlib::SchLibV2")
    }

    fn read_file_header<R: Read + Seek>(
        &mut self,
        _cf: &mut CompoundFile<R>,
    ) -> Result<Vec<String>> {
        unimplemented!("Replaced by v2::io::schlib::SchLibV2")
    }

    fn is_component_index_key(_key: &str) -> bool {
        unimplemented!("Replaced by v2::io::schlib::SchLibV2")
    }

    fn read_component<R: Read + Seek>(
        &self,
        _cf: &mut CompoundFile<R>,
        _section_key: &str,
    ) -> Result<SchLibComponent> {
        unimplemented!("Replaced by v2::io::schlib::SchLibV2")
    }

    fn read_record<R: Read>(&self, _reader: &mut R) -> Result<SchRecord> {
        unimplemented!("Replaced by v2::io::schlib::SchLibV2")
    }

    fn read_binary_pin<R: Read>(&self, _reader: &mut R) -> Result<SchRecord> {
        unimplemented!("Replaced by v2::io::schlib::SchLibV2")
    }

    /// Get the number of components.
    ///
    /// NOTE: With v1 IO stubbed, this always returns 0 since components Vec is never populated.
    /// Callers must use v2::io::schlib::SchLibV2 to obtain actual component data.
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Iterate over components.
    ///
    /// NOTE: With v1 IO stubbed, this always yields nothing since components Vec is never populated.
    /// Callers must use v2::io::schlib::SchLibV2 to obtain actual component data.
    pub fn iter(&self) -> impl Iterator<Item = &SchLibComponent> {
        self.components.iter()
    }
}

#[allow(deprecated)]
impl SchLibComponent {
    /// Get the component name (LIBREFERENCE).
    pub fn name(&self) -> &str {
        &self.component.lib_reference
    }

    /// Get the component description.
    pub fn description(&self) -> &str {
        &self.component.component_description
    }

    /// Get the number of pins.
    pub fn pin_count(&self) -> usize {
        self.primitives
            .iter()
            .filter(|r| matches!(r, SchRecord::Pin(_)))
            .count()
    }

    /// Get total primitive count.
    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
    }
}

// DumpTree implementations
use crate::dump::{DumpTree, TreeBuilder};

#[allow(deprecated)]
impl DumpTree for SchLib {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.root(&format!("SchLib ({} components)", self.components.len()));

        for (i, comp) in self.components.iter().enumerate() {
            tree.push(i < self.components.len() - 1);
            comp.dump(tree);
            tree.pop();
        }
    }
}

#[allow(deprecated)]
impl DumpTree for SchLibComponent {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.begin_node(&format!("Symbol: {}", self.component.lib_reference));
        tree.push(true);

        // Metadata section
        tree.push(self.primitives.len() > 1);
        let mut meta_props = vec![];
        if !self.component.component_description.is_empty() {
            meta_props.push(("description", self.component.component_description.clone()));
        }
        meta_props.push(("parts", format!("{}", self.component.part_count)));
        meta_props.push(("pins", format!("{}", self.pin_count())));
        meta_props.push(("primitives", format!("{}", self.primitive_count())));
        tree.add_leaf("Info", &meta_props);
        tree.pop();

        // Primitives section (skip first which is the component itself)
        let child_primitives: Vec<_> = self.primitives.iter().skip(1).collect();
        if !child_primitives.is_empty() {
            tree.push(false);
            tree.begin_node(&format!("Primitives ({})", child_primitives.len()));
            for (i, prim) in child_primitives.iter().enumerate() {
                tree.push(i < child_primitives.len() - 1);
                prim.dump(tree);
                tree.pop();
            }
            tree.pop();
        }

        tree.pop();
    }
}

//! PcbLib reader/writer for Altium PCB footprint library files.
//!
//! **DEPRECATED**: V1 IO is replaced by v2 with correct coordinate scale.
//! V1 uses 1 unit/mil (incorrect); v2 uses 10K units/mil. Use v2::pcb::io::pcblib::PcbLibV2.

#![allow(unused_imports)]
#![allow(dead_code)]

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use cfb::CompoundFile;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;

use crate::error::{AltiumError, Result};
use crate::io::reader::{
    read_block, read_parameters_block, read_pascal_short_string, read_string_block,
};
use crate::io::writer::{
    write_block, write_parameters, write_pascal_short_string, write_string_block,
};
use crate::records::pcb::{
    PcbArc, PcbComponent, PcbComponentBody, PcbFill, PcbObjectId, PcbPad, PcbRecord, PcbRegion,
    PcbText, PcbTrack, PcbVia,
};
use crate::traits::{FromBinary, ToBinary};
use crate::types::ParameterCollection;

/// A PCB footprint library containing components.
///
/// **DEPRECATED**: Use `v2::pcb::io::pcblib::PcbLibV2` instead.
/// V1 has coordinate scale bugs (uses 1 unit/mil instead of 10K units/mil).
#[deprecated(note = "Use v2::pcb::io::pcblib::PcbLibV2")]
#[derive(Debug, Default)]
pub struct PcbLib {
    /// Section keys mapping pattern names to storage paths.
    section_keys: HashMap<String, String>,
    /// Unique ID of the library.
    pub unique_id: String,
    /// Components (footprints) in the library.
    pub components: Vec<PcbComponent>,
    /// Library parameters from Library/Data stream (board config, layer settings, grid settings).
    /// When present, these are written back verbatim instead of using build_library_parameters().
    pub library_parameters: Option<ParameterCollection>,
    /// FileHeader version string (e.g., "PCB 6.0 Binary Library File").
    pub file_header_version: String,
    /// FileHeader optional field 1 (version-related float string).
    pub file_header_field1: String,
    /// FileHeader optional field 2 (token/marker string, e.g., "DVLTOKCO").
    pub file_header_field2: String,
}

#[allow(deprecated)]
impl PcbLib {
    /// Open and read a PcbLib file.
    ///
    /// **DEPRECATED**: Use `v2::pcb::io::pcblib::PcbLibV2::open()` instead.
    #[deprecated(note = "Use v2::pcb::io::pcblib::PcbLibV2::open()")]
    pub fn open<R: Read + Seek>(_reader: R) -> Result<Self> {
        unimplemented!("Use v2::pcb::io::pcblib::PcbLibV2::open() - v1 API has been deprecated")
    }

    /// Open and read a PcbLib file from a path.
    ///
    /// **DEPRECATED**: Use `v2::pcb::io::pcblib::PcbLibV2::open_file()` instead.
    #[deprecated(note = "Use v2::pcb::io::pcblib::PcbLibV2::open_file()")]
    pub fn open_file<P: AsRef<Path>>(_path: P) -> Result<Self> {
        unimplemented!("Use v2::pcb::io::pcblib::PcbLibV2::open_file() - v1 API has been deprecated")
    }

    /// Save the PcbLib to a file.
    ///
    /// **DEPRECATED**: Use `v2::pcb::io::pcblib::PcbLibV2::write()` instead.
    #[deprecated(note = "Use v2::pcb::io::pcblib::PcbLibV2::write()")]
    pub fn save<W: Read + Write + Seek>(&self, _writer: W) -> Result<()> {
        unimplemented!("Use v2::pcb::io::pcblib::PcbLibV2::write() - v1 API has been deprecated")
    }

    /// Save the PcbLib to a file path.
    ///
    /// **DEPRECATED**: Use `v2::pcb::io::pcblib::PcbLibV2::write_to_file()` instead.
    #[deprecated(note = "Use v2::pcb::io::pcblib::PcbLibV2::write_to_file()")]
    pub fn save_to_file<P: AsRef<Path>>(&self, _path: P) -> Result<()> {
        unimplemented!(
            "Use v2::pcb::io::pcblib::PcbLibV2::write_to_file() - v1 API has been deprecated"
        )
    }

    // Internal methods stubbed to prevent accidental usage.

    fn write_file_header<F: Read + Write + Seek>(&self, _cf: &mut CompoundFile<F>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcblib::PcbLibV2")
    }

    fn storage_name_for(_pattern: &str) -> String {
        unimplemented!("Replaced by v2::pcb::io::pcblib::PcbLibV2")
    }

    fn write_section_keys<F: Read + Write + Seek>(&self, _cf: &mut CompoundFile<F>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcblib::PcbLibV2")
    }

    fn write_library<F: Read + Write + Seek>(&self, _cf: &mut CompoundFile<F>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcblib::PcbLibV2")
    }

    fn write_library_header<F: Read + Write + Seek>(
        &self,
        _cf: &mut CompoundFile<F>,
    ) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcblib::PcbLibV2")
    }

    fn write_library_data<F: Read + Write + Seek>(&self, _cf: &mut CompoundFile<F>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcblib::PcbLibV2")
    }

    fn build_library_parameters() -> ParameterCollection {
        unimplemented!("Replaced by v2::pcb::io::pcblib::PcbLibV2")
    }

    fn write_library_substorages<F: Read + Write + Seek>(
        &self,
        _cf: &mut CompoundFile<F>,
    ) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcblib::PcbLibV2")
    }

    fn write_file_version_info<F: Read + Write + Seek>(
        &self,
        _cf: &mut CompoundFile<F>,
    ) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcblib::PcbLibV2")
    }

    fn write_footprint<F: Read + Write + Seek>(
        &self,
        _cf: &mut CompoundFile<F>,
        _comp: &PcbComponent,
        _storage_name: &str,
    ) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcblib::PcbLibV2")
    }

    fn write_primitive<W: Write>(&self, _writer: &mut W, _record: &PcbRecord) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcblib::PcbLibV2")
    }

    fn get_section_key(&self, _ref_name: &str) -> String {
        unimplemented!("Replaced by v2::pcb::io::pcblib::PcbLibV2")
    }

    fn read_file_header<R: Read + Seek>(&mut self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcblib::PcbLibV2")
    }

    fn read_section_keys<R: Read + Seek>(&mut self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcblib::PcbLibV2")
    }

    fn read_library<R: Read + Seek>(&mut self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcblib::PcbLibV2")
    }

    fn read_footprint<R: Read + Seek>(
        &self,
        _cf: &mut CompoundFile<R>,
        _section_key: &str,
    ) -> Result<PcbComponent> {
        unimplemented!("Replaced by v2::pcb::io::pcblib::PcbLibV2")
    }

    fn read_wide_strings<R: Read + Seek>(
        &self,
        _cf: &mut CompoundFile<R>,
        _storage_path: &str,
    ) -> Result<Vec<String>> {
        unimplemented!("Replaced by v2::pcb::io::pcblib::PcbLibV2")
    }

    fn read_primitive(
        &self,
        _cursor: &mut Cursor<&Vec<u8>>,
        _wide_strings: &[String],
    ) -> Result<PcbRecord> {
        unimplemented!("Replaced by v2::pcb::io::pcblib::PcbLibV2")
    }

    /// Get the number of components.
    ///
    /// NOTE: With v1 IO stubbed, this always returns 0 since components Vec is never populated.
    /// Callers must use v2::pcb::io::pcblib::PcbLibV2 to obtain actual component data.
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Iterate over components.
    ///
    /// NOTE: With v1 IO stubbed, this always yields nothing since components Vec is never populated.
    /// Callers must use v2::pcb::io::pcblib::PcbLibV2 to obtain actual component data.
    pub fn iter(&self) -> impl Iterator<Item = &PcbComponent> {
        self.components.iter()
    }
}

// DumpTree implementation
use crate::dump::{DumpTree, TreeBuilder};

#[allow(deprecated)]
impl DumpTree for PcbLib {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.root(&format!("PcbLib ({} footprints)", self.components.len()));

        for (i, comp) in self.components.iter().enumerate() {
            tree.push(i < self.components.len() - 1);
            comp.dump(tree);
            tree.pop();
        }
    }
}

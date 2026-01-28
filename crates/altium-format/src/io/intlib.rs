//! IntLib reader/writer for Altium Integrated Library files.
//!
//! IntLib files are CFB containers that bundle:
//! - Embedded SchLib (zlib-compressed CFB)
//! - Embedded PcbLib (zlib-compressed CFB)
//! - Cross-reference mapping components to symbols and footprints
//! - Consolidated component parameters

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
#[derive(Debug, Default)]
pub struct IntLib {
    /// Version of the IntLib format.
    pub version: u32,
    /// Embedded schematic library.
    pub schlib: SchLib,
    /// Embedded PCB footprint library.
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

impl IntLib {
    /// Open and read an IntLib file.
    pub fn open<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut intlib = IntLib::default();
        let mut cf = CompoundFile::open(reader).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })?;

        // Read version
        intlib.read_version(&mut cf)?;

        // Read cross-references
        intlib.read_cross_refs(&mut cf)?;

        // Read parameters
        intlib.read_parameters(&mut cf)?;

        // Read embedded SchLib
        intlib.read_schlib(&mut cf)?;

        // Read embedded PcbLib
        intlib.read_pcblib(&mut cf)?;

        Ok(intlib)
    }

    /// Open and read an IntLib file from a path.
    pub fn open_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        Self::open(file)
    }

    /// Save the IntLib to a file.
    pub fn save<W: Read + Write + Seek>(&self, writer: W) -> Result<()> {
        let mut cf = CompoundFile::create_with_version(cfb::Version::V3, writer)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        // Write version
        self.write_version(&mut cf)?;

        // Write cross-references
        self.write_cross_refs(&mut cf)?;

        // Write parameters
        self.write_parameters(&mut cf)?;

        // Write embedded SchLib
        self.write_schlib(&mut cf)?;

        // Write embedded PcbLib
        self.write_pcblib(&mut cf)?;

        cf.flush()
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Save the IntLib to a file path.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        self.save(file)
    }

    /// Read the version stream.
    fn read_version<R: Read + Seek>(&mut self, cf: &mut CompoundFile<R>) -> Result<()> {
        let stream_path = "/Version.Txt";
        if cf.entry(stream_path).is_err() {
            return Ok(());
        }

        let mut stream = cf.open_stream(stream_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        let mut data = Vec::new();
        stream.read_to_end(&mut data)?;

        if data.len() >= 5 {
            // Format: [0x00, version_low, version_high, 0x00, 0x00]
            self.version = data[1] as u32 | ((data[2] as u32) << 8);
        }

        Ok(())
    }

    /// Write the version stream.
    fn write_version<F: Read + Write + Seek>(&self, cf: &mut CompoundFile<F>) -> Result<()> {
        let mut data = vec![0u8; 5];
        data[1] = (self.version & 0xFF) as u8;
        data[2] = ((self.version >> 8) & 0xFF) as u8;

        let stream = cf
            .create_stream("/Version.Txt")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        let mut stream = stream;
        stream.write_all(&data)?;

        Ok(())
    }

    /// Read the cross-reference stream.
    fn read_cross_refs<R: Read + Seek>(&mut self, cf: &mut CompoundFile<R>) -> Result<()> {
        let stream_path = "/LibCrossRef.Txt";
        if cf.entry(stream_path).is_err() {
            return Ok(());
        }

        let mut stream = cf.open_stream(stream_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        let mut data = Vec::new();
        stream.read_to_end(&mut data)?;

        if data.is_empty() {
            return Ok(());
        }

        // Decompress: first byte is 0x02, rest is zlib
        if data.len() > 1 && data[0] == 0x02 {
            let decompressed = decompress_zlib(&data[1..])?;
            self.parse_cross_refs(&decompressed)?;
        }

        Ok(())
    }

    /// Parse cross-reference data.
    ///
    /// The format is:
    /// - 4-byte entry count
    /// - For each entry, 11 blocks:
    ///   0: name, 1: schlib_path, 2: empty, 3: description, 4: schlib_source,
    ///   5: empty, 6: footprint, 7: pcblib_type, 8: empty, 9: pcblib_path, 10: pcblib_source
    ///
    /// Each block is:
    /// - 4-byte size
    /// - If size <= 1: empty (no content)
    /// - If size > 1: 1-byte length + string
    fn parse_cross_refs(&mut self, data: &[u8]) -> Result<()> {
        use byteorder::{LittleEndian, ReadBytesExt};

        if data.len() < 4 {
            return Ok(());
        }

        let mut cursor = Cursor::new(data);

        // Read entry count
        let entry_count = cursor.read_u32::<LittleEndian>()? as usize;

        // Read each entry (11 blocks per entry)
        for _ in 0..entry_count {
            match self.read_cross_ref_entry(&mut cursor) {
                Ok(entry) => {
                    if !entry.name.is_empty() {
                        self.cross_refs.push(entry);
                    }
                }
                Err(_) => break,
            }
        }

        Ok(())
    }

    /// Read a string block.
    ///
    /// Format:
    /// - 4-byte size
    /// - If size <= 1: empty (no content bytes)
    /// - If size > 1: 1-byte length + string (total = size bytes)
    fn read_block_string<R: Read>(reader: &mut R) -> Result<String> {
        use byteorder::{LittleEndian, ReadBytesExt};

        let block_size = reader.read_u32::<LittleEndian>()? as usize;

        // Empty block - no content
        if block_size <= 1 {
            return Ok(String::new());
        }

        // Read the string length (1 byte)
        let str_len = reader.read_u8()? as usize;
        if str_len == 0 {
            // Skip remaining bytes in block
            let remaining = block_size.saturating_sub(1);
            if remaining > 0 {
                let mut skip = vec![0u8; remaining];
                let _ = reader.read_exact(&mut skip);
            }
            return Ok(String::new());
        }

        // Read string
        let mut buf = vec![0u8; str_len];
        reader.read_exact(&mut buf)?;

        Ok(String::from_utf8_lossy(&buf).to_string())
    }

    /// Read a single cross-reference entry (11 blocks).
    fn read_cross_ref_entry<R: Read>(&self, reader: &mut R) -> Result<CrossReference> {
        // Block 0: name
        let name = Self::read_block_string(reader)?;
        // Block 1: schlib_path (relative)
        let schlib_path = Self::read_block_string(reader)?;
        // Block 2: empty
        let _ = Self::read_block_string(reader)?;
        // Block 3: description
        let description = Self::read_block_string(reader)?;
        // Block 4: schlib_source (absolute path)
        let source_path = Self::read_block_string(reader)?;
        // Block 5: empty
        let _ = Self::read_block_string(reader)?;
        // Block 6: footprint
        let footprint = Self::read_block_string(reader)?;
        // Block 7: pcblib_type
        let pcblib_type = Self::read_block_string(reader)?;
        // Block 8: empty
        let _ = Self::read_block_string(reader)?;
        // Block 9: pcblib_path (relative)
        let pcblib_path = Self::read_block_string(reader)?;
        // Block 10: pcblib_source (absolute path)
        let pcblib_source_path = Self::read_block_string(reader)?;

        Ok(CrossReference {
            name,
            schlib_path,
            description,
            source_path,
            footprint,
            pcblib_type,
            pcblib_path,
            pcblib_source_path,
        })
    }

    /// Write cross-reference stream.
    fn write_cross_refs<F: Read + Write + Seek>(&self, cf: &mut CompoundFile<F>) -> Result<()> {
        if self.cross_refs.is_empty() {
            return Ok(());
        }

        use byteorder::{LittleEndian, WriteBytesExt};

        let mut data = Vec::new();

        // Write entry count
        data.write_u32::<LittleEndian>(self.cross_refs.len() as u32)?;

        // Write each entry
        for entry in &self.cross_refs {
            self.write_cross_ref_entry(&mut data, entry)?;
        }

        let compressed = compress_zlib(&data)?;
        let mut final_data = vec![0x02u8];
        final_data.extend(compressed);

        let stream = cf
            .create_stream("/LibCrossRef.Txt")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        let mut stream = stream;
        stream.write_all(&final_data)?;

        Ok(())
    }

    /// Write a string block.
    fn write_block_string<W: Write>(writer: &mut W, s: &str) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        if s.is_empty() {
            // Empty block - just size field with value 1
            writer.write_u32::<LittleEndian>(1)?;
        } else {
            // Normal block - size + length + string
            let bytes = s.as_bytes();
            let block_size = 1 + bytes.len(); // 1 for length byte + string
            writer.write_u32::<LittleEndian>(block_size as u32)?;
            writer.write_u8(bytes.len() as u8)?;
            writer.write_all(bytes)?;
        }
        Ok(())
    }

    /// Write a single cross-reference entry (11 blocks).
    fn write_cross_ref_entry<W: Write>(
        &self,
        writer: &mut W,
        entry: &CrossReference,
    ) -> Result<()> {
        // Block 0: name
        Self::write_block_string(writer, &entry.name)?;
        // Block 1: schlib_path
        Self::write_block_string(writer, &entry.schlib_path)?;
        // Block 2: empty
        Self::write_block_string(writer, "")?;
        // Block 3: description
        Self::write_block_string(writer, &entry.description)?;
        // Block 4: schlib_source
        Self::write_block_string(writer, &entry.source_path)?;
        // Block 5: empty
        Self::write_block_string(writer, "")?;
        // Block 6: footprint
        Self::write_block_string(writer, &entry.footprint)?;
        // Block 7: pcblib_type
        Self::write_block_string(writer, &entry.pcblib_type)?;
        // Block 8: empty
        Self::write_block_string(writer, "")?;
        // Block 9: pcblib_path
        Self::write_block_string(writer, &entry.pcblib_path)?;
        // Block 10: pcblib_source
        Self::write_block_string(writer, &entry.pcblib_source_path)?;

        Ok(())
    }

    /// Read the parameters stream.
    fn read_parameters<R: Read + Seek>(&mut self, cf: &mut CompoundFile<R>) -> Result<()> {
        // The stream name has spaces: "Parameters   .bin"
        let stream_path = "/Parameters   .bin";
        if cf.entry(stream_path).is_err() {
            return Ok(());
        }

        let mut stream = cf.open_stream(stream_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        let mut data = Vec::new();
        stream.read_to_end(&mut data)?;

        if data.is_empty() {
            return Ok(());
        }

        // Decompress: first byte is 0x02, rest is zlib
        if data.len() > 1 && data[0] == 0x02 {
            let decompressed = decompress_zlib(&data[1..])?;
            self.parse_parameters(&decompressed)?;
        }

        Ok(())
    }

    /// Parse parameters data.
    fn parse_parameters(&mut self, data: &[u8]) -> Result<()> {
        // Format: length-prefixed parameter blocks separated by some bytes
        // Each block is like: "Key1=Value1|Key2=Value2|..."
        let mut cursor = Cursor::new(data);

        while (cursor.position() as usize) < data.len() {
            match self.read_parameter_entry(&mut cursor) {
                Ok(entry) => self.parameters.push(entry),
                Err(_) => break,
            }
        }

        Ok(())
    }

    /// Read a single parameter entry.
    fn read_parameter_entry<R: Read>(&self, reader: &mut R) -> Result<ComponentParameters> {
        use byteorder::{LittleEndian, ReadBytesExt};

        // Read length (u16)
        let len = reader.read_u16::<LittleEndian>()? as usize;
        if len == 0 {
            return Err(AltiumError::Parse("Empty parameter block".to_string()));
        }

        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf)?;

        // Parse as pipe-delimited parameters
        let text = String::from_utf8_lossy(&buf).to_string();
        let params = ParameterCollection::from_string(&text);

        // Extract component name from Library Reference or Designator
        let name = params
            .get("Library Reference")
            .or_else(|| params.get("Designator"))
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();

        Ok(ComponentParameters { name, params })
    }

    /// Write parameters stream.
    fn write_parameters<F: Read + Write + Seek>(&self, cf: &mut CompoundFile<F>) -> Result<()> {
        if self.parameters.is_empty() {
            return Ok(());
        }

        use byteorder::{LittleEndian, WriteBytesExt};

        let mut data = Vec::new();
        for entry in &self.parameters {
            let param_str = entry.params.to_string();
            let bytes = param_str.as_bytes();
            data.write_u16::<LittleEndian>(bytes.len() as u16)?;
            data.write_all(bytes)?;
        }

        let compressed = compress_zlib(&data)?;
        let mut final_data = vec![0x02u8];
        final_data.extend(compressed);

        let stream = cf
            .create_stream("/Parameters   .bin")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        let mut stream = stream;
        stream.write_all(&final_data)?;

        Ok(())
    }

    /// Read the embedded SchLib.
    fn read_schlib<R: Read + Seek>(&mut self, cf: &mut CompoundFile<R>) -> Result<()> {
        let stream_path = "/SchLib/0.schlib";
        if cf.entry(stream_path).is_err() {
            return Ok(());
        }

        let mut stream = cf.open_stream(stream_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        let mut data = Vec::new();
        stream.read_to_end(&mut data)?;

        if data.is_empty() {
            return Ok(());
        }

        // Decompress: first byte is 0x02, rest is zlib
        if data.len() > 1 && data[0] == 0x02 {
            let decompressed = decompress_zlib(&data[1..])?;
            // Parse as SchLib CFB
            let cursor = Cursor::new(decompressed);
            self.schlib = SchLib::open(cursor)?;
        }

        Ok(())
    }

    /// Write the embedded SchLib.
    fn write_schlib<F: Read + Write + Seek>(&self, cf: &mut CompoundFile<F>) -> Result<()> {
        // Create SchLib storage
        cf.create_storage("/SchLib")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        // Write SchLib to a buffer
        let mut schlib_buf = Cursor::new(Vec::new());
        self.schlib.save(&mut schlib_buf)?;

        // Compress and write
        let compressed = compress_zlib(schlib_buf.get_ref())?;
        let mut final_data = vec![0x02u8];
        final_data.extend(compressed);

        let stream = cf
            .create_stream("/SchLib/0.schlib")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        let mut stream = stream;
        stream.write_all(&final_data)?;

        Ok(())
    }

    /// Read the embedded PcbLib.
    fn read_pcblib<R: Read + Seek>(&mut self, cf: &mut CompoundFile<R>) -> Result<()> {
        let stream_path = "/PCBLib/0.pcblib";
        if cf.entry(stream_path).is_err() {
            return Ok(());
        }

        let mut stream = cf.open_stream(stream_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        let mut data = Vec::new();
        stream.read_to_end(&mut data)?;

        if data.is_empty() {
            return Ok(());
        }

        // Decompress: first byte is 0x02, rest is zlib
        if data.len() > 1 && data[0] == 0x02 {
            let decompressed = decompress_zlib(&data[1..])?;
            // Parse as PcbLib CFB
            let cursor = Cursor::new(decompressed);
            self.pcblib = PcbLib::open(cursor)?;
        }

        Ok(())
    }

    /// Write the embedded PcbLib.
    fn write_pcblib<F: Read + Write + Seek>(&self, cf: &mut CompoundFile<F>) -> Result<()> {
        // Create PcbLib storage
        cf.create_storage("/PCBLib")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        // Write PcbLib to a buffer
        let mut pcblib_buf = Cursor::new(Vec::new());
        self.pcblib.save(&mut pcblib_buf)?;

        // Compress and write
        let compressed = compress_zlib(pcblib_buf.get_ref())?;
        let mut final_data = vec![0x02u8];
        final_data.extend(compressed);

        let stream = cf
            .create_stream("/PCBLib/0.pcblib")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        let mut stream = stream;
        stream.write_all(&final_data)?;

        Ok(())
    }

    /// Get the number of schematic components.
    pub fn schematic_component_count(&self) -> usize {
        self.schlib.component_count()
    }

    /// Get the number of PCB footprints.
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

/// Decompress zlib data.
fn decompress_zlib(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).map_err(|e| {
        AltiumError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("zlib decompress failed: {}", e),
        ))
    })?;
    Ok(decompressed)
}

/// Compress data with zlib.
fn compress_zlib(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    encoder.finish().map_err(|e| {
        AltiumError::Io(std::io::Error::other(format!(
            "zlib compress failed: {}",
            e
        )))
    })
}

// DumpTree implementation
use crate::dump::{DumpTree, TreeBuilder};

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

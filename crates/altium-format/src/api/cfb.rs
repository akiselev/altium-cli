//! Layer 1: CFB wrapper with Altium-specific convenience methods.
//!
//! Provides low-level access to OLE Compound Document files used by Altium,
//! with helpers for stream enumeration, block parsing, and reverse engineering.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Cursor, Read, Seek};
use std::path::Path;

use byteorder::{LittleEndian, ReadBytesExt};
use cfb::CompoundFile;

use crate::error::{AltiumError, Result};
use crate::io::reader::{decompress_zlib, read_string_block};
use crate::types::ParameterCollection;

/// Detected Altium file type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AltiumFileType {
    /// Schematic symbol library (.SchLib)
    SchLib,
    /// Schematic document (.SchDoc)
    SchDoc,
    /// PCB footprint library (.PcbLib)
    PcbLib,
    /// PCB document (.PcbDoc)
    PcbDoc,
    /// Unknown file type
    Unknown,
}

impl AltiumFileType {
    /// Returns human-readable name for this file type.
    pub fn name(&self) -> &'static str {
        match self {
            AltiumFileType::SchLib => "SchLib",
            AltiumFileType::SchDoc => "SchDoc",
            AltiumFileType::PcbLib => "PcbLib",
            AltiumFileType::PcbDoc => "PcbDoc",
            AltiumFileType::Unknown => "Unknown",
        }
    }

    /// Returns true if this is a library file (SchLib or PcbLib).
    pub fn is_library(&self) -> bool {
        matches!(self, AltiumFileType::SchLib | AltiumFileType::PcbLib)
    }

    /// Returns true if this is a schematic file (SchLib or SchDoc).
    pub fn is_schematic(&self) -> bool {
        matches!(self, AltiumFileType::SchLib | AltiumFileType::SchDoc)
    }

    /// Returns true if this is a PCB file (PcbLib or PcbDoc).
    pub fn is_pcb(&self) -> bool {
        matches!(self, AltiumFileType::PcbLib | AltiumFileType::PcbDoc)
    }
}

/// Information about a stream in the CFB container.
#[derive(Debug, Clone)]
pub struct StreamInfo {
    /// Full path to the stream (e.g., "/FileHeader")
    pub path: String,
    /// Size in bytes
    pub size: u64,
}

/// Information about a storage (directory) in the CFB container.
#[derive(Debug, Clone)]
pub struct StorageInfo {
    /// Full path to the storage (e.g., "/Resistor")
    pub path: String,
    /// Number of child entries (streams + storages)
    pub child_count: usize,
}

/// A raw block from a size-prefixed stream.
#[derive(Debug, Clone)]
pub struct Block {
    /// Offset in the stream where this block starts
    pub offset: usize,
    /// Size of the data (excluding header)
    pub size: usize,
    /// Flags from the high byte of the size field
    pub flags: u8,
    /// Raw block data
    pub data: Vec<u8>,
}

impl Block {
    /// Returns true if the binary flag (0x01) is set.
    ///
    /// In schematic files, this indicates a binary record rather than
    /// a pipe-delimited parameter string.
    pub fn is_binary(&self) -> bool {
        (self.flags & 0x01) != 0
    }

    /// Try to parse the block as a ParameterCollection.
    ///
    /// Returns None if the block appears to be binary or parsing fails.
    pub fn as_params(&self) -> Option<ParameterCollection> {
        if self.is_binary() || self.data.is_empty() {
            return None;
        }

        // Try to interpret as null-terminated string
        let end = self
            .data
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.data.len());
        let text = String::from_utf8_lossy(&self.data[..end]);

        // Quick check: parameters should start with separator
        if !text.starts_with('|') && !text.starts_with('`') {
            return None;
        }

        Some(ParameterCollection::from_string(&text))
    }

    /// Returns the raw data bytes.
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

/// Wrapper around CFB providing Altium-specific convenience methods.
///
/// This is Layer 1 of the API, providing low-level access to the CFB container
/// with helpers for reverse engineering and exploration.
pub struct AltiumCfb<R: Read + Seek> {
    cf: CompoundFile<R>,
    file_type: AltiumFileType,
    section_keys: Option<HashMap<String, String>>,
}

impl AltiumCfb<File> {
    /// Opens a CFB file by path.
    pub fn open_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref()).map_err(AltiumError::Io)?;
        Self::open(file)
    }
}

impl<R: Read + Seek> AltiumCfb<R> {
    /// Opens a CFB from a reader.
    pub fn open(reader: R) -> Result<Self> {
        let cf = CompoundFile::open(reader).map_err(|e| {
            AltiumError::Io(io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
        })?;

        let mut wrapper = AltiumCfb {
            cf,
            file_type: AltiumFileType::Unknown,
            section_keys: None,
        };

        wrapper.file_type = wrapper.detect_file_type();
        Ok(wrapper)
    }

    /// Returns the detected file type.
    pub fn file_type(&self) -> AltiumFileType {
        self.file_type
    }

    /// Detects the file type by examining stream structure.
    fn detect_file_type(&mut self) -> AltiumFileType {
        // Check for PCB-specific streams first
        if self.exists("/Board6/Data") {
            return AltiumFileType::PcbDoc;
        }
        if self.exists("/Library/Data") {
            return AltiumFileType::PcbLib;
        }

        // Check for schematic streams
        if self.exists("/FileHeader") {
            // SchLib has /SectionKeys or nested storages
            if self.exists("/SectionKeys") {
                return AltiumFileType::SchLib;
            }

            // Check for component storages (SchLib pattern)
            for entry in self.cf.walk() {
                if entry.is_storage() && entry.path().components().count() == 2 {
                    // Has a nested storage at depth 1 - likely SchLib
                    return AltiumFileType::SchLib;
                }
            }

            return AltiumFileType::SchDoc;
        }

        AltiumFileType::Unknown
    }

    // --- Navigation ---

    /// Returns true if the given path exists in the CFB.
    pub fn exists(&mut self, path: &str) -> bool {
        self.cf.exists(path)
    }

    /// Lists all streams in the CFB container.
    pub fn streams(&mut self) -> Result<Vec<StreamInfo>> {
        let mut streams = Vec::new();

        for entry in self.cf.walk() {
            if entry.is_stream() {
                streams.push(StreamInfo {
                    path: entry.path().to_string_lossy().to_string(),
                    size: entry.len(),
                });
            }
        }

        // Sort by path for consistent ordering
        streams.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(streams)
    }

    /// Lists all storages (directories) in the CFB container.
    pub fn storages(&mut self) -> Result<Vec<StorageInfo>> {
        let mut storages = Vec::new();
        let mut child_counts: HashMap<String, usize> = HashMap::new();

        // Count children for each storage
        for entry in self.cf.walk() {
            if let Some(parent) = entry.path().parent() {
                let parent_path = parent.to_string_lossy().to_string();
                *child_counts.entry(parent_path).or_insert(0) += 1;
            }
        }

        for entry in self.cf.walk() {
            if entry.is_storage() {
                let path = entry.path().to_string_lossy().to_string();
                let child_count = child_counts.get(&path).copied().unwrap_or(0);
                storages.push(StorageInfo { path, child_count });
            }
        }

        storages.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(storages)
    }

    // --- Section Keys ---

    /// Loads and caches section keys mapping (LIBREF → storage path).
    ///
    /// Section keys map full component names to truncated storage paths,
    /// which is necessary because CFB has a 31-character limit on entry names.
    pub fn section_keys(&mut self) -> Result<&HashMap<String, String>> {
        if self.section_keys.is_none() {
            self.section_keys = Some(self.load_section_keys()?);
        }
        Ok(self.section_keys.as_ref().unwrap())
    }

    /// Loads section keys from /SectionKeys stream.
    fn load_section_keys(&mut self) -> Result<HashMap<String, String>> {
        let mut keys = HashMap::new();

        if !self.exists("/SectionKeys") {
            return Ok(keys);
        }

        let data = self.read_stream("/SectionKeys")?;
        let mut cursor = Cursor::new(&data);

        // Read key count
        let count = cursor.read_i32::<LittleEndian>().unwrap_or(0);

        for _ in 0..count {
            // Read LIBREF (full name)
            let lib_ref = match read_string_block(&mut cursor) {
                Ok(s) => s,
                Err(_) => break,
            };

            // Read section key (storage path)
            let section_key = match read_string_block(&mut cursor) {
                Ok(s) => s,
                Err(_) => break,
            };

            keys.insert(lib_ref, section_key);
        }

        Ok(keys)
    }

    /// Resolves a component/footprint name to its storage path.
    ///
    /// For names ≤31 chars without '/', returns the name directly.
    /// For longer names or names with '/', looks up in section keys.
    pub fn resolve_section(&mut self, lib_ref: &str) -> Result<String> {
        // Check if name needs mapping
        if lib_ref.len() <= 31 && !lib_ref.contains('/') {
            return Ok(lib_ref.to_string());
        }

        let keys = self.section_keys()?;
        keys.get(lib_ref)
            .cloned()
            .ok_or_else(|| AltiumError::MissingData(format!("Section key not found: {}", lib_ref)))
    }

    /// Lists all component/footprint names in a library file.
    pub fn list_components(&mut self) -> Result<Vec<String>> {
        match self.file_type {
            AltiumFileType::SchLib => self.list_schlib_components(),
            AltiumFileType::PcbLib => self.list_pcblib_components(),
            _ => Ok(Vec::new()),
        }
    }

    fn list_schlib_components(&mut self) -> Result<Vec<String>> {
        if !self.exists("/FileHeader") {
            return Ok(Vec::new());
        }

        let data = self.read_stream("/FileHeader")?;
        let mut cursor = Cursor::new(&data);

        // Read header parameters block
        let params = ParameterCollection::read_from(&mut cursor)?;
        let count = params
            .get("COMPCOUNT")
            .or_else(|| params.get("WEIGHT"))
            .map(|v| v.as_int_or(0))
            .unwrap_or(0);

        let mut components = Vec::with_capacity(count as usize);
        for _ in 0..count {
            if let Ok(name) = read_string_block(&mut cursor) {
                components.push(name);
            }
        }

        Ok(components)
    }

    fn list_pcblib_components(&mut self) -> Result<Vec<String>> {
        if !self.exists("/Library/Data") {
            return Ok(Vec::new());
        }

        let data = self.read_stream("/Library/Data")?;
        let mut cursor = Cursor::new(&data);

        // Read header block
        let _header = ParameterCollection::read_from(&mut cursor)?;

        // Read component count
        let count = cursor.read_i32::<LittleEndian>().unwrap_or(0);

        let mut components = Vec::with_capacity(count as usize);
        for _ in 0..count {
            if let Ok(name) = read_string_block(&mut cursor) {
                components.push(name);
            }
        }

        Ok(components)
    }

    // --- Stream Reading ---

    /// Reads raw bytes from a stream.
    pub fn read_stream(&mut self, path: &str) -> Result<Vec<u8>> {
        let mut stream = self
            .cf
            .open_stream(path)
            .map_err(|e| AltiumError::Io(io::Error::new(io::ErrorKind::NotFound, e.to_string())))?;

        let mut data = Vec::new();
        stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
        Ok(data)
    }

    /// Reads a stream and parses it as a single ParameterCollection.
    pub fn read_params(&mut self, path: &str) -> Result<ParameterCollection> {
        let data = self.read_stream(path)?;
        let mut cursor = Cursor::new(&data);
        ParameterCollection::read_from(&mut cursor)
    }

    /// Reads a stream as size-prefixed blocks.
    pub fn read_blocks(&mut self, path: &str) -> Result<Vec<Block>> {
        let data = self.read_stream(path)?;
        Self::parse_blocks(&data)
    }

    /// Parses raw data as size-prefixed blocks.
    pub fn parse_blocks(data: &[u8]) -> Result<Vec<Block>> {
        let mut blocks = Vec::new();
        let mut cursor = Cursor::new(data);

        while (cursor.position() as usize) < data.len() {
            let offset = cursor.position() as usize;

            let size_raw = match cursor.read_i32::<LittleEndian>() {
                Ok(s) => s,
                Err(_) => break,
            };

            let flags = ((size_raw as u32) >> 24) as u8;
            let size = (size_raw & 0x00FFFFFF) as usize;

            if size == 0 || offset + 4 + size > data.len() {
                break;
            }

            let mut block_data = vec![0u8; size];
            if cursor.read_exact(&mut block_data).is_err() {
                break;
            }

            blocks.push(Block {
                offset,
                size,
                flags,
                data: block_data,
            });
        }

        Ok(blocks)
    }

    // --- Decompression ---

    /// Decompresses a zlib-compressed stream.
    pub fn decompress(&mut self, path: &str) -> Result<Vec<u8>> {
        let data = self.read_stream(path)?;
        decompress_zlib(&data)
    }

    /// Decompresses data starting at a given offset.
    pub fn decompress_at(&mut self, path: &str, offset: usize) -> Result<Vec<u8>> {
        let data = self.read_stream(path)?;
        if offset >= data.len() {
            return Err(AltiumError::Parse("Offset beyond stream length".into()));
        }
        decompress_zlib(&data[offset..])
    }

    // --- Debugging/Reverse Engineering ---

    /// Generates a hexdump of stream content.
    pub fn hexdump(
        &mut self,
        path: &str,
        offset: usize,
        length: usize,
        width: usize,
    ) -> Result<String> {
        let data = self.read_stream(path)?;
        let end = if length == 0 {
            data.len()
        } else {
            (offset + length).min(data.len())
        };

        if offset >= data.len() {
            return Ok(String::new());
        }

        let slice = &data[offset..end];
        Ok(format_hexdump(slice, offset, width))
    }

    /// Finds printable strings in a stream.
    pub fn find_strings(&mut self, path: &str, min_length: usize) -> Result<Vec<FoundString>> {
        let data = self.read_stream(path)?;
        Ok(extract_strings(&data, min_length))
    }

    /// Searches for a pattern across all streams.
    pub fn search(&mut self, pattern: &str, ignore_case: bool) -> Result<Vec<SearchMatch>> {
        let mut matches = Vec::new();
        let pattern_bytes = if ignore_case {
            pattern.to_lowercase().into_bytes()
        } else {
            pattern.as_bytes().to_vec()
        };

        for stream in self.streams()? {
            let data = self.read_stream(&stream.path)?;
            let search_data = if ignore_case {
                data.iter()
                    .map(|b| b.to_ascii_lowercase())
                    .collect::<Vec<_>>()
            } else {
                data.clone()
            };

            for (i, window) in search_data.windows(pattern_bytes.len()).enumerate() {
                if window == pattern_bytes.as_slice() {
                    matches.push(SearchMatch {
                        stream: stream.path.clone(),
                        offset: i,
                        context: extract_context(&data, i, 32),
                    });
                }
            }
        }

        Ok(matches)
    }

    /// Returns mutable access to the underlying CompoundFile.
    ///
    /// Use this for advanced operations not covered by the wrapper.
    pub fn inner(&mut self) -> &mut CompoundFile<R> {
        &mut self.cf
    }

    /// Returns immutable access to the underlying CompoundFile.
    pub fn inner_ref(&self) -> &CompoundFile<R> {
        &self.cf
    }

    /// Returns the CFB format version.
    pub fn version(&self) -> cfb::Version {
        self.cf.version()
    }

    /// Checks if a path exists and returns whether it's a stream.
    ///
    /// Returns `Some(true)` if the path is a stream, `Some(false)` if it's a storage,
    /// `None` if the path doesn't exist.
    pub fn entry_type(&mut self, path: &str) -> Option<bool> {
        match self.cf.entry(path) {
            Ok(entry) => Some(entry.is_stream()),
            Err(_) => None,
        }
    }

    /// Returns the size of a stream, or None if the path doesn't exist or isn't a stream.
    pub fn stream_size(&mut self, path: &str) -> Option<u64> {
        match self.cf.entry(path) {
            Ok(entry) if entry.is_stream() => Some(entry.len()),
            _ => None,
        }
    }

    /// Returns an iterator over all entries in the CFB, yielding (path, is_stream, size).
    pub fn entries(&mut self) -> Vec<(String, bool, u64)> {
        self.cf
            .walk()
            .map(|e| {
                (
                    e.path().to_string_lossy().to_string(),
                    e.is_stream(),
                    e.len(),
                )
            })
            .collect()
    }
}

/// A string found during string extraction.
#[derive(Debug, Clone)]
pub struct FoundString {
    /// Offset in the stream
    pub offset: usize,
    /// The string content
    pub content: String,
    /// Detected encoding
    pub encoding: StringEncoding,
}

/// Detected string encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringEncoding {
    Ascii,
    Utf16Le,
    Windows1252,
}

/// A search match result.
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// Stream path where match was found
    pub stream: String,
    /// Offset in the stream
    pub offset: usize,
    /// Context around the match
    pub context: String,
}

// --- Helper Functions ---

fn format_hexdump(data: &[u8], base_offset: usize, width: usize) -> String {
    let mut result = String::new();
    let width = width.clamp(8, 32);

    for (i, chunk) in data.chunks(width).enumerate() {
        let offset = base_offset + i * width;

        // Offset
        result.push_str(&format!("{:08x}  ", offset));

        // Hex bytes
        for (j, byte) in chunk.iter().enumerate() {
            if j == width / 2 {
                result.push(' ');
            }
            result.push_str(&format!("{:02x} ", byte));
        }

        // Padding for incomplete lines
        for j in chunk.len()..width {
            if j == width / 2 {
                result.push(' ');
            }
            result.push_str("   ");
        }

        // ASCII representation
        result.push_str(" |");
        for byte in chunk {
            let c = if byte.is_ascii_graphic() || *byte == b' ' {
                *byte as char
            } else {
                '.'
            };
            result.push(c);
        }
        result.push_str("|\n");
    }

    result
}

fn extract_strings(data: &[u8], min_length: usize) -> Vec<FoundString> {
    let mut strings = Vec::new();
    let mut current = Vec::new();
    let mut start = 0;

    for (i, &byte) in data.iter().enumerate() {
        if byte.is_ascii_graphic() || byte == b' ' {
            if current.is_empty() {
                start = i;
            }
            current.push(byte);
        } else if !current.is_empty() {
            if current.len() >= min_length {
                strings.push(FoundString {
                    offset: start,
                    content: String::from_utf8_lossy(&current).to_string(),
                    encoding: StringEncoding::Ascii,
                });
            }
            current.clear();
        }
    }

    // Handle trailing string
    if current.len() >= min_length {
        strings.push(FoundString {
            offset: start,
            content: String::from_utf8_lossy(&current).to_string(),
            encoding: StringEncoding::Ascii,
        });
    }

    strings
}

fn extract_context(data: &[u8], offset: usize, context_len: usize) -> String {
    let start = offset.saturating_sub(context_len / 2);
    let end = (offset + context_len / 2).min(data.len());

    let slice = &data[start..end];
    let mut result = String::new();

    for &byte in slice {
        if byte.is_ascii_graphic() || byte == b' ' {
            result.push(byte as char);
        } else {
            result.push('.');
        }
    }

    result
}

use crate::traits::FromBinary;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_flags() {
        let block = Block {
            offset: 0,
            size: 100,
            flags: 0x01,
            data: vec![],
        };
        assert!(block.is_binary());

        let block = Block {
            offset: 0,
            size: 100,
            flags: 0x00,
            data: vec![],
        };
        assert!(!block.is_binary());
    }

    #[test]
    fn test_block_as_params() {
        let data = b"|RECORD=1|NAME=Test|\0".to_vec();
        let block = Block {
            offset: 0,
            size: data.len(),
            flags: 0x00,
            data,
        };

        let params = block.as_params().expect("Should parse as params");
        assert_eq!(params.get("RECORD").unwrap().as_int_or(0), 1);
        assert_eq!(params.get("NAME").unwrap().as_str(), "Test");
    }

    #[test]
    fn test_hexdump_format() {
        let data = b"Hello, World!";
        let dump = format_hexdump(data, 0, 16);
        assert!(dump.contains("48 65 6c 6c")); // "Hell"
        assert!(dump.contains("|Hello, World!|"));
    }

    #[test]
    fn test_extract_strings() {
        let data = b"\x00\x00Hello\x00World\x00\x00";
        let strings = extract_strings(data, 4);
        assert_eq!(strings.len(), 2);
        assert_eq!(strings[0].content, "Hello");
        assert_eq!(strings[1].content, "World");
    }
}

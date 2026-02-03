//! SchDoc file I/O — lossless CFB roundtrip.
//!
//! # CFB Storage Hierarchy
//!
//! ```text
//! Root
//! ├── FileHeader    (header params + primitive records)
//! ├── Storage       (icon storage header)
//! └── Additional    (additional params)
//! ```
//!
//! # FileHeader Format
//!
//! The FileHeader stream is a sequence of framed records:
//! - `[u32 size_with_flags][data]`
//! - Low 24 bits = actual length, high byte = flags
//! - Bit 24 set = binary mode record, clear = text/ASCII mode
//! - First record is the header block with HEADER, WEIGHT params
//! - Remaining records are schematic primitives

use serde::{Deserialize, Serialize};
use std::io::{self, Cursor, Read, Seek, Write};

use crate::v2::serializer::SchSerializer;
use crate::v2::serializer::ascii::AsciiSerializer;

/// A parsed SchDoc file.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SchDocV2 {
    /// Parsed record count from the header.
    pub weight: i32,
    /// Parsed primitive records from the FileHeader stream.
    pub records: Vec<SchDocRecord>,
    /// All raw CFB streams for lossless roundtrip.
    #[serde(skip)]
    pub raw_streams: Vec<(String, Vec<u8>)>,
}

/// A single record within the SchDoc FileHeader.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchDocRecord {
    /// Object ID / record type.
    pub record_id: u8,
    /// Extended record ID (when record_id == 254).
    pub record_id_ex: Option<i32>,
    /// Decoded parameter string (empty for binary records).
    pub params: String,
    /// Raw record bytes for lossless roundtrip.
    #[serde(skip)]
    pub raw: Vec<u8>,
}

/// Size flag mask: low 24 bits are the actual length, high byte is flags.
const SIZE_FLAG_MASK: u32 = 0x00FFFFFF;

impl SchDocV2 {
    /// Open and parse a SchDoc CFB file.
    pub fn open<R: Read + Seek>(reader: R) -> io::Result<Self> {
        let mut cfb = cfb::CompoundFile::open(reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut doc = SchDocV2::default();

        // Parse FileHeader for structured data
        if let Ok(fh_data) = read_cfb_stream(&mut cfb, "/FileHeader") {
            let (weight, records) = parse_file_header(&fh_data);
            doc.weight = weight;
            doc.records = records;
        }

        // Collect ALL streams for lossless roundtrip
        let all_entries: Vec<(String, bool)> = cfb
            .walk()
            .map(|e| (e.path().to_string_lossy().replace('\\', "/"), e.is_stream()))
            .collect();

        for (path, is_stream) in &all_entries {
            if !is_stream {
                continue;
            }
            let normalized = if path.starts_with('/') {
                path.clone()
            } else {
                format!("/{}", path)
            };
            if let Ok(data) = read_cfb_stream(&mut cfb, &normalized) {
                doc.raw_streams.push((normalized, data));
            }
        }

        Ok(doc)
    }

    /// Write a SchDoc to a CFB compound file, serializing from typed fields.
    ///
    /// The `/FileHeader` stream is rebuilt from `self.records`.
    /// Other streams (Storage, Additional) are written from `raw_streams`.
    pub fn write<W: Read + Write + Seek>(&self, writer: W) -> io::Result<()> {
        let mut cfb = cfb::CompoundFile::create(writer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Write non-FileHeader streams from raw_streams
        for (path, data) in &self.raw_streams {
            if path == "/FileHeader" {
                continue;
            }
            let mut stream = cfb.create_stream(path)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("create stream {}: {}", path, e)))?;
            stream.write_all(data)?;
        }

        // Rebuild FileHeader from records
        let fh_data = build_file_header(&self.records)?;
        let mut stream = cfb.create_stream("/FileHeader")
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("create FileHeader: {}", e)))?;
        stream.write_all(&fh_data)?;

        cfb.flush()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("CFB flush: {}", e)))?;
        Ok(())
    }
}

/// Parse the FileHeader stream into weight + records.
fn parse_file_header(data: &[u8]) -> (i32, Vec<SchDocRecord>) {
    let mut records = Vec::new();
    let mut cursor = Cursor::new(data);
    let total_len = data.len() as u64;
    let mut weight = 0i32;
    let mut is_first = true;

    while cursor.position() < total_len {
        let mut len_buf = [0u8; 4];
        if cursor.read_exact(&mut len_buf).is_err() {
            break;
        }
        let size_raw = u32::from_le_bytes(len_buf);
        let is_binary = (size_raw & !SIZE_FLAG_MASK) != 0;
        let record_len = (size_raw & SIZE_FLAG_MASK) as usize;

        if record_len == 0 {
            continue;
        }

        if cursor.position() as usize + record_len > data.len() {
            break;
        }

        let mut record_data = vec![0u8; record_len];
        if cursor.read_exact(&mut record_data).is_err() {
            break;
        }

        if is_first {
            // First record is the header block — extract WEIGHT
            is_first = false;
            let param_str = String::from_utf8_lossy(&record_data);
            let mut ser = AsciiSerializer::from_params(&param_str);
            weight = ser.import_long_int("WEIGHT").unwrap_or(0);
            // Store as a record too for completeness
            let record_id = ser.import_instruction("RECORD").unwrap_or(0);
            records.push(SchDocRecord {
                record_id,
                record_id_ex: None,
                params: param_str.to_string(),
                raw: record_data,
            });
            continue;
        }

        if is_binary {
            let mut full_raw = Vec::with_capacity(4 + record_len);
            full_raw.extend_from_slice(&len_buf);
            full_raw.extend_from_slice(&record_data);

            let record_type = if record_data.len() >= 4 {
                u32::from_le_bytes([record_data[0], record_data[1], record_data[2], record_data[3]]) as u8
            } else {
                0
            };

            records.push(SchDocRecord {
                record_id: record_type,
                record_id_ex: None,
                params: String::new(),
                raw: full_raw,
            });
        } else {
            let param_str = String::from_utf8_lossy(&record_data).to_string();
            let mut ser = AsciiSerializer::from_params(&param_str);
            let record_id = ser.import_instruction("RECORD").unwrap_or(0);
            let record_id_ex = if record_id == 254 {
                Some(ser.import_instruction_ex("RECORDEX").unwrap_or(0))
            } else {
                None
            };

            records.push(SchDocRecord {
                record_id,
                record_id_ex,
                params: param_str,
                raw: record_data,
            });
        }
    }

    (weight, records)
}

/// Rebuild the FileHeader stream from records.
///
/// Binary records: `raw` contains `[u32 size_with_flag][data]` — written verbatim.
/// Text records: `raw` contains just `[data]` — prepend `[u32 size]` before writing.
/// Text records without raw: fall back to `params` string.
fn build_file_header(records: &[SchDocRecord]) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();

    for record in records {
        if record.params.is_empty() && !record.raw.is_empty() {
            // Binary record — raw includes the size+flag header
            output.extend_from_slice(&record.raw);
        } else if !record.raw.is_empty() {
            // Text record with raw bytes — prepend u32 size (no flag bit)
            let len = record.raw.len() as u32;
            output.extend_from_slice(&len.to_le_bytes());
            output.extend_from_slice(&record.raw);
        } else {
            // Text record without raw — fall back to params string
            let bytes = record.params.as_bytes();
            let len = bytes.len() as u32;
            output.extend_from_slice(&len.to_le_bytes());
            output.extend_from_slice(bytes);
        }
    }

    Ok(output)
}

fn read_cfb_stream<F: Read + Seek>(cfb: &mut cfb::CompoundFile<F>, path: &str) -> io::Result<Vec<u8>> {
    let mut stream = cfb.open_stream(path)
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e))?;
    let mut data = Vec::new();
    stream.read_to_end(&mut data)?;
    Ok(data)
}

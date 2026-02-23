//! Layer 3 stream parser for PCB binary file headers (Format E).
//! Used by PcbDoc FileHeader, FileHeaderSix, and PcbLib FileHeader.

use altium_format_types::constants::streams::FILE_HEADER;

use crate::binary_io::BinaryReader;
use crate::{AltiumFormatError, Result};

/// Parsed PCB file header (pascal-block format).
#[derive(Debug, Clone)]
pub(crate) struct PcbFileHeader {
    /// Version string (e.g. "PCB 6.0 Binary File").
    pub(crate) version_string: String,
    /// Version number (always 5.01).
    pub(crate) version: f64,
    /// Unique identifier (8-char alpha for PcbLib, GUID for PcbDoc).
    pub(crate) unique_id: String,
}

/// Parse a PcbDoc FileHeaderSix or PcbLib FileHeader (pascal-block format).
///
/// Format: two consecutive pascal blocks:
///   Block 1: u32 outer_length + u8 string_length + version_string + f64 version
///   Block 2: u32 outer_length + u8 string_length + unique_id
pub(crate) fn parse_pcb_file_header(data: &[u8]) -> Result<PcbFileHeader> {
    let mut reader = BinaryReader::new(data);

    // Block 1: version string + version number
    let outer_len1 = reader.read_u32_le()? as usize;
    let mut block1 = reader.sub_reader(outer_len1)?;
    let version_string = block1.read_pascal_string()?;
    let version = block1.read_f64_le()?;
    block1.assert_exhausted()?;

    // Block 2: unique ID
    let outer_len2 = reader.read_u32_le()? as usize;
    let mut block2 = reader.sub_reader(outer_len2)?;
    let unique_id = block2.read_pascal_string()?;
    block2.assert_exhausted()?;

    reader.assert_exhausted()?;

    Ok(PcbFileHeader {
        version_string,
        version,
        unique_id,
    })
}

/// Parse a PcbDoc legacy FileHeader (UTF-16LE format).
///
/// Format: u32 LE char_count + UTF-16LE string (char_count × 2 bytes).
/// Known quirk: the u32 stores the character count (e.g. 19), not byte count (38).
/// Returns the decoded version string (e.g. "PCB 5.0 Binary File").
pub(crate) fn parse_pcb_legacy_header(data: &[u8]) -> Result<String> {
    let mut reader = BinaryReader::new(data);
    let char_count = reader.read_u32_le()? as usize;
    let byte_count = char_count * 2;
    let bytes = reader.read_bytes(byte_count)?;
    let (decoded, _, had_errors) = encoding_rs::UTF_16LE.decode(bytes);
    if had_errors {
        return Err(AltiumFormatError::InvalidParamValue {
            key: FILE_HEADER.to_owned(),
            detail: "UTF-16LE decode error in legacy PCB header".to_owned(),
        });
    }
    reader.assert_exhausted()?;
    Ok(decoded.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_io::BinaryWriter;

    #[test]
    fn parse_pcb6_file_header() {
        let version_str = "PCB 6.0 Binary File";
        let version_f64: f64 = 5.01;
        let unique_id = "ABCDEFGH";

        // Build block 1: pascal string + f64
        let mut block1_inner = BinaryWriter::new();
        block1_inner.write_pascal_string(version_str);
        block1_inner.write_f64_le(version_f64);
        let block1_data = block1_inner.finish();

        // Build block 2: pascal string
        let mut block2_inner = BinaryWriter::new();
        block2_inner.write_pascal_string(unique_id);
        let block2_data = block2_inner.finish();

        // Assemble full header
        let mut w = BinaryWriter::new();
        w.write_u32_le(block1_data.len() as u32);
        w.write_bytes(&block1_data);
        w.write_u32_le(block2_data.len() as u32);
        w.write_bytes(&block2_data);
        let data = w.finish();

        let header = parse_pcb_file_header(&data).unwrap();
        assert_eq!(header.version_string, version_str);
        assert!((header.version - 5.01).abs() < 1e-10);
        assert_eq!(header.unique_id, unique_id);
    }

    #[test]
    fn parse_legacy_header() {
        let text = "PCB 5.0 Binary File";
        let utf16: Vec<u8> = text.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
        let mut w = BinaryWriter::new();
        w.write_u32_le(text.chars().count() as u32); // char count, not byte count
        w.write_bytes(&utf16);
        let data = w.finish();

        let result = parse_pcb_legacy_header(&data).unwrap();
        assert_eq!(result, text);
    }

    #[test]
    fn legacy_header_truncated() {
        let mut w = BinaryWriter::new();
        w.write_u32_le(100); // claims 100 chars = 200 bytes
        w.write_bytes(&[0x41, 0x00]); // only 2 bytes (1 char)
        let data = w.finish();
        let err = parse_pcb_legacy_header(&data).unwrap_err();
        assert!(matches!(err, crate::AltiumFormatError::BinaryReadPastEnd { .. }));
    }

    #[test]
    fn file_header_trailing_data_error() {
        let mut block1 = BinaryWriter::new();
        block1.write_pascal_string("PCB 6.0 Binary File");
        block1.write_f64_le(5.01);
        let b1 = block1.finish();

        let mut block2 = BinaryWriter::new();
        block2.write_pascal_string("TESTTEST");
        let b2 = block2.finish();

        let mut w = BinaryWriter::new();
        w.write_u32_le(b1.len() as u32);
        w.write_bytes(&b1);
        w.write_u32_le(b2.len() as u32);
        w.write_bytes(&b2);
        w.write_u8(0xFF); // trailing junk
        let data = w.finish();

        let err = parse_pcb_file_header(&data).unwrap_err();
        assert!(matches!(err, crate::AltiumFormatError::UnexpectedTrailingData { .. }));
    }
}

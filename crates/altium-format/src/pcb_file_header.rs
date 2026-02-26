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
    /// Optional unique identifier block (8-char alpha for PcbLib, GUID for PcbDoc).
    pub(crate) unique_id: Option<String>,
}

/// Parse a PcbDoc FileHeaderSix or PcbLib FileHeader.
///
/// Format: two consecutive records:
///   Record 1: u32 string_length + u8 pascal_prefix + version_string + f64 version
///   Record 2 (optional in some files): u32 string_length + u8 pascal_prefix + unique_id
///
/// The u32 stores the string length (matching the pascal prefix byte),
/// NOT the total record size.
pub(crate) fn parse_pcb_file_header(data: &[u8]) -> Result<PcbFileHeader> {
    let mut reader = BinaryReader::new(data);

    // Record 1: version string + version number
    let str_len1 = reader.read_u32_le()? as usize;
    let version_string = reader.read_pascal_string()?;
    if version_string.len() != str_len1 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: FILE_HEADER.to_owned(),
            detail: format!(
                "version string length mismatch: u32 says {} but pascal says {}",
                str_len1,
                version_string.len()
            ),
        });
    }
    let version = reader.read_f64_le()?;

    // Record 2: unique ID (optional in some observed PcbDoc files).
    let unique_id = if reader.remaining() == 0 {
        None
    } else {
        let str_len2 = reader.read_u32_le()? as usize;
        let unique_id = reader.read_pascal_string()?;
        if unique_id.len() != str_len2 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: FILE_HEADER.to_owned(),
                detail: format!(
                    "unique_id length mismatch: u32 says {} but pascal says {}",
                    str_len2,
                    unique_id.len()
                ),
            });
        }
        Some(unique_id)
    };

    reader.assert_exhausted()?;

    Ok(PcbFileHeader {
        version_string,
        version,
        unique_id,
    })
}

/// Parse a PcbDoc legacy FileHeader (UTF-16LE format).
///
/// Format: u32 LE char_count + truncated UTF-16LE prefix payload.
/// Known quirk: observed files store a fixed 20-byte UTF-16LE payload after the u32,
/// even though char_count is 19 ("PCB 5.0 Binary File" char count).
/// Returns the decoded version string (e.g. "PCB 5.0 Binary File").
pub(crate) fn parse_pcb_legacy_header(data: &[u8]) -> Result<String> {
    let mut reader = BinaryReader::new(data);
    let _char_count = reader.read_u32_le()? as usize;
    let byte_count = reader.remaining();
    if (byte_count % 2) != 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: FILE_HEADER.to_owned(),
            detail: format!("legacy UTF-16LE payload has odd byte length {byte_count}"),
        });
    }
    let bytes = reader.read_bytes(byte_count)?;
    let (decoded, _, had_errors) = encoding_rs::UTF_16LE.decode(bytes);
    reader.assert_exhausted()?;
    let utf16 = decoded.into_owned();
    if !had_errors && utf16.starts_with("PCB ") {
        return Ok(utf16);
    }
    // Some observed files store the legacy header payload in single-byte encoding.
    let (ansi, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
    let ansi = ansi.into_owned();
    if ansi.starts_with("PCB ") {
        return Ok(ansi);
    }
    let (utf16be, _, _) = encoding_rs::UTF_16BE.decode(bytes);
    let utf16be = utf16be.into_owned();
    if utf16be.starts_with("PCB ") {
        return Ok(utf16be);
    }
    if had_errors {
        return Err(AltiumFormatError::InvalidParamValue {
            key: FILE_HEADER.to_owned(),
            detail: "UTF-16LE decode error in legacy PCB header".to_owned(),
        });
    }
    Ok(utf16)
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

        // Format: u32(str_len) + pascal_string + f64 + u32(str_len) + pascal_string
        let mut w = BinaryWriter::new();
        w.write_u32_le(version_str.len() as u32);
        w.write_pascal_string(version_str).unwrap();
        w.write_f64_le(version_f64);
        w.write_u32_le(unique_id.len() as u32);
        w.write_pascal_string(unique_id).unwrap();
        let data = w.finish();

        let header = parse_pcb_file_header(&data).unwrap();
        assert_eq!(header.version_string, version_str);
        assert!((header.version - 5.01).abs() < 1e-10);
        assert_eq!(header.unique_id, Some(unique_id.to_owned()));
    }

    #[test]
    fn parse_legacy_header() {
        let text = "PCB 5.0 Bi";
        let utf16: Vec<u8> = text.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
        let mut w = BinaryWriter::new();
        w.write_u32_le(19); // legacy char count field, independent of payload byte count
        w.write_bytes(&utf16);
        let data = w.finish();

        let result = parse_pcb_legacy_header(&data).unwrap();
        assert_eq!(result, text);
    }

    #[test]
    fn legacy_header_odd_byte_count_errors() {
        let mut w = BinaryWriter::new();
        w.write_u32_le(19);
        w.write_bytes(&[0x50, 0x00, 0x43]); // odd UTF-16LE byte count
        let data = w.finish();
        let err = parse_pcb_legacy_header(&data).unwrap_err();
        assert!(matches!(
            err,
            crate::AltiumFormatError::InvalidParamValue { .. }
        ));
    }

    #[test]
    fn parse_pcb6_file_header_without_unique_id() {
        let version_str = "PCB 6.0 Binary File";
        let version_f64: f64 = 5.01;

        let mut w = BinaryWriter::new();
        w.write_u32_le(version_str.len() as u32);
        w.write_pascal_string(version_str).unwrap();
        w.write_f64_le(version_f64);
        let data = w.finish();

        let header = parse_pcb_file_header(&data).unwrap();
        assert_eq!(header.version_string, version_str);
        assert!((header.version - 5.01).abs() < 1e-10);
        assert_eq!(header.unique_id, None);
    }

    #[test]
    fn file_header_trailing_data_error() {
        let version_str = "PCB 6.0 Binary File";
        let unique_id = "TESTTEST";

        let mut w = BinaryWriter::new();
        w.write_u32_le(version_str.len() as u32);
        w.write_pascal_string(version_str).unwrap();
        w.write_f64_le(5.01);
        w.write_u32_le(unique_id.len() as u32);
        w.write_pascal_string(unique_id).unwrap();
        w.write_u8(0xFF); // trailing junk
        let data = w.finish();

        let err = parse_pcb_file_header(&data).unwrap_err();
        assert!(matches!(
            err,
            crate::AltiumFormatError::UnexpectedTrailingData { .. }
        ));
    }
}

//! Layer 3 stream parser for PcbDoc WideStrings6 binary TLV (Format D).
//! Used by PcbDoc `/WideStrings6/Data` only. NOT block-framed.
//! Binary type-length-value encoding for Unicode string replacement.
//!
//! Type codes:
//! - 0x06: u8 length, ASCII string data
//! - 0x0C: u32 LE byte count, ASCII string data
//! - 0x12: u32 LE char count (×2 for bytes), UTF-16LE string data
//! - 0x14: u32 LE byte count, UTF-8 string data
//!
//! Critical distinction: PcbLib per-footprint WideStrings uses parameter-block
//! format (Format A), NOT this TLV format.

use altium_format_types::constants::parsing::{
    WIDE_STRING_TYPE_ASCII_U8, WIDE_STRING_TYPE_ASCII_U32, WIDE_STRING_TYPE_UTF16LE,
    WIDE_STRING_TYPE_UTF8,
};

use crate::binary_io::BinaryReader;
use crate::{AltiumFormatError, Result};

/// A single WideStrings6 TLV entry.
#[derive(Debug, Clone)]
pub(crate) struct WideStringEntry {
    /// The decoded Unicode text.
    pub(crate) text: String,
}

/// Parse the WideStrings6 binary TLV stream.
/// Returns entries indexed by position (0-based).
/// Validates that the entire stream is consumed.
pub(crate) fn parse_wide_strings_tlv(stream_data: &[u8]) -> Result<Vec<WideStringEntry>> {
    let mut reader = BinaryReader::new(stream_data);
    let mut entries = Vec::new();
    while reader.remaining() > 0 {
        let type_code = reader.read_u8()?;
        let text = match type_code {
            WIDE_STRING_TYPE_ASCII_U8 => {
                // u8 length + ASCII bytes
                let len = reader.read_u8()? as usize;
                let bytes = reader.read_bytes(len)?;
                String::from_utf8(bytes.to_vec()).map_err(|e| {
                    AltiumFormatError::InvalidParamValue {
                        key: "WideStrings6".to_owned(),
                        detail: format!("type 0x06 ASCII decode error: {e}"),
                    }
                })?
            }
            WIDE_STRING_TYPE_ASCII_U32 => {
                // u32 LE byte count + ASCII bytes
                let len = reader.read_u32_le()? as usize;
                let bytes = reader.read_bytes(len)?;
                String::from_utf8(bytes.to_vec()).map_err(|e| {
                    AltiumFormatError::InvalidParamValue {
                        key: "WideStrings6".to_owned(),
                        detail: format!("type 0x0C ASCII decode error: {e}"),
                    }
                })?
            }
            WIDE_STRING_TYPE_UTF16LE => {
                // u32 LE char count + UTF-16LE bytes (char_count × 2 = byte count)
                let char_count = reader.read_u32_le()? as usize;
                let byte_count = char_count * 2;
                let bytes = reader.read_bytes(byte_count)?;
                let (decoded, _, had_errors) =
                    encoding_rs::UTF_16LE.decode(bytes);
                if had_errors {
                    return Err(AltiumFormatError::InvalidParamValue {
                        key: "WideStrings6".to_owned(),
                        detail: "type 0x12 UTF-16LE decode error".to_owned(),
                    });
                }
                decoded.into_owned()
            }
            WIDE_STRING_TYPE_UTF8 => {
                // u32 LE byte count + UTF-8 bytes
                let len = reader.read_u32_le()? as usize;
                let bytes = reader.read_bytes(len)?;
                std::str::from_utf8(bytes).map_err(|e| {
                    AltiumFormatError::InvalidParamValue {
                        key: "WideStrings6".to_owned(),
                        detail: format!("type 0x14 UTF-8 decode error: {e}"),
                    }
                })?.to_owned()
            }
            other => {
                return Err(AltiumFormatError::InvalidParamValue {
                    key: "WideStrings6".to_owned(),
                    detail: format!("unknown TLV type code {other:#04x}"),
                });
            }
        };
        entries.push(WideStringEntry { text });
    }
    reader.assert_exhausted()?;
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_io::BinaryWriter;

    #[test]
    fn parse_type_06_ascii() {
        let mut w = BinaryWriter::new();
        w.write_u8(0x06);
        w.write_u8(5); // length
        w.write_bytes(b"Hello");
        let data = w.finish();
        let entries = parse_wide_strings_tlv(&data).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "Hello");
    }

    #[test]
    fn parse_type_0c_ascii() {
        let mut w = BinaryWriter::new();
        w.write_u8(0x0C);
        w.write_u32_le(3);
        w.write_bytes(b"ABC");
        let data = w.finish();
        let entries = parse_wide_strings_tlv(&data).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "ABC");
    }

    #[test]
    fn parse_type_12_utf16le() {
        let mut w = BinaryWriter::new();
        w.write_u8(0x12);
        // "Hi" in UTF-16LE = [0x48, 0x00, 0x69, 0x00]
        w.write_u32_le(2); // char count
        w.write_bytes(&[0x48, 0x00, 0x69, 0x00]);
        let data = w.finish();
        let entries = parse_wide_strings_tlv(&data).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "Hi");
    }

    #[test]
    fn parse_type_14_utf8() {
        let mut w = BinaryWriter::new();
        w.write_u8(0x14);
        let text = "café";
        let text_bytes = text.as_bytes();
        w.write_u32_le(text_bytes.len() as u32);
        w.write_bytes(text_bytes);
        let data = w.finish();
        let entries = parse_wide_strings_tlv(&data).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "café");
    }

    #[test]
    fn parse_multiple_entries() {
        let mut w = BinaryWriter::new();
        // Entry 1: type 0x06
        w.write_u8(0x06);
        w.write_u8(2);
        w.write_bytes(b"AB");
        // Entry 2: type 0x14
        w.write_u8(0x14);
        w.write_u32_le(3);
        w.write_bytes(b"XYZ");
        let data = w.finish();
        let entries = parse_wide_strings_tlv(&data).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "AB");
        assert_eq!(entries[1].text, "XYZ");
    }

    #[test]
    fn empty_stream() {
        let entries = parse_wide_strings_tlv(&[]).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn unknown_type_code_returns_error() {
        let mut w = BinaryWriter::new();
        w.write_u8(0xFF); // unknown type
        w.write_u8(0);
        let data = w.finish();
        let err = parse_wide_strings_tlv(&data).unwrap_err();
        assert!(matches!(err, AltiumFormatError::InvalidParamValue { .. }));
    }

    #[test]
    fn truncated_data_returns_error() {
        let mut w = BinaryWriter::new();
        w.write_u8(0x0C);
        w.write_u32_le(100); // claims 100 bytes
        w.write_bytes(&[0x41]); // only 1 byte
        let data = w.finish();
        let err = parse_wide_strings_tlv(&data).unwrap_err();
        assert!(matches!(err, AltiumFormatError::BinaryReadPastEnd { .. }));
    }
}

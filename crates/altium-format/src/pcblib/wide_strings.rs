//! PcbLib WideStrings sidecar stream parser and serializer.
//!
//! Format: u32_le(length) + NUL-terminated parameter string.
//! Empty stream (single 0x00 byte or completely empty) means no wide strings.
//!
//! Keys are ENCODEDTEXT{N} where N is the text-primitive index.
//! Values are comma-separated decimal UTF-16 code units.
//! Example: "39,46,68,101,115" decodes as the string "'.Des".

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::block_stream::write_text_block;
use crate::param_collection::ParameterCollection;
use crate::pcblib::PcbPrimitive;
use crate::{AltiumFormatError, Result};

/// Parses the PcbLib WideStrings sidecar stream.
///
/// Returns a map from text-primitive index to decoded UTF-8 string.
/// Empty stream or a single 0x00 byte returns an empty map.
pub(crate) fn parse_pcblib_wide_strings(data: &[u8]) -> Result<HashMap<usize, String>> {
    if data.is_empty() || data == b"\x00" {
        return Ok(HashMap::new());
    }

    if data.len() < 4 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "WideStrings".to_owned(),
            detail: format!("stream too short: {} bytes", data.len()),
        });
    }

    let len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let payload_start = 4;
    let payload_end = payload_start + len;

    if data.len() < payload_end {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "WideStrings".to_owned(),
            detail: format!(
                "declared length {} exceeds stream size {}",
                len,
                data.len() - 4
            ),
        });
    }

    let payload = &data[payload_start..payload_end];
    let stripped = payload.strip_suffix(b"\x00").unwrap_or(payload);
    let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(stripped);
    let mut params = ParameterCollection::from_str(&decoded)?;

    let keys = params.keys_matching("ENCODEDTEXT");
    let mut result = HashMap::new();
    for key in keys {
        let lower = key.to_ascii_lowercase();
        let index_str = lower.strip_prefix("encodedtext").ok_or_else(|| {
            AltiumFormatError::InvalidParamValue {
                key: key.clone(),
                detail: "unexpected key format".to_owned(),
            }
        })?;
        let index: usize = index_str
            .parse()
            .map_err(|_| AltiumFormatError::InvalidParamValue {
                key: key.clone(),
                detail: format!("non-numeric index suffix: '{index_str}'"),
            })?;
        let value: String = params.remove_required(&key)?;
        let decoded_string = decode_encoded_text(&key, &value)?;
        match result.entry(index) {
            Entry::Vacant(e) => {
                e.insert(decoded_string);
            }
            Entry::Occupied(_) => {
                return Err(AltiumFormatError::InvalidParamValue {
                    key: key.clone(),
                    detail: format!("duplicate ENCODEDTEXT index {index}"),
                });
            }
        }
    }

    Ok(result)
}

/// Decodes a comma-separated decimal UTF-16 code-unit list to a string.
///
/// An empty value or a value with only empty tokens represents an empty string.
fn decode_encoded_text(key: &str, value: &str) -> Result<String> {
    let code_units: Vec<u16> = value
        .split(',')
        .filter(|token| !token.trim().is_empty())
        .map(|token| {
            token
                .trim()
                .parse::<u16>()
                .map_err(|_| AltiumFormatError::InvalidParamValue {
                    key: key.to_owned(),
                    detail: format!("invalid decimal UTF-16 code unit token: '{}'", token.trim()),
                })
        })
        .collect::<Result<Vec<u16>>>()?;

    let mut decoded = String::new();
    for item in char::decode_utf16(code_units) {
        decoded.push(item.map_err(|e| AltiumFormatError::InvalidParamValue {
            key: key.to_owned(),
            detail: format!("invalid UTF-16 code unit 0x{:04X}", e.unpaired_surrogate()),
        })?);
    }
    Ok(decoded)
}

/// Encodes a UTF-8 string as comma-separated decimal UTF-16 code units.
fn encode_text_for_wide_strings(text: &str) -> String {
    text.encode_utf16()
        .map(|code_unit| code_unit.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// Serializes the WideStrings sidecar stream for a footprint.
///
/// Iterates primitives, encoding each Text primitive's content as an
/// ENCODEDTEXT{N} parameter (N = text-primitive index, 0-based).
/// Returns the complete stream bytes (text block with u32 length prefix),
/// or an empty Vec if the footprint has no Text primitives.
pub(crate) fn serialize_pcblib_wide_strings(primitives: &[PcbPrimitive]) -> Vec<u8> {
    let mut params = ParameterCollection::new();
    let mut text_count = 0usize;

    for primitive in primitives {
        if let PcbPrimitive::Text(text) = primitive {
            let encoded = encode_text_for_wide_strings(&text.text);
            params.insert(&format!("ENCODEDTEXT{text_count}"), encoded);
            text_count += 1;
        }
    }

    if text_count == 0 {
        // Altium writes a minimal WideStrings stream even for footprints with
        // no Text primitives: a text block containing a single NUL byte.
        return write_text_block(&[0x00]);
    }

    write_text_block(&params.to_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_stream_returns_empty_map() {
        let result = parse_pcblib_wide_strings(b"\x00").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn completely_empty_stream_returns_empty_map() {
        let result = parse_pcblib_wide_strings(b"").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn decode_encodedtext_ascii_values() {
        // Build a WideStrings stream for ENCODEDTEXT0 = "Foo" = [70, 111, 111]
        // The param string is: |ENCODEDTEXT0=70,111,111\0
        let param_str = "|ENCODEDTEXT0=70,111,111\x00";
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(param_str);
        let len = encoded.len() as u32;
        let mut stream = len.to_le_bytes().to_vec();
        stream.extend_from_slice(&encoded);

        let result = parse_pcblib_wide_strings(&stream).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[&0], "Foo");
    }

    #[test]
    fn decode_encodedtext_non_ascii_code_units() {
        let param_str = "|ENCODEDTEXT0=67,97,102,233\x00";
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(param_str);
        let len = encoded.len() as u32;
        let mut stream = len.to_le_bytes().to_vec();
        stream.extend_from_slice(&encoded);

        let result = parse_pcblib_wide_strings(&stream).unwrap();
        assert_eq!(result[&0], "Café");
    }

    #[test]
    fn decode_encodedtext_code_unit_above_byte_range() {
        // Real PcbLib files store Unicode code units such as U+03A9 OHM SIGN,
        // proving this field is not a byte list.
        let param_str = "|ENCODEDTEXT0=52,45,56,937,32,51,87\x00";
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(param_str);
        let len = encoded.len() as u32;
        let mut stream = len.to_le_bytes().to_vec();
        stream.extend_from_slice(&encoded);

        let result = parse_pcblib_wide_strings(&stream).unwrap();
        assert_eq!(result[&0], "4-8Ω 3W");
    }

    #[test]
    fn encode_text_uses_utf16_code_units() {
        assert_eq!(
            encode_text_for_wide_strings("4-8Ω 3W"),
            "52,45,56,937,32,51,87"
        );
        assert_eq!(encode_text_for_wide_strings("😀"), "55357,56832");
    }

    #[test]
    fn decode_multiple_encodedtext_keys() {
        // ENCODEDTEXT0 = "Hi" = [72, 105], ENCODEDTEXT1 = "Bye" = [66, 121, 101]
        let param_str = "|ENCODEDTEXT0=72,105|ENCODEDTEXT1=66,121,101\x00";
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(param_str);
        let len = encoded.len() as u32;
        let mut stream = len.to_le_bytes().to_vec();
        stream.extend_from_slice(&encoded);

        let result = parse_pcblib_wide_strings(&stream).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[&0], "Hi");
        assert_eq!(result[&1], "Bye");
    }

    #[test]
    fn empty_value_returns_empty_string() {
        // ENCODEDTEXT2=| with no bytes between = and | is a valid empty string.
        // This matches real files like lucashudson-Arduino.PcbLib.
        let param_str = "|ENCODEDTEXT0=70,111,111|ENCODEDTEXT1=|ENCODEDTEXT2=66,65,82\x00";
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(param_str);
        let len = encoded.len() as u32;
        let mut stream = len.to_le_bytes().to_vec();
        stream.extend_from_slice(&encoded);

        let result = parse_pcblib_wide_strings(&stream).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[&0], "Foo");
        assert_eq!(result[&1], "");
        assert_eq!(result[&2], "BAR");
    }

    #[test]
    fn invalid_non_decimal_token_returns_error() {
        let param_str = "|ENCODEDTEXT0=70,xyz,111\x00";
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(param_str);
        let len = encoded.len() as u32;
        let mut stream = len.to_le_bytes().to_vec();
        stream.extend_from_slice(&encoded);

        let err = parse_pcblib_wide_strings(&stream).unwrap_err();
        assert!(matches!(err, AltiumFormatError::InvalidParamValue { .. }));
    }

    #[test]
    fn designator_roundtrip_from_real_file_format() {
        // C0805 WideStrings: ENCODEDTEXT0 = '.Designator' = [39,46,68,101,115,105,103,110,97,116,111,114,39]
        let param_str = "|ENCODEDTEXT0=39,46,68,101,115,105,103,110,97,116,111,114,39\x00";
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(param_str);
        let len = encoded.len() as u32;
        let mut stream = len.to_le_bytes().to_vec();
        stream.extend_from_slice(&encoded);

        let result = parse_pcblib_wide_strings(&stream).unwrap();
        assert_eq!(result[&0], "'.Designator'");
    }
}

//! PcbLib WideStrings sidecar stream parser.
//!
//! Format: u32_le(length) + NUL-terminated parameter string.
//! Empty stream (single 0x00 byte or completely empty) means no wide strings.
//!
//! Keys are ENCODEDTEXT{N} where N is the text-primitive index.
//! Values are comma-separated decimal byte values (ASCII code points).
//! Example: "39,46,68,101,115" decodes as the byte string "'.Des".
//! The decoded bytes are validated as UTF-8 (strict).

use std::collections::HashMap;

use crate::param_collection::ParameterCollection;
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
        let index_str = lower
            .strip_prefix("encodedtext")
            .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                key: key.clone(),
                detail: "unexpected key format".to_owned(),
            })?;
        let index: usize =
            index_str
                .parse()
                .map_err(|_| AltiumFormatError::InvalidParamValue {
                    key: key.clone(),
                    detail: format!("non-numeric index suffix: '{index_str}'"),
                })?;
        let value: String = params.remove_required(&key)?;
        let decoded_string = decode_encoded_text(&key, &value)?;
        result.insert(index, decoded_string);
    }

    Ok(result)
}

/// Decodes a comma-separated decimal byte list to a UTF-8 string.
fn decode_encoded_text(key: &str, value: &str) -> Result<String> {
    let bytes: Vec<u8> = value
        .split(',')
        .map(|token| {
            token.trim().parse::<u8>().map_err(|_| AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: format!("invalid decimal byte token: '{}'", token.trim()),
            })
        })
        .collect::<Result<Vec<u8>>>()?;
    std::str::from_utf8(&bytes)
        .map(|s| s.to_owned())
        .map_err(|e| AltiumFormatError::InvalidParamValue {
            key: key.to_owned(),
            detail: format!("decoded bytes are not valid UTF-8: {e}"),
        })
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

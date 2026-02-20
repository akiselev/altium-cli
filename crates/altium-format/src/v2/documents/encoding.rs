//! Shared encoding helpers for Altium document I/O.
//!
//! Altium files use Windows-1252 encoding for text records and a common
//! length-prefixed parameter block format across SchLib, PcbLib, and other
//! document types. This module provides the shared primitives.

use encoding_rs::WINDOWS_1252;

use crate::v2::parameters::ParameterCollection;

/// Size flag mask: low 24 bits = length, bit 24+ = binary mode flag.
pub(crate) const SIZE_FLAG_MASK: u32 = 0x00FF_FFFF;

/// Decode raw bytes as Windows-1252 into a Rust String.
///
/// Altium files use Windows-1252 encoding for text records. Using this
/// instead of `String::from_utf8_lossy` preserves all byte values as
/// proper Unicode characters (e.g. `\xb5` → µ) instead of replacing
/// them with U+FFFD.
pub(crate) fn decode_win1252(bytes: &[u8]) -> String {
    let (text, _, _) = WINDOWS_1252.decode(bytes);
    text.into_owned()
}

/// Encode a Rust String back to Windows-1252 bytes.
///
/// This is the inverse of `decode_win1252` — characters that originated
/// from Windows-1252 bytes are mapped back to their original single-byte
/// values, enabling byte-perfect round-tripping.
pub(crate) fn encode_win1252(s: &str) -> Vec<u8> {
    let (bytes, _, _) = WINDOWS_1252.encode(s);
    bytes.into_owned()
}

/// Parse the first length-prefixed parameter block from raw stream data.
///
/// Format: `u32_le(len) + payload[len]` (payload typically NUL-terminated).
/// Returns `None` if the data is too short or the length field has
/// unexpected flags set.
pub(crate) fn parse_first_param_block(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 4 {
        return None;
    }
    let raw_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if (raw_len & !SIZE_FLAG_MASK) != 0 {
        return None;
    }
    let len = (raw_len & SIZE_FLAG_MASK) as usize;
    if len == 0 || 4 + len > data.len() {
        return None;
    }
    Some(data[4..4 + len].to_vec())
}

/// Encode a `ParameterCollection` into the standard length-prefixed block
/// format used by SchLib and PcbLib streams.
///
/// Output: `u32_le(payload_len) + win1252_payload + NUL`.
pub(crate) fn encode_single_param_block(params: &ParameterCollection) -> Vec<u8> {
    let mut payload = encode_win1252(&params.to_param_string());
    payload.push(0);
    let mut out = Vec::with_capacity(payload.len() + 4);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&payload);
    out
}

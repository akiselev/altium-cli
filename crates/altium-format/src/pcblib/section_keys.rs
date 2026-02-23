use std::collections::HashMap;
use std::collections::hash_map::Entry;

use altium_format_types::constants::component::LIB_REF;
use altium_format_types::constants::record_structure::{KEY_COUNT, RECORD, SECTION_KEY};
use altium_format_types::constants::streams::SECTION_KEYS;

use crate::binary_io::BinaryReader;
use crate::block_stream::{BlockFormat, parse_blocks};
use crate::param_collection::ParameterCollection;
use crate::{AltiumFormatError, Result};

/// Parses the PcbLib SectionKeys stream.
///
/// PcbLib uses a binary format (NOT the block-framed text format used by SchLib):
///
/// ```text
/// u32 count                      // number of display_name → cfb_key pairs
/// For each pair:
///   u32 display_name_size        // pascal_len + 1
///   u8  pascal_len
///   [pascal_len bytes] display_name   // full footprint name (Windows-1252)
///   u32 cfb_key_size             // pascal_len + 1
///   u8  pascal_len
///   [pascal_len bytes] cfb_key        // truncated to 31 chars for CFB
/// ```
pub(crate) fn parse_section_keys(data: &[u8]) -> Result<HashMap<String, String>> {
    let mut reader = BinaryReader::new(data);
    let count = reader.read_u32_le()? as usize;

    let mut map = HashMap::new();
    for n in 0..count {
        let display_name = read_size_prefixed_pascal_string(&mut reader).map_err(|e| {
            AltiumFormatError::WithContext {
                context: format!("SectionKeys entry {n} display name"),
                source: Box::new(e),
            }
        })?;
        let cfb_key = read_size_prefixed_pascal_string(&mut reader).map_err(|e| {
            AltiumFormatError::WithContext {
                context: format!("SectionKeys entry {n} CFB key"),
                source: Box::new(e),
            }
        })?;

        match map.entry(display_name) {
            Entry::Vacant(e) => {
                e.insert(cfb_key);
            }
            Entry::Occupied(e) => {
                return Err(AltiumFormatError::InvalidParamValue {
                    key: SECTION_KEYS.to_owned(),
                    detail: format!("duplicate display name '{}'", e.key()),
                });
            }
        }
    }

    reader
        .assert_exhausted()
        .map_err(|e| AltiumFormatError::WithContext {
            context: "SectionKeys".to_owned(),
            source: Box::new(e),
        })?;

    Ok(map)
}

/// Reads a u32 size prefix followed by a Pascal string (u8 len + bytes).
/// The size prefix covers the pascal length byte + string data (i.e., 1 + string_len).
fn read_size_prefixed_pascal_string(reader: &mut BinaryReader) -> Result<String> {
    let entry_size = reader.read_u32_le()? as usize;
    let entry_bytes = reader.read_bytes(entry_size)?;
    if entry_bytes.is_empty() {
        return Err(AltiumFormatError::InvalidParamValue {
            key: SECTION_KEYS.to_owned(),
            detail: "empty entry (no pascal length byte)".to_owned(),
        });
    }
    let pascal_len = entry_bytes[0] as usize;
    if pascal_len + 1 != entry_size {
        return Err(AltiumFormatError::InvalidParamValue {
            key: SECTION_KEYS.to_owned(),
            detail: format!("pascal length {pascal_len} + 1 != entry size {entry_size}"),
        });
    }
    let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(&entry_bytes[1..]);
    Ok(decoded.into_owned())
}

/// Parses the SchLib SectionKeys stream.
///
/// SchLib uses a block-framed text format with parameter keys:
/// `KEYCOUNT=N`, `LIBREF{N}=<display_name>`, `SECTIONKEY{N}=<cfb_key>`.
pub(crate) fn parse_section_keys_text(data: &[u8]) -> Result<HashMap<String, String>> {
    let blocks = parse_blocks(data)?;
    if blocks.len() != 1 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: SECTION_KEYS.to_owned(),
            detail: format!("expected 1 block, got {}", blocks.len()),
        });
    }
    let block = &blocks[0];
    if block.format != BlockFormat::Text {
        return Err(AltiumFormatError::InvalidParamValue {
            key: SECTION_KEYS.to_owned(),
            detail: "expected text block, got binary".to_owned(),
        });
    }

    let mut params = ParameterCollection::from_bytes(&block.data)?;

    if let Some(record) = params.remove_optional::<i32>(RECORD)? {
        if record != 0 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: RECORD.to_owned(),
                detail: format!("SectionKeys RECORD must be 0, got {record}"),
            });
        }
    }

    let mut map = HashMap::new();
    let count: i32 = params.remove_required(KEY_COUNT)?;
    for n in 0..count {
        let lib_ref: String = params.remove_required(&format!("{}{}", LIB_REF, n))?;
        let section_key: String = params.remove_required(&format!("{}{}", SECTION_KEY, n))?;
        match map.entry(lib_ref) {
            Entry::Vacant(e) => {
                e.insert(section_key);
            }
            Entry::Occupied(e) => {
                return Err(AltiumFormatError::InvalidParamValue {
                    key: format!("{}{}", LIB_REF, n),
                    detail: format!("duplicate LIBREF '{}'", e.key()),
                });
            }
        }
    }

    params.assert_exhausted()?;

    Ok(map)
}

pub(crate) fn resolve_footprint_key(name: &str, section_keys: &HashMap<String, String>) -> String {
    let key = section_keys.get(name).map(String::as_str).unwrap_or(name);
    sanitize_cfb_name(key)
}

pub(crate) fn sanitize_cfb_name(name: &str) -> String {
    name.chars()
        .map(|c| if "/\\:*?\"<>|!".contains(c) { '_' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_short_name_unchanged() {
        let keys = HashMap::new();
        assert_eq!(resolve_footprint_key("SOT23", &keys), "SOT23");
    }

    #[test]
    fn sanitize_replaces_illegal_chars() {
        assert_eq!(sanitize_cfb_name("A/B:C"), "A_B_C");
    }
}

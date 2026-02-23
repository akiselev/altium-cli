use std::collections::hash_map::Entry;
use std::collections::HashMap;

use altium_format_types::constants::component::LIB_REF;
use altium_format_types::constants::record_structure::{KEY_COUNT, RECORD, SECTION_KEY};
use altium_format_types::constants::streams::SECTION_KEYS;

use crate::block_stream::{parse_blocks, BlockFormat};
use crate::param_collection::ParameterCollection;
use crate::{AltiumFormatError, Result};

pub(crate) fn parse_section_keys(data: &[u8]) -> Result<HashMap<String, String>> {
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

//! PcbLib sidecar stream parsers and merge logic.
//!
//! Sidecar streams augment footprint primitive data with UniqueIDs, Unicode
//! strings (WideStrings), and extended properties. They live alongside the
//! core Data/Header streams in each footprint's CFB storage.

use std::collections::HashMap;

use altium_format_types::PcbObjectId;

use crate::block_stream::iter_blocks;
use crate::param_collection::ParameterCollection;
use crate::pcb_binary_stream::parse_pcb_section_header;
use crate::pcblib::PcbPrimitive;
use crate::{AltiumFormatError, Result};

/// One entry from UniqueIDPrimitiveInformation.
#[derive(Debug)]
pub(crate) struct UniqueIdEntry {
    pub(crate) primitive_index: usize,
    pub(crate) object_id: PcbObjectId,
    pub(crate) unique_id: String,
}

/// Parses the UniqueIDPrimitiveInformation/Header and Data streams.
///
/// Header is a u32 count. Data is a sequence of block-framed parameter strings,
/// each containing PRIMITIVEINDEX, PRIMITIVEOBJECTID, and UNIQUEID.
pub(crate) fn parse_unique_id_primitive_information(
    header_data: &[u8],
    data: &[u8],
) -> Result<Vec<UniqueIdEntry>> {
    let expected_count = parse_pcb_section_header(header_data)? as usize;

    let mut entries = Vec::with_capacity(expected_count);
    for block_result in iter_blocks(data) {
        let block = block_result?;
        let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(&block.data);
        let mut params = ParameterCollection::from_str(&decoded)?;

        let primitive_index: i32 = params.remove_required("PRIMITIVEINDEX")?;
        if primitive_index < 0 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "PRIMITIVEINDEX".to_owned(),
                detail: format!("negative primitive index: {primitive_index}"),
            });
        }
        let primitive_index = primitive_index as usize;

        let object_id_str: String = params.remove_required("PRIMITIVEOBJECTID")?;
        let object_id = PcbObjectId::from_primitive_object_id_str(&object_id_str).ok_or_else(
            || AltiumFormatError::InvalidParamValue {
                key: "PRIMITIVEOBJECTID".to_owned(),
                detail: format!("unknown primitive object ID string: '{object_id_str}'"),
            },
        )?;

        let unique_id: String = params.remove_required("UNIQUEID")?;
        params.assert_exhausted()?;

        entries.push(UniqueIdEntry { primitive_index, object_id, unique_id });
    }

    if entries.len() != expected_count {
        return Err(AltiumFormatError::RecordCountMismatch {
            section: "UniqueIDPrimitiveInformation".to_owned(),
            expected: expected_count,
            actual: entries.len(),
        });
    }

    Ok(entries)
}

/// Parses the ExtendedPrimitiveInformation/Header and Data streams.
///
/// Validates the format but uses drain_remaining() on each block since the
/// full set of possible keys has not been determined.
pub(crate) fn parse_extended_primitive_information(
    header_data: &[u8],
    data: &[u8],
) -> Result<Vec<usize>> {
    let expected_count = parse_pcb_section_header(header_data)? as usize;

    let mut indices = Vec::with_capacity(expected_count);
    for block_result in iter_blocks(data) {
        let block = block_result?;
        let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(&block.data);
        let mut params = ParameterCollection::from_str(&decoded)?;

        let primitive_index: i32 = params.remove_required("PRIMITIVEINDEX")?;
        if primitive_index < 0 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "PRIMITIVEINDEX".to_owned(),
                detail: format!("negative primitive index: {primitive_index}"),
            });
        }
        // drain_remaining() is intentional: ExtendedPrimitiveInformation has
        // not been fully reverse-engineered, so we accept unknown keys here.
        params.drain_remaining();

        indices.push(primitive_index as usize);
    }

    if indices.len() != expected_count {
        return Err(AltiumFormatError::RecordCountMismatch {
            section: "ExtendedPrimitiveInformation".to_owned(),
            expected: expected_count,
            actual: indices.len(),
        });
    }

    Ok(indices)
}

/// Validates the PrimitiveGuids/Header and Data streams.
///
/// If the count is greater than zero, returns an error because the format
/// has not been investigated. An empty (count == 0) stream is accepted.
pub(crate) fn validate_primitive_guids(header_data: &[u8], data: &[u8]) -> Result<()> {
    let count = parse_pcb_section_header(header_data)? as usize;
    if count > 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "PrimitiveGuids".to_owned(),
            detail: format!(
                "PrimitiveGuids stream has {count} entries; format not yet implemented"
            ),
        });
    }
    if !data.is_empty() {
        return Err(AltiumFormatError::UnexpectedTrailingData {
            offset: 0,
            count: data.len(),
        });
    }
    Ok(())
}

/// Merges parsed sidecar data into footprint primitives in place.
///
/// - `wide_strings`: text-primitive index -> replacement text string
/// - `unique_ids`: primitive index -> unique ID assignment with type validation
///
/// WideStrings are applied by counting Text primitives in order; the Nth Text
/// primitive (0-based) receives wide_strings[N] if present.
///
/// UniqueIDs are applied by primitive index with type validation: the object ID
/// from the sidecar must match the actual primitive type at that index.
pub(crate) fn merge_sidecars(
    primitives: &mut Vec<PcbPrimitive>,
    wide_strings: HashMap<usize, String>,
    unique_ids: Vec<UniqueIdEntry>,
) -> Result<()> {
    // Apply WideStrings: count Text primitives, replace text field for matching indices.
    let mut text_count = 0usize;
    for primitive in primitives.iter_mut() {
        if let PcbPrimitive::Text(text) = primitive {
            if let Some(wide_text) = wide_strings.get(&text_count) {
                text.text = wide_text.clone();
            }
            text_count += 1;
        }
    }

    // Apply UniqueIDs: validate primitive type, then set unique_id.
    let primitive_count = primitives.len();
    for entry in unique_ids {
        let idx = entry.primitive_index;
        let primitive = primitives.get_mut(idx).ok_or_else(|| {
            AltiumFormatError::InvalidParamValue {
                key: "PRIMITIVEINDEX".to_owned(),
                detail: format!(
                    "primitive index {idx} out of range (footprint has {primitive_count} primitives)"
                ),
            }
        })?;

        let actual_object_id = primitive_object_id(primitive);
        if actual_object_id != entry.object_id {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "PRIMITIVEOBJECTID".to_owned(),
                detail: format!(
                    "primitive at index {idx} is {:?} but sidecar says {:?}",
                    actual_object_id, entry.object_id
                ),
            });
        }

        set_unique_id(primitive, entry.unique_id);
    }

    Ok(())
}

/// Returns the PcbObjectId for a primitive variant.
fn primitive_object_id(primitive: &PcbPrimitive) -> PcbObjectId {
    match primitive {
        PcbPrimitive::Arc(_) => PcbObjectId::Arc,
        PcbPrimitive::Pad(_) => PcbObjectId::Pad,
        PcbPrimitive::Via(_) => PcbObjectId::Via,
        PcbPrimitive::Track(_) => PcbObjectId::Track,
        PcbPrimitive::Text(_) => PcbObjectId::Text,
        PcbPrimitive::Fill(_) => PcbObjectId::Fill,
        PcbPrimitive::Region(_) => PcbObjectId::Region,
        PcbPrimitive::ComponentBody(_) => PcbObjectId::ComponentBody,
    }
}

/// Sets the unique_id field on a primitive variant.
fn set_unique_id(primitive: &mut PcbPrimitive, unique_id: String) {
    match primitive {
        PcbPrimitive::Arc(p) => p.unique_id = Some(unique_id),
        PcbPrimitive::Pad(p) => p.unique_id = Some(unique_id),
        PcbPrimitive::Via(p) => p.unique_id = Some(unique_id),
        PcbPrimitive::Track(p) => p.unique_id = Some(unique_id),
        PcbPrimitive::Text(p) => p.unique_id = Some(unique_id),
        PcbPrimitive::Fill(p) => p.unique_id = Some(unique_id),
        PcbPrimitive::Region(p) => p.unique_id = Some(unique_id),
        PcbPrimitive::ComponentBody(p) => p.unique_id = Some(unique_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_stream::write_text_block;

    fn make_header(count: u32) -> Vec<u8> {
        count.to_le_bytes().to_vec()
    }

    fn make_unique_id_block(index: i32, object_id: &str, unique_id: &str) -> Vec<u8> {
        let param = format!("|PRIMITIVEINDEX={index}|PRIMITIVEOBJECTID={object_id}|UNIQUEID={unique_id}\x00");
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(&param);
        write_text_block(&encoded)
    }

    #[test]
    fn parse_unique_id_single_pad_entry() {
        let header = make_header(1);
        let data = make_unique_id_block(0, "Pad", "ABCDEFGH");

        let entries = parse_unique_id_primitive_information(&header, &data).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].primitive_index, 0);
        assert_eq!(entries[0].object_id, PcbObjectId::Pad);
        assert_eq!(entries[0].unique_id, "ABCDEFGH");
    }

    #[test]
    fn parse_unique_id_multiple_entries() {
        let header = make_header(2);
        let mut data = make_unique_id_block(0, "Pad", "AAAAAAAA");
        data.extend(make_unique_id_block(1, "Pad", "BBBBBBBB"));

        let entries = parse_unique_id_primitive_information(&header, &data).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].unique_id, "AAAAAAAA");
        assert_eq!(entries[1].unique_id, "BBBBBBBB");
    }

    #[test]
    fn parse_unique_id_count_mismatch_returns_error() {
        let header = make_header(3);
        let data = make_unique_id_block(0, "Pad", "AAAAAAAA");

        let err = parse_unique_id_primitive_information(&header, &data).unwrap_err();
        assert!(matches!(err, AltiumFormatError::RecordCountMismatch { expected: 3, actual: 1, .. }));
    }

    #[test]
    fn parse_unique_id_unknown_object_id_returns_error() {
        let header = make_header(1);
        let data = make_unique_id_block(0, "UnknownType", "AAAAAAAA");

        let err = parse_unique_id_primitive_information(&header, &data).unwrap_err();
        assert!(matches!(err, AltiumFormatError::InvalidParamValue { .. }));
    }

    #[test]
    fn validate_primitive_guids_empty_is_ok() {
        let header = make_header(0);
        validate_primitive_guids(&header, &[]).unwrap();
    }

    #[test]
    fn validate_primitive_guids_nonzero_returns_error() {
        let header = make_header(1);
        let err = validate_primitive_guids(&header, &[]).unwrap_err();
        assert!(matches!(err, AltiumFormatError::InvalidParamValue { .. }));
    }
}

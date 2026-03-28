//! PcbLib sidecar stream parsers and merge logic.
//!
//! Sidecar streams augment footprint primitive data with UniqueIDs, Unicode
//! strings (WideStrings), and extended properties. They live alongside the
//! core Data/Header streams in each footprint's CFB storage.

use std::collections::HashMap;

use altium_format_types::{MaskExpansionMode, PcbObjectId, ViewableObjectId};

use crate::binary_io::{BinaryReader, BinaryWriter};
use crate::block_stream::{iter_blocks, write_text_block};
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
        let object_id =
            PcbObjectId::from_primitive_object_id_str(&object_id_str).ok_or_else(|| {
                AltiumFormatError::InvalidParamValue {
                    key: "PRIMITIVEOBJECTID".to_owned(),
                    detail: format!("unknown primitive object ID string: '{object_id_str}'"),
                }
            })?;

        let unique_id: String = params.remove_required("UNIQUEID")?;
        params.assert_exhausted()?;

        entries.push(UniqueIdEntry {
            primitive_index,
            object_id,
            unique_id,
        });
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

/// One entry from ExtendedPrimitiveInformation.
#[derive(Debug, Clone)]
pub(crate) struct ExtendedPrimitiveInfoEntry {
    pub(crate) primitive_index: usize,
    pub(crate) primitive_object_id: PcbObjectId,
    pub(crate) info_type: String,
    pub(crate) solder_mask_expansion_mode: MaskExpansionMode,
    pub(crate) solder_mask_expansion_manual: String,
    pub(crate) paste_mask_expansion_mode: MaskExpansionMode,
    pub(crate) paste_mask_expansion_manual: String,
}

/// One entry from PrimitiveGuids.
///
/// Uses `ViewableObjectId` (not `PcbObjectId`) because the PrimitiveGuids sidecar
/// tracks GUIDs for all viewable object types, including groups, rules, dimension
/// subtypes, etc. — not just binary-record primitive types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrimitiveGuidEntry {
    pub(crate) object_id: ViewableObjectId,
    pub(crate) index_for_save: i32,
    pub(crate) guid: [u8; 16],
}

/// Parses a mask expansion mode string ("None", "NoMask", "Rule", "Manual").
fn parse_mask_expansion_mode(key: &str, value: &str) -> Result<MaskExpansionMode> {
    match value {
        "None" | "NoMask" => Ok(MaskExpansionMode::NoMask),
        "Rule" => Ok(MaskExpansionMode::Rule),
        "Manual" => Ok(MaskExpansionMode::Manual),
        _ => Err(AltiumFormatError::InvalidParamValue {
            key: key.to_owned(),
            detail: format!("unknown mask expansion mode: '{value}'"),
        }),
    }
}

/// Parses the ExtendedPrimitiveInformation/Header and Data streams.
///
/// Each entry contains mask expansion properties for a specific primitive.
/// Known keys: PRIMITIVEINDEX, PRIMITIVEOBJECTID, TYPE, SOLDERMASKEXPANSIONMODE,
/// SOLDERMASKEXPANSION_MANUAL, PASTEMASKEXPANSIONMODE, PASTEMASKEXPANSION_MANUAL.
pub(crate) fn parse_extended_primitive_information(
    header_data: &[u8],
    data: &[u8],
) -> Result<Vec<ExtendedPrimitiveInfoEntry>> {
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

        let object_id_str: String = params.remove_required("PRIMITIVEOBJECTID")?;
        let primitive_object_id = PcbObjectId::from_primitive_object_id_str(&object_id_str)
            .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                key: "PRIMITIVEOBJECTID".to_owned(),
                detail: format!("unknown primitive object ID string: '{object_id_str}'"),
            })?;

        let info_type = params
            .remove_optional::<String>("TYPE")?
            .unwrap_or_default();

        let solder_mode_str = params
            .remove_optional::<String>("SOLDERMASKEXPANSIONMODE")?
            .unwrap_or_else(|| "None".to_owned());
        let solder_mask_expansion_mode =
            parse_mask_expansion_mode("SOLDERMASKEXPANSIONMODE", &solder_mode_str)?;
        let solder_mask_expansion_manual = params
            .remove_optional::<String>("SOLDERMASKEXPANSION_MANUAL")?
            .unwrap_or_default();

        let paste_mode_str = params
            .remove_optional::<String>("PASTEMASKEXPANSIONMODE")?
            .unwrap_or_else(|| "None".to_owned());
        let paste_mask_expansion_mode =
            parse_mask_expansion_mode("PASTEMASKEXPANSIONMODE", &paste_mode_str)?;
        let paste_mask_expansion_manual = params
            .remove_optional::<String>("PASTEMASKEXPANSION_MANUAL")?
            .unwrap_or_default();

        params.assert_exhausted()?;

        entries.push(ExtendedPrimitiveInfoEntry {
            primitive_index: primitive_index as usize,
            primitive_object_id,
            info_type,
            solder_mask_expansion_mode,
            solder_mask_expansion_manual,
            paste_mask_expansion_mode,
            paste_mask_expansion_manual,
        });
    }

    if entries.len() != expected_count {
        return Err(AltiumFormatError::RecordCountMismatch {
            section: "ExtendedPrimitiveInformation".to_owned(),
            expected: expected_count,
            actual: entries.len(),
        });
    }

    Ok(entries)
}

/// Size of a single PrimitiveGuids record: ObjectId (4) + IndexForSave (4) + GUID (16).
const PRIMITIVE_GUID_RECORD_SIZE: usize = 24;

/// PcbDoc PrimitiveGuids entry — stores full i32 ObjectId (upper bytes have metadata).
#[derive(Debug, Clone)]
pub(crate) struct PrimitiveGuidEntryPcbDoc {
    pub(crate) object_id_raw: i32,
    pub(crate) index_for_save: i32,
    pub(crate) guid: [u8; 16],
}

/// Parses PrimitiveGuids/Header and Data streams for PcbDoc.
///
/// PcbDoc stores the full i32 ObjectId (upper bytes carry metadata), unlike
/// PcbLib which truncates to a u8 ViewableObjectId. The Data stream contains
/// `count` fixed-size 24-byte binary records.
pub(crate) fn parse_primitive_guids_pcbdoc(
    header_data: &[u8],
    data: &[u8],
) -> Result<Vec<PrimitiveGuidEntryPcbDoc>> {
    let count = parse_pcb_section_header(header_data)? as usize;
    let expected_bytes = count * PRIMITIVE_GUID_RECORD_SIZE;
    if data.len() != expected_bytes {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "PrimitiveGuids/Data".to_owned(),
            detail: format!(
                "expected {} bytes ({} × {}), got {}",
                expected_bytes,
                count,
                PRIMITIVE_GUID_RECORD_SIZE,
                data.len()
            ),
        });
    }
    let mut reader = BinaryReader::new(data);
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let object_id_raw = reader.read_i32_le()?;
        let index_for_save = reader.read_i32_le()?;
        let mut guid = [0u8; 16];
        guid.copy_from_slice(reader.read_bytes(16)?);
        entries.push(PrimitiveGuidEntryPcbDoc {
            object_id_raw,
            index_for_save,
            guid,
        });
    }
    reader.assert_exhausted()?;
    Ok(entries)
}

/// Parses PrimitiveGuids/Header and Data streams.
///
/// The Data stream contains `count` fixed-size 24-byte binary records
/// (`RT_PCB.TPrimitiveGUID`: i32 ObjectId + i32 IndexForSave + Guid).
pub(crate) fn parse_primitive_guids(
    header_data: &[u8],
    data: &[u8],
) -> Result<Vec<PrimitiveGuidEntry>> {
    let count = parse_pcb_section_header(header_data)? as usize;
    let expected_bytes = count * PRIMITIVE_GUID_RECORD_SIZE;
    if data.len() != expected_bytes {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "PrimitiveGuids".to_owned(),
            detail: format!(
                "expected {} bytes ({} entries * {} bytes), got {} bytes",
                expected_bytes,
                count,
                PRIMITIVE_GUID_RECORD_SIZE,
                data.len()
            ),
        });
    }
    let mut reader = BinaryReader::new(data);
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let object_id_value = reader.read_i32_le()?;
        let object_id_u8 =
            u8::try_from(object_id_value).map_err(|_| AltiumFormatError::InvalidParamValue {
                key: "PrimitiveGuids.ObjectId".to_owned(),
                detail: format!("object id out of byte range: {object_id_value}"),
            })?;
        let object_id = ViewableObjectId::try_from(object_id_u8).map_err(|_| {
            AltiumFormatError::InvalidParamValue {
                key: "PrimitiveGuids.ObjectId".to_owned(),
                detail: format!("unknown viewable object id value: {object_id_value}"),
            }
        })?;
        let index_for_save = reader.read_i32_le()?;
        let mut guid = [0u8; 16];
        guid.copy_from_slice(reader.read_bytes(16)?);
        entries.push(PrimitiveGuidEntry {
            object_id,
            index_for_save,
            guid,
        });
    }
    reader.assert_exhausted()?;
    Ok(entries)
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

/// Validates that ExtendedPrimitiveInformation entries reference valid primitives.
pub(crate) fn validate_extended_entries(
    primitives: &[PcbPrimitive],
    entries: &[ExtendedPrimitiveInfoEntry],
) -> Result<()> {
    let primitive_count = primitives.len();
    for entry in entries {
        let idx = entry.primitive_index;
        let primitive = primitives.get(idx).ok_or_else(|| {
            AltiumFormatError::InvalidParamValue {
                key: "PRIMITIVEINDEX".to_owned(),
                detail: format!(
                    "extended info primitive index {idx} out of range (footprint has {primitive_count} primitives)"
                ),
            }
        })?;
        let actual_object_id = primitive_object_id(primitive);
        if actual_object_id != entry.primitive_object_id {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "PRIMITIVEOBJECTID".to_owned(),
                detail: format!(
                    "extended info at index {idx} is {:?} but sidecar says {:?}",
                    actual_object_id, entry.primitive_object_id
                ),
            });
        }
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

/// Returns the unique_id field of a primitive variant.
pub(crate) fn get_unique_id(primitive: &PcbPrimitive) -> Option<&str> {
    match primitive {
        PcbPrimitive::Arc(p) => p.unique_id.as_deref(),
        PcbPrimitive::Pad(p) => p.unique_id.as_deref(),
        PcbPrimitive::Via(p) => p.unique_id.as_deref(),
        PcbPrimitive::Track(p) => p.unique_id.as_deref(),
        PcbPrimitive::Text(p) => p.unique_id.as_deref(),
        PcbPrimitive::Fill(p) => p.unique_id.as_deref(),
        PcbPrimitive::Region(p) => p.unique_id.as_deref(),
        PcbPrimitive::ComponentBody(p) => p.unique_id.as_deref(),
    }
}

/// Converts MaskExpansionMode to the string used by Altium files.
///
/// Altium uses "None" for the no-mask case (our parser accepts both "None" and
/// "NoMask", but Altium files consistently write "None").
fn mask_expansion_mode_to_str(mode: MaskExpansionMode) -> Result<&'static str> {
    match mode {
        MaskExpansionMode::NoMask => Ok("None"),
        MaskExpansionMode::Rule => Ok("Rule"),
        MaskExpansionMode::Manual => Ok("Manual"),
        other => Err(AltiumFormatError::InvalidParamValue {
            key: "MaskExpansionMode".to_owned(),
            detail: format!("unhandled variant {other:?}"),
        }),
    }
}

/// Serializes UniqueIDPrimitiveInformation Header and Data streams.
///
/// Iterates primitives and emits a text block for each that has a non-empty
/// unique_id. Returns (header_bytes, data_bytes).
pub(crate) fn serialize_unique_id_primitive_information(
    primitives: &[PcbPrimitive],
) -> (Vec<u8>, Vec<u8>) {
    let mut data = Vec::new();
    let mut count: u32 = 0;

    for (index, primitive) in primitives.iter().enumerate() {
        if let Some(uid) = get_unique_id(primitive) {
            let object_id = primitive_object_id(primitive);
            let param_str = format!(
                "|PRIMITIVEINDEX={}|PRIMITIVEOBJECTID={}|UNIQUEID={}\x00",
                index, object_id, uid
            );
            let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(&param_str);
            data.extend_from_slice(&write_text_block(&encoded));
            count += 1;
        }
    }

    let mut w = BinaryWriter::new();
    w.write_u32_le(count);
    (w.finish(), data)
}

/// Serializes ExtendedPrimitiveInformation Header and Data streams.
///
/// Each entry becomes a text block with mask expansion parameters.
/// Returns (header_bytes, data_bytes).
pub(crate) fn serialize_extended_primitive_information(
    entries: &[ExtendedPrimitiveInfoEntry],
) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut data = Vec::new();

    for entry in entries {
        let mut params = ParameterCollection::new();
        params.insert("PRIMITIVEINDEX", entry.primitive_index.to_string());
        params.insert("PRIMITIVEOBJECTID", entry.primitive_object_id.to_string());
        if !entry.info_type.is_empty() {
            params.insert("TYPE", entry.info_type.clone());
        }
        params.insert(
            "SOLDERMASKEXPANSIONMODE",
            mask_expansion_mode_to_str(entry.solder_mask_expansion_mode)?.to_owned(),
        );
        if !entry.solder_mask_expansion_manual.is_empty() {
            params.insert(
                "SOLDERMASKEXPANSION_MANUAL",
                entry.solder_mask_expansion_manual.clone(),
            );
        }
        params.insert(
            "PASTEMASKEXPANSIONMODE",
            mask_expansion_mode_to_str(entry.paste_mask_expansion_mode)?.to_owned(),
        );
        if !entry.paste_mask_expansion_manual.is_empty() {
            params.insert(
                "PASTEMASKEXPANSION_MANUAL",
                entry.paste_mask_expansion_manual.clone(),
            );
        }
        data.extend_from_slice(&write_text_block(&params.to_bytes()));
    }

    let mut w = BinaryWriter::new();
    w.write_u32_le(entries.len() as u32);
    Ok((w.finish(), data))
}

/// Serializes PrimitiveGuids Header and Data streams.
///
/// Each entry is a 24-byte binary record: i32 object_id + i32 index_for_save + 16-byte GUID.
/// Returns (header_bytes, data_bytes).
pub(crate) fn serialize_primitive_guids(entries: &[PrimitiveGuidEntry]) -> (Vec<u8>, Vec<u8>) {
    let mut w = BinaryWriter::new();
    for entry in entries {
        w.write_i32_le(entry.object_id as u8 as i32);
        w.write_i32_le(entry.index_for_save);
        w.write_bytes(&entry.guid);
    }

    let mut header = BinaryWriter::new();
    header.write_u32_le(entries.len() as u32);
    (header.finish(), w.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_stream::write_text_block;

    fn make_header(count: u32) -> Vec<u8> {
        count.to_le_bytes().to_vec()
    }

    fn make_unique_id_block(index: i32, object_id: &str, unique_id: &str) -> Vec<u8> {
        let param = format!(
            "|PRIMITIVEINDEX={index}|PRIMITIVEOBJECTID={object_id}|UNIQUEID={unique_id}\x00"
        );
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
        assert!(matches!(
            err,
            AltiumFormatError::RecordCountMismatch {
                expected: 3,
                actual: 1,
                ..
            }
        ));
    }

    #[test]
    fn parse_unique_id_unknown_object_id_returns_error() {
        let header = make_header(1);
        let data = make_unique_id_block(0, "UnknownType", "AAAAAAAA");

        let err = parse_unique_id_primitive_information(&header, &data).unwrap_err();
        assert!(matches!(err, AltiumFormatError::InvalidParamValue { .. }));
    }

    #[test]
    fn parse_primitive_guids_empty_is_ok() {
        let header = make_header(0);
        let entries = parse_primitive_guids(&header, &[]).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_primitive_guids_with_entries() {
        let header = make_header(2);
        // 2 entries * 24 bytes = 48 bytes
        let data = vec![0u8; 48];
        let entries = parse_primitive_guids(&header, &data).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn parse_primitive_guids_wrong_size_returns_error() {
        let header = make_header(1);
        // 1 entry expects 24 bytes, provide only 10
        let data = vec![0u8; 10];
        let err = parse_primitive_guids(&header, &data).unwrap_err();
        assert!(matches!(err, AltiumFormatError::InvalidParamValue { .. }));
    }
}

use altium_format_types::constants::parsing::{
    BLOCK_SIZE_MASK, DEFAULT_SUBRECORD_COUNT, PAD_SUBRECORD_COUNT, TEXT_SUBRECORD_COUNT,
};
use altium_format_types::{Coord, PcbObjectId};

use crate::binary_io::BinaryReader;
use crate::param_collection::ParameterCollection;
use crate::pcb_binary_stream::parse_pcb_section_header;
use crate::pcblib::PcbFootprint;
use crate::pcblib::custom_shapes::{
    parse_corner_radius_chamfer, parse_custom_mask_shapes, parse_custom_shapes,
    validate_custom_shape_entries,
};
use crate::pcblib::primitives;
use crate::pcblib::sidecar::{
    merge_sidecars, parse_extended_primitive_information, parse_primitive_guids,
    parse_unique_id_primitive_information, validate_extended_entries,
};
use crate::pcblib::wide_strings::parse_pcblib_wide_strings;
use crate::tracked_cfb::TrackedCfbDocument;
use crate::{AltiumFormatError, Result, ResultExt};

pub(crate) fn load_footprint(
    doc: &mut TrackedCfbDocument,
    cfb_key: &str,
    display_name: &str,
) -> Result<PcbFootprint> {
    let params_path = format!("/{cfb_key}/Parameters");
    let header_path = format!("/{cfb_key}/Header");
    let data_path = format!("/{cfb_key}/Data");

    let params_raw = doc.read_stream(&params_path)?;
    let (pattern, height, description, item_guid, revision_guid, component_kind) =
        parse_parameters_stream(&params_raw).with_context(|| format!("parsing {params_path}"))?;

    let header_raw = doc.read_stream(&header_path)?;
    let expected_count = parse_pcb_section_header(&header_raw)
        .with_context(|| format!("parsing {header_path}"))? as usize;

    let data_raw = doc.read_stream(&data_path)?;
    let (data_pattern, primitives_vec) =
        parse_pcblib_data_stream(&data_raw).with_context(|| format!("parsing {data_path}"))?;

    if data_pattern != pattern {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "PATTERN".to_owned(),
            detail: format!(
                "Data stream pattern '{}' does not match Parameters PATTERN '{}'",
                data_pattern, pattern
            ),
        });
    }

    if primitives_vec.len() != expected_count {
        return Err(AltiumFormatError::RecordCountMismatch {
            section: format!("{cfb_key}/Data"),
            expected: expected_count,
            actual: primitives_vec.len(),
        });
    }

    let mut footprint = PcbFootprint {
        display_name: display_name.to_owned(),
        cfb_key: cfb_key.to_owned(),
        pattern,
        height,
        description,
        item_guid,
        revision_guid,
        component_kind,
        primitives: primitives_vec,
        extended_primitive_info: Vec::new(),
        primitive_guids: Vec::new(),
        custom_shapes: Vec::new(),
        custom_mask_shapes: Vec::new(),
        corner_radius_chamfer: Vec::new(),
        shared_unions: Vec::new(),
    };

    load_sidecars(doc, cfb_key, &mut footprint)
        .with_context(|| format!("loading sidecars for /{cfb_key}"))?;

    // Mark the footprint storage node itself as consumed.
    doc.consume_storage(&format!("/{cfb_key}"));

    Ok(footprint)
}

fn parse_parameters_stream(
    data: &[u8],
) -> Result<(String, Coord, String, String, String, Option<i32>)> {
    let mut reader = BinaryReader::new(data);
    let str_len = reader.read_u32_le()? as usize;
    let str_bytes = reader.read_bytes(str_len)?;
    reader.assert_exhausted()?;
    let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(str_bytes);
    let mut params = ParameterCollection::from_str(&decoded)?;

    // Apply UNICODE sidecars first so field values contain true Unicode text.
    params.apply_unicode_sidecars()?;

    let pattern = params.remove_required::<String>("PATTERN")?;
    let height = parse_height_mil(&mut params)?;
    let description = params
        .remove_optional::<String>("DESCRIPTION")?
        .unwrap_or_default();
    let item_guid = params
        .remove_optional::<String>("ITEMGUID")?
        .unwrap_or_default();
    let revision_guid = params
        .remove_optional::<String>("REVISIONGUID")?
        .unwrap_or_default();
    let component_kind = params.remove_optional::<i32>("COMPONENTKIND")?;
    // Known optional metadata parameters in footprint Parameters streams.
    let _area = params.remove_optional::<String>("AREA")?;
    let _title = params.remove_optional::<String>("TITLE")?;
    let _grid_sn_guide = params.remove_optional::<String>("GRIDSNGUIDE")?;
    // Optional smart-union metadata appears in some vendor libraries.
    let _smart_union_storage = params.remove_optional::<String>("SMARTUNIONSSTORAGE")?;
    let _smart_union_items = params.remove_prefixed("SMARTUNION_");
    params.assert_exhausted()?;
    Ok((
        pattern,
        height,
        description,
        item_guid,
        revision_guid,
        component_kind,
    ))
}

/// Parses the HEIGHT parameter from PcbLib footprint Parameters.
///
/// Format: floating-point mils value with "mil" suffix, e.g. "59.0551mil", "0mil".
/// Some files use comma as decimal separator (e.g. "15,748mil").
fn parse_height_mil(params: &mut ParameterCollection) -> Result<Coord> {
    let raw: Option<String> = params.remove_optional("HEIGHT")?;
    match raw {
        None => Ok(Coord::ZERO),
        Some(s) => {
            let trimmed = s.strip_suffix("mil").unwrap_or(&s);
            // Handle comma decimal separator (locale-dependent Altium installs)
            let normalized = trimmed.replace(',', ".");
            let mils: f64 = normalized.parse().map_err(|e: std::num::ParseFloatError| {
                AltiumFormatError::InvalidParamValue {
                    key: "HEIGHT".to_owned(),
                    detail: format!("cannot parse HEIGHT '{}': {}", s, e),
                }
            })?;
            Ok(Coord::from_mils_f64(mils))
        }
    }
}

/// Returns the number of subrecords for a given PcbLib primitive type.
///
/// Pad (6 subrecords) and Text (2 subrecords) use multi-subrecord format;
/// all other types use a single subrecord.
fn subrecord_count(object_id: PcbObjectId) -> usize {
    match object_id {
        PcbObjectId::Pad => PAD_SUBRECORD_COUNT,
        PcbObjectId::Text => TEXT_SUBRECORD_COUNT,
        _ => DEFAULT_SUBRECORD_COUNT,
    }
}

fn parse_pcblib_data_stream(data: &[u8]) -> Result<(String, Vec<crate::pcblib::PcbPrimitive>)> {
    let mut reader = BinaryReader::new(data);

    let block_len = reader.read_u32_le()? as usize;
    let mut name_block = reader.sub_reader(block_len)?;
    let str_len = name_block.read_u8()? as usize;
    let name_bytes = name_block.read_bytes(str_len)?;
    let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(name_bytes);
    let pattern_name = decoded.into_owned();
    name_block.assert_exhausted()?;

    let mut records = Vec::new();
    while reader.remaining() > 0 {
        let record_offset = reader.position();
        let type_byte = reader.read_u8()?;
        let object_id = PcbObjectId::try_from(type_byte)?;
        let n_subrecords = subrecord_count(object_id);

        let mut subrecords: Vec<&[u8]> = Vec::with_capacity(n_subrecords);
        for _ in 0..n_subrecords {
            let encoded_len = reader.read_u32_le()?;
            let payload_len = (encoded_len & BLOCK_SIZE_MASK) as usize;
            let payload = reader.read_bytes(payload_len)?;
            subrecords.push(payload);
        }
        let primitive =
            primitives::dispatch_primitive(object_id, &subrecords).with_context(|| {
                format!(
                    "primitive #{} ({:?}) at Data offset 0x{:X} ({} subrecords)",
                    records.len(),
                    object_id,
                    record_offset,
                    n_subrecords,
                )
            })?;
        records.push(primitive);
    }
    reader.assert_exhausted()?;

    Ok((pattern_name, records))
}

/// Loads all optional sidecar streams for a footprint and merges their data.
fn load_sidecars(
    doc: &mut TrackedCfbDocument,
    cfb_key: &str,
    footprint: &mut PcbFootprint,
) -> Result<()> {
    // WideStrings: optional, single stream.
    let wide_strings = match doc.read_stream_optional(&format!("/{cfb_key}/WideStrings"))? {
        Some(data) => parse_pcblib_wide_strings(&data)?,
        None => std::collections::HashMap::new(),
    };

    // UniqueIDPrimitiveInformation: optional Header/Data substorage.
    let unique_ids = if doc.exists(&format!("/{cfb_key}/UniqueIDPrimitiveInformation/Header")) {
        let header_data =
            doc.read_stream(&format!("/{cfb_key}/UniqueIDPrimitiveInformation/Header"))?;
        let data = doc.read_stream(&format!("/{cfb_key}/UniqueIDPrimitiveInformation/Data"))?;
        // Mark the parent storage node as consumed.
        doc.consume_storage(&format!("/{cfb_key}/UniqueIDPrimitiveInformation"));
        parse_unique_id_primitive_information(&header_data, &data)?
    } else {
        // Ensure the optional stream path is marked consumed even when absent.
        let _ =
            doc.read_stream_optional(&format!("/{cfb_key}/UniqueIDPrimitiveInformation/Header"))?;
        let _ =
            doc.read_stream_optional(&format!("/{cfb_key}/UniqueIDPrimitiveInformation/Data"))?;
        vec![]
    };

    // ExtendedPrimitiveInformation: optional Header/Data substorage.
    let extended_info = if doc.exists(&format!("/{cfb_key}/ExtendedPrimitiveInformation/Header")) {
        let header_data =
            doc.read_stream(&format!("/{cfb_key}/ExtendedPrimitiveInformation/Header"))?;
        let data = doc.read_stream(&format!("/{cfb_key}/ExtendedPrimitiveInformation/Data"))?;
        doc.consume_storage(&format!("/{cfb_key}/ExtendedPrimitiveInformation"));
        parse_extended_primitive_information(&header_data, &data)?
    } else {
        let _ =
            doc.read_stream_optional(&format!("/{cfb_key}/ExtendedPrimitiveInformation/Header"))?;
        let _ =
            doc.read_stream_optional(&format!("/{cfb_key}/ExtendedPrimitiveInformation/Data"))?;
        vec![]
    };
    validate_extended_entries(&footprint.primitives, &extended_info)?;
    footprint.extended_primitive_info = extended_info;

    // PrimitiveGuids: optional Header/Data substorage.
    if doc.exists(&format!("/{cfb_key}/PrimitiveGuids/Header")) {
        let header_data = doc.read_stream(&format!("/{cfb_key}/PrimitiveGuids/Header"))?;
        let data = doc.read_stream(&format!("/{cfb_key}/PrimitiveGuids/Data"))?;
        doc.consume_storage(&format!("/{cfb_key}/PrimitiveGuids"));
        footprint.primitive_guids = parse_primitive_guids(&header_data, &data)?;
    } else {
        doc.read_stream_optional(&format!("/{cfb_key}/PrimitiveGuids/Header"))?;
        doc.read_stream_optional(&format!("/{cfb_key}/PrimitiveGuids/Data"))?;
    }

    // CustomShapes: optional single stream.
    footprint.custom_shapes = match doc.read_stream_optional(&format!("/{cfb_key}/CustomShapes"))? {
        Some(data) => parse_custom_shapes(&data)
            .with_context(|| format!("parsing /{cfb_key}/CustomShapes"))?,
        None => vec![],
    };

    // CustomMaskShapes: optional single stream.
    footprint.custom_mask_shapes =
        match doc.read_stream_optional(&format!("/{cfb_key}/CustomMaskShapes"))? {
            Some(data) => parse_custom_mask_shapes(&data)
                .with_context(|| format!("parsing /{cfb_key}/CustomMaskShapes"))?,
            None => vec![],
        };

    // CornerRadiusChamfer: optional single stream.
    footprint.corner_radius_chamfer =
        match doc.read_stream_optional(&format!("/{cfb_key}/CornerRadiusChamfer"))? {
            Some(data) => parse_corner_radius_chamfer(&data)
                .with_context(|| format!("parsing /{cfb_key}/CornerRadiusChamfer"))?,
            None => vec![],
        };

    // Validate that custom shape entries reference valid pad primitives.
    validate_custom_shape_entries(
        &footprint.primitives,
        &footprint.custom_shapes,
        &footprint.custom_mask_shapes,
        &footprint.corner_radius_chamfer,
    )?;

    // SharedUnion: optional single stream.
    footprint.shared_unions = match doc.read_stream_optional(&format!("/{cfb_key}/SharedUnion"))? {
        Some(data) => crate::shared_union::parse_shared_union_stream(&data)
            .with_context(|| format!("parsing /{cfb_key}/SharedUnion"))?,
        None => Vec::new(),
    };

    merge_sidecars(&mut footprint.primitives, wide_strings, unique_ids)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_io::BinaryWriter;
    use crate::pcblib::PcbPrimitive;
    use altium_format_types::{Coord, CoordPoint};

    fn make_parameters_stream(pattern: &str) -> Vec<u8> {
        let param_str = format!("|PATTERN={pattern}|\x00");
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(&param_str);
        let mut w = BinaryWriter::new();
        w.write_u32_le(encoded.len() as u32);
        w.write_bytes(&encoded);
        w.finish()
    }

    fn make_header_stream(count: u32) -> Vec<u8> {
        let mut w = BinaryWriter::new();
        w.write_u32_le(count);
        w.finish()
    }

    fn make_data_stream_with_pattern(pattern: &str) -> Vec<u8> {
        let name_bytes = pattern.as_bytes();
        let str_len = name_bytes.len() as u8;
        let block_len = 1 + name_bytes.len() as u32;
        let mut w = BinaryWriter::new();
        w.write_u32_le(block_len);
        w.write_u8(str_len);
        w.write_bytes(name_bytes);
        w.finish()
    }

    #[test]
    fn parse_parameters_stream_extracts_pattern() {
        let data = make_parameters_stream("TestFootprint");
        let (pattern, height, description, item_guid, revision_guid, component_kind) =
            parse_parameters_stream(&data).unwrap();
        assert_eq!(pattern, "TestFootprint");
        assert_eq!(height, Coord::ZERO);
        assert!(description.is_empty());
        assert!(item_guid.is_empty());
        assert!(revision_guid.is_empty());
        assert!(component_kind.is_none());
    }

    #[test]
    fn parse_pcblib_data_stream_empty_records() {
        let data = make_data_stream_with_pattern("MyPattern");
        let (name, records) = parse_pcblib_data_stream(&data).unwrap();
        assert_eq!(name, "MyPattern");
        assert!(records.is_empty());
    }

    #[test]
    fn parse_pcblib_data_stream_with_arc() {
        let mut w = BinaryWriter::new();
        let pattern = "ArcTest";
        let name_bytes = pattern.as_bytes();
        let block_len = 1 + name_bytes.len() as u32;
        w.write_u32_le(block_len);
        w.write_u8(name_bytes.len() as u8);
        w.write_bytes(name_bytes);

        // Arc record: type byte + u32 length + payload
        // Build arc payload (56 bytes: 13 common + 32 fields + 11 trailing)
        let mut arc_payload = BinaryWriter::new();
        arc_payload.write_u8(1); // layer
        arc_payload.write_u8(0); // pad_byte
        arc_payload.write_u16_le(0); // flags
        arc_payload.write_i32_le(-1); // net_index
        arc_payload.write_u16_le(0xFFFF); // polygon_index
        arc_payload.write_u16_le(0); // component_index
        arc_payload.write_u8(0); // unknown
        arc_payload.write_coord_point(CoordPoint::new(
            Coord::from_internal(10_000),
            Coord::from_internal(20_000),
        ));
        arc_payload.write_coord(Coord::from_internal(5_000)); // radius
        arc_payload.write_f64_le(0.0); // start_angle
        arc_payload.write_f64_le(360.0); // end_angle
        arc_payload.write_coord(Coord::from_internal(1_000)); // width
        arc_payload.write_u16_le(0xFFFF); // subpoly_index
        arc_payload.write_u8(0); // user_routed
        arc_payload.write_i32_le(0); // union_index
        arc_payload.write_u32_le(0); // v7_layer
        let arc_bytes = arc_payload.finish();

        w.write_u8(PcbObjectId::Arc as u8);
        w.write_u32_le(arc_bytes.len() as u32);
        w.write_bytes(&arc_bytes);

        let data = w.finish();
        let (name, records) = parse_pcblib_data_stream(&data).unwrap();
        assert_eq!(name, "ArcTest");
        assert_eq!(records.len(), 1);
        assert!(matches!(records[0], PcbPrimitive::Arc(_)));
    }

    #[test]
    fn record_count_mismatch_returns_error() {
        // Header says 5 records, Data has 0
        let params_data = make_parameters_stream("FP");
        let header_data = make_header_stream(5);
        let data_data = make_data_stream_with_pattern("FP");

        let (pattern, _height, _description, _item_guid, _revision_guid, _component_kind) =
            parse_parameters_stream(&params_data).unwrap();
        let expected_count = parse_pcb_section_header(&header_data).unwrap() as usize;
        let (data_pattern, primitives_vec) = parse_pcblib_data_stream(&data_data).unwrap();

        assert_eq!(data_pattern, pattern);
        let err = if primitives_vec.len() != expected_count {
            Some(AltiumFormatError::RecordCountMismatch {
                section: "test/Data".to_owned(),
                expected: expected_count,
                actual: primitives_vec.len(),
            })
        } else {
            None
        };
        assert!(err.is_some());
        assert!(matches!(
            err.unwrap(),
            AltiumFormatError::RecordCountMismatch {
                expected: 5,
                actual: 0,
                ..
            }
        ));
    }
}

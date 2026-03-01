use crate::binary_io::BinaryReader;
use crate::param_collection::ParameterCollection;
use crate::prefixed_param_stream::parse_prefixed_param_blocks;
use crate::{AltiumFormatError, Result, ResultExt};
use altium_format_types::{CoordPoint, V6Layer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PrimitiveSectionKind {
    Arcs6,
    Pads6,
    Vias6,
    Tracks6,
    Texts6,
    Fills6,
    Regions6,
    ShapeBasedRegions6,
    ComponentBodies6,
    ShapeBasedComponentBodies6,
    BoardRegions,
    Texts,
}

impl PrimitiveSectionKind {
    pub(crate) fn from_storage_name(name: &str) -> Option<Self> {
        match name {
            "Arcs6" => Some(Self::Arcs6),
            "Pads6" => Some(Self::Pads6),
            "Vias6" => Some(Self::Vias6),
            "Tracks6" => Some(Self::Tracks6),
            "Texts6" => Some(Self::Texts6),
            "Fills6" => Some(Self::Fills6),
            "Regions6" => Some(Self::Regions6),
            "ShapeBasedRegions6" => Some(Self::ShapeBasedRegions6),
            "ComponentBodies6" => Some(Self::ComponentBodies6),
            "ShapeBasedComponentBodies6" => Some(Self::ShapeBasedComponentBodies6),
            "BoardRegions" => Some(Self::BoardRegions),
            "Texts" => Some(Self::Texts),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParamSectionKind {
    Board6,
    Nets6,
    Components6,
    Polygons6,
    Classes6,
    DifferentialPairs6,
    FromTos6,
    EmbeddedBoards6,
    Embeddeds6,
    UniqueIdPrimitiveInformation,
    ExtendedPrimitiveInformation,
    PadViaLibrary,
    PadViaLibraryCache,
    PadViaLibraryLinks,
    PinPairsSection,
    SignalClasses,
    SmartUnions,
    WaivedViolations,
    AdvancedPlacerOptions6,
    AdvancedRouterOptions6,
    DesignRuleCheckerOptions6,
    PinSwapOptions6,
    FileVersionInfo,
    LayerKindMapping,
    ModelsNoEmbed,
    Textures,
    CustomShapes,
    TClearanceViolation,
    TShortCircuitViolation,
    TSilkToSilkClearanceViolation,
    TRoutingViaStyleViolation,
    TMinSolderMaskSliverViolation,
    TModifiedPolygonViolation,
    TNetAntennaeViolation,
    TDiffPairsViolation,
    TBoardOutlineClearanceViolation,
    TUnconnectedPinViolation,
    TSilkToSolderMaskClearanceViolation,
    TMinimumAnnularRingViolation,
    TComponentClearanceViolation,
    TMaxMinLengthViolation,
    TDisconnectedSubnetsViolation,
    TMatchedNetLengthsViolation,
    THoleToHoleViolation,
    TMaxMinComponentHeightViolation,
    CustomMaskShapes,
    CornerRadiusChamfer,
    ViaStructures,
    ViaStructureManager,
}

impl ParamSectionKind {
    pub(crate) fn from_storage_name(name: &str) -> Option<Self> {
        match name {
            "Board6" => Some(Self::Board6),
            "Nets6" => Some(Self::Nets6),
            "Components6" => Some(Self::Components6),
            "Polygons6" => Some(Self::Polygons6),
            "Classes6" => Some(Self::Classes6),
            "DifferentialPairs6" => Some(Self::DifferentialPairs6),
            "FromTos6" => Some(Self::FromTos6),
            "EmbeddedBoards6" => Some(Self::EmbeddedBoards6),
            "Embeddeds6" => Some(Self::Embeddeds6),
            "UniqueIDPrimitiveInformation" => Some(Self::UniqueIdPrimitiveInformation),
            "ExtendedPrimitiveInformation" => Some(Self::ExtendedPrimitiveInformation),
            "PadViaLibrary" => Some(Self::PadViaLibrary),
            "PadViaLibraryCache" => Some(Self::PadViaLibraryCache),
            "PadViaLibraryLinks" => Some(Self::PadViaLibraryLinks),
            "PinPairsSection" => Some(Self::PinPairsSection),
            "SignalClasses" => Some(Self::SignalClasses),
            "SmartUnions" => Some(Self::SmartUnions),
            "WaivedViolations" => Some(Self::WaivedViolations),
            "Advanced Placer Options6" => Some(Self::AdvancedPlacerOptions6),
            "Advanced Router Options6" => Some(Self::AdvancedRouterOptions6),
            "Design Rule Checker Options6" => Some(Self::DesignRuleCheckerOptions6),
            "Pin Swap Options6" => Some(Self::PinSwapOptions6),
            "FileVersionInfo" => Some(Self::FileVersionInfo),
            "LayerKindMapping" => Some(Self::LayerKindMapping),
            "ModelsNoEmbed" => Some(Self::ModelsNoEmbed),
            "Textures" => Some(Self::Textures),
            "CustomShapes" => Some(Self::CustomShapes),
            "TClearanceViolation" => Some(Self::TClearanceViolation),
            "TShortCircuitViolation" => Some(Self::TShortCircuitViolation),
            "TSilkToSilkClearanceViolation" => Some(Self::TSilkToSilkClearanceViolation),
            "TRoutingViaStyleViolation" => Some(Self::TRoutingViaStyleViolation),
            "TMinSolderMaskSliverViolation" => Some(Self::TMinSolderMaskSliverViolation),
            "TModifiedPolygonViolation" => Some(Self::TModifiedPolygonViolation),
            "TNetAntennaeViolation" => Some(Self::TNetAntennaeViolation),
            "TDiffPairsViolation" => Some(Self::TDiffPairsViolation),
            "TBoardOutlineClearanceViolation" => Some(Self::TBoardOutlineClearanceViolation),
            "TUnconnectedPinViolation" => Some(Self::TUnconnectedPinViolation),
            // CFB V3 truncates storage names to 31 chars, so the full name
            // "TSilkToSolderMaskClearanceViolation" appears truncated.
            "TSilkToSolderMaskClearanceViola" => Some(Self::TSilkToSolderMaskClearanceViolation),
            "TMinimumAnnularRingViolation" => Some(Self::TMinimumAnnularRingViolation),
            "TComponentClearanceViolation" => Some(Self::TComponentClearanceViolation),
            "TMaxMinLengthViolation" => Some(Self::TMaxMinLengthViolation),
            "TDisconnectedSubnetsViolation" => Some(Self::TDisconnectedSubnetsViolation),
            "TMatchedNetLengthsViolation" => Some(Self::TMatchedNetLengthsViolation),
            "THoleToHoleViolation" => Some(Self::THoleToHoleViolation),
            "TMaxMinComponentHeightViolation" => Some(Self::TMaxMinComponentHeightViolation),
            "CustomMaskShapes" => Some(Self::CustomMaskShapes),
            "CornerRadiusChamfer" => Some(Self::CornerRadiusChamfer),
            "ViaStructures" => Some(Self::ViaStructures),
            "ViaStructureManager" => Some(Self::ViaStructureManager),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BinaryLenSectionKind {
    Connections6,
}

impl BinaryLenSectionKind {
    pub(crate) fn from_storage_name(name: &str) -> Option<Self> {
        match name {
            "Connections6" => Some(Self::Connections6),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PrefixedParamSectionKind {
    Rules6,
    NewRules6,
    Dimensions6,
    Coordinates6,
}

impl PrefixedParamSectionKind {
    pub(crate) fn from_storage_name(name: &str) -> Option<Self> {
        match name {
            "Rules6" => Some(Self::Rules6),
            "NewRules6" => Some(Self::NewRules6),
            "Dimensions6" => Some(Self::Dimensions6),
            "Coordinates6" => Some(Self::Coordinates6),
            _ => None,
        }
    }
}

pub(crate) struct StandardParamRecord {
    pub(crate) params: ParameterCollection,
}

pub(crate) struct PrefixedParamRecord {
    pub(crate) prefix: u16,
    pub(crate) params: ParameterCollection,
}

pub(crate) struct BinaryLenRecord {
    pub(crate) common: ConnectionCommonHeader,
    pub(crate) from: CoordPoint,
    pub(crate) to: CoordPoint,
    pub(crate) from_layer: V6Layer,
    pub(crate) to_layer: V6Layer,
    pub(crate) connection_layer_enum: i32,
    pub(crate) from_layer_enum: i32,
    pub(crate) to_layer_enum: i32,
}

pub(crate) struct ConnectionCommonHeader {
    pub(crate) layer: V6Layer,
    pub(crate) flags: u16,
    pub(crate) net_index: i16,
    pub(crate) unknown_1: i16,
    pub(crate) component_index: i16,
    pub(crate) polygon_index: i16,
    pub(crate) unknown_2: i16,
}

pub(crate) struct WideString6Record {
    pub(crate) index: u32,
    pub(crate) text: String,
}

pub(crate) struct UnionNameRecord {
    pub(crate) union_index: u32,
    pub(crate) name: String,
}

pub(crate) fn parse_standard_param_records(data: &[u8]) -> Result<Vec<StandardParamRecord>> {
    let mut reader = BinaryReader::new(data);
    let mut out = Vec::new();
    while reader.remaining() > 0 {
        let size = reader.read_u32_le()? as usize;
        let payload = reader.read_bytes(size)?;
        let params = match ParameterCollection::from_bytes(payload) {
            Ok(v) => v,
            Err(AltiumFormatError::InvalidParamValue { detail, .. })
                if detail == "segment has no '=' separator" =>
            {
                let repaired = repair_param_payload_with_bare_flags(payload);
                ParameterCollection::from_bytes(&repaired)?
            }
            Err(e) => return Err(e),
        };
        out.push(StandardParamRecord { params });
    }
    reader.assert_exhausted()?;
    Ok(out)
}

fn repair_param_payload_with_bare_flags(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 16);
    for segment in payload.split(|b| *b == b'|') {
        if segment.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(b'|');
        }
        out.extend_from_slice(segment);
        if !segment.contains(&b'=') {
            out.extend_from_slice(b"=1");
        }
    }
    out
}

pub(crate) fn parse_len_prefixed_binary_records(data: &[u8]) -> Result<Vec<BinaryLenRecord>> {
    let mut reader = BinaryReader::new(data);
    let mut out = Vec::new();
    while reader.remaining() > 0 {
        let size = reader.read_u32_le()? as usize;
        let payload = reader.read_bytes(size)?;
        if size != 43 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "Connections6/Data".to_owned(),
                detail: format!("expected 43-byte connection payload, got {size}"),
            });
        }

        let mut payload_reader = BinaryReader::new(payload);
        let common = ConnectionCommonHeader {
            layer: V6Layer::try_from(payload_reader.read_u8()?)?,
            flags: payload_reader.read_u16_le()?,
            net_index: payload_reader.read_i16_le()?,
            unknown_1: payload_reader.read_i16_le()?,
            component_index: payload_reader.read_i16_le()?,
            polygon_index: payload_reader.read_i16_le()?,
            unknown_2: payload_reader.read_i16_le()?,
        };
        let from = payload_reader.read_coord_point()?;
        let to = payload_reader.read_coord_point()?;
        let from_layer = V6Layer::try_from(payload_reader.read_u8()?)?;
        let to_layer = V6Layer::try_from(payload_reader.read_u8()?)?;
        let connection_layer_enum = payload_reader.read_i32_le()?;
        let from_layer_enum = payload_reader.read_i32_le()?;
        let to_layer_enum = payload_reader.read_i32_le()?;
        payload_reader.assert_exhausted()?;

        out.push(BinaryLenRecord {
            common,
            from,
            to,
            from_layer,
            to_layer,
            connection_layer_enum,
            from_layer_enum,
            to_layer_enum,
        });
    }
    reader.assert_exhausted()?;
    Ok(out)
}

pub(crate) fn parse_wide_strings6_records(data: &[u8]) -> Result<Vec<WideString6Record>> {
    let mut out = Vec::new();
    let mut offset = 0usize;

    while offset < data.len() {
        let expected_index = out.len() as u32;

        // Format A (common): [u32 index][u32 utf16_byte_len][bytes...]
        let try_full = if offset + 8 <= data.len() {
            let index_bytes = &data[offset..offset + 4];
            let index = u32::from_le_bytes([
                index_bytes[0],
                index_bytes[1],
                index_bytes[2],
                index_bytes[3],
            ]);
            let len_bytes = &data[offset + 4..offset + 8];
            let byte_len =
                u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]])
                    as usize;
            let data_off = offset + 8;
            (index, byte_len, data_off)
        } else {
            (u32::MAX, usize::MAX, usize::MAX)
        };

        let mut parsed = None;
        if try_full.0 == expected_index
            && (try_full.1 % 2) == 0
            && try_full.2 <= data.len()
            && try_full.1 <= (data.len() - try_full.2)
        {
            parsed = Some((try_full.0, try_full.1, try_full.2));
        } else if offset + 6 <= data.len() {
            // Format B (observed variant): [u16=0][u32 utf16_byte_len][bytes...]
            // Index is implicit and must equal the next sequential index.
            let sentinel_bytes = &data[offset..offset + 2];
            let sentinel = u16::from_le_bytes([sentinel_bytes[0], sentinel_bytes[1]]);
            let len_bytes = &data[offset + 2..offset + 6];
            let byte_len =
                u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]])
                    as usize;
            let data_off = offset + 6;
            if sentinel == 0
                && (byte_len % 2) == 0
                && data_off <= data.len()
                && byte_len <= (data.len() - data_off)
            {
                parsed = Some((expected_index, byte_len, data_off));
            }
        }

        let (index, size, payload_off) = parsed.ok_or_else(|| {
            let remaining = data.len() - offset;
            let sample_len = remaining.min(16);
            let sample = &data[offset..offset + sample_len];
            AltiumFormatError::InvalidParamValue {
                key: "WideStrings6/Data".to_owned(),
                detail: format!(
                    "cannot decode entry at offset {offset} (expected index {expected_index}); next bytes {:02x?}",
                    sample
                ),
            }
        })?;

        let payload = &data[payload_off..payload_off + size];
        let (decoded, _, had_errors) = encoding_rs::UTF_16LE.decode(payload);
        if had_errors {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "WideStrings6/Data".to_owned(),
                detail: format!("invalid UTF-16LE sequence for string index {index}"),
            });
        }

        out.push(WideString6Record {
            index,
            text: decoded.trim_end_matches('\0').to_owned(),
        });
        offset = payload_off + size;
    }

    Ok(out)
}

pub(crate) fn parse_prefixed_param_records(data: &[u8]) -> Result<Vec<PrefixedParamRecord>> {
    let mut out = Vec::new();
    for block in parse_prefixed_param_blocks(data)? {
        let params = ParameterCollection::from_bytes(&block.data).map_err(|e| {
            AltiumFormatError::WithContext {
                context: format!(
                    "decoding prefixed parameter block with prefix {}",
                    block.prefix
                ),
                source: Box::new(e),
            }
        })?;
        out.push(PrefixedParamRecord {
            prefix: block.prefix,
            params,
        });
    }
    Ok(out)
}

pub(crate) fn parse_union_name_records(data: &[u8]) -> Result<Vec<UnionNameRecord>> {
    let mut reader = BinaryReader::new(data);
    let count = reader.read_u32_le()? as usize;
    let mut out = Vec::with_capacity(count);

    for _ in 0..count {
        let union_index = reader.read_u32_le()?;
        let byte_len = reader.read_u32_le()? as usize;
        if (byte_len % 2) != 0 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "UnionNames/Data".to_owned(),
                detail: format!("UTF-16LE byte length must be even, got {byte_len}"),
            });
        }
        let payload = reader.read_bytes(byte_len)?;
        let (decoded, _, had_errors) = encoding_rs::UTF_16LE.decode(payload);
        if had_errors {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "UnionNames/Data".to_owned(),
                detail: format!("invalid UTF-16LE for union index {union_index}"),
            });
        }

        out.push(UnionNameRecord {
            union_index,
            name: decoded.trim_end_matches('\0').to_owned(),
        });
    }

    reader.assert_exhausted()?;
    Ok(out)
}

/// An indexed parameter record for sections like UnionFeatures.
/// Each record has a u32 index prefix before the standard u32 len + payload.
pub(crate) struct IndexedParamRecord {
    pub(crate) index: u32,
    pub(crate) params: ParameterCollection,
}

/// A hierarchical group for SharedUnion: a header block with HIDDENPRIMITIVESCOUNT=N
/// followed by N detail blocks describing the hidden primitives.
pub(crate) struct SharedUnionParamGroup {
    pub(crate) header: ParameterCollection,
    pub(crate) hidden_primitives: Vec<ParameterCollection>,
}

/// A component group within the PrimitiveParameters section.
/// Each group has a component header block (with PRIMITIVEID, COUNT, etc.)
/// followed by COUNT parameter blocks (each with NAME, VALUE, ISIMPORTED).
pub(crate) struct PrimitiveParameterGroup {
    pub(crate) component_header: ParameterCollection,
    pub(crate) parameters: Vec<ParameterCollection>,
}

pub(crate) struct UnionRelationRecord {
    pub(crate) parent_id: i32,
    pub(crate) child_id: i32,
}

/// Parses UnionRelations Data stream: binary i32 pairs (parent_id, child_id) until exhausted.
pub(crate) fn parse_union_relation_records(data: &[u8]) -> Result<Vec<UnionRelationRecord>> {
    let mut reader = BinaryReader::new(data);
    let mut out = Vec::new();
    while reader.remaining() > 0 {
        let parent_id = reader.read_i32_le()?;
        let child_id = reader.read_i32_le()?;
        out.push(UnionRelationRecord {
            parent_id,
            child_id,
        });
    }
    reader.assert_exhausted()?;
    Ok(out)
}

/// Parses PrimitiveParameters Data stream with hierarchical grouping.
///
/// Format: repeating groups of [component header block] + [N parameter blocks].
/// The component header has `COUNT=N` indicating how many parameter blocks follow.
/// The Header u32 count is the number of component groups, NOT total blocks.
pub(crate) fn parse_primitive_parameter_records(
    data: &[u8],
) -> Result<Vec<PrimitiveParameterGroup>> {
    let mut reader = BinaryReader::new(data);
    let mut groups = Vec::new();

    while reader.remaining() > 0 {
        let group_index = groups.len();

        // Read component header block: [u32 len][NUL-terminated param string]
        let header_size = reader.read_u32_le()? as usize;
        let header_payload = reader.read_bytes(header_size)?;
        let mut header_params = ParameterCollection::from_bytes(header_payload)
            .with_context(|| {
                format!("PrimitiveParameters group {group_index} component header")
            })?;

        // Extract COUNT to know how many parameter blocks follow
        let count: usize = header_params.remove_required("COUNT").with_context(|| {
            format!("PrimitiveParameters group {group_index} missing COUNT key")
        })?;

        // Read COUNT parameter blocks
        let mut parameters = Vec::with_capacity(count);
        for i in 0..count {
            let param_size = reader.read_u32_le()? as usize;
            let param_payload = reader.read_bytes(param_size)?;
            let params = ParameterCollection::from_bytes(param_payload).with_context(|| {
                format!("PrimitiveParameters group {group_index}, parameter block {i}")
            })?;
            parameters.push(params);
        }

        groups.push(PrimitiveParameterGroup {
            component_header: header_params,
            parameters,
        });
    }

    reader.assert_exhausted()?;
    Ok(groups)
}

/// Parses an indexed parameter section like UnionFeatures.
/// Format: `[u32 index][u32 param_len][param_payload]` × N records.
/// The index is a reference (e.g. union index) identifying what the record applies to.
pub(crate) fn parse_indexed_param_records(data: &[u8]) -> Result<Vec<IndexedParamRecord>> {
    let mut reader = BinaryReader::new(data);
    let mut out = Vec::new();

    while reader.remaining() > 0 {
        let index = reader.read_u32_le()?;
        let size = reader.read_u32_le()? as usize;
        let payload = reader.read_bytes(size)?;
        let params = ParameterCollection::from_bytes(payload).with_context(|| {
            format!("indexed param record {} (index={index})", out.len())
        })?;
        out.push(IndexedParamRecord { index, params });
    }

    reader.assert_exhausted()?;
    Ok(out)
}

/// Parses SharedUnion Data stream with hierarchical grouping.
///
/// Three observed variants:
/// 1. Header has `HIDDENPRIMITIVESCOUNT=N`: N detail blocks follow the header
/// 2. Header has `PRIMITIVESCOUNT=N` with inline `REFnINDEX`/`REFnOBJID`: no child blocks
/// 3. Header has neither count key: no child blocks
///
/// Format: `[u32 len][header block]` optionally followed by
/// `[u32 len][detail block]` × HIDDENPRIMITIVESCOUNT.
pub(crate) fn parse_shared_union_param_records(
    data: &[u8],
) -> Result<Vec<SharedUnionParamGroup>> {
    let mut reader = BinaryReader::new(data);
    let mut groups = Vec::new();

    while reader.remaining() > 0 {
        let group_index = groups.len();

        // Read header block
        let header_size = reader.read_u32_le()? as usize;
        let header_payload = reader.read_bytes(header_size)?;
        let mut header_params =
            ParameterCollection::from_bytes(header_payload).with_context(|| {
                format!("SharedUnion group {group_index} header")
            })?;

        // HIDDENPRIMITIVESCOUNT indicates child detail blocks follow;
        // PRIMITIVESCOUNT uses inline REFnINDEX/REFnOBJID references (no child blocks);
        // absent means no child data at all.
        let count: usize = header_params
            .remove_optional::<usize>("HIDDENPRIMITIVESCOUNT")
            .with_context(|| format!("SharedUnion group {group_index}"))?
            .unwrap_or(0);

        // Read detail blocks
        let mut hidden_primitives = Vec::with_capacity(count);
        for i in 0..count {
            let detail_size = reader.read_u32_le()? as usize;
            let detail_payload = reader.read_bytes(detail_size)?;
            let params =
                ParameterCollection::from_bytes(detail_payload).with_context(|| {
                    format!("SharedUnion group {group_index}, detail block {i}")
                })?;
            hidden_primitives.push(params);
        }

        groups.push(SharedUnionParamGroup {
            header: header_params,
            hidden_primitives,
        });
    }

    reader.assert_exhausted()?;
    Ok(groups)
}

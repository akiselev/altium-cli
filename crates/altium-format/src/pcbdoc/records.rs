use crate::binary_io::BinaryReader;
use crate::param_collection::ParameterCollection;
use crate::prefixed_param_stream::parse_prefixed_param_blocks;
use crate::{AltiumFormatError, Result};
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
    PrimitiveParameters,
    PadViaLibrary,
    PadViaLibraryCache,
    PadViaLibraryLinks,
    PinPairsSection,
    SignalClasses,
    SmartUnions,
    UnionNames,
    UnionRelations,
    WaivedViolations,
    AdvancedPlacerOptions6,
    AdvancedRouterOptions6,
    DesignRuleCheckerOptions6,
    PinSwapOptions6,
    FileVersionInfo,
    LayerKindMapping,
    ModelsNoEmbed,
    Textures,
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
            "PrimitiveParameters" => Some(Self::PrimitiveParameters),
            "PadViaLibrary" => Some(Self::PadViaLibrary),
            "PadViaLibraryCache" => Some(Self::PadViaLibraryCache),
            "PadViaLibraryLinks" => Some(Self::PadViaLibraryLinks),
            "PinPairsSection" => Some(Self::PinPairsSection),
            "SignalClasses" => Some(Self::SignalClasses),
            "SmartUnions" => Some(Self::SmartUnions),
            "UnionNames" => Some(Self::UnionNames),
            "UnionRelations" => Some(Self::UnionRelations),
            "WaivedViolations" => Some(Self::WaivedViolations),
            "Advanced Placer Options6" => Some(Self::AdvancedPlacerOptions6),
            "Advanced Router Options6" => Some(Self::AdvancedRouterOptions6),
            "Design Rule Checker Options6" => Some(Self::DesignRuleCheckerOptions6),
            "Pin Swap Options6" => Some(Self::PinSwapOptions6),
            "FileVersionInfo" => Some(Self::FileVersionInfo),
            "LayerKindMapping" => Some(Self::LayerKindMapping),
            "ModelsNoEmbed" => Some(Self::ModelsNoEmbed),
            "Textures" => Some(Self::Textures),
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

pub(crate) fn parse_standard_param_records(data: &[u8]) -> Result<Vec<StandardParamRecord>> {
    let mut reader = BinaryReader::new(data);
    let mut out = Vec::new();
    while reader.remaining() > 0 {
        let size = reader.read_u32_le()? as usize;
        let payload = reader.read_bytes(size)?;
        let params = ParameterCollection::from_bytes(payload)?;
        out.push(StandardParamRecord { params });
    }
    reader.assert_exhausted()?;
    Ok(out)
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
    let mut reader = BinaryReader::new(data);
    let mut out = Vec::new();
    while reader.remaining() > 0 {
        let index = reader.read_u32_le()?;
        let size = reader.read_u32_le()? as usize;
        if (size % 2) != 0 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "WideStrings6/Data".to_owned(),
                detail: format!("string byte length must be even for UTF-16LE, got {size}"),
            });
        }
        let payload = reader.read_bytes(size)?;
        let (decoded, _, had_errors) = encoding_rs::UTF_16LE.decode(payload);
        if had_errors {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "WideStrings6/Data".to_owned(),
                detail: format!("invalid UTF-16LE sequence for string index {index}"),
            });
        }
        let text = decoded.trim_end_matches('\0').to_owned();
        out.push(WideString6Record { index, text });
    }
    reader.assert_exhausted()?;
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

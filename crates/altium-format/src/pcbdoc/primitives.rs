use altium_format_types::constants::parsing::BLOCK_SIZE_MASK;
use altium_format_types::{Coord, CoordPoint, PcbFlags, PcbObjectId, V6Layer, V7Layer};

use crate::binary_io::BinaryReader;
use crate::{AltiumFormatError, Result};

use super::records::PrimitiveSectionKind;

#[derive(Debug)]
pub(crate) struct PcbPrimitiveCommon {
    pub(crate) layer: V6Layer,
    pub(crate) flags: PcbFlags,
    pub(crate) net_index: i16,
    pub(crate) unknown_1: i16,
    pub(crate) component_index: i16,
    pub(crate) polygon_index: i16,
    pub(crate) unknown_2: i16,
}

#[derive(Debug)]
pub(crate) struct PcbArc {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) center: CoordPoint,
    pub(crate) radius: Coord,
    pub(crate) start_angle: f64,
    pub(crate) end_angle: f64,
    pub(crate) width: Coord,
    pub(crate) subpoly_index: u16,
    pub(crate) user_routed: bool,
    pub(crate) union_index: i32,
    pub(crate) layer_enum_index: V7Layer,
    pub(crate) keepout_restrictions: Option<i32>,
}

#[derive(Debug)]
pub(crate) struct PcbTrack {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) start: CoordPoint,
    pub(crate) end: CoordPoint,
    pub(crate) width: Coord,
    pub(crate) subpoly_index: u16,
    pub(crate) user_routed: bool,
    pub(crate) union_index: i32,
    pub(crate) track_kind: u8,
    pub(crate) layer_enum_index: V7Layer,
    pub(crate) keepout_restrictions: Option<i32>,
}

#[derive(Debug)]
pub(crate) struct PcbFill {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) corner_1: CoordPoint,
    pub(crate) corner_2: CoordPoint,
    pub(crate) rotation: f64,
    pub(crate) user_routed: Option<bool>,
    pub(crate) union_index: Option<i32>,
    pub(crate) layer_enum_index: Option<V7Layer>,
    pub(crate) keepout_restrictions: Option<i32>,
}

#[derive(Debug)]
pub(crate) struct PcbRawPrimitive {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) raw_payload: Vec<u8>,
}

#[derive(Debug)]
pub(crate) enum PcbPrimitive {
    Arc(PcbArc),
    Track(PcbTrack),
    Fill(PcbFill),
    Pad(PcbRawPrimitive),
    Via(PcbRawPrimitive),
    Text(PcbRawPrimitive),
    Region(PcbRawPrimitive),
    ComponentBody(PcbRawPrimitive),
}

#[derive(Debug)]
pub(crate) struct ParsedPrimitiveRecord {
    pub(crate) object_id: PcbObjectId,
    pub(crate) primitive: PcbPrimitive,
}

fn expected_object_id(kind: PrimitiveSectionKind) -> PcbObjectId {
    match kind {
        PrimitiveSectionKind::Arcs6 => PcbObjectId::Arc,
        PrimitiveSectionKind::Pads6 => PcbObjectId::Pad,
        PrimitiveSectionKind::Vias6 => PcbObjectId::Via,
        PrimitiveSectionKind::Tracks6 => PcbObjectId::Track,
        PrimitiveSectionKind::Texts6 => PcbObjectId::Text,
        PrimitiveSectionKind::Fills6 => PcbObjectId::Fill,
        PrimitiveSectionKind::Regions6 => PcbObjectId::Region,
        PrimitiveSectionKind::ShapeBasedRegions6 => PcbObjectId::Region,
        PrimitiveSectionKind::ComponentBodies6 => PcbObjectId::ComponentBody,
        PrimitiveSectionKind::ShapeBasedComponentBodies6 => PcbObjectId::ComponentBody,
        PrimitiveSectionKind::BoardRegions => PcbObjectId::Region,
        PrimitiveSectionKind::Texts => PcbObjectId::Text,
    }
}

pub(crate) fn parse_primitive_records(
    kind: PrimitiveSectionKind,
    data: &[u8],
) -> Result<Vec<ParsedPrimitiveRecord>> {
    let expected = expected_object_id(kind);
    let mut reader = BinaryReader::new(data);
    let mut out = Vec::new();

    while reader.remaining() > 0 {
        let object_id = PcbObjectId::try_from(reader.read_u8()?)?;
        if object_id != expected {
            return Err(AltiumFormatError::InvalidParamValue {
                key: format!("{kind:?} object ID"),
                detail: format!("section expects {:?}, found {:?}", expected, object_id),
            });
        }
        let raw_length = reader.read_u32_le()?;
        let length = (raw_length & BLOCK_SIZE_MASK) as usize;
        let payload = reader.read_bytes(length)?;
        let primitive = parse_primitive_payload(object_id, payload)?;
        out.push(ParsedPrimitiveRecord {
            object_id,
            primitive,
        });
    }

    reader.assert_exhausted()?;
    Ok(out)
}

fn parse_primitive_payload(object_id: PcbObjectId, payload: &[u8]) -> Result<PcbPrimitive> {
    match object_id {
        PcbObjectId::Arc => parse_arc(payload).map(PcbPrimitive::Arc),
        PcbObjectId::Track => parse_track(payload).map(PcbPrimitive::Track),
        PcbObjectId::Fill => parse_fill(payload).map(PcbPrimitive::Fill),
        PcbObjectId::Pad => parse_raw(payload).map(PcbPrimitive::Pad),
        PcbObjectId::Via => parse_raw(payload).map(PcbPrimitive::Via),
        PcbObjectId::Text => parse_raw(payload).map(PcbPrimitive::Text),
        PcbObjectId::Region => parse_raw(payload).map(PcbPrimitive::Region),
        PcbObjectId::ComponentBody => parse_raw(payload).map(PcbPrimitive::ComponentBody),
        other => Err(AltiumFormatError::UnknownObjectId(other as u8)),
    }
}

fn parse_common_header(reader: &mut BinaryReader) -> Result<PcbPrimitiveCommon> {
    let layer = V6Layer::try_from(reader.read_u8()?)?;
    let flags = PcbFlags::new(reader.read_u16_le()?);
    let net_index = reader.read_i16_le()?;
    let unknown_1 = reader.read_i16_le()?;
    let component_index = reader.read_i16_le()?;
    let polygon_index = reader.read_i16_le()?;
    let unknown_2 = reader.read_i16_le()?;

    Ok(PcbPrimitiveCommon {
        layer,
        flags,
        net_index,
        unknown_1,
        component_index,
        polygon_index,
        unknown_2,
    })
}

fn parse_arc(data: &[u8]) -> Result<PcbArc> {
    let mut reader = BinaryReader::new(data);
    let common = parse_common_header(&mut reader)?;
    let center = reader.read_coord_point()?;
    let radius = reader.read_coord()?;
    let start_angle = reader.read_f64_le()?;
    let end_angle = reader.read_f64_le()?;
    let width = reader.read_coord()?;
    let subpoly_index = reader.read_u16_le()?;
    let user_routed = reader.read_u8()? != 0;
    let union_index = reader.read_i32_le()?;
    let layer_enum_index = V7Layer::new(reader.read_u32_le()?);
    let keepout_restrictions = if reader.remaining() >= 4 {
        Some(reader.read_i32_le()?)
    } else {
        None
    };
    reader.assert_exhausted()?;

    Ok(PcbArc {
        common,
        center,
        radius,
        start_angle,
        end_angle,
        width,
        subpoly_index,
        user_routed,
        union_index,
        layer_enum_index,
        keepout_restrictions,
    })
}

fn parse_track(data: &[u8]) -> Result<PcbTrack> {
    let mut reader = BinaryReader::new(data);
    let common = parse_common_header(&mut reader)?;
    let start = reader.read_coord_point()?;
    let end = reader.read_coord_point()?;
    let width = reader.read_coord()?;
    let subpoly_index = reader.read_u16_le()?;
    let user_routed = reader.read_u8()? != 0;
    let union_index = reader.read_i32_le()?;
    let track_kind = reader.read_u8()?;
    let layer_enum_index = V7Layer::new(reader.read_u32_le()?);
    let keepout_restrictions = if reader.remaining() >= 4 {
        Some(reader.read_i32_le()?)
    } else {
        None
    };
    reader.assert_exhausted()?;

    Ok(PcbTrack {
        common,
        start,
        end,
        width,
        subpoly_index,
        user_routed,
        union_index,
        track_kind,
        layer_enum_index,
        keepout_restrictions,
    })
}

fn parse_fill(data: &[u8]) -> Result<PcbFill> {
    let mut reader = BinaryReader::new(data);
    let common = parse_common_header(&mut reader)?;
    let corner_1 = reader.read_coord_point()?;
    let corner_2 = reader.read_coord_point()?;
    let rotation = reader.read_f64_le()?;
    let user_routed = if reader.remaining() >= 1 {
        Some(reader.read_u8()? != 0)
    } else {
        None
    };
    let union_index = if reader.remaining() >= 4 {
        Some(reader.read_i32_le()?)
    } else {
        None
    };
    let layer_enum_index = if reader.remaining() >= 4 {
        Some(V7Layer::new(reader.read_u32_le()?))
    } else {
        None
    };
    let keepout_restrictions = if reader.remaining() >= 4 {
        Some(reader.read_i32_le()?)
    } else {
        None
    };
    reader.assert_exhausted()?;

    Ok(PcbFill {
        common,
        corner_1,
        corner_2,
        rotation,
        user_routed,
        union_index,
        layer_enum_index,
        keepout_restrictions,
    })
}

fn parse_raw(data: &[u8]) -> Result<PcbRawPrimitive> {
    let mut reader = BinaryReader::new(data);
    let common = parse_common_header(&mut reader)?;
    let raw_payload = reader.read_bytes(reader.remaining())?.to_vec();
    Ok(PcbRawPrimitive {
        common,
        raw_payload,
    })
}

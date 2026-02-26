use altium_format_types::constants::parsing::{BLOCK_SIZE_MASK, PAD_SUBRECORD_COUNT};
use altium_format_types::{
    BarcodeKind, BarcodeRenderMode, Coord, CoordPoint, PcbFlags, PcbObjectId, TextKind, V6Layer,
    V7Layer,
};

use crate::binary_io::BinaryReader;
use crate::pcblib::primitives::pad::parse_pad;
use crate::pcblib::PcbPad;
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
pub(crate) struct PcbText {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) location: CoordPoint,
    pub(crate) height: Coord,
    pub(crate) stroke_font_type: u16,
    pub(crate) text_kind: TextKind,
    pub(crate) rotation: f64,
    pub(crate) is_mirrored: bool,
    pub(crate) stroke_width: Coord,
    pub(crate) is_comment: bool,
    pub(crate) is_designator: bool,
    pub(crate) user_routed: bool,
    pub(crate) is_bold: bool,
    pub(crate) is_italic: bool,
    pub(crate) font_name: String,
    pub(crate) is_inverted: bool,
    pub(crate) margin_border_width: i32,
    pub(crate) wide_string_index: i32,
    pub(crate) union_index: i32,
    pub(crate) is_inverted_rect: bool,
    pub(crate) textbox_rect_width: i32,
    pub(crate) textbox_rect_height: i32,
    pub(crate) textbox_rect_justification: u8,
    pub(crate) text_offset_width: i32,
    pub(crate) unk_vec_x: i32,
    pub(crate) unk_vec_y: i32,
    pub(crate) barcode_margin_x: i32,
    pub(crate) barcode_margin_y: i32,
    pub(crate) barcode_min_width: i32,
    pub(crate) barcode_kind: BarcodeKind,
    pub(crate) barcode_render_mode: BarcodeRenderMode,
    pub(crate) barcode_inverted: bool,
    pub(crate) barcode_font_type: u8,
    pub(crate) barcode_font_name: String,
    pub(crate) barcode_min_pixel_size: i32,
    pub(crate) barcode_show_text: bool,
    pub(crate) has_v7_layer_data: bool,
    pub(crate) layer_enum_index: i32,
    pub(crate) sentinel_1: i32,
    pub(crate) sentinel_2: i32,
    pub(crate) trailing_flag_1: i32,
    pub(crate) trailing_flag_2: i32,
    pub(crate) trailing_is_justification_valid: Option<bool>,
    pub(crate) advance_snapping: Option<u8>,
    pub(crate) advance_mode: Option<u8>,
    pub(crate) advance_justification_x: Option<i32>,
    pub(crate) advance_justification_y: Option<i32>,
    pub(crate) use_text_alignment_by_snap: Option<i32>,
    pub(crate) snap_point_x: Option<i32>,
    pub(crate) snap_point_y: Option<i32>,
    pub(crate) text: String,
}

#[derive(Debug)]
pub(crate) enum PcbPrimitive {
    Arc(PcbArc),
    Track(PcbTrack),
    Fill(PcbFill),
    Pad(PcbPad),
    Text(PcbText),
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
    if kind == PrimitiveSectionKind::Pads6 {
        return parse_pad_records(data);
    }

    if matches!(
        kind,
        PrimitiveSectionKind::Texts | PrimitiveSectionKind::Texts6
    ) {
        return parse_text_records(kind, data);
    }

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
        PcbObjectId::Pad => Err(AltiumFormatError::InvalidParamValue {
            key: "Pads6/Data".to_owned(),
            detail: "Pad parsing requires 6-subrecord framing; reached single-payload path"
                .to_owned(),
        }),
        PcbObjectId::Via => Err(AltiumFormatError::InvalidParamValue {
            key: "Vias6/Data".to_owned(),
            detail: "Via parsing is not implemented without raw payload passthrough".to_owned(),
        }),
        PcbObjectId::Text => Err(AltiumFormatError::InvalidParamValue {
            key: "Texts/Data".to_owned(),
            detail: "Text parsing requires 2-subrecord framing; reached single-payload path"
                .to_owned(),
        }),
        PcbObjectId::Region => Err(AltiumFormatError::InvalidParamValue {
            key: "Regions6/Data".to_owned(),
            detail: "Region parsing is not implemented without raw payload passthrough".to_owned(),
        }),
        PcbObjectId::ComponentBody => Err(AltiumFormatError::InvalidParamValue {
            key: "ComponentBodies6/Data".to_owned(),
            detail: "ComponentBody parsing is not implemented without raw payload passthrough"
                .to_owned(),
        }),
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

fn parse_text_records(
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

        let raw_len_1 = reader.read_u32_le()?;
        let len_1 = (raw_len_1 & BLOCK_SIZE_MASK) as usize;
        let subrecord_1 = reader.read_bytes(len_1)?;

        let raw_len_2 = reader.read_u32_le()?;
        let len_2 = (raw_len_2 & BLOCK_SIZE_MASK) as usize;
        let subrecord_2 = reader.read_bytes(len_2)?;

        let primitive = parse_text_subrecords(subrecord_1, subrecord_2)?;
        out.push(ParsedPrimitiveRecord {
            object_id,
            primitive: PcbPrimitive::Text(primitive),
        });
    }

    reader.assert_exhausted()?;
    Ok(out)
}

fn parse_text_subrecords(subrecord_1: &[u8], subrecord_2: &[u8]) -> Result<PcbText> {
    let mut reader = BinaryReader::new(subrecord_1);
    let common = parse_common_header(&mut reader)?;
    let location = reader.read_coord_point()?;
    let height = reader.read_coord()?;
    let stroke_font_type = reader.read_u16_le()?;
    let mut text_kind = TextKind::StrokeFont;
    let rotation = reader.read_f64_le()?;
    let is_mirrored = reader.read_bool()?;
    let stroke_width = reader.read_coord()?;

    let mut is_comment = false;
    let mut is_designator = false;
    let mut user_routed = false;
    let mut is_bold = false;
    let mut is_italic = false;
    let mut font_name = String::new();
    let mut is_inverted = false;
    let mut margin_border_width = 0;
    let mut wide_string_index = 0;
    let mut union_index = 0;
    let mut is_inverted_rect = false;
    let mut textbox_rect_width = 0;
    let mut textbox_rect_height = 0;
    let mut textbox_rect_justification = 0;
    let mut text_offset_width = 0;
    let mut unk_vec_x = 0;
    let mut unk_vec_y = 0;
    let mut barcode_margin_x = 0;
    let mut barcode_margin_y = 0;
    let mut barcode_min_width = 0;
    let mut barcode_kind = BarcodeKind::default();
    let mut barcode_render_mode = BarcodeRenderMode::default();
    let mut barcode_inverted = false;
    let mut barcode_font_type = 0;
    let mut barcode_font_name = String::new();
    let mut barcode_min_pixel_size = 0;
    let mut barcode_show_text = false;
    let mut has_v7_layer_data = false;
    let mut layer_enum_index = 0;
    let mut sentinel_1 = 0;
    let mut sentinel_2 = 0;
    let mut trailing_flag_1 = 0;
    let mut trailing_flag_2 = 0;
    let mut trailing_is_justification_valid = None;
    let mut advance_snapping = None;
    let mut advance_mode = None;
    let mut advance_justification_x = None;
    let mut advance_justification_y = None;
    let mut use_text_alignment_by_snap = None;
    let mut snap_point_x = None;
    let mut snap_point_y = None;

    if reader.remaining() >= 83 {
        is_comment = reader.read_bool()?;
        is_designator = reader.read_bool()?;
        user_routed = reader.read_bool()?;
        let text_kind_ext = reader.read_u8()?;
        text_kind = TextKind::try_from(text_kind_ext)?;
        is_bold = reader.read_bool()?;
        is_italic = reader.read_bool()?;
        font_name = reader.read_wide_string_fixed(32)?;
        is_inverted = reader.read_bool()?;
        margin_border_width = reader.read_i32_le()?;
        wide_string_index = reader.read_i32_le()?;
        union_index = reader.read_i32_le()?;
        is_inverted_rect = reader.read_bool()?;
        textbox_rect_width = reader.read_i32_le()?;
        textbox_rect_height = reader.read_i32_le()?;
        textbox_rect_justification = reader.read_u8()?;
        text_offset_width = reader.read_i32_le()?;
    }

    if reader.remaining() >= 103 {
        unk_vec_x = reader.read_i32_le()?;
        unk_vec_y = reader.read_i32_le()?;
        barcode_margin_x = reader.read_i32_le()?;
        barcode_margin_y = reader.read_i32_le()?;
        barcode_min_width = reader.read_i32_le()?;
        barcode_kind = BarcodeKind::try_from(reader.read_u8()?)?;
        barcode_render_mode = BarcodeRenderMode::try_from(reader.read_u8()?)?;
        barcode_inverted = reader.read_bool()?;
        barcode_font_type = reader.read_u8()?;
        barcode_font_name = reader.read_wide_string_fixed(32)?;
        barcode_min_pixel_size = reader.read_i32_le()?;
        barcode_show_text = reader.read_bool()?;
    }

    if reader.remaining() >= 2 {
        advance_snapping = Some(reader.read_u8()?);
        advance_mode = Some(reader.read_u8()?);
    }
    if reader.remaining() >= 8 {
        advance_justification_x = Some(reader.read_i32_le()?);
        advance_justification_y = Some(reader.read_i32_le()?);
    }
    if reader.remaining() >= 4 {
        use_text_alignment_by_snap = Some(reader.read_i32_le()?);
    }
    if reader.remaining() >= 8 {
        snap_point_x = Some(reader.read_i32_le()?);
        snap_point_y = Some(reader.read_i32_le()?);
    }

    if reader.remaining() >= 22 {
        has_v7_layer_data = reader.read_bool()?;
        layer_enum_index = reader.read_i32_le()?;
        sentinel_1 = reader.read_i32_le()?;
        sentinel_2 = reader.read_i32_le()?;
        trailing_flag_1 = reader.read_i32_le()?;
        trailing_flag_2 = reader.read_i32_le()?;
    }

    if reader.remaining() == 1 {
        trailing_is_justification_valid = Some(reader.read_bool()?);
    }

    reader.assert_exhausted()?;
    let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(subrecord_2);
    let text = decoded.trim_end_matches('\0').to_owned();

    Ok(PcbText {
        common,
        location,
        height,
        stroke_font_type,
        text_kind,
        rotation,
        is_mirrored,
        stroke_width,
        is_comment,
        is_designator,
        user_routed,
        is_bold,
        is_italic,
        font_name,
        is_inverted,
        margin_border_width,
        wide_string_index,
        union_index,
        is_inverted_rect,
        textbox_rect_width,
        textbox_rect_height,
        textbox_rect_justification,
        text_offset_width,
        unk_vec_x,
        unk_vec_y,
        barcode_margin_x,
        barcode_margin_y,
        barcode_min_width,
        barcode_kind,
        barcode_render_mode,
        barcode_inverted,
        barcode_font_type,
        barcode_font_name,
        barcode_min_pixel_size,
        barcode_show_text,
        has_v7_layer_data,
        layer_enum_index,
        sentinel_1,
        sentinel_2,
        trailing_flag_1,
        trailing_flag_2,
        trailing_is_justification_valid,
        advance_snapping,
        advance_mode,
        advance_justification_x,
        advance_justification_y,
        use_text_alignment_by_snap,
        snap_point_x,
        snap_point_y,
        text,
    })
}

fn parse_pad_records(data: &[u8]) -> Result<Vec<ParsedPrimitiveRecord>> {
    let mut reader = BinaryReader::new(data);
    let mut out = Vec::new();

    while reader.remaining() > 0 {
        let object_id = PcbObjectId::try_from(reader.read_u8()?)?;
        if object_id != PcbObjectId::Pad {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "Pads6 object ID".to_owned(),
                detail: format!(
                    "section expects {:?}, found {:?}",
                    PcbObjectId::Pad,
                    object_id
                ),
            });
        }

        let mut subrecords: Vec<&[u8]> = Vec::with_capacity(PAD_SUBRECORD_COUNT);
        for _ in 0..PAD_SUBRECORD_COUNT {
            let raw_len = reader.read_u32_le()?;
            let len = (raw_len & BLOCK_SIZE_MASK) as usize;
            subrecords.push(reader.read_bytes(len)?);
        }

        let primitive = parse_pad(&subrecords)?;
        out.push(ParsedPrimitiveRecord {
            object_id,
            primitive: PcbPrimitive::Pad(primitive),
        });
    }

    reader.assert_exhausted()?;
    Ok(out)
}


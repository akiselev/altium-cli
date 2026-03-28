use altium_format_types::constants::parsing::TEXT_SUBRECORD_COUNT;
use altium_format_types::{BarcodeRenderMode, TextAutoposition, TextKind};

use crate::binary_io::BinaryReader;
use crate::pcblib::PcbText;
use crate::pcblib::primitives::common::parse_common_header;
use crate::{AltiumFormatError, Result};

/// Fixed size of the WideChar font name buffers (32 WideChars = 64 bytes).
const FONT_NAME_WCHAR_COUNT: usize = 32;

/// Minimum subrecord 0 size: the base format through barcode_font_name (offset 224).
const TEXT_BASE_SIZE: usize = 225;

/// Parses a Text primitive from its 2 PcbLib subrecords.
///
/// Subrecord 0: text properties (variable size depending on file version):
///   - 225 bytes: base format (through barcode_font_name)
///   - 230 bytes: adds tail flags (autoposition, etc.)
///   - 232 bytes: adds advance_snapping
///   - 240 bytes: adds advance justification X/Y
///   - 244 bytes: adds use_text_alignment_by_snap + padding
///   - 252 bytes: adds snap_point X/Y (current AD26 format)
///
/// Subrecord 1: text string content (raw Win1252 bytes)
///
/// Binary layout of subrecord 0 (252-byte AD26 format):
///
///   Offset  Size  Field
///   ------  ----  -----
///   0-12    13    common header
///   13-20    8    location (CoordPoint)
///   21-24    4    height (Coord)
///   25       1    text_kind (TextKind)
///   26       1    (reserved, always 0)
///   27-34    8    rotation (f64, degrees)
///   35       1    is_mirrored (bool)
///   36-39    4    stroke_width (Coord)
///   40-42    3    (reserved)
///   43       1    is_italic (bool)
///   44       1    is_bold (bool)
///   45       1    (reserved)
///   46-109  64    font_name (UTF-16LE, 32 WideChar fixed buffer)
///   110      1    inverted (bool)
///   111-114  4    inverted_tt_text_border (Coord) — IPCB_Text.InvertedTTTextBorder
///   115-118  4    wide_string_index (i32)
///   119-122  4    union_index (i32)
///   123      1    is_inverted_rect (bool) — IPCB_Text.IsInvertedRect
///   124-127  4    ttf_text_width (Coord)
///   128-131  4    ttf_text_height (Coord)
///   132-135  4    font_id (i32)
///   136      1    barcode_inverted (bool)
///   137-140  4    barcode_full_width (Coord)
///   141-144  4    barcode_full_height (Coord)
///   145-148  4    barcode_x_margin (Coord)
///   149-152  4    barcode_y_margin (Coord)
///   153-156  4    barcode_min_width (Coord)
///   157      1    (reserved)
///   158      1    barcode_show_text (bool)
///   159      1    barcode_render_mode (u8)
///   160      1    multiline (bool)
///   161-224 64    barcode_font_name (UTF-16LE, 32 WideChar fixed buffer)
///   --- version-dependent tail ---
///   225      1    ttf_inverted_justify (TTextAutoposition, u8)
///   226      1    ttf_offset_from_inverted_rect (u8)
///   227      1    (reserved, always 0)
///   228      1    multiline_auto_position (TTextAutoposition, u8)
///   229      1    is_advance_justification_valid (bool)
///   230      1    advance_snapping (u8)
///   231      1    (reserved, always 0)
///   232-235  4    advance_justification_x (i32; 0x80000000 = not set)
///   236-239  4    advance_justification_y (i32; 0x80000000 = not set)
///   240-243  4    use_text_alignment_by_snap (i32)
///   244-247  4    snap_point_x (Coord)
///   248-251  4    snap_point_y (Coord)
pub(crate) fn parse_text(subrecords: &[&[u8]]) -> Result<PcbText> {
    if subrecords.len() != TEXT_SUBRECORD_COUNT {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "Text subrecords".to_owned(),
            detail: format!(
                "expected {} subrecords, got {}",
                TEXT_SUBRECORD_COUNT,
                subrecords.len()
            ),
        });
    }

    let mut reader = BinaryReader::new(subrecords[0]);
    let sub0_len = subrecords[0].len();

    if sub0_len < TEXT_BASE_SIZE {
        return Err(AltiumFormatError::BinaryReadPastEnd {
            offset: 0,
            needed: TEXT_BASE_SIZE,
            available: sub0_len,
        });
    }

    // --- Core fields (offsets 0-224, always present) ---
    let common = parse_common_header(&mut reader)?;
    let location = reader.read_coord_point()?;
    let height = reader.read_coord()?;
    let text_kind = TextKind::try_from(reader.read_u8()?)?;
    reader.read_reserved_zero(1)?; // reserved byte 26
    let rotation = reader.read_f64_le()?;
    let is_mirrored = reader.read_bool()?;
    let stroke_width = reader.read_coord()?;
    reader.read_reserved_zero(3)?; // reserved bytes 40-42
    let is_italic = reader.read_bool()?;
    let is_bold = reader.read_bool()?;
    reader.read_reserved_zero(1)?; // reserved byte 45
    let font_name = reader.read_wide_string_fixed(FONT_NAME_WCHAR_COUNT)?;
    let inverted = reader.read_bool()?;
    let inverted_tt_text_border = reader.read_coord()?; // InvertedTTTextBorder (offset 111-114)
    let wide_string_index = reader.read_i32_le()?; // offset 115-118
    let union_index = reader.read_i32_le()?; // offset 119-122
    let is_inverted_rect = reader.read_bool()?; // offset 123
    let ttf_text_width = reader.read_coord()?;
    let ttf_text_height = reader.read_coord()?;
    let font_id = reader.read_i32_le()?;
    let barcode_inverted = reader.read_bool()?;
    let barcode_full_width = reader.read_coord()?;
    let barcode_full_height = reader.read_coord()?;
    let barcode_x_margin = reader.read_coord()?;
    let barcode_y_margin = reader.read_coord()?;
    let barcode_min_width = reader.read_coord()?;
    reader.read_reserved_zero(1)?; // reserved byte 157
    let barcode_show_text = reader.read_bool()?;
    let barcode_render_mode = BarcodeRenderMode::try_from(reader.read_u8()?)?;
    let multiline = reader.read_bool()?;
    let barcode_font_name = reader.read_wide_string_fixed(FONT_NAME_WCHAR_COUNT)?;

    // --- Version-dependent tail fields (offset 225+) ---
    //
    // Bytes 225-229: TTF inverted justify, TTF offset from rect, padding, multiline
    //                auto-position, is_advance_justification_valid.
    // Bytes 230-231: advance_snapping, reserved.
    // Bytes 232-239: advance_justification X/Y (i32 each; 0x80000000 = not set).
    // Bytes 240-243: use_text_alignment_by_snap (i32).
    // Bytes 244-251: snap_point X/Y (Coord each).
    let mut ttf_inverted_justify = None;
    let mut ttf_offset_from_inverted_rect = None;
    let mut tail_reserved_227 = None;
    let mut multiline_auto_position = None;
    let mut is_advance_justification_valid = None;
    let mut advance_snapping = None;
    let mut tail_reserved_231 = None;
    let mut advance_justification_x = None;
    let mut advance_justification_y = None;
    let mut use_text_alignment_by_snap = None;
    let mut snap_point_x = None;
    let mut snap_point_y = None;

    if reader.remaining() >= 5 {
        ttf_inverted_justify = Some(TextAutoposition::try_from(reader.read_u8()?)?);
        ttf_offset_from_inverted_rect = Some(reader.read_u8()?);
        let reserved = reader.read_u8()?;
        if reserved != 0 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "reserved byte 227".to_owned(),
                detail: format!("expected 0, got {reserved:#04X}"),
            });
        }
        tail_reserved_227 = Some(reserved);
        multiline_auto_position = Some(TextAutoposition::try_from(reader.read_u8()?)?);
        is_advance_justification_valid = Some(reader.read_bool()?);
    }
    if reader.remaining() >= 2 {
        advance_snapping = Some(reader.read_u8()?);
        let reserved = reader.read_u8()?;
        if reserved != 0 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "reserved byte 231".to_owned(),
                detail: format!("expected 0, got {reserved:#04X}"),
            });
        }
        tail_reserved_231 = Some(reserved);
    }
    if reader.remaining() >= 8 {
        advance_justification_x = Some(reader.read_i32_le()?);
        advance_justification_y = Some(reader.read_i32_le()?);
    }
    if reader.remaining() >= 4 {
        use_text_alignment_by_snap = Some(reader.read_i32_le()?);
    }
    if reader.remaining() >= 8 {
        snap_point_x = Some(reader.read_coord()?);
        snap_point_y = Some(reader.read_coord()?);
    }

    if reader.remaining() != 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "Text subrecord 0".to_owned(),
            detail: format!(
                "unsupported trailing bytes after known Text layout: {} bytes remain",
                reader.remaining()
            ),
        });
    }

    // Subrecord 1 contains the text string (Win1252 encoded).
    let text_bytes = subrecords[1];
    let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(text_bytes);
    let text = decoded.into_owned();

    Ok(PcbText {
        common,
        location,
        height,
        text_kind,
        rotation,
        is_mirrored,
        stroke_width,
        is_italic,
        is_bold,
        font_name,
        inverted,
        inverted_tt_text_border,
        wide_string_index,
        union_index,
        is_inverted_rect,
        ttf_text_width,
        ttf_text_height,
        font_id,
        barcode_inverted,
        barcode_full_width,
        barcode_full_height,
        barcode_x_margin,
        barcode_y_margin,
        barcode_min_width,
        barcode_show_text,
        barcode_render_mode,
        multiline,
        barcode_font_name,
        ttf_inverted_justify,
        ttf_offset_from_inverted_rect,
        tail_reserved_227,
        multiline_auto_position,
        is_advance_justification_valid,
        advance_snapping,
        tail_reserved_231,
        advance_justification_x,
        advance_justification_y,
        use_text_alignment_by_snap,
        snap_point_x,
        snap_point_y,
        text,
        unique_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_io::BinaryWriter;
    use altium_format_types::{Coord, CoordPoint, TextKind};

    fn write_common_header(w: &mut BinaryWriter) {
        w.write_u8(33); // layer = TopOverlay
        w.write_u16_le(0x000C); // flags
        w.write_u16_le(0xFFFF); // net_index = none
        w.write_u16_le(0xFFFF); // polygon_index = none
        w.write_u16_le(0xFFFF); // component_index = none
        w.write_u16_le(0xFFFF); // coordinate_index = none
        w.write_u16_le(0xFFFF); // dimension_index = none
    }

    /// Writes the 225-byte base format of Text subrecord 0.
    fn make_text_sub0_base(
        text_kind: TextKind,
        font_name: &str,
        barcode_font_name: &str,
    ) -> crate::Result<BinaryWriter> {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(100_000),
            Coord::from_internal(200_000),
        ));
        w.write_coord(Coord::from_internal(60_000)); // height
        w.write_u8(text_kind as u8); // text_kind
        w.write_u8(0); // reserved
        w.write_f64_le(90.0); // rotation
        w.write_bool(false); // is_mirrored
        w.write_coord(Coord::from_internal(10_000)); // stroke_width
        w.write_bytes(&[0, 0, 0]); // reserved
        w.write_bool(false); // is_italic
        w.write_bool(true); // is_bold
        w.write_u8(0); // reserved
        w.write_wide_string_fixed(font_name, FONT_NAME_WCHAR_COUNT)?; // font_name
        w.write_bool(false); // inverted
        w.write_i32_le(0); // inverted_tt_text_border
        w.write_i32_le(0); // wide_string_index
        w.write_i32_le(0); // union_index
        w.write_bool(false); // is_inverted_rect
        w.write_coord(Coord::from_internal(250_000)); // ttf_text_width
        w.write_coord(Coord::from_internal(80_000)); // ttf_text_height
        w.write_i32_le(3); // font_id
        w.write_bool(false); // barcode_inverted
        w.write_coord(Coord::from_internal(10_500_000)); // barcode_full_width
        w.write_coord(Coord::from_internal(2_100_000)); // barcode_full_height
        w.write_coord(Coord::from_internal(200_000)); // barcode_x_margin
        w.write_coord(Coord::from_internal(200_000)); // barcode_y_margin
        w.write_coord(Coord::from_internal(0)); // barcode_min_width
        w.write_u8(0); // reserved
        w.write_bool(true); // barcode_show_text
        w.write_u8(1); // barcode_render_mode
        w.write_bool(false); // multiline
        w.write_wide_string_fixed(barcode_font_name, FONT_NAME_WCHAR_COUNT)?; // barcode_font_name
        Ok(w)
    }

    fn make_text_subrecords(sub0: Vec<u8>, text: &str) -> [Vec<u8>; 2] {
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(text);
        [sub0, encoded.to_vec()]
    }

    #[test]
    fn parse_text_225_byte_base() {
        let w = make_text_sub0_base(TextKind::TrueTypeFont, "Arial", "Arial").unwrap();
        let sub0 = w.finish();
        assert_eq!(sub0.len(), TEXT_BASE_SIZE);

        let subs = make_text_subrecords(sub0, "R1");
        let sub_refs: Vec<&[u8]> = subs.iter().map(|s| s.as_slice()).collect();
        let text = parse_text(&sub_refs).unwrap();

        assert_eq!(text.location.x.to_internal(), 100_000);
        assert_eq!(text.location.y.to_internal(), 200_000);
        assert_eq!(text.height.to_internal(), 60_000);
        assert_eq!(text.text_kind, TextKind::TrueTypeFont);
        assert_eq!(text.rotation, 90.0);
        assert!(!text.is_mirrored);
        assert_eq!(text.stroke_width.to_internal(), 10_000);
        assert!(!text.is_italic);
        assert!(text.is_bold);
        assert_eq!(text.font_name, "Arial");
        assert!(!text.inverted);
        assert_eq!(text.wide_string_index, 0);
        assert_eq!(text.ttf_text_width.to_internal(), 250_000);
        assert_eq!(text.ttf_text_height.to_internal(), 80_000);
        assert_eq!(text.font_id, 3);
        assert!(!text.barcode_inverted);
        assert_eq!(text.barcode_full_width.to_internal(), 10_500_000);
        assert_eq!(text.barcode_full_height.to_internal(), 2_100_000);
        assert_eq!(text.barcode_x_margin.to_internal(), 200_000);
        assert_eq!(text.barcode_y_margin.to_internal(), 200_000);
        assert_eq!(text.barcode_min_width.to_internal(), 0);
        assert!(text.barcode_show_text);
        assert_eq!(text.barcode_render_mode, BarcodeRenderMode::ByFullWidth);
        assert!(!text.multiline);
        assert_eq!(text.barcode_font_name, "Arial");
        assert_eq!(text.text, "R1");
        assert!(text.unique_id.is_none());
    }

    #[test]
    fn parse_text_252_byte_ad26() {
        let mut w = make_text_sub0_base(TextKind::TrueTypeFont, "Consolas", "Arial").unwrap();
        // Tail bytes (225-251): 27 bytes of version-dependent fields.
        w.write_bytes(&[1, 6, 0, 3, 1]); // bytes 225-229
        w.write_bytes(&[1, 0]); // bytes 230-231 (advance_snapping + reserved)
        w.write_i32_le(0); // bytes 232-235 (advance_justification_x)
        w.write_i32_le(0); // bytes 236-239 (advance_justification_y)
        w.write_bytes(&[0, 0, 0, 0]); // bytes 240-243 (use_text_alignment + padding)
        w.write_i32_le(500_000); // bytes 244-247 (snap_point_x)
        w.write_i32_le(600_000); // bytes 248-251 (snap_point_y)
        let sub0 = w.finish();
        assert_eq!(sub0.len(), 252);

        let subs = make_text_subrecords(sub0, ".Designator");
        let sub_refs: Vec<&[u8]> = subs.iter().map(|s| s.as_slice()).collect();
        let text = parse_text(&sub_refs).unwrap();

        assert_eq!(text.text_kind, TextKind::TrueTypeFont);
        assert_eq!(text.font_name, "Consolas");
        assert_eq!(text.barcode_font_name, "Arial");
        assert_eq!(text.text, ".Designator");
    }

    #[test]
    fn parse_text_230_byte_format() {
        let mut w = make_text_sub0_base(TextKind::StrokeFont, "Arial", "Arial").unwrap();
        w.write_bytes(&[1, 6, 0, 3, 1]); // 5-byte tail
        let sub0 = w.finish();
        assert_eq!(sub0.len(), 230);

        let subs = make_text_subrecords(sub0, "C1");
        let sub_refs: Vec<&[u8]> = subs.iter().map(|s| s.as_slice()).collect();
        let text = parse_text(&sub_refs).unwrap();

        assert_eq!(text.text_kind, TextKind::StrokeFont);
        assert_eq!(text.text, "C1");
    }

    #[test]
    fn parse_text_barcode() {
        let w = make_text_sub0_base(TextKind::Barcode, "Arial", "Code39").unwrap();
        let sub0 = w.finish();

        let subs = make_text_subrecords(sub0, "SN12345");
        let sub_refs: Vec<&[u8]> = subs.iter().map(|s| s.as_slice()).collect();
        let text = parse_text(&sub_refs).unwrap();

        assert_eq!(text.text_kind, TextKind::Barcode);
        assert_eq!(text.barcode_font_name, "Code39");
        assert_eq!(text.text, "SN12345");
    }

    #[test]
    fn wrong_subrecord_count_returns_error() {
        let sub0 = vec![0u8; TEXT_BASE_SIZE];
        let sub_refs: Vec<&[u8]> = vec![sub0.as_slice()];
        let result = parse_text(&sub_refs);
        assert!(result.is_err());
    }

    #[test]
    fn truncated_sub0_returns_error() {
        let sub0 = vec![0u8; 100]; // way too short
        let sub1 = b"hello".to_vec();
        let sub_refs: Vec<&[u8]> = vec![sub0.as_slice(), sub1.as_slice()];
        let result = parse_text(&sub_refs);
        assert!(matches!(
            result,
            Err(AltiumFormatError::BinaryReadPastEnd { .. })
        ));
    }
}

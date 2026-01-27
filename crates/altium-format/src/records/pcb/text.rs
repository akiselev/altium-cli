//! PCB Text record.

use std::io::Read;

use altium_format_derive::AltiumRecord;

use super::primitive::{
    PcbPrimitiveCommon, PcbRectangularBase, PcbTextJustification, PcbTextKind, PcbTextStrokeFont,
};
use crate::error::{AltiumError, Result};
use crate::traits::{FromBinary, ToBinary};
use crate::types::{Coord, CoordPoint, CoordRect};

/// Font name stored as 32-byte fixed-length UTF-16 LE string.
///
/// Format: 32 bytes (16 UTF-16 LE code units)
/// - Typically null-terminated for strings < 16 characters
/// - May use all 16 code units (no null) for exactly 16-character strings
/// - Remaining bytes after null are zero-padded
#[derive(Debug, Clone, Default)]
struct FontName(String);

impl FromBinary for FontName {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = [0u8; 32];
        reader.read_exact(&mut buf)?;

        // Read UTF-16 LE code units until null terminator or end of buffer
        let mut code_units = Vec::new();
        for chunk in buf.chunks_exact(2) {
            let code = u16::from_le_bytes([chunk[0], chunk[1]]);
            if code == 0 {
                break;
            }
            code_units.push(code);
        }

        // Convert UTF-16 to Rust string
        let value = String::from_utf16(&code_units)
            .map_err(|e| AltiumError::Encoding(format!("Invalid UTF-16 font name: {}", e)))?;
        Ok(FontName(value))
    }
}

impl ToBinary for FontName {
    fn write_to<W: std::io::Write>(&self, writer: &mut W) -> Result<()> {
        use crate::io::writer::write_font_name;

        write_font_name(writer, &self.0)
    }

    fn binary_size(&self) -> usize {
        32
    }
}

#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(format = "binary")]
struct PcbTextBaseBinary {
    #[altium(flatten)]
    common: PcbPrimitiveCommon,
    #[altium(coord_point)]
    corner1: CoordPoint,
    #[altium(coord)]
    height: Coord,
    stroke_font: PcbTextStrokeFont,
    rotation: f64,
    mirrored: bool,
    #[altium(coord)]
    stroke_width: Coord,
}

#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(format = "binary")]
struct PcbTextExtendedBinary {
    _unknown1: u16,
    _unknown2: u8,
    text_kind: PcbTextKind,
    font_bold: bool,
    font_italic: bool,
    font_name: FontName,
    #[altium(coord)]
    barcode_lr_margin: Coord,
    #[altium(coord)]
    barcode_tb_margin: Coord,
    _unknown3: i32,
    _unknown4: i32,
    _unknown5: u8,
    _unknown6: u8,
    _unknown7: i32,
    _unknown8: u16,
    _unknown9: i32,
    _unknown10: i32,
    font_inverted: bool,
    #[altium(coord)]
    font_inverted_border: Coord,
    wide_strings_index: i32,
    _unknown11: i32,
    font_inverted_rect: bool,
    #[altium(coord)]
    font_inverted_rect_width: Coord,
    #[altium(coord)]
    font_inverted_rect_height: Coord,
    font_inverted_rect_justification: PcbTextJustification,
    #[altium(coord)]
    font_inverted_rect_text_offset: Coord,
}

/// PCB Text primitive.
#[derive(Debug, Clone, Default)]
pub struct PcbText {
    /// Base rectangular fields.
    pub base: PcbRectangularBase,
    /// Whether the text is mirrored.
    pub mirrored: bool,
    /// Text kind (Stroke, TrueType, BarCode).
    pub text_kind: PcbTextKind,
    /// Stroke font type.
    pub stroke_font: PcbTextStrokeFont,
    /// Stroke width.
    pub stroke_width: Coord,
    /// Font is bold.
    pub font_bold: bool,
    /// Font is italic.
    pub font_italic: bool,
    /// TrueType font name.
    pub font_name: String,
    /// Barcode left/right margin.
    pub barcode_lr_margin: Coord,
    /// Barcode top/bottom margin.
    pub barcode_tb_margin: Coord,
    /// Font is inverted.
    pub font_inverted: bool,
    /// Inverted border width.
    pub font_inverted_border: Coord,
    /// Inverted rectangle mode.
    pub font_inverted_rect: bool,
    /// Inverted rectangle width.
    pub font_inverted_rect_width: Coord,
    /// Inverted rectangle height.
    pub font_inverted_rect_height: Coord,
    /// Inverted rectangle justification.
    pub font_inverted_rect_justification: PcbTextJustification,
    /// Inverted rectangle text offset.
    pub font_inverted_rect_text_offset: Coord,
    /// The actual text content.
    pub text: String,
    /// Index into wide strings array (for Unicode text).
    pub wide_strings_index: i32,
}

impl FromBinary for PcbText {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let base = <PcbTextBaseBinary as FromBinary>::read_from(reader)?;

        let mut remaining = Vec::new();
        reader.read_to_end(&mut remaining)?;

        let mut extended = PcbTextExtendedBinary::default();
        if remaining.len() >= extended.binary_size() {
            let mut cursor = std::io::Cursor::new(&remaining);
            extended = <PcbTextExtendedBinary as FromBinary>::read_from(&mut cursor)?;
        }

        let corner2 = CoordPoint::new(base.corner1.x, base.corner1.y + base.height);

        Ok(PcbText {
            base: PcbRectangularBase {
                common: base.common,
                corner1: base.corner1,
                corner2,
                rotation: base.rotation,
            },
            mirrored: base.mirrored,
            text_kind: extended.text_kind,
            stroke_font: base.stroke_font,
            stroke_width: base.stroke_width,
            font_bold: extended.font_bold,
            font_italic: extended.font_italic,
            font_name: extended.font_name.0,
            barcode_lr_margin: extended.barcode_lr_margin,
            barcode_tb_margin: extended.barcode_tb_margin,
            font_inverted: extended.font_inverted,
            font_inverted_border: extended.font_inverted_border,
            font_inverted_rect: extended.font_inverted_rect,
            font_inverted_rect_width: extended.font_inverted_rect_width,
            font_inverted_rect_height: extended.font_inverted_rect_height,
            font_inverted_rect_justification: extended.font_inverted_rect_justification,
            font_inverted_rect_text_offset: extended.font_inverted_rect_text_offset,
            text: String::new(), // Set later from string block
            wide_strings_index: extended.wide_strings_index,
        })
    }
}

use std::io::Write;

impl ToBinary for PcbText {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        // Common primitive fields
        self.base.common.write_to(writer)?;

        // Corner1
        writer.write_i32::<LittleEndian>(self.base.corner1.x.to_raw())?;
        writer.write_i32::<LittleEndian>(self.base.corner1.y.to_raw())?;

        // Height (derived from corner2 - corner1)
        let height = self.base.corner2.y.to_raw() - self.base.corner1.y.to_raw();
        writer.write_i32::<LittleEndian>(height)?;

        // Stroke font
        writer.write_i16::<LittleEndian>(self.stroke_font.to_i16())?;

        // Rotation
        writer.write_f64::<LittleEndian>(self.base.rotation)?;

        // Mirrored
        writer.write_u8(if self.mirrored { 1 } else { 0 })?;

        // Stroke width
        writer.write_i32::<LittleEndian>(self.stroke_width.to_raw())?;

        // Extended binary data
        writer.write_u16::<LittleEndian>(0)?; // _unknown1
        writer.write_u8(0)?; // _unknown2

        // Text kind
        writer.write_u8(self.text_kind.to_byte())?;

        // Font style
        writer.write_u8(if self.font_bold { 1 } else { 0 })?;
        writer.write_u8(if self.font_italic { 1 } else { 0 })?;

        // Font name (32 bytes)
        FontName(self.font_name.clone()).write_to(writer)?;

        // Barcode margins
        writer.write_i32::<LittleEndian>(self.barcode_lr_margin.to_raw())?;
        writer.write_i32::<LittleEndian>(self.barcode_tb_margin.to_raw())?;

        // Unknown fields
        writer.write_i32::<LittleEndian>(0)?; // _unknown3
        writer.write_i32::<LittleEndian>(0)?; // _unknown4
        writer.write_u8(0)?; // _unknown5
        writer.write_u8(0)?; // _unknown6
        writer.write_i32::<LittleEndian>(0)?; // _unknown7
        writer.write_u16::<LittleEndian>(0)?; // _unknown8
        writer.write_i32::<LittleEndian>(0)?; // _unknown9
        writer.write_i32::<LittleEndian>(0)?; // _unknown10

        // Font inverted
        writer.write_u8(if self.font_inverted { 1 } else { 0 })?;
        writer.write_i32::<LittleEndian>(self.font_inverted_border.to_raw())?;

        // Wide strings index
        writer.write_i32::<LittleEndian>(self.wide_strings_index)?;

        // Unknown11
        writer.write_i32::<LittleEndian>(0)?;

        // Font inverted rect
        writer.write_u8(if self.font_inverted_rect { 1 } else { 0 })?;
        writer.write_i32::<LittleEndian>(self.font_inverted_rect_width.to_raw())?;
        writer.write_i32::<LittleEndian>(self.font_inverted_rect_height.to_raw())?;
        writer.write_u8(self.font_inverted_rect_justification.to_byte())?;
        writer.write_i32::<LittleEndian>(self.font_inverted_rect_text_offset.to_raw())?;

        Ok(())
    }

    fn binary_size(&self) -> usize {
        // This is an approximation - actual size depends on content
        200
    }
}

impl PcbText {
    /// Create a new text element.
    ///
    /// # Arguments
    /// * `x` - X position in millimeters
    /// * `y` - Y position in millimeters
    /// * `text` - Text content
    /// * `height` - Text height in millimeters
    /// * `stroke_width` - Stroke width in millimeters
    /// * `rotation` - Rotation angle in degrees
    /// * `mirrored` - Whether the text is mirrored
    /// * `layer` - The layer to place the text on
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        x: f64,
        y: f64,
        text: &str,
        height: f64,
        stroke_width: f64,
        rotation: f64,
        mirrored: bool,
        layer: crate::types::Layer,
    ) -> Self {
        use super::primitive::{PcbFlags, PcbPrimitiveCommon};

        let corner1 = CoordPoint::from_mms(x, y);
        let corner2 = CoordPoint::from_mms(x, y + height);

        PcbText {
            base: PcbRectangularBase {
                common: PcbPrimitiveCommon {
                    layer,
                    flags: PcbFlags::UNLOCKED | PcbFlags::UNKNOWN8,
                    unique_id: None,
                },
                corner1,
                corner2,
                rotation,
            },
            mirrored,
            text_kind: PcbTextKind::Stroke,
            stroke_font: PcbTextStrokeFont::Default,
            stroke_width: Coord::from_mms(stroke_width),
            font_bold: false,
            font_italic: false,
            font_name: String::new(),
            barcode_lr_margin: Coord::from_raw(0),
            barcode_tb_margin: Coord::from_raw(0),
            font_inverted: false,
            font_inverted_border: Coord::from_raw(0),
            font_inverted_rect: false,
            font_inverted_rect_width: Coord::from_raw(0),
            font_inverted_rect_height: Coord::from_raw(0),
            font_inverted_rect_justification: PcbTextJustification::BottomLeft,
            font_inverted_rect_text_offset: Coord::from_raw(0),
            text: text.to_string(),
            wide_strings_index: -1,
        }
    }

    /// Get the text height.
    pub fn height(&self) -> Coord {
        self.base.height()
    }

    /// Calculate the bounding rectangle.
    pub fn calculate_bounds(&self) -> CoordRect {
        self.base.calculate_bounds()
    }
}

//! PCB Pad record.
//!
//! **DEPRECATED**: Use `v2::pcb::PcbPadV2` with `v2::pcb::io` instead.

use std::io::Read;

use altium_format_derive::AltiumRecord;

use super::primitive::{PcbPadHoleShape, PcbPadShape, PcbPrimitiveCommon, PcbStackMode};
use crate::error::Result;
use crate::traits::FromBinary;
use crate::types::{Coord, CoordPoint, CoordRect, Layer, MaskExpansion};

/// PCB Pad primitive.
///
/// **DEPRECATED**: Use `v2::pcb::PcbPadV2` instead.
#[deprecated(note = "Use v2::pcb::PcbPadV2")]
#[derive(Debug, Clone)]
pub struct PcbPad {
    /// Common primitive fields.
    pub common: PcbPrimitiveCommon,
    /// Pad designator (e.g., "1", "A1").
    pub designator: String,
    /// Center location.
    pub location: CoordPoint,
    /// Rotation angle in degrees.
    pub rotation: f64,
    /// Whether the pad hole is plated.
    pub is_plated: bool,
    /// Jumper ID.
    pub jumper_id: i16,
    /// Stack mode.
    pub stack_mode: PcbStackMode,
    /// Hole size.
    pub hole_size: Coord,
    /// Hole shape.
    pub hole_shape: PcbPadHoleShape,
    /// Hole rotation.
    pub hole_rotation: f64,
    /// Hole slot length (for slot holes).
    pub hole_slot_length: Coord,
    /// Paste mask expansion mode and value.
    pub paste_mask_expansion: MaskExpansion,
    /// Solder mask expansion mode and value.
    pub solder_mask_expansion: MaskExpansion,
    /// Sizes for each layer (32 layers).
    pub size_layers: [CoordPoint; 32],
    /// Shapes for each layer (32 layers).
    pub shape_layers: [PcbPadShape; 32],
    /// Corner radius percentage for each layer (32 layers).
    pub corner_radius_percentage: [u8; 32],
    /// Offsets from hole center for each layer (32 layers).
    pub offsets_from_hole_center: [CoordPoint; 32],
}

#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(format = "binary")]
struct PcbPadMainBinary {
    #[altium(flatten)]
    common: PcbPrimitiveCommon,
    #[altium(coord_point)]
    location: CoordPoint,
    #[altium(coord_point)]
    size_top: CoordPoint,
    #[altium(coord_point)]
    size_mid: CoordPoint,
    #[altium(coord_point)]
    size_bottom: CoordPoint,
    #[altium(coord)]
    hole_size: Coord,
    shape_top: PcbPadShape,
    shape_mid: PcbPadShape,
    shape_bottom: PcbPadShape,
    rotation: f64,
    is_plated: bool,
    _unknown91: u8,
    stack_mode: PcbStackMode,
    _unknown1: u8,
    _unknown2: i32,
    _unknown3: i32,
    _unknown4: i16,
    _unknown5: u32,
    _unknown6: u32,
    _unknown7: u32,
    #[altium(coord)]
    paste_mask_expansion: Coord,
    #[altium(coord)]
    solder_mask_expansion: Coord,
    #[altium(array = 7)]
    _unknown8: [u8; 7],
    // Binary struct keeps raw u8 fields for expansion_manual flags; the public API uses MaskExpansion
    // enum (Auto | Manual(Coord)) for type safety. Conversion happens in apply_main_block().
    // See Decision Log in docs/plans/plan-altium.md: "Binary structs keep raw fields for format fidelity"
    paste_mask_expansion_manual: u8,
    solder_mask_expansion_manual: u8,
    _unknown9: u8,
    _unknown10: u8,
    _unknown11: u8,
    _unknown12: u32,
    jumper_id: i16,
    _unknown13: i16,
}

#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(format = "binary")]
struct PcbPadExtendedBinary {
    #[altium(array = 29)]
    mid_x_sizes: [i32; 29],
    #[altium(array = 29)]
    mid_y_sizes: [i32; 29],
    #[altium(array = 29)]
    mid_shapes: [PcbPadShape; 29],
    _unknown0: u8,
    hole_shape: PcbPadHoleShape,
    #[altium(coord)]
    hole_slot_length: Coord,
    hole_rotation: f64,
    #[altium(array = 32)]
    offset_x: [i32; 32],
    #[altium(array = 32)]
    offset_y: [i32; 32],
    has_rounded_rect: bool,
    #[altium(array = 32)]
    rounded_rect_shapes: [PcbPadShape; 32],
    #[altium(array = 32)]
    corner_radius_percentage: [u8; 32],
}

impl Default for PcbPad {
    fn default() -> Self {
        let default_size = CoordPoint::from_mils(60.0, 60.0);
        Self {
            common: PcbPrimitiveCommon {
                layer: Layer::multi_layer(),
                ..Default::default()
            },
            designator: String::new(),
            location: CoordPoint::default(),
            rotation: 0.0,
            is_plated: true,
            jumper_id: 0,
            stack_mode: PcbStackMode::Simple,
            hole_size: Coord::from_mils(30.0),
            hole_shape: PcbPadHoleShape::Round,
            hole_rotation: 0.0,
            hole_slot_length: Coord::default(),
            paste_mask_expansion: MaskExpansion::Auto,
            solder_mask_expansion: MaskExpansion::Auto,
            size_layers: [default_size; 32],
            shape_layers: [PcbPadShape::Round; 32],
            corner_radius_percentage: [50; 32],
            offsets_from_hole_center: [CoordPoint::default(); 32],
        }
    }
}

impl PcbPad {
    /// Read a PcbPad from binary stream.
    ///
    /// Pad format is complex: multiple blocks for designator, unknown data, and main data.
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Self::read_from_impl(reader)
    }

    fn read_from_impl<R: Read>(reader: &mut R) -> Result<Self> {
        use crate::io::reader::{read_block, read_string_block};

        // Block 1: Designator string
        let designator = read_string_block(reader)?;

        // Block 2: Unknown purpose, observed as empty in all test files.
        // Kept for format compatibility; may contain extension data in future Altium versions.
        let block2 = read_block(reader)?;
        if !block2.is_empty() {
            log::trace!("Pad block 2 (unknown): {} bytes", block2.len());
        }

        // Block 3: Constant string "|&|0" - purpose unknown, likely a format marker or delimiter
        let block3_str = read_string_block(reader)?;
        if block3_str != "|&|0" {
            log::trace!(
                "Pad block 3: unexpected value {:?}, expected \"|&|0\"",
                block3_str
            );
        }

        // Block 4: Unknown purpose, observed as empty in all test files.
        // Kept for format compatibility; may contain extension data in future Altium versions.
        let block4 = read_block(reader)?;
        if !block4.is_empty() {
            log::trace!("Pad block 4 (unknown): {} bytes", block4.len());
        }

        let mut pad = PcbPad {
            designator,
            ..Default::default()
        };

        // Block 5: Main pad data
        let main_block = read_block(reader)?;
        if !main_block.is_empty() {
            let mut cursor = std::io::Cursor::new(&main_block);
            let raw = <PcbPadMainBinary as FromBinary>::read_from(&mut cursor)?;
            pad.apply_main_block(raw);
        }

        // Block 6: Extended size/shape data
        let ext_block = read_block(reader)?;
        if !ext_block.is_empty() {
            let mut cursor = std::io::Cursor::new(&ext_block);
            let raw = <PcbPadExtendedBinary as FromBinary>::read_from(&mut cursor)?;
            pad.apply_extended_block(raw);
        }

        Ok(pad)
    }

    fn apply_main_block(&mut self, raw: PcbPadMainBinary) {
        self.common = raw.common;
        self.location = raw.location;
        self.rotation = raw.rotation;
        self.is_plated = raw.is_plated;
        self.stack_mode = raw.stack_mode;
        self.hole_size = raw.hole_size;
        // Altium format uses value 2 for manual mode (verified in test files)
        self.paste_mask_expansion = if raw.paste_mask_expansion_manual == 2 {
            MaskExpansion::Manual(raw.paste_mask_expansion)
        } else {
            MaskExpansion::Auto
        };
        self.solder_mask_expansion = if raw.solder_mask_expansion_manual == 2 {
            MaskExpansion::Manual(raw.solder_mask_expansion)
        } else {
            MaskExpansion::Auto
        };
        self.jumper_id = raw.jumper_id;

        self.size_layers[0] = raw.size_top;
        for i in 1..31 {
            self.size_layers[i] = raw.size_mid;
        }
        self.size_layers[31] = raw.size_bottom;

        self.shape_layers[0] = raw.shape_top;
        for i in 1..31 {
            self.shape_layers[i] = raw.shape_mid;
        }
        self.shape_layers[31] = raw.shape_bottom;
    }

    fn apply_extended_block(&mut self, raw: PcbPadExtendedBinary) {
        for i in 0..29 {
            self.size_layers[i + 1] = CoordPoint::from_raw(raw.mid_x_sizes[i], raw.mid_y_sizes[i]);
        }
        for i in 0..29 {
            self.shape_layers[i + 1] = raw.mid_shapes[i];
        }

        self.hole_shape = raw.hole_shape;
        self.hole_slot_length = raw.hole_slot_length;
        self.hole_rotation = raw.hole_rotation;

        for i in 0..32 {
            self.offsets_from_hole_center[i] =
                CoordPoint::from_raw(raw.offset_x[i], raw.offset_y[i]);
        }

        if raw.has_rounded_rect {
            self.shape_layers = raw.rounded_rect_shapes;
        }

        self.corner_radius_percentage = raw.corner_radius_percentage;
    }

    /// Get the top layer size.
    pub fn size_top(&self) -> CoordPoint {
        self.size_layers[0]
    }

    /// Get the bottom layer size.
    pub fn size_bottom(&self) -> CoordPoint {
        self.size_layers[31]
    }

    /// Get the top layer shape.
    pub fn shape_top(&self) -> PcbPadShape {
        self.shape_layers[0]
    }

    /// Get the bottom layer shape.
    pub fn shape_bottom(&self) -> PcbPadShape {
        self.shape_layers[31]
    }

    /// Check if this is a through-hole pad (has hole).
    pub fn has_hole(&self) -> bool {
        self.common.layer == Layer::multi_layer()
    }

    /// Calculate the bounding rectangle.
    pub fn calculate_bounds(&self) -> CoordRect {
        let size = self.size_top();
        let half_w = size.x.to_raw() / 2;
        let half_h = size.y.to_raw() / 2;

        CoordRect::from_raw(
            self.location.x.to_raw() - half_w,
            self.location.y.to_raw() - half_h,
            size.x.to_raw(),
            size.y.to_raw(),
        )
    }
}

impl FromBinary for PcbPad {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        PcbPad::read_from_impl(reader)
    }
}

use crate::io::writer::{write_block, write_string_block};
use crate::traits::ToBinary;
use std::io::Write;

impl ToBinary for PcbPad {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Block 1: Designator string
        write_string_block(writer, &self.designator)?;

        // Block 2: Unknown data (empty for new pads)
        write_block(writer, &[], 0)?;

        // Block 3: Constant string "|&|0"
        write_string_block(writer, "|&|0")?;

        // Block 4: Unknown data (empty for new pads)
        write_block(writer, &[], 0)?;

        // Block 5: Main pad data
        let main_data = self.build_main_block()?;
        write_block(writer, &main_data, 0)?;

        // Block 6: Extended size/shape data
        let ext_data = self.build_extended_block()?;
        write_block(writer, &ext_data, 0)?;

        Ok(())
    }

    fn binary_size(&self) -> usize {
        // Approximate size - actual size is computed during write
        // Block headers (6 * 4) + designator + main block (~145) + extended block (~500+)
        24 + self.designator.len() + 5 + 145 + 600
    }
}

impl PcbPad {
    fn build_main_block(&self) -> Result<Vec<u8>> {
        use byteorder::{LittleEndian, WriteBytesExt};

        let mut data = Vec::new();

        // Common primitive fields (13 bytes)
        self.common.write_to(&mut data)?;

        // Location (8 bytes)
        data.write_i32::<LittleEndian>(self.location.x.to_raw())?;
        data.write_i32::<LittleEndian>(self.location.y.to_raw())?;

        // Size top (8 bytes)
        data.write_i32::<LittleEndian>(self.size_layers[0].x.to_raw())?;
        data.write_i32::<LittleEndian>(self.size_layers[0].y.to_raw())?;

        // Size mid (using layer 15 as mid) (8 bytes)
        data.write_i32::<LittleEndian>(self.size_layers[15].x.to_raw())?;
        data.write_i32::<LittleEndian>(self.size_layers[15].y.to_raw())?;

        // Size bottom (8 bytes)
        data.write_i32::<LittleEndian>(self.size_layers[31].x.to_raw())?;
        data.write_i32::<LittleEndian>(self.size_layers[31].y.to_raw())?;

        // Hole size (4 bytes)
        data.write_i32::<LittleEndian>(self.hole_size.to_raw())?;

        // Shapes (3 bytes)
        data.write_u8(self.shape_layers[0].to_byte())?;
        data.write_u8(self.shape_layers[15].to_byte())?;
        data.write_u8(self.shape_layers[31].to_byte())?;

        // Rotation (8 bytes)
        data.write_f64::<LittleEndian>(self.rotation)?;

        // Is plated (1 byte)
        data.write_u8(if self.is_plated { 1 } else { 0 })?;

        // Unknown91 (1 byte)
        data.write_u8(0)?;

        // Stack mode (1 byte)
        data.write_u8(self.stack_mode.to_byte())?;

        // Unknown bytes
        data.write_u8(0)?; // _unknown1
        data.write_i32::<LittleEndian>(0)?; // _unknown2
        data.write_i32::<LittleEndian>(0)?; // _unknown3
        data.write_i16::<LittleEndian>(0)?; // _unknown4
        data.write_u32::<LittleEndian>(0)?; // _unknown5
        data.write_u32::<LittleEndian>(0)?; // _unknown6
        data.write_u32::<LittleEndian>(0)?; // _unknown7

        // Paste mask expansion (4 bytes)
        data.write_i32::<LittleEndian>(self.paste_mask_expansion.value().to_raw())?;

        // Solder mask expansion (4 bytes)
        data.write_i32::<LittleEndian>(self.solder_mask_expansion.value().to_raw())?;

        // Unknown array (7 bytes)
        data.write_all(&[0u8; 7])?;

        // Paste mask expansion manual (1 byte)
        data.write_u8(if self.paste_mask_expansion.is_manual() {
            2
        } else {
            0
        })?;

        // Solder mask expansion manual (1 byte)
        data.write_u8(if self.solder_mask_expansion.is_manual() {
            2
        } else {
            0
        })?;

        // More unknown bytes
        data.write_u8(0)?; // _unknown9
        data.write_u8(0)?; // _unknown10
        data.write_u8(0)?; // _unknown11
        data.write_u32::<LittleEndian>(0)?; // _unknown12

        // Jumper ID (2 bytes)
        data.write_i16::<LittleEndian>(self.jumper_id)?;

        // Unknown13 (2 bytes)
        data.write_i16::<LittleEndian>(0)?;

        Ok(data)
    }

    fn build_extended_block(&self) -> Result<Vec<u8>> {
        use byteorder::{LittleEndian, WriteBytesExt};

        let mut data = Vec::new();

        // Mid X sizes (29 * 4 = 116 bytes)
        for i in 0..29 {
            data.write_i32::<LittleEndian>(self.size_layers[i + 1].x.to_raw())?;
        }

        // Mid Y sizes (29 * 4 = 116 bytes)
        for i in 0..29 {
            data.write_i32::<LittleEndian>(self.size_layers[i + 1].y.to_raw())?;
        }

        // Mid shapes (29 bytes)
        for i in 0..29 {
            data.write_u8(self.shape_layers[i + 1].to_byte())?;
        }

        // Unknown0 (1 byte)
        data.write_u8(0)?;

        // Hole shape (1 byte)
        data.write_u8(self.hole_shape.to_byte())?;

        // Hole slot length (4 bytes)
        data.write_i32::<LittleEndian>(self.hole_slot_length.to_raw())?;

        // Hole rotation (8 bytes)
        data.write_f64::<LittleEndian>(self.hole_rotation)?;

        // Offset X (32 * 4 = 128 bytes)
        for i in 0..32 {
            data.write_i32::<LittleEndian>(self.offsets_from_hole_center[i].x.to_raw())?;
        }

        // Offset Y (32 * 4 = 128 bytes)
        for i in 0..32 {
            data.write_i32::<LittleEndian>(self.offsets_from_hole_center[i].y.to_raw())?;
        }

        // Has rounded rect flag (1 byte)
        let has_rounded = self.shape_layers.contains(&PcbPadShape::RoundedRectangle);
        data.write_u8(if has_rounded { 1 } else { 0 })?;

        // Rounded rect shapes (32 bytes)
        for i in 0..32 {
            data.write_u8(self.shape_layers[i].to_byte())?;
        }

        // Corner radius percentage (32 bytes)
        for i in 0..32 {
            data.write_u8(self.corner_radius_percentage[i])?;
        }

        Ok(data)
    }
}

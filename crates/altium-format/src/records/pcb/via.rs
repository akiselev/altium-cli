//! PCB Via record.

use std::io::Read;

use altium_format_derive::AltiumRecord;

use super::primitive::{PcbPrimitiveCommon, PcbStackMode};
use crate::error::Result;
use crate::traits::FromBinary;
use crate::types::{Coord, CoordPoint, CoordRect, Layer, MaskExpansion};

/// PCB Via primitive.
#[derive(Debug, Clone)]
pub struct PcbVia {
    /// Common primitive fields.
    pub common: PcbPrimitiveCommon,
    /// Center location.
    pub location: CoordPoint,
    /// Hole size.
    pub hole_size: Coord,
    /// From layer.
    pub from_layer: Layer,
    /// To layer.
    pub to_layer: Layer,
    /// Thermal relief air gap width.
    pub thermal_relief_air_gap_width: Coord,
    /// Thermal relief conductors count.
    pub thermal_relief_conductors: u8,
    /// Thermal relief conductor width.
    pub thermal_relief_conductors_width: Coord,
    /// Solder mask expansion mode and value.
    pub solder_mask_expansion: MaskExpansion,
    /// Diameter stack mode.
    pub diameter_stack_mode: PcbStackMode,
    /// Diameters for each layer (32 layers).
    pub diameters: [Coord; 32],
    /// Unknown binary data at the end of the record (variable length: 112-142 bytes).
    ///
    /// Based on analysis of RFSoC_AMC.PcbDoc:
    /// - Most vias (7,872 of 7,875) have 121 bytes
    /// - 3 simple vias have 112 bytes (all zeros)
    /// - Contains thermal relief configuration and design rule GUIDs
    /// - Bytes 58-89: Two 16-byte GUIDs for design rules/via styles
    /// - Safe default: 112 bytes of zeros (simple via without GUIDs)
    ///
    /// Preserved for round-trip integrity.
    pub unknown: Vec<u8>,
}

#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(format = "binary")]
struct PcbViaBinary {
    #[altium(flatten)]
    common: PcbPrimitiveCommon,
    #[altium(coord_point)]
    location: CoordPoint,
    #[altium(coord)]
    diameter: Coord,
    #[altium(coord)]
    hole_size: Coord,
    from_layer: Layer,
    to_layer: Layer,
    _unknown0: u8,
    #[altium(coord)]
    thermal_relief_air_gap_width: Coord,
    thermal_relief_conductors: u8,
    _unknown1: u8,
    #[altium(coord)]
    thermal_relief_conductors_width: Coord,
    _unknown2: i32,
    _unknown3: i32,
    _unknown4: i32,
    #[altium(coord)]
    solder_mask_expansion: Coord,
    #[altium(array = 8)]
    _unknown5: [u8; 8],
    solder_mask_expansion_manual: u8,
    _unknown6: u8,
    _unknown7: i16,
    _unknown8: i32,
    diameter_stack_mode: PcbStackMode,
    #[altium(array = 32)]
    diameters: [Coord; 32],
    _unknown9: i16,
    _unknown10: i32,
    /// Unknown binary data at the end of the record (variable length: 112-142 bytes).
    /// Contains GUIDs, additional thermal relief data, and other unknown fields.
    /// Length varies based on via configuration (thermal relief, tenting, etc.).
    #[altium(unknown_binary)]
    unknown: Vec<u8>,
}

impl Default for PcbVia {
    fn default() -> Self {
        let default_diameter = Coord::from_mils(50.0);
        Self {
            common: PcbPrimitiveCommon::default(),
            location: CoordPoint::default(),
            hole_size: Coord::from_mils(28.0),
            from_layer: Layer(1), // TopLayer
            to_layer: Layer(32),  // BottomLayer
            thermal_relief_air_gap_width: Coord::from_mils(10.0),
            thermal_relief_conductors: 4,
            thermal_relief_conductors_width: Coord::from_mils(10.0),
            solder_mask_expansion: MaskExpansion::Auto,
            diameter_stack_mode: PcbStackMode::Simple,
            diameters: [default_diameter; 32],
            unknown: vec![0u8; 112],
        }
    }
}

impl FromBinary for PcbVia {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let raw = <PcbViaBinary as FromBinary>::read_from(reader)?;
        let mut diameters = raw.diameters;

        // In Simple stack mode, copy the main diameter to all layers
        if raw.diameter_stack_mode == PcbStackMode::Simple {
            for d in diameters.iter_mut() {
                *d = raw.diameter;
            }
        }

        Ok(PcbVia {
            common: raw.common,
            location: raw.location,
            hole_size: raw.hole_size,
            from_layer: raw.from_layer,
            to_layer: raw.to_layer,
            thermal_relief_air_gap_width: raw.thermal_relief_air_gap_width,
            thermal_relief_conductors: raw.thermal_relief_conductors,
            thermal_relief_conductors_width: raw.thermal_relief_conductors_width,
            solder_mask_expansion: if raw.solder_mask_expansion_manual == 2 {
                MaskExpansion::Manual(raw.solder_mask_expansion)
            } else {
                MaskExpansion::Auto
            },
            diameter_stack_mode: raw.diameter_stack_mode,
            diameters,
            unknown: raw.unknown,
        })
    }
}

use crate::traits::ToBinary;
use std::io::Write;

impl ToBinary for PcbVia {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        // Common primitive fields
        self.common.write_to(writer)?;

        // Location
        writer.write_i32::<LittleEndian>(self.location.x.to_raw())?;
        writer.write_i32::<LittleEndian>(self.location.y.to_raw())?;

        // Diameter (use diameters[31] as main diameter)
        writer.write_i32::<LittleEndian>(self.diameters[31].to_raw())?;

        // Hole size
        writer.write_i32::<LittleEndian>(self.hole_size.to_raw())?;

        // From/To layers
        self.from_layer.write_to(writer)?;
        self.to_layer.write_to(writer)?;

        // Unknown0
        writer.write_u8(0)?;

        // Thermal relief fields
        writer.write_i32::<LittleEndian>(self.thermal_relief_air_gap_width.to_raw())?;
        writer.write_u8(self.thermal_relief_conductors)?;
        writer.write_u8(0)?; // _unknown1
        writer.write_i32::<LittleEndian>(self.thermal_relief_conductors_width.to_raw())?;

        // Unknown fields
        writer.write_i32::<LittleEndian>(0)?; // _unknown2
        writer.write_i32::<LittleEndian>(0)?; // _unknown3
        writer.write_i32::<LittleEndian>(0)?; // _unknown4

        // Solder mask expansion
        writer.write_i32::<LittleEndian>(self.solder_mask_expansion.value().to_raw())?;

        // Unknown array
        writer.write_all(&[0u8; 8])?; // _unknown5

        // Solder mask expansion manual
        writer.write_u8(if self.solder_mask_expansion.is_manual() {
            2
        } else {
            0
        })?;

        // More unknown fields
        writer.write_u8(0)?; // _unknown6
        writer.write_i16::<LittleEndian>(0)?; // _unknown7
        writer.write_i32::<LittleEndian>(0)?; // _unknown8

        // Diameter stack mode
        writer.write_u8(self.diameter_stack_mode.to_byte())?;

        // Diameters array (32 * i32)
        for diameter in &self.diameters {
            writer.write_i32::<LittleEndian>(diameter.to_raw())?;
        }

        // Unknown trailer
        writer.write_i16::<LittleEndian>(0)?; // _unknown9
        writer.write_i32::<LittleEndian>(0)?; // _unknown10

        // Write unknown binary data (variable length) for round-trip integrity
        writer.write_all(&self.unknown)?;

        Ok(())
    }

    fn binary_size(&self) -> usize {
        // 13 (common) + 8 (location) + 4 (diameter) + 4 (hole) + 2 (layers) + 1 +
        // 4 + 1 + 1 + 4 + 12 + 4 + 8 + 1 + 1 + 2 + 4 + 1 + 128 + 2 + 4 + unknown
        209 + self.unknown.len()
    }
}

impl PcbVia {
    /// Create a new through-hole via with simple stack mode.
    pub fn new(location: CoordPoint, diameter: Coord, hole_size: Coord) -> Self {
        use super::primitive::PcbFlags;
        Self {
            common: PcbPrimitiveCommon {
                layer: Layer::MULTI_LAYER,
                flags: PcbFlags::default(),
                unique_id: None,
            },
            location,
            hole_size,
            from_layer: Layer::TOP_LAYER,
            to_layer: Layer::BOTTOM_LAYER,
            thermal_relief_air_gap_width: Coord::from_mils(10.0),
            thermal_relief_conductors: 4,
            thermal_relief_conductors_width: Coord::from_mils(10.0),
            solder_mask_expansion: MaskExpansion::Auto,
            diameter_stack_mode: PcbStackMode::Simple,
            diameters: [diameter; 32],
            unknown: vec![0u8; 112],
        }
    }

    /// Create a via from measurements in mils.
    pub fn from_mils(x: f64, y: f64, diameter: f64, hole_size: f64) -> Self {
        Self::new(
            CoordPoint::from_mils(x, y),
            Coord::from_mils(diameter),
            Coord::from_mils(hole_size),
        )
    }

    /// Create a via from measurements in mm.
    pub fn from_mms(x: f64, y: f64, diameter: f64, hole_size: f64) -> Self {
        Self::new(
            CoordPoint::from_mms(x, y),
            Coord::from_mms(diameter),
            Coord::from_mms(hole_size),
        )
    }

    /// Create a blind or buried via.
    pub fn new_blind_buried(
        location: CoordPoint,
        diameter: Coord,
        hole_size: Coord,
        from_layer: Layer,
        to_layer: Layer,
    ) -> Self {
        let mut via = Self::new(location, diameter, hole_size);
        via.from_layer = from_layer;
        via.to_layer = to_layer;
        via
    }

    /// Set thermal relief parameters.
    pub fn with_thermal_relief(
        mut self,
        air_gap: Coord,
        conductor_width: Coord,
        conductor_count: u8,
    ) -> Self {
        self.thermal_relief_air_gap_width = air_gap;
        self.thermal_relief_conductors_width = conductor_width;
        self.thermal_relief_conductors = conductor_count;
        self
    }

    /// Set solder mask expansion with manual override.
    pub fn with_solder_mask_expansion(mut self, expansion: Coord, manual: bool) -> Self {
        self.solder_mask_expansion = if manual {
            MaskExpansion::Manual(expansion)
        } else {
            MaskExpansion::Auto
        };
        self
    }

    /// Set the diameter stack mode and diameters.
    pub fn with_stack_mode(mut self, mode: PcbStackMode, diameters: [Coord; 32]) -> Self {
        self.diameter_stack_mode = mode;
        self.diameters = diameters;
        self
    }

    /// Get the main diameter.
    pub fn diameter(&self) -> Coord {
        self.diameters[31]
    }

    /// Set the diameter for all layers (simple stack mode).
    pub fn set_diameter(&mut self, diameter: Coord) {
        self.diameter_stack_mode = PcbStackMode::Simple;
        for d in self.diameters.iter_mut() {
            *d = diameter;
        }
    }

    /// Calculate the bounding rectangle.
    pub fn calculate_bounds(&self) -> CoordRect {
        let d = self.diameter().to_raw();
        CoordRect::from_raw(
            self.location.x.to_raw() - d / 2,
            self.location.y.to_raw() - d / 2,
            d,
            d,
        )
    }
}

//! PCB layer definitions for Altium.

use std::fmt;

use byteorder::{ReadBytesExt, WriteBytesExt};

use crate::error::Result;
use crate::traits::{FromBinary, ToBinary};

/// PCB layer identifier.
///
/// Altium uses byte values (0-255) to identify layers.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Layer(pub u8);

impl Layer {
    /// Creates a new Layer from a u8 value.
    pub const fn new(value: u8) -> Self {
        Layer(value)
    }

    /// Returns the MultiLayer constant (for through-hole pads).
    pub const fn multi_layer() -> Self {
        Layer::MULTI_LAYER
    }

    // Standard layer constants
    pub const NO_LAYER: Layer = Layer(0);
    pub const TOP_LAYER: Layer = Layer(1);
    pub const MID_LAYER_1: Layer = Layer(2);
    pub const MID_LAYER_2: Layer = Layer(3);
    pub const MID_LAYER_3: Layer = Layer(4);
    pub const MID_LAYER_4: Layer = Layer(5);
    pub const MID_LAYER_5: Layer = Layer(6);
    pub const MID_LAYER_6: Layer = Layer(7);
    pub const MID_LAYER_7: Layer = Layer(8);
    pub const MID_LAYER_8: Layer = Layer(9);
    pub const MID_LAYER_9: Layer = Layer(10);
    pub const MID_LAYER_10: Layer = Layer(11);
    pub const MID_LAYER_11: Layer = Layer(12);
    pub const MID_LAYER_12: Layer = Layer(13);
    pub const MID_LAYER_13: Layer = Layer(14);
    pub const MID_LAYER_14: Layer = Layer(15);
    pub const MID_LAYER_15: Layer = Layer(16);
    pub const MID_LAYER_16: Layer = Layer(17);
    pub const MID_LAYER_17: Layer = Layer(18);
    pub const MID_LAYER_18: Layer = Layer(19);
    pub const MID_LAYER_19: Layer = Layer(20);
    pub const MID_LAYER_20: Layer = Layer(21);
    pub const MID_LAYER_21: Layer = Layer(22);
    pub const MID_LAYER_22: Layer = Layer(23);
    pub const MID_LAYER_23: Layer = Layer(24);
    pub const MID_LAYER_24: Layer = Layer(25);
    pub const MID_LAYER_25: Layer = Layer(26);
    pub const MID_LAYER_26: Layer = Layer(27);
    pub const MID_LAYER_27: Layer = Layer(28);
    pub const MID_LAYER_28: Layer = Layer(29);
    pub const MID_LAYER_29: Layer = Layer(30);
    pub const MID_LAYER_30: Layer = Layer(31);
    pub const BOTTOM_LAYER: Layer = Layer(32);
    pub const TOP_OVERLAY: Layer = Layer(33);
    pub const BOTTOM_OVERLAY: Layer = Layer(34);
    pub const TOP_PASTE: Layer = Layer(35);
    pub const BOTTOM_PASTE: Layer = Layer(36);
    pub const TOP_SOLDER: Layer = Layer(37);
    pub const BOTTOM_SOLDER: Layer = Layer(38);
    pub const INTERNAL_PLANE_1: Layer = Layer(39);
    pub const INTERNAL_PLANE_2: Layer = Layer(40);
    pub const INTERNAL_PLANE_3: Layer = Layer(41);
    pub const INTERNAL_PLANE_4: Layer = Layer(42);
    pub const INTERNAL_PLANE_5: Layer = Layer(43);
    pub const INTERNAL_PLANE_6: Layer = Layer(44);
    pub const INTERNAL_PLANE_7: Layer = Layer(45);
    pub const INTERNAL_PLANE_8: Layer = Layer(46);
    pub const INTERNAL_PLANE_9: Layer = Layer(47);
    pub const INTERNAL_PLANE_10: Layer = Layer(48);
    pub const INTERNAL_PLANE_11: Layer = Layer(49);
    pub const INTERNAL_PLANE_12: Layer = Layer(50);
    pub const INTERNAL_PLANE_13: Layer = Layer(51);
    pub const INTERNAL_PLANE_14: Layer = Layer(52);
    pub const INTERNAL_PLANE_15: Layer = Layer(53);
    pub const INTERNAL_PLANE_16: Layer = Layer(54);
    pub const DRILL_GUIDE: Layer = Layer(55);
    pub const KEEP_OUT_LAYER: Layer = Layer(56);
    pub const MECHANICAL_1: Layer = Layer(57);
    pub const MECHANICAL_2: Layer = Layer(58);
    pub const MECHANICAL_3: Layer = Layer(59);
    pub const MECHANICAL_4: Layer = Layer(60);
    pub const MECHANICAL_5: Layer = Layer(61);
    pub const MECHANICAL_6: Layer = Layer(62);
    pub const MECHANICAL_7: Layer = Layer(63);
    pub const MECHANICAL_8: Layer = Layer(64);
    pub const MECHANICAL_9: Layer = Layer(65);
    pub const MECHANICAL_10: Layer = Layer(66);
    pub const MECHANICAL_11: Layer = Layer(67);
    pub const MECHANICAL_12: Layer = Layer(68);
    pub const MECHANICAL_13: Layer = Layer(69);
    pub const MECHANICAL_14: Layer = Layer(70);
    pub const MECHANICAL_15: Layer = Layer(71);
    pub const MECHANICAL_16: Layer = Layer(72);
    pub const DRILL_DRAWING: Layer = Layer(73);
    pub const MULTI_LAYER: Layer = Layer(74);
    pub const CONNECT_LAYER: Layer = Layer(75);
    pub const BACKGROUND_LAYER: Layer = Layer(76);
    pub const DRC_ERROR_LAYER: Layer = Layer(77);
    pub const HIGHLIGHT_LAYER: Layer = Layer(78);
    pub const GRID_COLOR_1: Layer = Layer(79);
    pub const GRID_COLOR_10: Layer = Layer(80);
    pub const PAD_HOLE_LAYER: Layer = Layer(81);
    pub const VIA_HOLE_LAYER: Layer = Layer(82);
    pub const TOP_PAD_MASTER: Layer = Layer(83);
    pub const BOTTOM_PAD_MASTER: Layer = Layer(84);
    pub const DRC_DETAIL_LAYER: Layer = Layer(85);
    // Extended mechanical layers (added in Altium Designer 18+)
    pub const MECHANICAL_17: Layer = Layer(86);
    pub const MECHANICAL_18: Layer = Layer(87);
    pub const MECHANICAL_19: Layer = Layer(88);
    pub const MECHANICAL_20: Layer = Layer(89);
    pub const MECHANICAL_21: Layer = Layer(90);
    pub const MECHANICAL_22: Layer = Layer(91);
    pub const MECHANICAL_23: Layer = Layer(92);
    pub const MECHANICAL_24: Layer = Layer(93);
    pub const MECHANICAL_25: Layer = Layer(94);
    pub const MECHANICAL_26: Layer = Layer(95);
    pub const MECHANICAL_27: Layer = Layer(96);
    pub const MECHANICAL_28: Layer = Layer(97);
    pub const MECHANICAL_29: Layer = Layer(98);
    pub const MECHANICAL_30: Layer = Layer(99);
    pub const MECHANICAL_31: Layer = Layer(100);
    pub const MECHANICAL_32: Layer = Layer(101);
    pub const UNKNOWN: Layer = Layer(255);

    /// Creates a layer from a byte value.
    #[inline]
    pub const fn from_byte(value: u8) -> Self {
        Layer(value)
    }

    /// Returns the byte value of this layer.
    #[inline]
    pub const fn to_byte(self) -> u8 {
        self.0
    }

    /// Returns the internal name of this layer.
    pub fn name(self) -> &'static str {
        match self.0 {
            0 => "NoLayer",
            1 => "TopLayer",
            2..=31 => {
                const MID_NAMES: [&str; 30] = [
                    "MidLayer1",
                    "MidLayer2",
                    "MidLayer3",
                    "MidLayer4",
                    "MidLayer5",
                    "MidLayer6",
                    "MidLayer7",
                    "MidLayer8",
                    "MidLayer9",
                    "MidLayer10",
                    "MidLayer11",
                    "MidLayer12",
                    "MidLayer13",
                    "MidLayer14",
                    "MidLayer15",
                    "MidLayer16",
                    "MidLayer17",
                    "MidLayer18",
                    "MidLayer19",
                    "MidLayer20",
                    "MidLayer21",
                    "MidLayer22",
                    "MidLayer23",
                    "MidLayer24",
                    "MidLayer25",
                    "MidLayer26",
                    "MidLayer27",
                    "MidLayer28",
                    "MidLayer29",
                    "MidLayer30",
                ];
                MID_NAMES[(self.0 - 2) as usize]
            }
            32 => "BottomLayer",
            33 => "TopOverlay",
            34 => "BottomOverlay",
            35 => "TopPaste",
            36 => "BottomPaste",
            37 => "TopSolder",
            38 => "BottomSolder",
            39..=54 => {
                const PLANE_NAMES: [&str; 16] = [
                    "InternalPlane1",
                    "InternalPlane2",
                    "InternalPlane3",
                    "InternalPlane4",
                    "InternalPlane5",
                    "InternalPlane6",
                    "InternalPlane7",
                    "InternalPlane8",
                    "InternalPlane9",
                    "InternalPlane10",
                    "InternalPlane11",
                    "InternalPlane12",
                    "InternalPlane13",
                    "InternalPlane14",
                    "InternalPlane15",
                    "InternalPlane16",
                ];
                PLANE_NAMES[(self.0 - 39) as usize]
            }
            55 => "DrillGuide",
            56 => "KeepOutLayer",
            57..=72 => {
                const MECH_NAMES: [&str; 16] = [
                    "Mechanical1",
                    "Mechanical2",
                    "Mechanical3",
                    "Mechanical4",
                    "Mechanical5",
                    "Mechanical6",
                    "Mechanical7",
                    "Mechanical8",
                    "Mechanical9",
                    "Mechanical10",
                    "Mechanical11",
                    "Mechanical12",
                    "Mechanical13",
                    "Mechanical14",
                    "Mechanical15",
                    "Mechanical16",
                ];
                MECH_NAMES[(self.0 - 57) as usize]
            }
            73 => "DrillDrawing",
            74 => "MultiLayer",
            75 => "ConnectLayer",
            76 => "BackGroundLayer",
            77 => "DRCErrorLayer",
            78 => "HighlightLayer",
            79 => "GridColor1",
            80 => "GridColor10",
            81 => "PadHoleLayer",
            82 => "ViaHoleLayer",
            83 => "TopPadMaster",
            84 => "BottomPadMaster",
            85 => "DRCDetailLayer",
            86..=101 => {
                const MECH_EXT_NAMES: [&str; 16] = [
                    "Mechanical17",
                    "Mechanical18",
                    "Mechanical19",
                    "Mechanical20",
                    "Mechanical21",
                    "Mechanical22",
                    "Mechanical23",
                    "Mechanical24",
                    "Mechanical25",
                    "Mechanical26",
                    "Mechanical27",
                    "Mechanical28",
                    "Mechanical29",
                    "Mechanical30",
                    "Mechanical31",
                    "Mechanical32",
                ];
                MECH_EXT_NAMES[(self.0 - 86) as usize]
            }
            _ => "Unknown",
        }
    }

    /// Parses a layer from its name string.
    pub fn from_name(name: &str) -> Option<Layer> {
        let name = name.trim();

        // Check standard names
        match name {
            "NoLayer" => return Some(Layer::NO_LAYER),
            "TopLayer" => return Some(Layer::TOP_LAYER),
            "BottomLayer" => return Some(Layer::BOTTOM_LAYER),
            "TopOverlay" => return Some(Layer::TOP_OVERLAY),
            "BottomOverlay" => return Some(Layer::BOTTOM_OVERLAY),
            "TopPaste" => return Some(Layer::TOP_PASTE),
            "BottomPaste" => return Some(Layer::BOTTOM_PASTE),
            "TopSolder" => return Some(Layer::TOP_SOLDER),
            "BottomSolder" => return Some(Layer::BOTTOM_SOLDER),
            "DrillGuide" => return Some(Layer::DRILL_GUIDE),
            "KeepOutLayer" => return Some(Layer::KEEP_OUT_LAYER),
            "DrillDrawing" => return Some(Layer::DRILL_DRAWING),
            "MultiLayer" => return Some(Layer::MULTI_LAYER),
            "ConnectLayer" => return Some(Layer::CONNECT_LAYER),
            "BackGroundLayer" => return Some(Layer::BACKGROUND_LAYER),
            "DRCErrorLayer" => return Some(Layer::DRC_ERROR_LAYER),
            "HighlightLayer" => return Some(Layer::HIGHLIGHT_LAYER),
            "GridColor1" => return Some(Layer::GRID_COLOR_1),
            "GridColor10" => return Some(Layer::GRID_COLOR_10),
            "PadHoleLayer" => return Some(Layer::PAD_HOLE_LAYER),
            "ViaHoleLayer" => return Some(Layer::VIA_HOLE_LAYER),
            "TopPadMaster" => return Some(Layer::TOP_PAD_MASTER),
            "BottomPadMaster" => return Some(Layer::BOTTOM_PAD_MASTER),
            "DRCDetailLayer" => return Some(Layer::DRC_DETAIL_LAYER),
            _ => {}
        }

        // Check numbered layers (MidLayer1-30, InternalPlane1-16, Mechanical1-32)
        if let Some(num_str) = name.strip_prefix("MidLayer") {
            if let Ok(num) = num_str.parse::<u8>() {
                if (1..=30).contains(&num) {
                    return Some(Layer(1 + num));
                }
            }
        }
        if let Some(num_str) = name.strip_prefix("InternalPlane") {
            if let Ok(num) = num_str.parse::<u8>() {
                if (1..=16).contains(&num) {
                    return Some(Layer(38 + num));
                }
            }
        }
        if let Some(num_str) = name.strip_prefix("Mechanical") {
            if let Ok(num) = num_str.parse::<u8>() {
                if (1..=16).contains(&num) {
                    return Some(Layer(56 + num));
                } else if (17..=32).contains(&num) {
                    return Some(Layer(69 + num)); // 86 - 17 = 69
                }
            }
        }

        None
    }

    /// Returns true if this is a copper layer (top, bottom, or mid layers).
    pub fn is_copper(self) -> bool {
        matches!(self.0, 1..=32)
    }

    /// Returns true if this is an internal plane layer.
    pub fn is_plane(self) -> bool {
        matches!(self.0, 39..=54)
    }

    /// Returns true if this is a mechanical layer (1-32).
    pub fn is_mechanical(self) -> bool {
        matches!(self.0, 57..=72 | 86..=101)
    }

    /// Returns true if this is a signal layer (top, bottom, or mid).
    pub fn is_signal(self) -> bool {
        self.is_copper()
    }

    /// Returns true if this is an overlay/silkscreen layer.
    pub fn is_overlay(self) -> bool {
        matches!(self.0, 33 | 34)
    }
}

impl FromBinary for Layer {
    fn read_from<R: std::io::Read>(reader: &mut R) -> Result<Self> {
        Ok(Layer(reader.read_u8()?))
    }
}

impl ToBinary for Layer {
    fn write_to<W: std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u8(self.0)?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        1
    }
}

impl Default for Layer {
    fn default() -> Self {
        Layer::NO_LAYER
    }
}

impl From<u8> for Layer {
    #[inline]
    fn from(value: u8) -> Self {
        Layer(value)
    }
}

impl From<Layer> for u8 {
    #[inline]
    fn from(layer: Layer) -> Self {
        layer.0
    }
}

impl fmt::Debug for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Layer::{}", self.name())
    }
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_names() {
        assert_eq!(Layer::TOP_LAYER.name(), "TopLayer");
        assert_eq!(Layer::BOTTOM_LAYER.name(), "BottomLayer");
        assert_eq!(Layer::MID_LAYER_1.name(), "MidLayer1");
        assert_eq!(Layer::MECHANICAL_16.name(), "Mechanical16");
    }

    #[test]
    fn test_layer_from_name() {
        assert_eq!(Layer::from_name("TopLayer"), Some(Layer::TOP_LAYER));
        assert_eq!(Layer::from_name("MidLayer5"), Some(Layer::MID_LAYER_5));
        assert_eq!(Layer::from_name("Mechanical10"), Some(Layer::MECHANICAL_10));
        assert_eq!(Layer::from_name("InvalidLayer"), None);
    }

    #[test]
    fn test_layer_categories() {
        assert!(Layer::TOP_LAYER.is_copper());
        assert!(Layer::BOTTOM_LAYER.is_copper());
        assert!(Layer::MID_LAYER_15.is_copper());
        assert!(!Layer::TOP_OVERLAY.is_copper());

        assert!(Layer::INTERNAL_PLANE_1.is_plane());
        assert!(!Layer::TOP_LAYER.is_plane());

        assert!(Layer::MECHANICAL_1.is_mechanical());
        assert!(!Layer::TOP_LAYER.is_mechanical());
    }
}

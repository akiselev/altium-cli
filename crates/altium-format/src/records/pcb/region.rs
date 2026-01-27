//! PCB Region record.

use std::io::Read;

use altium_format_derive::AltiumRecord;

use super::{PcbOutline, primitive::PcbPrimitiveCommon};
use crate::error::Result;
use crate::traits::FromBinary;
use crate::types::{CoordPoint, CoordRect, ParameterCollection};

/// PCB Region primitive (polygon area).
#[derive(Debug, Clone, Default)]
pub struct PcbRegion {
    /// Common primitive fields.
    pub common: PcbPrimitiveCommon,
    /// Parameters from the region data.
    pub parameters: ParameterCollection,
    /// Outline vertices.
    pub outline: Vec<CoordPoint>,
}

#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(format = "binary")]
struct PcbRegionBinary {
    #[altium(flatten)]
    common: PcbPrimitiveCommon,
    _unknown1: u32,
    _unknown2: u8,
    parameters: ParameterCollection,
    outline: PcbOutline,
}

impl FromBinary for PcbRegion {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let raw = <PcbRegionBinary as FromBinary>::read_from(reader)?;
        Ok(PcbRegion {
            common: raw.common,
            parameters: raw.parameters,
            outline: raw.outline.into(),
        })
    }
}

use crate::traits::ToBinary;
use std::io::Write;

impl ToBinary for PcbRegion {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        // Common primitive fields
        self.common.write_to(writer)?;

        // Unknown fields
        writer.write_u32::<LittleEndian>(0)?; // _unknown1
        writer.write_u8(0)?; // _unknown2

        // Parameters
        self.parameters.write_to(writer)?;

        // Outline
        let outline: PcbOutline = self.outline.clone().into();
        outline.write_to(writer)?;

        Ok(())
    }

    fn binary_size(&self) -> usize {
        let outline: PcbOutline = self.outline.clone().into();
        13 + 5 + self.parameters.binary_size() + outline.binary_size()
    }
}

impl PcbRegion {
    /// Calculate the bounding rectangle.
    pub fn calculate_bounds(&self) -> CoordRect {
        if self.outline.is_empty() {
            return CoordRect::default();
        }

        let min_x = self.outline.iter().map(|p| p.x.to_raw()).min().unwrap_or(0);
        let max_x = self.outline.iter().map(|p| p.x.to_raw()).max().unwrap_or(0);
        let min_y = self.outline.iter().map(|p| p.y.to_raw()).min().unwrap_or(0);
        let max_y = self.outline.iter().map(|p| p.y.to_raw()).max().unwrap_or(0);

        CoordRect::from_raw(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

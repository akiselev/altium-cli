//! PCB outline helper type (counted list of CoordPoint stored as f64 pairs).

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::error::Result;
use crate::traits::{FromBinary, ToBinary};
use crate::types::CoordPoint;

#[derive(Debug, Clone, Default)]
pub(crate) struct PcbOutline(pub Vec<CoordPoint>);

impl From<PcbOutline> for Vec<CoordPoint> {
    fn from(outline: PcbOutline) -> Self {
        outline.0
    }
}

impl From<Vec<CoordPoint>> for PcbOutline {
    fn from(points: Vec<CoordPoint>) -> Self {
        PcbOutline(points)
    }
}

impl FromBinary for PcbOutline {
    fn read_from<R: std::io::Read>(reader: &mut R) -> Result<Self> {
        let count = reader.read_u32::<LittleEndian>()? as usize;
        let mut points = Vec::with_capacity(count);
        for _ in 0..count {
            let x = reader.read_f64::<LittleEndian>()? as i32;
            let y = reader.read_f64::<LittleEndian>()? as i32;
            points.push(CoordPoint::from_raw(x, y));
        }
        Ok(PcbOutline(points))
    }
}

impl ToBinary for PcbOutline {
    fn write_to<W: std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32::<LittleEndian>(self.0.len() as u32)?;
        for point in &self.0 {
            writer.write_f64::<LittleEndian>(point.x.to_raw() as f64)?;
            writer.write_f64::<LittleEndian>(point.y.to_raw() as f64)?;
        }
        Ok(())
    }

    fn binary_size(&self) -> usize {
        4 + self.0.len() * 16
    }
}

//! PCB Arc binary record (60 bytes in AD26).
//!
//! Ghidra: FUN_01857610 + FUN_0185dda0.
//!
//! ```text
//! Offset  Size  Field
//!   0     13    Common Header
//!  13      4    center_x (i32)
//!  17      4    center_y (i32)
//!  21      4    radius (i32)
//!  25      8    start_angle (f64, degrees)
//!  33      8    end_angle (f64, degrees)
//!  41      4    width (i32)
//!  45      2    subpoly_index (u16)
//!  47     13    Trailing (user_routed, union_index, layer_enum, keepout)
//! ```
//! Total: 60 bytes. Arc has 13 trailing bytes (no extra bool).

use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

use super::coord::PcbCoord;
use super::primitive::{PcbCommonHeader, PcbTrailingFields};

/// PCB Arc record (60 bytes).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbArc {
    pub header: PcbCommonHeader,
    pub center_x: PcbCoord,
    pub center_y: PcbCoord,
    pub radius: PcbCoord,
    pub start_angle: f64,
    pub end_angle: f64,
    pub width: PcbCoord,
    pub subpoly_index: u16,
    pub trailing: PcbTrailingFields,
}

impl PcbArc {
    pub const SIZE: usize = 60;

    pub fn read_from(r: &mut impl Read) -> io::Result<Self> {
        let header = PcbCommonHeader::read_from(r)?;

        let mut buf = [0u8; 34]; // 4+4+4+8+8+4+2 = 34
        r.read_exact(&mut buf)?;

        let center_x = PcbCoord::from_raw(i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]));
        let center_y = PcbCoord::from_raw(i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]));
        let radius = PcbCoord::from_raw(i32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]));
        let start_angle = f64::from_le_bytes(buf[12..20].try_into().unwrap());
        let end_angle = f64::from_le_bytes(buf[20..28].try_into().unwrap());
        let width = PcbCoord::from_raw(i32::from_le_bytes([buf[28], buf[29], buf[30], buf[31]]));
        let subpoly_index = u16::from_le_bytes([buf[32], buf[33]]);

        // Trailing fields are present in AD26+ but may be absent in older files
        let trailing = PcbTrailingFields::read_13(r).unwrap_or_default();

        Ok(Self {
            header,
            center_x,
            center_y,
            radius,
            start_angle,
            end_angle,
            width,
            subpoly_index,
            trailing,
        })
    }

    pub fn write_to(&self, w: &mut impl Write) -> io::Result<()> {
        self.header.write_to(w)?;
        w.write_all(&self.center_x.to_raw().to_le_bytes())?;
        w.write_all(&self.center_y.to_raw().to_le_bytes())?;
        w.write_all(&self.radius.to_raw().to_le_bytes())?;
        w.write_all(&self.start_angle.to_le_bytes())?;
        w.write_all(&self.end_angle.to_le_bytes())?;
        w.write_all(&self.width.to_raw().to_le_bytes())?;
        w.write_all(&self.subpoly_index.to_le_bytes())?;
        self.trailing.write_to(w)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn round_trip() {
        let arc = PcbArc {
            header: PcbCommonHeader::default(),
            center_x: PcbCoord::from_mils(500.0),
            center_y: PcbCoord::from_mils(500.0),
            radius: PcbCoord::from_mils(100.0),
            start_angle: 0.0,
            end_angle: 90.0,
            width: PcbCoord::from_mils(10.0),
            subpoly_index: 0xFFFF,
            trailing: PcbTrailingFields::default(),
        };
        let mut buf = Vec::new();
        arc.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), PcbArc::SIZE);

        let parsed = PcbArc::read_from(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(arc, parsed);
    }

    #[test]
    fn size_check() {
        // 13 header + 34 type-specific + 13 trailing = 60
        assert_eq!(PcbCommonHeader::SIZE + 34 + 13, PcbArc::SIZE);
    }
}

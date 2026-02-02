//! PCB Fill binary record (50 bytes in AD26).
//!
//! Ghidra: FUN_018574c0 + FUN_0185dcd0.
//!
//! ```text
//! Offset  Size  Field
//!   0     13    Common Header
//!  13      4    corner1_x (i32)
//!  17      4    corner1_y (i32)
//!  21      4    corner2_x (i32)
//!  25      4    corner2_y (i32)
//!  29      8    rotation (f64, degrees)
//!  37     13    Trailing (user_routed, union_index, layer_enum, keepout)
//! ```
//! Total: 50 bytes. Fill has 13 trailing bytes (no extra bool).

use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

use super::coord::PcbCoord;
use super::primitive::{PcbCommonHeader, PcbTrailingFields};

/// PCB Fill record (50 bytes).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbFill {
    pub header: PcbCommonHeader,
    pub corner1_x: PcbCoord,
    pub corner1_y: PcbCoord,
    pub corner2_x: PcbCoord,
    pub corner2_y: PcbCoord,
    pub rotation: f64,
    pub trailing: PcbTrailingFields,
}

impl PcbFill {
    pub const SIZE: usize = 50;

    pub fn read_from(r: &mut impl Read) -> io::Result<Self> {
        let header = PcbCommonHeader::read_from(r)?;

        let mut buf = [0u8; 24]; // 4*4 + 8 = 24
        r.read_exact(&mut buf)?;

        let corner1_x = PcbCoord::from_raw(i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]));
        let corner1_y = PcbCoord::from_raw(i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]));
        let corner2_x = PcbCoord::from_raw(i32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]));
        let corner2_y = PcbCoord::from_raw(i32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]));
        let rotation = f64::from_le_bytes(buf[16..24].try_into().unwrap());

        // Trailing fields are present in AD26+ but may be absent in older files
        let trailing = PcbTrailingFields::read_13(r).unwrap_or_default();

        Ok(Self {
            header,
            corner1_x,
            corner1_y,
            corner2_x,
            corner2_y,
            rotation,
            trailing,
        })
    }

    pub fn write_to(&self, w: &mut impl Write) -> io::Result<()> {
        self.header.write_to(w)?;
        w.write_all(&self.corner1_x.to_raw().to_le_bytes())?;
        w.write_all(&self.corner1_y.to_raw().to_le_bytes())?;
        w.write_all(&self.corner2_x.to_raw().to_le_bytes())?;
        w.write_all(&self.corner2_y.to_raw().to_le_bytes())?;
        w.write_all(&self.rotation.to_le_bytes())?;
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
        let fill = PcbFill {
            header: PcbCommonHeader::default(),
            corner1_x: PcbCoord::from_mils(100.0),
            corner1_y: PcbCoord::from_mils(100.0),
            corner2_x: PcbCoord::from_mils(200.0),
            corner2_y: PcbCoord::from_mils(200.0),
            rotation: 45.0,
            trailing: PcbTrailingFields::default(),
        };
        let mut buf = Vec::new();
        fill.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), PcbFill::SIZE);

        let parsed = PcbFill::read_from(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(fill, parsed);
    }

    #[test]
    fn size_check() {
        // 13 header + 24 type-specific + 13 trailing = 50
        assert_eq!(PcbCommonHeader::SIZE + 24 + 13, PcbFill::SIZE);
    }
}

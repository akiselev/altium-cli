//! PCB Fill binary record (37-50 bytes depending on file format).
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
//!  37     0-13  Trailing (user_routed, union_index, layer_enum, [keepout])
//! ```
//! PcbDoc AD26: 50 bytes (13 trailing). PcbLib: may be shorter.

use serde::{Deserialize, Serialize};
use std::io::{self, Write};

use super::coord::PcbCoord;
use super::primitive::{PcbCommonHeader, PcbTrailingFields};

/// PCB Fill record.
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
    const MIN_SIZE: usize = 37; // 13 header + 24 type-specific

    /// Parse from a block data slice (after type byte + u32 len consumed).
    pub fn from_block(block: &[u8]) -> io::Result<Self> {
        if block.len() < Self::MIN_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("fill block too short: {} < {}", block.len(), Self::MIN_SIZE),
            ));
        }

        let mut cursor = std::io::Cursor::new(&block[..13]);
        let header = PcbCommonHeader::read_from(&mut cursor)?;

        let buf = &block[13..37];
        let corner1_x = PcbCoord::from_raw(i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]));
        let corner1_y = PcbCoord::from_raw(i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]));
        let corner2_x = PcbCoord::from_raw(i32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]));
        let corner2_y = PcbCoord::from_raw(i32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]));
        let rotation = f64::from_le_bytes(buf[16..24].try_into().unwrap());

        let trailing = PcbTrailingFields::from_remaining(&block[37..], false);

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

    /// Legacy: parse from a reader.
    pub fn read_from(r: &mut impl std::io::Read) -> io::Result<Self> {
        let mut data = Vec::new();
        r.read_to_end(&mut data)?;
        Self::from_block(&data)
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

    #[test]
    fn round_trip_50() {
        let fill = PcbFill {
            header: PcbCommonHeader::default(),
            corner1_x: PcbCoord::from_mils(100.0),
            corner1_y: PcbCoord::from_mils(100.0),
            corner2_x: PcbCoord::from_mils(200.0),
            corner2_y: PcbCoord::from_mils(200.0),
            rotation: 45.0,
            trailing: PcbTrailingFields {
                keepout_restrictions: Some(0),
                ..Default::default()
            },
        };
        let mut buf = Vec::new();
        fill.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), 50);

        let parsed = PcbFill::from_block(&buf).unwrap();
        assert_eq!(fill, parsed);
    }

    #[test]
    fn round_trip_short() {
        // PcbLib: no keepout
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
        assert_eq!(buf.len(), 46); // 37 + 9 (no keepout)

        let parsed = PcbFill::from_block(&buf).unwrap();
        assert_eq!(fill, parsed);
    }
}

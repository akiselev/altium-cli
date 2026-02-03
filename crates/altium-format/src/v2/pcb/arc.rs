//! PCB Arc binary record (47-60 bytes depending on file format).
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
//!  47     0-13  Trailing (user_routed, union_index, layer_enum, [keepout])
//! ```
//! PcbDoc AD26: 60 bytes (13 trailing). PcbLib: may be shorter.

use serde::{Deserialize, Serialize};
use std::io::{self, Write};

use super::coord::PcbCoord;
use super::primitive::{PcbCommonHeader, PcbTrailingFields};

/// PCB Arc record.
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
    const MIN_SIZE: usize = 47; // 13 header + 34 type-specific

    /// Parse from a block data slice (after type byte + u32 len consumed).
    pub fn from_block(block: &[u8]) -> io::Result<Self> {
        if block.len() < Self::MIN_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("arc block too short: {} < {}", block.len(), Self::MIN_SIZE),
            ));
        }

        let mut cursor = std::io::Cursor::new(&block[..13]);
        let header = PcbCommonHeader::read_from(&mut cursor)?;

        let buf = &block[13..47];
        let center_x = PcbCoord::from_raw(i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]));
        let center_y = PcbCoord::from_raw(i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]));
        let radius = PcbCoord::from_raw(i32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]));
        let start_angle = f64::from_le_bytes(buf[12..20].try_into().unwrap());
        let end_angle = f64::from_le_bytes(buf[20..28].try_into().unwrap());
        let width = PcbCoord::from_raw(i32::from_le_bytes([buf[28], buf[29], buf[30], buf[31]]));
        let subpoly_index = u16::from_le_bytes([buf[32], buf[33]]);

        let trailing = PcbTrailingFields::from_remaining(&block[47..], false);

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

    /// Legacy: parse from a reader.
    pub fn read_from(r: &mut impl std::io::Read) -> io::Result<Self> {
        let mut data = Vec::new();
        r.read_to_end(&mut data)?;
        Self::from_block(&data)
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

    #[test]
    fn round_trip_60() {
        let arc = PcbArc {
            header: PcbCommonHeader::default(),
            center_x: PcbCoord::from_mils(500.0),
            center_y: PcbCoord::from_mils(500.0),
            radius: PcbCoord::from_mils(100.0),
            start_angle: 0.0,
            end_angle: 90.0,
            width: PcbCoord::from_mils(10.0),
            subpoly_index: 0xFFFF,
            trailing: PcbTrailingFields {
                keepout_restrictions: Some(0),
                ..Default::default()
            },
        };
        let mut buf = Vec::new();
        arc.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), 60);

        let parsed = PcbArc::from_block(&buf).unwrap();
        assert_eq!(arc, parsed);
    }

    #[test]
    fn round_trip_short() {
        // PcbLib: no keepout
        let arc = PcbArc {
            header: PcbCommonHeader::default(),
            center_x: PcbCoord::from_mils(500.0),
            center_y: PcbCoord::from_mils(500.0),
            radius: PcbCoord::from_mils(100.0),
            start_angle: 0.0,
            end_angle: 90.0,
            width: PcbCoord::from_mils(10.0),
            subpoly_index: 0xFFFF,
            trailing: PcbTrailingFields::default(), // no keepout
        };
        let mut buf = Vec::new();
        arc.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), 56); // 47 + 9 (no keepout)

        let parsed = PcbArc::from_block(&buf).unwrap();
        assert_eq!(arc, parsed);
    }
}

//! PCB Track binary record (49 bytes in AD26).
//!
//! Ghidra: FUN_01856d20 + FUN_0185db80.
//!
//! ```text
//! Offset  Size  Field
//!   0     13    Common Header
//!  13      4    start_x (i32)
//!  17      4    start_y (i32)
//!  21      4    end_x (i32)
//!  25      4    end_y (i32)
//!  29      4    width (i32)
//!  33      2    subpoly_index (u16)
//!  35      1    user_routed (u8 bool)
//!  36      4    union_index (i32)
//!  40      1    track_bool (u8)
//!  41      4    layer_enum (i32)
//!  45      4    keepout_restrictions (i32)
//! ```
//! Total: 49 bytes. Track has 14 trailing bytes (extra bool at offset 40).

use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

use super::coord::PcbCoord;
use super::primitive::{PcbCommonHeader, PcbTrailingFields};

/// PCB Track record (49 bytes).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbTrack {
    pub header: PcbCommonHeader,
    pub start_x: PcbCoord,
    pub start_y: PcbCoord,
    pub end_x: PcbCoord,
    pub end_y: PcbCoord,
    pub width: PcbCoord,
    pub subpoly_index: u16,
    pub trailing: PcbTrailingFields,
}

impl PcbTrack {
    pub const SIZE: usize = 49;

    pub fn read_from(r: &mut impl Read) -> io::Result<Self> {
        let header = PcbCommonHeader::read_from(r)?;

        let mut buf = [0u8; 22]; // 4*4 + 4 + 2 = 22
        r.read_exact(&mut buf)?;

        let start_x = PcbCoord::from_raw(i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]));
        let start_y = PcbCoord::from_raw(i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]));
        let end_x = PcbCoord::from_raw(i32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]));
        let end_y = PcbCoord::from_raw(i32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]));
        let width = PcbCoord::from_raw(i32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]));
        let subpoly_index = u16::from_le_bytes([buf[20], buf[21]]);

        // Trailing fields are present in AD26+ but may be absent in older files
        let trailing = PcbTrailingFields::read_14(r).unwrap_or_default();

        Ok(Self {
            header,
            start_x,
            start_y,
            end_x,
            end_y,
            width,
            subpoly_index,
            trailing,
        })
    }

    pub fn write_to(&self, w: &mut impl Write) -> io::Result<()> {
        self.header.write_to(w)?;
        w.write_all(&self.start_x.to_raw().to_le_bytes())?;
        w.write_all(&self.start_y.to_raw().to_le_bytes())?;
        w.write_all(&self.end_x.to_raw().to_le_bytes())?;
        w.write_all(&self.end_y.to_raw().to_le_bytes())?;
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
        let track = PcbTrack {
            header: PcbCommonHeader::default(),
            start_x: PcbCoord::from_mils(100.0),
            start_y: PcbCoord::from_mils(200.0),
            end_x: PcbCoord::from_mils(300.0),
            end_y: PcbCoord::from_mils(400.0),
            width: PcbCoord::from_mils(10.0),
            subpoly_index: 0xFFFF,
            trailing: PcbTrailingFields {
                user_routed: true,
                union_index: 0,
                track_bool: Some(false),
                layer_enum: 1,
                keepout_restrictions: 0,
            },
        };
        let mut buf = Vec::new();
        track.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), PcbTrack::SIZE);

        let parsed = PcbTrack::read_from(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(track, parsed);
    }

    #[test]
    fn trailing_is_14_bytes() {
        // Track has 14 trailing bytes (extra bool), total = 13 + 22 + 14 = 49
        assert_eq!(PcbCommonHeader::SIZE + 22 + 14, PcbTrack::SIZE);
    }
}

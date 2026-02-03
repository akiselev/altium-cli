//! PCB Track binary record (45-49 bytes depending on file format).
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
//!  35     10-14 Trailing (user_routed, union_index, track_bool, layer_enum, [keepout])
//! ```
//! PcbDoc AD26: 49 bytes (14 trailing). PcbLib: 45 bytes (10 trailing, no keepout).

use serde::{Deserialize, Serialize};
use std::io::{self, Write};

use super::coord::PcbCoord;
use super::primitive::{PcbCommonHeader, PcbTrailingFields};

/// PCB Track record.
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
    /// Minimum: 13 header + 22 type-specific = 35 bytes.
    const MIN_SIZE: usize = 35;

    /// Parse from a block data slice (after type byte + u32 len consumed).
    pub fn from_block(block: &[u8]) -> io::Result<Self> {
        if block.len() < Self::MIN_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("track block too short: {} < {}", block.len(), Self::MIN_SIZE),
            ));
        }

        let mut cursor = std::io::Cursor::new(&block[..13]);
        let header = PcbCommonHeader::read_from(&mut cursor)?;

        let buf = &block[13..35];
        let start_x = PcbCoord::from_raw(i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]));
        let start_y = PcbCoord::from_raw(i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]));
        let end_x = PcbCoord::from_raw(i32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]));
        let end_y = PcbCoord::from_raw(i32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]));
        let width = PcbCoord::from_raw(i32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]));
        let subpoly_index = u16::from_le_bytes([buf[20], buf[21]]);

        let trailing = PcbTrailingFields::from_remaining(&block[35..], true);

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

    /// Legacy: parse from a reader (reads all available bytes, e.g. for PcbDoc).
    pub fn read_from(r: &mut impl std::io::Read) -> io::Result<Self> {
        let mut data = Vec::new();
        r.read_to_end(&mut data)?;
        Self::from_block(&data)
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
    #[test]
    fn round_trip_49() {
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
                keepout_restrictions: Some(0),
            },
        };
        let mut buf = Vec::new();
        track.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), 49);

        let parsed = PcbTrack::from_block(&buf).unwrap();
        assert_eq!(track, parsed);
    }

    #[test]
    fn mutation_changes_output() {
        // Prove that write_to uses typed fields, not raw passthrough
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
                keepout_restrictions: Some(0),
            },
        };
        let mut buf1 = Vec::new();
        track.write_to(&mut buf1).unwrap();

        let mut mutated = track.clone();
        mutated.start_x = PcbCoord::from_mils(999.0);
        let mut buf2 = Vec::new();
        mutated.write_to(&mut buf2).unwrap();

        assert_ne!(buf1, buf2, "mutating start_x must change output bytes");
        // Verify the mutation appears at the right offset (13-16)
        assert_ne!(buf1[13..17], buf2[13..17]);
        // And other fields unchanged
        assert_eq!(buf1[17..], buf2[17..]);
    }

    #[test]
    fn round_trip_45() {
        // PcbLib format: 45 bytes (no keepout)
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
                keepout_restrictions: None,
            },
        };
        let mut buf = Vec::new();
        track.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), 45);

        let parsed = PcbTrack::from_block(&buf).unwrap();
        assert_eq!(track, parsed);
    }
}

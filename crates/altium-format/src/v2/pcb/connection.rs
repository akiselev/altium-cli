//! PCB Connection binary record (43 bytes, ID=7).
//!
//! Ghidra: FUN_01857730 + FUN_0185de70.
//!
//! **Key difference**: Connection records use `u32 len + data` framing (NO type byte prefix).
//! This is the only primitive data stream that omits the type byte.
//!
//! ```text
//! Offset  Size  Field
//!   0     13    Common Header
//!  13      4    from_x (i32)
//!  17      4    from_y (i32)
//!  21      4    to_x (i32)
//!  25      4    to_y (i32)
//!  29      1    from_layer (u8) — Altium layer ID
//!  30      1    to_layer (u8) — Altium layer ID
//!  31      4    connection_layer_enum (i32)
//!  35      4    from_layer_enum (i32)
//!  39      4    to_layer_enum (i32)
//! ```
//! Total: 43 bytes. No trailing fields — layer info replaces them.

use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

use super::coord::PcbCoord;
use super::primitive::PcbCommonHeader;

/// PCB Connection record (43 bytes).
///
/// Connections identify ratsnest endpoints by coordinates and layers only.
/// The SDK `IPCB_Connection` has `Layer1` (from), `Layer2` (to), and `Mode`
/// properties — no pad index/reference exists on connections.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PcbConnection {
    pub header: PcbCommonHeader,
    pub from_x: PcbCoord,
    pub from_y: PcbCoord,
    pub to_x: PcbCoord,
    pub to_y: PcbCoord,
    /// From-pad layer ID (Altium TLayer value).
    pub from_layer: u8,
    /// To-pad layer ID (Altium TLayer value).
    pub to_layer: u8,
    /// Extended layer enum for the connection itself.
    pub connection_layer_enum: i32,
    /// Extended layer enum for from-pad layer.
    pub from_layer_enum: i32,
    /// Extended layer enum for to-pad layer.
    pub to_layer_enum: i32,
}

impl PcbConnection {
    pub const SIZE: usize = 43;

    pub fn read_from(r: &mut impl Read) -> io::Result<Self> {
        let header = PcbCommonHeader::read_from(r)?;

        let mut buf = [0u8; 30]; // 4*4 + 1 + 1 + 4*3 = 30
        r.read_exact(&mut buf)?;

        let from_x = PcbCoord::from_raw(i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]));
        let from_y = PcbCoord::from_raw(i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]));
        let to_x = PcbCoord::from_raw(i32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]));
        let to_y = PcbCoord::from_raw(i32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]));
        let from_layer = buf[16];
        let to_layer = buf[17];
        let connection_layer_enum = i32::from_le_bytes([buf[18], buf[19], buf[20], buf[21]]);
        let from_layer_enum = i32::from_le_bytes([buf[22], buf[23], buf[24], buf[25]]);
        let to_layer_enum = i32::from_le_bytes([buf[26], buf[27], buf[28], buf[29]]);

        Ok(Self {
            header,
            from_x,
            from_y,
            to_x,
            to_y,
            from_layer,
            to_layer,
            connection_layer_enum,
            from_layer_enum,
            to_layer_enum,
        })
    }

    pub fn write_to(&self, w: &mut impl Write) -> io::Result<()> {
        self.header.write_to(w)?;
        w.write_all(&self.from_x.to_raw().to_le_bytes())?;
        w.write_all(&self.from_y.to_raw().to_le_bytes())?;
        w.write_all(&self.to_x.to_raw().to_le_bytes())?;
        w.write_all(&self.to_y.to_raw().to_le_bytes())?;
        w.write_all(&[self.from_layer])?;
        w.write_all(&[self.to_layer])?;
        w.write_all(&self.connection_layer_enum.to_le_bytes())?;
        w.write_all(&self.from_layer_enum.to_le_bytes())?;
        w.write_all(&self.to_layer_enum.to_le_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn round_trip() {
        let conn = PcbConnection {
            header: PcbCommonHeader::default(),
            from_x: PcbCoord::from_mils(100.0),
            from_y: PcbCoord::from_mils(200.0),
            to_x: PcbCoord::from_mils(300.0),
            to_y: PcbCoord::from_mils(400.0),
            from_layer: 1,
            to_layer: 32,
            connection_layer_enum: 75,
            from_layer_enum: 1,
            to_layer_enum: 32,
        };
        let mut buf = Vec::new();
        conn.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), PcbConnection::SIZE);

        let parsed = PcbConnection::read_from(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(conn, parsed);
    }

    #[test]
    fn no_type_byte_framing() {
        // Connection uses u32 len + data, NOT u8 type + u32 len + data.
        // The SIZE should be 43 (data only, no type byte).
        assert_eq!(PcbConnection::SIZE, 43);
        assert_eq!(PcbCommonHeader::SIZE + 30, PcbConnection::SIZE);
    }
}

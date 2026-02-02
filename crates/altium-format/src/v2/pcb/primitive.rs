//! Common PCB primitive header and trailing fields.
//!
//! Every binary record starts with a 13-byte header (Ghidra FUN_01849fd0).
//! Simple primitives (Track, Arc, Fill) share trailing fields after type-specific data.

use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

use super::constants::NO_REF;

/// PCB record type byte for binary framing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum PcbObjectId {
    Arc = 1,
    Pad = 2,
    Via = 3,
    Track = 4,
    Text = 5,
    Fill = 6,
    Region = 11,
    ComponentBody = 12,
}

impl PcbObjectId {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Arc),
            2 => Some(Self::Pad),
            3 => Some(Self::Via),
            4 => Some(Self::Track),
            5 => Some(Self::Text),
            6 => Some(Self::Fill),
            11 => Some(Self::Region),
            12 => Some(Self::ComponentBody),
            _ => None,
        }
    }
}

/// Common 13-byte header for all PCB binary records.
///
/// ```text
/// Offset  Size  Field
///   0     u8    layer
///   1     u16   flags (little-endian)
///   3     u16   net         (0xFFFF = none)
///   5     u16   polygon     (0xFFFF = none)
///   7     u16   component   (0xFFFF = none)
///   9     u16   ref4        (0xFFFF = none)
///  11     u16   ref5        (0xFFFF = none)
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PcbCommonHeader {
    pub layer: u8,
    pub flags: u16,
    pub net: u16,
    pub polygon: u16,
    pub component: u16,
    pub ref4: u16,
    pub ref5: u16,
}

impl Default for PcbCommonHeader {
    fn default() -> Self {
        Self {
            layer: 0,
            flags: 0,
            net: NO_REF,
            polygon: NO_REF,
            component: NO_REF,
            ref4: NO_REF,
            ref5: NO_REF,
        }
    }
}

impl PcbCommonHeader {
    pub const SIZE: usize = 13;

    pub fn read_from(r: &mut impl Read) -> io::Result<Self> {
        let mut buf = [0u8; 13];
        r.read_exact(&mut buf)?;
        Ok(Self {
            layer: buf[0],
            flags: u16::from_le_bytes([buf[1], buf[2]]),
            net: u16::from_le_bytes([buf[3], buf[4]]),
            polygon: u16::from_le_bytes([buf[5], buf[6]]),
            component: u16::from_le_bytes([buf[7], buf[8]]),
            ref4: u16::from_le_bytes([buf[9], buf[10]]),
            ref5: u16::from_le_bytes([buf[11], buf[12]]),
        })
    }

    pub fn write_to(&self, w: &mut impl Write) -> io::Result<()> {
        w.write_all(&[self.layer])?;
        w.write_all(&self.flags.to_le_bytes())?;
        w.write_all(&self.net.to_le_bytes())?;
        w.write_all(&self.polygon.to_le_bytes())?;
        w.write_all(&self.component.to_le_bytes())?;
        w.write_all(&self.ref4.to_le_bytes())?;
        w.write_all(&self.ref5.to_le_bytes())?;
        Ok(())
    }

    // ── Flag accessors ───────────────────────────────────────────────

    /// Bit 1 (0x0002): polygon outline.
    pub fn is_polygon_outline(&self) -> bool {
        self.flags & 0x0002 != 0
    }

    /// Bit 2 (0x0004): NOT locked (inverted — set means unlocked).
    pub fn is_locked(&self) -> bool {
        self.flags & 0x0004 == 0
    }

    /// Bit 4 (0x0010): teardrop.
    pub fn is_teardrop(&self) -> bool {
        self.flags & 0x0010 != 0
    }

    /// Bit 5 (0x0020): tent top.
    pub fn is_tent_top(&self) -> bool {
        self.flags & 0x0020 != 0
    }

    /// Bit 6 (0x0040): tent bottom.
    pub fn is_tent_bottom(&self) -> bool {
        self.flags & 0x0040 != 0
    }

    /// Bit 7 (0x0080): test/fab top.
    pub fn is_test_fab_top(&self) -> bool {
        self.flags & 0x0080 != 0
    }

    /// Bit 8 (0x0100): test/fab bottom.
    pub fn is_test_fab_bottom(&self) -> bool {
        self.flags & 0x0100 != 0
    }

    pub fn has_net(&self) -> bool {
        self.net != NO_REF
    }

    pub fn has_polygon(&self) -> bool {
        self.polygon != NO_REF
    }

    pub fn has_component(&self) -> bool {
        self.component != NO_REF
    }
}

/// Trailing fields shared by simple primitives (Track, Arc, Fill).
///
/// Track has 14 trailing bytes (extra bool at offset N+5).
/// Arc and Fill have 13 trailing bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PcbTrailingFields {
    pub user_routed: bool,
    pub union_index: i32,
    /// Track-only extra boolean (at offset 40 in Track records). `None` for Arc/Fill.
    pub track_bool: Option<bool>,
    pub layer_enum: i32,
    pub keepout_restrictions: i32,
}

impl PcbTrailingFields {
    /// Read 13 trailing bytes (Arc/Fill pattern).
    pub fn read_13(r: &mut impl Read) -> io::Result<Self> {
        let mut buf = [0u8; 13];
        r.read_exact(&mut buf)?;
        Ok(Self {
            user_routed: buf[0] != 0,
            union_index: i32::from_le_bytes([buf[1], buf[2], buf[3], buf[4]]),
            track_bool: None,
            layer_enum: i32::from_le_bytes([buf[5], buf[6], buf[7], buf[8]]),
            keepout_restrictions: i32::from_le_bytes([buf[9], buf[10], buf[11], buf[12]]),
        })
    }

    /// Read 14 trailing bytes (Track pattern — extra bool at offset 5).
    pub fn read_14(r: &mut impl Read) -> io::Result<Self> {
        let mut buf = [0u8; 14];
        r.read_exact(&mut buf)?;
        Ok(Self {
            user_routed: buf[0] != 0,
            union_index: i32::from_le_bytes([buf[1], buf[2], buf[3], buf[4]]),
            track_bool: Some(buf[5] != 0),
            layer_enum: i32::from_le_bytes([buf[6], buf[7], buf[8], buf[9]]),
            keepout_restrictions: i32::from_le_bytes([buf[10], buf[11], buf[12], buf[13]]),
        })
    }

    /// Write trailing bytes (13 for Arc/Fill, 14 for Track).
    pub fn write_to(&self, w: &mut impl Write) -> io::Result<()> {
        w.write_all(&[self.user_routed as u8])?;
        w.write_all(&self.union_index.to_le_bytes())?;
        if let Some(tb) = self.track_bool {
            w.write_all(&[tb as u8])?;
        }
        w.write_all(&self.layer_enum.to_le_bytes())?;
        w.write_all(&self.keepout_restrictions.to_le_bytes())?;
        Ok(())
    }

    /// Byte size of trailing fields (13 for Arc/Fill, 14 for Track).
    pub fn size(&self) -> usize {
        if self.track_bool.is_some() { 14 } else { 13 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn header_round_trip() {
        let hdr = PcbCommonHeader {
            layer: 1,
            flags: 0x0024,
            net: 5,
            polygon: NO_REF,
            component: 42,
            ref4: NO_REF,
            ref5: NO_REF,
        };
        let mut buf = Vec::new();
        hdr.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), 13);

        let parsed = PcbCommonHeader::read_from(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(hdr, parsed);
    }

    #[test]
    fn header_flags() {
        let hdr = PcbCommonHeader {
            flags: 0x0024,
            ..Default::default()
        };
        assert!(hdr.is_tent_top()); // 0x0020
        assert!(!hdr.is_locked()); // 0x0004 set = NOT locked
        assert!(!hdr.is_polygon_outline());
    }

    #[test]
    fn trailing_13_round_trip() {
        let tf = PcbTrailingFields {
            user_routed: true,
            union_index: 7,
            track_bool: None,
            layer_enum: 100,
            keepout_restrictions: 0,
        };
        let mut buf = Vec::new();
        tf.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), 13);

        let parsed = PcbTrailingFields::read_13(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(tf, parsed);
    }

    #[test]
    fn trailing_14_round_trip() {
        let tf = PcbTrailingFields {
            user_routed: false,
            union_index: -1,
            track_bool: Some(true),
            layer_enum: 50,
            keepout_restrictions: 3,
        };
        let mut buf = Vec::new();
        tf.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), 14);

        let parsed = PcbTrailingFields::read_14(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(tf, parsed);
    }
}

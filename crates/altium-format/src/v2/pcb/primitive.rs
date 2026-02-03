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
/// In PcbDoc (AD26): Track has 14 bytes, Arc/Fill have 13 bytes.
/// In PcbLib: records may be shorter — `keepout_restrictions` may be absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PcbTrailingFields {
    pub user_routed: bool,
    pub union_index: i32,
    /// Track-only extra boolean (at offset 40 in Track records). `None` for Arc/Fill.
    pub track_bool: Option<bool>,
    pub layer_enum: i32,
    /// Keepout restrictions. `None` if trailing data was too short (PcbLib).
    pub keepout_restrictions: Option<i32>,
}

impl PcbTrailingFields {
    /// Read trailing fields from a byte slice (after type-specific data).
    ///
    /// `has_track_bool`: true for Track records (extra bool between union_index and layer_enum).
    ///
    /// Reads adaptively based on available bytes:
    /// - 0 bytes: all defaults
    /// - 1 byte: user_routed only
    /// - 5 bytes: + union_index
    /// - 6 bytes (Track): + track_bool
    /// - 9/10 bytes: + layer_enum
    /// - 13/14 bytes: + keepout_restrictions
    pub fn from_remaining(data: &[u8], has_track_bool: bool) -> Self {
        let mut tf = Self::default();
        let len = data.len();
        if len == 0 {
            return tf;
        }

        tf.user_routed = data[0] != 0;
        if len < 5 {
            return tf;
        }
        tf.union_index = i32::from_le_bytes([data[1], data[2], data[3], data[4]]);

        let mut offset = 5;
        if has_track_bool {
            if len <= offset {
                return tf;
            }
            tf.track_bool = Some(data[offset] != 0);
            offset += 1;
        }

        if len < offset + 4 {
            return tf;
        }
        tf.layer_enum = i32::from_le_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
        ]);
        offset += 4;

        if len < offset + 4 {
            return tf;
        }
        tf.keepout_restrictions = Some(i32::from_le_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
        ]));

        tf
    }

    /// Read 13 trailing bytes (Arc/Fill pattern, PcbDoc AD26).
    pub fn read_13(r: &mut impl Read) -> io::Result<Self> {
        let mut buf = [0u8; 13];
        r.read_exact(&mut buf)?;
        Ok(Self::from_remaining(&buf, false))
    }

    /// Read 14 trailing bytes (Track pattern, PcbDoc AD26).
    pub fn read_14(r: &mut impl Read) -> io::Result<Self> {
        let mut buf = [0u8; 14];
        r.read_exact(&mut buf)?;
        Ok(Self::from_remaining(&buf, true))
    }

    /// Write trailing bytes. Size depends on which fields are present.
    pub fn write_to(&self, w: &mut impl Write) -> io::Result<()> {
        w.write_all(&[self.user_routed as u8])?;
        w.write_all(&self.union_index.to_le_bytes())?;
        if let Some(tb) = self.track_bool {
            w.write_all(&[tb as u8])?;
        }
        w.write_all(&self.layer_enum.to_le_bytes())?;
        if let Some(keepout) = self.keepout_restrictions {
            w.write_all(&keepout.to_le_bytes())?;
        }
        Ok(())
    }

    /// Byte size of trailing fields as written.
    pub fn size(&self) -> usize {
        let mut n = 1 + 4 + 4; // user_routed + union_index + layer_enum
        if self.track_bool.is_some() {
            n += 1;
        }
        if self.keepout_restrictions.is_some() {
            n += 4;
        }
        n
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
            keepout_restrictions: Some(0),
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
            keepout_restrictions: Some(3),
        };
        let mut buf = Vec::new();
        tf.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), 14);

        let parsed = PcbTrailingFields::read_14(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(tf, parsed);
    }

    #[test]
    fn trailing_10_round_trip() {
        // PcbLib Track: 10 trailing bytes (no keepout)
        let tf = PcbTrailingFields {
            user_routed: true,
            union_index: 0,
            track_bool: Some(false),
            layer_enum: 1,
            keepout_restrictions: None,
        };
        let mut buf = Vec::new();
        tf.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), 10);

        let parsed = PcbTrailingFields::from_remaining(&buf, true);
        assert_eq!(tf, parsed);
    }

    #[test]
    fn trailing_9_round_trip() {
        // PcbLib Arc/Fill: 9 trailing bytes (no keepout)
        let tf = PcbTrailingFields {
            user_routed: false,
            union_index: 5,
            track_bool: None,
            layer_enum: 2,
            keepout_restrictions: None,
        };
        let mut buf = Vec::new();
        tf.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), 9);

        let parsed = PcbTrailingFields::from_remaining(&buf, false);
        assert_eq!(tf, parsed);
    }
}

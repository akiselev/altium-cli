//! Binary read/write helpers for PCB record parsing.
//!
//! These functions operate on byte slices at a given offset, using little-endian
//! byte order to match Altium's binary format. They are used by hand-written
//! PCB record parsers to read and write individual fields from raw binary data.

use crate::coord::{AltiumCoord, PcbCoord};

// ---------------------------------------------------------------------------
// Read helpers — all read from `&[u8]` at `offset`
// ---------------------------------------------------------------------------

/// Reads a signed 8-bit integer at the given offset.
#[inline]
pub fn read_i8(data: &[u8], offset: usize) -> i8 {
    data[offset] as i8
}

/// Reads an unsigned 8-bit integer at the given offset.
#[inline]
pub fn read_u8(data: &[u8], offset: usize) -> u8 {
    data[offset]
}

/// Reads a signed 16-bit little-endian integer at the given offset.
#[inline]
pub fn read_i16_le(data: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes([data[offset], data[offset + 1]])
}

/// Reads an unsigned 16-bit little-endian integer at the given offset.
#[inline]
pub fn read_u16_le(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

/// Reads a signed 32-bit little-endian integer at the given offset.
#[inline]
pub fn read_i32_le(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

/// Reads an unsigned 32-bit little-endian integer at the given offset.
#[inline]
pub fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

/// Reads a 64-bit little-endian IEEE 754 float at the given offset.
#[inline]
pub fn read_f64_le(data: &[u8], offset: usize) -> f64 {
    f64::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ])
}

/// Reads a boolean from a single byte at the given offset.
///
/// Any non-zero value is treated as `true`.
#[inline]
pub fn read_bool(data: &[u8], offset: usize) -> bool {
    data[offset] != 0
}

// ---------------------------------------------------------------------------
// Write helpers — all write to `&mut [u8]` at `offset`
// ---------------------------------------------------------------------------

/// Writes a signed 8-bit integer at the given offset.
#[inline]
pub fn write_i8(data: &mut [u8], offset: usize, value: i8) {
    data[offset] = value as u8;
}

/// Writes an unsigned 8-bit integer at the given offset.
#[inline]
pub fn write_u8(data: &mut [u8], offset: usize, value: u8) {
    data[offset] = value;
}

/// Writes a signed 16-bit little-endian integer at the given offset.
#[inline]
pub fn write_i16_le(data: &mut [u8], offset: usize, value: i16) {
    let bytes = value.to_le_bytes();
    data[offset] = bytes[0];
    data[offset + 1] = bytes[1];
}

/// Writes an unsigned 16-bit little-endian integer at the given offset.
#[inline]
pub fn write_u16_le(data: &mut [u8], offset: usize, value: u16) {
    let bytes = value.to_le_bytes();
    data[offset] = bytes[0];
    data[offset + 1] = bytes[1];
}

/// Writes a signed 32-bit little-endian integer at the given offset.
#[inline]
pub fn write_i32_le(data: &mut [u8], offset: usize, value: i32) {
    let bytes = value.to_le_bytes();
    data[offset] = bytes[0];
    data[offset + 1] = bytes[1];
    data[offset + 2] = bytes[2];
    data[offset + 3] = bytes[3];
}

/// Writes an unsigned 32-bit little-endian integer at the given offset.
#[inline]
pub fn write_u32_le(data: &mut [u8], offset: usize, value: u32) {
    let bytes = value.to_le_bytes();
    data[offset] = bytes[0];
    data[offset + 1] = bytes[1];
    data[offset + 2] = bytes[2];
    data[offset + 3] = bytes[3];
}

/// Writes a 64-bit little-endian IEEE 754 float at the given offset.
#[inline]
pub fn write_f64_le(data: &mut [u8], offset: usize, value: f64) {
    let bytes = value.to_le_bytes();
    data[offset] = bytes[0];
    data[offset + 1] = bytes[1];
    data[offset + 2] = bytes[2];
    data[offset + 3] = bytes[3];
    data[offset + 4] = bytes[4];
    data[offset + 5] = bytes[5];
    data[offset + 6] = bytes[6];
    data[offset + 7] = bytes[7];
}

/// Writes a boolean as a single byte at the given offset.
///
/// `true` is written as `1`, `false` as `0`.
#[inline]
pub fn write_bool(data: &mut [u8], offset: usize, value: bool) {
    data[offset] = if value { 1 } else { 0 };
}

// ---------------------------------------------------------------------------
// PcbCoord helpers
// ---------------------------------------------------------------------------

/// Reads a `PcbCoord` (i32 little-endian) at the given offset.
#[inline]
pub fn read_pcb_coord(data: &[u8], offset: usize) -> PcbCoord {
    PcbCoord::from_raw(read_i32_le(data, offset))
}

/// Writes a `PcbCoord` (i32 little-endian) at the given offset.
#[inline]
pub fn write_pcb_coord(data: &mut [u8], offset: usize, value: PcbCoord) {
    write_i32_le(data, offset, value.to_raw());
}

// ---------------------------------------------------------------------------
// PcbCommonHeader
// ---------------------------------------------------------------------------

/// 13-byte binary header shared by all PCB primitives.
///
/// This header appears at the start of every PCB record and contains the
/// layer assignment, flags, net reference, and several cross-reference indices
/// (polygon, component, etc.).
///
/// Layout (little-endian):
/// ```text
/// Offset  Size  Field
///   0       1   layer (u8)
///   1       2   flags (u16)
///   3       2   net (u16)
///   5       2   polygon_ref (u16)
///   7       2   component_ref (u16)
///   9       2   ref4 (u16)
///  11       2   ref5 (u16)
/// Total: 13 bytes
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct PcbCommonHeader {
    pub layer: u8,
    pub flags: u16,
    pub net: u16,
    pub polygon_ref: u16,
    pub component_ref: u16,
    pub ref4: u16,
    pub ref5: u16,
}

impl PcbCommonHeader {
    /// Size of the header in bytes.
    pub const SIZE: usize = 13;

    /// Reads a `PcbCommonHeader` from binary data at the given offset.
    ///
    /// # Panics
    ///
    /// Panics if `offset + 13` exceeds `data.len()`.
    pub fn read(data: &[u8], offset: usize) -> Self {
        PcbCommonHeader {
            layer: read_u8(data, offset),
            flags: read_u16_le(data, offset + 1),
            net: read_u16_le(data, offset + 3),
            polygon_ref: read_u16_le(data, offset + 5),
            component_ref: read_u16_le(data, offset + 7),
            ref4: read_u16_le(data, offset + 9),
            ref5: read_u16_le(data, offset + 11),
        }
    }

    /// Writes this `PcbCommonHeader` into binary data at the given offset.
    ///
    /// # Panics
    ///
    /// Panics if `offset + 13` exceeds `data.len()`.
    pub fn write(&self, data: &mut [u8], offset: usize) {
        write_u8(data, offset, self.layer);
        write_u16_le(data, offset + 1, self.flags);
        write_u16_le(data, offset + 3, self.net);
        write_u16_le(data, offset + 5, self.polygon_ref);
        write_u16_le(data, offset + 7, self.component_ref);
        write_u16_le(data, offset + 9, self.ref4);
        write_u16_le(data, offset + 11, self.ref5);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------------------
    // Pascal string helpers
    // ---------------------------------------------------------------------------

    /// Reads a Pascal-style string from binary data.
    ///
    /// Pascal strings in Altium's binary format use a length prefix followed by
    /// the string bytes (no null terminator). The length prefix is either:
    /// - 1 byte for short strings (when the first byte is less than 0xFF and the
    ///   context uses 1-byte prefixes), or
    /// - 4 bytes (u32 little-endian) for longer strings.
    ///
    /// This function reads a **1-byte length prefix** variant (the most common
    /// form in PCB binary records). Returns a tuple of `(string_slice, bytes_consumed)`
    /// where `bytes_consumed` includes the length prefix byte.
    ///
    /// # Panics
    ///
    /// Panics if `offset + 1 + len` exceeds `data.len()`.
    pub fn read_pascal_string(data: &[u8], offset: usize) -> (&str, usize) {
        let len = data[offset] as usize;
        let start = offset + 1;
        let end = start + len;
        let s =
            std::str::from_utf8(&data[start..end]).expect("pascal string contains invalid UTF-8");
        (s, 1 + len)
    }

    /// Reads a Pascal-style string with a 4-byte (u32 LE) length prefix.
    ///
    /// Returns a tuple of `(string_slice, bytes_consumed)` where `bytes_consumed`
    /// includes the 4-byte length prefix.
    ///
    /// # Panics
    ///
    /// Panics if `offset + 4 + len` exceeds `data.len()`.
    pub fn read_pascal_string_u32(data: &[u8], offset: usize) -> (&str, usize) {
        let len = read_u32_le(data, offset) as usize;
        let start = offset + 4;
        let end = start + len;
        let s = std::str::from_utf8(&data[start..end])
            .expect("pascal string (u32) contains invalid UTF-8");
        (s, 4 + len)
    }

    /// Writes a Pascal-style string with a 1-byte length prefix.
    ///
    /// Writes the length byte followed by the string bytes into `data` starting
    /// at `offset`. Returns the number of bytes written (1 + string length).
    ///
    /// # Panics
    ///
    /// Panics if the string length exceeds 255 bytes, or if the destination
    /// slice is too small.
    pub fn write_pascal_string(data: &mut [u8], offset: usize, s: &str) -> usize {
        let bytes = s.as_bytes();
        assert!(
            bytes.len() <= 255,
            "pascal string too long: {} bytes (max 255)",
            bytes.len()
        );
        data[offset] = bytes.len() as u8;
        data[offset + 1..offset + 1 + bytes.len()].copy_from_slice(bytes);
        1 + bytes.len()
    }

    /// Writes a Pascal-style string with a 4-byte (u32 LE) length prefix.
    ///
    /// Writes the 4-byte length followed by the string bytes into `data` starting
    /// at `offset`. Returns the number of bytes written (4 + string length).
    ///
    /// # Panics
    ///
    /// Panics if the destination slice is too small.
    pub fn write_pascal_string_u32(data: &mut [u8], offset: usize, s: &str) -> usize {
        let bytes = s.as_bytes();
        write_u32_le(data, offset, bytes.len() as u32);
        data[offset + 4..offset + 4 + bytes.len()].copy_from_slice(bytes);
        4 + bytes.len()
    }

    #[test]
    fn read_write_i32() {
        let mut buf = [0u8; 8];
        let values: &[i32] = &[0, 1, -1, i32::MAX, i32::MIN, 0x12345678];
        for &v in values {
            write_i32_le(&mut buf, 2, v);
            assert_eq!(read_i32_le(&buf, 2), v, "roundtrip failed for {v}");
        }
    }

    #[test]
    fn read_write_f64() {
        let mut buf = [0u8; 16];
        let values: &[f64] = &[0.0, 1.0, -1.0, 3.14159265358979, f64::MAX, f64::MIN];
        for &v in values {
            write_f64_le(&mut buf, 4, v);
            assert_eq!(read_f64_le(&buf, 4), v, "roundtrip failed for {v}");
        }
    }

    #[test]
    fn read_write_pcb_coord() {
        let mut buf = [0u8; 8];
        let values = [
            PcbCoord::from_raw(0),
            PcbCoord::from_raw(100_000),
            PcbCoord::from_raw(-50_000),
            PcbCoord::from_raw(i32::MAX),
            PcbCoord::from_raw(i32::MIN),
        ];
        for &v in &values {
            write_pcb_coord(&mut buf, 2, v);
            assert_eq!(read_pcb_coord(&buf, 2), v, "roundtrip failed for {:?}", v);
        }
    }

    #[test]
    fn pcb_common_header_roundtrip() {
        let header = PcbCommonHeader {
            layer: 42,
            flags: 0xABCD,
            net: 7,
            polygon_ref: 100,
            component_ref: 200,
            ref4: 300,
            ref5: 400,
        };

        let mut buf = [0u8; 32];
        let base = 5; // non-zero offset to test alignment independence
        header.write(&mut buf, base);

        let restored = PcbCommonHeader::read(&buf, base);
        assert_eq!(header, restored);

        // Verify the size constant matches actual layout
        assert_eq!(PcbCommonHeader::SIZE, 13);
    }

    #[test]
    fn pascal_string_roundtrip() {
        let mut buf = [0u8; 64];

        // Short ASCII string
        let written = write_pascal_string(&mut buf, 0, "hello");
        assert_eq!(written, 6); // 1 byte length + 5 bytes data
        let (s, consumed) = read_pascal_string(&buf, 0);
        assert_eq!(s, "hello");
        assert_eq!(consumed, 6);

        // Empty string
        let written = write_pascal_string(&mut buf, 10, "");
        assert_eq!(written, 1); // just the length byte (0)
        let (s, consumed) = read_pascal_string(&buf, 10);
        assert_eq!(s, "");
        assert_eq!(consumed, 1);

        // Longer string at non-zero offset
        let test_str = "PCB Track 42";
        let written = write_pascal_string(&mut buf, 20, test_str);
        assert_eq!(written, 1 + test_str.len());
        let (s, consumed) = read_pascal_string(&buf, 20);
        assert_eq!(s, test_str);
        assert_eq!(consumed, 1 + test_str.len());
    }

    #[test]
    fn pascal_string_u32_roundtrip() {
        let mut buf = [0u8; 64];

        let written = write_pascal_string_u32(&mut buf, 0, "test");
        assert_eq!(written, 8); // 4 byte length + 4 bytes data
        let (s, consumed) = read_pascal_string_u32(&buf, 0);
        assert_eq!(s, "test");
        assert_eq!(consumed, 8);
    }

    #[test]
    fn read_write_i8() {
        let mut buf = [0u8; 4];
        for v in [0i8, 1, -1, 127, -128] {
            write_i8(&mut buf, 1, v);
            assert_eq!(read_i8(&buf, 1), v);
        }
    }

    #[test]
    fn read_write_u8() {
        let mut buf = [0u8; 4];
        for v in [0u8, 1, 127, 255] {
            write_u8(&mut buf, 1, v);
            assert_eq!(read_u8(&buf, 1), v);
        }
    }

    #[test]
    fn read_write_i16_le() {
        let mut buf = [0u8; 4];
        for v in [0i16, 1, -1, i16::MAX, i16::MIN] {
            write_i16_le(&mut buf, 1, v);
            assert_eq!(read_i16_le(&buf, 1), v);
        }
    }

    #[test]
    fn read_write_u16_le() {
        let mut buf = [0u8; 4];
        for v in [0u16, 1, 0x8000, u16::MAX] {
            write_u16_le(&mut buf, 1, v);
            assert_eq!(read_u16_le(&buf, 1), v);
        }
    }

    #[test]
    fn read_write_u32_le() {
        let mut buf = [0u8; 8];
        for v in [0u32, 1, 0x80000000, u32::MAX] {
            write_u32_le(&mut buf, 2, v);
            assert_eq!(read_u32_le(&buf, 2), v);
        }
    }

    #[test]
    fn read_write_bool() {
        let mut buf = [0u8; 4];

        write_bool(&mut buf, 0, true);
        assert!(read_bool(&buf, 0));

        write_bool(&mut buf, 0, false);
        assert!(!read_bool(&buf, 0));

        // Any non-zero byte reads as true
        buf[1] = 0xFF;
        assert!(read_bool(&buf, 1));
    }

    #[test]
    fn pcb_common_header_size_matches_layout() {
        // Verify the SIZE constant matches the actual field layout:
        // layer(1) + flags(2) + net(2) + polygon_ref(2) + component_ref(2) + ref4(2) + ref5(2) = 13
        assert_eq!(PcbCommonHeader::SIZE, 1 + 2 + 2 + 2 + 2 + 2 + 2);
    }

    #[test]
    fn pcb_common_header_individual_fields() {
        // Write a header and verify each field's byte position manually
        let header = PcbCommonHeader {
            layer: 0x1F,
            flags: 0x0203,
            net: 0x0405,
            polygon_ref: 0x0607,
            component_ref: 0x0809,
            ref4: 0x0A0B,
            ref5: 0x0C0D,
        };

        let mut buf = [0u8; 13];
        header.write(&mut buf, 0);

        // layer at offset 0
        assert_eq!(buf[0], 0x1F);
        // flags at offset 1 (LE)
        assert_eq!(buf[1], 0x03);
        assert_eq!(buf[2], 0x02);
        // net at offset 3 (LE)
        assert_eq!(buf[3], 0x05);
        assert_eq!(buf[4], 0x04);
        // polygon_ref at offset 5 (LE)
        assert_eq!(buf[5], 0x07);
        assert_eq!(buf[6], 0x06);
        // component_ref at offset 7 (LE)
        assert_eq!(buf[7], 0x09);
        assert_eq!(buf[8], 0x08);
        // ref4 at offset 9 (LE)
        assert_eq!(buf[9], 0x0B);
        assert_eq!(buf[10], 0x0A);
        // ref5 at offset 11 (LE)
        assert_eq!(buf[11], 0x0D);
        assert_eq!(buf[12], 0x0C);
    }
}

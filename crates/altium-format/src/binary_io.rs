//! Layer 4 of the 5-layer parsing stack: binary reader and writer primitives.
//!
//! `BinaryReader` wraps a borrowed byte slice and provides typed read methods
//! that advance a position cursor.  `BinaryWriter` accumulates bytes into an
//! owned `Vec<u8>` via typed write methods.
//!
//! Both types handle the Borland Turbo Pascal Real48 floating-point format used
//! throughout Altium binary streams.

use altium_format_types::{Coord, CoordPoint};

use crate::{AltiumFormatError, Result};

// ---------------------------------------------------------------------------
// BinaryReader
// ---------------------------------------------------------------------------

pub(crate) struct BinaryReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BinaryReader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub(crate) fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub(crate) fn position(&self) -> usize {
        self.pos
    }

    fn check_available(&self, needed: usize) -> Result<()> {
        let available = self.remaining();
        if available < needed {
            return Err(AltiumFormatError::BinaryReadPastEnd {
                offset: self.pos,
                needed,
                available,
            });
        }
        Ok(())
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8> {
        self.check_available(1)?;
        let v = self.data[self.pos];
        self.pos += 1;
        Ok(v)
    }

    pub(crate) fn read_i8(&mut self) -> Result<i8> {
        Ok(self.read_u8()? as i8)
    }

    pub(crate) fn read_u16_le(&mut self) -> Result<u16> {
        self.check_available(2)?;
        let bytes = [self.data[self.pos], self.data[self.pos + 1]];
        self.pos += 2;
        Ok(u16::from_le_bytes(bytes))
    }

    pub(crate) fn read_i16_le(&mut self) -> Result<i16> {
        self.check_available(2)?;
        let bytes = [self.data[self.pos], self.data[self.pos + 1]];
        self.pos += 2;
        Ok(i16::from_le_bytes(bytes))
    }

    pub(crate) fn read_u32_le(&mut self) -> Result<u32> {
        self.check_available(4)?;
        let bytes = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ];
        self.pos += 4;
        Ok(u32::from_le_bytes(bytes))
    }

    pub(crate) fn read_i32_le(&mut self) -> Result<i32> {
        self.check_available(4)?;
        let bytes = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ];
        self.pos += 4;
        Ok(i32::from_le_bytes(bytes))
    }

    pub(crate) fn read_u64_le(&mut self) -> Result<u64> {
        self.check_available(8)?;
        let bytes = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
            self.data[self.pos + 4],
            self.data[self.pos + 5],
            self.data[self.pos + 6],
            self.data[self.pos + 7],
        ];
        self.pos += 8;
        Ok(u64::from_le_bytes(bytes))
    }

    pub(crate) fn read_i64_le(&mut self) -> Result<i64> {
        self.check_available(8)?;
        let bytes = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
            self.data[self.pos + 4],
            self.data[self.pos + 5],
            self.data[self.pos + 6],
            self.data[self.pos + 7],
        ];
        self.pos += 8;
        Ok(i64::from_le_bytes(bytes))
    }

    pub(crate) fn read_f32_le(&mut self) -> Result<f32> {
        self.check_available(4)?;
        let bytes = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ];
        self.pos += 4;
        Ok(f32::from_le_bytes(bytes))
    }

    pub(crate) fn read_f64_le(&mut self) -> Result<f64> {
        self.check_available(8)?;
        let bytes = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
            self.data[self.pos + 4],
            self.data[self.pos + 5],
            self.data[self.pos + 6],
            self.data[self.pos + 7],
        ];
        self.pos += 8;
        Ok(f64::from_le_bytes(bytes))
    }

    /// Reads a 6-byte Borland Turbo Pascal Real48 and converts it to an IEEE f64.
    ///
    /// Layout: byte[0] = 8-bit biased exponent, bytes[1..5] = 40-bit mantissa,
    /// MSB of byte[5] = sign bit. Exponent 0 means 0.0.
    pub(crate) fn read_real48(&mut self) -> Result<f64> {
        self.check_available(6)?;
        let bytes = &self.data[self.pos..self.pos + 6];
        self.pos += 6;
        let exponent = bytes[0];
        if exponent == 0 {
            return Ok(0.0);
        }
        let sign = (bytes[5] & 0x80) as u64;
        // Collect 39-bit mantissa fraction from bytes 1-5 (byte 5 contributes 7 bits).
        let mantissa_raw = (bytes[1] as u64)
            | ((bytes[2] as u64) << 8)
            | ((bytes[3] as u64) << 16)
            | ((bytes[4] as u64) << 24)
            | (((bytes[5] & 0x7F) as u64) << 32);
        // Map 39-bit Real48 mantissa to 52-bit IEEE mantissa: left-shift by 13
        // to align the MSBs (Real48 bit 38 → IEEE bit 51).
        let ieee_mant = mantissa_raw << 13;
        let ieee_exp = ((exponent as i64 - 129 + 1023) as u64) & 0x7FF;
        let bits = (sign << 56) | (ieee_exp << 52) | ieee_mant;
        Ok(f64::from_bits(bits))
    }

    /// Reads one byte: 0x00 = false, any other value = true.
    pub(crate) fn read_bool(&mut self) -> Result<bool> {
        Ok(self.read_u8()? != 0)
    }

    /// Reads a 4-byte little-endian i32 and wraps it as a `Coord`.
    pub(crate) fn read_coord(&mut self) -> Result<Coord> {
        let raw = self.read_i32_le()?;
        Ok(Coord::from_internal(raw))
    }

    /// Reads two consecutive 4-byte little-endian i32 values as a `CoordPoint` (x, y).
    pub(crate) fn read_coord_point(&mut self) -> Result<CoordPoint> {
        let x = self.read_coord()?;
        let y = self.read_coord()?;
        Ok(CoordPoint::new(x, y))
    }

    /// Reads a length-prefixed string block: i32 LE byte count followed by Windows-1252 bytes.
    pub(crate) fn read_string_block(&mut self) -> Result<String> {
        let len = self.read_i32_le()? as usize;
        let bytes = self.read_bytes(len)?;
        let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
        Ok(decoded.into_owned())
    }

    /// Reads a Pascal-style string: u8 byte count followed by Windows-1252 bytes.
    pub(crate) fn read_pascal_string(&mut self) -> Result<String> {
        let len = self.read_u8()? as usize;
        let bytes = self.read_bytes(len)?;
        let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
        Ok(decoded.into_owned())
    }

    /// Returns a slice of `count` bytes starting at the current position and advances by `count`.
    pub(crate) fn read_bytes(&mut self, count: usize) -> Result<&'a [u8]> {
        self.check_available(count)?;
        let slice = &self.data[self.pos..self.pos + count];
        self.pos += count;
        Ok(slice)
    }

    /// Advances the position by `count` bytes without returning the data.
    pub(crate) fn skip(&mut self, count: usize) -> Result<()> {
        self.check_available(count)?;
        self.pos += count;
        Ok(())
    }

    /// Reads `count` reserved bytes and asserts they are all zero.
    ///
    /// Returns an error if any byte is non-zero, including the offset and
    /// actual values for debugging. Use this instead of `skip()` for reserved
    /// fields to enforce the fail-fast invariant.
    pub(crate) fn read_reserved_zero(&mut self, count: usize) -> Result<()> {
        let offset = self.pos;
        let bytes = self.read_bytes(count)?;
        if bytes.iter().any(|&b| b != 0) {
            return Err(AltiumFormatError::InvalidParamValue {
                key: format!("reserved bytes at offset {offset}"),
                detail: format!("expected {count} zero bytes, got {bytes:02X?}",),
            });
        }
        Ok(())
    }

    /// Creates a sub-reader over the next `len` bytes and advances the parent by `len`.
    pub(crate) fn sub_reader(&mut self, len: usize) -> Result<BinaryReader<'a>> {
        self.check_available(len)?;
        let sub_data = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Ok(BinaryReader::new(sub_data))
    }

    /// Reads a fixed-size UTF-16LE string buffer (`char_count` WideChars = `char_count * 2` bytes).
    ///
    /// Decodes up to the first null WideChar. Bytes after the null terminator are
    /// consumed but ignored (Delphi heap junk in fixed-size `WideChar[N]` buffers).
    pub(crate) fn read_wide_string_fixed(&mut self, char_count: usize) -> Result<String> {
        let byte_count = char_count * 2;
        let bytes = self.read_bytes(byte_count)?;
        // Find null terminator (00 00 at even offset).
        let mut end = byte_count;
        for i in (0..byte_count).step_by(2) {
            if bytes[i] == 0 && bytes[i + 1] == 0 {
                end = i;
                break;
            }
        }
        let (decoded, _, had_errors) = encoding_rs::UTF_16LE.decode(&bytes[..end]);
        if had_errors {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "UTF-16LE string".to_owned(),
                detail: format!(
                    "invalid UTF-16LE at buffer offset {}",
                    self.pos - byte_count
                ),
            });
        }
        Ok(decoded.into_owned())
    }

    /// Returns an error if there are any bytes remaining.
    pub(crate) fn assert_exhausted(&self) -> Result<()> {
        let count = self.remaining();
        if count > 0 {
            return Err(AltiumFormatError::UnexpectedTrailingData {
                offset: self.pos,
                count,
            });
        }
        Ok(())
    }

    /// Reads exactly `N` elements using the provided closure.
    pub(crate) fn read_array<T, const N: usize>(
        &mut self,
        mut read_one: impl FnMut(&mut Self) -> Result<T>,
    ) -> Result<[T; N]>
    where
        T: Copy + Default,
    {
        let mut arr = [T::default(); N];
        for slot in arr.iter_mut() {
            *slot = read_one(self)?;
        }
        Ok(arr)
    }
}

/// Read a Pascal-string prefix (u8 length + ASCII bytes) from the start of a
/// byte buffer, returning (name, remaining_bytes).
///
/// Used by PcbLib footprint Data streams which have a pattern name before
/// the packed binary records.
pub(crate) fn read_pascal_prefix(data: &[u8]) -> Result<(String, &[u8])> {
    let mut reader = BinaryReader::new(data);
    let name = reader.read_pascal_string()?;
    let consumed = reader.position();
    Ok((name, &data[consumed..]))
}

// ---------------------------------------------------------------------------
// BinaryWriter
// ---------------------------------------------------------------------------

pub(crate) struct BinaryWriter {
    buf: Vec<u8>,
}

impl BinaryWriter {
    pub(crate) fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub(crate) fn write_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub(crate) fn write_i8(&mut self, v: i8) {
        self.buf.push(v as u8);
    }

    pub(crate) fn write_u16_le(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn write_i16_le(&mut self, v: i16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn write_u32_le(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn write_i32_le(&mut self, v: i32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn write_u64_le(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn write_i64_le(&mut self, v: i64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn write_f32_le(&mut self, v: f32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn write_f64_le(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// Writes a boolean as 0x00 (false) or 0x01 (true).
    pub(crate) fn write_bool(&mut self, v: bool) {
        self.buf.push(if v { 0x01 } else { 0x00 });
    }

    /// Encodes an IEEE f64 as a 6-byte Borland Turbo Pascal Real48.
    pub(crate) fn write_real48(&mut self, v: f64) {
        if v == 0.0 {
            self.buf.extend_from_slice(&[0u8; 6]);
            return;
        }
        let bits = v.to_bits();
        let sign = ((bits >> 63) & 1) as u8;
        let ieee_exp = ((bits >> 52) & 0x7FF) as i64;
        let ieee_mant = bits & 0x000F_FFFF_FFFF_FFFF;
        let real48_exp = (ieee_exp - 1023 + 129) as u8;
        // Extract top 39 bits of the 52-bit IEEE mantissa into Real48 byte layout.
        // Byte 5 holds the top 7 mantissa bits (+ sign), bytes 4-1 hold the next 32 bits.
        let b1 = (ieee_mant >> 13) as u8;
        let b2 = (ieee_mant >> 21) as u8;
        let b3 = (ieee_mant >> 29) as u8;
        let b4 = (ieee_mant >> 37) as u8;
        let b5 = ((ieee_mant >> 45) as u8 & 0x7F) | (sign << 7);
        self.buf
            .extend_from_slice(&[real48_exp, b1, b2, b3, b4, b5]);
    }

    /// Writes a `Coord` as a 4-byte little-endian i32 via `to_internal()`.
    pub(crate) fn write_coord(&mut self, v: Coord) {
        self.write_i32_le(v.to_internal());
    }

    /// Writes a `CoordPoint` as two consecutive 4-byte little-endian i32 values (x, y).
    pub(crate) fn write_coord_point(&mut self, v: CoordPoint) {
        self.write_coord(v.x);
        self.write_coord(v.y);
    }

    /// Appends raw bytes to the buffer.
    pub(crate) fn write_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Writes an i32 LE byte count followed by Windows-1252-encoded string bytes.
    pub(crate) fn write_string_block(&mut self, s: &str) {
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(s);
        let len = encoded.len() as i32;
        self.write_i32_le(len);
        self.buf.extend_from_slice(&encoded);
    }

    /// Writes a u8 byte count followed by Windows-1252-encoded string bytes.
    ///
    /// Returns an error if the encoded string exceeds 255 bytes.
    pub(crate) fn write_pascal_string(&mut self, s: &str) -> crate::Result<()> {
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(s);
        if encoded.len() > 255 {
            return Err(crate::AltiumFormatError::InvalidParamValue {
                key: "pascal_string".to_owned(),
                detail: format!("string too long: {} bytes (max 255)", encoded.len()),
            });
        }
        self.write_u8(encoded.len() as u8);
        self.buf.extend_from_slice(&encoded);
        Ok(())
    }

    /// Writes a string as a fixed-size UTF-16LE buffer (`char_count` WideChars = `char_count * 2` bytes).
    ///
    /// The string is null-terminated and zero-padded to fill the buffer.
    /// Returns an error if the string exceeds `char_count - 1` characters.
    pub(crate) fn write_wide_string_fixed(&mut self, s: &str, char_count: usize) -> Result<()> {
        let byte_count = char_count * 2;
        let chars: Vec<u16> = s.encode_utf16().collect();
        if chars.len() >= char_count {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "WideString".to_owned(),
                detail: format!(
                    "string too long: {} chars (max {})",
                    chars.len(),
                    char_count - 1
                ),
            });
        }
        for &c in &chars {
            self.buf.extend_from_slice(&c.to_le_bytes());
        }
        // Zero-pad remainder (null terminator + padding).
        let written = chars.len() * 2;
        self.buf.resize(self.buf.len() + (byte_count - written), 0);
        Ok(())
    }

    /// Writes all elements of a fixed-size array using the provided closure.
    pub(crate) fn write_array<T, const N: usize>(
        &mut self,
        arr: &[T; N],
        mut write_one: impl FnMut(&mut Self, &T),
    ) {
        for item in arr.iter() {
            write_one(self, item);
        }
    }

    /// Consumes the writer and returns the accumulated bytes.
    pub(crate) fn finish(self) -> Vec<u8> {
        self.buf
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_write_u8_roundtrip() {
        let mut w = BinaryWriter::new();
        w.write_u8(0);
        w.write_u8(127);
        w.write_u8(255);
        let data = w.finish();
        let mut r = BinaryReader::new(&data);
        assert_eq!(r.read_u8().unwrap(), 0);
        assert_eq!(r.read_u8().unwrap(), 127);
        assert_eq!(r.read_u8().unwrap(), 255);
        r.assert_exhausted().unwrap();
    }

    #[test]
    fn read_write_i32_roundtrip() {
        let mut w = BinaryWriter::new();
        w.write_i32_le(0);
        w.write_i32_le(-1);
        w.write_i32_le(i32::MAX);
        w.write_i32_le(i32::MIN);
        let data = w.finish();
        let mut r = BinaryReader::new(&data);
        assert_eq!(r.read_i32_le().unwrap(), 0);
        assert_eq!(r.read_i32_le().unwrap(), -1);
        assert_eq!(r.read_i32_le().unwrap(), i32::MAX);
        assert_eq!(r.read_i32_le().unwrap(), i32::MIN);
        r.assert_exhausted().unwrap();
    }

    #[test]
    fn read_write_f64_roundtrip() {
        let mut w = BinaryWriter::new();
        w.write_f64_le(3.14159);
        w.write_f64_le(-0.0);
        w.write_f64_le(f64::INFINITY);
        let data = w.finish();
        let mut r = BinaryReader::new(&data);
        assert_eq!(r.read_f64_le().unwrap(), 3.14159);
        assert!(r.read_f64_le().unwrap().is_sign_negative());
        assert_eq!(r.read_f64_le().unwrap(), f64::INFINITY);
        r.assert_exhausted().unwrap();
    }

    #[test]
    fn read_bool_nonzero_is_true() {
        let mut r = BinaryReader::new(&[0x00, 0x01, 0xFF]);
        assert!(!r.read_bool().unwrap());
        assert!(r.read_bool().unwrap());
        assert!(r.read_bool().unwrap()); // 0xFF is also true
    }

    #[test]
    fn read_coord_from_known_bytes() {
        // 10000 in i32 LE = 1 mil
        let bytes = 10000i32.to_le_bytes();
        let mut r = BinaryReader::new(&bytes);
        let c = r.read_coord().unwrap();
        assert_eq!(c.to_internal(), 10000);
    }

    #[test]
    fn read_coord_point() {
        let mut w = BinaryWriter::new();
        w.write_i32_le(100);
        w.write_i32_le(200);
        let data = w.finish();
        let mut r = BinaryReader::new(&data);
        let p = r.read_coord_point().unwrap();
        assert_eq!(p.x.to_internal(), 100);
        assert_eq!(p.y.to_internal(), 200);
    }

    #[test]
    fn read_string_block_ascii() {
        let text = "Hello";
        let mut w = BinaryWriter::new();
        w.write_string_block(text);
        let data = w.finish();
        let mut r = BinaryReader::new(&data);
        assert_eq!(r.read_string_block().unwrap(), "Hello");
        r.assert_exhausted().unwrap();
    }

    #[test]
    fn read_pascal_string() {
        let mut w = BinaryWriter::new();
        w.write_pascal_string("AB").unwrap();
        let data = w.finish();
        let mut r = BinaryReader::new(&data);
        assert_eq!(r.read_pascal_string().unwrap(), "AB");
        r.assert_exhausted().unwrap();
    }

    #[test]
    fn sub_reader_advances_parent() {
        let data = [1u8, 2, 3, 4, 5, 6];
        let mut r = BinaryReader::new(&data);
        r.read_u8().unwrap(); // consume 1 byte
        let mut sub = r.sub_reader(3).unwrap();
        assert_eq!(sub.read_u8().unwrap(), 2);
        assert_eq!(sub.read_u8().unwrap(), 3);
        assert_eq!(sub.read_u8().unwrap(), 4);
        sub.assert_exhausted().unwrap();
        // Parent should be at position 4
        assert_eq!(r.position(), 4);
        assert_eq!(r.read_u8().unwrap(), 5);
    }

    #[test]
    fn assert_exhausted_on_empty() {
        let r = BinaryReader::new(&[]);
        r.assert_exhausted().unwrap();
    }

    #[test]
    fn assert_exhausted_with_remaining_returns_error() {
        let r = BinaryReader::new(&[1, 2, 3]);
        let err = r.assert_exhausted().unwrap_err();
        assert!(matches!(
            err,
            AltiumFormatError::UnexpectedTrailingData {
                offset: 0,
                count: 3
            }
        ));
    }

    #[test]
    fn read_past_end_returns_error() {
        let mut r = BinaryReader::new(&[]);
        let err = r.read_u8().unwrap_err();
        assert!(matches!(
            err,
            AltiumFormatError::BinaryReadPastEnd {
                offset: 0,
                needed: 1,
                available: 0
            }
        ));
    }

    #[test]
    fn read_real48_zero() {
        let mut r = BinaryReader::new(&[0, 0, 0, 0, 0, 0]);
        assert_eq!(r.read_real48().unwrap(), 0.0);
    }

    #[test]
    fn read_write_real48_roundtrip() {
        for &v in &[1.0, -1.0, 0.5, 100.0, 0.001, 360.0, -270.5] {
            let mut w = BinaryWriter::new();
            w.write_real48(v);
            let data = w.finish();
            assert_eq!(data.len(), 6, "real48 should be 6 bytes");
            let mut r = BinaryReader::new(&data);
            let read_back = r.read_real48().unwrap();
            // Real48 has ~11 decimal digits of precision (40-bit mantissa)
            assert!(
                (read_back - v).abs() < 1e-10 * v.abs().max(1.0),
                "roundtrip failed for {v}: got {read_back}"
            );
        }
    }

    #[test]
    fn read_array_fixed_size() {
        let mut w = BinaryWriter::new();
        w.write_u8(10);
        w.write_u8(20);
        w.write_u8(30);
        let data = w.finish();
        let mut r = BinaryReader::new(&data);
        let arr: [u8; 3] = r.read_array(|r| r.read_u8()).unwrap();
        assert_eq!(arr, [10, 20, 30]);
        r.assert_exhausted().unwrap();
    }

    #[test]
    fn write_coord_roundtrip() {
        let c = Coord::from_internal(12345);
        let mut w = BinaryWriter::new();
        w.write_coord(c);
        let data = w.finish();
        let mut r = BinaryReader::new(&data);
        assert_eq!(r.read_coord().unwrap().to_internal(), 12345);
    }

    #[test]
    fn write_coord_point_roundtrip() {
        let p = CoordPoint::new(Coord::from_internal(100), Coord::from_internal(200));
        let mut w = BinaryWriter::new();
        w.write_coord_point(p);
        let data = w.finish();
        let mut r = BinaryReader::new(&data);
        let p2 = r.read_coord_point().unwrap();
        assert_eq!(p2, p);
    }

    #[test]
    fn skip_advances_position() {
        let mut r = BinaryReader::new(&[1, 2, 3, 4, 5]);
        r.skip(3).unwrap();
        assert_eq!(r.position(), 3);
        assert_eq!(r.remaining(), 2);
        assert_eq!(r.read_u8().unwrap(), 4);
    }

    #[test]
    fn read_pascal_prefix_splits_buffer() {
        let mut w = BinaryWriter::new();
        w.write_pascal_string("MyFootprint").unwrap();
        w.write_bytes(&[0xDE, 0xAD, 0xBE, 0xEF]); // remaining data
        let data = w.finish();
        let (name, remaining) = read_pascal_prefix(&data).unwrap();
        assert_eq!(name, "MyFootprint");
        assert_eq!(remaining, &[0xDE, 0xAD, 0xBE, 0xEF]);
    }
}

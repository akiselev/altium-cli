//! Base writer utilities for Altium CFB files.
//!
//! Provides common writing operations for compound file binary format.

use std::io::Write;

#[cfg(test)]
use std::io::Cursor;

use byteorder::{LittleEndian, WriteBytesExt};
use encoding_rs::WINDOWS_1252;
use flate2::Compression;
use flate2::write::ZlibEncoder;

use crate::error::Result;
use crate::types::{Coord, CoordPoint, ParameterCollection};

/// Writes a size-prefixed block of data.
///
/// The block starts with an i32 size header (with optional flags in high byte),
/// followed by that many bytes.
pub fn write_block<W: Write>(writer: &mut W, data: &[u8], flags: u8) -> Result<()> {
    let size = data.len() as i32;
    let header = ((flags as i32) << 24) | size;
    writer.write_i32::<LittleEndian>(header)?;
    if !data.is_empty() {
        writer.write_all(data)?;
    }
    Ok(())
}

/// Writes a size-prefixed block using a serializer function.
///
/// The serializer writes to an internal buffer, then the block is written
/// with the computed size.
pub fn write_block_with<W: Write, F>(writer: &mut W, serializer: F, flags: u8) -> Result<()>
where
    F: FnOnce(&mut Vec<u8>) -> Result<()>,
{
    let mut buffer = Vec::new();
    serializer(&mut buffer)?;
    write_block(writer, &buffer, flags)
}

/// Compresses data using zlib format.
///
/// Includes the 2-byte zlib header (0x78 0x9C for default compression).
pub fn compress_zlib(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    let compressed = encoder.finish()?;
    Ok(compressed)
}

/// Encodes a string to Windows-1252 bytes.
pub fn encode_windows_1252(s: &str) -> Vec<u8> {
    let (bytes, _, _) = WINDOWS_1252.encode(s);
    bytes.into_owned()
}

/// Writes a raw string (no length prefix, no null terminator).
pub fn write_raw_string<W: Write>(writer: &mut W, s: &str) -> Result<()> {
    let bytes = encode_windows_1252(s);
    writer.write_all(&bytes)?;
    Ok(())
}

/// Writes a C-style null-terminated string.
pub fn write_c_string<W: Write>(writer: &mut W, s: &str) -> Result<()> {
    write_raw_string(writer, s)?;
    writer.write_u8(0)?;
    Ok(())
}

/// Writes a Pascal-style string (i32 size prefix, null-terminated content).
pub fn write_pascal_string<W: Write>(writer: &mut W, s: &str) -> Result<()> {
    let mut buffer = Vec::new();
    write_c_string(&mut buffer, s)?;
    write_block(writer, &buffer, 0)
}

/// Writes a Pascal short string (byte size prefix, no null terminator).
pub fn write_pascal_short_string<W: Write>(writer: &mut W, s: &str) -> Result<()> {
    let bytes = encode_windows_1252(s);
    if bytes.len() > 255 {
        // Truncate to 255 bytes max
        writer.write_u8(255)?;
        writer.write_all(&bytes[..255])?;
    } else {
        writer.write_u8(bytes.len() as u8)?;
        writer.write_all(&bytes)?;
    }
    Ok(())
}

/// Writes a fixed-length UTF-16 font name (32 bytes, null-terminated UTF-16).
///
/// Format: 32 bytes (16 UTF-16 LE code units)
/// - Strings up to 15 characters: written with null terminator, then zero-padded
/// - Strings exactly 16 characters: written without null terminator (uses all 32 bytes)
/// - Strings longer than 16 characters: truncated to 16 characters, no null terminator
///
/// This ensures round-trip integrity for font names of any length found in files.
pub fn write_font_name<W: Write>(writer: &mut W, s: &str) -> Result<()> {
    // Convert to UTF-16 and take at most 16 code units (to fit in 32 bytes)
    let utf16: Vec<u16> = s.encode_utf16().take(16).collect();

    // Write UTF-16 code units
    for &code in &utf16 {
        writer.write_u16::<LittleEndian>(code)?;
    }

    // Calculate bytes written
    let written = utf16.len() * 2;

    // If less than 16 code units, write null terminator and pad
    if utf16.len() < 16 {
        writer.write_u16::<LittleEndian>(0)?; // Null terminator
        let with_null = written + 2;

        // Pad remaining bytes to reach 32 total
        for _ in with_null..32 {
            writer.write_u8(0)?;
        }
    }
    // If exactly 16 code units, no padding needed (uses all 32 bytes)

    Ok(())
}

/// Writes a string block (i32 size prefix, then byte size prefix for content).
pub fn write_string_block<W: Write>(writer: &mut W, s: &str) -> Result<()> {
    let mut buffer = Vec::new();
    write_pascal_short_string(&mut buffer, s)?;
    write_block(writer, &buffer, 0)
}

/// Writes parameters as a C-string (null-terminated).
pub fn write_parameters<W: Write>(writer: &mut W, params: &ParameterCollection) -> Result<()> {
    let s = params.to_string();
    write_c_string(writer, &s)
}

/// Writes parameters as a raw string (no null terminator).
pub fn write_parameters_raw<W: Write>(writer: &mut W, params: &ParameterCollection) -> Result<()> {
    let s = params.to_string();
    write_raw_string(writer, &s)
}

/// Writes parameters in a block (i32 size prefix, null-terminated content).
pub fn write_parameters_block<W: Write>(
    writer: &mut W,
    params: &ParameterCollection,
) -> Result<()> {
    let mut buffer = Vec::new();
    write_parameters(&mut buffer, params)?;
    write_block(writer, &buffer, 0)
}

/// Writes a CoordPoint (two i32 values for x, y).
pub fn write_coord_point<W: Write>(writer: &mut W, point: CoordPoint) -> Result<()> {
    writer.write_i32::<LittleEndian>(point.x.to_raw())?;
    writer.write_i32::<LittleEndian>(point.y.to_raw())?;
    Ok(())
}

/// Writes the header stream (u32 record count).
pub fn write_header<W: Write>(writer: &mut W, record_count: u32) -> Result<()> {
    writer.write_u32::<LittleEndian>(record_count)?;
    Ok(())
}

/// Writes compressed storage: (id, data) pair with zlib compression.
///
/// Format: block containing [0xD0 tag, Pascal short string id, compressed data block]
pub fn write_compressed_storage<W: Write>(writer: &mut W, id: &str, data: &[u8]) -> Result<()> {
    write_block_with(
        writer,
        |buffer| {
            // Write 0xD0 tag
            buffer.write_u8(0xD0)?;

            // Write ID as Pascal short string
            write_pascal_short_string(buffer, id)?;

            // Compress and write data
            let compressed = compress_zlib(data)?;
            write_block(buffer, &compressed, 0)?;

            Ok(())
        },
        0x01,
    ) // Compressed blocks have 0x01 flag
}

/// Writes compressed storage with a serializer function.
pub fn write_compressed_storage_with<W: Write, F>(
    writer: &mut W,
    id: &str,
    serializer: F,
) -> Result<()>
where
    F: FnOnce(&mut Vec<u8>) -> Result<()>,
{
    let mut data = Vec::new();
    serializer(&mut data)?;
    write_compressed_storage(writer, id, &data)
}

/// Extension trait for convenient writing operations.
pub trait WriteExt: Write {
    /// Writes a Coord value (i32).
    fn write_coord(&mut self, value: Coord) -> Result<()>
    where
        Self: Sized,
    {
        self.write_i32::<LittleEndian>(value.to_raw())?;
        Ok(())
    }

    /// Writes a boolean byte.
    fn write_bool8(&mut self, value: bool) -> Result<()>
    where
        Self: Sized,
    {
        self.write_u8(if value { 1 } else { 0 })?;
        Ok(())
    }
}

impl<W: Write> WriteExt for W {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_block() {
        let mut buffer = Vec::new();
        write_block(&mut buffer, b"hello", 0).unwrap();

        // 4-byte size (5) + 5 bytes of data
        assert_eq!(buffer, [5, 0, 0, 0, b'h', b'e', b'l', b'l', b'o']);
    }

    #[test]
    fn test_write_block_with_flags() {
        let mut buffer = Vec::new();
        write_block(&mut buffer, b"test", 0x01).unwrap();

        // Flag in high byte: 0x01000004
        assert_eq!(buffer[0], 4);
        assert_eq!(buffer[3], 0x01);
    }

    #[test]
    fn test_write_pascal_short_string() {
        let mut buffer = Vec::new();
        write_pascal_short_string(&mut buffer, "hello").unwrap();

        // 1-byte size (5) + "hello"
        assert_eq!(buffer, [5, b'h', b'e', b'l', b'l', b'o']);
    }

    #[test]
    fn test_write_c_string() {
        let mut buffer = Vec::new();
        write_c_string(&mut buffer, "test").unwrap();

        assert_eq!(buffer, [b't', b'e', b's', b't', 0]);
    }

    #[test]
    fn test_write_coord_point() {
        let mut buffer = Vec::new();
        let point = CoordPoint::from_raw(65536, 131072);
        write_coord_point(&mut buffer, point).unwrap();

        // Two i32 values in little-endian
        assert_eq!(buffer.len(), 8);
        let mut cursor = Cursor::new(&buffer);
        use byteorder::ReadBytesExt;
        assert_eq!(cursor.read_i32::<LittleEndian>().unwrap(), 65536);
        assert_eq!(cursor.read_i32::<LittleEndian>().unwrap(), 131072);
    }

    #[test]
    fn test_encode_windows_1252() {
        let bytes = encode_windows_1252("Hello World");
        assert_eq!(bytes, b"Hello World");
    }

    #[test]
    fn test_write_font_name_empty() {
        let mut buffer = Vec::new();
        write_font_name(&mut buffer, "").unwrap();

        // Should be: null terminator (2 bytes) + 30 bytes of padding = 32 bytes
        assert_eq!(buffer.len(), 32);
        assert_eq!(&buffer[0..2], &[0, 0]); // Null terminator
        assert!(buffer[2..].iter().all(|&b| b == 0)); // All padding
    }

    #[test]
    fn test_write_font_name_short() {
        let mut buffer = Vec::new();
        write_font_name(&mut buffer, "Arial").unwrap();

        // Should be: "Arial" (5 chars * 2 bytes = 10 bytes) + null (2) + padding (20) = 32
        assert_eq!(buffer.len(), 32);

        // Check the string content
        let expected_utf16: Vec<u16> = "Arial".encode_utf16().collect();
        for (i, &code) in expected_utf16.iter().enumerate() {
            let offset = i * 2;
            let actual = u16::from_le_bytes([buffer[offset], buffer[offset + 1]]);
            assert_eq!(actual, code);
        }

        // Check null terminator
        let null_offset = 5 * 2;
        assert_eq!(buffer[null_offset], 0);
        assert_eq!(buffer[null_offset + 1], 0);

        // Check padding
        assert!(buffer[null_offset + 2..].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_write_font_name_15_chars() {
        let mut buffer = Vec::new();
        let name = "Times New Roman"; // Exactly 15 characters
        write_font_name(&mut buffer, name).unwrap();

        // Should be: 15 chars * 2 bytes = 30 bytes + null (2) = 32 bytes
        assert_eq!(buffer.len(), 32);

        // Verify content
        let utf16: Vec<u16> = name.encode_utf16().collect();
        assert_eq!(utf16.len(), 15);

        for (i, &code) in utf16.iter().enumerate() {
            let offset = i * 2;
            let actual = u16::from_le_bytes([buffer[offset], buffer[offset + 1]]);
            assert_eq!(actual, code);
        }

        // Verify null terminator at position 30
        assert_eq!(buffer[30], 0);
        assert_eq!(buffer[31], 0);
    }

    #[test]
    fn test_write_font_name_16_chars() {
        let mut buffer = Vec::new();
        let name = "1234567890ABCDEF"; // Exactly 16 characters
        write_font_name(&mut buffer, name).unwrap();

        // Should use all 32 bytes (16 chars * 2 bytes), no null terminator
        assert_eq!(buffer.len(), 32);

        // Verify content
        let utf16: Vec<u16> = name.encode_utf16().collect();
        assert_eq!(utf16.len(), 16);

        for (i, &code) in utf16.iter().enumerate() {
            let offset = i * 2;
            let actual = u16::from_le_bytes([buffer[offset], buffer[offset + 1]]);
            assert_eq!(actual, code);
        }

        // No null terminator or padding since all 32 bytes are used
    }

    #[test]
    fn test_write_font_name_too_long() {
        let mut buffer = Vec::new();
        let name = "ThisIsAVeryLongFontNameThatExceeds16Characters"; // > 16 characters
        write_font_name(&mut buffer, name).unwrap();

        // Should be truncated to 16 characters (32 bytes), no null
        assert_eq!(buffer.len(), 32);

        // Verify only first 16 characters are written
        let utf16: Vec<u16> = name.encode_utf16().take(16).collect();
        assert_eq!(utf16.len(), 16);

        for (i, &code) in utf16.iter().enumerate() {
            let offset = i * 2;
            let actual = u16::from_le_bytes([buffer[offset], buffer[offset + 1]]);
            assert_eq!(actual, code);
        }
    }

    #[test]
    fn test_write_font_name_unicode() {
        let mut buffer = Vec::new();
        write_font_name(&mut buffer, "微软雅黑").unwrap(); // Chinese font name

        assert_eq!(buffer.len(), 32);

        // Verify UTF-16 encoding
        let utf16: Vec<u16> = "微软雅黑".encode_utf16().collect();
        for (i, &code) in utf16.iter().enumerate() {
            let offset = i * 2;
            let actual = u16::from_le_bytes([buffer[offset], buffer[offset + 1]]);
            assert_eq!(actual, code);
        }

        // Should have null terminator since < 16 chars
        let null_offset = utf16.len() * 2;
        assert_eq!(buffer[null_offset], 0);
        assert_eq!(buffer[null_offset + 1], 0);
    }
}

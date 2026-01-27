//! Base reader utilities for Altium CFB files.
//!
//! Provides common reading operations for compound file binary format.

use byteorder::{LittleEndian, ReadBytesExt};
use encoding_rs::WINDOWS_1252;
use flate2::read::ZlibDecoder;
use std::io::{Cursor, Read, Seek, SeekFrom};

use crate::error::{AltiumError, Result};
use crate::format::{CFB_COMPRESSED_TAG, SIZE_FLAG_MASK};
use crate::types::{Coord, CoordPoint, ParameterCollection};

/// Reads a size-prefixed block of data.
///
/// The block starts with an i32 size header, followed by that many bytes.
pub fn read_block<R: Read>(reader: &mut R) -> Result<Vec<u8>> {
    let size = reader.read_i32::<LittleEndian>()?;
    // High byte contains flags; extract 24-bit size
    let clean_size = (size & SIZE_FLAG_MASK as i32) as usize;

    if clean_size == 0 {
        return Ok(Vec::new());
    }

    let mut buffer = vec![0u8; clean_size];
    reader.read_exact(&mut buffer)?;
    Ok(buffer)
}

/// Reads a size-prefixed block and interprets it using a callback.
pub fn read_block_with<R: Read, T, F>(reader: &mut R, interpreter: F) -> Result<T>
where
    F: FnOnce(&[u8]) -> Result<T>,
{
    let data = read_block(reader)?;
    interpreter(&data)
}

/// Decompresses zlib-compressed data.
///
/// Skips the 2-byte zlib header before decompressing.
pub fn decompress_zlib(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < 2 {
        return Err(AltiumError::Decompression(
            "Data too short for zlib".to_string(),
        ));
    }

    // Skip 2-byte zlib header
    let mut decoder = ZlibDecoder::new(&data[2..]);
    let mut output = Vec::new();
    decoder
        .read_to_end(&mut output)
        .map_err(|e| AltiumError::Decompression(format!("Zlib decompression failed: {}", e)))?;
    Ok(output)
}

/// Decodes a byte slice from Windows-1252 encoding to a String.
pub fn decode_windows_1252(data: &[u8]) -> String {
    let (cow, _, _) = WINDOWS_1252.decode(data);
    cow.into_owned()
}

/// Reads a raw string of known length (no length prefix, no null terminator).
pub fn read_raw_string<R: Read>(reader: &mut R, size: usize) -> Result<String> {
    if size == 0 {
        return Ok(String::new());
    }
    let mut buffer = vec![0u8; size];
    reader.read_exact(&mut buffer)?;
    Ok(decode_windows_1252(&buffer))
}

/// Reads a C-style null-terminated string of known buffer size.
///
/// The buffer is `size` bytes, but the string ends at the first null byte.
pub fn read_c_string<R: Read>(reader: &mut R, size: usize) -> Result<String> {
    if size == 0 {
        return Ok(String::new());
    }
    let mut buffer = vec![0u8; size];
    reader.read_exact(&mut buffer)?;

    // Find null terminator
    let end = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
    Ok(decode_windows_1252(&buffer[..end]))
}

/// Reads a Pascal-style string (i32 size prefix, null-terminated content).
pub fn read_pascal_string<R: Read>(reader: &mut R) -> Result<String> {
    let data = read_block(reader)?;
    if data.is_empty() {
        return Ok(String::new());
    }
    // Remove null terminator if present
    let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    Ok(decode_windows_1252(&data[..end]))
}

/// Reads a Pascal short string (byte size prefix, no null terminator).
pub fn read_pascal_short_string<R: Read>(reader: &mut R) -> Result<String> {
    let size = reader.read_u8()? as usize;
    read_raw_string(reader, size)
}

/// Reads a fixed-length UTF-16 font name (32 bytes, null-terminated UTF-16).
pub fn read_font_name<R: Read + Seek>(reader: &mut R) -> Result<String> {
    let start_pos = reader.stream_position()?;
    let mut chars = Vec::new();

    for _ in 0..16 {
        let code = reader.read_u16::<LittleEndian>()?;
        if code == 0 {
            break;
        }
        chars.push(code);
    }

    // Always read exactly 32 bytes total
    reader.seek(SeekFrom::Start(start_pos + 32))?;

    String::from_utf16(&chars)
        .map_err(|e| AltiumError::Encoding(format!("Invalid UTF-16 font name: {}", e)))
}

/// Reads a string block (i32 size prefix, then byte size prefix for content).
pub fn read_string_block<R: Read>(reader: &mut R) -> Result<String> {
    let data = read_block(reader)?;
    if data.is_empty() {
        return Ok(String::new());
    }
    let mut cursor = Cursor::new(data);
    read_pascal_short_string(&mut cursor)
}

/// Reads parameters from a sized block.
///
/// The block contains a null-terminated parameter string.
pub fn read_parameters<R: Read>(
    reader: &mut R,
    size: usize,
    raw: bool,
) -> Result<ParameterCollection> {
    let data = if raw {
        read_raw_string(reader, size)?
    } else {
        read_c_string(reader, size)?
    };
    Ok(ParameterCollection::from_string(&data))
}

/// Reads parameters from a block (i32 size prefix).
pub fn read_parameters_block<R: Read>(reader: &mut R) -> Result<ParameterCollection> {
    let data = read_block(reader)?;
    if data.is_empty() {
        return Ok(ParameterCollection::new());
    }
    // Remove null terminator
    let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    let s = decode_windows_1252(&data[..end]);
    Ok(ParameterCollection::from_string(&s))
}

/// Reads a CoordPoint (two i32 values for x, y).
pub fn read_coord_point<R: Read>(reader: &mut R) -> Result<CoordPoint> {
    let x = reader.read_i32::<LittleEndian>()?;
    let y = reader.read_i32::<LittleEndian>()?;
    Ok(CoordPoint::from_raw(x, y))
}

/// Reads the header stream (u32 data size).
pub fn read_header<R: Read>(reader: &mut R) -> Result<u32> {
    Ok(reader.read_u32::<LittleEndian>()?)
}

/// Reads compressed storage: (id, data) pair with zlib compression.
pub fn read_compressed_storage<R: Read>(reader: &mut R) -> Result<(String, Vec<u8>)> {
    let block_data = read_block(reader)?;
    if block_data.is_empty() {
        return Ok((String::new(), Vec::new()));
    }

    let mut cursor = Cursor::new(block_data);

    // Altium format uses 0xD0 tag to mark zlib-compressed streams
    let tag = cursor.read_u8()?;
    if tag != CFB_COMPRESSED_TAG {
        return Err(AltiumError::Parse(
            "Expected 0xD0 tag in compressed storage".to_string(),
        ));
    }

    // Read ID as Pascal short string
    let id = read_pascal_short_string(&mut cursor)?;

    // Read compressed data block
    let compressed = read_block(&mut cursor)?;

    // Decompress
    let data = decompress_zlib(&compressed)?;

    Ok((id, data))
}

/// Extension trait for convenient reading operations.
pub trait ReadExt: Read {
    /// Reads a Coord value (i32).
    fn read_coord(&mut self) -> Result<Coord>
    where
        Self: Sized,
    {
        let value = self.read_i32::<LittleEndian>()?;
        Ok(Coord::from_raw(value))
    }

    /// Reads a boolean byte.
    fn read_bool8(&mut self) -> Result<bool>
    where
        Self: Sized,
    {
        let value = self.read_u8()?;
        Ok(value != 0)
    }
}

impl<R: Read> ReadExt for R {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_block() {
        // 4-byte size (5) + 5 bytes of data = "hello"
        let data = [5, 0, 0, 0, b'h', b'e', b'l', b'l', b'o'];
        let mut cursor = Cursor::new(&data);

        let result = read_block(&mut cursor).unwrap();
        assert_eq!(result, b"hello");
    }

    #[test]
    fn test_read_pascal_short_string() {
        // 1-byte size (5) + "hello"
        let data = [5, b'h', b'e', b'l', b'l', b'o'];
        let mut cursor = Cursor::new(&data);

        let result = read_pascal_short_string(&mut cursor).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_read_coord_point() {
        // Two i32 values
        let data = [0, 0, 1, 0, 0, 0, 2, 0]; // 65536, 131072
        let mut cursor = Cursor::new(&data);

        let point = read_coord_point(&mut cursor).unwrap();
        assert_eq!(point.x.to_raw(), 65536);
        assert_eq!(point.y.to_raw(), 131072);
    }

    #[test]
    fn test_decode_windows_1252() {
        let data = b"Hello World";
        let result = decode_windows_1252(data);
        assert_eq!(result, "Hello World");
    }
}

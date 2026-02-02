//! Stream name constants and framing helpers for PCB binary files.

use std::io::{self, Read};

// ── PCB CFB stream names ─────────────────────────────────────────────────

pub const STREAM_BOARD6: &str = "Board6";
pub const STREAM_CLASSES6: &str = "Classes6";
pub const STREAM_COMPONENTS6: &str = "Components6";
pub const STREAM_CONNECTIONS6: &str = "Connections6";
pub const STREAM_DIMENSIONS6: &str = "Dimensions6";
pub const STREAM_FILLS6: &str = "Fills6";
pub const STREAM_NETS6: &str = "Nets6";
pub const STREAM_PADS6: &str = "Pads6";
pub const STREAM_POLYGONS6: &str = "Polygons6";
pub const STREAM_REGIONS6: &str = "Regions6";
pub const STREAM_RULES6: &str = "Rules6";
pub const STREAM_TRACKS6: &str = "Tracks6";
pub const STREAM_TEXTS6: &str = "Texts6";
pub const STREAM_VIAS6: &str = "Vias6";
pub const STREAM_ARCS6: &str = "Arcs6";
pub const STREAM_COMPONENT_BODIES6: &str = "ComponentBodies6";
pub const STREAM_SHAPE_BASED_REGIONS6: &str = "ShapeBasedRegions6";
pub const STREAM_SHAPE_BASED_COMPONENT_BODIES6: &str = "ShapeBasedComponentBodies6";
pub const STREAM_WIDE_STRINGS6: &str = "WideStrings6";
pub const STREAM_EXTENDED_PRIMITIVE_INFO: &str = "ExtendedPrimitiveInformation";
pub const STREAM_UNIQUE_ID_PRIMITIVE_INFO: &str = "UniqueIDPrimitiveInformation";
pub const STREAM_PRIMITIVE_PARAMETERS: &str = "PrimitiveParameters";
pub const STREAM_EMBEDDED_FONTS6: &str = "EmbeddedFonts6";
pub const STREAM_MODELS: &str = "Models";
pub const STREAM_FILE_HEADER: &str = "FileHeader";
pub const STREAM_FILE_VERSION_INFO: &str = "FileVersionInfo";

/// Sub-stream suffix for section headers (record count).
pub const SUB_HEADER: &str = "Header";
/// Sub-stream suffix for section data (binary records).
pub const SUB_DATA: &str = "Data";

// ── Framing helpers ──────────────────────────────────────────────────────

/// Read a standard binary block: `u8 type + u32 len + data`.
///
/// Returns `(type_byte, data)`.
pub fn read_binary_block(r: &mut impl Read) -> io::Result<(u8, Vec<u8>)> {
    let mut type_buf = [0u8; 1];
    r.read_exact(&mut type_buf)?;
    let type_byte = type_buf[0];

    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;

    let mut data = vec![0u8; len];
    r.read_exact(&mut data)?;

    Ok((type_byte, data))
}

/// Read a Connection block: `u32 len + data` (NO type byte).
///
/// Connections6 is the only primitive stream that omits the type byte.
pub fn read_connection_block(r: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;

    let mut data = vec![0u8; len];
    r.read_exact(&mut data)?;

    Ok(data)
}

/// Read a parametric block: `u32 len + |KEY=VALUE|` text.
///
/// Returns the raw text (with pipe delimiters, without null terminator).
pub fn read_parametric_block(r: &mut impl Read) -> io::Result<String> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;

    if len == 0 {
        return Ok(String::new());
    }

    let mut data = vec![0u8; len];
    r.read_exact(&mut data)?;

    // Trim trailing null
    if data.last() == Some(&0) {
        data.pop();
    }

    Ok(String::from_utf8_lossy(&data).into_owned())
}

/// Read a section header stream (just a u32 record count).
pub fn read_section_header(r: &mut impl Read) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

/// Write a standard binary block: `u8 type + u32 len + data`.
pub fn write_binary_block(w: &mut impl io::Write, type_byte: u8, data: &[u8]) -> io::Result<()> {
    w.write_all(&[type_byte])?;
    w.write_all(&(data.len() as u32).to_le_bytes())?;
    w.write_all(data)?;
    Ok(())
}

/// Write a connection block: `u32 len + data` (no type byte).
pub fn write_connection_block(w: &mut impl io::Write, data: &[u8]) -> io::Result<()> {
    w.write_all(&(data.len() as u32).to_le_bytes())?;
    w.write_all(data)?;
    Ok(())
}

/// Write a parametric block: `u32 len + text + null`.
pub fn write_parametric_block(w: &mut impl io::Write, text: &str) -> io::Result<()> {
    let bytes = text.as_bytes();
    // len includes null terminator
    w.write_all(&((bytes.len() + 1) as u32).to_le_bytes())?;
    w.write_all(bytes)?;
    w.write_all(&[0])?;
    Ok(())
}

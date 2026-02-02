//! PCB Region binary record (ID=11, hybrid binary+parametric).
//!
//! Framing: `u8 type(11)` + `u32 total_len` + data.
//!
//! Structure:
//! - 18-byte binary header (layer, flags, net, polygon, component, skip5, holecount, skip2)
//! - `u32 prop_len` + null-terminated `|KEY=VALUE|` parametric properties
//! - `u32 num_outline_vertices` + N × 16 bytes (f64 x, f64 y)
//! - For each hole: `u32 num_hole_vertices` + M × 16 bytes (f64 x, f64 y)

use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

use super::primitive::PcbCommonHeader;

/// A vertex in a PCB region (f64 coordinates in internal units).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegionVertex {
    pub x: f64,
    pub y: f64,
}

/// An extended vertex (37 bytes) used in ShapeBased variants.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtendedVertex {
    pub is_round: bool,
    pub x: i32,
    pub y: i32,
    pub cx: i32,
    pub cy: i32,
    pub radius: i32,
    pub angle1: f64,
    pub angle2: f64,
}

/// PCB Region record (hybrid binary+parametric).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbRegion {
    pub header: PcbCommonHeader,
    /// Bytes 9-13 (skip5, often 0xFF 0xFF 0xFF 0xFF 0x00).
    pub header_extra: [u8; 5],
    /// Number of holes (cutouts).
    pub hole_count: u16,
    /// Padding after hole_count (2 bytes).
    pub header_pad: [u8; 2],
    /// Parametric properties (`|KEY=VALUE|` text).
    pub properties: String,
    /// Outline vertices (standard f64 format).
    pub outline: Vec<RegionVertex>,
    /// Hole vertex lists.
    pub holes: Vec<Vec<RegionVertex>>,
}

impl PcbRegion {
    pub fn read_from(data: &[u8]) -> io::Result<Self> {
        if data.len() < 22 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "region data too short"));
        }

        let mut cursor = std::io::Cursor::new(data);
        let header = PcbCommonHeader::read_from(&mut cursor)?;

        let mut header_extra = [0u8; 5];
        cursor.read_exact(&mut header_extra)?;

        let mut hc_buf = [0u8; 2];
        cursor.read_exact(&mut hc_buf)?;
        let hole_count = u16::from_le_bytes(hc_buf);

        let mut header_pad = [0u8; 2];
        cursor.read_exact(&mut header_pad)?;

        // Parametric properties
        let mut prop_len_buf = [0u8; 4];
        cursor.read_exact(&mut prop_len_buf)?;
        let prop_len = u32::from_le_bytes(prop_len_buf) as usize;
        let mut prop_data = vec![0u8; prop_len];
        cursor.read_exact(&mut prop_data)?;
        let properties = String::from_utf8_lossy(&prop_data)
            .trim_end_matches('\0')
            .to_string();

        // Outline vertices
        let mut nv_buf = [0u8; 4];
        cursor.read_exact(&mut nv_buf)?;
        let num_outline = u32::from_le_bytes(nv_buf) as usize;
        let mut outline = Vec::with_capacity(num_outline);
        for _ in 0..num_outline {
            let mut vbuf = [0u8; 16];
            cursor.read_exact(&mut vbuf)?;
            outline.push(RegionVertex {
                x: f64::from_le_bytes(vbuf[0..8].try_into().unwrap()),
                y: f64::from_le_bytes(vbuf[8..16].try_into().unwrap()),
            });
        }

        // Hole vertices
        let mut holes = Vec::with_capacity(hole_count as usize);
        for _ in 0..hole_count {
            cursor.read_exact(&mut nv_buf)?;
            let num_hole = u32::from_le_bytes(nv_buf) as usize;
            let mut hole = Vec::with_capacity(num_hole);
            for _ in 0..num_hole {
                let mut vbuf = [0u8; 16];
                cursor.read_exact(&mut vbuf)?;
                hole.push(RegionVertex {
                    x: f64::from_le_bytes(vbuf[0..8].try_into().unwrap()),
                    y: f64::from_le_bytes(vbuf[8..16].try_into().unwrap()),
                });
            }
            holes.push(hole);
        }

        Ok(Self {
            header,
            header_extra,
            hole_count,
            header_pad,
            properties,
            outline,
            holes,
        })
    }

    pub fn write_to(&self, w: &mut impl Write) -> io::Result<()> {
        self.header.write_to(w)?;
        w.write_all(&self.header_extra)?;
        w.write_all(&self.hole_count.to_le_bytes())?;
        w.write_all(&self.header_pad)?;

        // Properties
        let prop_bytes = self.properties.as_bytes();
        // Include null terminator
        w.write_all(&((prop_bytes.len() + 1) as u32).to_le_bytes())?;
        w.write_all(prop_bytes)?;
        w.write_all(&[0])?;

        // Outline
        w.write_all(&(self.outline.len() as u32).to_le_bytes())?;
        for v in &self.outline {
            w.write_all(&v.x.to_le_bytes())?;
            w.write_all(&v.y.to_le_bytes())?;
        }

        // Holes
        for hole in &self.holes {
            w.write_all(&(hole.len() as u32).to_le_bytes())?;
            for v in hole {
                w.write_all(&v.x.to_le_bytes())?;
                w.write_all(&v.y.to_le_bytes())?;
            }
        }

        Ok(())
    }

    /// Parse the `|KEY=VALUE|` properties into a map.
    pub fn property_map(&self) -> std::collections::HashMap<String, String> {
        parse_parametric(&self.properties)
    }
}

/// Parse `|KEY=VALUE|KEY=VALUE|` text into a map.
pub fn parse_parametric(s: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for segment in s.split('|') {
        if segment.is_empty() {
            continue;
        }
        if let Some((k, v)) = segment.split_once('=') {
            map.insert(k.to_string(), v.to_string());
        }
    }
    map
}

/// Serialize a property map back to `|KEY=VALUE|` format.
pub fn serialize_parametric(map: &std::collections::HashMap<String, String>) -> String {
    let mut s = String::new();
    for (k, v) in map {
        s.push('|');
        s.push_str(k);
        s.push('=');
        s.push_str(v);
    }
    s
}

//! PCB Pad binary record (multi-block, ~912 bytes total).
//!
//! Framing: `u8 type(2)` + 6 subrecords, each with `u32 length` prefix (no type byte per sub).
//!
//! Subrecords (from Ghidra FUN_0187eb60):
//! 1. Pad name (WxString)
//! 2. Unknown string (often empty)
//! 3. Unknown string (often `|&|0`)
//! 4. Unknown string (often empty)
//! 5. Main pad data (172 bytes in AD26)
//! 6. Per-layer stack data (596/628/651 bytes)

use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

use super::coord::PcbCoord;
use super::primitive::PcbCommonHeader;

/// Read a length-prefixed string block (u32 len + data).
fn read_string_block(r: &mut impl Read) -> io::Result<String> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 {
        return Ok(String::new());
    }
    let mut data = vec![0u8; len];
    r.read_exact(&mut data)?;
    // Trim trailing null if present
    if data.last() == Some(&0) {
        data.pop();
    }
    Ok(String::from_utf8_lossy(&data).into_owned())
}

/// Write a length-prefixed string block (u32 len + data).
fn write_string_block(w: &mut impl Write, s: &str) -> io::Result<()> {
    let bytes = s.as_bytes();
    w.write_all(&(bytes.len() as u32).to_le_bytes())?;
    w.write_all(bytes)?;
    Ok(())
}

/// Read a length-prefixed binary block (u32 len + data), return raw bytes.
fn read_binary_block(r: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    let mut data = vec![0u8; len];
    r.read_exact(&mut data)?;
    Ok(data)
}

/// Pad main data (subrecord 5, 172 bytes in AD26).
///
/// Ghidra: FUN_0184ad40 + FUN_01858be0.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbPadCore {
    pub header: PcbCommonHeader,
    pub position_x: PcbCoord,
    pub position_y: PcbCoord,
    pub top_size_x: PcbCoord,
    pub top_size_y: PcbCoord,
    pub mid_size_x: PcbCoord,
    pub mid_size_y: PcbCoord,
    pub bot_size_x: PcbCoord,
    pub bot_size_y: PcbCoord,
    pub hole_size: PcbCoord,
    pub top_shape: u8,
    pub mid_shape: u8,
    pub bot_shape: u8,
    pub rotation: f64,
    pub is_plated: bool,
    pub pad_mode: u8,
    pub thermal_connect_mode: u8,
    pub thermal_relief_air_gap: PcbCoord,
    pub thermal_relief_spoke_count: u16,
    pub thermal_relief_spoke_width: PcbCoord,
    pub paste_mask_expansion: PcbCoord,
    pub solder_mask_expansion: PcbCoord,
    pub pad_layer_bitmask: u16,
    pub paste_mask_expansion_mode: u8,
    pub solder_mask_expansion_mode: u8,
    pub user_routed: bool,
    pub union_index: i32,
    pub layer_enum: i32,
    pub jumper_guid1: [u8; 16],
    pub jumper_guid2: [u8; 16],
    /// Raw bytes for fields we don't fully decode yet (offsets 61-171 minus known fields).
    /// Stored to preserve round-trip fidelity.
    #[serde(skip)]
    raw_core: Vec<u8>,
}

impl Default for PcbPadCore {
    fn default() -> Self {
        Self {
            header: PcbCommonHeader::default(),
            position_x: PcbCoord::ZERO,
            position_y: PcbCoord::ZERO,
            top_size_x: PcbCoord::ZERO,
            top_size_y: PcbCoord::ZERO,
            mid_size_x: PcbCoord::ZERO,
            mid_size_y: PcbCoord::ZERO,
            bot_size_x: PcbCoord::ZERO,
            bot_size_y: PcbCoord::ZERO,
            hole_size: PcbCoord::ZERO,
            top_shape: 0,
            mid_shape: 0,
            bot_shape: 0,
            rotation: 0.0,
            is_plated: true,
            pad_mode: 0,
            thermal_connect_mode: 0,
            thermal_relief_air_gap: PcbCoord::ZERO,
            thermal_relief_spoke_count: 4,
            thermal_relief_spoke_width: PcbCoord::ZERO,
            paste_mask_expansion: PcbCoord::ZERO,
            solder_mask_expansion: PcbCoord::ZERO,
            pad_layer_bitmask: 0,
            paste_mask_expansion_mode: 0,
            solder_mask_expansion_mode: 0,
            user_routed: false,
            union_index: 0,
            layer_enum: 0,
            jumper_guid1: [0; 16],
            jumper_guid2: [0; 16],
            raw_core: Vec::new(),
        }
    }
}

impl PcbPadCore {
    /// Parse from a raw subrecord 5 byte slice.
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        if data.len() < 110 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "pad core too short"));
        }

        let mut r = std::io::Cursor::new(data);
        let header = PcbCommonHeader::read_from(&mut r)?;

        let rd = |off: usize| -> i32 {
            i32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
        };
        let rd16 = |off: usize| -> u16 {
            u16::from_le_bytes([data[off], data[off + 1]])
        };

        Ok(Self {
            header,
            position_x: PcbCoord::from_raw(rd(13)),
            position_y: PcbCoord::from_raw(rd(17)),
            top_size_x: PcbCoord::from_raw(rd(21)),
            top_size_y: PcbCoord::from_raw(rd(25)),
            mid_size_x: PcbCoord::from_raw(rd(29)),
            mid_size_y: PcbCoord::from_raw(rd(33)),
            bot_size_x: PcbCoord::from_raw(rd(37)),
            bot_size_y: PcbCoord::from_raw(rd(41)),
            hole_size: PcbCoord::from_raw(rd(45)),
            top_shape: data[49],
            mid_shape: data[50],
            bot_shape: data[51],
            rotation: f64::from_le_bytes(data[52..60].try_into().unwrap()),
            is_plated: data[60] != 0,
            pad_mode: data[62],
            thermal_connect_mode: data[67],
            thermal_relief_air_gap: PcbCoord::from_raw(rd(68)),
            thermal_relief_spoke_count: rd16(72),
            thermal_relief_spoke_width: PcbCoord::from_raw(rd(74)),
            paste_mask_expansion: PcbCoord::from_raw(rd(86)),
            solder_mask_expansion: PcbCoord::from_raw(rd(90)),
            pad_layer_bitmask: rd16(94),
            paste_mask_expansion_mode: data[100],
            solder_mask_expansion_mode: data[101],
            user_routed: data.get(105).copied().unwrap_or(0) != 0,
            union_index: if data.len() > 109 { rd(106) } else { 0 },
            layer_enum: if data.len() > 117 { rd(114) } else { 0 },
            jumper_guid1: if data.len() >= 142 {
                data[126..142].try_into().unwrap()
            } else {
                [0; 16]
            },
            jumper_guid2: if data.len() >= 158 {
                data[142..158].try_into().unwrap()
            } else {
                [0; 16]
            },
            raw_core: data.to_vec(),
        })
    }

    /// Serialize back to bytes (returns the full subrecord 5 data).
    pub fn to_bytes(&self) -> Vec<u8> {
        // For round-trip, return raw if available and same size
        if !self.raw_core.is_empty() {
            return self.raw_core.clone();
        }
        // Otherwise build minimal 172-byte record
        let mut buf = vec![0u8; 172];
        let mut cursor = std::io::Cursor::new(&mut buf[..]);
        let _ = self.header.write_to(&mut cursor);
        // Write remaining fields at known offsets
        buf[13..17].copy_from_slice(&self.position_x.to_raw().to_le_bytes());
        buf[17..21].copy_from_slice(&self.position_y.to_raw().to_le_bytes());
        buf[21..25].copy_from_slice(&self.top_size_x.to_raw().to_le_bytes());
        buf[25..29].copy_from_slice(&self.top_size_y.to_raw().to_le_bytes());
        buf[29..33].copy_from_slice(&self.mid_size_x.to_raw().to_le_bytes());
        buf[33..37].copy_from_slice(&self.mid_size_y.to_raw().to_le_bytes());
        buf[37..41].copy_from_slice(&self.bot_size_x.to_raw().to_le_bytes());
        buf[41..45].copy_from_slice(&self.bot_size_y.to_raw().to_le_bytes());
        buf[45..49].copy_from_slice(&self.hole_size.to_raw().to_le_bytes());
        buf[49] = self.top_shape;
        buf[50] = self.mid_shape;
        buf[51] = self.bot_shape;
        buf[52..60].copy_from_slice(&self.rotation.to_le_bytes());
        buf[60] = self.is_plated as u8;
        buf[62] = self.pad_mode;
        buf[67] = self.thermal_connect_mode;
        buf[68..72].copy_from_slice(&self.thermal_relief_air_gap.to_raw().to_le_bytes());
        buf[72..74].copy_from_slice(&self.thermal_relief_spoke_count.to_le_bytes());
        buf[74..78].copy_from_slice(&self.thermal_relief_spoke_width.to_raw().to_le_bytes());
        buf[86..90].copy_from_slice(&self.paste_mask_expansion.to_raw().to_le_bytes());
        buf[90..94].copy_from_slice(&self.solder_mask_expansion.to_raw().to_le_bytes());
        buf[94..96].copy_from_slice(&self.pad_layer_bitmask.to_le_bytes());
        buf[100] = self.paste_mask_expansion_mode;
        buf[101] = self.solder_mask_expansion_mode;
        buf[105] = self.user_routed as u8;
        buf[106..110].copy_from_slice(&self.union_index.to_le_bytes());
        buf[114..118].copy_from_slice(&self.layer_enum.to_le_bytes());
        buf[126..142].copy_from_slice(&self.jumper_guid1);
        buf[142..158].copy_from_slice(&self.jumper_guid2);
        buf[171] = 1; // constant byte at offset 171
        buf
    }
}

/// Per-layer stack data (subrecord 6, 596/628/651 bytes).
///
/// From KiCad APAD6_SIZE_AND_SHAPE.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbPadStackData {
    /// Inner X sizes per layer (29 layers).
    pub inner_size_x: [i32; 29],
    /// Inner Y sizes per layer (29 layers).
    pub inner_size_y: [i32; 29],
    /// Inner shape per layer (29 layers).
    pub inner_shape: [u8; 29],
    /// Hole shape (0=Round, 1=Square, 2=Slot).
    pub hole_shape: u8,
    /// Slot size.
    pub slot_size: i32,
    /// Slot rotation.
    pub slot_rotation: f64,
    /// Hole offset X per layer (32 layers).
    pub hole_offset_x: [i32; 32],
    /// Hole offset Y per layer (32 layers).
    pub hole_offset_y: [i32; 32],
    /// Alt shape per layer (32 layers).
    pub alt_shape: [u8; 32],
    /// Corner radius % per layer (32 layers).
    pub corner_radius: [u8; 32],
    /// Raw bytes beyond offset 596 (for round-trip fidelity).
    pub extra: Vec<u8>,
}

impl Default for PcbPadStackData {
    fn default() -> Self {
        Self {
            inner_size_x: [0; 29],
            inner_size_y: [0; 29],
            inner_shape: [0; 29],
            hole_shape: 0,
            slot_size: 0,
            slot_rotation: 0.0,
            hole_offset_x: [0; 32],
            hole_offset_y: [0; 32],
            alt_shape: [0; 32],
            corner_radius: [0; 32],
            extra: Vec::new(),
        }
    }
}

impl PcbPadStackData {
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        if data.is_empty() {
            return Ok(Self::default());
        }
        if data.len() < 596 {
            // Older files may have shorter or no stack data
            return Ok(Self::default());
        }

        let rd = |off: usize| -> i32 {
            i32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
        };

        let mut inner_size_x = [0i32; 29];
        let mut inner_size_y = [0i32; 29];
        let mut inner_shape = [0u8; 29];
        for i in 0..29 {
            inner_size_x[i] = rd(i * 4);
            inner_size_y[i] = rd(116 + i * 4);
            inner_shape[i] = data[232 + i];
        }

        let hole_shape = data[262];
        let slot_size = rd(263);
        let slot_rotation = f64::from_le_bytes(data[267..275].try_into().unwrap());

        let mut hole_offset_x = [0i32; 32];
        let mut hole_offset_y = [0i32; 32];
        for i in 0..32 {
            hole_offset_x[i] = rd(275 + i * 4);
            hole_offset_y[i] = rd(403 + i * 4);
        }

        let mut alt_shape = [0u8; 32];
        alt_shape.copy_from_slice(&data[532..564]);

        let mut corner_radius = [0u8; 32];
        corner_radius.copy_from_slice(&data[564..596]);

        let extra = if data.len() > 596 {
            data[596..].to_vec()
        } else {
            Vec::new()
        };

        Ok(Self {
            inner_size_x,
            inner_size_y,
            inner_shape,
            hole_shape,
            slot_size,
            slot_rotation,
            hole_offset_x,
            hole_offset_y,
            alt_shape,
            corner_radius,
            extra,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = vec![0u8; 596 + self.extra.len()];

        for i in 0..29 {
            buf[i * 4..i * 4 + 4].copy_from_slice(&self.inner_size_x[i].to_le_bytes());
            buf[116 + i * 4..116 + i * 4 + 4].copy_from_slice(&self.inner_size_y[i].to_le_bytes());
            buf[232 + i] = self.inner_shape[i];
        }
        // byte 261 is skip
        buf[262] = self.hole_shape;
        buf[263..267].copy_from_slice(&self.slot_size.to_le_bytes());
        buf[267..275].copy_from_slice(&self.slot_rotation.to_le_bytes());

        for i in 0..32 {
            buf[275 + i * 4..275 + i * 4 + 4].copy_from_slice(&self.hole_offset_x[i].to_le_bytes());
            buf[403 + i * 4..403 + i * 4 + 4].copy_from_slice(&self.hole_offset_y[i].to_le_bytes());
        }
        // byte 531 is skip
        buf[532..564].copy_from_slice(&self.alt_shape);
        buf[564..596].copy_from_slice(&self.corner_radius);

        if !self.extra.is_empty() {
            buf[596..].copy_from_slice(&self.extra);
        }

        buf
    }
}

/// Complete PCB Pad record with all 6 subrecords.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbPad {
    /// Subrecord 1: pad name (e.g., "1", "A1").
    pub name: String,
    /// Subrecord 2: unknown string (often empty).
    pub unknown_str2: String,
    /// Subrecord 3: unknown string (often `|&|0`).
    pub unknown_str3: String,
    /// Subrecord 4: unknown string (often empty).
    pub unknown_str4: String,
    /// Subrecord 5: main pad data (172 bytes).
    pub core: PcbPadCore,
    /// Subrecord 6: per-layer stack data (596+ bytes).
    pub stack: PcbPadStackData,
}

impl PcbPad {
    /// Read all 6 subrecords from stream (after type byte has been consumed).
    pub fn read_from(r: &mut impl Read) -> io::Result<Self> {
        let name = read_string_block(r)?;
        let unknown_str2 = read_string_block(r)?;
        let unknown_str3 = read_string_block(r)?;
        let unknown_str4 = read_string_block(r)?;

        let core_data = read_binary_block(r)?;
        let core = PcbPadCore::from_bytes(&core_data)?;

        let stack_data = read_binary_block(r)?;
        let stack = PcbPadStackData::from_bytes(&stack_data)?;

        Ok(Self {
            name,
            unknown_str2,
            unknown_str3,
            unknown_str4,
            core,
            stack,
        })
    }

    /// Write all 6 subrecords to stream (caller writes type byte).
    pub fn write_to(&self, w: &mut impl Write) -> io::Result<()> {
        write_string_block(w, &self.name)?;
        write_string_block(w, &self.unknown_str2)?;
        write_string_block(w, &self.unknown_str3)?;
        write_string_block(w, &self.unknown_str4)?;

        let core_bytes = self.core.to_bytes();
        w.write_all(&(core_bytes.len() as u32).to_le_bytes())?;
        w.write_all(&core_bytes)?;

        let stack_bytes = self.stack.to_bytes();
        w.write_all(&(stack_bytes.len() as u32).to_le_bytes())?;
        w.write_all(&stack_bytes)?;

        Ok(())
    }
}

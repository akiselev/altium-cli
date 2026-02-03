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

/// Read a length-prefixed raw block (u32 len + data), preserving exact bytes.
fn read_raw_block(r: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    let mut data = vec![0u8; len];
    r.read_exact(&mut data)?;
    Ok(data)
}

/// Write a length-prefixed raw block (u32 len + data).
fn write_raw_block(w: &mut impl Write, data: &[u8]) -> io::Result<()> {
    w.write_all(&(data.len() as u32).to_le_bytes())?;
    w.write_all(data)?;
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
    ///
    /// Patches typed field values into stored data, preserving undecoded
    /// bytes at their original offsets and maintaining exact record size.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = if !self.raw_core.is_empty() {
            self.raw_core.clone()
        } else {
            let mut b = vec![0u8; 172];
            b[171] = 1; // constant byte at offset 171
            b
        };

        // Patch all typed fields into known offsets
        {
            let mut cursor = std::io::Cursor::new(&mut buf[..13]);
            let _ = self.header.write_to(&mut cursor);
        }
        if buf.len() >= 49 {
            buf[13..17].copy_from_slice(&self.position_x.to_raw().to_le_bytes());
            buf[17..21].copy_from_slice(&self.position_y.to_raw().to_le_bytes());
            buf[21..25].copy_from_slice(&self.top_size_x.to_raw().to_le_bytes());
            buf[25..29].copy_from_slice(&self.top_size_y.to_raw().to_le_bytes());
            buf[29..33].copy_from_slice(&self.mid_size_x.to_raw().to_le_bytes());
            buf[33..37].copy_from_slice(&self.mid_size_y.to_raw().to_le_bytes());
            buf[37..41].copy_from_slice(&self.bot_size_x.to_raw().to_le_bytes());
            buf[41..45].copy_from_slice(&self.bot_size_y.to_raw().to_le_bytes());
            buf[45..49].copy_from_slice(&self.hole_size.to_raw().to_le_bytes());
        }
        if buf.len() >= 60 {
            buf[49] = self.top_shape;
            buf[50] = self.mid_shape;
            buf[51] = self.bot_shape;
            buf[52..60].copy_from_slice(&self.rotation.to_le_bytes());
        }
        if buf.len() > 60 { buf[60] = self.is_plated as u8; }
        if buf.len() > 62 { buf[62] = self.pad_mode; }
        if buf.len() > 67 { buf[67] = self.thermal_connect_mode; }
        if buf.len() >= 78 {
            buf[68..72].copy_from_slice(&self.thermal_relief_air_gap.to_raw().to_le_bytes());
            buf[72..74].copy_from_slice(&self.thermal_relief_spoke_count.to_le_bytes());
            buf[74..78].copy_from_slice(&self.thermal_relief_spoke_width.to_raw().to_le_bytes());
        }
        if buf.len() >= 96 {
            buf[86..90].copy_from_slice(&self.paste_mask_expansion.to_raw().to_le_bytes());
            buf[90..94].copy_from_slice(&self.solder_mask_expansion.to_raw().to_le_bytes());
            buf[94..96].copy_from_slice(&self.pad_layer_bitmask.to_le_bytes());
        }
        if buf.len() > 101 {
            buf[100] = self.paste_mask_expansion_mode;
            buf[101] = self.solder_mask_expansion_mode;
        }
        if buf.len() > 105 { buf[105] = self.user_routed as u8; }
        if buf.len() >= 110 {
            buf[106..110].copy_from_slice(&self.union_index.to_le_bytes());
        }
        if buf.len() >= 118 {
            buf[114..118].copy_from_slice(&self.layer_enum.to_le_bytes());
        }
        if buf.len() >= 142 {
            buf[126..142].copy_from_slice(&self.jumper_guid1);
        }
        if buf.len() >= 158 {
            buf[142..158].copy_from_slice(&self.jumper_guid2);
        }
        buf
    }
}

/// Per-layer stack data (subrecord 6, variable size: 0/596/628/651 bytes).
///
/// From KiCad APAD6_SIZE_AND_SHAPE.
/// In PcbLib, this subrecord may be empty or shorter than 596 bytes.
/// Typed fields are extracted when data is large enough; all data
/// is stored to preserve the exact size and undecoded bytes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbPadStackData {
    /// Inner X sizes per layer (29 layers). Only valid when data.len() >= 596.
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
    /// Full subrecord data — typed fields are extracted from this.
    /// Preserved for exact-size serialization and undecoded byte fidelity.
    #[serde(skip)]
    data: Vec<u8>,
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
            data: Vec::new(),
        }
    }
}

impl PcbPadStackData {
    pub fn from_bytes(input: &[u8]) -> io::Result<Self> {
        let mut result = Self {
            data: input.to_vec(),
            ..Self::default()
        };

        if input.len() < 596 {
            return Ok(result);
        }

        let rd = |off: usize| -> i32 {
            i32::from_le_bytes([input[off], input[off + 1], input[off + 2], input[off + 3]])
        };

        for i in 0..29 {
            result.inner_size_x[i] = rd(i * 4);
            result.inner_size_y[i] = rd(116 + i * 4);
            result.inner_shape[i] = input[232 + i];
        }

        result.hole_shape = input[262];
        result.slot_size = rd(263);
        result.slot_rotation = f64::from_le_bytes(input[267..275].try_into().unwrap());

        for i in 0..32 {
            result.hole_offset_x[i] = rd(275 + i * 4);
            result.hole_offset_y[i] = rd(403 + i * 4);
        }

        result.alt_shape.copy_from_slice(&input[532..564]);
        result.corner_radius.copy_from_slice(&input[564..596]);

        Ok(result)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        if self.data.len() < 596 {
            // Short or empty stack data — return exact original bytes
            return self.data.clone();
        }

        // Patch typed fields into copy of full data
        let mut buf = self.data.clone();

        for i in 0..29 {
            buf[i * 4..i * 4 + 4].copy_from_slice(&self.inner_size_x[i].to_le_bytes());
            buf[116 + i * 4..116 + i * 4 + 4].copy_from_slice(&self.inner_size_y[i].to_le_bytes());
            buf[232 + i] = self.inner_shape[i];
        }
        buf[262] = self.hole_shape;
        buf[263..267].copy_from_slice(&self.slot_size.to_le_bytes());
        buf[267..275].copy_from_slice(&self.slot_rotation.to_le_bytes());

        for i in 0..32 {
            buf[275 + i * 4..275 + i * 4 + 4].copy_from_slice(&self.hole_offset_x[i].to_le_bytes());
            buf[403 + i * 4..403 + i * 4 + 4].copy_from_slice(&self.hole_offset_y[i].to_le_bytes());
        }
        buf[532..564].copy_from_slice(&self.alt_shape);
        buf[564..596].copy_from_slice(&self.corner_radius);

        buf
    }
}

/// Complete PCB Pad record with all 6 subrecords.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbPad {
    /// Subrecord 1: pad name raw bytes. Use `name()` for clean string.
    pub name_bytes: Vec<u8>,
    /// Subrecord 2: raw bytes (often empty or single null byte).
    pub unknown_block2: Vec<u8>,
    /// Subrecord 3: raw bytes (often `|&|0`).
    pub unknown_block3: Vec<u8>,
    /// Subrecord 4: raw bytes (often empty or single null byte).
    pub unknown_block4: Vec<u8>,
    /// Subrecord 5: main pad data (172 bytes).
    pub core: PcbPadCore,
    /// Subrecord 6: per-layer stack data (596+ bytes).
    pub stack: PcbPadStackData,
}

impl PcbPad {
    /// Get the pad name as a clean string (trimming trailing null).
    pub fn name(&self) -> String {
        let mut s = String::from_utf8_lossy(&self.name_bytes).into_owned();
        while s.ends_with('\0') {
            s.pop();
        }
        s
    }

    /// Read all 6 subrecords from stream (after type byte has been consumed).
    pub fn read_from(r: &mut impl Read) -> io::Result<Self> {
        let name_bytes = read_raw_block(r)?;
        let unknown_block2 = read_raw_block(r)?;
        let unknown_block3 = read_raw_block(r)?;
        let unknown_block4 = read_raw_block(r)?;

        let core_data = read_binary_block(r)?;
        let core = PcbPadCore::from_bytes(&core_data)?;

        let stack_data = read_binary_block(r)?;
        let stack = PcbPadStackData::from_bytes(&stack_data)?;

        Ok(Self {
            name_bytes,
            unknown_block2,
            unknown_block3,
            unknown_block4,
            core,
            stack,
        })
    }

    /// Write all 6 subrecords to stream (caller writes type byte).
    pub fn write_to(&self, w: &mut impl Write) -> io::Result<()> {
        write_raw_block(w, &self.name_bytes)?;
        write_raw_block(w, &self.unknown_block2)?;
        write_raw_block(w, &self.unknown_block3)?;
        write_raw_block(w, &self.unknown_block4)?;

        let core_bytes = self.core.to_bytes();
        w.write_all(&(core_bytes.len() as u32).to_le_bytes())?;
        w.write_all(&core_bytes)?;

        let stack_bytes = self.stack.to_bytes();
        w.write_all(&(stack_bytes.len() as u32).to_le_bytes())?;
        w.write_all(&stack_bytes)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_core_mutation_changes_output() {
        // Build a 172-byte raw core with known values
        let mut data = vec![0u8; 172];
        data[13..17].copy_from_slice(&1000i32.to_le_bytes()); // position_x
        data[171] = 1; // constant byte
        let core = PcbPadCore::from_bytes(&data).unwrap();
        let original = core.to_bytes();
        assert_eq!(original, data, "unchanged core must roundtrip exactly");

        // Mutate position_x
        let mut mutated_core = core.clone();
        mutated_core.position_x = PcbCoord::from_raw(9999);
        let mutated = mutated_core.to_bytes();
        assert_ne!(original[13..17], mutated[13..17],
            "mutating position_x must change bytes at offset 13");
        // Other bytes unchanged
        assert_eq!(original[17..], mutated[17..]);
    }

    #[test]
    fn pad_stack_mutation_changes_output() {
        let mut data = vec![0u8; 596];
        // Set inner_size_x[0] at offset 0
        data[0..4].copy_from_slice(&500i32.to_le_bytes());
        // Set hole_shape at offset 262
        data[262] = 1;
        let stack = PcbPadStackData::from_bytes(&data).unwrap();
        let original = stack.to_bytes();
        assert_eq!(original, data, "unchanged stack must roundtrip exactly");

        // Mutate inner_size_x[0]
        let mut mutated_stack = stack.clone();
        mutated_stack.inner_size_x[0] = 9999;
        let mutated = mutated_stack.to_bytes();
        assert_ne!(original[0..4], mutated[0..4],
            "mutating inner_size_x[0] must change bytes at offset 0");

        // Mutate hole_shape
        let mut mutated_stack2 = stack.clone();
        mutated_stack2.hole_shape = 2;
        let mutated2 = mutated_stack2.to_bytes();
        assert_ne!(original[262], mutated2[262],
            "mutating hole_shape must change byte at offset 262");
    }
}

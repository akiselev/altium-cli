//! PCB Text binary record (ID=5, 2 subrecords).
//!
//! Framing: `u8 type(5)` + 2 subrecords with `u32 len` prefix each.
//! Subrecord 1: Main text data (252 bytes in AD26, minimum 40).
//! Subrecord 2: Text string (variable length, null-terminated ASCII).
//!
//! Ghidra: FUN_01880680 → FUN_0185e100 → FUN_01856e60 → FUN_01849fd0.

use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

use super::coord::PcbCoord;
use super::primitive::PcbCommonHeader;

/// PCB Text record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbText {
    pub header: PcbCommonHeader,
    pub position_x: PcbCoord,
    pub position_y: PcbCoord,
    pub height: PcbCoord,
    /// Stroke font type (u16).
    pub stroke_font_type: u16,
    pub rotation: f64,
    pub is_mirrored: bool,
    pub stroke_width: PcbCoord,

    // Extended fields (if subrecord1 >= 123 bytes)
    pub is_comment: Option<bool>,
    pub is_designator: Option<bool>,
    pub user_routed: Option<bool>,
    pub font_type: Option<u8>,
    pub is_bold: Option<bool>,
    pub is_italic: Option<bool>,
    /// Font name raw bytes (UTF-16LE, 64 bytes = 32 wchars). Use `font_name()` for clean string.
    pub font_name_raw: Option<Vec<u8>>,
    pub is_inverted: Option<bool>,
    pub margin_border_width: Option<PcbCoord>,
    pub widestring_index: Option<u32>,
    pub union_index: Option<i32>,

    // Further extended (if >= 137)
    pub is_inverted_rect: Option<bool>,
    pub textbox_rect_width: Option<PcbCoord>,
    pub textbox_rect_height: Option<PcbCoord>,
    pub textbox_rect_justification: Option<u8>,
    pub text_offset_width: Option<PcbCoord>,

    // Barcode fields (if >= 240)
    pub barcode_type: Option<u8>,
    pub barcode_inverted: Option<bool>,
    pub barcode_font_type: Option<u8>,
    /// Barcode font name raw bytes (UTF-16LE, 64 bytes = 32 wchars). Use `barcode_font_name()` for clean string.
    pub barcode_font_name_raw: Option<Vec<u8>>,
    pub is_frame: Option<bool>,
    pub is_offset_border: Option<bool>,

    // Layer/sentinel fields (if >= 252, from Ghidra)
    pub layer_enum_index: Option<i32>,

    /// The text string (subrecord 2), clean for API use.
    pub text: String,

    /// Raw subrecord 1 data for round-trip fidelity.
    #[serde(skip)]
    raw_sub1: Vec<u8>,
    /// Raw subrecord 2 data for round-trip fidelity.
    #[serde(skip)]
    raw_sub2: Vec<u8>,
}

impl PcbText {
    /// Decode font name from raw bytes as a clean String.
    pub fn font_name(&self) -> Option<String> {
        self.font_name_raw.as_ref().map(|raw| {
            let wchar_count = raw.len() / 2;
            let utf16: Vec<u16> = (0..wchar_count)
                .map(|i| u16::from_le_bytes([raw[i * 2], raw[i * 2 + 1]]))
                .collect();
            String::from_utf16_lossy(&utf16)
                .trim_end_matches('\0')
                .to_string()
        })
    }

    /// Decode barcode font name from raw bytes as a clean String.
    pub fn barcode_font_name(&self) -> Option<String> {
        self.barcode_font_name_raw.as_ref().map(|raw| {
            let wchar_count = raw.len() / 2;
            let utf16: Vec<u16> = (0..wchar_count)
                .map(|i| u16::from_le_bytes([raw[i * 2], raw[i * 2 + 1]]))
                .collect();
            String::from_utf16_lossy(&utf16)
                .trim_end_matches('\0')
                .to_string()
        })
    }

    /// Parse from two subrecord data blobs.
    pub fn from_subrecords(sub1: &[u8], sub2: &[u8]) -> io::Result<Self> {
        if sub1.len() < 40 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "text sub1 too short"));
        }

        let rd = |off: usize| -> i32 {
            i32::from_le_bytes([sub1[off], sub1[off + 1], sub1[off + 2], sub1[off + 3]])
        };
        let rd16 = |off: usize| -> u16 {
            u16::from_le_bytes([sub1[off], sub1[off + 1]])
        };
        let rd32u = |off: usize| -> u32 {
            u32::from_le_bytes([sub1[off], sub1[off + 1], sub1[off + 2], sub1[off + 3]])
        };

        let mut r = std::io::Cursor::new(&sub1[..13]);
        let header = PcbCommonHeader::read_from(&mut r)?;

        let position_x = PcbCoord::from_raw(rd(13));
        let position_y = PcbCoord::from_raw(rd(17));
        let height = PcbCoord::from_raw(rd(21));
        let stroke_font_type = rd16(25);
        let rotation = f64::from_le_bytes(sub1[27..35].try_into().unwrap());
        let is_mirrored = sub1[35] != 0;
        let stroke_width = PcbCoord::from_raw(rd(36));

        let len = sub1.len();

        // Extended fields
        let (is_comment, is_designator, user_routed, font_type, is_bold, is_italic) =
            if len >= 46 {
                (
                    Some(sub1[40] != 0),
                    Some(sub1[41] != 0),
                    Some(sub1[42] != 0),
                    Some(sub1[43]),
                    Some(sub1[44] != 0),
                    Some(sub1[45] != 0),
                )
            } else {
                (None, None, None, None, None, None)
            };

        let font_name_raw = if len >= 110 {
            Some(sub1[46..110].to_vec())
        } else {
            None
        };

        let is_inverted = if len > 110 { Some(sub1[110] != 0) } else { None };
        let margin_border_width = if len > 114 { Some(PcbCoord::from_raw(rd(111))) } else { None };
        let widestring_index = if len > 118 { Some(rd32u(115)) } else { None };
        let union_index = if len > 122 { Some(rd(119)) } else { None };

        let is_inverted_rect = if len > 123 { Some(sub1[123] != 0) } else { None };
        let textbox_rect_width = if len > 127 { Some(PcbCoord::from_raw(rd(124))) } else { None };
        let textbox_rect_height = if len > 131 { Some(PcbCoord::from_raw(rd(128))) } else { None };
        let textbox_rect_justification = if len > 132 { Some(sub1[132]) } else { None };
        let text_offset_width = if len > 136 { Some(PcbCoord::from_raw(rd(133))) } else { None };

        // Barcode fields
        let barcode_type = if len > 157 { Some(sub1[157]) } else { None };
        let barcode_inverted = if len > 159 { Some(sub1[159] != 0) } else { None };
        let barcode_font_type = if len > 160 { Some(sub1[160]) } else { None };
        let barcode_font_name_raw = if len >= 225 {
            Some(sub1[161..225].to_vec())
        } else {
            None
        };
        let is_frame = if len > 230 { Some(sub1[230] != 0) } else { None };
        let is_offset_border = if len > 231 { Some(sub1[231] != 0) } else { None };

        let layer_enum_index = if len > 229 { Some(rd(226)) } else { None };

        // Subrecord 2: text string (clean version for API use)
        let text = String::from_utf8_lossy(sub2)
            .trim_end_matches('\0')
            .to_string();

        Ok(Self {
            header,
            position_x,
            position_y,
            height,
            stroke_font_type,
            rotation,
            is_mirrored,
            stroke_width,
            is_comment,
            is_designator,
            user_routed,
            font_type,
            is_bold,
            is_italic,
            font_name_raw,
            is_inverted,
            margin_border_width,
            widestring_index,
            union_index,
            is_inverted_rect,
            textbox_rect_width,
            textbox_rect_height,
            textbox_rect_justification,
            text_offset_width,
            barcode_type,
            barcode_inverted,
            barcode_font_type,
            barcode_font_name_raw,
            is_frame,
            is_offset_border,
            layer_enum_index,
            text,
            raw_sub1: sub1.to_vec(),
            raw_sub2: sub2.to_vec(),
        })
    }

    /// Read from stream (after type byte consumed). Reads 2 subrecords.
    pub fn read_from(r: &mut impl Read) -> io::Result<Self> {
        // Subrecord 1
        let mut len_buf = [0u8; 4];
        r.read_exact(&mut len_buf)?;
        let sub1_len = u32::from_le_bytes(len_buf) as usize;
        let mut sub1 = vec![0u8; sub1_len];
        r.read_exact(&mut sub1)?;

        // Subrecord 2
        r.read_exact(&mut len_buf)?;
        let sub2_len = u32::from_le_bytes(len_buf) as usize;
        let mut sub2 = vec![0u8; sub2_len];
        r.read_exact(&mut sub2)?;

        Self::from_subrecords(&sub1, &sub2)
    }

    /// Build subrecord 1 bytes from typed fields.
    ///
    /// If raw_sub1 exists, clones it and patches all typed fields at their
    /// known offsets. Otherwise builds from scratch at minimum size (40 bytes).
    fn build_sub1(&self) -> Vec<u8> {
        let mut buf = if !self.raw_sub1.is_empty() {
            self.raw_sub1.clone()
        } else {
            vec![0u8; 40]
        };
        let len = buf.len();

        // Core fields (always present, min 40 bytes)
        {
            let mut cursor = std::io::Cursor::new(&mut buf[..13]);
            let _ = self.header.write_to(&mut cursor);
        }
        buf[13..17].copy_from_slice(&self.position_x.to_raw().to_le_bytes());
        buf[17..21].copy_from_slice(&self.position_y.to_raw().to_le_bytes());
        buf[21..25].copy_from_slice(&self.height.to_raw().to_le_bytes());
        buf[25..27].copy_from_slice(&self.stroke_font_type.to_le_bytes());
        buf[27..35].copy_from_slice(&self.rotation.to_le_bytes());
        buf[35] = self.is_mirrored as u8;
        buf[36..40].copy_from_slice(&self.stroke_width.to_raw().to_le_bytes());

        // Extended fields (offset 40-45)
        if len >= 46 {
            if let Some(v) = self.is_comment { buf[40] = v as u8; }
            if let Some(v) = self.is_designator { buf[41] = v as u8; }
            if let Some(v) = self.user_routed { buf[42] = v as u8; }
            if let Some(v) = self.font_type { buf[43] = v; }
            if let Some(v) = self.is_bold { buf[44] = v as u8; }
            if let Some(v) = self.is_italic { buf[45] = v as u8; }
        }

        // Font name (raw 64 bytes at offset 46)
        if len >= 110 {
            if let Some(ref raw) = self.font_name_raw {
                let n = raw.len().min(64);
                buf[46..46 + n].copy_from_slice(&raw[..n]);
            }
        }

        if len > 110 { if let Some(v) = self.is_inverted { buf[110] = v as u8; } }
        if len > 114 { if let Some(v) = self.margin_border_width { buf[111..115].copy_from_slice(&v.to_raw().to_le_bytes()); } }
        if len > 118 { if let Some(v) = self.widestring_index { buf[115..119].copy_from_slice(&v.to_le_bytes()); } }
        if len > 122 { if let Some(v) = self.union_index { buf[119..123].copy_from_slice(&v.to_le_bytes()); } }

        // Further extended (offset 123-136)
        if len > 123 { if let Some(v) = self.is_inverted_rect { buf[123] = v as u8; } }
        if len > 127 { if let Some(v) = self.textbox_rect_width { buf[124..128].copy_from_slice(&v.to_raw().to_le_bytes()); } }
        if len > 131 { if let Some(v) = self.textbox_rect_height { buf[128..132].copy_from_slice(&v.to_raw().to_le_bytes()); } }
        if len > 132 { if let Some(v) = self.textbox_rect_justification { buf[132] = v; } }
        if len > 136 { if let Some(v) = self.text_offset_width { buf[133..137].copy_from_slice(&v.to_raw().to_le_bytes()); } }

        // Barcode fields
        if len > 157 { if let Some(v) = self.barcode_type { buf[157] = v; } }
        if len > 159 { if let Some(v) = self.barcode_inverted { buf[159] = v as u8; } }
        if len > 160 { if let Some(v) = self.barcode_font_type { buf[160] = v; } }
        // Barcode font name (raw 64 bytes at offset 161)
        if len >= 225 {
            if let Some(ref raw) = self.barcode_font_name_raw {
                let n = raw.len().min(64);
                buf[161..161 + n].copy_from_slice(&raw[..n]);
            }
        }
        if len > 229 { if let Some(v) = self.layer_enum_index { buf[226..230].copy_from_slice(&v.to_le_bytes()); } }
        if len > 230 { if let Some(v) = self.is_frame { buf[230] = v as u8; } }
        if len > 231 { if let Some(v) = self.is_offset_border { buf[231] = v as u8; } }

        buf
    }

    /// Build subrecord 2 bytes from typed fields.
    ///
    /// If raw_sub2 exists and text hasn't changed, returns raw_sub2.
    /// Otherwise builds from the text string.
    fn build_sub2(&self) -> Vec<u8> {
        if !self.raw_sub2.is_empty() {
            // Check if text matches raw_sub2 content
            let raw_text = String::from_utf8_lossy(&self.raw_sub2)
                .trim_end_matches('\0')
                .to_string();
            if raw_text == self.text {
                return self.raw_sub2.clone();
            }
        }
        // Build from text field
        let mut bytes = self.text.as_bytes().to_vec();
        bytes.push(0); // null terminator
        bytes
    }

    /// Write to stream (caller writes type byte). Writes 2 subrecords.
    pub fn write_to(&self, w: &mut impl Write) -> io::Result<()> {
        // Subrecord 1: build from typed fields (patching into raw_sub1 if available)
        let sub1 = self.build_sub1();
        w.write_all(&(sub1.len() as u32).to_le_bytes())?;
        w.write_all(&sub1)?;

        // Subrecord 2: build from text field
        let sub2 = self.build_sub2();
        w.write_all(&(sub2.len() as u32).to_le_bytes())?;
        w.write_all(&sub2)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_sub1(size: usize) -> Vec<u8> {
        let mut data = vec![0u8; size];
        // position_x at offset 13
        data[13..17].copy_from_slice(&100i32.to_le_bytes());
        // position_y at offset 17
        data[17..21].copy_from_slice(&200i32.to_le_bytes());
        // height at offset 21
        data[21..25].copy_from_slice(&300i32.to_le_bytes());
        // stroke_font_type at offset 25
        data[25..27].copy_from_slice(&1u16.to_le_bytes());
        // rotation at offset 27 (f64)
        data[27..35].copy_from_slice(&90.0f64.to_le_bytes());
        // is_mirrored at offset 35
        data[35] = 0;
        // stroke_width at offset 36
        data[36..40].copy_from_slice(&50i32.to_le_bytes());
        data
    }

    #[test]
    fn round_trip_basic() {
        let sub1 = make_test_sub1(40);
        let sub2 = b"Hello\0".to_vec();
        let text = PcbText::from_subrecords(&sub1, &sub2).unwrap();

        let mut out = Vec::new();
        text.write_to(&mut out).unwrap();

        // Parse output: u32 len + sub1 + u32 len + sub2
        let sub1_len = u32::from_le_bytes(out[0..4].try_into().unwrap()) as usize;
        assert_eq!(sub1_len, 40);
        assert_eq!(&out[4..4 + sub1_len], &sub1);
        let sub2_start = 4 + sub1_len;
        let sub2_len = u32::from_le_bytes(out[sub2_start..sub2_start + 4].try_into().unwrap()) as usize;
        assert_eq!(&out[sub2_start + 4..sub2_start + 4 + sub2_len], &sub2[..]);
    }

    #[test]
    fn mutation_changes_position() {
        let sub1 = make_test_sub1(252);
        let sub2 = b"Test\0".to_vec();
        let mut text = PcbText::from_subrecords(&sub1, &sub2).unwrap();

        let mut out1 = Vec::new();
        text.write_to(&mut out1).unwrap();

        // Mutate position_x
        text.position_x = PcbCoord::from_raw(999);
        let mut out2 = Vec::new();
        text.write_to(&mut out2).unwrap();

        assert_ne!(out1, out2, "mutating position_x must change output");
        // Check the specific bytes changed (offset 13-16 within sub1, which starts at byte 4)
        assert_ne!(out1[4 + 13..4 + 17], out2[4 + 13..4 + 17]);
    }

    #[test]
    fn mutation_changes_text_string() {
        let sub1 = make_test_sub1(40);
        let sub2 = b"Original\0".to_vec();
        let mut text = PcbText::from_subrecords(&sub1, &sub2).unwrap();

        let mut out1 = Vec::new();
        text.write_to(&mut out1).unwrap();

        text.text = "Modified".to_string();
        let mut out2 = Vec::new();
        text.write_to(&mut out2).unwrap();

        assert_ne!(out1, out2, "mutating text string must change output");
    }

    #[test]
    fn mutation_changes_extended_fields() {
        let mut sub1 = make_test_sub1(252);
        // Set is_bold at offset 44
        sub1[44] = 0;
        let sub2 = b"T\0".to_vec();
        let mut text = PcbText::from_subrecords(&sub1, &sub2).unwrap();
        assert_eq!(text.is_bold, Some(false));

        let mut out1 = Vec::new();
        text.write_to(&mut out1).unwrap();

        text.is_bold = Some(true);
        let mut out2 = Vec::new();
        text.write_to(&mut out2).unwrap();

        assert_ne!(out1[4 + 44], out2[4 + 44], "mutating is_bold must change byte at offset 44");
    }
}

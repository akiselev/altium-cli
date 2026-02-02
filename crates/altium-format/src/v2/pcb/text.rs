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
    /// Font name (UTF-16LE, up to 32 wchars = 64 bytes).
    pub font_name: Option<String>,
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
    pub barcode_font_name: Option<String>,
    pub is_frame: Option<bool>,
    pub is_offset_border: Option<bool>,

    // Layer/sentinel fields (if >= 252, from Ghidra)
    pub layer_enum_index: Option<i32>,

    /// The text string (subrecord 2).
    pub text: String,

    /// Raw subrecord 1 data for round-trip fidelity.
    #[serde(skip)]
    raw_sub1: Vec<u8>,
}

impl PcbText {
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

        let font_name = if len >= 110 {
            // UTF-16LE font name at offset 46, 64 bytes (32 wchars)
            let raw_utf16: Vec<u16> = (0..32)
                .map(|i| u16::from_le_bytes([sub1[46 + i * 2], sub1[47 + i * 2]]))
                .collect();
            let name = String::from_utf16_lossy(&raw_utf16);
            Some(name.trim_end_matches('\0').to_string())
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
        let barcode_font_name = if len >= 225 {
            let raw_utf16: Vec<u16> = (0..32)
                .map(|i| u16::from_le_bytes([sub1[161 + i * 2], sub1[162 + i * 2]]))
                .collect();
            let name = String::from_utf16_lossy(&raw_utf16);
            Some(name.trim_end_matches('\0').to_string())
        } else {
            None
        };
        let is_frame = if len > 230 { Some(sub1[230] != 0) } else { None };
        let is_offset_border = if len > 231 { Some(sub1[231] != 0) } else { None };

        let layer_enum_index = if len > 229 { Some(rd(226)) } else { None };

        // Subrecord 2: text string
        let text = String::from_utf8_lossy(sub2)
            .trim_end_matches('\0')
            .replace("\r\n", "\n")
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
            font_name,
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
            barcode_font_name,
            is_frame,
            is_offset_border,
            layer_enum_index,
            text,
            raw_sub1: sub1.to_vec(),
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

    /// Write to stream (caller writes type byte). Writes 2 subrecords.
    pub fn write_to(&self, w: &mut impl Write) -> io::Result<()> {
        // Subrecord 1: use raw for round-trip
        let sub1 = if !self.raw_sub1.is_empty() {
            &self.raw_sub1
        } else {
            // Minimal: would need to rebuild — for now require raw
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "text write without raw data not yet supported",
            ));
        };
        w.write_all(&(sub1.len() as u32).to_le_bytes())?;
        w.write_all(sub1)?;

        // Subrecord 2: text string
        let text_bytes = self.text.as_bytes();
        w.write_all(&(text_bytes.len() as u32).to_le_bytes())?;
        w.write_all(text_bytes)?;

        Ok(())
    }
}

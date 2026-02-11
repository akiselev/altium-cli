//! PCB Pad record type for the v2 API.
//!
//! The pad record is a complex multi-section binary format with 6 subrecords.
//! It uses custom parse/serialize functions which are stubbed for now and
//! will be fully implemented in Phase 4 (document I/O).
//!
//! Subrecords (from Ghidra):
//! 1. Pad name (WxString)
//! 2. Unknown string (often empty)
//! 3. Unknown string (often `|&|0`)
//! 4. Unknown string (often empty)
//! 5. Main pad data (172 bytes in AD26)
//! 6. Per-layer stack data (596/628/651 bytes)

use altium_format_derive::altium_record;
use crate::v2::coord::PcbCoord;

#[altium_record(kind = "pcb", object_id = Pad, codec = "binary",
    parse_fn = "parse_pad", serialize_fn = "serialize_pad")]
pub struct PcbPadRecord {
    /// X position in PCB coordinates.
    position_x: PcbCoord,
    /// Y position in PCB coordinates.
    position_y: PcbCoord,
    /// Top layer X size.
    top_size_x: PcbCoord,
    /// Top layer Y size.
    top_size_y: PcbCoord,
    /// Mid layer X size.
    mid_size_x: PcbCoord,
    /// Mid layer Y size.
    mid_size_y: PcbCoord,
    /// Bottom layer X size.
    bot_size_x: PcbCoord,
    /// Bottom layer Y size.
    bot_size_y: PcbCoord,
    /// Hole size.
    hole_size: PcbCoord,
    /// Top layer shape (TShape enum value).
    top_shape: u8,
    /// Mid layer shape.
    mid_shape: u8,
    /// Bottom layer shape.
    bot_shape: u8,
    /// Rotation in degrees.
    rotation: f64,
    /// Whether the pad hole is plated.
    is_plated: bool,
    /// Pad stack mode (TPadMode enum value).
    pad_mode: u8,
    /// Paste mask expansion.
    paste_mask_expansion: PcbCoord,
    /// Solder mask expansion.
    solder_mask_expansion: PcbCoord,
    /// Layer (byte 0 of common header in subrecord 5).
    layer: u8,
}

/// Parse pad data from the raw binary block (6 subrecords).
///
/// Each subrecord is length-prefixed with u32. The typed fields are
/// extracted from subrecord 5 (main pad core data) at fixed offsets.
fn parse_pad(data: &[u8]) -> crate::Result<crate::v2::backing_store::RecordOrigin> {
    use crate::v2::backing_store::{BinaryOrigin, FieldSpan};
    use crate::error::AltiumError;

    // Walk through 4 string subrecords to find subrecord 5's data offset
    let mut offset = 0usize;
    for i in 0..4 {
        if offset + 4 > data.len() {
            return Err(AltiumError::Parse(format!(
                "pad data too short reading subrecord {} length", i + 1
            )));
        }
        let sub_len = u32::from_le_bytes(
            data[offset..offset + 4].try_into().unwrap(),
        ) as usize;
        offset += 4 + sub_len;
    }

    // Subrecord 5: main pad core data
    if offset + 4 > data.len() {
        return Err(AltiumError::Parse(
            "pad data too short for subrecord 5 length".into(),
        ));
    }
    let core_len = u32::from_le_bytes(
        data[offset..offset + 4].try_into().unwrap(),
    ) as usize;
    let core_start = offset + 4; // skip length prefix
    if core_start + core_len > data.len() {
        return Err(AltiumError::Parse(
            "pad subrecord 5 extends beyond data".into(),
        ));
    }
    if core_len < 94 {
        return Err(AltiumError::Parse(format!(
            "pad core too short: {} bytes (need >= 94)", core_len
        )));
    }

    // Field offsets within subrecord 5 data (from v1 PcbPadCore::from_bytes)
    // Byte 0-12: PcbCommonHeader (13 bytes), then typed fields follow
    let s = core_start; // base offset into raw_block
    let spans = vec![
        FieldSpan::new(s + 13, 4),  // 0: position_x
        FieldSpan::new(s + 17, 4),  // 1: position_y
        FieldSpan::new(s + 21, 4),  // 2: top_size_x
        FieldSpan::new(s + 25, 4),  // 3: top_size_y
        FieldSpan::new(s + 29, 4),  // 4: mid_size_x
        FieldSpan::new(s + 33, 4),  // 5: mid_size_y
        FieldSpan::new(s + 37, 4),  // 6: bot_size_x
        FieldSpan::new(s + 41, 4),  // 7: bot_size_y
        FieldSpan::new(s + 45, 4),  // 8: hole_size
        FieldSpan::new(s + 49, 1),  // 9: top_shape
        FieldSpan::new(s + 50, 1),  // 10: mid_shape
        FieldSpan::new(s + 51, 1),  // 11: bot_shape
        FieldSpan::new(s + 52, 8),  // 12: rotation
        FieldSpan::new(s + 60, 1),  // 13: is_plated
        FieldSpan::new(s + 62, 1),  // 14: pad_mode (offset 62, byte 61 is padding)
        FieldSpan::new(s + 86, 4),  // 15: paste_mask_expansion
        FieldSpan::new(s + 90, 4),  // 16: solder_mask_expansion
        FieldSpan::new(s + 0, 1),   // 17: layer (byte 0 of common header)
    ];

    Ok(crate::v2::backing_store::RecordOrigin::Binary(
        BinaryOrigin::with_spans(data.to_vec(), spans),
    ))
}

impl PcbPadRecord {
    /// Create a `PcbPadRecord` from raw binary pad data.
    ///
    /// Parses the 6-subrecord binary format and creates an origin with
    /// proper field spans for typed access via the generated getters.
    pub fn from_binary(data: &[u8]) -> crate::Result<Self> {
        let origin = parse_pad(data)?;
        Ok(Self::from_origin(origin))
    }

    /// Returns the pad designator string (extracted from subrecord 1).
    ///
    /// The designator is stored as a length-prefixed string at the start
    /// of the raw binary block, before the core data subrecords.
    pub fn designator(&self) -> String {
        use crate::v2::traits::RecordType;
        let origin = self.origin();
        if let Some(binary) = origin.as_binary() {
            let data = &binary.raw_block;
            if data.len() < 4 {
                return String::new();
            }
            let name_len = u32::from_le_bytes(
                data[0..4].try_into().unwrap_or([0; 4]),
            ) as usize;
            if 4 + name_len > data.len() {
                return String::new();
            }
            String::from_utf8_lossy(&data[4..4 + name_len])
                .trim_end_matches('\0')
                .to_string()
        } else {
            String::new()
        }
    }
}

/// Serialize pad data back to binary.
///
/// Returns the raw_block which has already been patched by setters.
fn serialize_pad(origin: &crate::v2::backing_store::BinaryOrigin) -> crate::Result<Vec<u8>> {
    Ok(origin.raw_block.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{BinaryOrigin, FieldSpan, RecordOrigin};
    use crate::v2::coord::AltiumCoord;

    /// Build a test BinaryOrigin with field spans for the pad record.
    fn make_test_pad_origin() -> RecordOrigin {
        let mut data = vec![0u8; 256];

        // position_x at offset 0
        data[0..4].copy_from_slice(&100_000i32.to_le_bytes());
        // position_y at offset 4
        data[4..8].copy_from_slice(&200_000i32.to_le_bytes());
        // top_size_x at offset 8
        data[8..12].copy_from_slice(&50_000i32.to_le_bytes());
        // top_size_y at offset 12
        data[12..16].copy_from_slice(&50_000i32.to_le_bytes());
        // mid_size_x at offset 16
        data[16..20].copy_from_slice(&40_000i32.to_le_bytes());
        // mid_size_y at offset 20
        data[20..24].copy_from_slice(&40_000i32.to_le_bytes());
        // bot_size_x at offset 24
        data[24..28].copy_from_slice(&50_000i32.to_le_bytes());
        // bot_size_y at offset 28
        data[28..32].copy_from_slice(&50_000i32.to_le_bytes());
        // hole_size at offset 32
        data[32..36].copy_from_slice(&10_000i32.to_le_bytes());
        // top_shape at offset 36
        data[36] = 1; // Rounded
        // mid_shape at offset 37
        data[37] = 1;
        // bot_shape at offset 38
        data[38] = 1;
        // rotation at offset 39
        data[39..47].copy_from_slice(&45.0f64.to_le_bytes());
        // is_plated at offset 47
        data[47] = 1;
        // pad_mode at offset 48
        data[48] = 0; // Simple
        // paste_mask_expansion at offset 49
        data[49..53].copy_from_slice(&1000i32.to_le_bytes());
        // solder_mask_expansion at offset 53
        data[53..57].copy_from_slice(&2000i32.to_le_bytes());
        // layer at offset 57
        data[57] = 74; // MultiLayer

        let spans = vec![
            FieldSpan::new(0, 4),   // position_x
            FieldSpan::new(4, 4),   // position_y
            FieldSpan::new(8, 4),   // top_size_x
            FieldSpan::new(12, 4),  // top_size_y
            FieldSpan::new(16, 4),  // mid_size_x
            FieldSpan::new(20, 4),  // mid_size_y
            FieldSpan::new(24, 4),  // bot_size_x
            FieldSpan::new(28, 4),  // bot_size_y
            FieldSpan::new(32, 4),  // hole_size
            FieldSpan::new(36, 1),  // top_shape
            FieldSpan::new(37, 1),  // mid_shape
            FieldSpan::new(38, 1),  // bot_shape
            FieldSpan::new(39, 8),  // rotation
            FieldSpan::new(47, 1),  // is_plated
            FieldSpan::new(48, 1),  // pad_mode
            FieldSpan::new(49, 4),  // paste_mask_expansion
            FieldSpan::new(53, 4),  // solder_mask_expansion
            FieldSpan::new(57, 1),  // layer
        ];

        RecordOrigin::Binary(BinaryOrigin::with_spans(data, spans))
    }

    #[test]
    fn pad_read_from_field_spans() {
        let origin = make_test_pad_origin();
        let rec = PcbPadRecord::from_origin(origin);

        assert_eq!(rec.position_x().to_raw(), 100_000);
        assert_eq!(rec.position_y().to_raw(), 200_000);
        assert_eq!(rec.top_size_x().to_raw(), 50_000);
        assert_eq!(rec.hole_size().to_raw(), 10_000);
        assert_eq!(rec.top_shape(), 1);
        assert!((rec.rotation() - 45.0).abs() < 1e-10);
        assert!(rec.is_plated());
        assert_eq!(rec.pad_mode(), 0);
        assert_eq!(rec.paste_mask_expansion().to_raw(), 1000);
        assert_eq!(rec.solder_mask_expansion().to_raw(), 2000);
        assert_eq!(rec.layer(), 74);
    }

    #[test]
    fn pad_write_via_field_spans() {
        let origin = make_test_pad_origin();
        let mut rec = PcbPadRecord::from_origin(origin);

        rec.set_position_x(PcbCoord::from_raw(999_999));
        assert_eq!(rec.position_x().to_raw(), 999_999);

        rec.set_rotation(90.0);
        assert!((rec.rotation() - 90.0).abs() < 1e-10);

        rec.set_is_plated(false);
        assert!(!rec.is_plated());
    }
}

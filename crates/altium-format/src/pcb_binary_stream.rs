//! Layer 3 stream parser for PCB binary record streams (Format B).
//! Used by PcbDoc/PcbLib primitive sections (Arcs6, Pads6, Tracks6, Vias6,
//! Texts6, Fills6, Regions6, etc.).
//! Each record: u8 object_id + u32 LE record_length + payload bytes.
//! The high byte of record_length may contain flags; mask with SIZE_MASK.

use altium_format_types::constants::parsing::BLOCK_SIZE_MASK;
use altium_format_types::PcbObjectId;

use crate::binary_io::BinaryReader;
use crate::Result;

/// A raw PCB binary record before type dispatch.
#[derive(Debug)]
pub(crate) struct PcbBinaryRecord {
    /// Typed PCB object identifier (Arc, Pad, Via, Track, etc.)
    pub(crate) object_id: PcbObjectId,
    /// Raw payload bytes for this record.
    pub(crate) data: Vec<u8>,
}

/// Parse all PCB binary records from a section Data stream.
///
/// Each record is: u8 object_id + u32 LE length (masked) + payload.
/// Validates that the entire stream is consumed (no trailing bytes).
pub(crate) fn parse_pcb_binary_records(stream_data: &[u8]) -> Result<Vec<PcbBinaryRecord>> {
    let mut reader = BinaryReader::new(stream_data);
    let mut records = Vec::new();
    while reader.remaining() > 0 {
        let raw_id = reader.read_u8()?;
        let object_id = PcbObjectId::try_from(raw_id)?;
        let raw_length = reader.read_u32_le()?;
        let length = (raw_length & BLOCK_SIZE_MASK) as usize;
        let data = reader.read_bytes(length)?.to_vec();
        records.push(PcbBinaryRecord { object_id, data });
    }
    reader.assert_exhausted()?;
    Ok(records)
}

/// Read the record count from a PCB section Header stream.
/// Header is always exactly 4 bytes: u32 LE count.
pub(crate) fn parse_pcb_section_header(header_data: &[u8]) -> Result<u32> {
    let mut reader = BinaryReader::new(header_data);
    let count = reader.read_u32_le()?;
    reader.assert_exhausted()?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_io::BinaryWriter;
    use crate::AltiumFormatError;

    #[test]
    fn parse_single_record() {
        let mut w = BinaryWriter::new();
        w.write_u8(PcbObjectId::Arc as u8);
        w.write_u32_le(3); // length = 3
        w.write_bytes(&[0xAA, 0xBB, 0xCC]); // payload
        let data = w.finish();
        let records = parse_pcb_binary_records(&data).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].object_id, PcbObjectId::Arc);
        assert_eq!(records[0].data, vec![0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn parse_multiple_records() {
        let mut w = BinaryWriter::new();
        // Record 1: Arc with 2 bytes
        w.write_u8(PcbObjectId::Arc as u8);
        w.write_u32_le(2);
        w.write_bytes(&[0x01, 0x02]);
        // Record 2: Pad with 1 byte
        w.write_u8(PcbObjectId::Pad as u8);
        w.write_u32_le(1);
        w.write_bytes(&[0xFF]);
        let data = w.finish();
        let records = parse_pcb_binary_records(&data).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].object_id, PcbObjectId::Arc);
        assert_eq!(records[0].data, vec![0x01, 0x02]);
        assert_eq!(records[1].object_id, PcbObjectId::Pad);
        assert_eq!(records[1].data, vec![0xFF]);
    }

    #[test]
    fn parse_record_with_flags_in_high_byte() {
        let mut w = BinaryWriter::new();
        w.write_u8(PcbObjectId::Track as u8);
        // Length with flags in high byte: 0x01000003 -> actual size = 3
        w.write_u32_le(0x01000003);
        w.write_bytes(&[0xAA, 0xBB, 0xCC]);
        let data = w.finish();
        let records = parse_pcb_binary_records(&data).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].object_id, PcbObjectId::Track);
        assert_eq!(records[0].data, vec![0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn parse_empty_stream() {
        let records = parse_pcb_binary_records(&[]).unwrap();
        assert!(records.is_empty());
    }

    #[test]
    fn truncated_record_returns_error() {
        // Only object_id, no length
        let err = parse_pcb_binary_records(&[0x01]).unwrap_err();
        assert!(matches!(err, AltiumFormatError::BinaryReadPastEnd { .. }));
    }

    #[test]
    fn payload_past_end_returns_error() {
        let mut w = BinaryWriter::new();
        w.write_u8(PcbObjectId::Arc as u8);
        w.write_u32_le(100); // claims 100 bytes
        w.write_bytes(&[0xAA]); // only 1 byte
        let data = w.finish();
        let err = parse_pcb_binary_records(&data).unwrap_err();
        assert!(matches!(err, AltiumFormatError::BinaryReadPastEnd { .. }));
    }

    #[test]
    fn parse_section_header() {
        let mut w = BinaryWriter::new();
        w.write_u32_le(42);
        let data = w.finish();
        let count = parse_pcb_section_header(&data).unwrap();
        assert_eq!(count, 42);
    }

    #[test]
    fn section_header_wrong_size() {
        // Too short
        let err = parse_pcb_section_header(&[0x01, 0x02]).unwrap_err();
        assert!(matches!(err, AltiumFormatError::BinaryReadPastEnd { .. }));
    }

    #[test]
    fn section_header_trailing_data() {
        let mut w = BinaryWriter::new();
        w.write_u32_le(10);
        w.write_u8(0xFF); // extra byte
        let data = w.finish();
        let err = parse_pcb_section_header(&data).unwrap_err();
        assert!(matches!(err, AltiumFormatError::UnexpectedTrailingData { .. }));
    }
}

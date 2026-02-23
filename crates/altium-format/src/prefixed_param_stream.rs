//! Layer 3 stream parser for PCB prefixed parameter blocks (Format C).
//! Used by PcbDoc sections Rules6, NewRules6, Dimensions6, Coordinates6.
//! Each block: u16 LE prefix + u32 LE payload_size + NUL-terminated parameter string.

use crate::Result;
use crate::binary_io::BinaryReader;

/// A prefixed parameter block from Rules6/Dimensions6/etc.
#[derive(Debug)]
pub(crate) struct PrefixedParamBlock {
    /// Section-specific prefix value.
    pub(crate) prefix: u16,
    /// Raw payload bytes (NUL-terminated pipe-delimited parameter string).
    pub(crate) data: Vec<u8>,
}

/// Parse all prefixed parameter blocks from a section Data stream.
///
/// Each block is: u16 LE prefix + u32 LE payload_size + payload bytes.
/// Validates that the entire stream is consumed (no trailing bytes).
pub(crate) fn parse_prefixed_param_blocks(stream_data: &[u8]) -> Result<Vec<PrefixedParamBlock>> {
    let mut reader = BinaryReader::new(stream_data);
    let mut blocks = Vec::new();
    while reader.remaining() > 0 {
        let prefix = reader.read_u16_le()?;
        let size = reader.read_u32_le()? as usize;
        let data = reader.read_bytes(size)?.to_vec();
        blocks.push(PrefixedParamBlock { prefix, data });
    }
    reader.assert_exhausted()?;
    Ok(blocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AltiumFormatError;
    use crate::binary_io::BinaryWriter;

    #[test]
    fn parse_single_block() {
        let mut w = BinaryWriter::new();
        w.write_u16_le(0x0001);
        let payload = b"|RULEKIND=Clearance|\0";
        w.write_u32_le(payload.len() as u32);
        w.write_bytes(payload);
        let data = w.finish();
        let blocks = parse_prefixed_param_blocks(&data).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].prefix, 1);
        assert_eq!(blocks[0].data, payload);
    }

    #[test]
    fn parse_multiple_blocks() {
        let mut w = BinaryWriter::new();
        let p1 = b"|A=1|\0";
        w.write_u16_le(10);
        w.write_u32_le(p1.len() as u32);
        w.write_bytes(p1);
        let p2 = b"|B=2|\0";
        w.write_u16_le(20);
        w.write_u32_le(p2.len() as u32);
        w.write_bytes(p2);
        let data = w.finish();
        let blocks = parse_prefixed_param_blocks(&data).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].prefix, 10);
        assert_eq!(blocks[1].prefix, 20);
    }

    #[test]
    fn empty_stream() {
        let blocks = parse_prefixed_param_blocks(&[]).unwrap();
        assert!(blocks.is_empty());
    }

    #[test]
    fn truncated_header_returns_error() {
        let err = parse_prefixed_param_blocks(&[0x01]).unwrap_err();
        assert!(matches!(err, AltiumFormatError::BinaryReadPastEnd { .. }));
    }

    #[test]
    fn payload_past_end_returns_error() {
        let mut w = BinaryWriter::new();
        w.write_u16_le(1);
        w.write_u32_le(100); // claims 100 bytes
        w.write_bytes(&[0xAA]); // only 1 byte
        let data = w.finish();
        let err = parse_prefixed_param_blocks(&data).unwrap_err();
        assert!(matches!(err, AltiumFormatError::BinaryReadPastEnd { .. }));
    }
}

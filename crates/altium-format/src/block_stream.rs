//! Layer 3 of the 5-layer parsing stack: block-stream framing.
//! Each Altium stream is a sequence of length-prefixed blocks.
//! The 4-byte header encodes payload size (bits 0-23) and format (bits 24-31):
//! 0x00 = text (pipe-delimited parameters), 0x01 = binary (packed struct).
//! Unknown flag values are hard errors — Altium has no other documented formats.
use altium_format_types::constants::parsing::{
    BLOCK_FLAG_BINARY, BLOCK_FLAG_SHIFT, BLOCK_FLAG_TEXT, BLOCK_SIZE_MASK,
};

use crate::{AltiumFormatError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockFormat {
    Text,
    Binary,
}

#[derive(Debug, Clone)]
pub(crate) struct Block {
    pub(crate) format: BlockFormat,
    pub(crate) data: Vec<u8>,
}

// Parses all blocks from `stream_data` eagerly, returning an error on the first bad header.
pub(crate) fn parse_blocks(stream_data: &[u8]) -> Result<Vec<Block>> {
    let mut iter = BlockIter::new(stream_data);
    let mut blocks = Vec::new();
    for result in &mut iter {
        blocks.push(result?);
    }
    Ok(blocks)
}

// Returns a lazy iterator over blocks; use when processing a stream incrementally.
pub(crate) fn iter_blocks(stream_data: &[u8]) -> BlockIter<'_> {
    BlockIter::new(stream_data)
}

/// Encodes a payload as a text-format block: 4-byte header (flags=0x00) + payload bytes.
pub(crate) fn write_text_block(payload: &[u8]) -> Vec<u8> {
    let header: i32 = (BLOCK_FLAG_TEXT as i32) << BLOCK_FLAG_SHIFT | (payload.len() as i32 & BLOCK_SIZE_MASK as i32);
    let mut out = Vec::with_capacity(4 + payload.len());
    out.extend_from_slice(&header.to_le_bytes());
    out.extend_from_slice(payload);
    out
}

/// Encodes a payload as a binary-format block: 4-byte header (flags=0x01) + payload bytes.
pub(crate) fn write_binary_block(payload: &[u8]) -> Vec<u8> {
    let header: i32 = (BLOCK_FLAG_BINARY as i32) << BLOCK_FLAG_SHIFT | (payload.len() as i32 & BLOCK_SIZE_MASK as i32);
    let mut out = Vec::with_capacity(4 + payload.len());
    out.extend_from_slice(&header.to_le_bytes());
    out.extend_from_slice(payload);
    out
}

pub(crate) struct BlockIter<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BlockIter<'a> {
    // Wraps a byte slice for lazy block parsing starting at position 0.
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
}

impl<'a> Iterator for BlockIter<'a> {
    type Item = Result<Block>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.data.len() {
            return None;
        }
        if self.data.len() - self.pos < 4 {
            return Some(Err(AltiumFormatError::InvalidBlockHeader {
                offset: self.pos,
                detail: format!(
                    "truncated header: only {} bytes remain",
                    self.data.len() - self.pos
                ),
            }));
        }
        let header_offset = self.pos;
        let header_bytes = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ];
        let header = i32::from_le_bytes(header_bytes);
        let size = (header & BLOCK_SIZE_MASK as i32) as usize;
        let flags = (header >> BLOCK_FLAG_SHIFT) as u8;
        self.pos += 4;
        let format = match flags {
            BLOCK_FLAG_TEXT => BlockFormat::Text,
            BLOCK_FLAG_BINARY => BlockFormat::Binary,
            other => {
                return Some(Err(AltiumFormatError::InvalidBlockHeader {
                    offset: header_offset,
                    detail: format!("unknown flags byte {other:#04x}"),
                }));
            }
        };
        if self.pos + size > self.data.len() {
            return Some(Err(AltiumFormatError::InvalidBlockHeader {
                offset: header_offset,
                detail: format!(
                    "payload size {size} extends past stream end (stream has {} bytes, pos {})",
                    self.data.len(),
                    self.pos
                ),
            }));
        }
        let data = self.data[self.pos..self.pos + size].to_vec();
        self.pos += size;
        Some(Ok(Block { format, data }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_block_bytes(payload: &[u8], flags: u8) -> Vec<u8> {
        let size = payload.len() as i32;
        let header = size | ((flags as i32) << 24);
        let mut bytes = header.to_le_bytes().to_vec();
        bytes.extend_from_slice(payload);
        bytes
    }

    #[test]
    fn empty_stream_produces_empty_vec() {
        let blocks = parse_blocks(&[]).unwrap();
        assert!(blocks.is_empty());
    }

    #[test]
    fn single_text_block() {
        let payload = b"hello";
        let data = make_block_bytes(payload, BLOCK_FLAG_TEXT);
        let blocks = parse_blocks(&data).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].format, BlockFormat::Text);
        assert_eq!(blocks[0].data, payload);
    }

    #[test]
    fn single_binary_block() {
        let payload = b"\x01\x02\x03";
        let data = make_block_bytes(payload, BLOCK_FLAG_BINARY);
        let blocks = parse_blocks(&data).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].format, BlockFormat::Binary);
        assert_eq!(blocks[0].data, payload);
    }

    #[test]
    fn multiple_blocks() {
        let mut data = make_block_bytes(b"text", BLOCK_FLAG_TEXT);
        data.extend_from_slice(&make_block_bytes(b"\xAA\xBB", BLOCK_FLAG_BINARY));
        let blocks = parse_blocks(&data).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].format, BlockFormat::Text);
        assert_eq!(blocks[0].data, b"text");
        assert_eq!(blocks[1].format, BlockFormat::Binary);
        assert_eq!(blocks[1].data, b"\xAA\xBB");
    }

    #[test]
    fn truncated_header_returns_error() {
        let result = parse_blocks(&[0x01, 0x02, 0x03]);
        assert!(matches!(result, Err(AltiumFormatError::InvalidBlockHeader { .. })));
    }

    #[test]
    fn payload_past_end_returns_error() {
        // Header says 100 bytes but only 2 bytes follow
        let header = 100i32.to_le_bytes();
        let mut data = header.to_vec();
        data.extend_from_slice(&[0xAA, 0xBB]);
        let result = parse_blocks(&data);
        assert!(matches!(result, Err(AltiumFormatError::InvalidBlockHeader { .. })));
    }

    #[test]
    fn unknown_flags_returns_error() {
        let data = make_block_bytes(b"x", 0x02);
        let result = parse_blocks(&data);
        assert!(matches!(result, Err(AltiumFormatError::InvalidBlockHeader { .. })));
    }

    #[test]
    fn write_text_block_roundtrips() {
        let payload = b"|RECORD=1|KEY=VALUE|\0";
        let block_bytes = write_text_block(payload);
        let blocks = parse_blocks(&block_bytes).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].format, BlockFormat::Text);
        assert_eq!(blocks[0].data, payload);
    }

    #[test]
    fn write_binary_block_roundtrips() {
        let payload = b"\x02\x00\x00\x00\x01\x00";
        let block_bytes = write_binary_block(payload);
        let blocks = parse_blocks(&block_bytes).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].format, BlockFormat::Binary);
        assert_eq!(blocks[0].data, payload);
    }

    #[test]
    fn write_multiple_blocks_roundtrip() {
        let mut stream = write_text_block(b"|RECORD=0|\0");
        stream.extend_from_slice(&write_binary_block(b"\x02\x03"));
        stream.extend_from_slice(&write_text_block(b"|KEY=val|\0"));
        let blocks = parse_blocks(&stream).unwrap();
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].format, BlockFormat::Text);
        assert_eq!(blocks[1].format, BlockFormat::Binary);
        assert_eq!(blocks[2].format, BlockFormat::Text);
    }

    #[test]
    fn write_empty_payload_roundtrips() {
        let block_bytes = write_text_block(b"");
        let blocks = parse_blocks(&block_bytes).unwrap();
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].data.is_empty());
    }

    #[test]
    fn iter_blocks_matches_parse_blocks() {
        let mut data = make_block_bytes(b"abc", BLOCK_FLAG_TEXT);
        data.extend_from_slice(&make_block_bytes(b"\x01\x02", BLOCK_FLAG_BINARY));
        let parsed: Vec<Block> = parse_blocks(&data).unwrap();
        let iterated: Vec<Block> = iter_blocks(&data).collect::<std::result::Result<Vec<_>, _>>().unwrap();
        assert_eq!(parsed.len(), iterated.len());
        for (p, i) in parsed.iter().zip(iterated.iter()) {
            assert_eq!(p.format, i.format);
            assert_eq!(p.data, i.data);
        }
    }
}

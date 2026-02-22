//! Layer 4 parser for the embedded object envelope format.
//! Each entry in a Storage or sidecar block stream is a 0xD0-tagged envelope:
//! tag(1) + id_length(1) + id(N) + inner_header(4) + inner_data(M).
//! The inner header uses the same bit layout as the block header (bits 0-23
//! = size, bits 24-31 = format discriminant).
//! `parse_embedded_object_stream` consumes the header block's params internally
//! so callers never receive a partially-consumed `ParameterCollection`.
use crate::binary_io::BinaryReader;
use crate::block_stream::{Block, BlockFormat};
use crate::param_collection::ParameterCollection;
use crate::{AltiumFormatError, Result};

#[derive(Debug)]
pub(crate) struct EmbeddedObject {
    pub(crate) id: String,
    pub(crate) inner_format: BlockFormat,
    pub(crate) inner_data: Vec<u8>,
}

// Parses a single 0xD0-tagged embedded object envelope from a binary block payload.
pub(crate) fn parse_embedded_object(data: &[u8]) -> Result<EmbeddedObject> {
    let mut reader = BinaryReader::new(data);
    let tag = reader.read_u8()?;
    if tag != 0xD0 {
        return Err(AltiumFormatError::InvalidEmbeddedObject(format!(
            "expected 0xD0 tag, got {tag:#04x}"
        )));
    }
    let id_len = reader.read_u8()? as usize;
    let id_bytes = reader.read_bytes(id_len)?;
    let id = String::from_utf8(id_bytes.to_vec()).map_err(|e| {
        AltiumFormatError::InvalidEmbeddedObject(format!(
            "embedded object id contains invalid UTF-8: {e}"
        ))
    })?;
    let inner_header = reader.read_i32_le()?;
    let inner_size = (inner_header & 0x00FF_FFFF) as usize;
    let inner_flags = (inner_header >> 24) as u8;
    let inner_format = match inner_flags {
        0x00 => BlockFormat::Text,
        0x01 => BlockFormat::Binary,
        other => {
            return Err(AltiumFormatError::InvalidEmbeddedObject(format!(
                "unknown inner block flags {other:#04x}"
            )));
        }
    };
    let inner_data = reader.read_bytes(inner_size)?.to_vec();
    reader.assert_exhausted()?;
    Ok(EmbeddedObject { id, inner_format, inner_data })
}

// Parses the Storage-style block stream: block 0 = header params, blocks 1..N = entries.
// Header params (RECORD, Weight) are consumed internally; callers receive only the entries.
pub(crate) fn parse_embedded_object_stream(
    blocks: &[Block],
) -> Result<Vec<EmbeddedObject>> {
    if blocks.is_empty() {
        return Err(AltiumFormatError::InvalidEmbeddedObject(
            "empty block list for embedded object stream".to_owned(),
        ));
    }
    if blocks[0].format != BlockFormat::Text {
        return Err(AltiumFormatError::InvalidEmbeddedObject(
            "first block of embedded object stream must be text format".to_owned(),
        ));
    }
    let mut params = ParameterCollection::from_bytes(&blocks[0].data)?;
    // RECORD=0 sentinel may appear on the header block; consume it without dispatch.
    params.remove_optional::<i32>("RECORD")?;
    let weight: usize = params.remove_required("Weight")?;
    params.assert_exhausted()?;
    let entries: Result<Vec<EmbeddedObject>> =
        blocks[1..].iter().map(|b| parse_embedded_object(&b.data)).collect();
    let entries = entries?;
    if entries.len() != weight {
        return Err(AltiumFormatError::RecordCountMismatch {
            section: "EmbeddedObjectStream".to_owned(),
            expected: weight,
            actual: entries.len(),
        });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_io::BinaryWriter;

    fn make_envelope(id: &str, inner_flags: u8, inner_data: &[u8]) -> Vec<u8> {
        let mut w = BinaryWriter::new();
        w.write_u8(0xD0);
        w.write_u8(id.len() as u8);
        w.write_bytes(id.as_bytes());
        let inner_header = (inner_data.len() as i32) | ((inner_flags as i32) << 24);
        w.write_i32_le(inner_header);
        w.write_bytes(inner_data);
        w.finish()
    }

    #[test]
    fn parse_single_envelope_text() {
        let inner = b"hello";
        let data = make_envelope("comp1", 0x00, inner);
        let obj = parse_embedded_object(&data).unwrap();
        assert_eq!(obj.id, "comp1");
        assert_eq!(obj.inner_format, BlockFormat::Text);
        assert_eq!(obj.inner_data, inner);
    }

    #[test]
    fn parse_single_envelope_binary() {
        let inner = b"\x01\x02\x03";
        let data = make_envelope("item", 0x01, inner);
        let obj = parse_embedded_object(&data).unwrap();
        assert_eq!(obj.id, "item");
        assert_eq!(obj.inner_format, BlockFormat::Binary);
        assert_eq!(obj.inner_data, inner);
    }

    #[test]
    fn wrong_tag_returns_error() {
        let mut data = make_envelope("x", 0x00, b"");
        data[0] = 0xE3;
        let err = parse_embedded_object(&data).unwrap_err();
        assert!(matches!(err, AltiumFormatError::InvalidEmbeddedObject(_)));
    }

    #[test]
    fn truncated_data_returns_error() {
        let err = parse_embedded_object(&[0xD0]).unwrap_err();
        assert!(matches!(err, AltiumFormatError::BinaryReadPastEnd { .. }));
    }

    #[test]
    fn parse_stream_with_weight() {
        // Build header block (text): |RECORD=0|Weight=2|\0
        let header_data = b"|RECORD=0|Weight=2|\0";
        let header_block = Block {
            format: BlockFormat::Text,
            data: header_data.to_vec(),
        };
        // Build two entry blocks (binary format internally, but Block format must be binary
        // for the envelope to pass through parse_embedded_object which reads raw bytes)
        let entry1 = make_envelope("e1", 0x00, b"abc");
        let entry2 = make_envelope("e2", 0x01, b"\xFF");
        let block1 = Block { format: BlockFormat::Binary, data: entry1 };
        let block2 = Block { format: BlockFormat::Binary, data: entry2 };
        let blocks = vec![header_block, block1, block2];
        let entries = parse_embedded_object_stream(&blocks).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, "e1");
        assert_eq!(entries[1].id, "e2");
    }

    #[test]
    fn weight_mismatch_returns_error() {
        let header_data = b"|Weight=3|\0";
        let header_block = Block {
            format: BlockFormat::Text,
            data: header_data.to_vec(),
        };
        let entry = make_envelope("e1", 0x00, b"x");
        let block1 = Block { format: BlockFormat::Binary, data: entry };
        let blocks = vec![header_block, block1];
        let err = parse_embedded_object_stream(&blocks).unwrap_err();
        assert!(matches!(err, AltiumFormatError::RecordCountMismatch { .. }));
    }

    #[test]
    fn empty_blocks_returns_error() {
        let err = parse_embedded_object_stream(&[]).unwrap_err();
        assert!(matches!(err, AltiumFormatError::InvalidEmbeddedObject(_)));
    }
}

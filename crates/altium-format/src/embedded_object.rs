//! Layer 4 parser for the embedded object envelope format.
//! Each entry in a Storage or sidecar block stream is a 0xD0-tagged envelope:
//! tag(1) + id_length(1) + id(N) + compressed_length(4) + zlib_data(M).
//! The 4-byte compressed_length field stores the size of the zlib-compressed
//! payload. The decompressed bytes are stored in `inner_data`.
//! `parse_embedded_object_stream` consumes the header block's params internally
//! so callers never receive a partially-consumed `ParameterCollection`.
use std::io::{Read, Write};

use altium_format_types::constants::parsing::{BLOCK_SIZE_MASK, INSTRUCTION_BINARY};
use altium_format_types::constants::record_structure::{HEADER, RECORD, WEIGHT};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

use crate::binary_io::{BinaryReader, BinaryWriter};
use crate::block_stream::{Block, BlockFormat, write_binary_block, write_text_block};
use crate::param_collection::ParameterCollection;
use crate::{AltiumFormatError, Result};

#[derive(Debug)]
pub(crate) struct EmbeddedObject {
    pub(crate) id: String,
    pub(crate) inner_data: Vec<u8>,
}

// Parses a single 0xD0-tagged embedded object envelope from a binary block payload.
// The inner payload is zlib-decompressed before being stored in `inner_data`.
pub(crate) fn parse_embedded_object(data: &[u8]) -> Result<EmbeddedObject> {
    let mut reader = BinaryReader::new(data);
    let tag = reader.read_u8()?;
    if tag != INSTRUCTION_BINARY {
        return Err(AltiumFormatError::InvalidEmbeddedObject(format!(
            "expected {INSTRUCTION_BINARY:#04x} tag, got {tag:#04x}"
        )));
    }
    let id_len = reader.read_u8()? as usize;
    let id_bytes = reader.read_bytes(id_len)?;
    let (id_cow, _encoding_used, _had_replacements) =
        encoding_rs::WINDOWS_1252.decode(id_bytes);
    let id = id_cow.into_owned();
    let compressed_size = (reader.read_i32_le()? & BLOCK_SIZE_MASK as i32) as usize;
    let compressed_bytes = reader.read_bytes(compressed_size)?;
    let inner_data = zlib_decompress(compressed_bytes)?;
    reader.assert_exhausted()?;
    Ok(EmbeddedObject { id, inner_data })
}

fn zlib_decompress(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).map_err(|e| {
        AltiumFormatError::InvalidEmbeddedObject(format!("zlib decompress failed: {e}"))
    })?;
    Ok(out)
}

// Parses the Storage-style block stream: block 0 = header params, blocks 1..N = entries.
// Header params (RECORD, Weight) are consumed internally; callers receive only the entries.
pub(crate) fn parse_embedded_object_stream(blocks: &[Block]) -> Result<Vec<EmbeddedObject>> {
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
    params.remove_optional::<i32>(RECORD)?;
    // HEADER=<stream_name> appears in pin sidecar stream headers; consume without checking.
    params.remove_optional::<String>(HEADER)?;
    // Some legacy files omit Weight from Storage-style headers.
    let weight = params
        .remove_optional::<usize>(WEIGHT)?
        .unwrap_or(blocks.len().saturating_sub(1));
    params.assert_exhausted()?;
    let entries: Result<Vec<EmbeddedObject>> = blocks[1..]
        .iter()
        .map(|b| parse_embedded_object(&b.data))
        .collect();
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

/// Compresses data using zlib (standard deflate with zlib header).
pub(crate) fn zlib_compress(data: &[u8]) -> Result<Vec<u8>> {
    // Altium's serializer uses Ionic.Zlib with default compression settings.
    // Use flate2 default level to match on-disk byte patterns for roundtrips.
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(data).map_err(|e| {
        AltiumFormatError::InvalidEmbeddedObject(format!("zlib compress failed: {e}"))
    })?;
    enc.finish().map_err(|e| {
        AltiumFormatError::InvalidEmbeddedObject(format!("zlib compress finish failed: {e}"))
    })
}

/// Serializes a single embedded object into the 0xD0-tagged envelope format.
/// The inner_data is zlib-compressed before being wrapped.
pub(crate) fn serialize_embedded_object(id: &str, inner_data: &[u8]) -> Result<Vec<u8>> {
    let compressed = zlib_compress(inner_data)?;
    let mut w = BinaryWriter::new();
    w.write_u8(INSTRUCTION_BINARY);
    let (id_bytes, _encoding_used, _had_unmappable) =
        encoding_rs::WINDOWS_1252.encode(id);
    w.write_u8(id_bytes.len() as u8);
    w.write_bytes(&id_bytes);
    w.write_i32_le(compressed.len() as i32);
    w.write_bytes(&compressed);
    Ok(w.finish())
}

/// Serializes a complete embedded object stream: header text block + entry binary blocks.
/// The header block contains RECORD=0, HEADER=<header_name>, Weight=<count>.
/// Each entry is a 0xD0-tagged envelope in a binary block.
pub(crate) fn serialize_embedded_object_stream(
    header_name: &str,
    entries: &[(String, Vec<u8>)],
) -> Result<Vec<u8>> {
    // Build header params (no RECORD=0 — original files omit it from sidecar/storage headers)
    let mut params = ParameterCollection::new();
    params.insert(HEADER, header_name.to_owned());
    params.insert(WEIGHT, entries.len().to_string());
    let header_bytes = params.to_bytes();

    let mut stream = write_text_block(&header_bytes);

    // Write each entry as a binary block containing a 0xD0 envelope
    for (id, inner_data) in entries {
        let envelope = serialize_embedded_object(id, inner_data)?;
        stream.extend_from_slice(&write_binary_block(&envelope));
    }

    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_stream::parse_blocks;
    use altium_format_types::constants::parsing::INSTRUCTION_FILE_STREAM;

    fn make_envelope(id: &str, inner_data: &[u8]) -> Vec<u8> {
        let compressed = zlib_compress(inner_data).unwrap();
        let (id_bytes, _, _) = encoding_rs::WINDOWS_1252.encode(id);
        let mut w = BinaryWriter::new();
        w.write_u8(INSTRUCTION_BINARY);
        w.write_u8(id_bytes.len() as u8);
        w.write_bytes(&id_bytes);
        w.write_i32_le(compressed.len() as i32);
        w.write_bytes(&compressed);
        w.finish()
    }

    #[test]
    fn parse_single_envelope_decompresses() {
        let inner = b"hello world";
        let data = make_envelope("comp1", inner);
        let obj = parse_embedded_object(&data).unwrap();
        assert_eq!(obj.id, "comp1");
        assert_eq!(obj.inner_data, inner);
    }

    #[test]
    fn parse_binary_inner_decompresses() {
        let inner = b"\x01\x02\x03";
        let data = make_envelope("item", inner);
        let obj = parse_embedded_object(&data).unwrap();
        assert_eq!(obj.id, "item");
        assert_eq!(obj.inner_data, inner);
    }

    #[test]
    fn wrong_tag_returns_error() {
        let mut data = make_envelope("x", b"");
        data[0] = INSTRUCTION_FILE_STREAM;
        let err = parse_embedded_object(&data).unwrap_err();
        assert!(matches!(err, AltiumFormatError::InvalidEmbeddedObject(_)));
    }

    #[test]
    fn truncated_data_returns_error() {
        let err = parse_embedded_object(&[INSTRUCTION_BINARY]).unwrap_err();
        assert!(matches!(err, AltiumFormatError::BinaryReadPastEnd { .. }));
    }

    #[test]
    fn parse_stream_with_weight() {
        let header_data = b"|RECORD=0|Weight=2|\0";
        let header_block = Block {
            format: BlockFormat::Text,
            data: header_data.to_vec(),
        };
        let entry1 = make_envelope("e1", b"abc");
        let entry2 = make_envelope("e2", b"\x01\x02");
        let block1 = Block {
            format: BlockFormat::Binary,
            data: entry1,
        };
        let block2 = Block {
            format: BlockFormat::Binary,
            data: entry2,
        };
        let blocks = vec![header_block, block1, block2];
        let entries = parse_embedded_object_stream(&blocks).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, "e1");
        assert_eq!(entries[0].inner_data, b"abc");
        assert_eq!(entries[1].id, "e2");
        assert_eq!(entries[1].inner_data, b"\x01\x02");
    }

    #[test]
    fn weight_mismatch_returns_error() {
        let header_data = b"|Weight=3|\0";
        let header_block = Block {
            format: BlockFormat::Text,
            data: header_data.to_vec(),
        };
        let entry = make_envelope("e1", b"x");
        let block1 = Block {
            format: BlockFormat::Binary,
            data: entry,
        };
        let blocks = vec![header_block, block1];
        let err = parse_embedded_object_stream(&blocks).unwrap_err();
        assert!(matches!(err, AltiumFormatError::RecordCountMismatch { .. }));
    }

    #[test]
    fn parse_stream_without_weight_uses_block_count() {
        let header_data = b"|HEADER=Icon storage|\0";
        let header_block = Block {
            format: BlockFormat::Text,
            data: header_data.to_vec(),
        };
        let blocks = vec![header_block];
        let entries = parse_embedded_object_stream(&blocks).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn empty_blocks_returns_error() {
        let err = parse_embedded_object_stream(&[]).unwrap_err();
        assert!(matches!(err, AltiumFormatError::InvalidEmbeddedObject(_)));
    }

    #[test]
    fn parse_windows_1252_id() {
        // Build an envelope with a Windows-1252 encoded ID containing
        // bytes that are invalid UTF-8: e-acute (0xE9), euro sign (0x80).
        let id_w1252: &[u8] = &[0x63, 0x61, 0x66, 0xE9, 0x80]; // "caf\xE9\x80"
        let inner = b"data";
        let compressed = zlib_compress(inner).unwrap();
        let mut w = BinaryWriter::new();
        w.write_u8(INSTRUCTION_BINARY);
        w.write_u8(id_w1252.len() as u8);
        w.write_bytes(id_w1252);
        w.write_i32_le(compressed.len() as i32);
        w.write_bytes(&compressed);
        let data = w.finish();

        let obj = parse_embedded_object(&data).unwrap();
        // Windows-1252 0xE9 = e-acute (U+00E9), 0x80 = euro sign (U+20AC)
        assert_eq!(obj.id, "caf\u{00E9}\u{20AC}");
        assert_eq!(obj.inner_data, inner);
    }

    // ── Serialization roundtrip tests ──────────────────────────────────

    #[test]
    fn serialize_embedded_object_roundtrips() {
        let inner = b"test data payload";
        let envelope = serialize_embedded_object("myid", inner).unwrap();
        let obj = parse_embedded_object(&envelope).unwrap();
        assert_eq!(obj.id, "myid");
        assert_eq!(obj.inner_data, inner);
    }

    #[test]
    fn serialize_embedded_object_stream_roundtrips() {
        let entries = vec![
            ("e1".to_owned(), b"abc".to_vec()),
            ("e2".to_owned(), b"\x01\x02\x03".to_vec()),
        ];
        let stream_bytes = serialize_embedded_object_stream("TestStream", &entries).unwrap();
        let blocks = parse_blocks(&stream_bytes).unwrap();
        let parsed = parse_embedded_object_stream(&blocks).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].id, "e1");
        assert_eq!(parsed[0].inner_data, b"abc");
        assert_eq!(parsed[1].id, "e2");
        assert_eq!(parsed[1].inner_data, b"\x01\x02\x03");
    }

    #[test]
    fn zlib_compress_decompress_roundtrip() {
        let data = b"Hello, world! This is test data for zlib compression.";
        let compressed = zlib_compress(data).unwrap();
        assert!(compressed.len() > 0);
        let mut decoder = flate2::read::ZlibDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn serialize_empty_stream() {
        let entries: Vec<(String, Vec<u8>)> = vec![];
        let stream_bytes = serialize_embedded_object_stream("Empty", &entries).unwrap();
        let blocks = parse_blocks(&stream_bytes).unwrap();
        // Header block should have Weight=0
        assert_eq!(blocks.len(), 1); // only header, no entries
        let mut params = ParameterCollection::from_bytes(&blocks[0].data).unwrap();
        let weight: usize = params.remove_required("Weight").unwrap();
        assert_eq!(weight, 0);
    }
}

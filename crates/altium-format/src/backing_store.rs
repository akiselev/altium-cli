//! Backing store types for the v2 API.
//!
//! These types represent the raw parsed data from Altium files before it is
//! interpreted into domain-specific records. The backing store preserves the
//! original bytes so that unmodified records can be written back identically
//! (identity write-back).
//!
//! Key concepts:
//! - **`RecordOrigin`**: Either parameter-based (schematic) or binary (PCB)
//! - **`RecordNode`**: A single record with its origin, dirty tracking, and
//!   original snapshot for identity writes

use serde::{Deserialize, Serialize};

use crate::parameters::ParameterCollection;

// ---------------------------------------------------------------------------
// FieldSpan — describes a region within a binary block
// ---------------------------------------------------------------------------

/// Describes a contiguous region within a binary block.
///
/// Used by `BinaryOrigin` to track where individual fields were parsed from,
/// enabling targeted writes back into the raw block.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldSpan {
    /// Byte offset from the start of the containing block.
    pub offset: usize,
    /// Size in bytes of this field.
    pub size: usize,
}

impl FieldSpan {
    /// Creates a new field span.
    pub fn new(offset: usize, size: usize) -> Self {
        Self { offset, size }
    }

    /// Returns the exclusive end offset (`offset + size`).
    pub fn end(&self) -> usize {
        self.offset + self.size
    }

    /// Extracts the slice from the given data corresponding to this span.
    pub fn slice<'a>(&self, data: &'a [u8]) -> &'a [u8] {
        &data[self.offset..self.end()]
    }

    /// Extracts a mutable slice from the given data corresponding to this span.
    pub fn slice_mut<'a>(&self, data: &'a mut [u8]) -> &'a mut [u8] {
        &mut data[self.offset..self.end()]
    }
}

// ---------------------------------------------------------------------------
// ParamOrigin — backing store for parameter-based records (schematic)
// ---------------------------------------------------------------------------

/// Backing store for parameter-based records (used by schematic files).
///
/// Schematic records in Altium are stored as pipe-delimited key-value strings.
/// This struct preserves both the parsed collection and the raw text for
/// identity write-back.
#[derive(Clone, Debug)]
pub struct ParamOrigin {
    /// Parsed parameter collection for structured access.
    pub params: ParameterCollection,
    /// Raw record text as originally read from the file.
    pub raw_record_text: String,
}

impl ParamOrigin {
    /// Creates a new `ParamOrigin` from a raw parameter string.
    ///
    /// The string is parsed into a `ParameterCollection` and also stored
    /// verbatim for identity write-back.
    pub fn new(raw_text: &str) -> Self {
        Self {
            params: ParameterCollection::from_string(raw_text),
            raw_record_text: raw_text.to_string(),
        }
    }

    /// Creates a `ParamOrigin` from an existing `ParameterCollection`.
    ///
    /// The raw text is generated from the collection's `to_param_string()`.
    pub fn from_params(params: ParameterCollection) -> Self {
        let raw = params.to_param_string();
        Self {
            params,
            raw_record_text: raw,
        }
    }

    /// Returns the raw text bytes for snapshot purposes.
    pub fn snapshot_bytes(&self) -> Vec<u8> {
        self.raw_record_text.as_bytes().to_vec()
    }
}

// Custom serde: serialize the raw_record_text as the representation,
// and reconstruct the ParameterCollection on deserialization.
impl Serialize for ParamOrigin {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.raw_record_text.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ParamOrigin {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Ok(Self::new(&raw))
    }
}

// ---------------------------------------------------------------------------
// BinaryOrigin — backing store for binary records (PCB)
// ---------------------------------------------------------------------------

/// Backing store for binary records (used by PCB files).
///
/// PCB primitives in Altium are stored as fixed-layout binary blocks. This
/// struct preserves the raw block and records where individual fields were
/// parsed from so they can be written back with targeted modifications.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BinaryOrigin {
    /// Raw binary block as originally read from the file.
    pub raw_block: Vec<u8>,
    /// Field spans describing where parsed fields live within `raw_block`.
    pub field_spans: Vec<FieldSpan>,
}

impl BinaryOrigin {
    /// Creates a new `BinaryOrigin` with no field spans.
    pub fn new(raw_block: Vec<u8>) -> Self {
        Self {
            raw_block,
            field_spans: Vec::new(),
        }
    }

    /// Creates a new `BinaryOrigin` with the given field spans.
    pub fn with_spans(raw_block: Vec<u8>, field_spans: Vec<FieldSpan>) -> Self {
        Self {
            raw_block,
            field_spans,
        }
    }

    /// Returns the raw block bytes for snapshot purposes.
    pub fn snapshot_bytes(&self) -> Vec<u8> {
        self.raw_block.clone()
    }
}

// ---------------------------------------------------------------------------
// RecordOrigin — unified enum over param-based and binary-based records
// ---------------------------------------------------------------------------

/// The origin of a record's data: either parameter-based (schematic) or
/// binary (PCB).
///
/// This enum is the core of the backing-store architecture. Every record
/// in the v2 API stores its raw data here, enabling identity write-back
/// of unmodified records and targeted modification of changed fields.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RecordOrigin {
    /// Parameter-based record (schematic files).
    Param(ParamOrigin),
    /// Binary record (PCB files).
    Binary(BinaryOrigin),
}

impl RecordOrigin {
    /// Returns a reference to the `ParamOrigin` if this is the `Param` variant.
    pub fn as_param(&self) -> Option<&ParamOrigin> {
        match self {
            RecordOrigin::Param(p) => Some(p),
            _ => None,
        }
    }

    /// Returns a mutable reference to the `ParamOrigin` if this is the `Param` variant.
    pub fn as_param_mut(&mut self) -> Option<&mut ParamOrigin> {
        match self {
            RecordOrigin::Param(p) => Some(p),
            _ => None,
        }
    }

    /// Returns a reference to the `BinaryOrigin` if this is the `Binary` variant.
    pub fn as_binary(&self) -> Option<&BinaryOrigin> {
        match self {
            RecordOrigin::Binary(b) => Some(b),
            _ => None,
        }
    }

    /// Returns a mutable reference to the `BinaryOrigin` if this is the `Binary` variant.
    pub fn as_binary_mut(&mut self) -> Option<&mut BinaryOrigin> {
        match self {
            RecordOrigin::Binary(b) => Some(b),
            _ => None,
        }
    }

    /// Returns a reference to the `ParamOrigin`, panicking if this is not the
    /// `Param` variant.
    pub fn param(&self) -> &ParamOrigin {
        self.as_param().expect("expected Param origin")
    }

    /// Returns a mutable reference to the `ParamOrigin`, panicking if this is
    /// not the `Param` variant.
    pub fn param_mut(&mut self) -> &mut ParamOrigin {
        self.as_param_mut().expect("expected Param origin")
    }

    /// Returns a reference to the `BinaryOrigin`, panicking if this is not the
    /// `Binary` variant.
    pub fn binary(&self) -> &BinaryOrigin {
        self.as_binary().expect("expected Binary origin")
    }

    /// Returns a mutable reference to the `BinaryOrigin`, panicking if this is
    /// not the `Binary` variant.
    pub fn binary_mut(&mut self) -> &mut BinaryOrigin {
        self.as_binary_mut().expect("expected Binary origin")
    }

    /// Returns `true` if this origin is the `Binary` variant.
    pub fn is_binary(&self) -> bool {
        matches!(self, RecordOrigin::Binary(_))
    }

    /// Returns the snapshot bytes for this origin.
    fn snapshot_bytes(&self) -> Vec<u8> {
        match self {
            RecordOrigin::Param(p) => p.snapshot_bytes(),
            RecordOrigin::Binary(b) => b.snapshot_bytes(),
        }
    }
}

// ---------------------------------------------------------------------------
// RecordNode — a single record with dirty tracking
// ---------------------------------------------------------------------------

/// A single parsed record with its backing store and dirty tracking.
///
/// `RecordNode` is the fundamental unit of the v2 data model. It wraps a
/// `RecordOrigin` with:
/// - A `key` (record type ID)
/// - An `original_snapshot` of the bytes at parse time
/// - A `dirty` flag indicating whether the record has been modified
///
/// Unmodified records can be written back byte-for-byte using
/// `snapshot_bytes()`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordNode {
    /// Record type identifier (e.g., RECORD=1 for component).
    pub key: u8,
    /// The backing store (param-based or binary).
    pub origin: RecordOrigin,
    /// Original bytes at parse time, used for identity write-back.
    pub original_snapshot: Vec<u8>,
    /// Whether this record has been modified since parsing.
    pub dirty: bool,
    /// Origin stream name for formats with multiple record streams (e.g. SchDoc).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_name: Option<String>,
}

impl RecordNode {
    /// Creates a new `RecordNode` from a record key and origin.
    ///
    /// The original snapshot is captured from the origin at construction time
    /// and the dirty flag is initially `false`.
    pub fn new(key: u8, origin: RecordOrigin) -> Self {
        let snapshot = origin.snapshot_bytes();
        Self {
            key,
            origin,
            original_snapshot: snapshot,
            dirty: false,
            stream_name: None,
        }
    }

    /// Marks this record as dirty (modified).
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Returns whether this record has been modified since parsing.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Returns the original snapshot bytes for identity write-back.
    ///
    /// If the record has not been modified, these bytes can be written
    /// back to produce a byte-identical output file.
    pub fn snapshot_bytes(&self) -> &[u8] {
        &self.original_snapshot
    }
}

// ---------------------------------------------------------------------------
// PcbPrimitiveRef — reference to a PCB primitive by type and index
// ---------------------------------------------------------------------------

/// A reference to a PCB primitive within a footprint, identified by
/// its type ID and index.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PcbPrimitiveRef {
    /// The primitive's type identifier (e.g., track, pad, arc).
    pub type_id: u8,
    /// Index of this primitive within its type group.
    pub index: usize,
}

impl PcbPrimitiveRef {
    /// Creates a new `PcbPrimitiveRef`.
    pub fn new(type_id: u8, index: usize) -> Self {
        Self { type_id, index }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_node_dirty_tracking() {
        // Create a param-based record node.
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=1|NAME=Test|"));
        let mut node = RecordNode::new(1, origin);

        // Initially not dirty.
        assert!(!node.is_dirty());

        // Snapshot should match the raw text.
        assert_eq!(node.snapshot_bytes(), b"|RECORD=1|NAME=Test|");

        // Mark dirty.
        node.mark_dirty();
        assert!(node.is_dirty());

        // Snapshot is still the original bytes (unchanged).
        assert_eq!(node.snapshot_bytes(), b"|RECORD=1|NAME=Test|");

        // Verify we can modify the origin without affecting the snapshot.
        node.origin.param_mut().params.add("VALUE", "100");
        assert_eq!(node.snapshot_bytes(), b"|RECORD=1|NAME=Test|");
    }

    #[test]
    fn param_origin_access() {
        // Create a record with param origin.
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=4|DESIGNATOR=R1|VALUE=10k|"));

        // Access through RecordOrigin helpers.
        assert!(origin.as_param().is_some());
        assert!(origin.as_binary().is_none());

        let param = origin.param();
        assert_eq!(param.params.get("RECORD").unwrap().as_int_or(0), 4);
        assert_eq!(param.params.get("DESIGNATOR").unwrap().as_str(), "R1");
        assert_eq!(param.params.get("VALUE").unwrap().as_str(), "10k");

        // Mutate through RecordOrigin helpers.
        let mut origin = origin;
        origin.param_mut().params.add("COMMENT", "Test resistor");
        assert_eq!(
            origin.param().params.get("COMMENT").unwrap().as_str(),
            "Test resistor"
        );
    }

    #[test]
    fn binary_origin_field_span() {
        // Create a binary origin with some data and field spans.
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let spans = vec![
            FieldSpan::new(0, 2), // bytes [0x01, 0x02]
            FieldSpan::new(2, 4), // bytes [0x03, 0x04, 0x05, 0x06]
            FieldSpan::new(6, 2), // bytes [0x07, 0x08]
        ];
        let origin = BinaryOrigin::with_spans(data.clone(), spans);

        // Verify field spans extract correctly.
        assert_eq!(
            origin.field_spans[0].slice(&origin.raw_block),
            &[0x01, 0x02]
        );
        assert_eq!(
            origin.field_spans[1].slice(&origin.raw_block),
            &[0x03, 0x04, 0x05, 0x06]
        );
        assert_eq!(
            origin.field_spans[2].slice(&origin.raw_block),
            &[0x07, 0x08]
        );

        // Verify end() calculation.
        assert_eq!(origin.field_spans[0].end(), 2);
        assert_eq!(origin.field_spans[1].end(), 6);
        assert_eq!(origin.field_spans[2].end(), 8);

        // Verify RecordOrigin access.
        let record_origin = RecordOrigin::Binary(origin);
        assert!(record_origin.as_binary().is_some());
        assert!(record_origin.as_param().is_none());
        assert_eq!(record_origin.binary().raw_block, data);

        // Verify mutable field span access.
        let mut record_origin = record_origin;
        let binary = record_origin.binary_mut();
        let span = &binary.field_spans[0];
        let slice = span.slice_mut(&mut binary.raw_block);
        slice[0] = 0xFF;
        assert_eq!(record_origin.binary().raw_block[0], 0xFF);
    }

    #[test]
    fn param_origin_serde_roundtrip() {
        let original = ParamOrigin::new("|RECORD=1|NAME=Test|");
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ParamOrigin = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.raw_record_text, original.raw_record_text);
        assert_eq!(deserialized.params.get("NAME").unwrap().as_str(), "Test");
    }

    #[test]
    fn binary_origin_serde_roundtrip() {
        let original = BinaryOrigin::with_spans(
            vec![0x01, 0x02, 0x03],
            vec![FieldSpan::new(0, 2), FieldSpan::new(2, 1)],
        );
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: BinaryOrigin = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.raw_block, original.raw_block);
        assert_eq!(deserialized.field_spans.len(), 2);
        assert_eq!(deserialized.field_spans[0].offset, 0);
        assert_eq!(deserialized.field_spans[1].offset, 2);
    }

    #[test]
    fn record_node_serde_roundtrip() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=1|VALUE=100|"));
        let node = RecordNode::new(1, origin);
        let json = serde_json::to_string(&node).unwrap();
        let deserialized: RecordNode = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.key, 1);
        assert!(!deserialized.is_dirty());
        assert_eq!(deserialized.snapshot_bytes(), b"|RECORD=1|VALUE=100|");
    }

    #[test]
    fn pcb_primitive_ref_equality() {
        let a = PcbPrimitiveRef::new(4, 0);
        let b = PcbPrimitiveRef::new(4, 0);
        let c = PcbPrimitiveRef::new(5, 0);

        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}

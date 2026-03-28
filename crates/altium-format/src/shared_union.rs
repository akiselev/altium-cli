//! Parser and serializer for SharedUnion streams.
//!
//! Used by both PcbLib (`/<Footprint>/SharedUnion`) and PcbDoc (`/SharedUnions/Data`).
//!
//! Format:
//! - u32 entry_count
//! - For each entry:
//!   - u32 header_len
//!   - header_len bytes: pipe-delimited params (no leading `|`)
//!   - If HIDDENPRIMITIVESCOUNT > 0: N more len-prefixed param blocks (no leading `|`)
//!   - If PRIMITIVESCOUNT > 0: children are inline REF{i}INDEX + REF{i}OBJID in header

use altium_format_types::PcbObjectId;

use crate::binary_io::{BinaryReader, BinaryWriter};
use crate::param_collection::ParameterCollection;
use crate::{AltiumFormatError, Result};

#[derive(Clone)]
pub(crate) struct SharedUnionEntry {
    pub(crate) primitive_index: i32,
    pub(crate) object_id: PcbObjectId,
    pub(crate) children: SharedUnionChildren,
}

#[derive(Clone)]
pub(crate) enum SharedUnionChildren {
    /// Hidden inline primitives (HIDDENPRIMITIVESCOUNT > 0).
    /// Stored as ParameterCollection since these are full primitive descriptions.
    Hidden(Vec<ParameterCollection>),
    /// References to Data stream primitives (PRIMITIVESCOUNT > 0).
    Referenced(Vec<SharedUnionRef>),
    /// No children (both counts are 0).
    None,
}

#[derive(Clone)]
pub(crate) struct SharedUnionRef {
    pub(crate) index: i32,
    pub(crate) object_id: PcbObjectId,
}

/// Parses a SharedUnion stream (used by both PcbLib and PcbDoc).
pub(crate) fn parse_shared_union_stream(data: &[u8]) -> Result<Vec<SharedUnionEntry>> {
    let mut reader = BinaryReader::new(data);
    let entry_count = reader.read_u32_le()? as usize;
    let mut entries = Vec::with_capacity(entry_count);

    for entry_idx in 0..entry_count {
        let header_len = reader.read_u32_le()? as usize;
        let header_bytes = reader.read_bytes(header_len)?;

        // The format omits the leading `|`, so we prepend it for ParameterCollection::from_bytes.
        let mut prefixed = Vec::with_capacity(1 + header_bytes.len());
        prefixed.push(b'|');
        prefixed.extend_from_slice(header_bytes);
        let mut params = ParameterCollection::from_bytes(&prefixed).map_err(|e| {
            AltiumFormatError::WithContext {
                context: format!("SharedUnion entry #{entry_idx} header"),
                source: Box::new(e),
            }
        })?;

        let primitive_index: i32 = params.remove_required("PRIMITIVEINDEX")?;
        let object_id_str: String = params.remove_required("OBJECTID")?;
        let object_id =
            PcbObjectId::from_primitive_object_id_str(&object_id_str).ok_or_else(|| {
                AltiumFormatError::InvalidParamValue {
                    key: "OBJECTID".to_owned(),
                    detail: format!("unknown object ID string in SharedUnion: '{object_id_str}'"),
                }
            })?;

        let hidden_count: i32 = params.remove_with_default("HIDDENPRIMITIVESCOUNT", 0)?;
        let ref_count: i32 = params.remove_with_default("PRIMITIVESCOUNT", 0)?;

        let children = if hidden_count > 0 {
            let mut hidden = Vec::with_capacity(hidden_count as usize);
            // Extract REF*INDEX and REF*OBJID from header before assert_exhausted
            // (they may also appear in the header for hidden-mode unions).
            drain_ref_keys(&mut params);
            params
                .assert_exhausted()
                .map_err(|e| AltiumFormatError::WithContext {
                    context: format!("SharedUnion entry #{entry_idx} header (hidden mode)"),
                    source: Box::new(e),
                })?;

            for child_idx in 0..hidden_count {
                let child_len = reader.read_u32_le()? as usize;
                let child_bytes = reader.read_bytes(child_len)?;
                // Hidden primitive blocks also omit the leading `|`.
                let mut child_prefixed = Vec::with_capacity(1 + child_bytes.len());
                child_prefixed.push(b'|');
                child_prefixed.extend_from_slice(child_bytes);
                let child_params =
                    ParameterCollection::from_bytes(&child_prefixed).map_err(|e| {
                        AltiumFormatError::WithContext {
                            context: format!(
                                "SharedUnion entry #{entry_idx} hidden primitive #{child_idx}"
                            ),
                            source: Box::new(e),
                        }
                    })?;
                // We do NOT call assert_exhausted on hidden primitives — they are full
                // primitive descriptions with many keys that we store as-is.
                hidden.push(child_params);
            }
            SharedUnionChildren::Hidden(hidden)
        } else if ref_count > 0 {
            let mut refs = Vec::with_capacity(ref_count as usize);
            for i in 0..ref_count {
                let idx_key = format!("REF{}INDEX", i);
                let objid_key = format!("REF{}OBJID", i);
                let ref_index: i32 = params.remove_required(&idx_key)?;
                let ref_objid_str: String = params.remove_required(&objid_key)?;
                let ref_objid = PcbObjectId::from_primitive_object_id_str(&ref_objid_str)
                    .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                        key: objid_key.clone(),
                        detail: format!(
                            "unknown object ID string in SharedUnion ref: '{ref_objid_str}'"
                        ),
                    })?;
                refs.push(SharedUnionRef {
                    index: ref_index,
                    object_id: ref_objid,
                });
            }
            params
                .assert_exhausted()
                .map_err(|e| AltiumFormatError::WithContext {
                    context: format!("SharedUnion entry #{entry_idx} header (referenced mode)"),
                    source: Box::new(e),
                })?;
            SharedUnionChildren::Referenced(refs)
        } else {
            drain_ref_keys(&mut params);
            params
                .assert_exhausted()
                .map_err(|e| AltiumFormatError::WithContext {
                    context: format!("SharedUnion entry #{entry_idx} header (no children)"),
                    source: Box::new(e),
                })?;
            SharedUnionChildren::None
        };

        entries.push(SharedUnionEntry {
            primitive_index,
            object_id,
            children,
        });
    }
    reader.assert_exhausted()?;
    Ok(entries)
}

/// Removes any REF*INDEX / REF*OBJID keys from the collection.
/// These may be present even when PRIMITIVESCOUNT=0 (e.g. in hidden-mode or empty unions).
fn drain_ref_keys(params: &mut ParameterCollection) {
    // Consume any REF{N}INDEX and REF{N}OBJID keys that remain.
    let _ = params.remove_prefixed("REF");
}

/// Serializes SharedUnion entries back to stream bytes.
pub(crate) fn serialize_shared_union_stream(entries: &[SharedUnionEntry]) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    w.write_u32_le(entries.len() as u32);

    for entry in entries {
        let mut params = ParameterCollection::new();
        params.insert("PRIMITIVEINDEX", entry.primitive_index.to_string());
        params.insert("OBJECTID", format!("{}", entry.object_id));

        match &entry.children {
            SharedUnionChildren::Hidden(hidden) => {
                params.insert("HIDDENPRIMITIVESCOUNT", (hidden.len() as i32).to_string());
                // Write header
                let header_bytes = params.to_bytes();
                // Strip the leading `|` since the format omits it.
                let header_payload = strip_leading_pipe(&header_bytes);
                w.write_u32_le(header_payload.len() as u32);
                w.write_bytes(header_payload);
                // Write each hidden primitive
                for child in hidden {
                    let child_bytes = child.to_bytes();
                    let child_payload = strip_leading_pipe(&child_bytes);
                    w.write_u32_le(child_payload.len() as u32);
                    w.write_bytes(child_payload);
                }
            }
            SharedUnionChildren::Referenced(refs) => {
                params.insert("PRIMITIVESCOUNT", (refs.len() as i32).to_string());
                for (i, r) in refs.iter().enumerate() {
                    params.insert(&format!("REF{}INDEX", i), r.index.to_string());
                    params.insert(&format!("REF{}OBJID", i), format!("{}", r.object_id));
                }
                let header_bytes = params.to_bytes();
                let header_payload = strip_leading_pipe(&header_bytes);
                w.write_u32_le(header_payload.len() as u32);
                w.write_bytes(header_payload);
            }
            SharedUnionChildren::None => {
                let header_bytes = params.to_bytes();
                let header_payload = strip_leading_pipe(&header_bytes);
                w.write_u32_le(header_payload.len() as u32);
                w.write_bytes(header_payload);
            }
        }
    }

    w.finish()
}

/// Strips the leading `|` byte from param bytes, since SharedUnion format omits it.
fn strip_leading_pipe(bytes: &[u8]) -> &[u8] {
    if bytes.first() == Some(&b'|') {
        &bytes[1..]
    } else {
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_empty() {
        let data = serialize_shared_union_stream(&[]);
        let entries = parse_shared_union_stream(&data).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn roundtrip_referenced() {
        let entries = vec![SharedUnionEntry {
            primitive_index: 5,
            object_id: PcbObjectId::Region,
            children: SharedUnionChildren::Referenced(vec![
                SharedUnionRef {
                    index: 10,
                    object_id: PcbObjectId::Region,
                },
                SharedUnionRef {
                    index: 11,
                    object_id: PcbObjectId::Pad,
                },
            ]),
        }];

        let data = serialize_shared_union_stream(&entries);
        let parsed = parse_shared_union_stream(&data).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].primitive_index, 5);
        assert!(matches!(parsed[0].object_id, PcbObjectId::Region));
        match &parsed[0].children {
            SharedUnionChildren::Referenced(refs) => {
                assert_eq!(refs.len(), 2);
                assert_eq!(refs[0].index, 10);
                assert!(matches!(refs[0].object_id, PcbObjectId::Region));
                assert_eq!(refs[1].index, 11);
                assert!(matches!(refs[1].object_id, PcbObjectId::Pad));
            }
            _ => panic!("expected Referenced children"),
        }
    }

    #[test]
    fn roundtrip_no_children() {
        let entries = vec![SharedUnionEntry {
            primitive_index: 0,
            object_id: PcbObjectId::Pad,
            children: SharedUnionChildren::None,
        }];

        let data = serialize_shared_union_stream(&entries);
        let parsed = parse_shared_union_stream(&data).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].primitive_index, 0);
        assert!(matches!(parsed[0].children, SharedUnionChildren::None));
    }

    #[test]
    fn parse_empty_stream() {
        // 4 bytes: count = 0
        let data = [0u8; 4];
        let entries = parse_shared_union_stream(&data).unwrap();
        assert!(entries.is_empty());
    }
}

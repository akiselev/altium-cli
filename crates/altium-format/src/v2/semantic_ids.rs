//! Stable semantic IDs for Altium records, groups, and documents.
//!
//! These IDs survive saves, reorders, and round-trips, enabling external
//! tooling (diff, merge, tracking) to identify records across sessions.
//!
//! Three tiers are implemented:
//! - **DID** (Document ID): identifies the document instance
//! - **SGID** (Stream Group ID): identifies a component or footprint group
//! - **RID** (Record Instance ID): identifies an individual record

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::v2::backing_store::RecordOrigin;
use crate::v2::ids::{GroupId, RecordId};
use crate::v2::store::{DocumentStore, GroupMeta};

// ---------------------------------------------------------------------------
// SemanticId type
// ---------------------------------------------------------------------------

/// A stable, hash-based identifier for a document element.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SemanticId(String);

impl SemanticId {
    /// Returns the ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SemanticId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Core hash helper
// ---------------------------------------------------------------------------

/// Compute blake3, truncate to 128 bits, return lowercase hex.
fn blake3_128_hex(input: &str) -> String {
    let hash = blake3::hash(input.as_bytes());
    let bytes = hash.as_bytes();
    // First 16 bytes = 128 bits
    bytes[..16]
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

/// Compute blake3 hash of raw bytes, return lowercase hex (for doc_key).
pub fn blake3_content_hash(data: &[u8]) -> String {
    let hash = blake3::hash(data);
    let bytes = hash.as_bytes();
    bytes[..16]
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

// ---------------------------------------------------------------------------
// ID construction functions
// ---------------------------------------------------------------------------

/// Compute a Document ID: `did:<blake3_128(dtid + "|" + doc_key)>`.
pub fn compute_did(dtid: &str, doc_key: &str) -> SemanticId {
    let input = format!("{}|{}", dtid, doc_key);
    SemanticId(format!("did:{}", blake3_128_hex(&input)))
}

/// Compute a Stream Group ID: `sgid:<blake3_128(DID + "|" + group_key)>`.
pub fn compute_sgid(did: &SemanticId, group_key: &str) -> SemanticId {
    let input = format!("{}|{}", did.0, group_key);
    SemanticId(format!("sgid:{}", blake3_128_hex(&input)))
}

/// Compute a Record Instance ID: `rid:<blake3_128(parent_anchor + "|" + rtid + "|" + record_anchor)>`.
pub fn compute_rid(parent_anchor: &str, rtid: &str, record_anchor: &str) -> SemanticId {
    let input = format!("{}|{}|{}", parent_anchor, rtid, record_anchor);
    SemanticId(format!("rid:{}", blake3_128_hex(&input)))
}

// ---------------------------------------------------------------------------
// Volatile keys excluded from schematic fingerprints
// ---------------------------------------------------------------------------

/// Keys excluded from schematic record fingerprinting (order/position-only).
const VOLATILE_KEYS: &[&str] = &[
    "OWNERINDEX",
    "INDEXINSHEET",
    "LOCATION.X",
    "LOCATION.Y",
    "LOCATION.X_FRAC",
    "LOCATION.Y_FRAC",
];

// ---------------------------------------------------------------------------
// Anchor extraction helpers
// ---------------------------------------------------------------------------

/// Extract the anchor string for a schematic record.
///
/// Priority:
/// 1. UNIQUEID parameter if present and non-empty
/// 2. For pins (RECORD=2): `<owner_anchor>:pin:<pin_index>`
/// 3. Semantic fingerprint (blake3 of canonical sorted params, excluding volatile keys)
fn sch_record_anchor(
    origin: &RecordOrigin,
    record_key: u8,
    owner_anchor: Option<&str>,
    child_index: usize,
) -> String {
    if let Some(param) = origin.as_param() {
        // 1. UNIQUEID
        if let Some(uid) = param.params.get("UNIQUEID") {
            let s = uid.as_str().to_string();
            if !s.is_empty() {
                return s;
            }
        }

        // 2. Pin index anchor
        if record_key == 2 {
            if let Some(owner) = owner_anchor {
                return format!("{}:pin:{}", owner, child_index);
            }
        }

        // 3. Semantic fingerprint
        return sch_semantic_fingerprint(&param.params);
    }

    // Binary schematic records (rare) — use index-based fallback
    format!("data:index:{}", child_index)
}

/// Compute a semantic fingerprint for a schematic param record.
///
/// Canonical sorted params (lowercase keys), excluding volatile keys.
fn sch_semantic_fingerprint(params: &crate::v2::parameters::ParameterCollection) -> String {
    let mut entries: Vec<(String, String)> = Vec::new();
    for (key, value) in params.iter() {
        let key_upper = key.to_uppercase();
        if VOLATILE_KEYS.contains(&key_upper.as_str()) {
            continue;
        }
        entries.push((key.to_lowercase(), value.as_str().to_string()));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let canonical: String = entries
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("|");
    blake3_128_hex(&canonical)
}

/// Extract the anchor string for a PCB record.
///
/// Fallback: `data:index:<primitive_index>:hash:<blake3_128(raw_block)>`
fn pcb_record_anchor(origin: &RecordOrigin, child_index: usize) -> String {
    match origin {
        RecordOrigin::Binary(b) => {
            let hash = blake3_content_hash(&b.raw_block);
            format!("data:index:{}:hash:{}", child_index, hash)
        }
        RecordOrigin::Param(p) => {
            let hash = blake3_content_hash(p.raw_record_text.as_bytes());
            format!("data:index:{}:hash:{}", child_index, hash)
        }
    }
}

/// Extract the group anchor for a SchLib or SchDoc component.
///
/// SchLib: UNIQUEID from component record if present, else canonical LibRef (lowercased).
/// SchDoc: UNIQUEID from component record if present, else semantic fingerprint.
fn sch_group_anchor(
    store: &DocumentStore,
    group_id: GroupId,
    is_schlib: bool,
) -> String {
    let group = store.group(group_id);
    let parent = store.record(group.parent_id());

    if let Some(param) = parent.origin.as_param() {
        // Try UNIQUEID first
        if let Some(uid) = param.params.get("UNIQUEID") {
            let s = uid.as_str().to_string();
            if !s.is_empty() {
                return s;
            }
        }

        if is_schlib {
            // Fallback for SchLib: canonical LibRef (lowercased)
            if let GroupMeta::SchComponent { lib_ref, .. } = &group.meta() {
                return lib_ref.to_lowercase();
            }
        }

        // Fallback: semantic fingerprint of the component record
        return sch_semantic_fingerprint(&param.params);
    }

    // Binary parent (shouldn't happen for schematic) — use group index
    format!("group:{:?}", group_id)
}

/// Extract the group anchor for a PcbLib footprint.
///
/// PATTERN name (lowercased). Duplicate names get `:dup2`, `:dup3` suffixes.
fn pcb_group_anchors(store: &DocumentStore) -> HashMap<GroupId, String> {
    let mut anchors = HashMap::new();
    let mut name_counts: HashMap<String, usize> = HashMap::new();

    for &gid in store.group_ids() {
        let group = store.group(gid);
        let name_lower = match &group.meta() {
            GroupMeta::PcbFootprint { name, .. } => name.to_lowercase(),
            _ => continue,
        };

        let count = name_counts.entry(name_lower.clone()).or_insert(0);
        *count += 1;

        let anchor = if *count == 1 {
            name_lower
        } else {
            format!("{}:dup{}", name_lower, count)
        };

        anchors.insert(gid, anchor);
    }

    anchors
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Compute all semantic IDs for a document and store them.
///
/// Populates `store.document_id`, `store.group_semantic_ids`, and
/// `store.record_semantic_ids`.
pub fn compute_all_ids(store: &mut DocumentStore, dtid: &str, doc_key: &str) {
    let is_pcb = dtid.contains("pcb");
    let is_schlib = dtid == "dtid:schlib";

    // 1. Compute DID
    let did = compute_did(dtid, doc_key);
    store.document_id = Some(did.clone());

    // 2. Compute SGIDs for each group
    let group_ids: Vec<GroupId> = store.group_ids().to_vec();

    // For PCB, compute all anchors at once (handles duplicate names)
    let pcb_anchors = if is_pcb {
        pcb_group_anchors(store)
    } else {
        HashMap::new()
    };

    // Build group anchors and SGIDs
    let mut group_anchor_map: HashMap<GroupId, String> = HashMap::new();
    for &gid in &group_ids {
        let anchor = if is_pcb {
            pcb_anchors.get(&gid).cloned().unwrap_or_default()
        } else {
            sch_group_anchor(store, gid, is_schlib)
        };

        let group_key = if is_pcb {
            format!("footprint:{}", anchor)
        } else {
            format!("component:{}", anchor)
        };

        let sgid = compute_sgid(&did, &group_key);
        group_anchor_map.insert(gid, anchor);
        store.group_semantic_ids.insert(gid, sgid);
    }

    // 3. Compute RIDs for all records
    let mut all_rids: Vec<(RecordId, SemanticId)> = Vec::new();

    for &gid in &group_ids {
        let sgid_str = store
            .group_semantic_ids
            .get(&gid)
            .map(|s| s.0.clone())
            .unwrap_or_default();

        let group_anchor = group_anchor_map.get(&gid).cloned().unwrap_or_default();

        // Parent record
        let parent_id = store.group(gid).parent_id();
        let parent_key = store.record(parent_id).key;
        let parent_origin = store.record(parent_id).origin.clone();
        let parent_rtid = if is_pcb {
            format!("rtid:pcb:object:{}", parent_key)
        } else {
            format!("rtid:sch:record:{}", parent_key)
        };
        let parent_anchor = if is_pcb {
            pcb_record_anchor(&parent_origin, 0)
        } else {
            sch_record_anchor(&parent_origin, parent_key, None, 0)
        };
        let parent_rid = compute_rid(&sgid_str, &parent_rtid, &parent_anchor);
        all_rids.push((parent_id, parent_rid));

        // Child records
        let child_ids: Vec<RecordId> = store.group(gid).child_ids().to_vec();
        for (i, &child_id) in child_ids.iter().enumerate() {
            let child_key = store.record(child_id).key;
            let child_origin = store.record(child_id).origin.clone();
            let rtid = if is_pcb {
                format!("rtid:pcb:object:{}", child_key)
            } else {
                format!("rtid:sch:record:{}", child_key)
            };
            let record_anchor = if is_pcb {
                pcb_record_anchor(&child_origin, i + 1)
            } else {
                sch_record_anchor(&child_origin, child_key, Some(&group_anchor), i)
            };
            let rid = compute_rid(&sgid_str, &rtid, &record_anchor);
            all_rids.push((child_id, rid));
        }
    }

    // Orphan records
    let orphan_ids: Vec<RecordId> = store.orphan_ids().to_vec();
    for (i, &oid) in orphan_ids.iter().enumerate() {
        let key = store.record(oid).key;
        let origin = store.record(oid).origin.clone();
        let rtid = if is_pcb {
            format!("rtid:pcb:object:{}", key)
        } else {
            format!("rtid:sch:record:{}", key)
        };
        let record_anchor = if is_pcb {
            pcb_record_anchor(&origin, i)
        } else {
            sch_record_anchor(&origin, key, None, i)
        };
        let rid = compute_rid(&did.0, &rtid, &record_anchor);
        all_rids.push((oid, rid));
    }

    // 4. Collision detection: append :dup2, :dup3 for duplicates
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut final_rids: Vec<(RecordId, SemanticId)> = Vec::with_capacity(all_rids.len());

    for (record_id, rid) in all_rids {
        let count = seen.entry(rid.0.clone()).or_insert(0);
        *count += 1;
        let final_rid = if *count == 1 {
            rid
        } else {
            SemanticId(format!("{}:dup{}", rid.0, count))
        };
        final_rids.push((record_id, final_rid));
    }

    for (record_id, rid) in final_rids {
        store.record_semantic_ids.insert(record_id, rid);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{ParamOrigin, RecordNode, RecordOrigin};
    use crate::v2::store::{DocumentMeta, DocumentStore, GroupData, GroupMeta};

    #[test]
    fn blake3_128_hex_determinism() {
        let a = blake3_128_hex("hello world");
        let b = blake3_128_hex("hello world");
        assert_eq!(a, b);
        assert_eq!(a.len(), 32); // 16 bytes = 32 hex chars

        // Different input gives different output
        let c = blake3_128_hex("different");
        assert_ne!(a, c);
    }

    #[test]
    fn compute_did_format() {
        let did = compute_did("dtid:schlib", "test-key");
        assert!(did.as_str().starts_with("did:"));
        assert_eq!(did.as_str().len(), 4 + 32); // "did:" + 32 hex chars
    }

    #[test]
    fn compute_sgid_format() {
        let did = compute_did("dtid:schlib", "test-key");
        let sgid = compute_sgid(&did, "component:mycomp");
        assert!(sgid.as_str().starts_with("sgid:"));
        assert_eq!(sgid.as_str().len(), 5 + 32); // "sgid:" + 32 hex chars
    }

    #[test]
    fn compute_rid_format() {
        let rid = compute_rid("sgid:abc", "rtid:sch:record:2", "uid123");
        assert!(rid.as_str().starts_with("rid:"));
        assert_eq!(rid.as_str().len(), 4 + 32); // "rid:" + 32 hex chars
    }

    #[test]
    fn compute_did_stability() {
        // Same inputs should always produce the same DID
        let did1 = compute_did("dtid:schlib", "ABCDEF");
        let did2 = compute_did("dtid:schlib", "ABCDEF");
        assert_eq!(did1, did2);

        // Different doc_key -> different DID
        let did3 = compute_did("dtid:schlib", "OTHER");
        assert_ne!(did1, did3);
    }

    #[test]
    fn collision_dup_suffix() {
        let mut store = DocumentStore::new(DocumentMeta::SchLib {
            header_text: String::new(),
            weight: 0,
            minor_version: 0,
            unique_id: "test".to_string(),
            raw_header: None,
            section_keys: crate::v2::documents::section_keys::SectionKeyList::new(),
            raw_extra_streams: HashMap::new(),
        });

        // Create two identical components (same params -> same fingerprint)
        let param_str = "|RECORD=1|LIBREFERENCE=R1|";
        for _ in 0..2 {
            let origin = RecordOrigin::Param(ParamOrigin::new(param_str));
            let parent = RecordNode::new(1, origin);
            let parent_id = store.insert_record(parent);
            store.insert_group(GroupData {
                parent: parent_id,
                children: Vec::new(),
                original_indices: Vec::new(),
                parent_original_index: None,
                extra_streams: HashMap::new(),
                meta: GroupMeta::SchComponent {
                    lib_ref: "R1".to_string(),
                    description: String::new(),
                    part_count: 1,
                    section_key: String::new(),
                },
            });
        }

        compute_all_ids(&mut store, "dtid:schlib", "test");

        // Both records should have RIDs, and one should have :dup2
        let rids: Vec<&SemanticId> = store.record_semantic_ids.values().collect();
        let dup_count = rids.iter().filter(|r| r.as_str().contains(":dup")).count();
        assert!(dup_count >= 1, "Expected at least one :dup suffix");
    }

    #[test]
    fn sch_record_anchor_uses_uniqueid() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=2|UNIQUEID=ABC123|NAME=VCC|",
        ));
        let anchor = sch_record_anchor(&origin, 2, Some("comp"), 0);
        assert_eq!(anchor, "ABC123");
    }

    #[test]
    fn sch_record_anchor_pin_fallback() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=2|NAME=VCC|",
        ));
        let anchor = sch_record_anchor(&origin, 2, Some("comp_anchor"), 3);
        assert_eq!(anchor, "comp_anchor:pin:3");
    }

    #[test]
    fn sch_record_anchor_fingerprint_fallback() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=4|X1=100|Y1=200|",
        ));
        let anchor = sch_record_anchor(&origin, 4, None, 0);
        // Should be a blake3 hash (32 hex chars)
        assert_eq!(anchor.len(), 32);
    }

    #[test]
    fn pcb_record_anchor_format() {
        let origin = RecordOrigin::Binary(
            crate::v2::backing_store::BinaryOrigin::new(vec![0xAA; 10]),
        );
        let anchor = pcb_record_anchor(&origin, 5);
        assert!(anchor.starts_with("data:index:5:hash:"));
    }

    #[test]
    fn blake3_content_hash_determinism() {
        let data = b"test data for hashing";
        let h1 = blake3_content_hash(data);
        let h2 = blake3_content_hash(data);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 32);
    }

    #[test]
    fn semantic_fingerprint_excludes_volatile() {
        let params1 = crate::v2::parameters::ParameterCollection::from_string(
            "|RECORD=2|NAME=VCC|OWNERINDEX=0|LOCATION.X=100|",
        );
        let params2 = crate::v2::parameters::ParameterCollection::from_string(
            "|RECORD=2|NAME=VCC|OWNERINDEX=5|LOCATION.X=999|",
        );
        // Volatile keys differ but fingerprint should match
        assert_eq!(
            sch_semantic_fingerprint(&params1),
            sch_semantic_fingerprint(&params2)
        );
    }

    #[test]
    fn full_schlib_id_computation() {
        let mut store = DocumentStore::new(DocumentMeta::SchLib {
            header_text: String::new(),
            weight: 0,
            minor_version: 0,
            unique_id: "LIB-UID".to_string(),
            raw_header: None,
            section_keys: crate::v2::documents::section_keys::SectionKeyList::new(),
            raw_extra_streams: HashMap::new(),
        });

        let comp_origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=1|LIBREFERENCE=R1|UNIQUEID=COMP1|",
        ));
        let parent = RecordNode::new(1, comp_origin);
        let parent_id = store.insert_record(parent);

        let pin_origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=2|OWNERINDEX=0|NAME=1|DESIGNATOR=1|UNIQUEID=PIN1|",
        ));
        let pin = RecordNode::new(2, pin_origin);
        let pin_id = store.insert_record(pin);

        store.insert_group(GroupData {
            parent: parent_id,
            children: vec![pin_id],
            original_indices: vec![1],
            parent_original_index: None,
            extra_streams: HashMap::new(),
            meta: GroupMeta::SchComponent {
                lib_ref: "R1".to_string(),
                description: String::new(),
                part_count: 1,
                section_key: String::new(),
            },
        });

        compute_all_ids(&mut store, "dtid:schlib", "LIB-UID");

        // Document ID
        assert!(store.document_id.is_some());
        let did = store.document_id.as_ref().unwrap();
        assert!(did.as_str().starts_with("did:"));

        // Group semantic ID
        let gid = store.group_ids()[0];
        assert!(store.group_semantic_ids.contains_key(&gid));
        let sgid = &store.group_semantic_ids[&gid];
        assert!(sgid.as_str().starts_with("sgid:"));

        // Record semantic IDs
        assert!(store.record_semantic_ids.contains_key(&parent_id));
        assert!(store.record_semantic_ids.contains_key(&pin_id));
        let parent_rid = &store.record_semantic_ids[&parent_id];
        let pin_rid = &store.record_semantic_ids[&pin_id];
        assert!(parent_rid.as_str().starts_with("rid:"));
        assert!(pin_rid.as_str().starts_with("rid:"));

        // IDs should be stable across re-computation
        let did_str = did.as_str().to_string();
        compute_all_ids(&mut store, "dtid:schlib", "LIB-UID");
        assert_eq!(store.document_id.as_ref().unwrap().as_str(), did_str);
    }
}

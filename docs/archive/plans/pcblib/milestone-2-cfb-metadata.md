# Milestone 2: CFB Metadata & Footprint Enumeration

Parse the FileHeader, SectionKeys, and enumerate footprint storages in the CFB container.

## Files

- `crates/altium-format/src/pcblib/mod.rs` (PcbLib::open implementation)
- `crates/altium-format/src/pcblib/section_keys.rs` (new - shared SectionKeys parser)

## Flags

- `conformance`: SectionKeys format is identical to SchLib -- share code
- `needs-rationale`: FileHeader version validation rejects non-V6 formats

## Requirements

- Parse `/FileHeader` stream using existing `parse_pcb_file_header()` from `pcb_file_header.rs`
- Validate the header version string is exactly "PCB 6.0 Binary Library File" (reject PcbDoc
  and non-V6 formats; Decision: "FileHeader validation: exact string match")
- Extract shared SectionKeys parsing from SchLib into a common module that both SchLib and
  PcbLib can import
- Parse `/SectionKeys` stream (optional): build `full_name -> cfb_key` and reverse mappings
- Enumerate all top-level CFB storages, excluding system storages (`FileVersionInfo`, `Library`)
- For each footprint storage: verify it contains a `Data` sub-stream
- Resolve display names via SectionKeys mapping (or use storage name directly for names <= 31 chars)
- Build the footprint list with display names and CFB keys (primitives not yet loaded)
- Mark FileHeader and SectionKeys streams as consumed in TrackedCfbDocument

## Acceptance Criteria

- `altium validate <pcblib>` opens file without error (returns Ok, but no footprint parsing yet)
- FileHeader parsing succeeds on all test files
- SectionKeys parsing succeeds on Synthiam.PcbLib (which has 2 entries)
- Footprint enumeration returns correct count: LimeMicro = 281, Synthiam = exact count
  (determine via `altium cfb ls data/pcblib/Synthiam*.PcbLib --flat | grep -c '/Data'` before
  M2 implementation and replace this placeholder with the actual number)
- Non-PcbLib files are rejected with a clear format error

## Tests

- **Test files**: `#[cfg(test)]` in `pcblib/mod.rs` and `section_keys.rs`
- **Test type**: property-based (proptest for SectionKeys round-trip) + integration (real files)
- **Backing**: user-specified (property-based), doc-derived (integration with selected subset)
- **Scenarios**:
  - Normal: Parse FileHeader from a known-good PcbLib
  - Normal: Enumerate footprints from a library with multiple footprints
  - Edge: SectionKeys with names > 31 characters (Synthiam)
  - Edge: Library with no SectionKeys stream (LimeMicro, BlankPcbLib)
  - Error: Reject PcbDoc file opened as PcbLib (header mismatch)
  - Error: Reject hypothetical non-V6 library (exact string match)

## Code Intent

### Diff: create `crates/altium-format/src/pcblib/section_keys.rs`

```diff
--- /dev/null
+++ b/crates/altium-format/src/pcblib/section_keys.rs
@@ -0,0 +1,68 @@
+use std::collections::HashMap;
+
+use altium_format_types::constants::record_structure::{KEY_COUNT, RECORD, SECTION_KEY};
+use altium_format_types::constants::streams::SECTION_KEYS;
+
+use crate::block_stream::{parse_blocks, BlockFormat};
+use crate::param_collection::ParameterCollection;
+use crate::{AltiumFormatError, Result};
+
+pub(crate) fn parse_section_keys(data: &[u8]) -> Result<HashMap<String, String>> {
+    use altium_format_types::constants::component::LIB_REF;
+    let blocks = parse_blocks(data)?;
+    if blocks.len() != 1 {
+        return Err(AltiumFormatError::InvalidParamValue {
+            key: SECTION_KEYS.to_owned(),
+            detail: format!("expected 1 block, got {}", blocks.len()),
+        });
+    }
+    let block = &blocks[0];
+    if block.format != BlockFormat::Text {
+        return Err(AltiumFormatError::InvalidParamValue {
+            key: SECTION_KEYS.to_owned(),
+            detail: "expected text block, got binary".to_owned(),
+        });
+    }
+
+    let mut params = ParameterCollection::from_bytes(&block.data)?;
+
+    if let Some(record) = params.remove_optional::<i32>(RECORD)? {
+        if record != 0 {
+            return Err(AltiumFormatError::InvalidParamValue {
+                key: RECORD.to_owned(),
+                detail: format!("SectionKeys RECORD must be 0, got {record}"),
+            });
+        }
+    }
+
+    let mut map = HashMap::new();
+    let count: i32 = params.remove_required(KEY_COUNT)?;
+    for n in 0..count {
+        let lib_ref: String = params.remove_required(&format!("{}{}", LIB_REF, n))?;
+        let section_key: String = params.remove_required(&format!("{}{}", SECTION_KEY, n))?;
+        map.insert(lib_ref, section_key);
+    }
+
+    params.assert_exhausted()?;
+
+    Ok(map)
+}
+
+pub(crate) fn resolve_footprint_key(name: &str, section_keys: &HashMap<String, String>) -> String {
+    let key = section_keys.get(name).map(String::as_str).unwrap_or(name);
+    sanitize_cfb_name(key)
+}
+
+pub(crate) fn sanitize_cfb_name(name: &str) -> String {
+    name.chars()
+        .map(|c| if "/\\:*?\"<>|!".contains(c) { '_' } else { c })
+        .collect()
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn resolve_short_name_unchanged() {
+        let keys = HashMap::new();
+        assert_eq!(resolve_footprint_key("SOT23", &keys), "SOT23");
+    }
+
+    #[test]
+    fn sanitize_replaces_illegal_chars() {
+        assert_eq!(sanitize_cfb_name("A/B:C"), "A_B_C");
+    }
+}
```

### Diff: update `crates/altium-format/src/schlib.rs` to use shared section_keys module

```diff
--- a/crates/altium-format/src/schlib.rs
+++ b/crates/altium-format/src/schlib.rs
@@ -207,43 +207,12 @@ fn parse_file_header(data: &[u8]) -> Result<SchLibHeader> {
     Ok(SchLibHeader { weight, minor_version, unique_id, fonts, display_settings, components })
 }

-pub(crate) fn parse_section_keys(data: &[u8]) -> Result<HashMap<String, String>> {
-    let blocks = parse_blocks(data)?;
-    if blocks.len() != 1 {
-        return Err(AltiumFormatError::InvalidParamValue {
-            key: SECTION_KEYS.to_owned(),
-            detail: format!("expected 1 block, got {}", blocks.len()),
-        });
-    }
-    let block = &blocks[0];
-    if block.format != BlockFormat::Text {
-        return Err(AltiumFormatError::InvalidParamValue {
-            key: SECTION_KEYS.to_owned(),
-            detail: "expected text block, got binary".to_owned(),
-        });
-    }
-
-    let mut params = ParameterCollection::from_bytes(&block.data)?;
-
-    if let Some(record) = params.remove_optional::<i32>(RECORD)? {
-        if record != 0 {
-            return Err(AltiumFormatError::InvalidParamValue {
-                key: RECORD.to_owned(),
-                detail: format!("SectionKeys RECORD must be 0, got {record}"),
-            });
-        }
-    }
-
-    let mut map = HashMap::new();
-    let count: i32 = params.remove_required(KEY_COUNT)?;
-    for n in 0..count {
-        let lib_ref: String = params.remove_required(&format!("{}{}", LIB_REF, n))?;
-        let section_key: String = params.remove_required(&format!("{}{}", SECTION_KEY, n))?;
-        map.insert(lib_ref, section_key);
-    }
-
-    params.assert_exhausted()?;
-
-    Ok(map)
-}
+use crate::pcblib::section_keys::parse_section_keys;

 pub(crate) fn resolve_component_key(
     name: &str,
     section_keys: &HashMap<String, String>,
 ) -> String {
-    let key = section_keys.get(name).map(String::as_str).unwrap_or(name);
-    sanitize_cfb_name(key)
+    crate::pcblib::section_keys::sanitize_cfb_name(
+        section_keys.get(name).map(String::as_str).unwrap_or(name),
+    )
 }

+// NOTE: The local `sanitize_cfb_name` function is NOT removed — it is still called
+// by `build_section_key_for_name` (write-path code). Only `resolve_component_key`
+// delegates to the shared version. Both can coexist; in a future refactor the local
+// copy can be deleted once the write-path also uses the shared version.
```

Note: The `parse_section_keys` implementation is moved to `pcblib/section_keys.rs`. The schlib.rs `parse_section_keys` is removed and the shared version imported. The `schlib.rs` local `sanitize_cfb_name` function must NOT be deleted — `build_section_key_for_name` still calls it directly (write-path code). Only `resolve_component_key` is updated to call the shared version. The `build_section_keys`, `build_section_key_for_name`, `generate_unique_key`, and local `sanitize_cfb_name` remain private to `schlib.rs`.

### Diff: update `crates/altium-format/src/pcblib/mod.rs` — implement PcbLib::open with FileHeader, SectionKeys, and footprint enumeration

```diff
--- a/crates/altium-format/src/pcblib/mod.rs
+++ b/crates/altium-format/src/pcblib/mod.rs
@@ -1,9 +1,14 @@
+pub(crate) mod section_keys;
+
 use std::collections::HashMap;
 use std::path::Path;

 use altium_format_types::{Coord, CoordPoint, PcbFlags, PcbObjectId, V6Layer};
+use altium_format_types::constants::file_headers::PCB_LIBRARY_BINARY_HEADER_V6;
+use altium_format_types::constants::streams::{FILE_HEADER, SECTION_KEYS};

 use crate::pcb_file_header::{parse_pcb_file_header, PcbFileHeader};
 use crate::tracked_cfb::TrackedCfbDocument;
+use crate::{AltiumFormatError, Result};

 // ... (all struct definitions unchanged from M1) ...

@@ -108,14 +113,60 @@ impl PcbLib {
     pub fn open(path: impl AsRef<Path>) -> crate::Result<Self> {
         let path = path.as_ref();
-        let _doc = TrackedCfbDocument::open(path)?;
-        Ok(Self {
-            header: PcbFileHeader {
-                version_string: String::new(),
-                version: 0.0,
-                unique_id: String::new(),
-            },
-            section_keys: HashMap::new(),
-            footprints: Vec::new(),
-        })
+        let mut doc = TrackedCfbDocument::open(path)?;
+
+        // 1. FileHeader
+        let file_header_data = doc.read_stream(&format!("/{FILE_HEADER}"))?;
+        let header = parse_pcb_file_header(&file_header_data)?;
+        if header.version_string != PCB_LIBRARY_BINARY_HEADER_V6 {
+            return Err(AltiumFormatError::InvalidParamValue {
+                key: FILE_HEADER.to_owned(),
+                detail: format!(
+                    "expected \"{}\", got \"{}\"",
+                    PCB_LIBRARY_BINARY_HEADER_V6, header.version_string
+                ),
+            });
+        }
+
+        // 2. SectionKeys (optional)
+        let section_keys = match doc.read_stream_optional(&format!("/{SECTION_KEYS}"))? {
+            Some(data) => section_keys::parse_section_keys(&data)?,
+            None => HashMap::new(),
+        };
+
+        // 3. Enumerate top-level storages (exclude FileVersionInfo and Library system storages)
+        let (storages, _streams) = doc.list_entries("/")?;
+        let mut footprints = Vec::new();
+        for storage_name in &storages {
+            let name = storage_name.trim_start_matches('/');
+            if name == "FileVersionInfo" || name == "Library" {
+                continue;
+            }
+            let data_path = format!("/{name}/Data");
+            if !doc.exists(&data_path) {
+                continue;
+            }
+            let display_name = {
+                let reverse: HashMap<_, _> = section_keys
+                    .iter()
+                    .map(|(k, v)| (v.as_str(), k.as_str()))
+                    .collect();
+                reverse
+                    .get(name)
+                    .map(|s| s.to_string())
+                    .unwrap_or_else(|| name.to_owned())
+            };
+            footprints.push(PcbFootprint {
+                display_name,
+                cfb_key: name.to_owned(),
+                pattern: String::new(),
+                height: Coord::ZERO,
+                description: String::new(),
+                item_guid: String::new(),
+                revision_guid: String::new(),
+                primitives: Vec::new(),
+            });
+        }
+
+        doc.assert_all_consumed()?;
+
+        Ok(Self { header, section_keys, footprints })
     }
 }
```

Note: `PCB_LIBRARY_BINARY_HEADER_V6` must be added to `altium-format-types/src/constants/file_headers.rs` as `"PCB 6.0 Binary Library File"`. This addition to `file_headers.rs` is part of this milestone.

### Diff: add `PCB_LIBRARY_BINARY_HEADER_V6` constant to `crates/altium-format-types/src/constants/file_headers.rs`

```diff
--- a/crates/altium-format-types/src/constants/file_headers.rs
+++ b/crates/altium-format-types/src/constants/file_headers.rs
@@ -198,3 +198,9 @@
 /// **Container:** OLE2 compound document
 pub const ELECTRONICS_SYSTEM_DESIGN_JSON_HEADER_V1: &str =
     "Altium Designer - Electronics System Design JSON File Version 1.0";
+
+// ---------------------------------------------------------------------------
+// PCB library headers
+// ---------------------------------------------------------------------------
+
+/// V6 binary PCB footprint library header (OLE2 compound document).
+///
+/// **Era:** Current (AD6+)
+/// **Container:** OLE2 compound document
+pub const PCB_LIBRARY_BINARY_HEADER_V6: &str = "PCB 6.0 Binary Library File";
```

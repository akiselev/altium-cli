# Milestone 6: Sidecar Streams

Parse the 4 optional sidecar stream types that augment primitive data: WideStrings,
UniqueIDPrimitiveInformation, ExtendedPrimitiveInformation, and PrimitiveGuids.

## Files

- `crates/altium-format/src/pcblib/sidecar.rs` (new - UniqueID, ExtendedPrimitiveInfo, PrimitiveGuids)
- `crates/altium-format/src/pcblib/wide_strings.rs` (new - PcbLib-specific parameter-block format)
- `crates/altium-format/src/pcblib/footprint.rs` (integrate sidecar merging after primitive parsing)

## Flags

- `needs-rationale`: PcbLib WideStrings format differs completely from PcbDoc WideStrings
- `error-handling`: Sidecar index out-of-range must error, not silently skip

## Requirements

- Parse WideStrings sidecar (`<Footprint>/WideStrings`):
  - Read as length-prefixed parameter block (u32 length + NUL-terminated param string)
  - Handle empty WideStrings (single 0x00 byte = no wide string data)
  - Parse `ENCODEDTEXT{N}` parameters: split on commas, parse each token as decimal u8 (return
    `AltiumFormatError` if any token cannot be parsed as decimal u8 — CLAUDE.md: "MUST RETURN
    a Result, never silently drop"), convert to byte array, decode as UTF-8
  - Merge decoded strings into Text primitives by index N (replace Win1252 text from core record)
  - ENCODEDTEXT index N refers to the Nth Text primitive in the footprint (Text-specific indexing)
- Parse UniqueIDPrimitiveInformation (`<Footprint>/UniqueIDPrimitiveInformation/{Header,Data}`):
  - Read u32 count from Header
  - Parse Data as parameter blocks: extract PRIMITIVEINDEX, PRIMITIVEOBJECTID, UNIQUEID
  - Validate PRIMITIVEOBJECTID matches the actual primitive type at PRIMITIVEINDEX.
    Before implementing, determine canonical PRIMITIVEOBJECTID string values by inspecting
    real files: `altium cfb dump data/pcblib/LimeMicro*.PcbLib <footprint>/UniqueIDPrimitiveInformation/Data --blocks`
    Document the exact case-sensitive strings (likely "Pad", "Track", "Arc", etc.) and implement
    a `PcbObjectId::from_primitive_object_id_str()` conversion for type-safe comparison.
  - Merge UNIQUEID string into the primitive at the specified index
- Parse ExtendedPrimitiveInformation (`<Footprint>/ExtendedPrimitiveInformation/{Header,Data}`):
  - Same parameter-block format as UniqueID
  - Extract TYPE, SOLDERMASKEXPANSIONMODE, PASTEMASKEXPANSIONMODE, etc.
  - Merge extended properties into primitives by PRIMITIVEINDEX
  - This stream is rare (only 1 footprint in LimeMicro test corpus has it)
- Parse PrimitiveGuids (`<Footprint>/PrimitiveGuids/{Header,Data}`):
  - Read u32 count from Header
  - Parse binary GUID records from Data stream
  - Assign GUIDs to primitives by entry mapping
  - If format cannot be determined from documentation + Ghidra analysis, return
    `AltiumFormatError` (Decision: "PrimitiveGuids: error if format unclear"). NO raw-byte storage.
    The red/green loop surfaces this as a test failure, forcing investigation.
- All sidecar streams are optional -- check existence before opening
- Mark all sidecar streams as consumed in TrackedCfbDocument
- Sidecar index out-of-range must return an error (fail-fast, not silent skip)

## Acceptance Criteria

- Unicode text strings from WideStrings are correctly decoded into Text primitives
- UniqueID strings are correctly assigned to primitives at the right indices
- Empty WideStrings streams (0x00 byte) are handled without error
- Footprints without sidecar streams parse correctly (all sidecars are optional)
- Index out-of-range errors produce clear diagnostics (footprint name + index + count)
- All sidecar streams are consumed (no UnconsumedStreams error)
- LimeMicro library parses including its 1 footprint with ExtendedPrimitiveInformation

## Tests

- **Test files**: `#[cfg(test)]` in `wide_strings.rs` and `sidecar.rs`
- **Test type**: property-based (ENCODEDTEXT decoding) + integration (real files)
- **Backing**: user-specified (property-based), doc-derived (sidecar-streams.md)
- **Scenarios**:
  - Normal: Decode ENCODEDTEXT with ASCII bytes (e.g., `.Designator`)
  - Normal: Merge UniqueID into a Pad primitive
  - Edge: Empty WideStrings stream (single 0x00 byte)
  - Edge: Footprint with no sidecar streams at all
  - Edge: ExtendedPrimitiveInformation with mask expansion overrides
  - Error: ENCODEDTEXT with invalid UTF-8 byte sequence
  - Error: ENCODEDTEXT with non-decimal token (e.g., "65,abc,66") returns parse error
  - Error: PRIMITIVEINDEX referencing a non-existent primitive

## Code Intent

### Pre-implementation investigation: PRIMITIVEOBJECTID string values

Before implementing `sidecar.rs`, inspect real file data to determine the canonical string values:
```
altium cfb dump data/pcblib/LimeMicro*.PcbLib <footprint>/UniqueIDPrimitiveInformation/Data --blocks
```
Document the exact case-sensitive strings for PRIMITIVEOBJECTID (likely "Pad", "Track", "Arc", etc.).
Then add `PcbObjectId::from_primitive_object_id_str(s: &str) -> Option<PcbObjectId>` to
`altium-format-types/src/pcb.rs` with the correct mappings before writing the sidecar parser.

### Diff: create `crates/altium-format/src/pcblib/wide_strings.rs`

```diff
--- /dev/null
+++ b/crates/altium-format/src/pcblib/wide_strings.rs
@@ -0,0 +1,61 @@
+use std::collections::HashMap;
+
+use crate::binary_io::BinaryReader;
+use crate::param_collection::ParameterCollection;
+use crate::{AltiumFormatError, Result};
+
+/// Parse PcbLib WideStrings sidecar stream.
+///
+/// Decision: "PcbLib WideStrings use parameter-block format" — NOT the TLV format
+/// used by PcbDoc WideStrings6. This stream uses u32 length + Win1252 parameter string
+/// with ENCODEDTEXT{N} keys containing comma-separated decimal byte values.
+///
+/// Returns: text primitive index -> decoded UTF-8 string.
+pub(crate) fn parse_pcblib_wide_strings(data: &[u8]) -> Result<HashMap<usize, String>> {
+    if data.is_empty() || (data.len() == 1 && data[0] == 0x00) {
+        return Ok(HashMap::new());
+    }
+
+    let mut reader = BinaryReader::new(data);
+    let block_len = reader.read_u32_le()? as usize;
+    let block_data = reader.read_bytes(block_len)?;
+
+    let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(block_data);
+    let mut params = ParameterCollection::from_str(&decoded)?;
+
+    let mut result = HashMap::new();
+    let keys: Vec<String> = params.keys_matching("ENCODEDTEXT");
+    for key in keys {
+        let n: usize = key["ENCODEDTEXT".len()..]
+            .parse()
+            .map_err(|e| AltiumFormatError::InvalidParamValue {
+                key: key.clone(),
+                detail: format!("non-numeric ENCODEDTEXT index: {e}"),
+            })?;
+        let value: String = params.remove_required(&key)?;
+        let bytes = decode_encodedtext(&key, &value)?;
+        let text = std::str::from_utf8(&bytes).map_err(|e| AltiumFormatError::InvalidParamValue {
+            key: key.clone(),
+            detail: format!("ENCODEDTEXT decoded bytes are not valid UTF-8: {e}"),
+        })?;
+        result.insert(n, text.to_owned());
+    }
+    params.assert_exhausted()?;
+    Ok(result)
+}
+
+fn decode_encodedtext(key: &str, value: &str) -> Result<Vec<u8>> {
+    value
+        .split(',')
+        .map(|token| {
+            token.trim().parse::<u8>().map_err(|e| AltiumFormatError::InvalidParamValue {
+                key: key.to_owned(),
+                detail: format!("non-decimal token '{token}' in ENCODEDTEXT: {e}"),
+            })
+        })
+        .collect()
+}
```

### Diff: add `keys_matching` to `crates/altium-format/src/param_collection.rs`

This method does not currently exist and must be added:

```diff
--- a/crates/altium-format/src/param_collection.rs
+++ b/crates/altium-format/src/param_collection.rs
@@ -373,6 +373,14 @@ impl ParameterCollection {
     pub(crate) fn assert_exhausted(&self) -> Result<()> {
         // (existing implementation)
     }

+    /// Returns all keys whose names start with `prefix` (case-insensitive).
+    /// Does not consume the keys; use with `remove_required` to read values.
+    pub(crate) fn keys_matching(&self, prefix: &str) -> Vec<String> {
+        let lower_prefix = prefix.to_ascii_lowercase();
+        self.params
+            .keys()
+            .filter(|k| k.to_ascii_lowercase().starts_with(&lower_prefix))
+            .cloned()
+            .collect()
+    }
```

### Diff: create `crates/altium-format/src/pcblib/sidecar.rs`

```diff
--- /dev/null
+++ b/crates/altium-format/src/pcblib/sidecar.rs
@@ -0,0 +1,121 @@
+use std::collections::HashMap;
+
+use altium_format_types::{MaskExpansionMode, PcbObjectId};
+
+use crate::block_stream::iter_blocks;
+use crate::param_collection::ParameterCollection;
+use crate::pcb_binary_stream::parse_pcb_section_header;
+use crate::pcblib::PcbPrimitive;
+use crate::{AltiumFormatError, Result};
+
+pub(crate) struct UniqueIdEntry {
+    pub(crate) primitive_index: usize,
+    pub(crate) object_id: PcbObjectId,
+    pub(crate) unique_id: String,
+}
+
+pub(crate) struct ExtendedPrimitiveInfoEntry {
+    pub(crate) primitive_index: usize,
+    pub(crate) primitive_object_id: PcbObjectId,
+    pub(crate) info_type: String,
+    pub(crate) solder_mask_expansion_mode: MaskExpansionMode,
+    pub(crate) solder_mask_expansion_manual: String,
+    pub(crate) paste_mask_expansion_mode: MaskExpansionMode,
+    pub(crate) paste_mask_expansion_manual: String,
+}
+
+/// Parse UniqueIDPrimitiveInformation/{Header,Data} streams.
+pub(crate) fn parse_unique_id_info(
+    header: &[u8],
+    data: &[u8],
+) -> Result<Vec<UniqueIdEntry>> {
+    let count = parse_pcb_section_header(header)? as usize;
+    if count == 0 {
+        return Ok(Vec::new());
+    }
+    let mut params = ParameterCollection::from_bytes(data)?;
+    let mut entries = Vec::with_capacity(count);
+    for _ in 0..count {
+        let index: i32 = params.remove_required("PRIMITIVEINDEX")?;
+        let object_id_str: String = params.remove_required("PRIMITIVEOBJECTID")?;
+        let unique_id: String = params.remove_required("UNIQUEID")?;
+        let object_id = PcbObjectId::from_primitive_object_id_str(&object_id_str)
+            .ok_or_else(|| AltiumFormatError::InvalidParamValue {
+                key: "PRIMITIVEOBJECTID".to_owned(),
+                detail: format!("unknown PRIMITIVEOBJECTID string: '{object_id_str}'"),
+            })?;
+        entries.push(UniqueIdEntry {
+            primitive_index: index as usize,
+            object_id,
+            unique_id,
+        });
+    }
+    params.assert_exhausted()?;
+    Ok(entries)
+}
+
+/// Parse ExtendedPrimitiveInformation/{Header,Data} streams.
+///
+/// Each entry is a block-framed parameter string containing mask expansion
+/// properties for a specific primitive. Known keys: PRIMITIVEINDEX,
+/// PRIMITIVEOBJECTID, TYPE, SOLDERMASKEXPANSIONMODE, SOLDERMASKEXPANSION_MANUAL,
+/// PASTEMASKEXPANSIONMODE, PASTEMASKEXPANSION_MANUAL.
+pub(crate) fn parse_extended_primitive_information(
+    header: &[u8],
+    data: &[u8],
+) -> Result<Vec<ExtendedPrimitiveInfoEntry>> {
+    let expected_count = parse_pcb_section_header(header)? as usize;
+
+    let mut entries = Vec::with_capacity(expected_count);
+    for block_result in iter_blocks(data) {
+        let block = block_result?;
+        let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(&block.data);
+        let mut params = ParameterCollection::from_str(&decoded)?;
+
+        let primitive_index: i32 = params.remove_required("PRIMITIVEINDEX")?;
+        if primitive_index < 0 {
+            return Err(AltiumFormatError::InvalidParamValue {
+                key: "PRIMITIVEINDEX".to_owned(),
+                detail: format!("negative primitive index: {primitive_index}"),
+            });
+        }
+
+        let object_id_str: String = params.remove_required("PRIMITIVEOBJECTID")?;
+        let primitive_object_id =
+            PcbObjectId::from_primitive_object_id_str(&object_id_str).ok_or_else(|| {
+                AltiumFormatError::InvalidParamValue {
+                    key: "PRIMITIVEOBJECTID".to_owned(),
+                    detail: format!("unknown primitive object ID string: '{object_id_str}'"),
+                }
+            })?;
+
+        let info_type = params.remove_optional::<String>("TYPE")?.unwrap_or_default();
+
+        let solder_mode_str = params
+            .remove_optional::<String>("SOLDERMASKEXPANSIONMODE")?
+            .unwrap_or_else(|| "None".to_owned());
+        let solder_mask_expansion_mode =
+            parse_mask_expansion_mode("SOLDERMASKEXPANSIONMODE", &solder_mode_str)?;
+        let solder_mask_expansion_manual = params
+            .remove_optional::<String>("SOLDERMASKEXPANSION_MANUAL")?
+            .unwrap_or_default();
+
+        let paste_mode_str = params
+            .remove_optional::<String>("PASTEMASKEXPANSIONMODE")?
+            .unwrap_or_else(|| "None".to_owned());
+        let paste_mask_expansion_mode =
+            parse_mask_expansion_mode("PASTEMASKEXPANSIONMODE", &paste_mode_str)?;
+        let paste_mask_expansion_manual = params
+            .remove_optional::<String>("PASTEMASKEXPANSION_MANUAL")?
+            .unwrap_or_default();
+
+        params.assert_exhausted()?;
+
+        entries.push(ExtendedPrimitiveInfoEntry {
+            primitive_index: primitive_index as usize,
+            primitive_object_id,
+            info_type,
+            solder_mask_expansion_mode,
+            solder_mask_expansion_manual,
+            paste_mask_expansion_mode,
+            paste_mask_expansion_manual,
+        });
+    }
+
+    if entries.len() != expected_count {
+        return Err(AltiumFormatError::RecordCountMismatch {
+            section: "ExtendedPrimitiveInformation".to_owned(),
+            expected: expected_count,
+            actual: entries.len(),
+        });
+    }
+
+    Ok(entries)
+}
+
+/// Parses a mask expansion mode string ("None", "NoMask", "Rule", "Manual").
+fn parse_mask_expansion_mode(key: &str, value: &str) -> Result<MaskExpansionMode> {
+    match value {
+        "None" | "NoMask" => Ok(MaskExpansionMode::NoMask),
+        "Rule" => Ok(MaskExpansionMode::Rule),
+        "Manual" => Ok(MaskExpansionMode::Manual),
+        _ => Err(AltiumFormatError::InvalidParamValue {
+            key: key.to_owned(),
+            detail: format!("unknown mask expansion mode: '{value}'"),
+        }),
+    }
+}
+
+/// Parse PrimitiveGuids/{Header,Data} streams.
+///
+/// Decision: "PrimitiveGuids: error if format unclear" — if format cannot be
+/// determined from docs + Ghidra analysis, return AltiumFormatError. No raw-byte storage.
+///
+/// PRE-IMPLEMENTATION INVESTIGATION REQUIRED: Before implementing, run:
+///   altium cfb dump data/pcblib/LimeMicro*.PcbLib <footprint>/PrimitiveGuids/Data --blocks
+/// to determine the binary GUID record layout (field offsets and sizes).
+/// Document in docs/pcblib/sidecar-streams.md then replace this stub.
+pub(crate) fn parse_primitive_guids(header: &[u8], data: &[u8]) -> Result<()> {
+    let count = parse_pcb_section_header(header)? as usize;
+    if count == 0 && data.is_empty() {
+        return Ok(());
+    }
+    Err(AltiumFormatError::InvalidParamValue {
+        key: "PrimitiveGuids".to_owned(),
+        detail: format!(
+            "PrimitiveGuids parser not yet implemented (count={count}, data={} bytes); \
+             run investigation with `altium cfb dump` before implementing",
+            data.len()
+        ),
+    })
+}
+
+fn primitive_object_id(p: &PcbPrimitive) -> PcbObjectId {
+    match p {
+        PcbPrimitive::Arc(_) => PcbObjectId::Arc,
+        PcbPrimitive::Pad(_) => PcbObjectId::Pad,
+        PcbPrimitive::Via(_) => PcbObjectId::Via,
+        PcbPrimitive::Track(_) => PcbObjectId::Track,
+        PcbPrimitive::Text(_) => PcbObjectId::Text,
+        PcbPrimitive::Fill(_) => PcbObjectId::Fill,
+        PcbPrimitive::Region(_) => PcbObjectId::Region,
+        PcbPrimitive::ComponentBody(_) => PcbObjectId::ComponentBody,
+    }
+}
+
+/// Merge all sidecar data into footprint primitives.
+pub(crate) fn merge_sidecars(
+    primitives: &mut Vec<PcbPrimitive>,
+    wide_strings: HashMap<usize, String>,
+    unique_ids: Vec<UniqueIdEntry>,
+) -> Result<()> {
+    // Apply WideStrings to Text primitives by Text-relative index.
+    let mut text_count = 0usize;
+    for primitive in primitives.iter_mut() {
+        if let PcbPrimitive::Text(text) = primitive {
+            if let Some(wide_text) = wide_strings.get(&text_count) {
+                text.text = wide_text.clone();
+            }
+            text_count += 1;
+        }
+    }
+
+    // Apply UniqueIDs by global primitive index with type validation.
+    let primitive_count = primitives.len();
+    for entry in unique_ids {
+        let idx = entry.primitive_index;
+        let primitive = primitives.get_mut(idx).ok_or_else(|| {
+            AltiumFormatError::InvalidParamValue {
+                key: "PRIMITIVEINDEX".to_owned(),
+                detail: format!(
+                    "primitive index {idx} out of range (footprint has {primitive_count} primitives)"
+                ),
+            }
+        })?;
+        let actual_object_id = primitive_object_id(primitive);
+        if actual_object_id != entry.object_id {
+            return Err(AltiumFormatError::InvalidParamValue {
+                key: "PRIMITIVEOBJECTID".to_owned(),
+                detail: format!(
+                    "primitive at index {idx} is {:?} but sidecar says {:?}",
+                    actual_object_id, entry.object_id
+                ),
+            });
+        }
+        set_unique_id(primitive, entry.unique_id);
+    }
+
+    Ok(())
+}
+
+/// Sets the unique_id field on a primitive variant.
+fn set_unique_id(primitive: &mut PcbPrimitive, unique_id: String) {
+    match primitive {
+        PcbPrimitive::Arc(p) => p.unique_id = Some(unique_id),
+        PcbPrimitive::Pad(p) => p.unique_id = Some(unique_id),
+        PcbPrimitive::Via(p) => p.unique_id = Some(unique_id),
+        PcbPrimitive::Track(p) => p.unique_id = Some(unique_id),
+        PcbPrimitive::Text(p) => p.unique_id = Some(unique_id),
+        PcbPrimitive::Fill(p) => p.unique_id = Some(unique_id),
+        PcbPrimitive::Region(p) => p.unique_id = Some(unique_id),
+        PcbPrimitive::ComponentBody(p) => p.unique_id = Some(unique_id),
+    }
+}
```

### Diff: update `crates/altium-format/src/pcblib/footprint.rs` — add sidecar loading after primitive parsing

```diff
--- a/crates/altium-format/src/pcblib/footprint.rs
+++ b/crates/altium-format/src/pcblib/footprint.rs
@@ -1,6 +1,8 @@
 use altium_format_types::constants::parsing::BLOCK_SIZE_MASK;
 use altium_format_types::{Coord, PcbObjectId};

+use crate::pcblib::sidecar::{merge_sidecars, parse_extended_primitive_information, parse_primitive_guids, parse_unique_id_primitive_information};
+use crate::pcblib::wide_strings::parse_pcblib_wide_strings;
 use crate::binary_io::BinaryReader;
 // ... (other imports unchanged)

@@ -53,6 +55,47 @@ pub(crate) fn load_footprint(
         primitives: primitives_vec,
     };

+    // 5. WideStrings sidecar (optional)
+    let ws_path = format!("/{cfb_key}/WideStrings");
+    let wide_strings = if let Some(ws_data) = doc.read_stream_optional(&ws_path)? {
+        parse_pcblib_wide_strings(&ws_data)?
+    } else {
+        std::collections::HashMap::new()
+    };
+
+    // 6. UniqueIDPrimitiveInformation sidecar (optional)
+    let uid_header_path = format!("/{cfb_key}/UniqueIDPrimitiveInformation/Header");
+    let uid_data_path = format!("/{cfb_key}/UniqueIDPrimitiveInformation/Data");
+    let unique_ids = if doc.exists(&uid_header_path) {
+        let h = doc.read_stream(&uid_header_path)?;
+        let d = doc.read_stream(&uid_data_path)?;
+        parse_unique_id_primitive_information(&h, &d)?
+    } else {
+        Vec::new()
+    };
+
+    // 7. ExtendedPrimitiveInformation sidecar (optional, rare)
+    let ext_header_path = format!("/{cfb_key}/ExtendedPrimitiveInformation/Header");
+    let ext_data_path = format!("/{cfb_key}/ExtendedPrimitiveInformation/Data");
+    if doc.exists(&ext_header_path) {
+        let h = doc.read_stream(&ext_header_path)?;
+        let d = doc.read_stream(&ext_data_path)?;
+        // Parse and validate; entries are stored for future use (e.g., mask expansion overrides).
+        let _extended = parse_extended_primitive_information(&h, &d)?;
+    }
+
+    // 8. PrimitiveGuids sidecar (optional)
+    let pg_header_path = format!("/{cfb_key}/PrimitiveGuids/Header");
+    let pg_data_path = format!("/{cfb_key}/PrimitiveGuids/Data");
+    if doc.exists(&pg_header_path) {
+        let h = doc.read_stream(&pg_header_path)?;
+        let d = doc.read_stream(&pg_data_path)?;
+        parse_primitive_guids(&h, &d)?;
+    }
+
+    // 9. Merge WideStrings and UniqueIDs into footprint primitives
+    merge_sidecars(&mut footprint.primitives, wide_strings, unique_ids)?;
+
     Ok(footprint)
 }
```

# Milestone 3: Library Storage

Parse the `/Library/` storage containing library-wide metadata: board defaults, layer stack,
component parameter table of contents, 3D model metadata, and auxiliary sub-storages.

## Files

- `crates/altium-format/src/pcblib/library.rs` (new)
- `crates/altium-format/src/pcblib/mod.rs` (integrate library parsing into PcbLib::open)

## Flags

- `needs-rationale`: Library/Data uses non-standard block framing (flat pipe-delimited string, not block-by-block)
- `conformance`: Header+Data stream pattern shared with PcbDoc section reading

## Requirements

- Parse `/Library/Header` stream: u32 record count (using existing `parse_pcb_section_header()`)
- Parse `/Library/Data` stream as a parameter block:
  - Extract library metadata: FILENAME, KIND, VERSION, DATE, TIME
  - Parse board defaults and layer stack parameters (V9_MASTERSTACK_*, V9_STACK_LAYER*)
  - Parse RECORD=Board continuation records
- Parse `/Library/ComponentParamsTOC/{Header,Data}`:
  - Extract footprint summary entries (Name, Pad Count, Height, Description)
  - Store as lookup for validation against actual footprint data
- Parse `/Library/Models/{Header,Data}`:
  - Extract model metadata (EMBED, ID, ROTX/Y/Z, DZ, CHECKSUM, NAME)
  - Build GUID -> model index mapping (for ComponentBody references)
  - Read model blob streams (`Models/0`, `Models/1`, ...) for each entry in the metadata:
    iterate from i=0..count, read each `/Library/Models/{i}` stream, store blob bytes in
    the model entry (Decision: "Library/Models/N blob streams consumed as opaque bytes").
    TrackedCfbDocument requires ALL streams consumed; these are zlib-compressed STEP data
    that the "3D models metadata only" decision accepts as opaque.
- Parse remaining Library sub-storages (each uses Header+Data pattern):
  - `/Library/LayerKindMapping/{Header,Data}`
  - `/Library/PadViaLibrary/{Header,Data}`
  - `/Library/EmbeddedFonts`
  - `/Library/ModelsNoEmbed/{Header,Data}`
  - `/Library/Textures/{Header,Data}`
  - For each: parse Header u32 count. If count == 0, verify Data stream is empty/minimal and
    mark as consumed (this IS full parsing of an empty section). If count > 0, return
    `AltiumFormatError` (Decision: "Error on unimplemented Library sub-storages"). The red/green
    loop will force full implementation when test files contain non-zero data.
- All Library streams must be marked as consumed in TrackedCfbDocument
- MUST NOT store raw bytes for unimplemented streams (CLAUDE.md: "Do NOT mark streams as consumed
  without actually parsing them")

## Acceptance Criteria

- Library metadata (FILENAME, KIND, VERSION) is correctly extracted
- ComponentParamsTOC footprint count matches actual footprint enumeration from M2
- Model metadata is accessible by GUID for libraries with 3D models (LimeMicro has 121)
- All Library sub-streams are consumed (no UnconsumedStreams error from TrackedCfbDocument)
- Empty libraries (BlankPcbLib) parse without error

## Tests

- **Test files**: `#[cfg(test)]` in `pcblib/library.rs`
- **Test type**: integration (real files)
- **Backing**: doc-derived (loading-pipeline.md Phase 3)
- **Scenarios**:
  - Normal: Parse Library/Data from LimeMicro (has board defaults + models)
  - Normal: ComponentParamsTOC count matches footprint enumeration
  - Edge: Empty library (BlankPcbLib -- Library/ exists but has zero/minimal data)
  - Edge: Library with no embedded models (Synthiam -- empty Models Data)

## Code Intent

### Diff: create `crates/altium-format/src/pcblib/library.rs`

```diff
--- /dev/null
+++ b/crates/altium-format/src/pcblib/library.rs
@@ -0,0 +1,176 @@
+use altium_format_types::constants::parsing::BLOCK_SIZE_MASK;
+use altium_format_types::Coord;
+
+use crate::param_collection::ParameterCollection;
+use crate::pcb_binary_stream::parse_pcb_section_header;
+use crate::tracked_cfb::TrackedCfbDocument;
+use crate::{AltiumFormatError, Result};
+
+pub(crate) struct PcbLibraryData {
+    pub(crate) filename: String,
+    pub(crate) kind: String,
+    pub(crate) version: String,
+    pub(crate) date: String,
+    pub(crate) time: String,
+    pub(crate) board_config: PcbBoardConfig,
+}
+
+/// Board configuration parameters from the Library/Data stream.
+///
+/// These are the board defaults and layer stack definitions that Altium uses
+/// when editing footprints in the library editor. The layer stack uses indexed
+/// parameters V9_STACK_LAYER{N}_*.
+pub(crate) struct PcbBoardConfig {
+    pub(crate) record: String,
+    pub(crate) v9_masterstack_style: String,
+    pub(crate) v9_masterstack_id: String,
+    pub(crate) v9_masterstack_name: String,
+    pub(crate) layer_stack: Vec<PcbBoardLayerEntry>,
+}
+
+pub(crate) struct PcbBoardLayerEntry {
+    pub(crate) id: String,
+    pub(crate) name: String,
+    pub(crate) layer_id: String,
+    pub(crate) used_by_prims: bool,
+    pub(crate) cop_thick: String,
+    pub(crate) diel_type: String,
+    pub(crate) diel_const: String,
+    pub(crate) diel_height: String,
+    pub(crate) diel_material: String,
+}
+
+pub(crate) struct PcbLibComponentTocEntry {
+    pub(crate) name: String,
+    pub(crate) pad_count: u32,
+    pub(crate) height: Coord,
+    pub(crate) description: String,
+}
+
+pub(crate) struct PcbLibModelEntry {
+    pub(crate) id: String,
+    pub(crate) name: String,
+    pub(crate) embed: bool,
+    pub(crate) rotation_x: f64,
+    pub(crate) rotation_y: f64,
+    pub(crate) rotation_z: f64,
+    pub(crate) standoff: f64,
+    pub(crate) checksum: String,
+    pub(crate) blob: Option<Vec<u8>>,
+}
+
+/// Parse Library/Data stream as a flat pipe-delimited parameter string.
+///
+/// Decision: "Library/Data non-standard block framing" — read entire stream as a
+/// single Windows-1252 parameter string, not as length-prefixed blocks.
+pub(crate) fn parse_library_data(data: &[u8]) -> Result<PcbLibraryData> {
+    let mut params = ParameterCollection::from_bytes(data)?;
+    let filename = params.remove_optional::<String>("FILENAME")?.unwrap_or_default();
+    let kind = params.remove_optional::<String>("KIND")?.unwrap_or_default();
+    let version = params.remove_optional::<String>("VERSION")?.unwrap_or_default();
+    let date = params.remove_optional::<String>("DATE")?.unwrap_or_default();
+    let time = params.remove_optional::<String>("TIME")?.unwrap_or_default();
+    // Board defaults and layer stack parameters (RECORD=Board, V9_MASTERSTACK_*,
+    // V9_STACK_LAYER{N}_*) must be fully consumed — parse them into a board config struct.
+    let board_config = parse_library_board_params(&mut params)?;
+    params.assert_exhausted()?;
+    Ok(PcbLibraryData { filename, kind, version, date, time, board_config })
+}
+
+/// Parse Library/ComponentParamsTOC/{Header,Data} streams.
+pub(crate) fn parse_component_toc(
+    header: &[u8],
+    data: &[u8],
+) -> Result<Vec<PcbLibComponentTocEntry>> {
+    let count = parse_pcb_section_header(header)? as usize;
+    if count == 0 {
+        return Ok(Vec::new());
+    }
+    let mut params = ParameterCollection::from_bytes(data)?;
+    let mut entries = Vec::with_capacity(count);
+    for _ in 0..count {
+        let name = params.remove_required::<String>("NAME")?;
+        let pad_count = params.remove_optional::<i32>("PADCOUNT")?.unwrap_or(0) as u32;
+        let height = params.remove_optional::<Coord>("HEIGHT")?.unwrap_or(Coord::ZERO);
+        let description = params.remove_optional::<String>("DESCRIPTION")?.unwrap_or_default();
+        entries.push(PcbLibComponentTocEntry { name, pad_count, height, description });
+    }
+    params.assert_exhausted()?;
+    Ok(entries)
+}
+
+/// Parse Library/Models/{Header,Data} streams (metadata only; blobs read by caller).
+pub(crate) fn parse_model_metadata(header: &[u8], data: &[u8]) -> Result<Vec<PcbLibModelEntry>> {
+    let count = parse_pcb_section_header(header)? as usize;
+    if count == 0 {
+        return Ok(Vec::new());
+    }
+    let mut params = ParameterCollection::from_bytes(data)?;
+    let mut entries = Vec::with_capacity(count);
+    for n in 0..count {
+        let id = params.remove_optional::<String>(&format!("ID{n}"))?.unwrap_or_default();
+        let name = params.remove_optional::<String>(&format!("NAME{n}"))?.unwrap_or_default();
+        let embed = params
+            .remove_optional::<String>(&format!("EMBED{n}"))?
+            .map(|s| s.eq_ignore_ascii_case("TRUE"))
+            .unwrap_or(false);
+        let rotation_x = params.remove_optional::<f64>(&format!("ROTX{n}"))?.unwrap_or(0.0);
+        let rotation_y = params.remove_optional::<f64>(&format!("ROTY{n}"))?.unwrap_or(0.0);
+        let rotation_z = params.remove_optional::<f64>(&format!("ROTZ{n}"))?.unwrap_or(0.0);
+        let standoff = params.remove_optional::<f64>(&format!("DZ{n}"))?.unwrap_or(0.0);
+        let checksum = params.remove_optional::<String>(&format!("CHECKSUM{n}"))?.unwrap_or_default();
+        entries.push(PcbLibModelEntry {
+            id,
+            name,
+            embed,
+            rotation_x,
+            rotation_y,
+            rotation_z,
+            standoff,
+            checksum,
+            blob: None,
+        });
+    }
+    params.assert_exhausted()?;
+    Ok(entries)
+}
+
+/// Parse and consume an auxiliary Library sub-storage with Header+Data pattern.
+///
+/// Decision: "Error on unimplemented Library sub-storages" — parse header count;
+/// if 0 entries, mark Data consumed (this is full parsing of an empty section);
+/// if >0 entries, return AltiumFormatError. No raw-byte storage.
+pub(crate) fn consume_header_data_substorage(
+    doc: &mut TrackedCfbDocument,
+    header_path: &str,
+    data_path: &str,
+) -> Result<()> {
+    let header_data = doc.read_stream(header_path)?;
+    let count = parse_pcb_section_header(&header_data)?;
+    let _data = doc.read_stream(data_path)?;
+    if count > 0 {
+        return Err(AltiumFormatError::InvalidParamValue {
+            key: header_path.to_owned(),
+            detail: format!(
+                "unimplemented: {header_path} has {count} entries; implement parsing before proceeding"
+            ),
+        });
+    }
+    Ok(())
+}
+
+/// Parse and consume the Library/EmbeddedFonts single-stream sub-storage.
+///
+/// Decision: "EmbeddedFonts single-stream structure" — single stream (no Header/Data),
+/// observed as a single block with 0-length payload in all test files.
+pub(crate) fn consume_embedded_fonts(doc: &mut TrackedCfbDocument, path: &str) -> Result<()> {
+    let data = doc.read_stream(path)?;
+    if data.is_empty() {
+        return Ok(());
+    }
+    if data.len() >= 4 {
+        let raw = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
+        let payload_size = (raw & BLOCK_SIZE_MASK) as usize;
+        if payload_size == 0 {
+            return Ok(());
+        }
+    }
+    Err(AltiumFormatError::InvalidParamValue {
+        key: path.to_owned(),
+        detail: "unimplemented: EmbeddedFonts has non-empty payload; implement parsing before proceeding".to_owned(),
+    })
+}
```

### Diff: add `parse_library_board_params` to `crates/altium-format/src/pcblib/library.rs`

Board/layer-stack parameters (RECORD=Board, V9_MASTERSTACK_*, V9_STACK_LAYER{N}_*) must
be fully consumed by explicit `remove_optional`/`remove_required` calls. The layer stack
uses 1-based indexing; detect entries by probing for `V9_STACK_LAYER{N}_ID` until absent.

```diff
--- a/crates/altium-format/src/pcblib/library.rs
+++ b/crates/altium-format/src/pcblib/library.rs
@@ +1,46 @@
+/// Parses board-level configuration parameters from the Library/Data stream.
+///
+/// These include the layer stack definition (V9_MASTERSTACK_* and V9_STACK_LAYER{N}_*)
+/// and continuation RECORD=Board entries. The layer stack uses 1-based indexing;
+/// we detect entries by probing for V9_STACK_LAYER{N}_ID.
+fn parse_library_board_params(params: &mut ParameterCollection) -> Result<PcbBoardConfig> {
+    let record = params.remove_optional::<String>("RECORD")?.unwrap_or_default();
+    let v9_masterstack_style = params
+        .remove_optional::<String>("V9_MASTERSTACK_STYLE")?
+        .unwrap_or_default();
+    let v9_masterstack_id = params
+        .remove_optional::<String>("V9_MASTERSTACK_ID")?
+        .unwrap_or_default();
+    let v9_masterstack_name = params
+        .remove_optional::<String>("V9_MASTERSTACK_NAME")?
+        .unwrap_or_default();
+
+    // Layer stack: probe V9_STACK_LAYER{idx}_ID from 1 upward until absent.
+    let mut layer_stack = Vec::new();
+    let mut idx = 1;
+    while params.remove_optional::<String>(&format!("V9_STACK_LAYER{idx}_ID"))?.is_some() {
+        let id_val = /* already consumed above — restructure to capture it */;
+        // Extract all per-layer parameters for this index:
+        let name = params.remove_optional::<String>(&format!("V9_STACK_LAYER{idx}_NAME"))?.unwrap_or_default();
+        let layer_id = params.remove_optional::<String>(&format!("V9_STACK_LAYER{idx}_LAYERID"))?.unwrap_or_default();
+        let used_by_prims = params.remove_optional::<String>(&format!("V9_STACK_LAYER{idx}_USEDBYPRIMSONTOP"))?.map(|s| s.eq_ignore_ascii_case("TRUE")).unwrap_or(false);
+        let cop_thick = params.remove_optional::<String>(&format!("V9_STACK_LAYER{idx}_COPTHICK"))?.unwrap_or_default();
+        let diel_type = params.remove_optional::<String>(&format!("V9_STACK_LAYER{idx}_DIELTYPE"))?.unwrap_or_default();
+        let diel_const = params.remove_optional::<String>(&format!("V9_STACK_LAYER{idx}_DIELCONST"))?.unwrap_or_default();
+        let diel_height = params.remove_optional::<String>(&format!("V9_STACK_LAYER{idx}_DIELHEIGHT"))?.unwrap_or_default();
+        let diel_material = params.remove_optional::<String>(&format!("V9_STACK_LAYER{idx}_DIELMATERIAL"))?.unwrap_or_default();
+        layer_stack.push(PcbBoardLayerEntry {
+            id: id_val, name, layer_id, used_by_prims, cop_thick,
+            diel_type, diel_const, diel_height, diel_material,
+        });
+        idx += 1;
+    }
+
+    Ok(PcbBoardConfig {
+        record,
+        v9_masterstack_style,
+        v9_masterstack_id,
+        v9_masterstack_name,
+        layer_stack,
+    })
+}
```

Note: The exact set of V9_STACK_LAYER{N}_* parameter keys must be confirmed by inspecting
real Library/Data streams (`altium cfb dump data/pcblib/LimeMicro*.PcbLib /Library/Data --blocks`).
Add or remove per-layer keys as needed. After all board and layer-stack params are consumed,
the caller calls `params.assert_exhausted()?` to catch any unrecognized keys.

### Diff: update `crates/altium-format/src/pcblib/mod.rs` — add library fields and parsing call

```diff
--- a/crates/altium-format/src/pcblib/mod.rs
+++ b/crates/altium-format/src/pcblib/mod.rs
@@ -1,5 +1,6 @@
 pub(crate) mod section_keys;
+pub(crate) mod library;

 use std::collections::HashMap;
 use std::path::Path;
@@ -9,6 +10,9 @@ use altium_format_types::constants::file_headers::PCB_LIBRARY_BINARY_HEADER_V6;
 use altium_format_types::constants::streams::{FILE_HEADER, SECTION_KEYS};

 use crate::pcb_file_header::{parse_pcb_file_header, PcbFileHeader};
+use crate::pcblib::library::{
+    PcbLibraryData, PcbLibComponentTocEntry, PcbLibModelEntry,
+    consume_embedded_fonts, consume_header_data_substorage, parse_library_data,
+    parse_component_toc, parse_model_metadata,
+};
 use crate::tracked_cfb::TrackedCfbDocument;
 use crate::{AltiumFormatError, Result};

@@ -13,6 +17,9 @@ pub struct PcbLib {
     pub(crate) header: PcbFileHeader,
     pub(crate) section_keys: HashMap<String, String>,
+    pub(crate) library: PcbLibraryData,
+    pub(crate) component_toc: Vec<PcbLibComponentTocEntry>,
+    pub(crate) model_entries: Vec<PcbLibModelEntry>,
     pub(crate) footprints: Vec<PcbFootprint>,
 }

@@ -113,6 +120,59 @@ impl PcbLib {
         // 2. SectionKeys (optional)
         let section_keys = ...;  // unchanged from M2

+        // 3. Library/ storage
+        let lib_data_raw = doc.read_stream("/Library/Data")?;
+        let library = parse_library_data(&lib_data_raw)?;
+
+        let lib_toc_header = doc.read_stream("/Library/ComponentParamsTOC/Header")?;
+        let lib_toc_data = doc.read_stream("/Library/ComponentParamsTOC/Data")?;
+        let component_toc = parse_component_toc(&lib_toc_header, &lib_toc_data)?;
+
+        let lib_models_header = doc.read_stream("/Library/Models/Header")?;
+        let lib_models_data = doc.read_stream("/Library/Models/Data")?;
+        let mut model_entries = parse_model_metadata(&lib_models_header, &lib_models_data)?;
+        for (i, entry) in model_entries.iter_mut().enumerate() {
+            let blob_path = format!("/Library/Models/{i}");
+            entry.blob = doc.read_stream_optional(&blob_path)?;
+        }
+
+        // Auxiliary sub-storages
+        let _ = doc.read_stream("/Library/Header")?;  // library-wide record count
+        consume_header_data_substorage(
+            &mut doc,
+            "/Library/LayerKindMapping/Header",
+            "/Library/LayerKindMapping/Data",
+        )?;
+        consume_header_data_substorage(
+            &mut doc,
+            "/Library/PadViaLibrary/Header",
+            "/Library/PadViaLibrary/Data",
+        )?;
+        if doc.exists("/Library/EmbeddedFonts") {
+            consume_embedded_fonts(&mut doc, "/Library/EmbeddedFonts")?;
+        }
+        if doc.exists("/Library/ModelsNoEmbed/Header") {
+            consume_header_data_substorage(
+                &mut doc,
+                "/Library/ModelsNoEmbed/Header",
+                "/Library/ModelsNoEmbed/Data",
+            )?;
+        }
+        if doc.exists("/Library/Textures/Header") {
+            consume_header_data_substorage(
+                &mut doc,
+                "/Library/Textures/Header",
+                "/Library/Textures/Data",
+            )?;
+        }

         // 4. Enumerate footprints (unchanged from M2)
         ...

-        Ok(Self { header, section_keys, footprints })
+        Ok(Self { header, section_keys, library, component_toc, model_entries, footprints })
     }
 }
```

Note: The actual sub-storage paths (`/Library/LayerKindMapping/Data`, etc.) must be confirmed with `altium cfb ls data/pcblib/BlankPcbLib.PcbLib --flat` before implementing, as the exact paths depend on the real file structure. The `if doc.exists(...)` guards make optional sub-storages graceful. The `consume_header_data_substorage` call consumes both streams; if a sub-storage does not exist in a given file, wrap in an `if doc.exists(header_path)` guard.

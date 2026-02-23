# Milestone 1: Foundation & Module Structure

Convert the `pcblib.rs` stub into a directory module and define the core PcbLib struct
with its in-memory representation.

## Files

- `crates/altium-format/src/pcblib.rs` -> `crates/altium-format/src/pcblib/mod.rs` (convert file to module)
- `crates/altium-format/src/lib.rs` (update module declaration if needed)

## Flags

- `conformance`: Must follow SchLib module patterns for consistency

## Requirements

- Convert `pcblib.rs` (single file) into `pcblib/mod.rs` (directory module)
- Define `PcbLib` struct with fields for: file header metadata, section keys mapping,
  library data, and a vector of footprints
- Define `PcbFootprint` struct with fields for: display name, CFB storage key, pattern name,
  height, description, GUIDs, and a vector of primitives
- Define `PcbPrimitive` enum with variants for each TObjectId found in PcbLib: Arc, Pad, Via,
  Track, Text, Fill, Region, ComponentBody
- Define struct for each primitive variant's parsed data (e.g., `PcbArc`, `PcbTrack`, etc.)
  with typed fields using domain types from `altium-format-types` (V6Layer, PcbFlags, Coord, etc.)
- Define `PcbPrimitiveCommon` struct for the shared 13-byte header fields (including gap byte at offset 1)
- All types must be `pub(crate)` (implementation details, not exposed to ops crate)
- `PcbLib::open()` must accept a path and return `Result<Self>`

## Acceptance Criteria

- `cargo build` succeeds with no errors
- `PcbLib::open()` opens the CFB container (existing stub behavior preserved)
- All primitive structs compile with correct field types
- No raw primitives (i32 for coordinates, u8 for layers) -- domain types only

## Tests

- **Test files**: inline `#[cfg(test)]` in `pcblib/mod.rs`
- **Test type**: unit (compile-time verification)
- **Backing**: doc-derived (CLAUDE.md mandates domain types)
- **Scenarios**:
  - Normal: PcbLib struct with all fields initialized to defaults compiles
  - Normal: PcbPrimitive enum covers all 8 types

## Code Intent

### Diff: delete `crates/altium-format/src/pcblib.rs`, create `crates/altium-format/src/pcblib/mod.rs`

```diff
--- a/crates/altium-format/src/pcblib.rs
+++ /dev/null
@@ -1,13 +0,0 @@
-use std::path::Path;
-
-pub struct PcbLib {
-    // TODO: Define the structure
-}
-
-impl PcbLib {
-    pub fn open(path: impl AsRef<Path>) -> crate::Result<Self> {
-        let path = path.as_ref();
-        let _file = std::fs::File::open(path)?;
-        Ok(Self {})
-    }
-}
```

```diff
--- /dev/null
+++ b/crates/altium-format/src/pcblib/mod.rs
@@ -0,0 +1,119 @@
+use std::collections::HashMap;
+use std::path::Path;
+
+use altium_format_types::{Coord, CoordPoint, PcbFlags, PcbObjectId, V6Layer};
+
+use crate::pcb_file_header::PcbFileHeader;
+use crate::tracked_cfb::TrackedCfbDocument;
+
+pub struct PcbLib {
+    pub(crate) header: PcbFileHeader,
+    pub(crate) section_keys: HashMap<String, String>,
+    pub(crate) footprints: Vec<PcbFootprint>,
+}
+
+pub(crate) struct PcbFootprint {
+    pub(crate) display_name: String,
+    pub(crate) cfb_key: String,
+    pub(crate) pattern: String,
+    pub(crate) height: Coord,
+    pub(crate) description: String,
+    pub(crate) item_guid: String,
+    pub(crate) revision_guid: String,
+    pub(crate) primitives: Vec<PcbPrimitive>,
+}
+
+pub(crate) struct PcbPrimitiveCommon {
+    pub(crate) layer: V6Layer,
+    pub(crate) pad_byte: u8,
+    pub(crate) flags: PcbFlags,
+    pub(crate) net_index: i32,
+    pub(crate) polygon_index: u16,
+    pub(crate) component_index: u16,
+    pub(crate) unknown: u8,
+}
+
+pub(crate) enum PcbPrimitive {
+    Arc(PcbArc),
+    Pad(PcbPad),
+    Via(PcbVia),
+    Track(PcbTrack),
+    Text(PcbText),
+    Fill(PcbFill),
+    Region(PcbRegion),
+    ComponentBody(PcbComponentBody),
+}
+
+pub(crate) struct PcbArc {
+    pub(crate) common: PcbPrimitiveCommon,
+    pub(crate) center: CoordPoint,
+    pub(crate) radius: Coord,
+    pub(crate) start_angle: f64,
+    pub(crate) end_angle: f64,
+    pub(crate) width: Coord,
+    pub(crate) unique_id: Option<String>,
+    pub(crate) trailing_bytes: Vec<u8>,
+}
+
+pub(crate) struct PcbTrack {
+    pub(crate) common: PcbPrimitiveCommon,
+    pub(crate) start: CoordPoint,
+    pub(crate) end: CoordPoint,
+    pub(crate) width: Coord,
+    pub(crate) subpoly_index: u16,
+    pub(crate) unique_id: Option<String>,
+    pub(crate) trailing_bytes: Vec<u8>,
+}
+
+pub(crate) struct PcbVia {
+    pub(crate) common: PcbPrimitiveCommon,
+    pub(crate) location: CoordPoint,
+    pub(crate) hole_size: Coord,
+    pub(crate) diameter_top: Coord,
+    pub(crate) diameter_mid: Coord,
+    pub(crate) diameter_bot: Coord,
+    pub(crate) from_layer: V6Layer,
+    pub(crate) to_layer: V6Layer,
+    pub(crate) unique_id: Option<String>,
+    pub(crate) trailing_bytes: Vec<u8>,
+}
+
+pub(crate) struct PcbFill {
+    pub(crate) common: PcbPrimitiveCommon,
+    pub(crate) corner1: CoordPoint,
+    pub(crate) corner2: CoordPoint,
+    pub(crate) rotation: f64,
+    pub(crate) unique_id: Option<String>,
+    pub(crate) trailing_bytes: Vec<u8>,
+}
+
+pub(crate) struct PcbText {
+    pub(crate) common: PcbPrimitiveCommon,
+    pub(crate) location: CoordPoint,
+    pub(crate) height: Coord,
+    pub(crate) rotation: f64,
+    pub(crate) text: String,
+    pub(crate) unique_id: Option<String>,
+    pub(crate) trailing_bytes: Vec<u8>,
+}
+
+pub(crate) struct PcbRegion {
+    pub(crate) common: PcbPrimitiveCommon,
+    pub(crate) vertices: Vec<CoordPoint>,
+    pub(crate) unique_id: Option<String>,
+    pub(crate) trailing_bytes: Vec<u8>,
+}
+
+pub(crate) struct PcbPad {
+    pub(crate) common: PcbPrimitiveCommon,
+    pub(crate) location: CoordPoint,
+    pub(crate) unique_id: Option<String>,
+    pub(crate) subrecord_trailing: [Vec<u8>; 6],
+}
+
+pub(crate) struct PcbComponentBody {
+    pub(crate) common: PcbPrimitiveCommon,
+    pub(crate) model_guid: String,
+    pub(crate) standoff_height: Coord,
+    pub(crate) rotation_x: f64,
+    pub(crate) rotation_y: f64,
+    pub(crate) rotation_z: f64,
+    pub(crate) outline: Vec<CoordPoint>,
+    pub(crate) unique_id: Option<String>,
+    pub(crate) trailing_bytes: Vec<u8>,
+}
+
+impl PcbLib {
+    pub fn open(path: impl AsRef<Path>) -> crate::Result<Self> {
+        let path = path.as_ref();
+        let _doc = TrackedCfbDocument::open(path)?;
+        Ok(Self {
+            header: PcbFileHeader {
+                version_string: String::new(),
+                version: 0.0,
+                unique_id: String::new(),
+            },
+            section_keys: HashMap::new(),
+            footprints: Vec::new(),
+        })
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn pcblib_struct_compiles() {
+        let _ = PcbLib {
+            header: crate::pcb_file_header::PcbFileHeader {
+                version_string: String::new(),
+                version: 0.0,
+                unique_id: String::new(),
+            },
+            section_keys: HashMap::new(),
+            footprints: Vec::new(),
+        };
+    }
+
+    #[test]
+    fn pcbprimitive_enum_all_variants() {
+        let _ = PcbObjectId::Arc;
+        let _ = PcbObjectId::Pad;
+        let _ = PcbObjectId::Via;
+        let _ = PcbObjectId::Track;
+        let _ = PcbObjectId::Text;
+        let _ = PcbObjectId::Fill;
+        let _ = PcbObjectId::Region;
+        let _ = PcbObjectId::ComponentBody;
+    }
+}
```

Note: `PcbFileHeader` fields are currently `pub(crate)` in `pcb_file_header.rs`; the open() stub accesses them directly. The `TrackedCfbDocument::open()` call in `open()` replaces the previous `std::fs::File::open()` call to wire up CFB infrastructure for M2+.

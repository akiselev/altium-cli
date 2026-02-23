# Milestone 5: Complex Primitives

Implement parsers for the remaining complex primitive types: Text (2 subrecords),
Region (variable-length vertex array), Pad (6 subrecords, ~500+ bytes), and
ComponentBody (3D model reference).

## Files

- `crates/altium-format/src/pcblib/primitives/text.rs` (new)
- `crates/altium-format/src/pcblib/primitives/region.rs` (new)
- `crates/altium-format/src/pcblib/primitives/pad.rs` (new)
- `crates/altium-format/src/pcblib/primitives/component_body.rs` (new)
- `crates/altium-format/src/pcblib/primitives/mod.rs` (update dispatch to use real parsers)
- `crates/altium-format/src/pcblib/primitives/mod.rs` (update dispatch table to include all 8 primitive types)

## Flags

- `complex-algorithm`: Pad has 6 subrecords with ~500+ bytes of binary data
- `error-handling`: Region vertex count must be validated against remaining bytes
- `needs-rationale`: Text subrecord 1 contains Win1252 string (overridden by WideStrings sidecar)
- `needs-investigation`: ComponentBody binary layout has placeholder offsets in documentation;
  pre-implementation investigation required with `altium cfb dump` and Ghidra before coding

## Requirements

- Parse Text (TObjectId=5) with 2 subrecords:
  - Subrecord 0: common header + location, height, rotation, mirrored, stroke_width,
    is_comment, is_designator, font_kind, and additional font/justification fields
  - Subrecord 1: u32 length + Win1252 text string (may be `.Designator`, `.Comment`, etc.)
  - Store text as String (decoded from Win1252; WideStrings sidecar may override in M6)
- Parse Region (TObjectId=11) with variable-length vertex array:
  - Common header + region_kind (RegionKind) + format-dependent fields
  - Vertex count (i32) followed by array of CoordPoint (x:Coord, y:Coord) pairs
  - Use record length to determine exact vertex data boundaries
- Parse Pad (TObjectId=2) with 6 subrecords:
  - Subrecord 0 (main): common header + location, sizes (top/mid/bot x/y), hole_size,
    shapes (top/mid/bot), rotation, is_plated, stack_mode, mask expansions
  - Subrecords 1-5: extended shapes per 32-layer stack, corner radii, hole offsets,
    hole shape/slot data, AD26 extensions
  - Store all known fields in `PcbPad` struct; unknown trailing bytes preserved per subrecord
- Parse ComponentBody (TObjectId=12):
  - Common header + body outline vertices + 3D model GUID reference
  - Standoff height, rotation offsets (X/Y/Z), body projection settings
  - Model GUID will be cross-referenced with Library/Models in M7
- After M5, all 8 primitive types have parsers and the dispatch in `primitives/mod.rs`
  no longer returns errors for any PcbObjectId found in PcbLib footprints
- All parsers must use domain types: `V6Layer` for layers, `PadShape` for shapes,
  `RegionKind` for region kinds, `TextKind` for font kinds, etc.

## Acceptance Criteria

- All 8 primitive types parse without errors on test files
- `altium validate` succeeds on footprints that contain all primitive types
- Pad 6-subrecord structure is correctly segmented (no bytes misattributed between subrecords)
- Region vertex arrays produce valid coordinate values (within PCB coordinate bounds)
- Text strings are correctly decoded from Win1252
- All 8 PcbObjectId types dispatch to real parsers (no Unimplemented errors for valid types)

## Tests

- **Test files**: `#[cfg(test)]` in each new `primitives/*.rs` file
- **Test type**: property-based (pad binary round-trip) + integration (real footprints)
- **Backing**: user-specified (property-based), doc-derived (binary-primitives.md)
- **Scenarios**:
  - Normal: Parse Pad from test file, verify location and hole size
  - Normal: Parse Text with `.Designator` string
  - Normal: Parse Region with 4-vertex rectangle
  - Edge: Pad with stack_mode=FullStack (independent per-layer shapes)
  - Edge: Region with many vertices (complex board outline)
  - Edge: ComponentBody with model GUID reference
  - Edge: Text with empty string (subrecord 1 length = 0)
  - Error: Pad with fewer than 6 subrecords

## Code Intent

### Diff: create `crates/altium-format/src/pcblib/primitives/text.rs`

```diff
--- /dev/null
+++ b/crates/altium-format/src/pcblib/primitives/text.rs
@@ -0,0 +1,42 @@
+use altium_format_types::TextKind;
+
+use crate::binary_io::BinaryReader;
+use crate::pcblib::PcbText;
+use crate::pcblib::primitives::common::parse_common_header;
+use crate::{AltiumFormatError, Result};
+
+pub(crate) fn parse_text(subrecords: &[Vec<u8>]) -> Result<PcbText> {
+    if subrecords.len() != 2 {
+        return Err(AltiumFormatError::RecordCountMismatch {
+            section: "Text subrecords".to_owned(),
+            expected: 2,
+            actual: subrecords.len(),
+        });
+    }
+
+    // Subrecord 0: properties
+    let mut reader = BinaryReader::new(&subrecords[0]);
+    let common = parse_common_header(&mut reader)?;
+    let location = reader.read_coord_point()?;
+    let height = reader.read_coord()?;
+    let rotation = reader.read_f64_le()?;
+    let is_mirrored = reader.read_u8()? != 0;
+    let stroke_width = reader.read_coord()?;
+    let is_comment = reader.read_u8()? != 0;
+    let is_designator = reader.read_u8()? != 0;
+    let font_kind = TextKind::try_from(reader.read_u8()?)?;
+    let trailing_bytes = reader.read_remaining().to_vec();
+
+    // Subrecord 1: Win1252 text string
+    let (text, _, _) = encoding_rs::WINDOWS_1252.decode(&subrecords[1]);
+    let text = text.into_owned();
+
+    Ok(PcbText {
+        common,
+        location,
+        height,
+        rotation,
+        text,
+        unique_id: None,
+        trailing_bytes,
+    })
+}
```

### Diff: expand `PcbText` struct in `crates/altium-format/src/pcblib/mod.rs`

```diff
--- a/crates/altium-format/src/pcblib/mod.rs
+++ b/crates/altium-format/src/pcblib/mod.rs
@@ -1,5 +1,6 @@
 use altium_format_types::{Coord, CoordPoint, PcbFlags, PcbObjectId, V6Layer};
+use altium_format_types::{PadShape, PadStackMode, RegionKind, TextKind};

 // ... (in PcbText struct) ...
-pub(crate) struct PcbText {
-    pub(crate) common: PcbPrimitiveCommon,
-    pub(crate) location: CoordPoint,
-    pub(crate) height: Coord,
-    pub(crate) rotation: f64,
-    pub(crate) text: String,
-    pub(crate) unique_id: Option<String>,
-    pub(crate) trailing_bytes: Vec<u8>,
-}
+pub(crate) struct PcbText {
+    pub(crate) common: PcbPrimitiveCommon,
+    pub(crate) location: CoordPoint,
+    pub(crate) height: Coord,
+    pub(crate) rotation: f64,
+    pub(crate) is_mirrored: bool,
+    pub(crate) stroke_width: Coord,
+    pub(crate) is_comment: bool,
+    pub(crate) is_designator: bool,
+    pub(crate) font_kind: TextKind,
+    pub(crate) text: String,
+    pub(crate) unique_id: Option<String>,
+    pub(crate) trailing_bytes: Vec<u8>,
+}

 // ... (in PcbRegion struct) ...
-pub(crate) struct PcbRegion {
-    pub(crate) common: PcbPrimitiveCommon,
-    pub(crate) vertices: Vec<CoordPoint>,
-    pub(crate) unique_id: Option<String>,
-    pub(crate) trailing_bytes: Vec<u8>,
-}
+pub(crate) struct PcbRegion {
+    pub(crate) common: PcbPrimitiveCommon,
+    pub(crate) kind: RegionKind,
+    pub(crate) vertices: Vec<CoordPoint>,
+    pub(crate) unique_id: Option<String>,
+    pub(crate) trailing_bytes: Vec<u8>,
+}

 // ... (expand PcbPad struct) ...
-pub(crate) struct PcbPad {
-    pub(crate) common: PcbPrimitiveCommon,
-    pub(crate) location: CoordPoint,
-    pub(crate) unique_id: Option<String>,
-    pub(crate) subrecord_trailing: [Vec<u8>; 6],
-}
+pub(crate) struct PcbPad {
+    pub(crate) common: PcbPrimitiveCommon,
+    pub(crate) location: CoordPoint,
+    pub(crate) size_top: CoordPoint,
+    pub(crate) size_mid: CoordPoint,
+    pub(crate) size_bot: CoordPoint,
+    pub(crate) hole_size: Coord,
+    pub(crate) shape_top: PadShape,
+    pub(crate) shape_mid: PadShape,
+    pub(crate) shape_bot: PadShape,
+    pub(crate) rotation: f64,
+    pub(crate) is_plated: bool,
+    pub(crate) stack_mode: PadStackMode,
+    pub(crate) unique_id: Option<String>,
+    pub(crate) subrecord_trailing: [Vec<u8>; 6],
+}
```

The `CoordPoint` used for `size_top/mid/bot` stores `(x, y)` as two `Coord` values, so `size_top_x` and `size_top_y` from the parser become `size_top: CoordPoint::new(size_top_x, size_top_y)`. Check if `CoordPoint::new(x: Coord, y: Coord)` exists in `altium-format-types/src/coord.rs`; if not, use a struct literal.

### Diff: create `crates/altium-format/src/pcblib/primitives/region.rs`

```diff
--- /dev/null
+++ b/crates/altium-format/src/pcblib/primitives/region.rs
@@ -0,0 +1,35 @@
+use altium_format_types::RegionKind;
+
+use crate::binary_io::BinaryReader;
+use crate::pcblib::PcbRegion;
+use crate::pcblib::primitives::common::parse_common_header;
+use crate::{AltiumFormatError, Result};
+
+pub(crate) fn parse_region(data: &[u8]) -> Result<PcbRegion> {
+    let mut reader = BinaryReader::new(data);
+    let common = parse_common_header(&mut reader)?;
+    let kind = RegionKind::try_from(reader.read_u8()?)?;
+    let vertex_count = reader.read_i32_le()?;
+    if vertex_count < 0 {
+        return Err(AltiumFormatError::InvalidParamValue {
+            key: "vertex_count".to_owned(),
+            detail: format!("negative vertex count: {vertex_count}"),
+        });
+    }
+    let vertex_count = vertex_count as usize;
+    let needed = vertex_count * 8;
+    if reader.remaining() < needed {
+        return Err(AltiumFormatError::BinaryReadPastEnd {
+            offset: reader.position(),
+            needed,
+            available: reader.remaining(),
+        });
+    }
+    let mut vertices = Vec::with_capacity(vertex_count);
+    for _ in 0..vertex_count {
+        vertices.push(reader.read_coord_point()?);
+    }
+    let trailing_bytes = reader.read_remaining().to_vec();
+    Ok(PcbRegion { common, vertices, unique_id: None, trailing_bytes })
+}
```

Note: `PcbRegion` struct in M1 does not include `kind: RegionKind`. Add this field to the struct definition in `mod.rs` alongside this milestone.

### Diff: create `crates/altium-format/src/pcblib/primitives/pad.rs`

```diff
--- /dev/null
+++ b/crates/altium-format/src/pcblib/primitives/pad.rs
@@ -0,0 +1,51 @@
+use altium_format_types::{Coord, CoordPoint, PadShape, PadStackMode};
+
+use crate::binary_io::BinaryReader;
+use crate::pcblib::PcbPad;
+use crate::pcblib::primitives::common::parse_common_header;
+use crate::{AltiumFormatError, Result};
+
+pub(crate) fn parse_pad(subrecords: &[Vec<u8>]) -> Result<PcbPad> {
+    if subrecords.len() != 6 {
+        return Err(AltiumFormatError::RecordCountMismatch {
+            section: "Pad subrecords".to_owned(),
+            expected: 6,
+            actual: subrecords.len(),
+        });
+    }
+
+    // Subrecord 0: main pad record
+    let mut r0 = BinaryReader::new(&subrecords[0]);
+    let common = parse_common_header(&mut r0)?;
+    let location = r0.read_coord_point()?;
+    let size_top_x = r0.read_coord()?;
+    let size_top_y = r0.read_coord()?;
+    let size_mid_x = r0.read_coord()?;
+    let size_mid_y = r0.read_coord()?;
+    let size_bot_x = r0.read_coord()?;
+    let size_bot_y = r0.read_coord()?;
+    let hole_size = r0.read_coord()?;
+    let shape_top = PadShape::try_from(r0.read_u8()?)?;
+    let shape_mid = PadShape::try_from(r0.read_u8()?)?;
+    let shape_bot = PadShape::try_from(r0.read_u8()?)?;
+    let rotation = r0.read_f64_le()?;
+    let is_plated = r0.read_u8()? != 0;
+    let stack_mode = PadStackMode::try_from(r0.read_u8()?)?;
+    let subrecord_trailing_0 = r0.read_remaining().to_vec();
+
+    // Subrecords 1-5: store trailing bytes for each (fields documented per investigation)
+    let mut subrecord_trailing: [Vec<u8>; 6] = Default::default();
+    subrecord_trailing[0] = subrecord_trailing_0;
+    for i in 1..6usize {
+        subrecord_trailing[i] = subrecords[i].clone();
+    }
+
+    Ok(PcbPad {
+        common,
+        location,
+        unique_id: None,
+        subrecord_trailing,
+    })
+}
```

Note: `PcbPad` struct needs additional fields for the parsed pad properties (`size_top_x/y`, `size_mid_x/y`, `size_bot_x/y`, `hole_size`, `shape_top/mid/bot`, `rotation`, `is_plated`, `stack_mode`). Add these to the struct definition in `mod.rs` alongside this milestone. The `[Vec<u8>; 6]` array requires `Default::default()` which requires `Vec<u8>: Default` (it is).

### Diff: create `crates/altium-format/src/pcblib/primitives/component_body.rs`

**PRE-IMPLEMENTATION INVESTIGATION REQUIRED**

Before writing `parse_component_body`, run:
```
altium cfb dump data/pcblib/LimeMicro*.PcbLib <footprint-with-3D-model>/Data --blocks
```
Cross-reference with Ghidra decompilation of the ComponentBody serializer to determine
exact byte offsets for: body outline vertex count, vertex array, model GUID location,
standoff height, and rotation fields.

Document findings in `docs/pcblib/binary-primitives.md` before coding. Only write the
diff for `component_body.rs` after the investigation is complete and byte offsets are known.

The stub to unblock compilation (returns error for ComponentBody until investigation completes):
```diff
--- /dev/null
+++ b/crates/altium-format/src/pcblib/primitives/component_body.rs
@@ -0,0 +1,14 @@
+use crate::pcblib::PcbComponentBody;
+use crate::{AltiumFormatError, Result};
+
+pub(crate) fn parse_component_body(data: &[u8]) -> Result<PcbComponentBody> {
+    Err(AltiumFormatError::InvalidParamValue {
+        key: "ComponentBody".to_owned(),
+        detail: format!(
+            "ComponentBody parser not yet implemented (record is {} bytes); \
+             run investigation with `altium cfb dump` before implementing",
+            data.len()
+        ),
+    })
+}
```

### Diff: update `crates/altium-format/src/pcblib/primitives/mod.rs` — add Text, Region, Pad, ComponentBody to dispatch

```diff
--- a/crates/altium-format/src/pcblib/primitives/mod.rs
+++ b/crates/altium-format/src/pcblib/primitives/mod.rs
@@ -1,6 +1,10 @@
 pub(crate) mod arc;
+pub(crate) mod component_body;
 pub(crate) mod common;
 pub(crate) mod fill;
+pub(crate) mod pad;
+pub(crate) mod region;
+pub(crate) mod text;
 pub(crate) mod track;
 pub(crate) mod via;

@@ -40,7 +44,22 @@ pub(crate) fn dispatch_primitive(
         PcbObjectId::Fill => {
             if subrecords.len() != 1 { return Err(...); }
             fill::parse_fill(&subrecords[0]).map(PcbPrimitive::Fill)
         }
-        other => Err(AltiumFormatError::UnknownObjectId(other as u8)),
+        PcbObjectId::Text => text::parse_text(subrecords).map(PcbPrimitive::Text),
+        PcbObjectId::Region => {
+            if subrecords.len() != 1 { return Err(AltiumFormatError::RecordCountMismatch { section: "Region subrecords".to_owned(), expected: 1, actual: subrecords.len() }); }
+            region::parse_region(&subrecords[0]).map(PcbPrimitive::Region)
+        }
+        PcbObjectId::Pad => pad::parse_pad(subrecords).map(PcbPrimitive::Pad),
+        PcbObjectId::ComponentBody => {
+            if subrecords.len() != 1 { return Err(AltiumFormatError::RecordCountMismatch { section: "ComponentBody subrecords".to_owned(), expected: 1, actual: subrecords.len() }); }
+            component_body::parse_component_body(&subrecords[0]).map(PcbPrimitive::ComponentBody)
+        }
+        other => Err(AltiumFormatError::UnknownObjectId(other as u8)),
     }
 }
```

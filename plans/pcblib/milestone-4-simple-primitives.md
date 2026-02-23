# Milestone 4: Simple Primitives & Data Stream

Parse the per-footprint Data stream and implement binary parsers for the simpler primitive
types: Arc, Track, Via, and Fill. Also parse the Parameters and Header streams per footprint.

## Files

- `crates/altium-format/src/pcblib/primitives/mod.rs` (new - dispatch + common header)
- `crates/altium-format/src/pcblib/primitives/common.rs` (new - PcbPrimitiveCommon parser)
- `crates/altium-format/src/pcblib/primitives/arc.rs` (new)
- `crates/altium-format/src/pcblib/primitives/track.rs` (new)
- `crates/altium-format/src/pcblib/primitives/via.rs` (new)
- `crates/altium-format/src/pcblib/primitives/fill.rs` (new)
- `crates/altium-format/src/pcblib/footprint.rs` (new - Data/Parameters/Header stream loading)
- `crates/altium-format/src/pcblib/mod.rs` (integrate footprint loading)

## Flags

- `error-handling`: Binary parsing must fail fast on unexpected data
- `needs-rationale`: Version-dependent trailing bytes preserved as raw data
- `complex-algorithm`: Data stream parsing: pattern name block + binary record dispatch

## Requirements

- Parse the 13-byte common header shared by all primitives (note gap byte at offset 1):
  - `layer: V6Layer` (offset 0), `pad_byte: u8` (offset 1), `flags: PcbFlags` (offset 2),
    `net_index: i32` (offset 4), `polygon_index: u16` (offset 8),
    `component_index: u16` (offset 10), `unknown: u8` (offset 12)
- Parse Arc (TObjectId=1): common header + center_x/y (Coord), radius (Coord),
  start_angle/end_angle (f64), width (Coord). Handle legacy (45 bytes) and AD26 (58 bytes)
  variants by storing trailing bytes.
- Parse Track (TObjectId=4): common header + start_x/y, end_x/y, width (all Coord),
  subpoly_index (u16). Handle legacy (35 bytes) and AD26 (49 bytes).
- Parse Via (TObjectId=3): common header + location_x/y, hole_size, diameter_top/mid/bot (Coord),
  from_layer/to_layer (V6Layer). Handle trailing bytes.
- Parse Fill (TObjectId=6): common header + corner1_x/y, corner2_x/y (Coord), rotation (f64).
  Handle legacy (37 bytes) and AD26 (50 bytes).
- Implement a custom `parse_pcblib_data_stream()` function (NOT using `parse_pcb_binary_records()`
  from pcb_binary_stream.rs -- see Decision Log "Custom Data stream reader"):
  - Read pattern name block: `u32 block_length + u8 string_length + ASCII name`
  - Read packed binary records with multi-subrecord awareness:
    1. Read `u8` type byte (TObjectId)
    2. Determine subrecord count from type using named constants (add to
       `altium-format-types/src/constants/`): `PAD_SUBRECORD_COUNT = 6`,
       `TEXT_SUBRECORD_COUNT = 2`, `DEFAULT_SUBRECORD_COUNT = 1`. Verify values
       against C# FileFormatConsts.cs per CLAUDE.md.
    3. For each subrecord: read `u32` length + payload bytes
    4. Pass all subrecords to type-specific parser
  - For types not yet implemented (Pad, Text, Region, ComponentBody): return
    `AltiumFormatError` (Decision: "Error on unimplemented primitive types during incremental
    development"). M4 test files are pre-selected to contain only Arc/Track/Via/Fill primitives.
  - Assign sequential 0-based index to each primitive
- Parse the Parameters stream using length-prefixed parameter block format (per
  `docs/pcblib/parameters-stream.md:6-12`: u32 LE block length + u8 string length + Win1252
  parameter string). Extract PATTERN, HEIGHT, DESCRIPTION, ITEMGUID, REVISIONGUID.
- Parse the Header stream: u32 record count. Validate against actual parsed count.
- Verify pattern name from Data stream matches PATTERN from Parameters stream

## Acceptance Criteria

- Simple footprints (tracks + arcs only) parse completely without errors
- Record count from Header matches parsed primitive count
- Pattern name from Data matches PATTERN from Parameters
- Arc coordinates, angles, and widths are correctly parsed (validate with `altium cfb dump`)
- Unimplemented primitive types (Pad, Text, etc.) return AltiumFormatError (not silently stored)
- Version-variant records (legacy vs AD26) both parse without error
- Multi-subrecord framing correctly reads 1 type byte + N subrecord blocks

## Tests

- **Test files**: `#[cfg(test)]` in each `primitives/*.rs` file and `footprint.rs`
- **Test type**: property-based (binary parsing round-trip) + integration (real footprints)
- **Backing**: user-specified (property-based), doc-derived (binary-primitives.md)
- **Scenarios**:
  - Normal: Parse Arc from hand-crafted bytes matching documented layout
  - Normal: Parse Track with known start/end coordinates
  - Normal: Load a simple footprint from test file, verify primitive count
  - Edge: Arc with legacy (45-byte) vs AD26 (58-byte) record size
  - Edge: Fill with rotation = 0.0 (common case)
  - Error: Truncated record (insufficient bytes) returns BinaryReadPastEnd
  - Error: Record count mismatch between Header and actual data

## Code Intent

### Diff: add subrecord count constants to `crates/altium-format-types/src/constants/parsing.rs`

```diff
--- a/crates/altium-format-types/src/constants/parsing.rs
+++ b/crates/altium-format-types/src/constants/parsing.rs
@@ -272,3 +272,15 @@
 /// 1000.0 mm in DXP units.
 pub const C_1000_0_MM: i32 = 393_700_787;
+
+// ---------------------------------------------------------------------------
+// PcbLib Data stream subrecord counts
+// ---------------------------------------------------------------------------
+
+/// Number of subrecords for Pad primitives in PcbLib Data stream.
+/// Source: Altium file format — Pad serializer writes 6 sub-blocks.
+pub const PAD_SUBRECORD_COUNT: usize = 6;
+
+/// Number of subrecords for Text primitives in PcbLib Data stream.
+/// Source: Altium file format — Text serializer writes 2 sub-blocks.
+pub const TEXT_SUBRECORD_COUNT: usize = 2;
+
+/// Default subrecord count for all other primitive types in PcbLib Data stream.
+pub const DEFAULT_SUBRECORD_COUNT: usize = 1;
```

### Diff: create `crates/altium-format/src/pcblib/primitives/mod.rs`

```diff
--- /dev/null
+++ b/crates/altium-format/src/pcblib/primitives/mod.rs
@@ -0,0 +1,56 @@
+pub(crate) mod arc;
+pub(crate) mod common;
+pub(crate) mod fill;
+pub(crate) mod track;
+pub(crate) mod via;
+
+use altium_format_types::constants::parsing::{
+    DEFAULT_SUBRECORD_COUNT, PAD_SUBRECORD_COUNT, TEXT_SUBRECORD_COUNT,
+};
+use altium_format_types::PcbObjectId;
+
+use crate::pcblib::{PcbPrimitive};
+use crate::{AltiumFormatError, Result};
+
+pub(crate) fn subrecord_count(object_id: PcbObjectId) -> usize {
+    match object_id {
+        PcbObjectId::Pad => PAD_SUBRECORD_COUNT,
+        PcbObjectId::Text => TEXT_SUBRECORD_COUNT,
+        _ => DEFAULT_SUBRECORD_COUNT,
+    }
+}
+
+pub(crate) fn dispatch_primitive(
+    object_id: PcbObjectId,
+    subrecords: &[Vec<u8>],
+) -> Result<PcbPrimitive> {
+    match object_id {
+        PcbObjectId::Arc => {
+            if subrecords.len() != 1 {
+                return Err(AltiumFormatError::RecordCountMismatch {
+                    section: "Arc subrecords".to_owned(),
+                    expected: 1,
+                    actual: subrecords.len(),
+                });
+            }
+            arc::parse_arc(&subrecords[0]).map(PcbPrimitive::Arc)
+        }
+        PcbObjectId::Track => {
+            if subrecords.len() != 1 {
+                return Err(AltiumFormatError::RecordCountMismatch {
+                    section: "Track subrecords".to_owned(),
+                    expected: 1,
+                    actual: subrecords.len(),
+                });
+            }
+            track::parse_track(&subrecords[0]).map(PcbPrimitive::Track)
+        }
+        PcbObjectId::Via => {
+            if subrecords.len() != 1 { return Err(AltiumFormatError::RecordCountMismatch { section: "Via subrecords".to_owned(), expected: 1, actual: subrecords.len() }); }
+            via::parse_via(&subrecords[0]).map(PcbPrimitive::Via)
+        }
+        PcbObjectId::Fill => {
+            if subrecords.len() != 1 { return Err(AltiumFormatError::RecordCountMismatch { section: "Fill subrecords".to_owned(), expected: 1, actual: subrecords.len() }); }
+            fill::parse_fill(&subrecords[0]).map(PcbPrimitive::Fill)
+        }
+        other => Err(AltiumFormatError::UnknownObjectId(other as u8)),
+    }
+}
```

### Diff: create `crates/altium-format/src/pcblib/primitives/common.rs`

```diff
--- /dev/null
+++ b/crates/altium-format/src/pcblib/primitives/common.rs
@@ -0,0 +1,26 @@
+use altium_format_types::{PcbFlags, V6Layer};
+
+use crate::binary_io::BinaryReader;
+use crate::pcblib::PcbPrimitiveCommon;
+use crate::Result;
+
+pub(crate) fn parse_common_header(reader: &mut BinaryReader) -> Result<PcbPrimitiveCommon> {
+    let layer_byte = reader.read_u8()?;
+    let layer = V6Layer::try_from(layer_byte)?;
+    let pad_byte = reader.read_u8()?;
+    let flags_raw = reader.read_u16_le()?;
+    let flags = PcbFlags::new(flags_raw);
+    let net_index = reader.read_i32_le()?;
+    let polygon_index = reader.read_u16_le()?;
+    let component_index = reader.read_u16_le()?;
+    let unknown = reader.read_u8()?;
+    Ok(PcbPrimitiveCommon {
+        layer,
+        pad_byte,
+        flags,
+        net_index,
+        polygon_index,
+        component_index,
+        unknown,
+    })
+}
```

### Diff: create `crates/altium-format/src/pcblib/primitives/arc.rs`

```diff
--- /dev/null
+++ b/crates/altium-format/src/pcblib/primitives/arc.rs
@@ -0,0 +1,26 @@
+use crate::binary_io::BinaryReader;
+use crate::pcblib::PcbArc;
+use crate::pcblib::primitives::common::parse_common_header;
+use crate::Result;
+
+pub(crate) fn parse_arc(data: &[u8]) -> Result<PcbArc> {
+    let mut reader = BinaryReader::new(data);
+    let common = parse_common_header(&mut reader)?;
+    let center = reader.read_coord_point()?;
+    let radius = reader.read_coord()?;
+    let start_angle = reader.read_f64_le()?;
+    let end_angle = reader.read_f64_le()?;
+    let width = reader.read_coord()?;
+    let trailing_bytes = reader.read_remaining().to_vec();
+    Ok(PcbArc {
+        common,
+        center,
+        radius,
+        start_angle,
+        end_angle,
+        width,
+        unique_id: None,
+        trailing_bytes,
+    })
+}
```

### Diff: add `read_remaining` to `crates/altium-format/src/binary_io.rs`

This method does not currently exist and must be added:

```diff
--- a/crates/altium-format/src/binary_io.rs
+++ b/crates/altium-format/src/binary_io.rs
@@ -242,6 +242,14 @@ impl<'a> BinaryReader<'a> {
     pub(crate) fn assert_exhausted(&self) -> Result<()> {
         // (existing implementation)
     }
+
+    /// Reads all remaining bytes in the reader, advancing position to the end.
+    /// Returns an empty slice if no bytes remain.
+    pub(crate) fn read_remaining(&mut self) -> &'a [u8] {
+        let remaining = &self.data[self.pos..];
+        self.pos = self.data.len();
+        remaining
+    }
 }
```

### Diff: create `crates/altium-format/src/pcblib/primitives/track.rs`

```diff
--- /dev/null
+++ b/crates/altium-format/src/pcblib/primitives/track.rs
@@ -0,0 +1,23 @@
+use crate::binary_io::BinaryReader;
+use crate::pcblib::PcbTrack;
+use crate::pcblib::primitives::common::parse_common_header;
+use crate::Result;
+
+pub(crate) fn parse_track(data: &[u8]) -> Result<PcbTrack> {
+    let mut reader = BinaryReader::new(data);
+    let common = parse_common_header(&mut reader)?;
+    let start = reader.read_coord_point()?;
+    let end = reader.read_coord_point()?;
+    let width = reader.read_coord()?;
+    let subpoly_index = reader.read_u16_le()?;
+    let trailing_bytes = reader.read_remaining().to_vec();
+    Ok(PcbTrack {
+        common,
+        start,
+        end,
+        width,
+        subpoly_index,
+        unique_id: None,
+        trailing_bytes,
+    })
+}
```

### Diff: create `crates/altium-format/src/pcblib/primitives/via.rs`

```diff
--- /dev/null
+++ b/crates/altium-format/src/pcblib/primitives/via.rs
@@ -0,0 +1,30 @@
+use altium_format_types::V6Layer;
+
+use crate::binary_io::BinaryReader;
+use crate::pcblib::PcbVia;
+use crate::pcblib::primitives::common::parse_common_header;
+use crate::Result;
+
+pub(crate) fn parse_via(data: &[u8]) -> Result<PcbVia> {
+    let mut reader = BinaryReader::new(data);
+    let common = parse_common_header(&mut reader)?;
+    let location = reader.read_coord_point()?;
+    let hole_size = reader.read_coord()?;
+    let diameter_top = reader.read_coord()?;
+    let diameter_mid = reader.read_coord()?;
+    let diameter_bot = reader.read_coord()?;
+    let from_layer = V6Layer::try_from(reader.read_u8()?)?;
+    let to_layer = V6Layer::try_from(reader.read_u8()?)?;
+    let trailing_bytes = reader.read_remaining().to_vec();
+    Ok(PcbVia {
+        common,
+        location,
+        hole_size,
+        diameter_top,
+        diameter_mid,
+        diameter_bot,
+        from_layer,
+        to_layer,
+        unique_id: None,
+        trailing_bytes,
+    })
+}
```

### Diff: create `crates/altium-format/src/pcblib/primitives/fill.rs`

```diff
--- /dev/null
+++ b/crates/altium-format/src/pcblib/primitives/fill.rs
@@ -0,0 +1,21 @@
+use crate::binary_io::BinaryReader;
+use crate::pcblib::PcbFill;
+use crate::pcblib::primitives::common::parse_common_header;
+use crate::Result;
+
+pub(crate) fn parse_fill(data: &[u8]) -> Result<PcbFill> {
+    let mut reader = BinaryReader::new(data);
+    let common = parse_common_header(&mut reader)?;
+    let corner1 = reader.read_coord_point()?;
+    let corner2 = reader.read_coord_point()?;
+    let rotation = reader.read_f64_le()?;
+    let trailing_bytes = reader.read_remaining().to_vec();
+    Ok(PcbFill {
+        common,
+        corner1,
+        corner2,
+        rotation,
+        unique_id: None,
+        trailing_bytes,
+    })
+}
```

### Diff: create `crates/altium-format/src/pcblib/footprint.rs`

```diff
--- /dev/null
+++ b/crates/altium-format/src/pcblib/footprint.rs
@@ -0,0 +1,94 @@
+use altium_format_types::constants::parsing::BLOCK_SIZE_MASK;
+use altium_format_types::{Coord, PcbObjectId};
+
+use crate::binary_io::BinaryReader;
+use crate::param_collection::ParameterCollection;
+use crate::pcb_binary_stream::parse_pcb_section_header;
+use crate::pcblib::primitives;
+use crate::pcblib::PcbFootprint;
+use crate::tracked_cfb::TrackedCfbDocument;
+use crate::{AltiumFormatError, Result};
+
+pub(crate) fn load_footprint(
+    doc: &mut TrackedCfbDocument,
+    cfb_key: &str,
+    display_name: &str,
+) -> Result<PcbFootprint> {
+    let params_path = format!("/{cfb_key}/Parameters");
+    let header_path = format!("/{cfb_key}/Header");
+    let data_path = format!("/{cfb_key}/Data");
+
+    // 1. Parameters stream: u32 LE block length + u8 string length + Win1252 param string
+    let params_raw = doc.read_stream(&params_path)?;
+    let (pattern, height, description, item_guid, revision_guid) =
+        parse_parameters_stream(&params_raw)?;
+
+    // 2. Header stream: u32 record count
+    let header_raw = doc.read_stream(&header_path)?;
+    let expected_count = parse_pcb_section_header(&header_raw)? as usize;
+
+    // 3. Data stream: pattern name block + binary records
+    let data_raw = doc.read_stream(&data_path)?;
+    let (data_pattern, primitives_vec) = parse_pcblib_data_stream(&data_raw)?;
+
+    // Validate pattern name matches Parameters
+    if data_pattern != pattern {
+        return Err(AltiumFormatError::InvalidParamValue {
+            key: "PATTERN".to_owned(),
+            detail: format!(
+                "Data stream pattern '{}' does not match Parameters PATTERN '{}'",
+                data_pattern, pattern
+            ),
+        });
+    }
+
+    // Validate record count matches Header
+    if primitives_vec.len() != expected_count {
+        return Err(AltiumFormatError::RecordCountMismatch {
+            section: format!("{cfb_key}/Data"),
+            expected: expected_count,
+            actual: primitives_vec.len(),
+        });
+    }
+
+    Ok(PcbFootprint {
+        display_name: display_name.to_owned(),
+        cfb_key: cfb_key.to_owned(),
+        pattern,
+        height,
+        description,
+        item_guid,
+        revision_guid,
+        primitives: primitives_vec,
+    })
+}
+
+fn parse_parameters_stream(
+    data: &[u8],
+) -> Result<(String, Coord, String, String, String)> {
+    let mut reader = BinaryReader::new(data);
+    let outer_len = reader.read_u32_le()? as usize;
+    let mut block = reader.sub_reader(outer_len)?;
+    let str_len = block.read_u8()? as usize;
+    let str_bytes = block.read_bytes(str_len)?;
+    let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(str_bytes);
+    let mut params = ParameterCollection::from_str(&decoded)?;
+    let pattern = params.remove_required::<String>("PATTERN")?;
+    let height = params.remove_optional::<Coord>("HEIGHT")?.unwrap_or(Coord::ZERO);
+    let description = params.remove_optional::<String>("DESCRIPTION")?.unwrap_or_default();
+    let item_guid = params.remove_optional::<String>("ITEMGUID")?.unwrap_or_default();
+    let revision_guid = params.remove_optional::<String>("REVISIONGUID")?.unwrap_or_default();
+    params.assert_exhausted()?;
+    Ok((pattern, height, description, item_guid, revision_guid))
+}
+
+fn parse_pcblib_data_stream(data: &[u8]) -> Result<(String, Vec<crate::pcblib::PcbPrimitive>)> {
+    let mut reader = BinaryReader::new(data);
+
+    // Pattern name block: u32 block_len + u8 str_len + ASCII name
+    let block_len = reader.read_u32_le()? as usize;
+    let mut name_block = reader.sub_reader(block_len)?;
+    let str_len = name_block.read_u8()? as usize;
+    let name_bytes = name_block.read_bytes(str_len)?;
+    let pattern_name = std::str::from_utf8(name_bytes)
+        .map_err(|e| AltiumFormatError::InvalidParamValue {
+            key: "pattern_name".to_owned(),
+            detail: format!("non-UTF8 pattern name: {e}"),
+        })?
+        .to_owned();
+    name_block.assert_exhausted()?;
+
+    // Binary records: u8 type + N subrecords (each: u32 masked_len + payload)
+    let mut records = Vec::new();
+    while reader.remaining() > 0 {
+        let type_byte = reader.read_u8()?;
+        let object_id = PcbObjectId::try_from(type_byte)?;
+        let n = primitives::subrecord_count(object_id);
+        let mut subrecords = Vec::with_capacity(n);
+        for _ in 0..n {
+            let raw_len = reader.read_u32_le()?;
+            let payload_len = (raw_len & BLOCK_SIZE_MASK) as usize;
+            let payload = reader.read_bytes(payload_len)?.to_vec();
+            subrecords.push(payload);
+        }
+        let primitive = primitives::dispatch_primitive(object_id, &subrecords)?;
+        records.push(primitive);
+    }
+    reader.assert_exhausted()?;
+
+    Ok((pattern_name, records))
+}
```

### Diff: add `from_str` to `crates/altium-format/src/param_collection.rs`

`from_str_params` is private. Expose it as a `pub(crate)` method:

```diff
--- a/crates/altium-format/src/param_collection.rs
+++ b/crates/altium-format/src/param_collection.rs
@@ -218,6 +218,10 @@ impl ParameterCollection {
     // does not apply here. Only from_bytes (raw-byte path) strips %UTF8% and
     // switches to UTF-8 decoding for the value bytes.
     fn from_str_params(s: &str) -> Result<Self> {
+        Self::from_str(s)
+    }
+
+    pub(crate) fn from_str(s: &str) -> Result<Self> {
         let s = s.strip_suffix('\0').unwrap_or(s);
         let mut params = IndexMap::new();
         for segment in s.split('|') {
```

Alternatively, rename `from_str_params` to `from_str` directly and update the two call-sites in `from_utf16le_bytes` and `from_str_params`.

### Diff: update `crates/altium-format/src/pcblib/mod.rs` — wire up footprint loading

```diff
--- a/crates/altium-format/src/pcblib/mod.rs
+++ b/crates/altium-format/src/pcblib/mod.rs
@@ -1,6 +1,8 @@
 pub(crate) mod section_keys;
 pub(crate) mod library;
+pub(crate) mod primitives;
+pub(crate) mod footprint;

 // (imports and struct definitions unchanged)

@@ -175,12 +177,16 @@ impl PcbLib {
         // 4. Enumerate footprints
         let (storages, _streams) = doc.list_entries("/")?;
         let mut footprints = Vec::new();
         for storage_name in &storages {
             let name = storage_name.trim_start_matches('/');
             if name == "FileVersionInfo" || name == "Library" {
                 continue;
             }
-            // ... M2 stub: push placeholder PcbFootprint
+            let display_name = { /* reverse-lookup from section_keys, unchanged from M2 */ };
+            let fp = footprint::load_footprint(&mut doc, name, &display_name)?;
+            footprints.push(fp);
         }

         doc.assert_all_consumed()?;
         Ok(Self { header, section_keys, library, component_toc, model_entries, footprints })
     }
 }
```

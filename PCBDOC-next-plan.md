# PcbDoc: Implementation Plan (Round 2)

Four targeted fixes to move from 5/94 to ~25-30/94 passing V6 files.
All formats verified via hex dump + C# source in the research phase.

---

## Fix #1: ConstraintManager Decode Pipeline (26 files)

### What

Decode the ConstraintManager Data stream (UTF-16LE → base64 → zlib → XML string)
and store the raw XML. No XML schema parsing needed — all 26 test files have empty
constraint documents, so just validating the decode pipeline unblocks them.

### Format

```
/ConstraintManager/Header: [u32 LE] = 1 (version/flags, always 1 in test files)
/ConstraintManager/Data:   single text block containing UTF-16LE encoded string
                           → base64 decode → zlib decompress → UTF-8 XML
```

### Changes

**File: `crates/altium-format/src/pcbdoc/mod.rs`**

1. Add section data struct:
   ```rust
   pub(crate) struct ConstraintManagerSectionData {
       pub(crate) header_value: u32,
       pub(crate) xml: String,  // decompressed XML (empty string if empty document)
   }
   ```

2. Add `PcbDocSection::ConstraintManager(ConstraintManagerSectionData)` variant.

3. Add explicit dispatch handler (before the hard-error fallback at line 383):
   ```rust
   if storage_name == "ConstraintManager" {
       let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
       let data = doc.read_stream(&format!("{storage_path}/Data"))?;
       let header_value = parse_pcb_section_header(&header_data)?;
       let xml = decode_constraint_manager_data(&data)
           .with_context(|| format!("parsing {storage_path}/Data"))?;
       assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
       sections.push(PcbDocSection::ConstraintManager(
           ConstraintManagerSectionData { header_value, xml },
       ));
       continue;
   }
   ```

4. Add decode function (in `mod.rs` or a new `constraint_manager.rs` module):
   ```rust
   fn decode_constraint_manager_data(data: &[u8]) -> Result<String> {
       use base64::Engine;
       use flate2::read::ZlibDecoder;
       use std::io::Read;

       // Read single text block
       let mut blocks = crate::block_stream::iter_blocks(data);
       let block = match blocks.next() {
           Some(Ok(b)) => b,
           Some(Err(e)) => return Err(e),
           None => return Ok(String::new()), // empty data = empty constraints
       };
       // Verify no extra blocks
       if let Some(extra) = blocks.next() {
           let _ = extra?;
           return Err(AltiumFormatError::InvalidParamValue {
               key: "ConstraintManager/Data".to_owned(),
               detail: "expected single block".to_owned(),
           });
       }

       // Decode UTF-16LE to get base64 string
       let (base64_str, _, had_errors) = encoding_rs::UTF_16LE.decode(&block.data);
       if had_errors {
           return Err(AltiumFormatError::InvalidParamValue {
               key: "ConstraintManager/Data".to_owned(),
               detail: "invalid UTF-16LE encoding".to_owned(),
           });
       }
       let base64_str = base64_str.trim_end_matches('\0');
       if base64_str.is_empty() {
           return Ok(String::new());
       }

       // Base64 decode
       let compressed = base64::engine::general_purpose::STANDARD
           .decode(base64_str)
           .map_err(|e| AltiumFormatError::InvalidParamValue {
               key: "ConstraintManager/Data".to_owned(),
               detail: format!("base64 decode failed: {e}"),
           })?;

       // Zlib decompress
       let mut decoder = ZlibDecoder::new(&compressed[..]);
       let mut xml_bytes = Vec::new();
       decoder.read_to_end(&mut xml_bytes).map_err(|e| {
           AltiumFormatError::InvalidParamValue {
               key: "ConstraintManager/Data".to_owned(),
               detail: format!("zlib decompress failed: {e}"),
           }
       })?;

       // UTF-8 decode (strict — XML is always UTF-8)
       String::from_utf8(xml_bytes).map_err(|e| AltiumFormatError::InvalidParamValue {
           key: "ConstraintManager/Data".to_owned(),
           detail: format!("XML is not valid UTF-8: {e}"),
       })
   }
   ```

5. Update `section_identity()`:
   ```rust
   PcbDocSection::ConstraintManager(_) => "ConstraintManager".to_owned(),
   ```

### Dependencies

`base64` (0.22.1) and `flate2` (1.1.5) already in `altium-format/Cargo.toml`.
`encoding_rs` already imported. No new crate dependencies needed.

### Verification

```bash
cargo test -p altium-format && \
altium validate data/pcbdoc/thesis-tree-inspection.PcbDoc && \
altium validate data/pcbdoc/rover-arm.PcbDoc
```

These files previously failed on "unsupported storage '/ConstraintManager'".
After this fix they should progress past ConstraintManager and may hit
PadViaLibrary, ShapeBasedRegions6, or PrimitiveGuids next.

### Risk

Low — the encoding chain is fully verified. The only edge case is if some
files have non-empty XML that fails UTF-8 validation, but Altium's XML
serializer always produces UTF-8.

---

## Fix #2: ShapeBasedRegions6 / ShapeBasedComponentBodies6 (23 files)

### What

Add `TPolySegment` vertex format support to the region and component body parsers.
ShapeBasedRegions6 and ShapeBasedComponentBodies6 use a 37-byte extended vertex
format instead of the legacy 16-byte `(f64 x, f64 y)` pairs.

### Format (TPolySegment, Pack=1, 37 bytes)

```
u8   kind         // TPolySegmentType: 0=Line, 1=Arc
i32  vx           // vertex X (internal units, LE)
i32  vy           // vertex Y (internal units, LE)
i32  cx           // arc center X (0 for lines, LE)
i32  cy           // arc center Y (0 for lines, LE)
i32  radius       // arc radius (0 for lines, LE)
f64  angle1       // arc start angle in degrees (0 for lines, LE)
f64  angle2       // arc end angle in degrees (0 for lines, LE)
```

**Critical**: ShapeBasedRegions6 stores **N+1 vertices** for a count of N (closing
vertex duplicates the first vertex to close the contour). Legacy Regions6 stores
exactly N vertices with closing implied.

### Changes

**File: `crates/altium-format-types/src/pcb.rs`**

1. Add `PolySegmentKind` enum (or add to existing types):
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   #[repr(u8)]
   pub enum PolySegmentKind {
       Line = 0,
       Arc = 1,
   }
   ```
   With `TryFrom<u8>` impl.

**File: `crates/altium-format/src/pcblib/mod.rs`**

2. Add `PolySegment` struct and update contour types:
   ```rust
   /// A single edge/vertex from a TPolySegment record in ShapeBasedRegions6.
   #[derive(Debug, Clone)]
   pub(crate) struct PolySegment {
       pub(crate) kind: PolySegmentKind,
       pub(crate) vertex: CoordPoint,
       pub(crate) center: CoordPoint,    // (0, 0) for line segments
       pub(crate) radius: Coord,         // 0 for line segments
       pub(crate) angle1: f64,           // 0.0 for line segments
       pub(crate) angle2: f64,           // 0.0 for line segments
   }

   /// A contour: either legacy f64 vertices or extended TPolySegment edges.
   #[derive(Debug, Clone)]
   pub(crate) enum Contour {
       /// Legacy Regions6: N × (f64 x, f64 y) pairs, closing implied.
       Legacy(Vec<CoordPoint>),
       /// ShapeBasedRegions6: (N+1) × TPolySegment, closing vertex explicit.
       ShapeBased(Vec<PolySegment>),
   }
   ```

3. Update `PcbRegion` struct:
   ```rust
   // Replace:
   pub(crate) outline: Vec<CoordPoint>,
   pub(crate) holes: Vec<Vec<CoordPoint>>,
   // With:
   pub(crate) outline: Contour,
   pub(crate) holes: Vec<Contour>,
   ```

4. Update `PcbComponentBody` struct similarly:
   ```rust
   // Replace:
   pub(crate) outline: Vec<CoordPoint>,
   // With:
   pub(crate) outline: Contour,
   ```

**File: `crates/altium-format/src/pcblib/primitives/region.rs`**

5. Add `read_polysegment_contour()` function:
   ```rust
   /// Reads a shape-based contour: i32 edge_count + (edge_count + 1) × TPolySegment.
   fn read_polysegment_contour(reader: &mut BinaryReader, label: &str) -> Result<Vec<PolySegment>> {
       let edge_count_raw = reader.read_i32_le()?;
       if edge_count_raw < 0 {
           return Err(AltiumFormatError::InvalidParamValue {
               key: format!("Region.{}_edge_count", label),
               detail: format!("edge_count must be >= 0, got {}", edge_count_raw),
           });
       }
       let edge_count = edge_count_raw as usize;
       let vertex_count = edge_count + 1; // closing vertex
       let bytes_needed = vertex_count * 37;
       if reader.remaining() < bytes_needed {
           return Err(AltiumFormatError::BinaryReadPastEnd {
               offset: reader.position(),
               needed: bytes_needed,
               available: reader.remaining(),
           });
       }
       let mut segments = Vec::with_capacity(vertex_count);
       for _ in 0..vertex_count {
           let kind = PolySegmentKind::try_from(reader.read_u8()?)?;
           let vx = reader.read_i32_le()?;
           let vy = reader.read_i32_le()?;
           let cx = reader.read_i32_le()?;
           let cy = reader.read_i32_le()?;
           let radius = reader.read_i32_le()?;
           let angle1 = reader.read_f64_le()?;
           let angle2 = reader.read_f64_le()?;
           segments.push(PolySegment {
               kind,
               vertex: CoordPoint::new(Coord::from_internal(vx), Coord::from_internal(vy)),
               center: CoordPoint::new(Coord::from_internal(cx), Coord::from_internal(cy)),
               radius: Coord::from_internal(radius),
               angle1,
               angle2,
           });
       }
       Ok(segments)
   }
   ```

6. Add `is_shape_based: bool` parameter to `parse_region()`:
   ```rust
   pub(crate) fn parse_region(data: &[u8], is_shape_based_section: bool) -> Result<PcbRegion> {
   ```
   - When `is_shape_based_section && is_shape_based` (both section kind AND param flag):
     use `read_polysegment_contour()` and wrap in `Contour::ShapeBased(..)`
   - Otherwise: use `read_f64_contour()` and wrap in `Contour::Legacy(..)`

**File: `crates/altium-format/src/pcblib/primitives/component_body.rs`**

7. Add same `is_shape_based_section: bool` parameter to `parse_component_body()`.
   Replace inline f64 vertex reading with call to `read_f64_contour()` (legacy)
   or `read_polysegment_contour()` (shape-based).

**File: `crates/altium-format/src/pcbdoc/primitives.rs`**

8. Thread section kind through to primitive parsers:
   ```rust
   fn parse_primitive_payload(
       object_id: PcbObjectId,
       payload: &[u8],
       kind: PrimitiveSectionKind,
   ) -> Result<PcbPrimitive> {
       match object_id {
           PcbObjectId::Region => {
               let shape_based = matches!(
                   kind,
                   PrimitiveSectionKind::ShapeBasedRegions6
               );
               parse_region(payload, shape_based).map(PcbPrimitive::Region)
           }
           PcbObjectId::ComponentBody => {
               let shape_based = matches!(
                   kind,
                   PrimitiveSectionKind::ShapeBasedComponentBodies6
               );
               parse_component_body(payload, shape_based).map(PcbPrimitive::ComponentBody)
           }
           // ... rest unchanged
       }
   }
   ```

**Update callers of `outline` field**: All code accessing `PcbRegion.outline` and
`PcbComponentBody.outline` must be updated from `Vec<CoordPoint>` to `Contour`.
This includes:
- Invariant validation in `pcbdoc/mod.rs`
- Serialization in `pcblib/mod.rs` (region and component body serializers)
- Any query methods

### Verification

```bash
cargo test -p altium-format && \
for f in test-padshapes test-vias textbook-5v-regulator; do
  altium validate "data/pcbdoc/${f}.PcbDoc"
done
```

### Risk

Medium — the N+1 vertex convention needs careful handling. If any files have
`ISSHAPEBASED=FALSE` in a ShapeBasedRegions6 section (both co-exist in some
files), those records should still use legacy f64 vertices. The `is_shape_based`
parameter string flag is the authoritative indicator, not just the section kind.

**Fallback**: If the N+1 convention fails for some files, try reading exactly N
TPolySegment records and add the closing vertex synthetically.

---

## Fix #3: PadViaLibrary Multi-Record Template Format (18 files)

### What

Extend `parse_pad_via_library()` to read template records after the config block.
Template records use different framing: `[u8 index][u32 len][params]` instead of
standard text blocks.

### Format (verified from rover-arm.PcbDoc hex dump)

```
Header: u32 = template_count (NOT total block count)

Data stream:
  [standard text block]     Config: |PADVIALIBRARY.LIBRARYID=...|LIBRARYNAME=...|DISPLAYUNITS=...|
  [u8 index=2]              Template 1 index (starts at 2, increments)
  [u32 param_len]           Template 1 param string length
  [param_len bytes]         |TEMPLATE.EXTERNALLINK.LIBRARYID=...|TEMPLATE.TEMPLATENAME=...|...|NUL
  [u8 index=3]              Template 2 index
  [u32 param_len]           Template 2 param string length
  [param_len bytes]         |TEMPLATE.*|...|NUL
  ...
```

### Changes

**File: `crates/altium-format/src/pcblib/library.rs`**

1. Add template struct:
   ```rust
   pub(crate) struct PcbPadViaTemplate {
       pub(crate) index: u8,
       pub(crate) params: ParameterCollection,
   }
   ```

2. Add `templates` field to `PcbPadViaLibraryConfig`:
   ```rust
   pub(crate) struct PcbPadViaLibraryConfig {
       pub(crate) library_id: String,
       pub(crate) library_name: String,
       pub(crate) display_units: String,
       pub(crate) templates: Vec<PcbPadViaTemplate>,
   }
   ```

3. Update `parse_pad_via_library()` (lines 454-495):
   ```rust
   pub(crate) fn parse_pad_via_library(
       header: &[u8],
       data: &[u8],
   ) -> Result<Option<PcbPadViaLibraryConfig>> {
       let template_count = parse_pcb_section_header(header)? as usize;
       if data.is_empty() {
           return Ok(None);
       }

       // Read config block (standard text block framing)
       let mut blocks_iter = iter_blocks(data);
       let block = match blocks_iter.next() {
           Some(Ok(b)) => b,
           Some(Err(e)) => return Err(e),
           None => return Ok(None),
       };
       let mut params = ParameterCollection::from_bytes(&block.data)?;
       let library_id = params.remove_optional::<String>("PADVIALIBRARY.LIBRARYID")?.unwrap_or_default();
       let library_name = params.remove_optional::<String>("PADVIALIBRARY.LIBRARYNAME")?.unwrap_or_default();
       let display_units = params.remove_optional::<String>("PADVIALIBRARY.DISPLAYUNITS")?.unwrap_or_default();
       params.assert_exhausted()?;

       // Read template records: [u8 index][u32 len][params] × template_count
       // These are NOT standard text blocks — they use custom framing.
       let config_block_end = block.offset + 4 + block.data.len();
       let remaining = &data[config_block_end..];
       let mut reader = BinaryReader::new(remaining);
       let mut templates = Vec::with_capacity(template_count);

       for i in 0..template_count {
           let index = reader.read_u8()?;
           let param_len = reader.read_u32_le()? as usize;
           let param_bytes = reader.read_bytes(param_len)?;
           let tpl_params = ParameterCollection::from_bytes(param_bytes)
               .with_context(|| format!("PadViaLibrary template {i} (index={index})"))?;
           // Don't assert_exhausted on template params — they have many
           // TEMPLATE.* keys we don't parse yet. Just store them.
           templates.push(PcbPadViaTemplate {
               index,
               params: tpl_params,
           });
       }
       reader.assert_exhausted()?;

       Ok(Some(PcbPadViaLibraryConfig {
           library_id,
           library_name,
           display_units,
           templates,
       }))
   }
   ```

   **Note about assert_exhausted on template params**: The template param blocks
   contain dozens of `TEMPLATE.*` keys (TEMPLATENAME, TEMPLATEID, PAD.ISMULTILAYER,
   VIA.HOLESIZE, STACKDATA0.*, etc.). We should NOT call `assert_exhausted()` on
   them yet — just store the `ParameterCollection` for now. A future pass can type
   these fields when we need pad/via template editing.

4. Update `serialize_pad_via_library()` to also write template records
   (at `pcblib/mod.rs:1498-1504`).

5. Update PcbLib's handling (if it also passes through `parse_pad_via_library`).

### Verification

```bash
cargo test -p altium-format && \
altium validate data/pcbdoc/rover-arm.PcbDoc && \
altium validate data/pcbdoc/heron-feather.PcbDoc
```

### Risk

Low-Medium — format is hex-verified. The main uncertainty is whether some files
have 0 templates (no template data after config block). The Header count = 0
case should be handled by the existing `data.is_empty()` check or the loop
executing 0 times.

**Edge case**: `block.offset` field availability — need to verify that the
`Block` struct from `iter_blocks` exposes the offset after the block. If not,
calculate from `4 (header) + block.data.len()`.

---

## Fix #4: PrimitiveGuids in PcbDoc (11 files)

### What

Add PrimitiveGuids dispatch handler in PcbDoc. The parser already exists in
`pcblib/sidecar.rs` — we just need to wire it up and handle the PcbDoc ObjectId
difference (upper bytes contain metadata beyond ViewableObjectId).

### Format (verified: 24-byte raw binary records, NOT block-framed)

```
Header: u32 = entry_count
Data:   entry_count × 24 bytes, each:
          i32  ObjectId       (low byte = ViewableObjectId, upper bytes = metadata)
          i32  IndexForSave
          u8   GUID[16]
```

### Changes

**File: `crates/altium-format/src/pcblib/sidecar.rs`**

1. Relax the ObjectId validation for PcbDoc compatibility. Current code at line 225
   does `u8::try_from(object_id_value)` which rejects PcbDoc values > 255.

   Option A (recommended): Add a second parse function for PcbDoc that stores the
   full i32 ObjectId:
   ```rust
   pub(crate) fn parse_primitive_guids_pcbdoc(
       header_data: &[u8],
       data: &[u8],
   ) -> Result<Vec<PrimitiveGuidEntryPcbDoc>> {
       let count = parse_pcb_section_header(header_data)? as usize;
       let expected_bytes = count * PRIMITIVE_GUID_RECORD_SIZE;
       if data.len() != expected_bytes {
           return Err(/* size mismatch error */);
       }
       let mut reader = BinaryReader::new(data);
       let mut entries = Vec::with_capacity(count);
       for _ in 0..count {
           let object_id_raw = reader.read_i32_le()?;
           let index_for_save = reader.read_i32_le()?;
           let mut guid = [0u8; 16];
           guid.copy_from_slice(reader.read_bytes(16)?);
           entries.push(PrimitiveGuidEntryPcbDoc {
               object_id_raw,
               index_for_save,
               guid,
           });
       }
       reader.assert_exhausted()?;
       Ok(entries)
   }
   ```

   Or Option B: Make the existing `PrimitiveGuidEntry.object_id` field store the raw
   i32 and defer ViewableObjectId validation to a separate accessor.

**File: `crates/altium-format/src/pcbdoc/mod.rs`**

2. Add section data struct:
   ```rust
   pub(crate) struct PrimitiveGuidsSectionData {
       pub(crate) entries: Vec<sidecar::PrimitiveGuidEntryPcbDoc>,
   }
   ```

3. Add `PcbDocSection::PrimitiveGuids(PrimitiveGuidsSectionData)` variant.

4. Add dispatch handler (before the hard-error fallback):
   ```rust
   if storage_name == "PrimitiveGuids" {
       let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
       let data = doc.read_stream(&format!("{storage_path}/Data"))?;
       let entries = sidecar::parse_primitive_guids_pcbdoc(&header_data, &data)
           .with_context(|| format!("parsing {storage_path}"))?;
       assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
       sections.push(PcbDocSection::PrimitiveGuids(
           PrimitiveGuidsSectionData { entries },
       ));
       continue;
   }
   ```

5. Update `section_identity()`:
   ```rust
   PcbDocSection::PrimitiveGuids(_) => "PrimitiveGuids".to_owned(),
   ```

### Verification

```bash
cargo test -p altium-format && \
altium validate data/pcbdoc/artiq-hvsup-isol.PcbDoc && \
altium validate data/pcbdoc/cobra.PcbDoc
```

### Risk

Low — the 24-byte record format is confirmed (header × 24 = data size exactly).
The only question is whether some files have ObjectId values where the low byte
is out of range for ViewableObjectId. If so, we store the raw i32 and skip
ViewableObjectId validation for now.

---

## Implementation Order

Recommended order based on independence and unblocking potential:

1. **Fix #1 (ConstraintManager)** — 26 files, standalone, no struct changes
2. **Fix #4 (PrimitiveGuids)** — 11 files, standalone, small change
3. **Fix #3 (PadViaLibrary)** — 18 files, standalone, moderate change
4. **Fix #2 (ShapeBasedRegions6)** — 23 files, largest change (struct refactor)

Fixes 1, 3, and 4 are independent and can be implemented in parallel.
Fix 2 has the widest blast radius (Contour enum affects Region/ComponentBody
structs, serialization, tests, and invariant validation).

### Expected outcome

After all 4 fixes: **~25-30 of 94 V6 files should pass** (up from 5).
The remaining failures will be EmbeddedFonts6 (7), DrillManager (3),
WideStrings6 (1), and files that currently fail on one of these 4 issues
but will then progress to hit one of these remaining issues or a new one.

---

## Pre-merge checklist

- [ ] `cargo test -p altium-format` passes
- [ ] `cargo test -p altium-format --features test-fixtures` passes
- [ ] Full validation sweep shows expected improvements
- [ ] No forbidden patterns: `opaque|raw_payload|unknown_bytes|unparsed`
- [ ] All new parsers use `.context()` / `.with_context()` for error chains
- [ ] New section variants added to `section_identity()` match
- [ ] No `.ok()` or `.unwrap_or_default()` on fallible parse operations

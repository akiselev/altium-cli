# Manual Verification Guide

How to manually verify altium-cli support for each Altium Designer file type.

## Prerequisites

1. **Build the CLI**:
   ```bash
   cargo build --release
   alias altium='./target/release/altium-cli'
   ```

2. **Clone test fixtures** (if missing):
   ```bash
   git clone https://github.com/akiselev/altium-cli-test-schlib data/schlib
   git clone https://github.com/akiselev/altium-cli-test-pcblib data/pcblib
   git clone https://github.com/akiselev/altium-cli-test-schdoc data/schdoc
   git clone https://github.com/akiselev/altium-cli-test-pcbdoc data/pcbdoc
   git clone https://github.com/akiselev/altium-cli-test-intlib data/intlib
   ```

3. **Run the test suite** to confirm baseline:
   ```bash
   cargo test --workspace
   ```

---

## 1. SchLib (Schematic Library)

### What gets validated

`altium validate` checks:
- Header component count matches loaded components
- No duplicate library references in header index
- Each component's `lib_reference` and `part_count` match header index
- `all_pin_count` is non-negative
- OWNERINDEX validity (no negative, no out-of-range, no forward references)
- Parent-child record type constraints (Implementation→ImplementationList, etc.)
- Alias consistency (aliases in header index match global alias list)

### Manual verification steps

#### a) Validate all fixtures
```bash
for f in data/schlib/*.SchLib; do
  echo "=== $(basename "$f") ==="
  altium validate "$f" 2>&1 || echo "FAIL: $f"
done
```

Any failure means we hit an unrecognized record, parameter, or structural invariant
violation. This is the primary red/green loop — each failure is a bug to fix.

#### b) Roundtrip test (save-as)
```bash
altium save-as data/schlib/SomeLib.SchLib /tmp/roundtrip.SchLib
altium validate /tmp/roundtrip.SchLib
altium cfb diff data/schlib/SomeLib.SchLib /tmp/roundtrip.SchLib --blocks
```

A clean roundtrip means:
- All records parsed and re-serialized correctly
- Parameter ordering preserved
- Sidecar streams (9 per component: PinFrac → PinFunctionData) written correctly
- Block headers and encoding intact

#### c) Inspect structure
```bash
# List all streams/storages
altium cfb ls data/schlib/SomeLib.SchLib

# Inspect FileHeader blocks
altium cfb blocks data/schlib/SomeLib.SchLib /FileHeader

# Inspect a component's data stream
altium cfb blocks data/schlib/SomeLib.SchLib /SomeComponent/Data
altium cfb blocks data/schlib/SomeLib.SchLib /SomeComponent/Data --block 0
```

Verify:
- `/FileHeader` exists with component index and font table
- Each component has a `/Data` stream
- Sidecar streams present per component (PinTextData, PinWideText, etc.)
- `/SectionKeys` present if any component name exceeds 31 characters

#### d) Render visual check
```bash
altium render data/schlib/SomeLib.SchLib --format svg --output-dir /tmp/schlib-render/
# Open SVGs in browser and compare against Altium Designer
```

#### e) Ops (operations) test
```bash
cat > /tmp/test.ops <<'EOF'
add_component $comp {
  lib_reference = "TEST_RES"
  description = "Test resistor"
}
add_pin $pin1 to $comp {
  name = "1"
  designator = "1"
}
query $q from $comp { fields = [lib_reference, description] }
EOF
altium ops apply --spec-file /tmp/test.ops data/schlib/SomeLib.SchLib --output /tmp/modified.SchLib
altium validate /tmp/modified.SchLib
```

#### f) Automated tests
```bash
cargo test -p altium-format schlib
cargo test -p altium-format-spec --test ops_e2e_schlib
cargo test -p altium-format-spec --test executor_proptest
```

### Known format details to watch for

- **PinWideText is authoritative** — overrides PinDesc sidecar
- **OWNERINDEX is component-relative** in storage, adjusted to absolute during load
- **Aliases** are stored as redirect storages containing `|SECTIONNAME=<real_name>\0`
- **Component names > 31 chars** use `/SectionKeys` mapping to obfuscated storage keys

---

## 2. SchDoc (Schematic Document)

### What gets validated

`altium validate` checks:
- OWNERINDEX validity in both `/FileHeader` and `/Additional` record sections
- Owner indices don't point out-of-range
- Embedded SchImage records reference valid storage objects
- Cross-references between `/FileHeader` and `/Additional` sections valid

### Manual verification steps

#### a) Validate all fixtures
```bash
for f in data/schdoc/*.SchDoc; do
  echo "=== $(basename "$f") ==="
  altium validate "$f" 2>&1 || echo "FAIL: $f"
done
```

#### b) Roundtrip test
```bash
altium save-as data/schdoc/SomeDoc.SchDoc /tmp/roundtrip.SchDoc
altium validate /tmp/roundtrip.SchDoc
altium cfb diff data/schdoc/SomeDoc.SchDoc /tmp/roundtrip.SchDoc --blocks
```

#### c) Inspect structure
```bash
altium cfb ls data/schdoc/SomeDoc.SchDoc

# FileHeader contains all primitives as a flat list
altium cfb blocks data/schdoc/SomeDoc.SchDoc /FileHeader
altium cfb blocks data/schdoc/SomeDoc.SchDoc /FileHeader --block 0

# Check for Additional stream (overflow objects)
altium cfb blocks data/schdoc/SomeDoc.SchDoc /Additional 2>/dev/null

# Check extended streams
altium cfb ls data/schdoc/SomeDoc.SchDoc --flat | grep -E "ReuseBlocks|Harness|ObjectDef"
```

Verify:
- First block in `/FileHeader` is RECORD=0 header with HEADER, WEIGHT, MinorVersion
- WEIGHT value matches total record count
- `/Additional` stream present if document has overflow objects
- Extended streams (`/ReuseBlocks`, `/ReuseBlocksV2`, etc.) present for complex documents

#### d) Render visual check
```bash
altium render data/schdoc/SomeDoc.SchDoc --format svg --output-dir /tmp/schdoc-render/
```

#### e) Ops test
```bash
cat > /tmp/test-schdoc.ops <<'EOF'
add_component $comp {
  lib_reference = "RES"
  design_item_id = "RES"
}
add_pin $pin to $comp {
  name = "1"
  designator = "1"
}
query $q from $comp { fields = [lib_reference] }
EOF
altium ops apply --spec-file /tmp/test-schdoc.ops data/schdoc/SomeDoc.SchDoc --output /tmp/modified.SchDoc
altium validate /tmp/modified.SchDoc
```

#### f) Automated tests
```bash
cargo test -p altium-format schdoc
cargo test -p altium-format-spec --test ops_e2e_schdoc
cargo test -p altium-format-spec --test executor_schdoc_proptest
```

### Known format details to watch for

- **Flat ownership model** — all records in one list, parent-child via OWNERINDEX
- **Extended records** (RECORD > 225) sorted ascending by type; standard records preserve insertion order
- **RECORD >= 256** written as `RECORD=254` + `RECORDEX=<actual_value>`
- **Auto-junctions** appended at very end of the record list
- **Font table** embedded in RECORD=31 (Sheet) via ExportStyleAndFontTable
- **ASCII subdirectory** in fixtures contains older ASCII-format SchDocs

---

## 3. PcbLib (PCB Footprint Library)

### What gets validated

`altium validate` currently parses the file and checks that all streams, records, and
primitives can be decoded without error. Structural invariant checks are minimal (the
validator returns Ok if parsing succeeds).

### Manual verification steps

#### a) Validate all fixtures
```bash
for f in data/pcblib/*.PcbLib; do
  echo "=== $(basename "$f") ==="
  altium validate "$f" 2>&1 || echo "FAIL: $f"
done
```

#### b) Inspect structure
```bash
altium cfb ls data/pcblib/SomeLib.PcbLib

# Library-level metadata
altium cfb blocks data/pcblib/SomeLib.PcbLib /FileHeader
altium cfb blocks data/pcblib/SomeLib.PcbLib /Library/ComponentParamsTOC/Data

# Footprint data
altium cfb blocks data/pcblib/SomeLib.PcbLib /SomeFootprint/Parameters
altium cfb blocks data/pcblib/SomeLib.PcbLib /SomeFootprint/Header
altium cfb dump data/pcblib/SomeLib.PcbLib /SomeFootprint/Data --blocks

# WideStrings (parameter-block format, NOT binary TLV!)
altium cfb blocks data/pcblib/SomeLib.PcbLib /SomeFootprint/WideStrings
```

Verify:
- `/FileHeader` contains version string and format version
- `/Library/` storage exists with shared library data
- Each footprint has `Parameters`, `Header`, `Data`, and sidecar streams
- `/SectionKeys` present if any footprint name exceeds 31 characters
- `WideStrings` stream uses parameter-block format (pipe-delimited key=value)

#### c) Check binary primitive records
```bash
# Hex dump of footprint data to inspect primitive records
altium cfb dump data/pcblib/SomeLib.PcbLib /SomeFootprint/Data --blocks --limit 256

# Check sidecar streams
altium cfb ls data/pcblib/SomeLib.PcbLib --flat | grep -i "wide\|unique\|extended\|guid"
```

Each record in the Data stream starts with a pattern name block followed by binary
primitives: `[u8 ObjectID][u32 LE length][N-byte payload]`.

#### d) Render visual check
```bash
altium render data/pcblib/SomeLib.PcbLib --format svg --output-dir /tmp/pcblib-render/
# Compare rendered footprints against Altium Designer
```

#### e) Ops test
```bash
cat > /tmp/test-pcblib.ops <<'EOF'
add_track $t to * {
  layer = top
  x1 = 0
  y1 = 0
  x2 = 100000
  y2 = 0
  width = 10000
}
query $q from * { fields = [pattern] }
EOF
altium ops apply --spec-file /tmp/test-pcblib.ops data/pcblib/SomeLib.PcbLib --output /tmp/modified.PcbLib
altium validate /tmp/modified.PcbLib
```

#### f) Automated tests
```bash
cargo test -p altium-format pcblib
cargo test -p altium-format-spec --test executor_pcb_proptest
```

### Known format details to watch for

- **WideStrings format differs from PcbDoc** — PcbLib uses parameter-block format (pipe-delimited), PcbDoc uses binary TLV encoding
- **Binary primitives** — packed little-endian structs, no padding between records
- **Coordinates** — 10,000 internal units = 1 mil (i32 values)
- **Footprint names > 31 chars** use `/SectionKeys` obfuscation

---

## 4. PcbDoc (PCB Document)

### What gets validated

`altium validate` checks:
- Non-empty legacy header
- Correct v6 binary header (`"PCB 6.0 Binary File"`)
- Valid finite version number > 0.0
- No duplicate sections
- UnionNames format version == 1
- EmbeddedFonts header count matches entries count

### Manual verification steps

#### a) Validate all fixtures
```bash
for f in data/pcbdoc/*.PcbDoc; do
  echo "=== $(basename "$f") ==="
  altium validate "$f" 2>&1 || echo "FAIL: $f"
done
```

#### b) Inspect structure
```bash
altium cfb ls data/pcbdoc/SomeDoc.PcbDoc

# Check header
altium cfb dump data/pcbdoc/SomeDoc.PcbDoc /FileHeader --blocks

# List all section streams (fixed 23-section order)
altium cfb ls data/pcbdoc/SomeDoc.PcbDoc --flat | head -60

# Inspect a section's header (record count)
altium cfb dump data/pcbdoc/SomeDoc.PcbDoc /Pads6/Header --limit 32
altium cfb dump data/pcbdoc/SomeDoc.PcbDoc /Pads6/Data --limit 256
```

Verify the fixed section order:
1. Board6, Arcs6, Pads6, Vias6, Tracks6, Texts6, Fills6
2. Connections6, Nets6, Components6, Polygons6
3. Dimensions6, Coordinates6, Classes6, Rules6, FromTos6, Embeddeds6
4. Sidecar streams: WideStrings6, UniqueIDPrimitiveInformation, ExtendedPrimitiveInformation, PrimitiveGuids

#### c) Check sidecar streams
```bash
# WideStrings6 uses binary TLV encoding (types: 0x06/0x0C/0x12/0x14)
altium cfb dump data/pcbdoc/SomeDoc.PcbDoc /WideStrings6/Data --limit 128

# UniqueID sidecar
altium cfb blocks data/pcbdoc/SomeDoc.PcbDoc /UniqueIDPrimitiveInformation/Data 2>/dev/null

# Extended primitive info
altium cfb ls data/pcbdoc/SomeDoc.PcbDoc --flat | grep -i extended
```

#### d) Check section record counts
```bash
# Each section's Header stream contains a u32 record count
# Compare against actual records in Data stream
for section in Board6 Arcs6 Pads6 Vias6 Tracks6 Texts6 Fills6; do
  echo "--- $section ---"
  altium cfb cat data/pcbdoc/SomeDoc.PcbDoc "/$section/Header" 2>/dev/null | xxd | head -1
done
```

#### e) Ops test
```bash
cat > /tmp/test-pcbdoc.ops <<'EOF'
add_track $t {
  layer = top
  x1 = 0
  y1 = 0
  x2 = 500000
  y2 = 0
  width = 10000
}
query $q { fields = [net_name] }
EOF
altium ops apply --spec-file /tmp/test-pcbdoc.ops data/pcbdoc/SomeDoc.PcbDoc --output /tmp/modified.PcbDoc
altium validate /tmp/modified.PcbDoc
```

#### f) Automated tests
```bash
cargo test -p altium-format pcbdoc
cargo test -p altium-format-spec --test executor_pcb_proptest
```

### Known format details to watch for

- **Binary records** — `[u8 ObjectID][u32 LE length][N-byte payload]` per primitive
- **Section order is fixed** — 23 sections in strict order per `RegisterAllSectionsForExporting`
- **WideStrings6 uses binary TLV** — NOT the parameter-block format used by PcbLib
- **Sidecar matching** — records matched by `(ObjectId, IndexForSave)` pairs
- **Coordinates** — i32, 10,000 units = 1 mil
- **No roundtrip save-as yet** — PcbDoc/PcbLib save-as is not implemented in the CLI

---

## 5. Cross-Cutting Verification

### Version header check
```bash
altium get version data/schlib/SomeLib.SchLib
altium get version data/pcblib/SomeLib.PcbLib
```

### Full automated test suite
```bash
# All unit tests
cargo test -p altium-format

# All integration tests
cargo test -p altium-format-spec

# All property-based tests (proptest)
cargo test -p altium-format-spec --test executor_proptest
cargo test -p altium-format-spec --test executor_schdoc_proptest
cargo test -p altium-format-spec --test executor_pcb_proptest

# Everything
cargo test --workspace
```

### CFB diff workflow (for any file type)
When debugging a roundtrip or format mismatch:
```bash
# 1. Identify differing streams
altium cfb diff original.file roundtripped.file --blocks

# 2. Drill into the differing stream
altium cfb blocks original.file /SomeStream
altium cfb blocks original.file /SomeStream --block 0

# 3. Hex dump with block annotations
altium cfb dump original.file /SomeStream --blocks

# 4. Raw bytes for external tools
altium cfb cat original.file /SomeStream | xxd
```

### Semantic CFB diff (in tests)
For programmatic roundtrip verification, use the test utility:
```rust
use crate::test_utils::assert_cfb_files_semantic_eq;

let tmp = tempfile::NamedTempFile::new().unwrap();
doc.save(tmp.path()).unwrap();
assert_cfb_files_semantic_eq(original_path, tmp.path());
```

This compares:
- Entry sets and kinds (stream vs storage)
- Block framing and types (text vs binary)
- Text blocks as **order-agnostic parameter pairs** (tolerates reordering)
- Binary blocks byte-for-byte
- Embedded objects (0xD0 envelope) in **decompressed** form (ignores zlib differences)

---

## 6. Coverage Summary

| Capability        | SchLib | SchDoc | PcbLib | PcbDoc |
|-------------------|--------|--------|--------|--------|
| Parse (validate)  | Yes    | Yes    | Yes    | Yes    |
| Save-as roundtrip | Yes    | Yes    | No     | No     |
| Ops (add/edit)    | Yes    | Yes    | Yes    | Yes    |
| Render            | Yes    | Yes    | Yes    | No     |
| Invariant checks  | Full   | Full   | Parse-only | Basic |
| Proptest coverage | Yes    | Yes    | Yes    | Yes    |
| Semantic CFB diff | Yes    | Yes    | N/A    | N/A    |

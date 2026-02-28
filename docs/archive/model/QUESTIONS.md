# Design Decisions Requiring User Input

Every question below needs a decision before or during implementation. They
are ordered by dependency -- later questions often depend on earlier ones.

**Already decided (not asked here):**
- Round-trip preservation: **NO**. We do not store unknown fields or opaque blobs.
- Strict vs lenient parsing: **STRICT always**. Unknown fields/records are errors.
- Preserve unknown data: **NO**. If we can't fully model it, that's a bug to fix.

---

## Q1. Read-Only First vs Read-Write From Start

**Question:** Should we implement read-only parsing first, or design for
read-write from the start?

**Context:** Write support shapes the data model. Under the strict philosophy,
we don't preserve unknown fields (so we don't need the "bag of opaque bytes"
pattern). But write support still means we need serialization code, field
ordering conventions, and careful attention to what Altium will accept.

### Options

**A) Read-only first, add write later**
- Pros: Faster to ship a useful inspection tool. Simpler data model -- just
  parse fields into Rust structs, done. Focus engineering effort on completeness
  of the read model.
- Cons: Write path may reveal that the data model needs restructuring (e.g.,
  fields we thought were independent turn out to have write-time dependencies).
  Retrofitting is possible but wasteful.
- Risk: Low, since we're not preserving unknown blobs. The data model for read
  and write is the same Rust struct -- just adding `ToBinary`/`ToParams` impls.

**B) Read-write from the start**
- Pros: Serialization code is tested alongside deserialization from day one.
  Forces us to understand the format deeply enough to reproduce it. The derive
  macros can generate both directions at once.
- Cons: Slower time to first useful tool. Must handle write-side edge cases
  (field ordering, default suppression, zlib compression level) immediately.
  More code to write and test before anything works.

**C) Read first, but design the data model for write**
- Pros: Build the Rust structs as if write will happen (all fields present,
  correct types, no shortcuts), but only implement `FromBinary`/`FromParams`
  initially. `ToBinary`/`ToParams` follows as a second pass.
- Cons: Some write-side concerns may still be missed (e.g., field ordering
  dependencies).

### Recommendation: **Option C**

Design the structs for bidirectional use, but implement read first. The derive
macro system should generate both directions, but we can stub/skip the write
impls initially. This gives us a useful tool quickly while keeping the door
open for writes without restructuring.

---

## Q2. How To Handle Version Differences Across Altium Releases

**Question:** How should we handle the fact that Altium's file format has
evolved through V3, V4, V5, and V6, with different binary record sizes,
different sets of fields, and different sidecar streams?

**Context:**
- V3 (Protel 99): Ancient, significantly different. Common header = 14 bytes.
- V4 (DXP 2003): Adds unique_id field, common header = 19 bytes.
- V5: Intermediate.
- V6 (AD14+, ~2013-present): Current format. Most files in active use.
- Schematic has its own versioning: `MinorVersion` in FileHeader (values 2 and
  9 observed), V4 binary format vs V5 text format.
- Sidecar streams vary by version (PinFrac present in older files, absent in
  newer where the data is inline).

### Options

**A) V6 only for initial release**
- Pros: Covers all files from ~2013 onward. Simplest implementation. Avoids
  dealing with format evolution.
- Cons: Cannot read Protel 99 or early DXP files. Files saved in older formats
  by modern Altium (some users set "legacy" format) would be rejected.
- Strict compliance: Detect non-V6 files and return a clear error:
  `UnsupportedVersion { found: "V4", supported: "V6" }`.

**B) V4-V6 from the start**
- Pros: Covers DXP era (2003) through present. Most files users encounter.
- Cons: Must branch on version during deserialization for header size
  differences. More complex testing matrix.

**C) Version-aware parser with explicit version gates**
- The parser reads the format version from the file header. Each record
  type's parser takes the version as a parameter and branches as needed.
  Unsupported versions produce clear errors.
- Pros: Clean architecture for adding version support incrementally. V6 first,
  V4/V5 added by extending the version branches. Never silently misparses an
  older format.
- Cons: Version parameter threading adds boilerplate.

### Recommendation: **Option C, starting with V6 only**

Build the version-aware infrastructure from the start (file version detection,
version enum, version parameter on parsers), but only implement V6 parsing
initially. Non-V6 files get `AltiumFormatError::UnsupportedVersion`. Adding
V4/V5 is then incremental -- add version branches to individual record
parsers.

---

## Q3. Document Storage Model: Flat Indices vs Arena vs Nested Tree

**Question:** How should records be stored in the in-memory document model?

**Context:**
- **Schematic**: Flat stream of records. Parent-child via `OWNERINDEX` (index
  into the flat list). Component at index 0 owns pins at OWNERINDEX=0.
- **PCB**: Each primitive has a `component` index (-1 = board-level). Separate
  sections per primitive type. Cross-references by index into section arrays.

### Options

**A) Flat `Vec<Record>` with parent indices (mirror the file format)**

```rust
pub struct SchDoc {
    records: Vec<SchRecord>,  // flat, positional
}
// Parent lookup: record.owner_index() -> index into records vec
// Child lookup: scan records for matching owner_index
```

- Pros: Mirrors the file format exactly. Trivial serialization (indices are
  preserved). Simplest to implement. No translation between file indices and
  internal handles.
- Cons: O(n) child lookup without an auxiliary index. Insertion/deletion
  requires reindexing all owner_index values. No structural guarantee of
  consistency.

**B) Arena with stable handles**

```rust
pub struct SchDoc {
    records: SlotMap<RecordKey, SchRecord>,
    children: SecondaryMap<RecordKey, Vec<RecordKey>>,
    parent: SecondaryMap<RecordKey, RecordKey>,
}
```

- Pros: Stable handles survive insertion/deletion. O(1) parent and child
  lookup. Insertion doesn't invalidate existing handles.
- Cons: File format indices must be translated to/from handles on load/save.
  Adds `slotmap` dependency. More complex serialization.

**C) Nested tree (component owns its children directly)**

```rust
pub struct SchComponent {
    pub pins: Vec<SchPin>,
    pub parameters: Vec<SchParameter>,
    pub designator: Option<SchDesignator>,
    pub implementations: Vec<SchImplementation>,
}
```

- Pros: Type-safe hierarchy. Natural Rust ownership. Closest to the .NET data
  model. No index management.
- Cons: The file format's flat index scheme requires non-trivial reconstruction
  on load and flattening on save. Generic "iterate all records" requires
  recursive traversal. Moving records between parents is awkward.

### Recommendation

Asking because this is a fundamental architectural choice that affects
everything. Leaning toward **Option A** for PCB (where ownership is simple and
flat) and possibly **Option C** for schematic (where the tree structure is
well-defined and type-safe). But interested in your preference.

---

## Q4. Send + Sync on Document Types

**Question:** Should `SchDoc`, `PcbDoc`, `SchLib`, `PcbLib` be `Send + Sync`?

**Context:** The CLI is single-threaded. Library consumers might want to parse
files in parallel or share documents across threads.

### Options

**A) Send + Sync (use only thread-safe internals)**
- Constrains internals to `Vec`, `String`, `IndexMap`, `i32`, `f64`, etc.
  (all `Send + Sync`). No `Rc`, `Cell`, `RefCell`.
- Pros: Maximum flexibility for consumers. Files can be parsed in parallel
  threads. Standard Rust library practice.
- Cons: Prevents use of `RefCell` for lazy field computation or cached
  derived values. We'd need `OnceLock` instead.

**B) Send but not Sync**
- Allows `Cell`/`RefCell` for interior mutability. Files can be moved between
  threads but not shared.

**C) Don't constrain, let it fall out naturally**
- Don't explicitly enforce. If internal types are all Send+Sync, it works. If
  we add a non-Send type later, consumers deal with it.

### Recommendation: **Option A**

The data model is fundamentally owned data. There's no reason to use `Rc` or
`RefCell`. Add `static_assertions::assert_impl_all!(SchDoc: Send, Sync)` to
prevent accidental regression.

---

## Q5. Gating Unimplemented File Types

**Question:** If we support SchDoc but not PcbDoc yet, how should opening a
PcbDoc behave?

**Context:** We'll implement file types incrementally. The user may attempt to
open any Altium file. Partially implemented file types are dangerous -- we
might parse half the data and silently miss the rest.

### Options

**A) Error with `UnsupportedFileType { extension: ".PcbDoc" }`**
- Pros: Clear, honest. User knows exactly what's supported. No risk of
  partial/incorrect data.
- Cons: Frustrating if the user just wants basic info.

**B) Feature flags per file type: `feature = "pcb"`, `feature = "schematic"`**
- Pros: Compile-time gating. Unused code is not compiled.
- Cons: Most users will want both. Adds conditional compilation complexity.
  Shared types (Coord, Color, CFB) mean savings are minimal.

**C) Parse what we can, error on unimplemented sections/records**
- Pros: Files partially open. User gets something.
- Cons: Violates the strict philosophy. A "partially parsed" PcbDoc that's
  missing half its sections is worse than an error.

### Recommendation: **Option A**

Under the strict philosophy, partial parsing is not acceptable. Each file type
is either fully supported or returns `UnsupportedFileType`. We implement file
types in order of priority (SchLib -> SchDoc -> PcbLib -> PcbDoc) and gate
each behind a clear capability check. No feature flags -- just return errors
for unimplemented types.

---

## Q6. Error Type Granularity

**Question:** Should there be one unified `AltiumFormatError` enum, or
per-domain error types composed together?

**Context:** Error sources include: I/O, CFB container, parameter parsing,
binary parsing, coordinate overflow, unknown records, unknown fields, encoding,
compression, validation, version mismatch.

### Options

**A) Single `AltiumFormatError` enum with structured variants**

```rust
#[derive(Debug, Error)]
pub enum AltiumFormatError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("CFB error: {0}")]
    Cfb(#[from] cfb::Error),
    #[error("unknown record type {record_id} in {stream}")]
    UnknownRecord { stream: String, record_id: i32 },
    #[error("unknown field '{field}' in record {record_type} in {stream}")]
    UnknownField { stream: String, record_type: String, field: String },
    #[error("binary parse error at offset {offset} in {stream}: {detail}")]
    BinaryParse { stream: String, offset: u64, detail: String },
    #[error("parameter parse error in {stream}: {detail}")]
    ParamParse { stream: String, detail: String },
    #[error("unsupported file version: found {found}, expected {expected}")]
    UnsupportedVersion { found: String, expected: String },
    #[error("unsupported file type: {extension}")]
    UnsupportedFileType { extension: String },
    // ... etc
}
```

- Pros: One type to match on. One `Result<T>` alias. Simple.
- Cons: Enum grows large as coverage expands (15-20 variants).

**B) Per-domain error enums composed via `#[from]`**

```rust
pub enum AltiumFormatError {
    Container(ContainerError),
    Schematic(SchematicError),
    Pcb(PcbError),
}
```

- Pros: Each domain is focused. Easier to maintain separately.
- Cons: Deep nesting: `AltiumFormatError::Pcb(PcbError::BinaryParse { ... })`.
  More types to create and maintain.

### Recommendation: **Option A**

A single enum with rich variants. 15-20 variants is manageable. The library
has one natural error boundary (the file), not multiple independent error
domains. Each variant carries enough context (stream name, offset, field name)
to be actionable.

---

## Q7. binrw vs Manual Binary Parsing

**Question:** Should we use `binrw` for PCB binary record parsing, or stick
with our own derive macros + `byteorder`?

**Context:** PCB records are packed little-endian binary with:
- Fixed-size fields (i8, u8, i16le, u32le, f64le)
- Length-prefixed strings (Pascal-style u8 and i32 length)
- Multi-subrecord framing (PcbPad has 6 subrecords)
- Per-layer arrays ([CoordPoint; 32] for pad stack)
- Strict: every byte must be accounted for, no unknown trailing bytes

### Options

**A) `binrw` derive macros**
- Pros: Mature, well-tested. Rich attribute model for binary parsing.
  Generates both read and write. Built-in support for endianness, conditionals,
  count-prefixed arrays.
- Cons: Cannot express all our requirements (subrecord framing with type+length
  header, sidecar merging, strict "no remaining bytes" enforcement). Would need
  custom readers for complex records anyway. Adds ~8 seconds proc-macro compile
  time. Two derive systems (binrw + our AltiumRecord) is confusing.

**B) Our own derive macros + `byteorder` (current approach)**
- Pros: Full control over code generation. Can enforce strict "all bytes
  consumed" invariant. Handles Altium-specific patterns (subrecord framing,
  DXP fractional coords, sidecar merging) natively. One derive system.
- Cons: More code to write and maintain in the macro crate. Must handle edge
  cases ourselves.

**C) Hybrid: `binrw` for simple records, manual for complex ones**
- Pros: Less macro code to maintain for simple cases.
- Cons: Two binary parsing systems. Inconsistent code style. More dependencies.

### Recommendation: **Option B**

Our domain-specific requirements (subrecord framing, sidecar merge, strict
byte accounting) don't map to `binrw`'s model. The derive macros we need to
write are not significantly more complex than configuring `binrw` attributes
for the same behavior, and we get full control over strict enforcement.

---

## Q8. IntLib Support Strategy

**Question:** How should we handle IntLib (Integrated Library) files?

**Context:** IntLib files are CFB containers that bundle a SchLib and PcbLib
into a single file. They're a common distribution format for component
libraries. The IntLib contains embedded sub-files (compressed) that must be
extracted and parsed by the appropriate reader.

### Options

**A) No IntLib support initially**
- Pros: Reduces scope. SchLib and PcbLib are the primitives.
- Cons: IntLib is a common format. Users may need to inspect integrated
  libraries.

**B) IntLib extraction: decompose into SchLib + PcbLib, then parse those**
- Pros: Leverages existing parsers. IntLib is just a container.
- Cons: Need to understand the IntLib container layout (which streams contain
  the embedded files, how compression works).

**C) Full IntLib support: transparent access to components as if SchLib/PcbLib**
- Pros: Seamless user experience.
- Cons: Requires a unified component query abstraction across document types.
  Significantly more architecture.

### Recommendation: **Option B**

IntLib is a container format. Extract the embedded SchLib and PcbLib sub-files
into byte buffers, then pass them to the existing parsers. No unified query
layer needed. Under the strict philosophy, if we can't fully parse the IntLib
container structure, we error. But the embedded sub-files are just standard
SchLib/PcbLib.

---

## Q9. Testing Strategy With Real Altium Files

**Question:** How should we handle test fixtures? Real Altium files are binary,
potentially large, and may contain proprietary designs.

**Context:** The `data/` directory contains real Altium files for testing. These
are essential for validating the parser against real-world data. But they're
binary blobs that can't be meaningfully code-reviewed, and they may contain
designs the user doesn't want in a public repo.

### Options

**A) Commit real files to the repo in `data/`**
- Pros: Tests are self-contained. Anyone can clone and run tests. CI works.
- Cons: Binary blobs bloat the repo. Proprietary designs in public repos.
  Hard to review what the test files actually contain.

**B) Real files in `data/` (gitignored) + synthetic test files committed**
- Create minimal synthetic test files programmatically (using our write path
  or a script). These exercise specific features: a SchLib with one component
  and two pins, a PcbDoc with one pad and one track, etc.
- Pros: Synthetic files are small, reviewable, and non-proprietary. Real files
  provide comprehensive regression testing locally.
- Cons: Synthetic files may not exercise real-world edge cases. Two test
  datasets to maintain.

**C) Real files in a separate repo / LFS, synthetic files committed**
- Pros: Main repo stays clean. Real files available via LFS or submodule.
- Cons: Extra setup for contributors. CI needs LFS access.

### Recommendation: **Option B**

Commit small synthetic test files that exercise specific code paths. Use real
files from `data/` for local development and regression testing (gitignored or
in a test-data directory that's optional). Snapshot tests (`insta`) against
synthetic files for CI. Integration tests against real files as an optional
test suite.

---

## Q10. Layer Abstraction Design

**Question:** How should the PCB layer system be represented?

**Context:**
- V6 layer IDs: byte values 0-82 (TV6_Layer enum). Well-defined, fixed set.
- V7 layer IDs: 32-bit structured values with genus/family/species. Supports
  unlimited user-defined layers (extended mechanical, etc.).
- The file format stores V6 layer bytes in binary records.
- V7 is used in newer files for extended layer features.
- When Genus=0 and Family=0, the V7 species byte matches V6 IDs (backward
  compatible).

### Options

**A) Separate types: `V6Layer(u8)` + `V7Layer(u32)`**
- Pros: Type-safe. Cannot accidentally mix V6 and V7 where only one is valid.
  Matches the two distinct systems in the file format.
- Cons: Two types everywhere. Conversion boilerplate. Functions that work on
  "any layer" need generics or an enum.

**B) Unified `PcbLayer` enum with V6 and V7 variants**

```rust
pub enum PcbLayer {
    V6(V6Layer),
    V7(V7Layer),
}
```

- Pros: Single type. Pattern matching handles both. Clean API.
- Cons: Every layer operation must handle both variants. V7 layers that have
  no V6 equivalent need special handling.

**C) V6 as primary, with V7 as an extended representation**

```rust
pub struct PcbLayer(u8);  // V6 layer byte
impl PcbLayer {
    pub fn to_v7(self) -> V7Layer { ... }
    pub const TOP: PcbLayer = PcbLayer(1);
    pub const BOTTOM: PcbLayer = PcbLayer(32);
    // ...named constants for all 82 V6 layers
}
```

- Pros: Simple primary type. V7 is a derived computation, not a separate
  storage format. Most code operates on V6 because that's what the binary
  format stores.
- Cons: V7-only layers (extended mechanical > 16) cannot be represented as a
  V6 byte. Need a separate type or Option return.

### Recommendation

Leaning **Option C** (V6 primary) because the file format stores V6 bytes and
that's what we parse. V7 is a runtime interpretation layer. But if you expect
to encounter V7-only layers in real files, **Option B** may be more correct.

---

## Q11. How Strict on the Write Path

**Question:** When writing files, should we reject writes that would produce
data Altium Designer can't read?

**Context:** Under the strict philosophy, we error on anything we don't
understand when reading. But what about writing? Should we prevent the user
from creating a component with an empty designator, or a pad with zero size,
or a track with start == end?

### Options

**A) Validate on write: reject invalid data**
- The `save()` method runs validation before writing. Invalid data returns an
  error.
- Pros: Guarantees output files are valid Altium files. Prevents subtle bugs
  where a user creates a malformed component that Altium silently ignores or
  misinterprets.
- Cons: Must define what "valid" means for every field. Validation rules may
  be incomplete or overly strict. May prevent legitimate use cases.

**B) Write what the user gives us, no validation**
- Pros: Simple. The library is a format codec, not a design rule checker.
  Users know their intent.
- Cons: Can produce files that crash or confuse Altium Designer. Silent data
  corruption via malformed output.

**C) Validate with warnings, write anyway**
- Pros: Users are informed about potential issues but not blocked.
- Cons: Warnings may be ignored. Doesn't prevent the problem.

**D) Validate on write, with a `force` option to bypass**
- Default: validate and reject invalid data.
- `save_unchecked()`: write without validation for advanced users.
- Pros: Safe default, escape hatch for power users.
- Cons: Two code paths to maintain.

### Recommendation

Leaning **Option D** but want your input on how strict. Minimum viable
validation: field values are within documented ranges (layer IDs 0-82, shape
enums 0-10, etc.). Advanced validation (zero-size pads, empty designators)
can be added later.

---

## Q12. Schematic Record Coverage Scope

**Question:** Which schematic record types should be fully modeled in the
initial release?

**Context:** There are 115+ TObjectId values in the .NET source. The binary
format has ~50 distinct RECORD codes. In our test data, 31 distinct RECORD
IDs appear. The question is: which of these 31 (or more) should have full
struct definitions vs being deferred?

### Options

**A) Core set only (most common ~15 types)**
Records: Component(1), Pin(2), Label(4), Polyline(6), Polygon(7),
Rectangle(14), PowerObject(17), Port(18), NetLabel(25), Wire(27),
Junction(29), Sheet(31), Designator(34), Parameter(41), Implementation
chain (44-48).

- Pros: Covers the vast majority of real-world files. Fast to implement.
- Cons: Files with Arc(12), Ellipse(8), BusEntry(37), Image(30), etc. will
  fail to parse.

**B) All 31 observed record types**
Model every RECORD ID found in test data. If it appears in a real file, it
gets a struct.

- Pros: Handles all known real-world files. Under strict philosophy, we MUST
  model everything we encounter.
- Cons: Some types are rare (WarningSign=43, CompileMask=225). More types =
  more time.

**C) All 31 observed + all harness types (104-138)**
The .NET source shows harness wiring diagram types that may appear in newer
files.

- Pros: Future-proofed for harness designs.
- Cons: Harness types are rare. Significantly more work for uncertain benefit.

### Recommendation: **Option B**

Under the strict philosophy, if we encounter a record ID we don't model,
that's an error. We should model everything that appears in our test data.
Harness types (Option C) can be deferred until we encounter them in real files
(at which point they become errors that drive implementation).

---

## Q13. PCB Record Coverage Scope

**Question:** Same as Q12, but for PCB. Which TObjectId values should be
fully modeled?

**Context:** 27 TObjectId values exist. Our test data shows 8 distinct object
IDs: Arc(1), Pad(2), Via(3), Track(4), Text(5), Fill(6), Region(11),
ComponentBody(12). Additional types in docs but not in test data: Polygon(10),
Dimension(13), Coordinate(14).

### Options

**A) Only the 8 observed types**
- Pros: Covers all known test files. Fast.
- Cons: Files with Polygon pours, Dimensions, or Coordinates will fail.
  Polygon pours are common in real PCBs.

**B) Observed 8 + Component(9) + Polygon(10) + Dimension(13)**
- Pros: Covers all types likely to appear in real PCB files. Components are
  in the Components6/ section (text format), not binary, but they need
  modeling. Polygon pours are ubiquitous in real designs.
- Cons: More work.

**C) All 27 types**
- Pros: Complete coverage. No surprises.
- Cons: Many types are transient/runtime-only (Connection=7, Violation=19,
  Trace=23, SpareVia=24) and never appear in files. Implementing them is
  wasted effort.

### Recommendation: **Option B**

Model the 8 observed binary types + Component (text section) + Polygon + Dimension.
Net(8), Class(15), Rule(16) are metadata sections (text format, not binary
primitives) that also need modeling. Runtime-only types (Connection, Violation,
Trace, SpareVia) should NOT be modeled -- they never appear in files. If we
encounter one, error with `UnknownRecord`.

---

## Q14. Sidecar Stream Strategy

**Question:** When and how should sidecar streams be merged with main records?

**Context:** Both schematic and PCB formats have sidecar streams that supplement
main records with additional fields (PinFrac, WideStrings, UniqueIDs,
ExtendedPrimitiveInfo, etc.). Under the strict philosophy:
- If a sidecar stream exists, we must fully parse it.
- If a sidecar stream provides data for a field, that data must be present in
  our model after loading.
- If we don't understand a sidecar stream's format, that's an error.

### Options

**A) Merge eagerly during load**
- After parsing main records, iterate all sidecar streams and merge their
  fields into the corresponding records. Records are complete after `open()`.
- Pros: Simple model. Records have all their data. No deferred state.
- Cons: If any sidecar is malformed, the entire file fails to open.

**B) Parse sidecars into parallel structures, merge lazily**
- Main records are available immediately. Sidecar data is stored separately
  and merged when a field is accessed.
- Pros: Main records available even if sidecars are broken.
- Cons: Violates strict philosophy (broken sidecar = incomplete model = error).
  Complex accessor logic.

**C) Merge eagerly, error on unknown sidecar streams**
- Same as A, but also error if the file contains sidecar streams we don't
  know about (since we can't verify their data is captured in our model).
- Pros: Maximum strictness. We guarantee our model captures ALL data in the
  file.
- Cons: New sidecar streams added in future Altium versions would cause errors.

### Recommendation: **Option A**

Merge eagerly. If a sidecar stream is malformed, return an error. Missing
sidecar streams (legitimate for older file versions) are fine -- fields get
defaults.

Option C is tempting for maximum strictness, but erroring on unknown sidecar
stream NAMES (not content) is too brittle. A new Altium version might add a
`PrimitiveGuids` stream we don't use yet. We should only error if we encounter
data we can't interpret within a stream we're parsing.

---

## Q15. PrjPcb (Project File) Support

**Question:** Should we support PrjPcb project files?

**Context:** PrjPcb files are NOT OLE/CFB -- they're plain-text Windows INI
files. They list document paths, build configurations, and variant definitions.
They don't contain design data themselves.

### Options

**A) Don't support initially**
- Pros: Focus on actual design files (SchDoc, SchLib, PcbDoc, PcbLib).
- Cons: Users may want to list project documents or extract paths.

**B) Read-only INI parser for project metadata**
- Parse sections like `[Document1]`, `[Design]`, `[Configuration1]`.
- Expose document paths, project settings, variant definitions.
- Pros: Useful for project-level tooling (list all documents, check paths).
- Cons: INI parsing is simple but separate from the OLE/CFB pipeline.

**C) Full project file support including write**
- Pros: Can modify project settings, add/remove documents.
- Cons: Overkill for initial release.

### Recommendation: **Option B**

PrjPcb is simple to parse (split on `[Section]` headers, split on `=` for
keys) and provides useful project-level context. Read-only is sufficient
initially. No external INI-parsing crate needed -- hand-written parser for
this specific format is cleaner than pulling in a generic INI library.

---

## Q16. Coordinate Display and Formatting

**Question:** Should `Coord` display as internal units, mils, or mm? Should
the display format be configurable?

**Context:** Internal units (10000 per mil) are meaningless to users. Mils and
mm are both used in practice (IPC standards use mm, many US designers use mils).
The CLI needs to display coordinates in a human-readable format.

### Options

**A) Coord always displays as internal units**
- `Display` impl shows raw i32 value. Conversion is caller's responsibility.
- Pros: No ambiguity. Simple.
- Cons: Useless for human consumption. "Location: 10000000" means nothing.

**B) Coord displays as mils by default**
- `Display` shows `"1000.0mil"`. Methods for mm output.
- Pros: Mils matches Altium's traditional unit system.
- Cons: Hardcoded unit preference.

**C) Coord has no Display impl; formatting is in the CLI/ops layer**
- Coord provides `to_mils()`, `to_mm()`, `to_internal()`. The CLI decides
  how to format.
- Pros: Clean separation. The library doesn't impose display preferences.
- Cons: Slightly more work for consumers who want quick output.

### Recommendation: **Option C**

`Coord` provides conversion methods. Display formatting is the CLI's job, not
the library's. The ops layer could provide a `CoordFormatter` utility that
respects a user-configured unit preference.

---

## Q17. What Happens When We Find Data We Can't Parse

**Question:** Under the strict philosophy, encountering data we don't model is
an error. But what error? And how granular?

**Context:** "Unknown data" can mean several things:
1. Unknown RECORD ID (e.g., RECORD=99 in a SchDoc)
2. Unknown parameter key in a known record (e.g., `NEWFIELD=xyz` in a SchPin)
3. Unknown bits set in a flags field
4. Extra bytes at the end of a binary record
5. Unknown sidecar stream content

Each of these has different implications and might warrant different handling.

### Options

**A) All unknown data is the same error: `UnknownData`**
- Pros: Simple.
- Cons: Can't distinguish "unknown record type" (maybe a new Altium feature)
  from "unknown field in a known record" (maybe our model is incomplete).

**B) Distinct error variants for each category**

```rust
UnknownRecord { record_id, stream }
UnknownField { record_type, field_name, stream }
UnknownFlags { record_type, field_name, raw_value, unknown_bits }
ExtraBytes { record_type, expected_len, actual_len, stream }
```

- Pros: Actionable. "Unknown field 'NEWPARAM' in SchPin" tells the developer
  exactly what to implement next. Each variant guides a different fix.
- Cons: More error variants. More matching needed.

**C) Single error variant with a category enum**

```rust
UnknownData { category: UnknownCategory, context: String }
enum UnknownCategory { Record, Field, Flags, ExtraBytes, SidecarContent }
```

- Pros: Fewer top-level variants. Still categorized.
- Cons: Less type safety. Context is a string.

### Recommendation: **Option B**

Each category of "unknown data" drives a different action:
- Unknown record: implement the record type
- Unknown field: add the field to the record struct
- Unknown flags: add the flag to the bitflags definition
- Extra bytes: extend the binary layout
- Unknown sidecar content: implement the sidecar parser

Distinct error variants make this self-documenting.

---

## Q18. Parameter Key Case Sensitivity

**Question:** Altium parameter keys appear to be case-insensitive (keys like
`RECORD`, `Record`, `record` all mean the same thing in Altium Designer).
Should our parser be case-insensitive?

### Options

**A) Case-insensitive lookup, case-preserving storage**
- Store keys as-is. Lookup converts to uppercase. Write back original case.
- Pros: Matches Altium's behavior. Handles files with inconsistent casing.
- Cons: Slightly more complex lookup (must normalize case).

**B) Normalize all keys to uppercase on parse**
- Convert all keys to uppercase during parsing. Store and write as uppercase.
- Pros: Simple. Deterministic output. No case-insensitive lookup needed.
- Cons: Changes key casing in output files. Under our philosophy this is fine
  (we don't preserve exact original encoding).

**C) Case-sensitive (keys must match exactly)**
- Pros: Simplest implementation.
- Cons: May fail on files with unexpected casing.

### Recommendation: **Option B**

We don't do round-trip preservation of encoding details. Normalize keys to
uppercase during parsing. This matches Altium's canonical output format and
simplifies all key lookups to exact string matching.

# PrjPcb High-Level API Design

How the `altium-format` high-level API should be extended for PrjPcb projects,
following the patterns established by the SchLib API.

## Existing Pattern: How SchLib Does It

The SchLib high-level API is the reference implementation. Understanding its layers
is essential before designing the PrjPcb equivalent.

### Layer Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│  autopcb-spec (executor / reconciler)                         │
│                                                                     │
│  apply_spec_schlib(spec, doc)                                       │
│    → doc.component("R")        // query via high-level API          │
│    → merge_spec_into_component // build merged api::Component       │
│    → doc.update_component(c)   // mutate via high-level API         │
│                                                                     │
│  reconcile_schlib(spec, doc)                                        │
│    → doc.components()          // read all via high-level API       │
│    → diff_component(spec, existing)  // compare field by field      │
│    → EngineeringChangeOrder    // output ECO                        │
├─────────────────────────────────────────────────────────────────────┤
│  altium-format::api  (public high-level types)                      │
│                                                                     │
│  Component, Pin, Parameter, FootprintMap, Graphic (13 variants)     │
│  — Clean Rust structs, no format internals exposed                  │
│  — Option<T> fields in SpecModel → None means "don't override"      │
├─────────────────────────────────────────────────────────────────────┤
│  altium-format::api::schlib_read  (internal → public)               │
│                                                                     │
│  component_from_internal(SchLibComponent, SchLibComponentIndex)      │
│    → api::Component                                                 │
│  Dispatches on SchRecord variants, merges sidecar data              │
├─────────────────────────────────────────────────────────────────────┤
│  altium-format::api::schlib_write  (public → internal)              │
│                                                                     │
│  component_to_internal(api::Component)                              │
│    → (SchComponent, Vec<SchRecord>, Vec<SchRecord>, Index)          │
│  update_component_internal(api::Component, existing: &SchComponent) │
│    → preserves format-internal fields from existing                 │
├─────────────────────────────────────────────────────────────────────┤
│  altium-format::schlib  (document-level methods)                    │
│                                                                     │
│  SchLib::component_names() → Vec<String>                            │
│  SchLib::component(lib_ref) → Result<api::Component>                │
│  SchLib::components() → Result<Vec<api::Component>>                 │
│  SchLib::add_component(comp) → Result<()>                           │
│  SchLib::update_component(comp) → Result<()>                        │
│  SchLib::remove_component(lib_ref) → Result<()>                     │
│  SchLib::new_blank_ad26() → Self                                    │
├─────────────────────────────────────────────────────────────────────┤
│  altium-format::schlib  (internal storage)                          │
│                                                                     │
│  TrackedCfbDocument + SchLibComponent + SchLibComponentIndex         │
│  SchRecord enum (flat list per component)                           │
│  Sidecar streams (WideStrings, PinFrac, PinWideText, etc.)         │
└─────────────────────────────────────────────────────────────────────┘
```

### Key Design Principles (to carry forward)

1. **Public types are clean Rust structs** — no `Vec<u8>`, no raw primitives where domain
   types exist, no format-internal fields. The API hides CFB, block encoding, sidecars.

2. **Query returns owned values** — `component()` returns `api::Component` (owned), not
   a reference into internal storage. This decouples API consumers from storage layout.

3. **Mutation takes owned/ref API types** — `add_component(comp)` and
   `update_component(&comp)` accept the public API type. Internal conversion is private.

4. **Additive merge semantics** — `Option<T>` fields in the spec model mean "override if
   Some, preserve existing if None". The executor's `merge_spec_into_component` implements
   this.

5. **Validate after mutation** — Every mutating method calls `validate_invariants()` before
   returning. Broken invariants are caught immediately.

6. **Read and write paths are separate modules** — `schlib_read.rs` (internal→public) and
   `schlib_write.rs` (public→internal) are `pub(crate)`, keeping conversion details private.

---

## PrjPcb API Design

### Differences from SchLib

| Aspect | SchLib | PrjPcb |
|--------|--------|--------|
| Container | CFB compound document | Plain-text INI file |
| Internal storage | `TrackedCfbDocument` + `SchRecord` enums | Parsed `HashMap<String, String>` + section lists |
| Sidecar streams | 9+ pin sidecars, WideStrings, UniqueIDs | None — flat text |
| Entity nesting | Component → Pins, Parameters, Graphics, Footprints | Project → Documents, OutputGroups → Outputs |
| Record dispatch | `SchRecord` enum (50+ variants) | Section name + key prefix |
| Coordinates | Yes (Coord, CoordPoint) | No spatial data |
| Binary data | Yes (binary blocks, compressed objects) | None |
| Write complexity | High (block encoding, sidecar generation, CFB mutation) | Low (format INI sections as text) |

### Proposed API Types

These go in `crates/altium-format/src/api/project_types.rs`:

```rust
/// Top-level project. Natural key: project name (filename stem).
pub struct Project {
    pub name: String,

    // ── [Design] section ───────────────────────────────
    pub hierarchy_mode: FlattenMode,
    pub channel_room_naming_style: ChannelRoomNamingStyle,
    pub channel_designator_format: String,
    pub channel_room_level_separator: String,

    // Net naming
    pub allow_port_net_names: bool,
    pub allow_sheet_entry_net_names: bool,
    pub netlist_single_pin_nets: bool,
    pub append_sheet_number_to_local_nets: bool,
    pub name_nets_hierarchically: bool,
    pub power_port_names_take_priority: bool,

    // Pin swap
    pub pin_swap_by_netlabel: bool,
    pub pin_swap_by_pin: bool,

    // Cross-references
    pub cross_ref_sheet_style: CrossRefSheetStyle,
    pub cross_ref_location_style: CrossRefLocationStyle,
    pub cross_ref_ports: CrossRefPorts,
    pub cross_ref_cross_sheets: bool,
    pub cross_ref_sheet_entries: bool,
    pub cross_ref_follow_from_main_settings: bool,

    // Sheet numbering
    pub auto_sheet_numbering: bool,
    pub auto_cross_references: Option<bool>,  // None = undefined (-1)
    pub new_indexing_of_sheet_symbols: bool,

    // Build / output
    pub output_path: String,

    // ── Children ───────────────────────────────────────
    pub documents: Vec<DocumentRef>,
    pub configurations: Vec<BuildConfiguration>,
    pub output_groups: Vec<OutputGroup>,
    pub annotation: AnnotationSettings,
    pub class_gen: ClassGenSettings,
    pub library_update: LibraryUpdateSettings,
    pub comparison_options: Vec<ComparisonOption>,
    pub erc_matrix: ErcConnectionMatrix,
    pub erc_levels: Vec<ErcLevel>,
    pub modification_levels: Vec<ModificationLevel>,
    pub difference_levels: Vec<DifferenceLevel>,
    pub variants: Vec<ProjectVariant>,
    pub parameters: Vec<ProjectParameter>,
    pub diff_pair_suffixes: Vec<DiffPairSuffix>,
    pub net_infos: Vec<NetInfo>,
}

/// A document referenced by the project. Natural key: `path`.
pub struct DocumentRef {
    pub path: String,                        // Relative path from project dir
    pub unique_id: String,                   // 8-char cross-reference ID
    pub annotation_enabled: bool,
    pub annotate_start_value: i32,
    pub annotation_index_control_enabled: bool,
    pub annotate_suffix: String,
    pub annotate_scope: DocAnnotationScope,
    pub annotate_order: i32,
    pub do_library_update: bool,
    pub do_database_update: bool,
    pub class_gen_cc_auto_enabled: bool,
    pub class_gen_cc_auto_room_enabled: bool,
    pub class_gen_nc_auto_scope: DocAutoNetClassScope,
    pub generate_class_cluster: bool,
}

/// Build configuration. Natural key: `name`.
pub struct BuildConfiguration {
    pub name: String,
    pub variant: String,
    pub content_type_guid: String,
    pub configuration_type: String,
    pub parameter_count: i32,
    pub constraint_file_count: i32,
    pub output_jobs_count: i32,
}

/// Output job group. Natural key: `name`.
pub struct OutputGroup {
    pub name: String,
    pub description: String,
    pub target_printer: String,
    pub printer_options: String,              // Raw pipe-delimited (preserve as-is)
    pub outputs: Vec<OutputJob>,
}

/// Individual output within a group. Natural key: `name`.
pub struct OutputJob {
    pub name: String,
    pub output_type: String,
    pub document_path: String,
    pub variant_name: String,
    pub is_default: bool,
    pub page_options: Option<String>,         // Raw pipe-delimited (preserve as-is)
}

/// Annotation settings (singleton).
pub struct AnnotationSettings {
    pub sort_order: SortOrder,
    pub sort_location: SortLocation,
    pub replace_subparts: bool,
    pub physical_naming_format: String,
    pub global_index_sort_order: SortOrder,
    pub global_index_sort_location: SortLocation,
    pub match_parameters: Vec<AnnotationMatchParameter>,
}

pub struct AnnotationMatchParameter {
    pub name: String,
    pub strict: bool,
}

/// Class generation settings (singleton).
pub struct ClassGenSettings {
    pub comp_class_manual_enabled: bool,
    pub comp_class_manual_room_enabled: bool,
    pub net_class_auto_bus_enabled: bool,
    pub net_class_auto_comp_enabled: bool,
    pub net_class_auto_named_harness_enabled: bool,
    pub net_class_manual_enabled: bool,
    pub net_class_separate_for_bus_sections: bool,
}

/// Library update settings (singleton).
pub struct LibraryUpdateSettings {
    pub selected_only: bool,
    pub update_variants: bool,
    pub update_to_latest_revision: bool,
    pub full_replace: bool,
    pub update_designator_lock: bool,
    pub update_part_id_lock: bool,
    pub preserve_parameter_locations: bool,
    pub preserve_parameter_visibility: bool,
    pub do_graphics: bool,
    pub do_parameters: bool,
    pub do_models: bool,
    pub add_parameters: bool,
    pub remove_parameters: bool,
    pub add_models: bool,
    pub remove_models: bool,
    pub update_current_models: bool,
}

/// ECO comparison option. Natural key: `kind`.
pub struct ComparisonOption {
    pub kind: String,
    pub min_percent: i32,
    pub min_match: i32,
    pub show_match: bool,
    pub use_name: i32,            // -1=auto, 0=no, 1=yes
    pub include_all_rules: bool,
}

/// 17×17 ERC connection matrix.
pub struct ErcConnectionMatrix {
    /// Matrix[row][col] where row/col are TConnectionCode values (0..16).
    pub cells: [[ErrorLevel; 17]; 17],
}

/// Per-ErrorKind ERC check level.
pub struct ErcLevel {
    pub error_kind_index: u16,    // 1-based Type{N} key
    pub level: ErrorLevel,
}

/// Per-DifferenceKind modification level.
pub struct ModificationLevel {
    pub difference_kind_index: u16,
    pub enabled: bool,
}

/// Per-DifferenceKind difference check level.
pub struct DifferenceLevel {
    pub difference_kind_index: u16,
    pub level: DifferenceCheckLevel,
}

/// Project variant. Natural key: `description` (or unique_id).
pub struct ProjectVariant {
    pub unique_id: String,
    pub description: String,
    pub overwrite_pcb_footprint: bool,
    pub variations: Vec<ComponentVariation>,
    pub param_variations: Vec<ParameterVariation>,
}

pub struct ComponentVariation {
    pub designator: String,
    pub unique_id: String,
    pub kind: VariationKind,
    pub alternate_part: String,
}

pub struct ParameterVariation {
    pub designator: String,
    pub parameter_name: String,
    pub variant_value: String,
}

/// Project-level parameter. Natural key: `name`.
pub struct ProjectParameter {
    pub name: String,
    pub value: String,
}

/// Differential pair suffix. Natural key: index.
pub struct DiffPairSuffix {
    pub positive: String,
    pub negative: String,
}

/// Net color assignment. Natural key: `net_name`.
pub struct NetInfo {
    pub net_name: String,
    pub net_color: Color,
}
```

### Enum Types for `altium-format-types`

These new enums go in `crates/altium-format-types/src/`:

```rust
// In a new project.rs module or added to existing sch.rs

/// TFlattenMode — project hierarchy mode
pub enum FlattenMode {
    Smart = 0,
    Flat = 1,
    HierarchicalGlobalPorts = 2,
    Global = 3,
    HierarchicalStrict = 4,
}

/// TChannelRoomNamingStyle
pub enum ChannelRoomNamingStyle {
    FlatNumericWithNames = 0,
    FlatAlphaWithNames = 1,
    NumericNamePath = 2,
    AlphaNamePath = 3,
    MixedNamePath = 4,
}

/// TCrossRefSheetStyle
pub enum CrossRefSheetStyle {
    None = 0,
    Name = 1,
    Number = 2,
}

/// TCrossRefLocationStyle
pub enum CrossRefLocationStyle {
    None = 0,
    Zone = 1,
    XY = 2,
}

/// TCrossRefPorts
pub enum CrossRefPorts {
    Disabled = 0,
    SheetEntry = 1,
    Ports = 2,
    SheetEntryAndPorts = 3,
}

/// TSortOrder
pub enum SortOrder {
    UpThenAcross = 0,
    DownThenAcross = 1,
    AcrossThenUp = 2,
    AcrossThenDown = 3,
}

/// TSortLocation
pub enum SortLocation {
    Designator = 0,
    Part = 1,
}

/// TErrorLevel — used for ERC checks and connection matrix
pub enum ErrorLevel {
    NoReport = 0,
    Warning = 1,
    Error = 2,
    Fatal = 3,
}

/// TDifferenceCheckLevel — used for [Difference Levels]
pub enum DifferenceCheckLevel {
    Off = 0,
    On = 1,
    OnCaseSensitive = 2,
}

/// TVariationKind — variant component state
pub enum VariationKind {
    None = 0,
    NotFitted = 1,
    Alternate = 2,
}

/// TDocAnnotationScope
pub enum DocAnnotationScope {
    All = 0,
    IgnoreSelected = 1,
    OnlySelected = 2,
}

/// TDocAutoNetClassScope
pub enum DocAutoNetClassScope {
    None = 0,
    LocalOnly = 1,
    All = 2,
}

/// TConnectionCode — ERC matrix row/column type
pub enum ConnectionCode {
    PinInput = 0,
    PinIO = 1,
    PinOutput = 2,
    PinOpenCollector = 3,
    PinPassive = 4,
    PinHiZ = 5,
    PinOpenEmitter = 6,
    PinPower = 7,
    PortInput = 8,
    PortOutput = 9,
    PortBidirectional = 10,
    PortUnspecified = 11,
    SheetEntryInput = 12,
    SheetEntryOutput = 13,
    SheetEntryBidirectional = 14,
    SheetEntryUnspecified = 15,
    Unconnected = 16,
}
```

### Document-Level Methods

On `AltiumProject` in `crates/altium-format/src/project.rs`:

```rust
impl AltiumProject {
    // ── Factory ──────────────────────────────────────────
    pub fn new_blank() -> Self;
    pub fn open(path: impl AsRef<Path>) -> Result<Self>;
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()>;

    // ── Query (returns public API types) ─────────────────
    pub fn project(&self) -> Result<api::Project>;
    pub fn document_paths(&self) -> Vec<&str>;

    // ── Mutate (accepts public API types) ────────────────
    pub fn update_project(&mut self, project: &api::Project) -> Result<()>;
    pub fn add_document(&mut self, doc: api::DocumentRef) -> Result<()>;
    pub fn remove_document(&mut self, path: &str) -> Result<()>;
    pub fn add_output_group(&mut self, group: api::OutputGroup) -> Result<()>;
    pub fn add_variant(&mut self, variant: api::ProjectVariant) -> Result<()>;
}
```

### Read/Write Paths

```
crates/altium-format/src/api/
├── mod.rs                  (add project type re-exports)
├── project_types.rs        (public API types — Project, DocumentRef, etc.)
├── project_read.rs         (INI sections → api::Project)
└── project_write.rs        (api::Project → INI text)
```

**`project_read.rs`** — Parse the flat `ValueMap` + section lists into `api::Project`:
- Map `ValueMap` keys to `Project` struct fields using `PrjPcbConsts` key names
- Parse `[Document{N}]` entries → `Vec<DocumentRef>`
- Parse `[OutputGroup{N}]` → `Vec<OutputGroup>` with nested `Vec<OutputJob>`
- Parse `[ERC Connection Matrix]` → `ErcConnectionMatrix`
- Parse `[Electrical Rules Check]` → `Vec<ErcLevel>`
- etc.

**`project_write.rs`** — Format `api::Project` back into ordered INI text:
- Write `[Design]` keys in canonical order (matching Altium's output)
- Write `[Preferences]`
- Write `[Document{N}]` sections (renumber sequentially)
- Write `[Configuration{N}]` sections
- Write `[OutputGroup{N}]` sections with indexed outputs
- Write `[Modification Levels]`, `[Difference Levels]`, `[Electrical Rules Check]`
- Write `[ERC Connection Matrix]`
- Write `[Annotate]`, `[PrjClassGen]`, etc.

---

## Spec Model Types

In `crates/autopcb-spec/src/model.rs`:

```rust
// Add to SpecModel enum:
Proj(PrjPcbSpec),

// Add to SpecDomain enum:
Proj,

pub struct PrjPcbSpec {
    pub projects: Vec<ProjectSpec>,
}

pub struct ProjectSpec {
    pub name: String,

    // All fields Optional — None means "don't override"
    pub hierarchy_mode: Option<FlattenMode>,
    pub channel_room_naming_style: Option<ChannelRoomNamingStyle>,
    pub channel_designator_format: Option<String>,
    pub channel_room_level_separator: Option<String>,

    pub allow_port_net_names: Option<bool>,
    pub allow_sheet_entry_net_names: Option<bool>,
    pub netlist_single_pin_nets: Option<bool>,
    pub append_sheet_number_to_local_nets: Option<bool>,
    pub name_nets_hierarchically: Option<bool>,
    pub power_port_names_take_priority: Option<bool>,

    pub cross_ref_sheet_style: Option<CrossRefSheetStyle>,
    pub cross_ref_location_style: Option<CrossRefLocationStyle>,
    pub cross_ref_ports: Option<CrossRefPorts>,
    pub cross_ref_cross_sheets: Option<bool>,
    pub cross_ref_sheet_entries: Option<bool>,

    pub annotation: Option<AnnotationSpec>,
    pub documents: Vec<DocumentSpec>,
    pub erc_matrix_overrides: Vec<ErcMatrixOverride>,
    pub erc_level_overrides: Vec<ErcLevelOverride>,
    pub output_groups: Vec<OutputGroupSpec>,
    pub comparison_rules: Vec<ComparisonRuleSpec>,
    pub class_gen: Option<ClassGenSpec>,
    pub library_update: Option<LibraryUpdateSpec>,
    pub variants: Vec<VariantSpec>,
}
```

### ECO Entity Kinds

Add to `EntityKind` in `eco.rs`:

```rust
pub enum EntityKind {
    // Existing:
    Component, Pin, Parameter, Alias, Graphic, Footprint, Pad,
    Track, Via, Arc, Text, Fill, Region,

    // New for PrjPcb:
    Project,
    DocumentRef,
    OutputGroup,
    OutputJob,
    BuildConfiguration,
    Variant,
    Variation,
    ComparisonRule,
}
```

---

## Executor Flow

`apply_spec_prjpcb(spec: &PrjPcbSpec, doc: &mut AltiumProject)`:

```
for each ProjectSpec in spec.projects:
    existing = doc.project()

    // Merge scalar properties (Option fields: Some overrides, None preserves)
    merged = merge_spec_into_project(&existing, &spec)

    // Merge documents (match by path)
    for doc_spec in spec.documents:
        if existing has doc with same path:
            merge doc_spec fields into existing doc
        else:
            add new DocumentRef from doc_spec

    // Merge output groups (match by name)
    for group_spec in spec.output_groups:
        if existing has group with same name:
            merge outputs (match by name within group)
        else:
            add new OutputGroup from group_spec

    // Apply ERC matrix overrides (sparse)
    for override in spec.erc_matrix_overrides:
        existing.erc_matrix.cells[row][col] = override.level

    // Apply ERC level overrides (sparse)
    for override in spec.erc_level_overrides:
        existing.erc_levels[index].level = override.level

    doc.update_project(&merged)
```

## Reconciler Flow

`reconcile_prjpcb(spec: &PrjPcbSpec, doc: &AltiumProject)`:

```
for each ProjectSpec in spec.projects:
    existing = doc.project()

    // Diff scalar properties
    for each Option field in spec:
        if Some(val) and val != existing.field:
            emit PropChange

    // Diff documents (match by path)
    for doc_spec in spec.documents:
        if existing has matching path:
            diff fields → Update or Unchanged
        else:
            emit Add

    // Diff output groups (match by name)
    // Diff ERC matrix (only overridden cells)
    // Diff ERC levels (only overridden entries)
    // etc.

    return EngineeringChangeOrder
```

---

## Implementation Priority

| Priority | Task | Rationale |
|---|---|---|
| **P0** | INI parser in `altium-format` | Foundation — everything depends on reading the file |
| **P0** | Domain enums in `altium-format-types` | Required by API types |
| **P1** | API types (`project_types.rs`) | Public types for the high-level API |
| **P1** | Read path (`project_read.rs`) | INI sections → `api::Project` |
| **P1** | `AltiumProject::open()` + `project()` | Query existing projects |
| **P2** | Write path (`project_write.rs`) | `api::Project` → INI text |
| **P2** | `AltiumProject::save()` | Write back to disk |
| **P2** | `altium validate` for `.PrjPcb` | Red/green development loop |
| **P2** | `altium dump` for `.PrjPcb` | Reverse-generate spec from existing |
| **P3** | Spec language extension (parser + compiler) | `.proj` support |
| **P3** | Reconciler + ECO | `altium plan` for project specs |
| **P3** | Executor | `altium apply` for project specs |

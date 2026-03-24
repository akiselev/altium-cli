# PrjPcb Spec Language Design

How to extend the Altium Spec Language to support `.proj` files.

## Context: What the Spec Language Already Does

The spec language is a declarative DSL. A spec file says "the document should look like
this" and the system diffs it against the existing document to produce an Engineering
Change Order (ECO). The pipeline is:

```
source text → lexer → parser → AST → import resolver → compiler → SpecModel
                                                                      ↓
                                                           reconciler (diff vs document)
                                                                      ↓
                                                                     ECO
                                                                      ↓
                                                              executor (apply)
```

Currently supported domains:

| Extension | Domain | Top-level keyword | SpecModel variant |
|---|---|---|---|
| `.sym` | SchLib | `component` | `SpecModel::Sym(SymSpec)` |
| `.sym` | PcbLib | `footprint` | `SpecModel::Sym(SymSpec)` |

PrjPcb would add:

| Extension | Domain | Top-level keyword | SpecModel variant |
|---|---|---|---|
| `.proj` | PrjPcb | `project` | `SpecModel::Proj(PrjPcbSpec)` |

---

## Why a PrjPcb Spec?

A declarative spec for project files enables:

1. **Reproducible project creation** — `altium apply my-board.proj` creates a
   fully-configured project with documents, ERC settings, output jobs, etc.
2. **Project auditing** — `altium plan my-board.proj` diffs the spec against an
   existing project and reports drift (changed ERC levels, missing documents, etc.)
3. **Template enforcement** — company-wide spec templates ensure all projects use the
   same ERC matrix, annotation order, and output job configuration.
4. **Composability with SchLib/PcbLib specs** — a project spec could reference component
   and footprint specs, enabling full declarative project management from specs alone.

---

## PrjPcb vs SchLib/PcbLib: Key Differences

| Aspect | SchLib / PcbLib | PrjPcb |
|--------|-----------------|--------|
| Container | CFB compound document | Plain-text INI |
| Data model | Deeply nested (components → pins → graphics) | Mostly flat sections with key-value pairs |
| Identity keys | `lib_reference` / footprint name | Document path, output group name |
| Coordinates | Dimensional (mil, mm) | None — no spatial data |
| Expression needs | Heavy (dimensions, anchors, spreads) | Light (mostly strings, ints, bools, enums) |
| Bulk data | Few entities with many fields | Many settings sections with repetitive structure |

The key insight: PrjPcb specs need **less expression power** but more **section-level
structure**. The spec language already has the right primitives (let bindings, objects,
enums, strings, ints, bools), but needs new top-level entity types and a new compiler
backend.

---

## Proposed Syntax

### Minimal Example

```
project "MyBoard" {
    hierarchy_mode: flat

    document "MyBoard.SchDoc" {
        annotation_enabled: true
        annotate_start_value: 1
    }

    document "MyBoard.PcbDoc" {
        annotation_enabled: true
    }
}
```

### Full Example

```
// Shared ERC templates
let strict_erc = {
    output_output: error,
    passive_passive: no_report,
    power_power: warning
}

let standard_outputs = {
    gerber: true,
    nc_drill: true,
    pick_place: true,
    bom: true
}

project "HydroController" {
    // ── Core settings ──────────────────────────────────
    hierarchy_mode: smart
    channel_room_naming_style: flat_numeric_with_names
    channel_designator_format: "$Component_$RoomName"
    channel_room_level_separator: "_"

    // ── Net naming ─────────────────────────────────────
    allow_port_net_names: false
    allow_sheet_entry_net_names: true
    netlist_single_pin_nets: false
    append_sheet_number_to_local_nets: false
    name_nets_hierarchically: false
    power_port_names_take_priority: false

    // ── Cross-references ───────────────────────────────
    cross_ref_sheet_style: number
    cross_ref_location_style: zone
    cross_ref_ports: sheet_entry_and_ports
    cross_ref_cross_sheets: true
    cross_ref_sheet_entries: false

    // ── Annotation ─────────────────────────────────────
    annotation {
        sort_order: across_then_down
        sort_location: part
        replace_subparts: off
        match_parameter 1 { name: "Comment", strict: true }
        match_parameter 2 { name: "Library Reference", strict: true }
    }

    // ── Documents ──────────────────────────────────────
    document "HydroController.SchDoc" {
        annotation_enabled: true
        annotate_start_value: 1
        class_gen_cc_auto_enabled: true
        class_gen_nc_auto_scope: none
    }

    document "HydroController_Power.SchDoc" {
        annotation_enabled: true
        annotate_start_value: 100
    }

    document "HydroController.PcbDoc" {}

    // ── ERC Connection Matrix ──────────────────────────
    erc_matrix {
        // Override specific cells (row, col) = level
        // Unspecified cells keep Altium defaults
        (pin_output, pin_output): error
        (pin_output, pin_input): no_report
        (pin_passive, pin_passive): no_report
        (pin_power, pin_power): warning
    }

    // ── Electrical Rules Check levels ──────────────────
    erc_levels {
        // Override specific TErrorKind entries
        // Uses snake_case names derived from TErrorKind variants
        unconnected_pin: warning
        duplicate_net_names: error
        missing_power_pin: error
        floating_net_label: warning
    }

    // ── Output Groups ──────────────────────────────────
    output_group "Fabrication Outputs" {
        output "Gerber Files" { type: "Gerber" }
        output "NC Drill Files" { type: "NC Drill" }
        output "ODB++ Files" { type: "ODB" }
    }

    output_group "Assembly Outputs" {
        output "Pick and Place" { type: "Pick Place" }
        output "Assembly Drawings" { type: "Assembly" }
    }

    output_group "Report Outputs" {
        output "Bill of Materials" { type: "BOM_PartType" }
    }

    // ── Comparison Options ─────────────────────────────
    comparison {
        rule "Net" { min_percent: 75, min_match: 3, use_name: auto }
        rule "Net Class" { min_percent: 75, min_match: 3, use_name: auto }
        rule "Component Class" { min_percent: 75, min_match: 3, use_name: auto }
        rule "Differential Pair" { min_percent: 50, min_match: 1, use_name: false }
    }

    // ── Class Generation ───────────────────────────────
    class_gen {
        comp_class_manual_enabled: false
        net_class_auto_bus_enabled: true
        net_class_manual_enabled: true
    }

    // ── Library Update Options ─────────────────────────
    library_update {
        full_replace: true
        do_graphics: true
        do_parameters: true
        do_models: true
        add_models: true
        remove_models: true
        update_current_models: true
    }

    // ── Variants (optional) ────────────────────────────
    variant "Low Cost" {
        description: "Low-cost build variant"
        variation "C10" { kind: not_fitted }
        variation "U3" { kind: alternate, alternate_part: "LM358-CLONE" }
        param_variation "R5" { parameter: "Value", value: "4.7k" }
    }
}
```

---

## Language Extensions Required

### New Keyword: `project`

```
project NAME { properties_and_children }
```

The name becomes the project name / filename stem. Identity key for reconciliation.

### New Child Blocks

| Block | Parent | Identity Key | Purpose |
|---|---|---|---|
| `document PATH { … }` | `project` | document path | Per-document settings |
| `annotation { … }` | `project` | singleton | Annotation settings |
| `erc_matrix { … }` | `project` | singleton | ERC connection matrix overrides |
| `erc_levels { … }` | `project` | singleton | ERC check level overrides |
| `output_group NAME { … }` | `project` | group name | Output job group |
| `output NAME { … }` | `output_group` | output name | Individual output job |
| `comparison { … }` | `project` | singleton | ECO comparison options |
| `rule NAME { … }` | `comparison` | kind name | Per-kind comparison rule |
| `class_gen { … }` | `project` | singleton | Class generation options |
| `library_update { … }` | `project` | singleton | Library update options |
| `variant NAME { … }` | `project` | variant description/name | Project variant |
| `variation DESIGNATOR { … }` | `variant` | designator | Component variation |
| `param_variation DESIGNATOR { … }` | `variant` | designator + parameter | Parameter variation |
| `match_parameter N { … }` | `annotation` | index | Annotation match parameter |

### New Enum Types

The compiler needs these new enum type resolutions:

| Spec enum value | Maps to |
|---|---|
| `smart`, `flat`, `hierarchical_global_ports`, `global`, `hierarchical_strict` | `TFlattenMode` |
| `flat_numeric_with_names`, `flat_alpha_with_names`, `numeric_name_path`, `alpha_name_path`, `mixed_name_path` | `TChannelRoomNamingStyle` |
| `none_style`, `name`, `number` | `TCrossRefSheetStyle` |
| `none_location`, `zone`, `xy` | `TCrossRefLocationStyle` |
| `disabled`, `sheet_entry`, `ports`, `sheet_entry_and_ports` | `TCrossRefPorts` |
| `up_then_across`, `down_then_across`, `across_then_up`, `across_then_down` | `TSortOrder` |
| `designator`, `part_loc` | `TSortLocation` |
| `no_report`, `warning`, `error`, `fatal` | `TErrorLevel` |
| `off`, `on`, `on_case_sensitive` | `TDifferenceCheckLevel` |
| `not_fitted`, `alternate` | `TVariationKind` |
| `pin_input`, `pin_io`, `pin_output`, `pin_open_collector`, `pin_passive`, `pin_hi_z`, `pin_open_emitter`, `pin_power`, `port_input`, `port_output`, `port_bidirectional`, `port_unspecified`, `sheet_entry_input`, `sheet_entry_output`, `sheet_entry_bidirectional`, `sheet_entry_unspecified`, `unconnected` | `TConnectionCode` (for erc_matrix) |

### Lexer Changes

Minimal. The lexer already treats unknown identifiers as `Ident` tokens. New keywords
needed:

- `project` — new top-level entity keyword
- `document`, `annotation`, `erc_matrix`, `erc_levels`, `output_group`, `output`,
  `comparison`, `rule`, `class_gen`, `library_update`, `variant`, `variation`,
  `param_variation`, `match_parameter` — could be keywords OR parsed as identifiers
  in context (like `rectangle`, `line`, etc. for graphics)

**Recommendation:** Keep these as context-sensitive identifiers (not keywords) to avoid
polluting the keyword namespace. The parser knows it's inside a `project` block and
can recognize `document`, `annotation`, etc. positionally — just like it recognizes
`rectangle` inside `component` blocks today.

### Parser Changes

Add to `parse_top_level_item()`:
- `"project"` → `parse_project_declaration()`

`parse_project_declaration()` recognizes child items by identifier:
- `"document"` → `parse_document_block()`
- `"annotation"` → `parse_annotation_block()`
- `"erc_matrix"` → `parse_erc_matrix_block()`
- `"erc_levels"` → `parse_erc_levels_block()`
- `"output_group"` → `parse_output_group_block()`
- `"comparison"` → `parse_comparison_block()`
- `"class_gen"` → `parse_class_gen_block()`
- `"library_update"` → `parse_library_update_block()`
- `"variant"` → `parse_variant_block()`
- Plus bare `key: value` pairs for project-level properties

### AST Changes

New AST nodes in `ast.rs`:

```rust
// In TopLevelItem enum:
Project(Spanned<ProjectDecl>),

// New structs:
struct ProjectDecl {
    name: Spanned<String>,
    items: Vec<Spanned<ProjectItem>>,
}

enum ProjectItem {
    Property(Spanned<String>, Spanned<Expr>),
    Let(Spanned<String>, Spanned<Expr>),
    Document(Spanned<DocumentDecl>),
    Annotation(Spanned<AnnotationDecl>),
    ErcMatrix(Vec<Spanned<ErcMatrixEntry>>),
    ErcLevels(Vec<Spanned<ErcLevelEntry>>),
    OutputGroup(Spanned<OutputGroupDecl>),
    Comparison(Vec<Spanned<ComparisonRuleDecl>>),
    ClassGen(Vec<Spanned<PropertyPair>>),
    LibraryUpdate(Vec<Spanned<PropertyPair>>),
    Variant(Spanned<VariantDecl>),
}

struct DocumentDecl {
    path: Spanned<String>,
    properties: Vec<Spanned<PropertyPair>>,
}

struct AnnotationDecl {
    properties: Vec<Spanned<PropertyPair>>,
    match_parameters: Vec<Spanned<MatchParameterDecl>>,
}

struct OutputGroupDecl {
    name: Spanned<String>,
    outputs: Vec<Spanned<OutputDecl>>,
}

struct OutputDecl {
    name: Spanned<String>,
    properties: Vec<Spanned<PropertyPair>>,
}

// etc.
```

### Model Changes

New model types in `model.rs`:

```rust
// In SpecModel enum:
PrjPcb(PrjPcbSpec),

struct PrjPcbSpec {
    projects: Vec<ProjectSpec>,
}

struct ProjectSpec {
    name: String,

    // [Design] section properties
    hierarchy_mode: Option<FlattenMode>,
    channel_room_naming_style: Option<ChannelRoomNamingStyle>,
    channel_designator_format: Option<String>,
    channel_room_level_separator: Option<String>,

    // Net naming
    allow_port_net_names: Option<bool>,
    allow_sheet_entry_net_names: Option<bool>,
    // ... etc

    // Cross-references
    cross_ref_sheet_style: Option<CrossRefSheetStyle>,
    // ... etc

    // Children
    documents: Vec<DocumentSpec>,
    annotation: Option<AnnotationSpec>,
    erc_matrix: Option<ErcMatrixSpec>,
    erc_levels: Option<ErcLevelsSpec>,
    output_groups: Vec<OutputGroupSpec>,
    comparison: Option<ComparisonSpec>,
    class_gen: Option<ClassGenSpec>,
    library_update: Option<LibraryUpdateSpec>,
    variants: Vec<VariantSpec>,
}
```

### Compiler Changes

Add `compile_project()` to the compiler, similar to `compile_component()` but mapping
to PrjPcb-specific types. The compiler should:

1. Evaluate all `let` bindings in scope
2. Extract typed properties (enums, bools, strings, ints) from expressions
3. Compile child blocks (documents, ERC, outputs, etc.)
4. Validate enum values against allowed ranges

### Reconciler Changes

The PrjPcb reconciler compares `ProjectSpec` against an `AltiumProject` document:

- **Document list:** Match by `DocumentPath`, report adds/removes/updates
- **Scalar properties:** Compare field-by-field, report changes
- **ERC matrix:** Compare cell-by-cell (17×17), report only overridden cells that differ
- **ERC levels:** Compare only levels specified in spec
- **Output groups:** Match by group name, then outputs by output name
- **Variants:** Match by name/description

### Executor Changes

The PrjPcb executor writes an INI file from the spec:

1. Start from blank template (or existing project file)
2. Apply spec properties over defaults
3. Write sections in canonical order (matching Altium's output order)

Since the PrjPcb format is plain text, the executor can be simpler than the
CFB-based executors — just format key-value pairs and write lines.

---

## Implementation Phases

### Phase 1: Read-Only Parser + Format Types

1. Implement `AltiumProject` parser in `altium-format` (INI line parser, section state machine)
2. Add PrjPcb-specific enums to `altium-format-types` (`FlattenMode`, `ChannelRoomNamingStyle`, etc.)
3. Add `altium validate` support for `.PrjPcb` files
4. Add `altium dump` support to reverse-generate `.proj` from existing projects

### Phase 2: Spec Language Extension

1. Add `project` keyword and child block parsing to lexer/parser
2. Add `ProjectDecl` and child AST nodes
3. Add `PrjPcbSpec` and child model types
4. Implement `compile_project()` in the compiler
5. Implement `reconcile_prjpcb()` in the reconciler

### Phase 3: Executor + Writer

1. Implement PrjPcb INI writer (format sections and key-value pairs)
2. Implement `apply_spec_prjpcb()` executor
3. Add `altium apply` support for `.proj` files

### Phase 4: Cross-Domain Integration

1. Allow `.proj` to import `.sym` and `.sym`
2. `altium apply my-project.proj` creates the project AND its library files
3. Document cross-references between project spec and library specs

---

## Design Decisions and Trade-offs

### Decision: One `project` per spec file vs multiple

**Recommendation: One `project` per file.** Unlike SchLib (many components) or PcbLib
(many footprints), a project file describes a single project. Multiple projects in one
spec file would be confusing and has no real-world use case.

### Decision: Section-level blocks vs flat properties

**Recommendation: Use blocks for structured sections.** The ERC matrix, output groups,
and variants are naturally hierarchical. Flattening them into the project level would
make specs unreadable. The block syntax (`erc_matrix { … }`, `output_group NAME { … }`)
matches the conceptual structure.

### Decision: ERC matrix syntax

**Recommendation: Sparse cell overrides.** The full 17×17 matrix is too verbose to
declare in full. Specs should only override specific cells:

```
erc_matrix {
    (pin_output, pin_output): error
    (pin_passive, pin_passive): no_report
}
```

Unspecified cells inherit from the existing document or Altium defaults. This matches
the additive-by-default philosophy.

### Decision: Modification/Difference levels

**Recommendation: Omit from initial spec.** The 161-entry modification levels and
88-entry difference levels are rarely customized and are verbose even in sparse form.
Support can be added later if there's demand. The initial spec should focus on the
most commonly configured settings: hierarchy mode, net naming, cross-references,
documents, ERC, and output jobs.

### Decision: Output job detail level

**Recommendation: Declare output types only, not printer/page options.** The
`PrinterOptions` and `PageOptions` pipe-delimited strings are platform-dependent
(printer names) and extremely verbose. The spec should declare which outputs exist
and their types; printer configuration should remain in the project file.

### Decision: Naming convention

**Recommendation: snake_case for all property names.** This matches the existing spec
language convention (`is_solid`, `pin_mode`, `pad_shape`). The INI key names have
inconsistent casing (`AllowPortNetNames`, `PinSwapBy_Netlabel`); the spec normalizes
to snake_case and the compiler maps to the actual key names.

# `.prjpcb-spec` — Project files

The `.prjpcb-spec` domain describes an Altium PCB project (`.PrjPcb`): its
`[Design]` compilation options, member documents, ERC configuration, output
jobs, comparison rules, and assembly variants. A project spec also acts as the
entry point that `import`s the individual `.schdoc-spec`, `.pcbdoc-spec`,
`.schlib-spec`, and `.pcblib-spec` member files so the whole design can be
compiled or applied in one pass with `--all`.

## Related pages

- [Blocks overview](../language/blocks-overview.md) — how block dispatch works across domains.
- [Annotations](../language/annotations.md) — the `#[annotation(...)]` metadata prefix (distinct from the `annotation { }` block below).
- [Expressions](../language/expressions.md) — `import` references and `$alias` resolution.
- [`.schdoc-spec`](schdoc.md) / [`.pcbdoc-spec`](pcbdoc.md) — the member documents a project references.
- [Apply and plan](../operations/apply-and-plan.md) and [CLI](../operations/cli.md) — running specs, including `--all`.
- [Altium mapping](../reference/altium-mapping.md) — full enum/section reference.

---

## File structure

A `.prjpcb-spec` file contains zero or more `import` directives followed by one
or more top-level `project` blocks.

```
import "<relative-path>" [as <alias>]
...

project <NAME> {
  <design-property>: <value>
  ...
  document "<path>" { ... }
  annotation { ... }
  erc_matrix { ... }
  erc_levels { ... }
  output_group "<name>" { ... }
  comparison { ... }
  class_gen { ... }
  library_update { ... }
  variant "<name>" { ... }
}
```

A single file may declare multiple `project` blocks; each compiles to one
`ProjectSpec` in the resulting `PrjPcbSpec`.

**Maps to Altium:** each `project` block maps to one `.PrjPcb` file. The Altium
project file is an INI-style document whose sections (`[Design]`,
`[Document…]`, `[OutputGroup…]`, `[ProjectVariant…]`, ERC matrix keys, etc.)
are populated from the children below. See `altium_format::project::Project`
(`crates/altium-format/src/project.rs`) and
`altium_format_types::project` (`crates/altium-format-types/src/project.rs`).

---

## `import ... as ...` directives

```
import "<relative-path>"
import "<relative-path>" as <alias>
```

| Form | Effect |
| --- | --- |
| `import "x.schdoc-spec"` | **Bare import** — pulls the file in for `--all` processing; its own entities target its own output file. |
| `import "x.schlib-spec" as lib` | **Named import** — binds the file under alias `lib` so its entities can be referenced (e.g. `$lib.MyComponent`). |

Rules enforced by the import resolver (`src/import.rs`):

- Paths are **relative** to the importing file's directory. Absolute paths are rejected (`SpecErrorCode::FileNotFound`).
- The file extension determines the domain. A `.prjpcb-spec` may import any of
  `.schlib-spec`, `.pcblib-spec`, `.schdoc-spec`, `.pcbdoc-spec`, or another
  `.prjpcb-spec` — all cross-domain combinations are permitted (see the
  `prjpcb_can_import_*` tests in `src/import.rs`).
- Import aliases must be **unique within a file** (`SpecErrorCode::DuplicateImportAlias`).
- Import cycles are detected and rejected (`SpecErrorCode::CircularImport`).
- Imports are resolved in topological order (leaves first).

**Maps to Altium:** named imports are a spec-language linkage mechanism for
referencing other specs' entities (e.g. a `.schdoc-spec` symbol referencing an
imported `.schlib-spec` component via `$alias.Name`). They have no direct
`.PrjPcb` representation; the project file lists its members through `document`
blocks, not `import`. For how a project compiles every imported member, see
[`--all` processing](#--all-processing) below. For `$alias.Name` reference
evaluation, see [Expressions › import references](../language/expressions.md).

---

## `project` block

```
[binding =] project <NAME> { ... }
```

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `<NAME>` | identifier / string / integer | Yes | Project name (`ProjectSpec.name`). |
| body | project items | No | Design properties and child blocks (below). |

### Design properties

All design properties are optional scalars on `ProjectSpec`. A value of `None`
(property omitted) means "do not override the Altium default."

| Property | Type | Maps to | Accepted values |
| --- | --- | --- | --- |
| `hierarchy_mode` | enum | `FlattenMode` | `smart`, `flat`, `hierarchical_global_ports`, `global`, `hierarchical_strict` |
| `channel_room_naming_style` | enum | `ChannelRoomNamingStyle` | `flat_numeric_with_names`, `flat_numeric`, `fully_qualified`, `fully_qualified_short`, `mixed_name_path` |
| `channel_designator_format` | string | `channel_designator_format` | any string |
| `channel_room_level_separator` | string | `channel_room_level_separator` | any string |
| `allow_port_net_names` | bool | `allow_port_net_names` | `true` / `false` |
| `allow_sheet_entry_net_names` | bool | `allow_sheet_entry_net_names` | `true` / `false` |
| `netlist_single_pin_nets` | bool | `netlist_single_pin_nets` | `true` / `false` |
| `append_sheet_number_to_local_nets` | bool | `append_sheet_number_to_local_nets` | `true` / `false` |
| `name_nets_hierarchically` | bool | `name_nets_hierarchically` | `true` / `false` |
| `power_port_names_take_priority` | bool | `power_port_names_take_priority` | `true` / `false` |
| `pin_swap_by_netlabel` | bool | `pin_swap_by_netlabel` | `true` / `false` |
| `pin_swap_by_pin` | bool | `pin_swap_by_pin` | `true` / `false` |
| `cross_ref_sheet_style` | enum | `CrossRefSheetStyle` | `none`, `name`, `number` |
| `cross_ref_location_style` | enum | `CrossRefLocationStyle` | `none`, `zone`, `xy` |
| `cross_ref_ports` | enum | `CrossRefPorts` | `disabled`, `sheet_entry`, `ports`, `sheet_entry_and_ports` |
| `cross_ref_cross_sheets` | bool | `cross_ref_cross_sheets` | `true` / `false` |
| `cross_ref_sheet_entries` | bool | `cross_ref_sheet_entries` | `true` / `false` |
| `output_path` | string | `output_path` | any string |

`let` bindings are also permitted at project scope and are visible to all child
blocks (see [Expressions](../language/expressions.md)).

**Maps to Altium:** these populate the `[Design]` section of the `.PrjPcb`
file. The enum identifiers above are the spec-language keywords; the underlying
integer values are defined in `altium_format_types::project`. Compilation is in
`Compiler::compile_project` (`src/compiler.rs`), enum parsing in
`parse_flatten_mode`, `parse_cross_ref_*`, etc.

---

## `document` blocks (project members)

```
document "<path>" {
  annotation_enabled: <bool>
  annotate_start_value: <int>
  do_library_update: <bool>
  do_database_update: <bool>
}
```

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `<path>` | string | Yes | Path to the member document (e.g. `"Sheet1.SchDoc"`). `DocumentSpec.path`. |
| `annotation_enabled` | bool | No | Whether the document participates in designator annotation. |
| `annotate_start_value` | int | No | Starting designator index for annotation. |
| `do_library_update` | bool | No | Include this document in library-update operations. |
| `do_database_update` | bool | No | Include this document in database-update operations. |

**Maps to Altium:** each `document` block becomes a `[DocumentN]` section of the
`.PrjPcb` file (`Project::documents`, `src/project.rs`). Compiled by
`Compiler::compile_document`.

> A `document` block declares membership and per-document project settings. It
> is independent of `import` directives: `import` links other *specs* for
> reference/`--all` processing, while `document` lists the *Altium documents*
> that belong to the project file itself.

---

## `annotation` configuration block

This is the **board/project designator-annotation settings block**, written
`annotation { ... }`. It is a *different construct* from the
`#[annotation(...)]` sync-metadata prefix that may precede a block — see
[Annotations](../language/annotations.md) for that prefix. The AST type for
this block is `AnnotationBlockDecl`; the prefix is `BlockAnnotation`.

```
annotation {
  sort_order: <enum>
  sort_location: <enum>
  match_parameter <N> { key: value, ... }
  ...
}
```

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `sort_order` | enum | No | `SortOrder`: `up_then_across`, `down_then_across`, `across_then_up`, `across_then_down`. |
| `sort_location` | enum | No | `SortLocation`: `designator`, `part`. |
| `match_parameter <N> { ... }` | block(s) | No | Designator-matching parameter rows, keyed by integer index `N`. |

Each `match_parameter N { ... }` body is an object of key/value pairs stored as
strings in `AnnotationMatchParamSpec { index, properties }`.

**Maps to Altium:** the project annotation configuration (sort order/location)
plus the parameter-match table used by Altium's "Annotate" engine. Compiled by
`Compiler::compile_annotation`; enum parsing in `parse_sort_order` /
`parse_sort_location`.

---

## ERC configuration

### `erc_matrix` — violation matrix overrides

```
erc_matrix {
  (<row_code>, <col_code>): <level>
  ...
}
```

| Field | Type | Description |
| --- | --- | --- |
| `<row_code>` / `<col_code>` | enum | `ConnectionCode` (pin/port/sheet-entry kind). |
| `<level>` | enum | `ErrorLevel`: `no_report`, `warning`, `error`, `fatal`. |

Connection-code keywords (one per `ConnectionCode` variant): `pin_input`,
`pin_bidirectional`, `pin_output`, `pin_open_collector`, `pin_passive`,
`pin_hi_z`, `pin_open_emitter`, `pin_power`, `sheet_entry_input`,
`sheet_entry_bidirectional`, `sheet_entry_output`, `port_unspecified`,
`pin_unspecified`, `sheet_entry_unspecified`, `port_input`, `port_output`,
`unconnected`.

Each entry compiles to an `ErcMatrixOverride { row, col, level }`. Only
non-default (non-`no_report`) cells need to be listed.

### `erc_levels` — named ERC report-level overrides

```
erc_levels {
  <name>: <level>
  ...
}
```

| Field | Type | Description |
| --- | --- | --- |
| `<name>` | identifier | ERC report category name (`ErcLevelOverride.name`). |
| `<level>` | string / int | `ErrorLevel`, accepted as a keyword string (`"warning"`) or its integer code (`0`–`3`). |

Each entry compiles to an `ErcLevelOverride { name, level }`.

**Maps to Altium:** the `erc_matrix` cells map to the `.PrjPcb` ERC violation
matrix (`Project::erc_matrix`); `erc_levels` maps to named report-level
overrides. `ErrorLevel` is encoded as a single character (`N`/`W`/`E`/`F`) in
the matrix — see `ErrorLevel::to_matrix_char` in
`altium_format_types::project`. Compiled by `compile_erc_matrix_entry` /
`compile_erc_level_entry`.

---

## Output jobs

### `output_group` / `output`

```
output_group "<name>" {
  description: "<text>"
  output "<name>" {
    output_type: "<type>"
    document_path: "<path>"
    variant_name: "<variant>"
  }
  ...
}
```

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `output_group` name | string | Yes | Group name (`OutputGroupSpec.name`). |
| `description` | string | No | Group description. |
| `output` name | string | Yes | Output job name (`OutputSpec.name`). |
| `output_type` | string | No | Output kind (e.g. Gerber, BOM); stored as a free string. |
| `document_path` | string | No | Source document for this output. |
| `variant_name` | string | No | Assembly variant to generate the output for. |

**Maps to Altium:** output groups/jobs in the `[OutputGroupN]` sections of the
`.PrjPcb` file (`Project::output_groups`, `OutputGroupRaw`). Compiled by
`Compiler::compile_output_group`.

---

## `comparison` — comparator rules

```
comparison {
  rule "<Kind>" { key: value, ... }
  ...
}
```

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `rule` kind | string | Yes | Comparator rule kind (`ComparisonRuleSpec.kind`). |
| body | object | No | Rule parameters, stored as string key/value pairs in `ComparisonRuleSpec.properties`. |

**Maps to Altium:** the project comparator configuration
(`Project::comparison_options`). Difference-check sensitivity values correspond
to `DifferenceCheckLevel` in `altium_format_types::project`. Compiled by
`Compiler::compile_comparison_rule`.

---

## `class_gen` and `library_update`

```
class_gen {
  generate_component_classes: <bool>
  generate_net_classes: <bool>
}

library_update {
  update_components: <bool>
  update_models: <bool>
}
```

| Block | Property | Type | Maps to |
| --- | --- | --- | --- |
| `class_gen` | `generate_component_classes` | bool | `ClassGenSpec.generate_component_classes` |
| `class_gen` | `generate_net_classes` | bool | `ClassGenSpec.generate_net_classes` |
| `library_update` | `update_components` | bool | `LibraryUpdateSpec.update_components` |
| `library_update` | `update_models` | bool | `LibraryUpdateSpec.update_models` |

Both are plain property blocks (parsed by `parse_property_block`).

**Maps to Altium:** the `[ClassGenerationOptions]` / library-update settings
sections (`Project::class_gen`, `Project::library_update_options`).

---

## Variants

```
variant "<name>" {
  description: "<text>"
  variation "<designator>" {
    kind: <enum>
    alternate_part: "<part>"
  }
  param_variation "<designator>" {
    parameter: "<name>"
    value: "<value>"
  }
  ...
}
```

### `variant`

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| name | string | Yes | Variant name (`VariantSpec.name`). |
| `description` | string | No | Variant description. |

### `variation` — per-component fit/alternate

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| designator | string | Yes | Component designator the variation applies to. |
| `kind` | enum | No | `VariationKind`: `none`, `not_fitted`, `alternate`. |
| `alternate_part` | string | No | Replacement part for `alternate` variations. |

### `param_variation` — per-component parameter override

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| designator | string | Yes | Component designator. |
| `parameter` | string | Yes (effective) | Parameter name to override (defaults to empty if omitted). |
| `value` | string | Yes (effective) | New parameter value (defaults to empty if omitted). |

**Maps to Altium:** each `variant` becomes a `[ProjectVariant…]` section
(`Project::variants`); `variation`/`param_variation` rows are the component
variations within it. `VariationKind` integer values are in
`altium_format_types::project`. Compiled by `Compiler::compile_variant`.

---

## `--all` processing

By default `altium plan`/`altium apply` operate on a single spec file. The
`--all` flag (valid **only** for `.prjpcb-spec` files) additionally processes
every spec the project imports.

```
altium plan  project.prjpcb-spec --all
altium apply project.prjpcb-spec --all
```

Behavior (`run_plan` / `run_apply` in `crates/altium-cli/src/main.rs`):

1. The root project spec is compiled and reconciled/applied first.
2. With `--all`, each imported spec path (bare **and** named imports) is then
   compiled and reconciled/applied against its own default output document.
3. For `plan`, each import's ECO is printed under a `--- <path> ---` header and
   the overall "has changes" result is the OR across the root and all imports.
4. Passing `--all` with a non-`.prjpcb-spec` file is a hard error:
   `"--all is only valid for .prjpcb-spec files"`.

This lets one command compile/apply an entire project — its schematics,
boards, and libraries — from the single project spec that imports them.

See [Apply and plan](../operations/apply-and-plan.md) and the
[CLI reference](../operations/cli.md).

---

## Worked example

```
# project.prjpcb-spec

# Member documents are linked for --all processing.
import "main.schdoc-spec"
import "board.pcbdoc-spec"
# A named import lets schematics reference library components by alias.
import "passives.schlib-spec" as lib

project "MyBoard" {
    hierarchy_mode: flat
    cross_ref_sheet_style: name
    cross_ref_location_style: zone
    name_nets_hierarchically: true
    output_path: "Outputs"

    document "main.SchDoc" {
        annotation_enabled: true
        annotate_start_value: 1
        do_library_update: true
    }

    document "board.PcbDoc" {
        annotation_enabled: false
    }

    annotation {
        sort_order: down_then_across
        sort_location: designator
        match_parameter 1 {
            parameter: "Comment"
        }
    }

    erc_matrix {
        (pin_output, pin_output): error
        (pin_power, pin_passive): warning
    }

    erc_levels {
        "Duplicate Net Names": fatal
    }

    output_group "Fabrication" {
        description: "Gerbers + drill"
        output "Gerber Files" {
            output_type: "Gerber"
            document_path: "board.PcbDoc"
        }
    }

    comparison {
        rule "NetName" { tolerance: "Exact" }
    }

    class_gen {
        generate_component_classes: true
        generate_net_classes: false
    }

    library_update {
        update_components: true
        update_models: true
    }

    variant "Low-Cost" {
        description: "BOM-reduced build"
        variation "R5" {
            kind: not_fitted
        }
        variation "U2" {
            kind: alternate
            alternate_part: "OPA2333"
        }
        param_variation "R1" {
            parameter: "Value"
            value: "10k"
        }
    }
}
```

Compile and apply the whole project, including its imported member specs:

```
altium apply project.prjpcb-spec --all
```

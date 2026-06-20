# Altium mapping reference

A cross-reference from each spec-language construct to the Altium concept it
produces — the record type, primitive, stream, or document feature. Use this to
trace a `.spec` declaration to the binary artifact it creates or edits.

The mapping is derived from the compiler
([`src/compiler.rs`](../../../crates/altium-format-spec/src/compiler.rs), AST →
`SpecModel`) and the executor
([`src/executor.rs`](../../../crates/altium-format-spec/src/executor.rs),
`SpecModel` → mutated Altium document via `altium_format::api`). Altium domain
types referenced here live in
[`altium-format-types`](../../../crates/altium-format-types/src/). The compiler
selects a target document from `SpecModel`/`SpecDomain`
([`src/model.rs`](../../../crates/altium-format-spec/src/model.rs)):
`SchLib`, `PcbLib`, `SchDoc`, `PcbDoc`, `PrjPcb`.

**Related pages**

- [Grammar reference](grammar.md) — the productions named below
- [Keyword reference](keywords.md) — the tokens involved
- Format deep-dives in [`docs/dxp/`](../../dxp/README.md)

## How to read this table

- **Spec construct** — the grammar production (see [grammar](grammar.md)).
- **`api::` type / target** — the `altium_format::api` type the executor builds,
  or the document mutation it performs.
- **Altium concept** — the record, primitive, or feature in the binary file.
- **Deeper docs** — the relevant [`docs/dxp/`](../../dxp/README.md) page.

The `api` module is `altium-format`'s public construction surface; the names
below (`api::Component`, `api::Pad`, `api::PcbGraphic::Track`, …) are the exact
types the executor instantiates.

## SchLib (`.SchLib` — symbol library)

Target document: `SchLib`. Compiler: `compile_component`; executor:
`apply_spec_schlib` and friends.

| Spec construct | `api::` type / target | Altium concept | Deeper docs |
| --- | --- | --- | --- |
| `component NAME { … }` | `api::Component` | Schematic component / symbol (`RECORD=1` Component) | [schematic-records](../../dxp/schematic-records.md), [sch-files](../../dxp/sch-files.md) |
| `part N { … }` | sets part count / part on `api::Component` | Multi-part component (`PARTCOUNT`, owner part index) | [schematic-records](../../dxp/schematic-records.md) |
| `pin NAME { … }` | `api::Pin` | Pin record (`RECORD=2`) with pin sidecar streams | [schematic-records](../../dxp/schematic-records.md), [sidecar-streams-deep-dive](../../dxp/sidecar-streams-deep-dive.md) |
| `parameter NAME { … }` | `api::Parameter` | Component parameter (`RECORD=41`) | [schematic-records](../../dxp/schematic-records.md) |
| `alias NAME` | adds an alias on `api::Component` | Component alias name | [sch-files](../../dxp/sch-files.md) |
| `footprint REF { … }` (map) | `api::FootprintMap` / `api::PinPadMap` | Implementation/model link (`RECORD=45`/`46`) + pin-pad mapping | [sch-files](../../dxp/sch-files.md) |
| `line/rectangle/arc/…` (graphic) | `api::Graphic` (`LineGraphic`, `RectangleGraphic`, `RoundRectangleGraphic`, `ArcGraphic`, `EllipticalArcGraphic`, `EllipseGraphic`, `PieGraphic`, …) | Drawing primitives (`RECORD=13` Line, `14` Rectangle, `12` Arc, etc.) | [schematic-records](../../dxp/schematic-records.md) |
| `swap_group NAME { … }` | swap metadata on component | Pin/part swapping group | [sch-files](../../dxp/sch-files.md) |
| `import "lib.SchLib" as A` | loads an imported `SchLibSpec` | Symbol source for `$A.Component` references | [sch-files](../../dxp/sch-files.md) |

Pin angles use `api::SchAngle`; coordinates use `altium_format_types::Coord`;
colors use `altium_format_types::color::Color`.

## PcbLib (`.PcbLib` — footprint library)

Target document: `PcbLib`. Compiler: `compile_footprint` (`FootprintSpec`,
`PadSpec`, `PcbGraphicSpec`); executor: `pad_from_pcblib_spec`,
`pcb_graphic_from_spec`, `merge_spec_into_footprint`.

| Spec construct | `api::` type / target | Altium concept | Deeper docs |
| --- | --- | --- | --- |
| `footprint NAME { … }` | `api::Footprint` | PCB component / footprint storage in the library | [pcb-files](../../dxp/pcb-files.md), [pcb-records](../../dxp/pcb-records.md) |
| `pad NAME { … }` | `api::Pad` (+ `api::PadStack`) | Pad primitive (object id Pad), per-layer pad stack, hole/relief fields | [pcb-records](../../dxp/pcb-records.md), [altium-pad-field-analysis](../../dxp/altium-pad-field-analysis.md) |
| `row` / `column` / `grid { … }` | expands into multiple `api::Pad`s | Generated pad arrays (no direct record; lowering convenience) | [pcb-records](../../dxp/pcb-records.md) |
| `track { … }` (graphic) | `api::PcbGraphic::Track` (`TrackGraphic`) | Track primitive | [pcb-records](../../dxp/pcb-records.md) |
| `arc { … }` (graphic) | `api::PcbGraphic::Arc` (`PcbArcGraphic`) | Arc primitive | [pcb-records](../../dxp/pcb-records.md) |
| `fill { … }` (graphic) | `api::PcbGraphic::Fill` (`FillGraphic`) | Fill primitive | [pcb-records](../../dxp/pcb-records.md) |
| `region { … }` (graphic) | `api::PcbGraphic::Region` (`RegionGraphic`, `PcbContour`) | Region primitive with contour | [pcb-records](../../dxp/pcb-records.md) |
| `text { … }` (graphic) | `api::PcbGraphic::Text` (`TextGraphic`) | String/text primitive | [pcb-records](../../dxp/pcb-records.md) |
| `via { … }` (graphic) | `api::PcbGraphic::Via` (`ViaGraphic`) | Via primitive (multi-layer) | [pcb-records](../../dxp/pcb-records.md), [altium-via-field-analysis](../../dxp/altium-via-field-analysis.md) |
| `component_body { … }` (graphic) | `api::PcbGraphic::ComponentBody` (`ComponentBodyGraphic`) | 3D body / component body primitive | [pcb-records](../../dxp/pcb-records.md) |
| `line` / `polyline` (graphic) | rejected (error: "use track instead") | n/a — not a PcbLib primitive | [pcb-records](../../dxp/pcb-records.md) |

Layers resolve through `LayerRef` / `V6Layer`; pad shapes use
`altium_format_types::pcb::PadShape`; plane-connection style uses
`PlaneConnectionStyle`. See [coordinates](../../dxp/coordinates.md) for the
coordinate model.

## SchDoc (`.SchDoc` — schematic sheet)

Target document: `SchDoc`. Compiler: `compile_schdoc` (`SheetSpec`,
`SchDocComponentSpec`, `NetSpec`, `PowerSpec`, `SchDocObjectSpec`); executor
emits `api::SheetObject` variants and resolves pin connections.

| Spec construct | `api::` type / target | Altium concept | Deeper docs |
| --- | --- | --- | --- |
| `sheet { … }` | sheet metadata on the SchDoc | Sheet record (`RECORD=31`) / document header | [sch-files](../../dxp/sch-files.md) |
| `fonts { font N { … } }` | font table entries | Font table in the sheet header | [sch-files](../../dxp/sch-files.md) |
| `component NAME { … }` (placed) | `api::Component` placement | Placed component instance + designator | [schematic-records](../../dxp/schematic-records.md) |
| `pin X -> #NET` | wire stub + `api::NetLabel` (executor `resolve_pin_connections`) | Wire (`RECORD=27`) + Net Label (`RECORD=25`) | [schematic-records](../../dxp/schematic-records.md) |
| `pin X -> #PWR` (power net) | wire stub + `api::PowerObject` | Wire + Power Port (`RECORD=17`), `PowerObjectStyle` | [schematic-records](../../dxp/schematic-records.md) |
| `pin X -> nc` | `api::NoConnect` | No-ERC / No Connect marker (`RECORD=22`) | [schematic-records](../../dxp/schematic-records.md) |
| `net NAME { pins: […] }` | net classification + `api::Wire`/`api::NetLabel` | Signal net (wires + net labels) | [schematic-records](../../dxp/schematic-records.md) |
| `power NAME { … }` | `api::PowerObject` (`PowerObjectStyle`) | Power port net | [schematic-records](../../dxp/schematic-records.md) |
| `wire { … }` | `api::SheetObject` (Wire) | Wire (`RECORD=27`) | [schematic-records](../../dxp/schematic-records.md) |
| `bus { … }` | `api::SheetObject` (Bus) | Bus (`RECORD=26`) | [schematic-records](../../dxp/schematic-records.md) |
| `net_label NAME { … }` | `api::SheetObject` (NetLabel) | Net Label (`RECORD=25`) | [schematic-records](../../dxp/schematic-records.md) |
| `power_object NAME { … }` | `api::SheetObject` (PowerObject) | Power Port (`RECORD=17`) | [schematic-records](../../dxp/schematic-records.md) |
| `port NAME { … }` | `api::SheetObject` (Port) | Port (`RECORD=18`) | [schematic-records](../../dxp/schematic-records.md) |
| `junction { … }` | `api::SheetObject` (Junction) | Junction (`RECORD=29`) | [schematic-records](../../dxp/schematic-records.md) |
| `no_connect { … }` | `api::SheetObject` (NoConnect) | No-ERC marker (`RECORD=22`) | [schematic-records](../../dxp/schematic-records.md) |
| `bus_entry { … }` | `api::SheetObject` (BusEntry) | Bus Entry (`RECORD=28`) | [schematic-records](../../dxp/schematic-records.md) |
| `sheet_symbol { entry … }` | `api::SheetSymbolChild` / `api::SheetEntry` | Sheet Symbol (`RECORD=15`) + Sheet Entry (`RECORD=16`) | [schematic-records](../../dxp/schematic-records.md) |
| `parameter_set { … }` | `api::SheetObject` (ParameterSet) | Parameter Set (`RECORD=43`) | [schematic-records](../../dxp/schematic-records.md) |
| `note { … }` | `api::SheetObject` (Note) | Note / textual annotation | [schematic-records](../../dxp/schematic-records.md) |
| `probe { … }` | `api::SheetObject` (Probe) | Probe directive | [schematic-records](../../dxp/schematic-records.md) |
| `compile_mask { … }` | `api::SheetObject` (CompileMask) | Compile mask | [schematic-records](../../dxp/schematic-records.md) |
| `blanket { … }` | `api::SheetObject` (Blanket) | Blanket directive | [schematic-records](../../dxp/schematic-records.md) |
| `harness_connector { … }` | `api::SheetObject` (HarnessConnector) | Harness Connector | [schematic-records](../../dxp/schematic-records.md) |
| `signal_harness { … }` | `api::SheetObject` (SignalHarness) | Signal Harness | [schematic-records](../../dxp/schematic-records.md) |
| `parameter NAME { … }` (top level) | `api::SheetObject` (ParameterSet/Parameter) | Sheet-level parameter | [schematic-records](../../dxp/schematic-records.md) |
| `<graphic> { … }` | `api::Graphic` / `api::LineGraphic` | Sheet drawing primitive | [schematic-records](../../dxp/schematic-records.md) |
| `constraint <kind> { … }` (in `sheet`) | placement-constraint metadata | Placement/layout hint (consumed by placement) | [sch-files](../../dxp/sch-files.md) |

The signal-vs-power classification of nets is decided by a compiler pre-pass
(`power_declarations` on `SheetSpec`); `pin X -> #NET` resolution and orientation
transforms are in `resolve_pin_connections` / `transform_pin_orientation`.

## PcbDoc (`.PcbDoc` — PCB board)

Target document: `PcbDoc`. Compiler: `compile_pcbdoc` (`BoardSpec`,
`PcbDocPrimitive`, `PolygonSpec`, `PcbDocRuleSpec`, `ClassSpec`,
`DifferentialPairSpec`, `PcbDocNetSpec`, `PlacementSpec`); executor
`apply_sync_changes_to_pcbdoc` and primitive builders.

| Spec construct | `api::` type / target | Altium concept | Deeper docs |
| --- | --- | --- | --- |
| `board NAME { … }` | board settings (outline, layers, keepouts) | Board record / layer stack | [pcb-files](../../dxp/pcb-files.md) |
| `track { … }` | `api::PcbGraphic::Track` | Track primitive on a board layer | [pcb-records](../../dxp/pcb-records.md) |
| `arc { … }` | `api::PcbGraphic::Arc` | Arc primitive | [pcb-records](../../dxp/pcb-records.md) |
| `via { … }` | `api::PcbGraphic::Via` | Via primitive | [pcb-records](../../dxp/pcb-records.md), [altium-via-field-analysis](../../dxp/altium-via-field-analysis.md) |
| `fill { … }` | `api::PcbGraphic::Fill` | Fill primitive | [pcb-records](../../dxp/pcb-records.md) |
| `text { … }` | `api::PcbGraphic::Text` | String/text primitive | [pcb-records](../../dxp/pcb-records.md) |
| `region { … }` | `api::PcbGraphic::Region` | Region primitive | [pcb-records](../../dxp/pcb-records.md) |
| `component_body { … }` | `api::PcbGraphic::ComponentBody` | 3D component body | [pcb-records](../../dxp/pcb-records.md) |
| `dimension { … }` | dimension primitive | Dimension object | [pcb-records](../../dxp/pcb-records.md) |
| `pad NAME { … }` (top level) | `api::Pad` | Free pad primitive | [pcb-records](../../dxp/pcb-records.md) |
| `pad_net PAD: "NET"` (in component) | net assignment on a pad | Pad → net binding | [pcb-files](../../dxp/pcb-files.md) |
| `polygon NAME { … }` | polygon pour | Polygon / copper pour | [pcb-records](../../dxp/pcb-records.md) |
| `rule NAME { … }` | `PcbDocRuleSpec` (with `scope2` for 2-object rules) | Design rule | [pcb-files](../../dxp/pcb-files.md) |
| `class NAME { … }` | net/component class | Object class | [pcb-files](../../dxp/pcb-files.md) |
| `differential_pair NAME { … }` | differential-pair definition | Differential pair | [pcb-files](../../dxp/pcb-files.md) |
| `net …` (`PcbDocNetSpec`) | PCB net (with `routing_style`) | Net object | [pcb-files](../../dxp/pcb-files.md) |
| `routing { … }` | routing directives | Routing intent (tooling-level) | [pcb-files](../../dxp/pcb-files.md) |
| `placement { … }` | placement solver input | Component placement intent (tooling-level) | [pcb-files](../../dxp/pcb-files.md) |

PCB nets use `ConnectionCode` (`altium_format_types::project`) and
`PlaneConnectionStyle` (`altium_format_types::pcb`). Coordinates and layers use
`Coord`, `LayerRef`/`V6Layer`.

## Placement directives

The `placement { … }` block is a tooling-level construct: it drives the
placement solver rather than emitting Altium records directly. Its sub-blocks
compile into `PlacementSpec` fields.

| Spec construct | Compiled to | Effect |
| --- | --- | --- |
| `place D1, D2 { … }` | placement entries (`PlaceDecl`) | Fix / constrain designator positions |
| `left_of` / `right_of` / `above` / `below` | directional constraints | Relative-position constraints between `$refs` |
| `group NAME { … }` | placement group | Cluster components |
| `separate $a, $b { gap: … }` | separation constraint | Minimum-gap constraint between groups |
| `minimize wirelength` (+ `subject_to`) | optimization objective | Solver objective + relaxation hints |
| `optimize { … }` / `clearance { … }` / `autoplace { … }` | solver settings | Optimizer / clearance / auto-place configuration |

See [placement blocks](../blocks/placement.md).

## PrjPcb (`.PrjPcb` — project)

Target document: `PrjPcb`. Compiler: `compile_project` (`PrjPcbSpec`). Project
items map to project-file features (INI-style sections and ECO/output config)
rather than to schematic/PCB primitives.

| Spec construct | Compiled to | Altium concept |
| --- | --- | --- |
| `project NAME { … }` | `ProjectSpec` | Project file (`.PrjPcb`) |
| `document "file" { … }` | document entry | Member document reference + per-document options |
| `annotation { match_parameter N { … } }` | annotation config | Designator annotation scheme + match parameters |
| `erc_matrix { (row, col): level }` | ERC matrix | Electrical Rule Check connection matrix (`ConnectionCode`) |
| `erc_levels { name: level }` | ERC level overrides | ERC violation severities |
| `output_group "G" { output "O" { … } }` | output job config | Output group / output containers |
| `comparison { rule "Kind" { … } }` | comparison config | Schematic↔PCB comparison rules |
| `class_gen { … }` | class-generation config | Automatic class generation |
| `library_update { … }` | library-update config | Update-from-library options |
| `variant "V" { variation … / param_variation … }` | variant config | Assembly variant + variations |

See [prjpcb blocks](../blocks/prjpcb.md) and
[invariants](../../dxp/invariants.md).

## Cross-cutting: annotations, imports, bindings

| Spec construct | Compiled to | Purpose |
| --- | --- | --- |
| `#[annotation(id=…, stable=…, group=…, source_id=…)]` | `CompiledAnnotation` on the block | Stable identity for the sync/ECO system (not an Altium record) |
| `import "path" [as A]` | resolved sub-`SpecModel` / import alias map | Pulls in symbols/footprints from another spec or library |
| `NAME = <block>` (binding) | named entry referenced by `$NAME` | Lets later blocks reference earlier entities structurally |

Annotations feed the diff/ECO machinery in `sync.rs` and `reconciler.rs`; they
do not themselves serialize to the binary file. See
[Annotations](../language/annotations.md) and
[operations: apply and plan](../operations/apply-and-plan.md).

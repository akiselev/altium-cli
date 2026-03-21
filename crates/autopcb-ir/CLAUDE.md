# autopcb-ir

Format-independent PCB intermediate representation. Downstream consumers (router,
DRC, placer, viewer) depend only on this crate — never on `altium_format`.

See `README.md` for architecture, data flow, and scope resolution.

## Module Index

| Module | What | When |
| --- | --- | --- |
| `spec_compiler.rs` | `spec_to_ir()` — the only IR compilation path; converts `PcbDocSpec` to `PcbIr` | Implementing or debugging spec-to-IR compilation; adding new spec fields |
| `compile_error.rs` | `IrCompileError` enum — compilation errors from `spec_to_ir()` and `import_pcbdoc()` | Adding new hard-error conditions to the compiler |
| `pcbdoc_import.rs` | `import_pcbdoc()` adapter and `merge_pcbdoc_spec()` — converts `PcbDocBoard` to `PcbDocSpec` | Adding PcbDoc field support to the import adapter; debugging merge behavior |
| `spec_bridge.rs` | `load_ir_from_spec()` — full pipeline: open PcbDoc → import → merge → compile; `apply_component_pose()` | Entry point for all CLI and viewer code that loads IR from a spec file |
| `rule.rs` | `IrDesignRule`, `IrRuleParams`, `IrRuleScope`, `IrRuleScopePair` | Adding rule kinds; understanding scope resolution types |
| `extract.rs` | `PcbIr` struct definition; legacy direct PcbDoc extraction (bypasses spec pipeline) | Inspecting PcbIr fields; legacy direct extraction path |
| `handles.rs` | Typed handle IDs: `ComponentId`, `NetId`, `LayerId`, `PadId`, `RuleId`, etc.; `IdMap` | Adding new handle types; understanding sequential ID assignment |
| `layer_stack.rs` | `IrLayerStack`, `IrCopperLayer`, `PreferredDirection` | Working with copper layer ordering or routing direction hints |
| `net.rs` | `IrNet`, `IrNetPin` — net membership, diff-pair links, net-class assignment | Net-class scope resolution; diff-pair partner linking |
| `component.rs` | `IrComponent`, `IrComponentPad`, `PadShapeInfo`, `PadShapeKind` | Component placement, pad geometry, bounding box computation |
| `copper.rs` | `FreeCopperGeometry`, `IrTrack`, `IrVia`, `IrArc`, `IrFill` | Free copper (not component-owned) tracks and vias |
| `polygon.rs` | `IrPolygon` — copper fill zones | Polygon pour net/layer resolution |
| `board.rs` | `IrBoardGeometry`, `IrKeepoutZone` — board outline and keepout zones | Board boundary or keepout zone queries |
| `region.rs` | `IrRegion`, `IrRegionKind` | Copper regions and their classification |
| `text.rs` | `IrText` — PCB text objects | Text primitive handling in IR |
| `component_body.rs` | `IrComponentBody` — 3D body footprints | 3D clearance checks |
| `dimension.rs` | `IrDimension` — dimension annotations | Dimension primitive handling |
| `types.rs` | `PointMm`, `BoundingBoxMm`, `BoardSide` — coordinate primitives | Geometry helpers throughout the crate |

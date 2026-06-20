# Altium Mapping Reference

This page maps spec domains to the current compiler, model, executor, and public API layers. Field-level mapping belongs in code so it cannot drift into a second schema.

| Domain | Compiler/model | Apply path | Document API | Format guide |
| --- | --- | --- | --- | --- |
| SchLib | `compile_component`, `SchLibSpec` | `apply_spec_schlib` | `api::Component`, `Pin`, `Parameter`, `FootprintMap`, `Graphic` | [Schematic](../../format/schematic.md) |
| SchDoc | `compile_schdoc`, `SchDocSpec` | `apply_spec_schdoc` | `api::SchDocSheet`, `SchDocComponent`, `SheetObject`, `ComponentChild` | [Schematic](../../format/schematic.md) |
| PcbLib | `compile_footprint`, `PcbLibSpec` | `apply_spec_pcblib` | `api::Footprint`, `Pad`, `PcbGraphic`, `PadStack` | [PCB](../../format/pcb.md) |
| PcbDoc | `compile_pcbdoc`, `PcbDocSpec` | `apply_spec_pcbdoc` | `api::PcbDocBoard`, `PcbDocComponent`, nets, rules, classes, primitives | [PCB](../../format/pcb.md) |
| PrjPcb | `compile_project`, `PrjPcbSpec` | `apply_spec_prjpcb` | project API types | [PrjPcb](../../format/prjpcb.md) |

Authoritative implementation files:

- [`compiler.rs`](../../../crates/altium-format-spec/src/compiler.rs): AST to typed spec model
- [`model.rs`](../../../crates/altium-format-spec/src/model.rs): compiled domain models
- [`executor.rs`](../../../crates/altium-format-spec/src/executor.rs): typed model to document mutation
- [`reconciler.rs`](../../../crates/altium-format-spec/src/reconciler.rs): document/spec comparison and ECO generation
- [`dump.rs`](../../../crates/altium-format-spec/src/dump.rs): document to spec text
- [`altium-format/src/api`](../../../crates/altium-format/src/api/): public semantic document types

## Mapping rules

- Schematic declarations lower to typed schematic API objects and ultimately `SchRecordType` records.
- PCB declarations lower to typed PCB API objects and ultimately `PcbObjectId` primitives or typed PcbDoc parameter sections.
- `row`, `column`, and `grid` are compiler conveniences that expand into pads; they have no direct Altium record.
- Placement and routing blocks describe tooling intent. They are not automatically equivalent to persisted Altium primitives.
- Annotations provide spec/ECO identity metadata; they are not Altium records.
- Unsupported constructs and properties must fail during compile or apply. A parsed construct must never be silently discarded.

Use the block reference pages for supported syntax and the implementation files above to verify exact field flow.

# altium-format-spec

Spec DSL crate: parse, compile, execute, reconcile, dump, and sync Altium spec files.

## Read first: greenfield vs brownfield

Before changing the SchDoc executor/dump/reconciler (or any inline-children,
identity, or two-sided change-set behavior), read
`docs/spec-lang/explanation/greenfield-vs-brownfield.md`. It defines whether the
spec or the Altium files are authoritative, which decides whether inline children
are materialized verbatim (brownfield) or treated as overrides on an imported
symbol (greenfield), how object identity is tracked (UniqueId → embedded typed spec
params → structural match), and why `plan`/`apply` must write both the source spec
(linking annotations) and the destination document.

## Index

| File              | Contents (WHAT)                                                                   | Read When (WHEN)                                                        |
| ----------------- | --------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `README.md`       | Architecture, design decisions, invariants, sync IR rationale                     | Understanding overall spec pipeline or sync system design               |
| `src/lexer.rs`    | Tokenizer; `TokenKind` including `Hash` for `#` annotation prefix and `Arrow` (`->`) for pin connections | Debugging parse errors, adding new syntax tokens                        |
| `src/parser.rs`   | Recursive-descent parser; `parse_annotation()`, all `parse_*_decl()` methods, `parse_pin_connection_decl()` | Adding new spec syntax, debugging parser errors                         |
| `src/ast.rs`      | AST node types; `BlockAnnotation`, `AnnotationKey`, all `*Decl` structs; `PinConnectionDecl`, `PinConnectionTarget` | Modifying the spec grammar or AST structure                 |
| `src/compiler.rs` | AST → SpecModel lowering; unit resolution, layer resolution, `compile_annotation()` with `seen_ids`; power pre-pass for signal/power classification; `SymbolRef::Import` validation against imported SchLib; extracts `routing_style` string from net property blocks into `PcbDocNetSpec` | Changing compilation behavior, adding new model fields   |
| `src/model.rs`    | `SpecModel` and all typed spec structs; `annotation: Option<CompiledAnnotation>` on 15+ types; `PinConnectionSpec`, `PinConnectionTarget`, `power_declarations` on `SheetSpec`; `SyncComponent.source_unique_id`, `source_hierarchical_path`, `parameters` fields; `BoardSpec` geometry fields (`outline`, `keepouts`, `layers`); `KeepoutSpec`, `BoardLayerSpec`, `PadGeometrySpec`; `PcbDocRuleSpec.scope2` for two-object rules; split `SchLibSpec`/`PcbLibSpec`; `SpecDomain` variants: `SchLib`, `PcbLib`, `SchDoc`, `PcbDoc`, `PrjPcb`; `PcbDocNetSpec.routing_style: Option<String>` | Reading or writing spec model fields; working with board geometry or rule scope in spec files; tracing how `routing_style` flows from spec text to IR |
| `src/annotation.rs` | `CompiledAnnotation`, `generate_short_id()`, `validate_short_id()`, `compile_annotation()` | Working with annotation IDs, understanding ID format      |
| `src/sync.rs`     | `SyncSnapshot` IR, `SyncComponent` (with `source_unique_id`, `source_hierarchical_path`, `parameters`), `SyncPin` (designator = pad designator, not pin name), `project_schdoc_spec()`, `project_pcbdoc_spec()`, `build_pin_to_pad_map()`, `diff_snapshots()`, `filter_changes()`, `apply_sync_changes_to_pcbdoc()`, ECO report renderer | Implementing or debugging spec-to-spec sync, understanding pin→pad resolution |
| `src/validator.rs`| Phase 3 checks: `validate_schdoc_spec()`, `validate_pcbdoc_spec()`; duplicate designators, dangling net refs, duplicate annotation IDs | Running spec validation, understanding error codes |
| `src/resolver.rs` | Phase 4 library resolution: `resolve_schdoc_spec()`, `FootprintResolvedSpec`; designator → footprint mapping | Resolving footprints from SchLib, debugging library lookup |
| `src/executor.rs` | `apply_spec_*()`: SpecModel → mutate Altium document; `resolve_pin_connections()` generates wire stubs, NetLabels, PowerObjects, NoConnect markers from `PinConnectionSpec`; `transform_pin_orientation()`, `remap_label_orient()` | Changing how spec apply works for any document type, debugging pin connection placement |
| `src/reconciler.rs` | `reconcile_*()`: SpecModel diff document → `EngineeringChangeOrder`             | Changing reconcile/ECO diff behavior                                    |
| `src/dump.rs`     | `dump_*()`: Altium document → `.spec` text; emits `#[annotation(...)]` before each block | Changing dump output format or annotation emission                |
| `src/formatter.rs`| `format_spec()`: spec text reformatter; annotation line formatting; `fmt_pin_connection()` for `pin X -> #NET` and `pin X -> nc` | Changing spec formatting rules                                          |
| `src/eval.rs`     | Expression evaluator; `SpecError`, `SpecErrorCode`, `Severity`; `Value`, `ScopeStack`; `Value::ImportObject` (import alias map), `Value::ImportRef` (provenance-tracked import field access) | Adding new expression types, changing import reference evaluation |
| `src/eco.rs`      | `EngineeringChangeOrder`, `EntityChange`, `EntityKind`, `EcoSummary`              | Understanding or extending ECO report structure                         |
| `src/diagnostic.rs` | `Span`, `Spanned<T>`, `BinOp`, `Unit`; source location types                   | Working with parser/compiler error locations                            |
| `src/import.rs`   | `resolve_imports()`, `ResolvedSpec`; handles `import` directives in spec files    | Adding library import support or debugging import resolution            |
| `src/trivia.rs`   | `TriviaMap`, `parse_with_trivia()`, `CommentToken`; comment preservation          | Working with comment-preserving parse/rewrite flows                     |
| `src/lib.rs`      | Public API re-exports for all modules                                             | Finding the public surface of the crate                                 |

# altium-format-spec

Spec DSL crate: parse, compile, execute, reconcile, dump, and sync Altium spec files.

## Index

| File              | Contents (WHAT)                                                                   | Read When (WHEN)                                                        |
| ----------------- | --------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `README.md`       | Architecture, design decisions, invariants, sync IR rationale                     | Understanding overall spec pipeline or sync system design               |
| `src/lexer.rs`    | Tokenizer; `TokenKind` including `Hash` for `#` annotation prefix and `Arrow` (`->`) for pin connections | Debugging parse errors, adding new syntax tokens                        |
| `src/parser.rs`   | Recursive-descent parser; `parse_annotation()`, all `parse_*_decl()` methods, `parse_pin_connection_decl()` | Adding new spec syntax, debugging parser errors                         |
| `src/ast.rs`      | AST node types; `BlockAnnotation`, `AnnotationKey`, all `*Decl` structs; `PinConnectionDecl`, `PinConnectionTarget` | Modifying the spec grammar or AST structure                 |
| `src/compiler.rs` | AST → SpecModel lowering; unit resolution, layer resolution, `compile_annotation()` with `seen_ids`; power pre-pass for signal/power classification; `SymbolRef::Import` validation against imported SchLib | Changing compilation behavior, adding new model fields   |
| `src/model.rs`    | `SpecModel` and all typed spec structs; `annotation: Option<CompiledAnnotation>` on 15+ types; `PinConnectionSpec`, `PinConnectionTarget`, `power_declarations` on `SheetSpec` | Reading or writing spec model fields                         |
| `src/annotation.rs` | `CompiledAnnotation`, `generate_short_id()`, `validate_short_id()`, `compile_annotation()` | Working with annotation IDs, understanding ID format      |
| `src/sync.rs`     | `SyncSnapshot` IR, `project_schdoc_spec()`, `project_pcbdoc_spec()`, `diff_snapshots()`, `filter_changes()`, `apply_sync_changes_to_pcbdoc()`, ECO report renderer, spec text rewriter | Implementing or debugging spec-to-spec sync         |
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

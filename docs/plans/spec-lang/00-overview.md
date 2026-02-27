# 00 - Architecture Overview

## Goal

Implement the Altium Spec Language (spec-lang.md v0.3): a declarative DSL for
describing the desired state of SchLib/PcbLib files. The spec compiler reads a
`.schlib-spec` or `.pcblib-spec` file, diffs it against an existing document (or
an empty one), produces an Engineering Change Order (ECO), and optionally applies
it.

## Execution Pipeline

```
.schlib-spec / .pcblib-spec file
    |
    v
[1. Lexer]           crates/altium-format-ops/src/spec/lexer.rs
    |                 (extends existing parser/lexer.rs token set)
    v
[2. Parser]          crates/altium-format-ops/src/spec/parser.rs
    |                 (recursive-descent, produces SpecAst)
    v
[3. Import Resolver] crates/altium-format-ops/src/spec/import.rs
    |                 (topological sort, namespace binding)
    v
[4. Compiler]        crates/altium-format-ops/src/spec/compiler.rs
    |                 (anchor resolution, layout expansion, scope checking)
    |                 (produces SpecModel with absolute coordinates)
    v
[5. Reconciler]      crates/altium-format-ops/src/spec/reconciler.rs
    |                 (loads document, diffs against SpecModel)
    |                 (produces EngineeringChangeOrder)
    v
[6. ECO Output]      crates/altium-format-ops/src/spec/eco.rs
    |                 (text report / JSON for `plan` command)
    v
[7. Executor]        crates/altium-format-ops/src/spec/executor.rs
    |                 (converts EntityChange -> HighOp -> LowOp)
    |                 (uses existing lowering pipeline)
    v
[8. Document]        Mutated SchLib / PcbLib (via altium-format)
```

## Key Design Decisions

### D1: New parser, reuse lexer token types

The existing `parser/lexer.rs` already tokenizes all needed primitives:
identifiers, `$`-prefixed refs, strings, template strings, integers, floats,
dimensions, colors, `...` spread, and all needed punctuation. The spec language
needs additional **keywords** (`component`, `footprint`, `pin`, `pad`, `part`,
`parameter`, `alias`, `map`, `row`, `column`, `grid`, `import`, `as`) but the
token types are identical.

**Decision**: Create `spec/lexer.rs` that wraps the existing lexer with
spec-specific keyword classification. The core tokenization logic is shared
via a common `TokenKind` enum (or the spec lexer re-lexes from scratch using
the same patterns). This avoids touching the ops parser while sharing token
definitions.

Actually, the better path: **fork and specialize**. The spec lexer is a clean
rewrite in `spec/lexer.rs` using the same `TokenKind` enum from `parser/lexer.rs`
but with spec-specific keyword recognition. The ops lexer keywords (`assert`) are
not needed; the spec keywords (`component`, `pin`, etc.) are not needed in ops.
Both lexers share the same `Unit` enum and literal parsing logic.

### D2: Separate AST, not extension of ops AST

The ops AST (`parser/ast.rs`) is structured around imperative operations:
`Statement::Op { name, target, selector, body }`. The spec AST is fundamentally
different: declarative entity trees with natural keys, anchor references, and
layout blocks.

**Decision**: Create `spec/ast.rs` with a spec-specific AST. The expression
types (`Expr`, `Object`, `ObjectItem`) can be shared or duplicated — they are
small and the spec may need slightly different semantics (e.g., entity-name
positions that are not expressions).

### D3: SpecModel as typed intermediate representation

The parser produces an untyped AST. Before reconciliation, we need a fully
resolved model with:
- All anchor references resolved to absolute coordinates
- All `row`/`column`/`grid` blocks expanded to individual pads
- All let bindings and spreads evaluated
- All types checked and coerced
- All import namespaces resolved

This is the **SpecModel** — a typed, flattened representation that maps directly
to document entities.

**Decision**: The SpecModel is a new set of types in `spec/model.rs`. It is NOT
the same as the ops `HighOp` types — those are mutation commands, while SpecModel
describes desired state. The reconciler diffs SpecModel against the document to
produce `EntityChange` entries.

### D4: Reconciler produces ECO, not HighOps directly

The spec-lang.md defines an ECO (Engineering Change Order) as the output of
reconciliation. This is a structured diff with `Add`, `Update`, and `Unchanged`
entries — not a flat list of mutations.

**Decision**: The reconciler produces `EngineeringChangeOrder` containing
`Vec<EntityChange>`. The executor then converts `EntityChange::Add` and
`EntityChange::Update` entries into `HighOp` sequences that flow through the
existing lowering pipeline.

### D5: Reuse existing lowering and apply infrastructure

Once the executor produces `Vec<HighOp>`, the existing pipeline handles
everything: `HighOp -> ComposedOp -> SchLibLowOp/PcbLibLowOp -> apply`.

**Decision**: The spec executor is a thin adapter that maps ECO entries to the
existing `HighOp` types (primarily `AddComponent`, `AddPin`, `AddParameter`,
`AddAlias`, `AddFootprint`, graphics ops, and the edit ops once implemented).

### D6: Edit ops are needed but don't block initial implementation

The reconciler needs Edit ops for updates (spec-lang.md §14.2 marks them as
"Needed"). For the initial implementation, the reconciler can use
**delete + re-add** for entities where only Add exists. This is semantically
correct for library files (where entities have stable identity keys) though
less efficient.

**Decision**: Implement Add-only reconciliation first. Add Edit ops
incrementally. The reconciler should be structured to emit Edit when available,
falling back to delete+re-add.

## Crate Placement

All spec-language code goes in `crates/altium-format-ops/src/spec/`:

```
crates/altium-format-ops/src/spec/
    mod.rs              // Module root, public API
    lexer.rs            // Spec tokenizer
    ast.rs              // Spec AST types
    parser.rs           // Recursive-descent parser
    import.rs           // Import resolution
    compiler.rs         // AST -> SpecModel (type check, anchor resolve, layout expand)
    model.rs            // SpecModel types (typed IR)
    reconciler.rs       // SpecModel + Document -> ECO
    eco.rs              // ECO types and rendering
    executor.rs         // ECO -> HighOp adapter
    dump.rs             // Document -> spec file (reverse generation)
```

The `altium-format` crate is NOT modified (except to add missing Edit low-ops
in future milestones). All spec logic lives in `altium-format-ops`.

CLI commands (`plan`, `apply`, `dump`) are added to `crates/altium-cli/src/main.rs`.

## Dependencies

No new crate dependencies needed. The existing `altium-format-ops` dependencies
cover everything: `indexmap` for ordered maps, `thiserror` for errors, `serde` +
`serde_json` for JSON ECO output.

## Relationship to Existing Ops System

The ops system (`parser/` + `ops/`) is the **imperative** interface: "add this
component", "edit this pin", "query these records". It continues to exist and
work independently.

The spec system (`spec/`) is the **declarative** interface: "the document should
contain this component with these pins". Internally, it produces ops to make the
document match the spec.

Both systems share:
- Domain types from `altium-format-types` (`Coord`, `Color`, enums)
- `HighOp` type (executor output feeds into lowering pipeline)
- Low-level ops (`SchLibLowOp`, `PcbLibLowOp`) and their apply functions
- `ApplyReport` for tracking results

They do NOT share:
- Lexer/parser (different grammars)
- AST (different structures)
- Compilation (ops: typecheck to HighOp; spec: compile to SpecModel, reconcile, then HighOp)

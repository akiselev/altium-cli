# Spec Language Implementation Plan

Implementation plan for `docs/spec-lang.md` (v0.3).

## Document Index

| Document | Description |
|----------|-------------|
| [00-overview.md](00-overview.md) | Architecture overview, design decisions, codebase integration |
| [01-lexer.md](01-lexer.md) | Lexer: extending the existing tokenizer for spec syntax |
| [02-ast.md](02-ast.md) | AST: spec-specific node types |
| [03-parser.md](03-parser.md) | Parser: recursive-descent spec file parsing |
| [04-import-resolver.md](04-import-resolver.md) | Import system: file resolution, namespaces, cycle detection |
| [05-anchor-placement.md](05-anchor-placement.md) | Anchor-based placement: coordinate computation for pins/pads |
| [06-layout-expansion.md](06-layout-expansion.md) | Row/column/grid expansion into individual pads |
| [07-spec-model.md](07-spec-model.md) | SpecModel: typed intermediate representation |
| [08-reconciler.md](08-reconciler.md) | Reconciler: diff spec against document, produce ECO |
| [09-executor.md](09-executor.md) | Executor: ECO to low-level ops |
| [10-eco-output.md](10-eco-output.md) | ECO output: text and JSON formats |
| [11-dump.md](11-dump.md) | Reverse generation: document to spec file |
| [12-cli.md](12-cli.md) | CLI commands: plan, apply, dump |
| [13-missing-ops.md](13-missing-ops.md) | Missing low-level ops that must be implemented |
| [14-testing.md](14-testing.md) | Testing strategy |
| [15-milestones.md](15-milestones.md) | Implementation milestones and ordering |

## Quick Start

Read [00-overview.md](00-overview.md) first for the big picture, then
[15-milestones.md](15-milestones.md) for the implementation order.

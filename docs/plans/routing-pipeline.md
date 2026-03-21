# Routing Pipeline Integration

## Overview

Integrate the autorouter into the spec-driven pipeline. Routes are a sidecar artifact
(`.routes` file) that gets loaded during spec compilation and merged into the IR as
free copper. The `apply` command writes everything (placement + routes) to PcbDoc in
one step.

## Architecture

```
pcbdoc-spec (source of truth)
  + .routes sidecar (routing solution)
  + .PcbDoc (import source)
       ↓
  load_ir_from_spec() → PcbIr with routed copper in free_copper
       ↓
  apply → output .PcbDoc (placement + tracks + vias)
```

## Spec Syntax

```
routing {
    solution: "hub.routes"           // explicit path to .routes file
    grid_resolution: 0.1mm           // routing config overrides
    max_iterations: 50
}
```

Convention: if `solution` is omitted, look for `<spec_stem>.routes` next to the spec file.

## Implementation Steps

### Step 1: RoutingSpec model + PcbDocSpec field
- model.rs: Add `RoutingSpec { solution, config }`
- model.rs: Add `routing: Option<RoutingSpec>` to `PcbDocSpec`
- pcbdoc_import.rs: Add `routing: None` in import, carry through merge

### Step 2: AST + parser for `routing { }` block
- ast.rs: Add `RoutingDecl`, `SpecItem::Routing`
- parser.rs: Parse `routing { ... }` top-level block
- compiler.rs: Compile routing block to RoutingSpec

### Step 3: Routes loading in spec_bridge
- autopcb-ir/Cargo.toml: Add autopcb-routes dependency
- spec_bridge.rs: After spec_to_ir(), load .routes and merge into PcbIr.free_copper
- Convert TraceSegment → IrTrack, RoutedVia → IrVia

### Step 4: `routing solve` CLI command
- main.rs: Wire spec → IR → build_workspace() → route_board() → save .routes
- Print routing report

### Step 5: Routes in apply path
- CLI loads .routes, converts to PcbDocPrimitiveSpec tracks/vias
- Injects into spec before apply_spec_pcbdoc()
- Reuses existing primitive_to_track/primitive_to_via converters

### Step 6: Dump/format support
- dump.rs: Emit `routing { }` block
- formatter.rs: Handle SpecItem::Routing

## ID Space

The .routes file uses the same NetId/LayerId index space as the IR that generated it.
If the spec changes (nets added/removed), the .routes file becomes stale and must be
re-generated via `routing solve`.

## Convention-based Discovery

1. If `routing { solution: "path" }` is set, use that (relative to spec dir)
2. Otherwise, look for `<spec_stem>.routes` next to the spec file
3. If neither exists, no routes are loaded (empty free copper)

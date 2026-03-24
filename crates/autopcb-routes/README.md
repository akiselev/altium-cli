# autopcb-routes

Thin format crate for PCB route solution serialization. Defines the types written
by `autopcb-router` and read by the `autopcb-spec` import resolver.

## Why This Crate Exists

Both `autopcb-router` (producer) and `autopcb-spec` (consumer) need the route
solution types. Placing them in either crate creates a circular dependency. This crate
breaks the cycle by holding only serde data types with no algorithmic logic.

```
autopcb-router ──writes──┐
                         ├── autopcb-routes (types only)
autopcb-spec   ──reads───┘
```

## Types

| Type | Purpose |
|------|---------|
| `RouteSolution` | Top-level container: version, nets, unrouted nets, metrics, iteration snapshots |
| `RoutedNet` | Segments and vias for one net, plus total routed length |
| `TraceSegment` | Single trace: net, layer, start/end (mm), width (mm) |
| `RoutedVia` | Via: net, position (mm), from/to layer, drill (mm), annular ring (mm) |
| `RoutingMetrics` | Board-level summary: total length, via count, completion %, DRC violations |
| `RoutingIterationSnapshot` | PathFinder state after one iteration: conflict count, per-net path snapshot |
| `LayerId(u16)` | Domain newtype for layer identity. Independent of `autopcb-ir::LayerId` |
| `NetId(u32)` | Domain newtype for net identity. Independent of `autopcb-ir::NetId` |
| `RoutesError` | Error type: `UnsupportedVersion`, `Io`, `BincodeDecode`, `Json` |

## Coordinate Convention

All positions and dimensions are in millimetres (`f64`). The spec compiler's import
resolver converts mm to Altium internal units using `Coord::from_mm()` from
`altium-format-types` at the apply boundary. The router never touches Altium
coordinate space.

## Serialization

Two formats are supported via free functions:

| Function | Format | Use case |
|----------|--------|----------|
| `save_binary(solution, path)` | bincode | Production: compact, fast load |
| `load_binary(path)` | bincode | Production load |
| `save_json(solution, path)` | serde_json (pretty) | Debugging: human-readable, diffable |
| `load_json(path)` | serde_json | Debug load |

Both loaders check `solution.version` against `CURRENT_VERSION` (currently `1`) and
return `RoutesError::UnsupportedVersion` if the file was written by a newer version.

## Determinism Invariant

`RouteSolution::nets` and `RoutingIterationSnapshot::paths` use `BTreeMap<NetId, _>`,
not `HashMap`. This guarantees that bincode serialization produces byte-identical output
for identical inputs across runs and platforms. All other collection types are `Vec`,
which preserves insertion order.

## Usage

```rust
use autopcb_routes::{RouteSolution, save_binary, load_binary, save_json, load_json};
use std::path::Path;

// Create a solution (normally produced by autopcb-router)
let solution = RouteSolution::new();

// Write binary (production)
save_binary(&solution, Path::new("board.routes")).unwrap();

// Read binary
let loaded = load_binary(Path::new("board.routes")).unwrap();

// Write JSON (debugging)
save_json(&solution, Path::new("board.routes.json")).unwrap();

// Read JSON
let from_json = load_json(Path::new("board.routes.json")).unwrap();
```

## Version Compatibility

The `version` field in `RouteSolution` is checked on load. Current version is `1`.
Newer versions (version > CURRENT_VERSION) are rejected with `UnsupportedVersion`.
Older versions (version < CURRENT_VERSION) are accepted — all serde fields use
`#[serde(default)]` for forward-compatible deserialization.

## Dependencies

| Crate | Role |
|-------|------|
| `serde` | Derive `Serialize`/`Deserialize` on all types |
| `bincode` | Binary serialization |
| `serde_json` | JSON serialization |
| `thiserror` | `RoutesError` derive |

No dependency on `autopcb-ir`, `autopcb-spec`, or any Altium crate. This is intentional — consumers
of route files must not transitively depend on IR extraction logic.

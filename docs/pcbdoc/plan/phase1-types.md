# Phase 1: PcbDoc API Types

## Goal

Define all public API types in `crates/altium-format/src/api/pcbdoc_types.rs`.
No conversion logic yet — just the type definitions.

## File: `api/pcbdoc_types.rs`

### Types to Define

**Root type:**
- `PcbDocBoard` — top-level board container with all collections

**Board metadata:**
- `BoardSettings` — curated subset of Board6 configuration

**Named collections (identity by natural key):**
- `Net` — net definition (id = net name)
- `PcbDocComponent` — placed component instance (id = designator)
- `Polygon` — copper pour polygon (id = block-level name)
- `NetClass` — net/component class (id = class name)
- `DesignRule` — DRC rule (id = rule name)
- `DifferentialPair` — diff pair definition
- `Dimension` — dimension annotation
- `Model3D` — 3D model reference

**Primitives (identity by `id` field):**
- `Track` — routed trace segment
- `Arc` — arc primitive
- `Via` — via hole
- `Pad` — component pad (also has `pad_name` natural key within component)
- `Fill` — rectangular fill
- `Text` — text string
- `Region` — polygonal region (board outline, keepout, copper shape)
- `ComponentBody` — 3D body outline

### Common Patterns

All types:
- `id: String` field as first field
- `#[derive(Debug, Clone)]`

Primitive types additionally:
- `layer: LayerRef`
- `net: Option<String>` — resolved net name
- `component: Option<String>` — resolved component designator

### Dependencies

Uses types from `altium_format_types`:
- `Coord`, `CoordPoint` — coordinates
- `LayerRef`, `V6Layer` — layers
- `Color` — Win32 COLORREF
- `PcbFlags` — primitive flags
- `PadShape`, `PadStackMode` — pad properties
- `PlaneConnectionStyle` — thermal relief
- `RegionKind` — region subtypes
- `RuleKind` — DRC rule types
- `TextKind` — stroke/truetype/barcode

### Wire-up in `api/mod.rs`

Add to `api/mod.rs`:
```rust
mod pcbdoc_types;
pub(crate) mod pcbdoc_read;   // placeholder, Phase 2
pub(crate) mod pcbdoc_write;  // placeholder, Phase 3

pub use pcbdoc_types::{ ... };
```

### Estimated Scope

- ~300 lines of type definitions
- No logic, pure struct/enum definitions
- Test: compiles cleanly, no warnings

### Validation

- `cargo check -p altium-format`
- Types match the design in `../high-level-api.md`

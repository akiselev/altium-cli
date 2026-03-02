# Phase 4: CLI Integration

## Goal

Wire the PcbDoc high-level API into the CLI commands: `info`, `query`, `dump`.

## Prerequisites

Phase 2 (read path) must be complete. Phase 3 (write) needed for `plan`/`apply`.

## 4a: `altium info` for PcbDoc

Currently `info` works for SchLib, PcbLib, SchDoc. Add PcbDoc support.

**Output format** (similar to existing info output):
```
File: board.PcbDoc
Type: PcbDoc
Version: PCB 6.0 Binary File
Nets: 127
Components: 45
Tracks: 1,234
Vias: 89
Pads: 360
Arcs: 12
Fills: 3
Texts: 67
Regions: 15
Rules: 23
Classes: 8
```

**Implementation:**
- In `altium-cli/src/main.rs`, add PcbDoc match arm in info command
- Call `doc.board()?`, extract counts
- Print formatted summary

## 4b: `altium query` for PcbDoc

The query engine (altium-format-query) needs a PcbDoc entity adapter.

**Entity types to register:**
- `track`, `arc`, `via`, `pad`, `fill`, `text`, `region`, `component_body`
- `net`, `pcbdoc_component`, `polygon`, `rule`, `class`

**Attribute accessors per entity:**
- Primitives: `id`, `layer`, `net`, `component`, `width`, `start`, `end`, etc.
- Nets: `name`, `color`, `visible`
- Components: `designator`, `pattern`, `location`, `layer`, `rotation`

**Pseudo-classes:**
- `:smd` — pads that are SMD (no hole)
- `:through_hole` — pads with holes
- `:top` — objects on top layer
- `:bottom` — objects on bottom layer

**Combinators:**
- `component > pad` — pads belonging to a component
- `net > track` — tracks in a net

**Implementation:**
- New file: `crates/altium-format-query/src/pcbdoc.rs`
- Implement `EntityAdapter` for PcbDocBoard
- Register entity types, attributes, pseudo-classes

## 4c: `altium dump` for PcbDoc

Generate `.pcbdoc-spec` source from an existing PcbDoc file.

**Implementation:**
- New function in `altium-format-spec/src/dump.rs`: `dump_pcbdoc()`
- Iterate board collections and primitives
- Generate spec syntax with auto-generated IDs as block-level names
- Use `{type}_{index}` format for primitive names

**Output format:**
```
board "MyBoard" {
    signal_layer_count: 4
    snap_grid: 25mil
}

net GND { }
net VCC3P3 { }

component U1 {
    lib_reference: "QFP-48"
    at: (1000mil, 1000mil)
    layer: top
}

track track_0 {
    start: (100mil, 200mil)
    end: (300mil, 200mil)
    width: 10mil
    layer: top
    net: VCC3P3
}
```

## Estimated Scope

- 4a (info): ~50 lines
- 4b (query): ~300-500 lines (entity adapter is boilerplate-heavy)
- 4c (dump): ~200-400 lines

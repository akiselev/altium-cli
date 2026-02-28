# 11 - Reverse Generation (Dump)

## Location

`crates/altium-format-ops/src/spec/dump.rs`

## Purpose

Read an existing `.SchLib` or `.PcbLib` file and generate a corresponding
`.schlib-spec` or `.pcblib-spec` file. This enables bootstrapping spec-based
management of existing libraries.

## Public API

```rust
/// Generate spec source from a SchLib document.
pub fn dump_schlib(lib: &SchLib) -> String

/// Generate spec source from a PcbLib document.
pub fn dump_pcblib(lib: &PcbLib) -> String
```

## Generated Output Properties

Per spec-lang.md §13:

1. **Absolute placement only**: Dump generates pins/pads with `at: (x, y)` and
   explicit `orientation`. Anchor inference (reverse-computing which rectangle
   a pin is relative to) is explicitly out of scope.

2. **No row/column/grid blocks**: Even if the original footprint has regular
   pad patterns, dump generates individual pad declarations. Pattern detection
   is a future enhancement.

3. **All properties explicit**: Every non-default field is emitted. No spread
   or template usage.

4. **Stable binding names**: Graphics with unique_ids that match the
   `spec:{context}:{name}` pattern get their binding name extracted. Other
   graphics are unnamed (auto-generated unique_ids are not stable).

5. **Roundtrip correctness**: The generated spec, when applied to an empty
   document, recreates the original library (modulo serialization ordering
   and default values).

## SchLib Dump

```rust
fn dump_schlib(lib: &SchLib) -> String {
    let mut out = String::new();

    for component in &lib.components {
        dump_component(&mut out, component);
        out.push('\n');
    }

    out
}

fn dump_component(out: &mut String, comp: &SchComponent) {
    writeln!(out, "component {} {{", quote_entity_name(&comp.lib_reference));
    writeln!(out, "    designator: {}", quote_string(&comp.designator));
    if !comp.description.is_empty() {
        writeln!(out, "    description: {}", quote_string(&comp.description));
    }
    // ... other component properties

    // Group by part
    let shared_pins: Vec<_> = comp.pins.iter()
        .filter(|p| p.owner_part_id == 0)
        .collect();
    let part_groups = group_by_part(&comp.pins, &comp.graphics);

    // Dump per-part blocks
    for (part_num, pins, graphics) in &part_groups {
        writeln!(out, "    part {} {{", part_num);
        for graphic in graphics {
            dump_graphic(out, graphic, 8);
        }
        for pin in pins {
            dump_pin(out, pin, 8);
        }
        writeln!(out, "    }}");
    }

    // Dump shared pins
    for pin in &shared_pins {
        dump_pin(out, pin, 4);
    }

    // Parameters
    for param in &comp.parameters {
        dump_parameter(out, param, 4);
    }

    // Aliases
    for alias in &comp.aliases {
        writeln!(out, "    alias {}", quote_entity_name(alias));
    }

    // Footprint maps
    for impl_ in &comp.implementations {
        dump_footprint_map(out, impl_, 4);
    }

    writeln!(out, "}}");
}
```

## Pin Dump

```rust
fn dump_pin(out: &mut String, pin: &SchPin, indent: usize) {
    let pad = " ".repeat(indent);
    write!(out, "{}pin {} {{ ", pad, quote_entity_name(&pin.designator));
    write!(out, "at: ({}, {}), ", format_coord(pin.location.x), format_coord(pin.location.y));
    write!(out, "orientation: {}, ", pin.orientation.degrees());
    write!(out, "electrical: {}", format_electrical(pin.electrical));
    if pin.length != Coord::from_mils(25) {
        write!(out, ", length: {}", format_coord(pin.length));
    }
    if !pin.name.is_empty() {
        write!(out, ", name: {}", quote_string(&pin.name));
    }
    if pin.is_hidden {
        write!(out, ", is_hidden: true");
    }
    if !pin.hidden_net_name.is_empty() {
        write!(out, ", hidden_net_name: {}", quote_string(&pin.hidden_net_name));
    }
    writeln!(out, " }}");
}
```

## PcbLib Dump

```rust
fn dump_pcblib(lib: &PcbLib) -> String {
    let mut out = String::new();

    for fp in &lib.footprints {
        dump_footprint(&mut out, fp);
        out.push('\n');
    }

    out
}

fn dump_footprint(out: &mut String, fp: &PcbFootprint) {
    writeln!(out, "footprint {} {{", quote_entity_name(&fp.display_name));
    if !fp.description.is_empty() {
        writeln!(out, "    description: {}", quote_string(&fp.description));
    }
    if fp.height != Coord::ZERO {
        writeln!(out, "    height: {}", format_coord(fp.height));
    }

    // Pads first
    for prim in &fp.primitives {
        if let PcbPrimitive::Pad(pad) = prim {
            dump_pad(out, pad, 4);
        }
    }

    // Then other primitives
    for prim in &fp.primitives {
        match prim {
            PcbPrimitive::Track(t) => dump_track(out, t, 4),
            PcbPrimitive::Arc(a) => dump_arc(out, a, 4),
            // ... etc
            PcbPrimitive::Pad(_) => {} // already dumped
        }
    }

    writeln!(out, "}}");
}
```

## Formatting Helpers

```rust
/// Format a coordinate in the most natural unit.
fn format_coord(c: Coord) -> String {
    let mils = c.to_mils_f64();
    let mm = c.to_mm_f64();

    // Prefer mm if it's a clean value
    if (mm * 1000.0).round() == mm * 1000.0 && mm.abs() >= 0.001 {
        format!("{}mm", format_float(mm))
    } else {
        format!("{}mil", format_float(mils))
    }
}

/// Format a float, removing trailing zeros.
fn format_float(v: f64) -> String {
    let s = format!("{:.4}", v);
    let s = s.trim_end_matches('0');
    let s = s.trim_end_matches('.');
    s.to_string()
}

/// Quote an entity name if it contains special characters.
fn quote_entity_name(name: &str) -> String {
    if is_valid_ident(name) || name.parse::<i32>().is_ok() {
        name.to_string()
    } else {
        format!("\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

/// Quote a string value.
fn quote_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}
```

## Future Enhancements

Not in initial scope but noted for future:

- **Template extraction** (`--extract-templates`): Detect repeated property
  patterns across pins/pads and generate let bindings.
- **Row/grid detection**: Detect regular pad patterns and generate row/grid
  blocks instead of individual pads.
- **Anchor inference**: Detect pins on rectangle edges and generate anchor-based
  placement.

## Test Strategy

- Dump a simple SchLib, parse the output, verify AST structure
- Dump a PcbLib with various pad types
- Roundtrip: dump -> parse -> compile -> reconcile against original -> all Unchanged
- Multi-part component dump (verify part blocks)
- Entity name quoting (spaces, special chars)
- Coordinate formatting (mils vs mm)
- Hidden pin properties
- Footprint map dump

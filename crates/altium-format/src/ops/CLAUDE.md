# ops/ - High-level Operations

Business logic layer for Altium file operations. Separates logic from CLI parsing and file I/O.

## Module Index

| Module | Purpose | Functions |
|--------|---------|-----------|
| `schlib.rs` | Schematic library operations | `cmd_overview`, `cmd_list`, `cmd_search`, `cmd_info`, `cmd_component`, `cmd_pins`, `cmd_primitives`, `cmd_create`, `cmd_add_component`, `cmd_add_pin`, `cmd_json` |
| `schdoc.rs` | Schematic document operations | `cmd_overview`, `cmd_info`, `cmd_components`, `cmd_netlist`, `cmd_wires`, `cmd_ports`, `cmd_power_map`, `cmd_json` |
| `pcblib.rs` | PCB library operations | `cmd_overview`, `cmd_list`, `cmd_search`, `cmd_info`, `cmd_footprint`, `cmd_pads`, `cmd_primitives`, `cmd_holes`, `cmd_measure`, `cmd_create`, `cmd_add_*`, `cmd_gen_chip`, `cmd_render_*`, `cmd_json` |
| `pcbdoc.rs` | PCB document operations | `cmd_overview`, `cmd_info`, `cmd_rules`, `cmd_components`, `cmd_tracks`, `cmd_vias`, `cmd_polygons`, `cmd_nets`, `cmd_create`, `cmd_add_*`, `cmd_json` |
| `prjpcb.rs` | Project file operations | `cmd_overview`, `cmd_info`, `cmd_documents`, `cmd_bom`, `cmd_validate`, `cmd_json` |
| `intlib.rs` | Integrated library operations | `cmd_overview`, `cmd_list`, `cmd_search`, `cmd_component`, `cmd_info`, `cmd_symbols`, `cmd_footprints`, `cmd_parameters`, `cmd_extract_schlib`, `cmd_extract_pcblib`, `cmd_json` |
| `output.rs` | Output data structures | Serializable types for all cmd_* return values |
| `categorization.rs` | Component categorization | `categorize_component` shared logic |

## Pattern

All `cmd_*` functions follow the same signature pattern:

```rust
pub fn cmd_<name>(path: &Path, ...) -> Result<OutputType, Box<dyn std::error::Error>>
```

Output types are defined in `output.rs` and implement `Serialize` for JSON output.

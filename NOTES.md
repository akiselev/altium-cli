# Scenario Test Notes

Ran all 12 scenarios from `docs/scenarios/`. Results below.

## Summary Table

| # | Scenario | Pass | Fail | Total | Notes |
|---|----------|------|------|-------|-------|
| 01 | Single Resistor | 48 | 4 | 52 | Best scenario - designators work, routing works |
| 02 | LED + Resistor | 35 | 12 | 48 | Designator persistence bug blocks routing |
| 03 | MCU Power | ~30 | ~20 | ~50 | gen-ic works, designator bug blocks routing |
| 04 | Voltage Divider | 28 | 18 | 46 | Same designator + gen-chip syntax issues |
| 05 | I2C Temp Sensor | 45 | 21 | 70 | Designator bug, default component_count off-by-1 |
| 06 | Op-Amp Buffer | 42 | 16 | 60 | Designators work but routing still fails ("No route found") |
| 07 | MOSFET LED Driver | 29 | 26 | 55 | Designator bug, add-dual-row needs --pad-width |
| 08 | RS-485 Transceiver | 32 | 36 | 68 | Designator bug, add-dual-row needs --pad-width |
| 09 | LDO Regulator | 33 | 27 | 60 | Designator bug, add-dual-row needs --pad-width |
| 10 | SPI Flash | 51 | 23 | 74 | Designator bug blocks all routing |
| 11 | Dual Op-Amp | 55 | 24 | 79 | Designator bug blocks all routing |
| 12 | Current Sensor | 56 | 27 | 83 | gen-chip missing 2512 size, designator bug |

## Critical Bugs

### BUG 1: Designator text not persisted in SchDoc save (CRITICAL)

**Location:** `crates/altium-format/src/edit/library.rs` (transform_primitive / instantiate_component)

**Symptom:** After `edit add-component ... R1`, the saved SchDoc contains empty designator text. `schdoc components` shows designators as `<none>`. Route commands fail with `Component not found: 'R1'. Available: ["", "", ""]`.

**Impact:** Blocks ALL routing commands across scenarios 02-12. This is the single biggest blocker - ~150+ route command failures trace back to this bug.

**Inconsistency:** Scenario 01 and 06 sometimes show designators correctly while others don't. May relate to how gen-ic vs manually-created components store Designator records, or timing of save/reload cycles.

**Reproduction:**
```bash
altium-cli schlib create lib.SchLib
altium-cli schlib add-component lib.SchLib RES --description "Resistor"
altium-cli schlib add-pin lib.SchLib RES 1 "1" --orientation right --electrical passive -- -50 0
altium-cli schlib add-pin lib.SchLib RES 2 "2" --orientation left --electrical passive -- 50 0
altium-cli schdoc create design.SchDoc
altium-cli edit design.SchDoc -c "add-component lib.SchLib RES 1000 1000 R1"
altium-cli schdoc components design.SchDoc --json
# designator shows as "<none>" instead of "R1"
```

### BUG 2: Routing engine pin location resolution

**Location:** `crates/altium-format/src/edit/layout.rs` (get_pin_locations)

**Symptom:** Even when designators work (scenario 06), all route commands fail with `"No route found"`. Pin absolute locations resolve to (0, 0) instead of their true schematic positions.

**Impact:** Blocks routing in all scenarios. Even scenario 01 only succeeds because it uses net labels at the right positions rather than relying on pin-to-pin routing.

### BUG 3: import-to-pcb targets wrong PCB file (MEDIUM)

**Location:** `crates/altium-cli/src/commands/prjpcb.rs` (import-to-pcb)

**Symptom:** Uses default `PCB1.PcbDoc` from the project template instead of the user-added `board.PcbDoc`. Fails with `Os { code: 2, kind: NotFound }`.

**Workaround:** Use `--pcb board.PcbDoc` flag (discovered in help but not used in scenarios).

**Impact:** Every scenario's import-to-pcb step fails.

### BUG 4: PcbDoc save only writes rules, not nets/components

**Location:** `crates/altium-format/src/io/pcbdoc.rs:513` (save_to_file)

**Symptom:** `save_to_file` only calls `write_rules`. Component placement code has a comment `// Would add component here - for now just count`. Nets pushed in-memory are lost on save.

**Impact:** Even if import-to-pcb ran correctly, the PCB would not persist the imported data.

### BUG 5: Routing netlist assertion fails (scenario 01)

**Symptom:** `schdoc netlist --json` shows `total_nets: 0` even after successful wire routing in scenario 01. The wires exist (verified by `schdoc wires`) but the netlist extraction doesn't find connections.

## Non-Critical Issues

### Default component/footprint created on library creation

`schlib create` adds a default `Component_1` and `pcblib create` adds a default `PCBComponent_1`. This inflates all component/footprint counts by 1 vs what scenarios expect.

### gen-chip auto-naming

`pcblib gen-chip 0402` creates a footprint named `CHIP_0402`, not the scenario-specified `R0402`/`P0402`/etc. There's no way to specify a custom name.

### gen-chip missing 2512 size

Only supports: 0201, 0402, 0603, 0805, 1206. Scenario 12 needs 2512 for a shunt resistor.

### add-dual-row requires --pad-width for SMD

Scenarios 07-09 fail on `pcblib add-dual-row` because `--pad-width` and `--pad-height` are required for SMD pads but not specified in the scenarios.

### add-dual-row only supports symmetric pad counts

SOT-23-5 (scenario 06) needs 3+2 pads but `add-dual-row` only creates symmetric layouts (3+3).

### pcbdoc add-rule fails on existing default rules

`pcbdoc create` generates 35 default rules including `Clearance` and `Width`. Scenarios that try `add-rule Clearance` fail with "already exists". Should use `modify-rule` instead.

### Rule kind is case-sensitive

`add-rule` rejects `clearance` and `width` (lowercase) — requires `Clearance` and `Width`.

## Scenario Command Syntax Mismatches

The scenario files use CLI syntax that doesn't match the actual implementation:

| Scenario Syntax | Actual CLI Syntax |
|----------------|-------------------|
| `schlib add-pin --direction right` | `schlib add-pin --orientation right` |
| `pcblib gen-chip NAME --size 0402` | `pcblib gen-chip PATH 0402` (auto-names CHIP_0402) |
| `schlib gen-ic NAME --pins "A,B,C"` | `schlib gen-ic PATH NAME PINS` (positional, format: `desig:name:type:side,...`) |
| `pcbdoc set-settings --grid-size 25` | `pcbdoc set-settings --snap-grid 25` |
| `pcbdoc add-rule "Clearance" --value 8` | `pcbdoc add-rule PATH Clearance NAME --gap 8` |
| `schlib dump` / `pcblib dump` | `schlib json` / `pcblib json` (no `dump` subcommand) |
| Negative coordinates: `-50 0` | Need `--` separator: `-- -50 0` |

## Output Files

Each scenario directory in `scenarios/NN-name/` contains:
- `run.log` — Full command-by-command execution log with PASS/FAIL for each step
- `parts-schlib-dump.json` — SchLib JSON dump
- `fps-pcblib-dump.json` — PcbLib JSON dump
- `design-schdoc-dump.json` — SchDoc JSON dump
- `board-pcbdoc-dump.json` — PcbDoc JSON dump (after setup, before import)
- `board-final-dump.json` — PcbDoc JSON dump (after import attempt)

Scenarios 07-09 also have `summary.md` files with detailed step tables.

Work files (the actual Altium files) are in `work/scenario-NN/`.

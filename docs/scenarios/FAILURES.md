# Scenario Test Failures

Test run: 2026-02-08 | Binary: `target/release/altium-cli` | Scenarios 01-06

## Summary

| Scenario | Pass | Fail | Total |
|----------|------|------|-------|
| 01 - Single Resistor | 11 | 4 | 15 |
| 02 - LED + Resistor | 11 | 3 | 14 |
| 03 - MCU Power-On | 10 | 3 | 13 |
| 04 - Voltage Divider | 7 | 3 | 10 |
| 05 - I2C Temp Sensor | 5 | 7 | 12 |
| 06 - Op-Amp Buffer | 5 | 4 | 9 |
| **Total** | **49** | **24** | **73** |

---

## Failure Categories

### F1: Default placeholder in created libraries

**Affected:** All scenarios (01-06)
**Severity:** Low (cosmetic count mismatch)

`schlib create` and `pcblib create` produce files with a default empty entry (Component_1 / PCBComponent_1). This inflates `component_count` and `total_footprints` by 1.

| Scenario | Assertion | Expected | Actual |
|----------|-----------|----------|--------|
| 02 | `schlib info .component_count == 2` | 2 | 3 |
| 02 | `pcblib list .total_footprints == 2` | 2 | 3 |
| 03 | `schlib info .component_count == 2` | 2 | 3 |
| 05 | `schlib info .component_count == 4` | 4 | 5 |
| 05 | `pcblib list .total_footprints == 3` | 3 | 4 |
| 06 | `schlib info .component_count == 3` | 3 | 4 |
| 06 | `pcblib list .total_footprints == 2` | 2 | 3 |

**Fix:** Either remove the default placeholder on create, or exclude it from counts.

---

### F2: PCB component placement not implemented

**Affected:** All scenarios (01-06)
**Severity:** High (blocks PCB verification)

`prjpcb import-to-pcb` adds nets to the PcbDoc but prints "component placement not yet implemented". `pcbdoc components --json` returns `total_components: 0` for every scenario.

| Scenario | Expected components |
|----------|-------------------|
| 01 | R1 |
| 02 | R1, D1 |
| 03 | U1, C1 |
| 04 | R1, R2, R3 |
| 05 | U1, R1, R2, C1, J1 |
| 06 | U1, R1, C1, C2 |

**Note:** Nets are imported correctly in all scenarios.

---

### F3: Routing fails for gen-ic components (pins at origin)

**Affected:** Scenarios 03, 05, 06
**Severity:** High (blocks wiring for IC components)

When a component is created with `schlib gen-ic` and placed with `edit add-component`, the `route` command fails with "No route found". Pin locations are reported as `(0 mil, 0 mil)` regardless of component placement coordinates.

This does not affect manually-created components (`schlib add-component` + `schlib add-pin`), which route correctly.

| Scenario | Failed routes | Total routes |
|----------|--------------|-------------|
| 03 | 2 (U1.VCC, U1.GND) | 4 |
| 05 | 13 (all U1.*, R1.*, R2.*, J1.*) | 15 |
| 06 | 6 (U1.VS_POS, U1.VS_NEG, U1.IN_POS, U1.OUT, U1.OUT, U1.IN_NEG) | 12 |

**Root cause:** `gen-ic` pin positions are not being resolved correctly after component placement. The edit session reads pin locations as (0,0) for gen-ic components, so the A* router has no valid start/end points.

---

### F4: Netlist uses generic names instead of net labels

**Affected:** Scenario 04
**Severity:** Medium (netlist queries by label name fail)

`schdoc netlist --json` assigns generic names (Net1, Net2, ...) instead of resolving net label names (VIN, VMID1, VMID2, GND). Filtering with `--filter VMID1` returns 0 results.

The `prjpcb import-to-pcb` command resolves labels correctly via a different code path, proving the labels are stored properly.

| Filter | Expected | Actual |
|--------|----------|--------|
| `--filter VMID1` | >= 2 pins | 0 nets |
| `--filter VMID2` | >= 2 pins | 0 nets |

**Note:** `schdoc netlist` without `--filter` shows `total_nets: 6`, which is correct (VIN, VMID1, VMID2, GND + 2 unnamed segment nets). The issue is that net labels are not being used as net names.

---

### F5: gen-chip ignores user-provided footprint name

**Affected:** Scenarios 01-06
**Severity:** Low (workaround: use auto-generated name)

`pcblib gen-chip fps.PcbLib R0402 --size 0402` ignores the `R0402` argument and auto-names the footprint `CHIP_0402`. The user-provided name is treated as a positional arg that gets overridden.

| Scenario | Requested name | Actual name |
|----------|---------------|-------------|
| 01 | R0402 | CHIP_0402 |
| 02 | R0402, LED0805 | CHIP_0402, CHIP_0805 |
| 03 | C0402 | CHIP_0402 |
| 04 | R0402 | CHIP_0402 |
| 05 | P0402 | CHIP_0402 |
| 06 | P0402 | CHIP_0402 |

---

### F6: add-dual-row creates pads-per-side, not total

**Affected:** Scenario 06
**Severity:** Medium (wrong pad count for asymmetric packages)

`pcblib add-dual-row SOT-23-5 5 --pitch 38 --span 102` creates 10 pads (5 per side) instead of 5 total. The count argument means pads-per-side, not total pads.

Asymmetric packages like SOT-23-5 (3 pads on one side, 2 on the other) cannot be represented with this command.

| Footprint | Expected pads | Actual pads |
|-----------|--------------|-------------|
| SOT-23-5 | 5 | 10 |

---

### F7: Validation MissingJunction false positive

**Affected:** Scenario 01
**Severity:** Low

`edit validate` reports `MissingJunction` at (1050, 1010) even though the wiring is functionally correct. The net labels are placed near pin endpoints and the validator flags the overlap as needing a junction.

---

### F8: Footprint bounding box exceeds expected range

**Affected:** Scenario 01
**Severity:** Low (assertion too strict)

`pcblib measure R0402 --json` reports bounding box width of 112.2 mils. The scenario expected 20-80 mils, but that range was intended for individual pad width, not overall footprint width including pad-to-pad span.

---

## Priority Order for Fixes

1. **F3** - gen-ic pin position resolution (high impact, blocks 3 scenarios)
2. **F2** - PCB component placement (high impact, blocks all scenarios)
3. **F4** - Netlist label resolution (medium impact)
4. **F1** - Default placeholder removal (low effort)
5. **F5** - gen-chip naming (low effort)
6. **F6** - add-dual-row total vs per-side semantics (medium effort)
7. **F7/F8** - Validator/assertion tuning (low priority)

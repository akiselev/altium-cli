# Scenario 04 — Voltage Divider

Three resistors forming a voltage divider with named net labels for VIN, VMID,
and GND. Tests net-label-based connectivity (no direct wires between components),
multiple named nets, and netlist verification of the tap point.

**Parts:** 3 (R1: 10k, R2: 10k, R3: 10k — three equal resistors for 1/3 and 2/3 taps)

---

## Phase 1 — Part Selection

### Step 1.1

```
RUN: datasheet-cli search "10k ohm 0402 1%"
ASSERT: exit 0
```

**Record:** R1=R2=R3 = 10k 0402. VMID1 = VIN × R2R3/(R1+R2+R3), VMID2 = VIN × R3/(R1+R2+R3).

---

## Phase 2 — Schematic Library

### Step 2.1: Create library

```
RUN: altium-cli schlib create work/scenario-04/parts.SchLib
ASSERT: exit 0
```

### Step 2.2: Add resistor symbol

```
RUN: altium-cli schlib add-component work/scenario-04/parts.SchLib RES --description "Resistor"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-04/parts.SchLib RES 1 "1" -50 0 --direction right --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-04/parts.SchLib RES 2 "2" 50 0 --direction left --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-rectangle work/scenario-04/parts.SchLib RES -30 -10 30 10
ASSERT: exit 0
```

### Step 2.3: Verify

```
RUN: altium-cli schlib component work/scenario-04/parts.SchLib RES --json
ASSERT: exit 0
ASSERT: json .pin_count == 2
```

---

## Phase 3 — Footprint Library

### Step 3.1: Create and generate 0402

```
RUN: altium-cli pcblib create work/scenario-04/fps.PcbLib
ASSERT: exit 0
```

```
RUN: altium-cli pcblib gen-chip work/scenario-04/fps.PcbLib R0402 --size 0402
ASSERT: exit 0
```

### Step 3.2: Verify

```
RUN: altium-cli pcblib footprint work/scenario-04/fps.PcbLib R0402 --json
ASSERT: exit 0
ASSERT: json .pad_count == 2
```

---

## Phase 4 — Schematic Entry

### Step 4.1: Create schematic

```
RUN: altium-cli schdoc create work/scenario-04/design.SchDoc
ASSERT: exit 0
```

### Step 4.2: Place three resistors vertically stacked

```
RUN: altium-cli edit work/scenario-04/design.SchDoc -c "add-component work/scenario-04/parts.SchLib RES 1000 1800 R1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-04/design.SchDoc -c "add-component work/scenario-04/parts.SchLib RES 1000 1400 R2"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-04/design.SchDoc -c "add-component work/scenario-04/parts.SchLib RES 1000 1000 R3"
ASSERT: exit 0
```

### Step 4.3: Verify 3 components

```
RUN: altium-cli schdoc components work/scenario-04/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 3
```

### Step 4.4: Add net labels — VIN at top, VMID1 between R1-R2, VMID2 between R2-R3, GND at bottom

```
RUN: altium-cli edit work/scenario-04/design.SchDoc -c "add-net-label VIN 1000 2000"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-04/design.SchDoc -c "add-net-label VMID1 1000 1600"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-04/design.SchDoc -c "add-net-label VMID2 1000 1200"
ASSERT: exit 0
```

### Step 4.5: Add GND power port at bottom

```
RUN: altium-cli edit work/scenario-04/design.SchDoc -c "add-power GND 1000 800 ground up"
ASSERT: exit 0
```

### Step 4.6: Wire R1 into VIN and VMID1 nets

```
RUN: altium-cli edit work/scenario-04/design.SchDoc -c "route R1.1 %VIN"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-04/design.SchDoc -c "route R1.2 %VMID1"
ASSERT: exit 0
```

### Step 4.7: Wire R2 into VMID1 and VMID2 nets

```
RUN: altium-cli edit work/scenario-04/design.SchDoc -c "route R2.1 %VMID1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-04/design.SchDoc -c "route R2.2 %VMID2"
ASSERT: exit 0
```

### Step 4.8: Wire R3 into VMID2 and GND nets

```
RUN: altium-cli edit work/scenario-04/design.SchDoc -c "route R3.1 %VMID2"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-04/design.SchDoc -c "route R3.2 @GND"
ASSERT: exit 0
```

### Step 4.9: Add junctions where nets merge

```
RUN: altium-cli edit work/scenario-04/design.SchDoc -c "add-missing-junctions"
ASSERT: exit 0
```

---

## Phase 5 — Validation

### Step 5.1: Validate

```
RUN: altium-cli edit work/scenario-04/design.SchDoc -c "validate"
ASSERT: exit 0
```

### Step 5.2: Verify 4 nets exist (VIN, VMID1, VMID2, GND)

```
RUN: altium-cli schdoc netlist work/scenario-04/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_nets >= 4
```

### Step 5.3: Verify VMID1 net connects R1.2 and R2.1

```
RUN: altium-cli schdoc netlist work/scenario-04/design.SchDoc --filter VMID1 --json
ASSERT: exit 0
ASSERT: json net "VMID1" has >= 2 pins
```

### Step 5.4: Verify VMID2 net connects R2.2 and R3.1

```
RUN: altium-cli schdoc netlist work/scenario-04/design.SchDoc --filter VMID2 --json
ASSERT: exit 0
ASSERT: json net "VMID2" has >= 2 pins
```

### Step 5.5: BOM

```
RUN: altium-cli schdoc bom work/scenario-04/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 3
```

### MANUAL CHECKPOINT A

Open `work/scenario-04/design.SchDoc` in Altium.
**Check:** Three resistors in a vertical chain. Net labels VIN, VMID1, VMID2 visible at each tap. GND power symbol at bottom.

---

## Phase 6 — Project

### Step 6.1: Create project

```
RUN: altium-cli prjpcb create work/scenario-04/project.PrjPcb --name "Voltage Divider"
ASSERT: exit 0
```

### Step 6.2: Add documents

```
RUN: altium-cli prjpcb add-document work/scenario-04/project.PrjPcb work/scenario-04/design.SchDoc
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-04/project.PrjPcb work/scenario-04/parts.SchLib
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-04/project.PrjPcb work/scenario-04/fps.PcbLib
ASSERT: exit 0
```

---

## Phase 7 — PCB Setup

### Step 7.1: Create PCB (10mm x 15mm = 394 x 591 mil)

```
RUN: altium-cli pcbdoc create work/scenario-04/board.PcbDoc
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc set-outline-rect work/scenario-04/board.PcbDoc 394 591
ASSERT: exit 0
```

### Step 7.2: Rules

```
RUN: altium-cli pcbdoc add-rule work/scenario-04/board.PcbDoc "Clearance" --value 6
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-04/board.PcbDoc "MinTrackWidth" --value 6
ASSERT: exit 0
```

### Step 7.3: Add to project

```
RUN: altium-cli prjpcb add-document work/scenario-04/project.PrjPcb work/scenario-04/board.PcbDoc
ASSERT: exit 0
```

### Step 7.4: Verify

```
RUN: altium-cli pcbdoc rules work/scenario-04/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json .total_rules >= 2
```

---

## Phase 8 — Import

### Step 8.1: Dry-run

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-04/project.PrjPcb --dry-run
ASSERT: exit 0
ASSERT: stdout contains "R1"
ASSERT: stdout contains "R2"
ASSERT: stdout contains "R3"
```

### Step 8.2: Import

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-04/project.PrjPcb
ASSERT: exit 0
```

### Step 8.3: Verify 3 components

```
RUN: altium-cli pcbdoc components work/scenario-04/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes designator "R1"
ASSERT: json output includes designator "R2"
ASSERT: json output includes designator "R3"
```

### Step 8.4: Verify all 4 nets transferred

```
RUN: altium-cli pcbdoc nets work/scenario-04/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes net "VIN"
ASSERT: json output includes net "VMID1"
ASSERT: json output includes net "VMID2"
ASSERT: json output includes net "GND"
```

### MANUAL CHECKPOINT B

Open `work/scenario-04/board.PcbDoc` in Altium.
**Check:** Three identical 0402 footprints. Ratsnest shows chain connectivity (no isolated components).

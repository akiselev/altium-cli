# Scenario 01 — Single Resistor

Bare-minimum end-to-end: one 10k 0402 resistor from part lookup through PCB
import. Tests that the pipeline works at all.

**Parts:** 1 (R1: 10k 0402)

---

## Phase 1 — Part Selection

### Step 1.1: Look up resistor specs

```
RUN: datasheet-cli search "10k ohm 0402 1%"
ASSERT: exit 0
ASSERT: stdout contains "0402"
ASSERT: stdout contains "10k" or "10000"
```

### Step 1.2: Confirm package dimensions

```
RUN: datasheet-cli specs "RC0402FR-0710KL" --fields package,tolerance,power-rating
ASSERT: exit 0
ASSERT: stdout contains "0402"
```

**Record:** R1 = 10k, 0402, 1%, 1/16W

---

## Phase 2 — Schematic Library

### Step 2.1: Create SchLib

```
RUN: altium-cli schlib create work/scenario-01/parts.SchLib
ASSERT: exit 0
ASSERT: stdout contains "Created new SchLib"
ASSERT: file exists work/scenario-01/parts.SchLib
```

### Step 2.2: Add resistor symbol

```
RUN: altium-cli schlib add-component work/scenario-01/parts.SchLib RES --description "Resistor"
ASSERT: exit 0
ASSERT: stdout contains "Added component"
```

### Step 2.3: Add pin 1

```
RUN: altium-cli schlib add-pin work/scenario-01/parts.SchLib RES 1 "1" -50 0 --direction right --electrical passive
ASSERT: exit 0
ASSERT: stdout contains "Added pin"
```

### Step 2.4: Add pin 2

```
RUN: altium-cli schlib add-pin work/scenario-01/parts.SchLib RES 2 "2" 50 0 --direction left --electrical passive
ASSERT: exit 0
ASSERT: stdout contains "Added pin"
```

### Step 2.5: Add body rectangle

```
RUN: altium-cli schlib add-rectangle work/scenario-01/parts.SchLib RES -30 -10 30 10
ASSERT: exit 0
ASSERT: stdout contains "Added rectangle"
```

### Step 2.6: Verify symbol

```
RUN: altium-cli schlib component work/scenario-01/parts.SchLib RES --json
ASSERT: exit 0
ASSERT: json .pin_count == 2
ASSERT: json .name == "RES"
```

### Step 2.7: Verify pin details

```
RUN: altium-cli schlib pins work/scenario-01/parts.SchLib --json
ASSERT: exit 0
ASSERT: json .total_pins == 2
```

---

## Phase 3 — PCB Footprint Library

### Step 3.1: Create PcbLib

```
RUN: altium-cli pcblib create work/scenario-01/fps.PcbLib
ASSERT: exit 0
ASSERT: stdout contains "Created new"
ASSERT: file exists work/scenario-01/fps.PcbLib
```

### Step 3.2: Generate 0402 chip footprint

```
RUN: altium-cli pcblib gen-chip work/scenario-01/fps.PcbLib R0402 --size 0402
ASSERT: exit 0
ASSERT: stdout contains "Added" or stdout contains "Generated"
```

### Step 3.3: Verify footprint exists

```
RUN: altium-cli pcblib footprint work/scenario-01/fps.PcbLib R0402 --json
ASSERT: exit 0
ASSERT: json .pad_count == 2
ASSERT: json .pattern == "R0402"
```

### Step 3.4: Verify pad dimensions are in 0402 range

```
RUN: altium-cli pcblib measure work/scenario-01/fps.PcbLib R0402 --json
ASSERT: exit 0
ASSERT: json .dimensions.width.mils > 20
ASSERT: json .dimensions.width.mils < 80
```

### MANUAL CHECKPOINT A

Open `work/scenario-01/fps.PcbLib` in Altium → footprint R0402.
**Check:** Two rectangular pads, roughly 1mm apart, with silkscreen outline.

---

## Phase 4 — Schematic Entry

### Step 4.1: Create schematic

```
RUN: altium-cli schdoc create work/scenario-01/design.SchDoc
ASSERT: exit 0
ASSERT: stdout contains "Created new schematic"
ASSERT: file exists work/scenario-01/design.SchDoc
```

### Step 4.2: Place resistor

```
RUN: altium-cli edit work/scenario-01/design.SchDoc -c "add-component work/scenario-01/parts.SchLib RES 1000 1000 R1"
ASSERT: exit 0
ASSERT: stdout contains "Success" or stdout contains "Added"
```

### Step 4.3: Verify component placed

```
RUN: altium-cli schdoc components work/scenario-01/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 1
```

### Step 4.4: Add power ports for test connectivity

```
RUN: altium-cli edit work/scenario-01/design.SchDoc -c "add-net-label NET1 950 1000"
ASSERT: exit 0
```

### Step 4.5: Add second net label

```
RUN: altium-cli edit work/scenario-01/design.SchDoc -c "add-net-label NET2 1050 1000"
ASSERT: exit 0
```

### Step 4.6: Route pin 1 to NET1

```
RUN: altium-cli edit work/scenario-01/design.SchDoc -c "route R1.1 %NET1"
ASSERT: exit 0
ASSERT: stdout contains "Success"
```

### Step 4.7: Route pin 2 to NET2

```
RUN: altium-cli edit work/scenario-01/design.SchDoc -c "route R1.2 %NET2"
ASSERT: exit 0
ASSERT: stdout contains "Success"
```

---

## Phase 5 — Validation

### Step 5.1: Validate schematic

```
RUN: altium-cli edit work/scenario-01/design.SchDoc -c "validate"
ASSERT: exit 0
ASSERT: stdout contains "Success" or json .is_valid == true
```

### Step 5.2: Check netlist

```
RUN: altium-cli schdoc netlist work/scenario-01/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_nets >= 2
```

### Step 5.3: Check BOM

```
RUN: altium-cli schdoc bom work/scenario-01/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 1
```

### MANUAL CHECKPOINT B

Open `work/scenario-01/design.SchDoc` in Altium.
**Check:** Single resistor symbol R1 with wires to NET1 and NET2 labels.

---

## Phase 6 — Project Setup

### Step 6.1: Create project

```
RUN: altium-cli prjpcb create work/scenario-01/project.PrjPcb --name "Single Resistor"
ASSERT: exit 0
ASSERT: stdout contains "Created project"
```

### Step 6.2: Add schematic

```
RUN: altium-cli prjpcb add-document work/scenario-01/project.PrjPcb work/scenario-01/design.SchDoc
ASSERT: exit 0
ASSERT: stdout contains "Added document"
```

### Step 6.3: Add libraries

```
RUN: altium-cli prjpcb add-document work/scenario-01/project.PrjPcb work/scenario-01/parts.SchLib
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-01/project.PrjPcb work/scenario-01/fps.PcbLib
ASSERT: exit 0
```

### Step 6.4: Verify project

```
RUN: altium-cli prjpcb overview work/scenario-01/project.PrjPcb --json
ASSERT: exit 0
ASSERT: json .document_summary.total_documents >= 3
```

---

## Phase 7 — PCB Setup

### Step 7.1: Create PCB

```
RUN: altium-cli pcbdoc create work/scenario-01/board.PcbDoc
ASSERT: exit 0
ASSERT: stdout contains "Created new PCB"
ASSERT: file exists work/scenario-01/board.PcbDoc
```

### Step 7.2: Set board outline (10mm x 10mm = 394 x 394 mil)

```
RUN: altium-cli pcbdoc set-outline-rect work/scenario-01/board.PcbDoc 394 394
ASSERT: exit 0
ASSERT: stdout contains "outline"
```

### Step 7.3: Add clearance rule

```
RUN: altium-cli pcbdoc add-rule work/scenario-01/board.PcbDoc "Clearance" --value 6
ASSERT: exit 0
```

### Step 7.4: Add min track width rule

```
RUN: altium-cli pcbdoc add-rule work/scenario-01/board.PcbDoc "MinTrackWidth" --value 6
ASSERT: exit 0
```

### Step 7.5: Verify board setup

```
RUN: altium-cli pcbdoc outline work/scenario-01/board.PcbDoc --json
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc rules work/scenario-01/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json .total_rules >= 2
```

### Step 7.6: Add PCB to project

```
RUN: altium-cli prjpcb add-document work/scenario-01/project.PrjPcb work/scenario-01/board.PcbDoc
ASSERT: exit 0
```

---

## Phase 8 — Import to PCB

### Step 8.1: Dry-run import

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-01/project.PrjPcb --dry-run
ASSERT: exit 0
ASSERT: stdout contains "R1"
```

### Step 8.2: Execute import

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-01/project.PrjPcb
ASSERT: exit 0
```

### Step 8.3: Verify component on PCB

```
RUN: altium-cli pcbdoc components work/scenario-01/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes designator "R1"
```

### Step 8.4: Verify nets on PCB

```
RUN: altium-cli pcbdoc nets work/scenario-01/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes net "NET1"
ASSERT: json output includes net "NET2"
```

### MANUAL CHECKPOINT C

Open `work/scenario-01/board.PcbDoc` in Altium.
**Check:** One 0402 footprint (R1) present, two ratsnest lines visible.

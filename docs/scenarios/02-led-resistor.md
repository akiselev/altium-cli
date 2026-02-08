# Scenario 02 — LED + Current-Limiting Resistor

Two-component circuit with a direct wire between them. Tests `route` between
component pins and mixed footprint types (0402 resistor + 0805 LED).

**Parts:** 2 (R1: 330R 0402, D1: red LED 0805)

---

## Phase 1 — Part Selection

### Step 1.1

```
RUN: datasheet-cli search "red LED 0805 SMD"
ASSERT: exit 0
ASSERT: stdout contains "0805"
```

### Step 1.2

```
RUN: datasheet-cli specs "red LED 0805" --fields forward-voltage,forward-current
ASSERT: exit 0
ASSERT: stdout contains "forward-voltage" or stdout contains "Vf"
```

**Record:** D1 = red LED 0805, Vf~2.0V, If~20mA. R1 = 330R 0402 → (3.3V-2.0V)/330 = 3.9mA.

---

## Phase 2 — Schematic Library

### Step 2.1: Create library

```
RUN: altium-cli schlib create work/scenario-02/parts.SchLib
ASSERT: exit 0
ASSERT: file exists work/scenario-02/parts.SchLib
```

### Step 2.2: Add resistor symbol with pins and body

```
RUN: altium-cli schlib add-component work/scenario-02/parts.SchLib RES --description "Resistor"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-02/parts.SchLib RES 1 "1" -50 0 --direction right --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-02/parts.SchLib RES 2 "2" 50 0 --direction left --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-rectangle work/scenario-02/parts.SchLib RES -30 -10 30 10
ASSERT: exit 0
```

### Step 2.3: Add LED symbol with pins and body

```
RUN: altium-cli schlib add-component work/scenario-02/parts.SchLib LED --description "LED"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-02/parts.SchLib LED A "Anode" -50 0 --direction right --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-02/parts.SchLib LED K "Cathode" 50 0 --direction left --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-rectangle work/scenario-02/parts.SchLib LED -30 -20 30 20
ASSERT: exit 0
```

### Step 2.4: Verify library has 2 components

```
RUN: altium-cli schlib info work/scenario-02/parts.SchLib --json
ASSERT: exit 0
ASSERT: json .component_count == 2
```

### Step 2.5: Verify resistor pins

```
RUN: altium-cli schlib component work/scenario-02/parts.SchLib RES --json
ASSERT: exit 0
ASSERT: json .pin_count == 2
```

### Step 2.6: Verify LED pins

```
RUN: altium-cli schlib component work/scenario-02/parts.SchLib LED --json
ASSERT: exit 0
ASSERT: json .pin_count == 2
```

---

## Phase 3 — Footprint Library

### Step 3.1: Create PcbLib

```
RUN: altium-cli pcblib create work/scenario-02/fps.PcbLib
ASSERT: exit 0
ASSERT: file exists work/scenario-02/fps.PcbLib
```

### Step 3.2: Generate 0402 for resistor

```
RUN: altium-cli pcblib gen-chip work/scenario-02/fps.PcbLib R0402 --size 0402
ASSERT: exit 0
```

### Step 3.3: Generate 0805 for LED

```
RUN: altium-cli pcblib gen-chip work/scenario-02/fps.PcbLib LED0805 --size 0805
ASSERT: exit 0
```

### Step 3.4: Verify both footprints exist

```
RUN: altium-cli pcblib list work/scenario-02/fps.PcbLib --json
ASSERT: exit 0
ASSERT: json .total_footprints == 2
```

### Step 3.5: Verify 0402 has 2 pads

```
RUN: altium-cli pcblib footprint work/scenario-02/fps.PcbLib R0402 --json
ASSERT: exit 0
ASSERT: json .pad_count == 2
```

### Step 3.6: Verify 0805 has 2 pads

```
RUN: altium-cli pcblib footprint work/scenario-02/fps.PcbLib LED0805 --json
ASSERT: exit 0
ASSERT: json .pad_count == 2
```

### MANUAL CHECKPOINT A

Open `work/scenario-02/fps.PcbLib` in Altium.
**Check:** R0402 pads are visibly smaller than LED0805 pads.

---

## Phase 4 — Schematic Entry

### Step 4.1: Create schematic

```
RUN: altium-cli schdoc create work/scenario-02/design.SchDoc
ASSERT: exit 0
ASSERT: file exists work/scenario-02/design.SchDoc
```

### Step 4.2: Place resistor at (800, 1000)

```
RUN: altium-cli edit work/scenario-02/design.SchDoc -c "add-component work/scenario-02/parts.SchLib RES 800 1000 R1"
ASSERT: exit 0
ASSERT: stdout contains "Success" or stdout contains "Added"
```

### Step 4.3: Place LED at (1200, 1000)

```
RUN: altium-cli edit work/scenario-02/design.SchDoc -c "add-component work/scenario-02/parts.SchLib LED 1200 1000 D1"
ASSERT: exit 0
```

### Step 4.4: Verify 2 components placed

```
RUN: altium-cli schdoc components work/scenario-02/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 2
```

### Step 4.5: Add VCC power port above R1

```
RUN: altium-cli edit work/scenario-02/design.SchDoc -c "add-power VCC 800 1200 bar down"
ASSERT: exit 0
```

### Step 4.6: Add GND power port below D1

```
RUN: altium-cli edit work/scenario-02/design.SchDoc -c "add-power GND 1200 800 ground up"
ASSERT: exit 0
```

### Step 4.7: Wire VCC → R1 pin 1

```
RUN: altium-cli edit work/scenario-02/design.SchDoc -c "route @VCC R1.1"
ASSERT: exit 0
ASSERT: stdout contains "Success"
```

### Step 4.8: Wire R1 pin 2 → D1 Anode (direct wire between components)

```
RUN: altium-cli edit work/scenario-02/design.SchDoc -c "route R1.2 D1.A"
ASSERT: exit 0
ASSERT: stdout contains "Success"
```

### Step 4.9: Wire D1 Cathode → GND

```
RUN: altium-cli edit work/scenario-02/design.SchDoc -c "route D1.K @GND"
ASSERT: exit 0
ASSERT: stdout contains "Success"
```

### Step 4.10: Verify wire count

```
RUN: altium-cli schdoc wires work/scenario-02/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_wires >= 3
```

---

## Phase 5 — Validation

### Step 5.1: Validate

```
RUN: altium-cli edit work/scenario-02/design.SchDoc -c "validate"
ASSERT: exit 0
```

### Step 5.2: Check netlist — should have VCC, GND, and the R1-D1 junction net

```
RUN: altium-cli schdoc netlist work/scenario-02/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_nets >= 2
```

### Step 5.3: Check BOM

```
RUN: altium-cli schdoc bom work/scenario-02/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 2
```

### MANUAL CHECKPOINT B

Open `work/scenario-02/design.SchDoc` in Altium.
**Check:** R1 and D1 connected in series between VCC and GND, with a visible wire from R1.2 to D1.A.

---

## Phase 6 — Project

### Step 6.1: Create project

```
RUN: altium-cli prjpcb create work/scenario-02/project.PrjPcb --name "LED Circuit"
ASSERT: exit 0
```

### Step 6.2: Add all documents

```
RUN: altium-cli prjpcb add-document work/scenario-02/project.PrjPcb work/scenario-02/design.SchDoc
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-02/project.PrjPcb work/scenario-02/parts.SchLib
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-02/project.PrjPcb work/scenario-02/fps.PcbLib
ASSERT: exit 0
```

### Step 6.3: Verify project has all docs

```
RUN: altium-cli prjpcb documents work/scenario-02/project.PrjPcb --json
ASSERT: exit 0
ASSERT: json .total_documents >= 3
```

---

## Phase 7 — PCB Setup

### Step 7.1: Create PCB

```
RUN: altium-cli pcbdoc create work/scenario-02/board.PcbDoc
ASSERT: exit 0
ASSERT: file exists work/scenario-02/board.PcbDoc
```

### Step 7.2: Set board outline (15mm x 10mm = 591 x 394 mil)

```
RUN: altium-cli pcbdoc set-outline-rect work/scenario-02/board.PcbDoc 591 394
ASSERT: exit 0
```

### Step 7.3: Set grid and track settings

```
RUN: altium-cli pcbdoc set-settings work/scenario-02/board.PcbDoc --imperial --grid-size 25 --track-width 10
ASSERT: exit 0
```

### Step 7.4: Add clearance rule

```
RUN: altium-cli pcbdoc add-rule work/scenario-02/board.PcbDoc "Clearance" --value 8
ASSERT: exit 0
```

### Step 7.5: Verify rules

```
RUN: altium-cli pcbdoc rules work/scenario-02/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json .total_rules >= 1
```

### Step 7.6: Add PCB to project

```
RUN: altium-cli prjpcb add-document work/scenario-02/project.PrjPcb work/scenario-02/board.PcbDoc
ASSERT: exit 0
```

---

## Phase 8 — Import to PCB

### Step 8.1: Dry-run

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-02/project.PrjPcb --dry-run
ASSERT: exit 0
ASSERT: stdout contains "R1"
ASSERT: stdout contains "D1"
```

### Step 8.2: Execute import

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-02/project.PrjPcb
ASSERT: exit 0
```

### Step 8.3: Verify components on PCB

```
RUN: altium-cli pcbdoc components work/scenario-02/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes designator "R1"
ASSERT: json output includes designator "D1"
```

### Step 8.4: Verify nets on PCB

```
RUN: altium-cli pcbdoc nets work/scenario-02/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes net "VCC"
ASSERT: json output includes net "GND"
```

### MANUAL CHECKPOINT C

Open `work/scenario-02/board.PcbDoc` in Altium.
**Check:** R1 has smaller 0402 pads, D1 has larger 0805 pads. Ratsnest connects them.

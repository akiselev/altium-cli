# Scenario 05 — I2C Temperature Sensor Breakout

Sensor IC + two I2C pull-up resistors + bypass cap + pin header. Tests `gen-ic`
with 6 pins, I2C bus net labels (SDA/SCL shared across multiple components), and
mixed footprints (WSON-6 + 0402 + through-hole header).

**Parts:** 5 (U1: TMP117 WSON-6, R1: 4.7k SDA pull-up, R2: 4.7k SCL pull-up,
C1: 100nF bypass, J1: 1x4 header)

---

## Phase 1 — Part Selection

### Step 1.1

```
RUN: datasheet-cli search "TMP117" --package WSON
ASSERT: exit 0
ASSERT: stdout contains "TMP117"
```

### Step 1.2

```
RUN: datasheet-cli pinout "TMP117AIDRVR"
ASSERT: exit 0
ASSERT: stdout contains "SDA"
ASSERT: stdout contains "SCL"
```

**Record:** U1 = TMP117, WSON-6. Pins: SCL(1), GND(2), ALERT(3), ADD0(4), SDA(5), V+(6).

---

## Phase 2 — Schematic Library

### Step 2.1: Create

```
RUN: altium-cli schlib create work/scenario-05/parts.SchLib
ASSERT: exit 0
```

### Step 2.2: Generate TMP117 symbol

```
RUN: altium-cli schlib gen-ic work/scenario-05/parts.SchLib TMP117 --pins "SCL,GND,ALERT,ADD0,SDA,VDD" --description "I2C Temp Sensor"
ASSERT: exit 0
```

### Step 2.3: Verify TMP117 has 6 pins

```
RUN: altium-cli schlib component work/scenario-05/parts.SchLib TMP117 --json
ASSERT: exit 0
ASSERT: json .pin_count == 6
```

### Step 2.4: Add resistor symbol

```
RUN: altium-cli schlib add-component work/scenario-05/parts.SchLib RES --description "Resistor"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-05/parts.SchLib RES 1 "1" -50 0 --direction right --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-05/parts.SchLib RES 2 "2" 50 0 --direction left --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-rectangle work/scenario-05/parts.SchLib RES -30 -10 30 10
ASSERT: exit 0
```

### Step 2.5: Add capacitor symbol

```
RUN: altium-cli schlib add-component work/scenario-05/parts.SchLib CAP --description "Capacitor"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-05/parts.SchLib CAP 1 "1" 0 50 --direction down --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-05/parts.SchLib CAP 2 "2" 0 -50 --direction up --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-line work/scenario-05/parts.SchLib CAP -20 10 20 10
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-line work/scenario-05/parts.SchLib CAP -20 -10 20 -10
ASSERT: exit 0
```

### Step 2.6: Generate 1x4 header symbol

```
RUN: altium-cli schlib gen-ic work/scenario-05/parts.SchLib HDR4 --pins "VDD,GND,SDA,SCL" --description "1x4 Header"
ASSERT: exit 0
```

### Step 2.7: Verify 4 components in library

```
RUN: altium-cli schlib info work/scenario-05/parts.SchLib --json
ASSERT: exit 0
ASSERT: json .component_count == 4
```

---

## Phase 3 — Footprint Library

### Step 3.1: Create

```
RUN: altium-cli pcblib create work/scenario-05/fps.PcbLib
ASSERT: exit 0
```

### Step 3.2: WSON-6 footprint (dual-row, 3 per side)

```
RUN: altium-cli pcblib add-footprint work/scenario-05/fps.PcbLib WSON-6 --description "WSON-6 2x2mm"
ASSERT: exit 0
```

```
RUN: altium-cli pcblib add-dual-row work/scenario-05/fps.PcbLib WSON-6 6 --pitch 26 --span 80
ASSERT: exit 0
```

### Step 3.3: Verify WSON-6 has 6 pads

```
RUN: altium-cli pcblib footprint work/scenario-05/fps.PcbLib WSON-6 --json
ASSERT: exit 0
ASSERT: json .pad_count == 6
```

### Step 3.4: 0402 for passives

```
RUN: altium-cli pcblib gen-chip work/scenario-05/fps.PcbLib P0402 --size 0402
ASSERT: exit 0
```

### Step 3.5: 1x4 through-hole header

```
RUN: altium-cli pcblib add-footprint work/scenario-05/fps.PcbLib HDR-1X4 --description "1x4 2.54mm header"
ASSERT: exit 0
```

```
RUN: altium-cli pcblib add-pad-row work/scenario-05/fps.PcbLib HDR-1X4 4 --pitch 100
ASSERT: exit 0
```

### Step 3.6: Verify header has 4 through-hole pads

```
RUN: altium-cli pcblib footprint work/scenario-05/fps.PcbLib HDR-1X4 --json
ASSERT: exit 0
ASSERT: json .pad_count == 4
```

### Step 3.7: Verify library has 3 footprints

```
RUN: altium-cli pcblib list work/scenario-05/fps.PcbLib --json
ASSERT: exit 0
ASSERT: json .total_footprints == 3
```

### MANUAL CHECKPOINT A

Open `work/scenario-05/fps.PcbLib` in Altium → WSON-6.
**Check:** 6 small SMD pads in two rows of 3, body outline visible.

---

## Phase 4 — Schematic Entry

### Step 4.1: Create schematic

```
RUN: altium-cli schdoc create work/scenario-05/design.SchDoc
ASSERT: exit 0
```

### Step 4.2: Place components

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "add-component work/scenario-05/parts.SchLib TMP117 1200 1400 U1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "add-component work/scenario-05/parts.SchLib RES 900 1800 R1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "add-component work/scenario-05/parts.SchLib RES 1100 1800 R2"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "add-component work/scenario-05/parts.SchLib CAP 1500 1200 C1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "add-component work/scenario-05/parts.SchLib HDR4 500 1400 J1"
ASSERT: exit 0
```

### Step 4.3: Verify 5 components

```
RUN: altium-cli schdoc components work/scenario-05/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 5
```

### Step 4.4: Add power symbols

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "add-power VDD 1200 2100 bar down"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "add-power GND 1200 800 ground up"
ASSERT: exit 0
```

### Step 4.5: Add I2C bus net labels

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "add-net-label SDA 900 1600"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "add-net-label SCL 1100 1600"
ASSERT: exit 0
```

### Step 4.6: Wire sensor power

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "route U1.VDD @VDD"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "route U1.GND @GND"
ASSERT: exit 0
```

### Step 4.7: Wire sensor I2C pins to bus labels

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "route U1.SDA %SDA"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "route U1.SCL %SCL"
ASSERT: exit 0
```

### Step 4.8: Wire pull-up R1 between VDD and SDA

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "route R1.1 @VDD"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "route R1.2 %SDA"
ASSERT: exit 0
```

### Step 4.9: Wire pull-up R2 between VDD and SCL

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "route R2.1 @VDD"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "route R2.2 %SCL"
ASSERT: exit 0
```

### Step 4.10: Wire bypass cap

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "route C1.1 @VDD"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "route C1.2 @GND"
ASSERT: exit 0
```

### Step 4.11: Wire header to buses

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "route J1.VDD @VDD"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "route J1.GND @GND"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "route J1.SDA %SDA"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "route J1.SCL %SCL"
ASSERT: exit 0
```

### Step 4.12: Tie ADD0 to GND (address 0x48)

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "route U1.ADD0 @GND"
ASSERT: exit 0
```

### Step 4.13: Add junctions

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "add-missing-junctions"
ASSERT: exit 0
```

---

## Phase 5 — Validation

### Step 5.1: Validate

```
RUN: altium-cli edit work/scenario-05/design.SchDoc -c "validate"
ASSERT: exit 0
```

### Step 5.2: Verify SDA net has 3 pins (U1.SDA, R1.2, J1.SDA)

```
RUN: altium-cli schdoc netlist work/scenario-05/design.SchDoc --filter SDA --json
ASSERT: exit 0
ASSERT: json net "SDA" has >= 3 pins
```

### Step 5.3: Verify SCL net has 3 pins (U1.SCL, R2.2, J1.SCL)

```
RUN: altium-cli schdoc netlist work/scenario-05/design.SchDoc --filter SCL --json
ASSERT: exit 0
ASSERT: json net "SCL" has >= 3 pins
```

### Step 5.4: Verify VDD net has 5 pins (U1.VDD, R1.1, R2.1, C1.1, J1.VDD)

```
RUN: altium-cli schdoc netlist work/scenario-05/design.SchDoc --filter VDD --json
ASSERT: exit 0
ASSERT: json net "VDD" has >= 5 pins
```

### Step 5.5: BOM has 5 parts

```
RUN: altium-cli schdoc bom work/scenario-05/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 5
```

### MANUAL CHECKPOINT B

Open `work/scenario-05/design.SchDoc` in Altium.
**Check:** SDA and SCL labels appear at U1, at both pull-up resistors, and at J1. Pull-ups go to VDD rail.

---

## Phase 6 — Project

### Step 6.1

```
RUN: altium-cli prjpcb create work/scenario-05/project.PrjPcb --name "I2C Temp Sensor"
ASSERT: exit 0
```

### Step 6.2

```
RUN: altium-cli prjpcb add-document work/scenario-05/project.PrjPcb work/scenario-05/design.SchDoc
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-05/project.PrjPcb work/scenario-05/parts.SchLib
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-05/project.PrjPcb work/scenario-05/fps.PcbLib
ASSERT: exit 0
```

---

## Phase 7 — PCB Setup

### Step 7.1: Create PCB (20mm x 12mm = 787 x 472 mil)

```
RUN: altium-cli pcbdoc create work/scenario-05/board.PcbDoc
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc set-outline-rect work/scenario-05/board.PcbDoc 787 472
ASSERT: exit 0
```

### Step 7.2: Rules

```
RUN: altium-cli pcbdoc set-settings work/scenario-05/board.PcbDoc --metric --grid-size 10 --track-width 8
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-05/board.PcbDoc "Clearance" --value 6
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-05/board.PcbDoc "MinTrackWidth" --value 6
ASSERT: exit 0
```

### Step 7.3: Add to project

```
RUN: altium-cli prjpcb add-document work/scenario-05/project.PrjPcb work/scenario-05/board.PcbDoc
ASSERT: exit 0
```

---

## Phase 8 — Import

### Step 8.1: Dry-run

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-05/project.PrjPcb --dry-run
ASSERT: exit 0
ASSERT: stdout contains "U1"
ASSERT: stdout contains "R1"
ASSERT: stdout contains "R2"
ASSERT: stdout contains "C1"
ASSERT: stdout contains "J1"
```

### Step 8.2: Import

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-05/project.PrjPcb
ASSERT: exit 0
```

### Step 8.3: Verify 5 components on PCB

```
RUN: altium-cli pcbdoc components work/scenario-05/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes designator "U1"
ASSERT: json output includes designator "R1"
ASSERT: json output includes designator "R2"
ASSERT: json output includes designator "C1"
ASSERT: json output includes designator "J1"
```

### Step 8.4: Verify key nets

```
RUN: altium-cli pcbdoc nets work/scenario-05/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes net "SDA"
ASSERT: json output includes net "SCL"
ASSERT: json output includes net "VDD"
ASSERT: json output includes net "GND"
```

### MANUAL CHECKPOINT C

Open `work/scenario-05/board.PcbDoc` in Altium.
**Check:** U1 is a tiny 6-pad WSON, J1 is a through-hole 4-pin header (visible drill holes). Three 0402s for passives. Ratsnest shows SDA/SCL buses.

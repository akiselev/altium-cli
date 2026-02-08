# Scenario 03 — MCU Power-On (IC + Bypass Cap)

An 8-pin IC generated with `gen-ic` plus a bypass capacitor connected via
VCC/GND power ports. Tests IC symbol generation, power port wiring, and
SOIC-8 footprint creation.

**Parts:** 2 (U1: ATtiny85 SOIC-8, C1: 100nF 0402)

---

## Phase 1 — Part Selection

### Step 1.1

```
RUN: datasheet-cli search "ATtiny85" --package SOIC-8
ASSERT: exit 0
ASSERT: stdout contains "ATtiny85"
```

### Step 1.2

```
RUN: datasheet-cli pinout "ATtiny85-20SU"
ASSERT: exit 0
ASSERT: stdout contains "VCC"
ASSERT: stdout contains "GND"
```

**Record:** U1 = ATtiny85, SOIC-8. Pins: PB5(1), PB3(2), PB4(3), GND(4), PB0(5), PB1(6), PB2(7), VCC(8). C1 = 100nF 0402 between VCC and GND.

---

## Phase 2 — Schematic Library

### Step 2.1: Create library

```
RUN: altium-cli schlib create work/scenario-03/parts.SchLib
ASSERT: exit 0
ASSERT: file exists work/scenario-03/parts.SchLib
```

### Step 2.2: Generate ATtiny85 IC symbol with gen-ic

```
RUN: altium-cli schlib gen-ic work/scenario-03/parts.SchLib ATtiny85 --pins "PB5,PB3,PB4,GND,PB0,PB1,PB2,VCC" --description "8-bit AVR MCU"
ASSERT: exit 0
ASSERT: stdout contains "Generated IC symbol"
```

### Step 2.3: Verify IC has 8 pins

```
RUN: altium-cli schlib component work/scenario-03/parts.SchLib ATtiny85 --json
ASSERT: exit 0
ASSERT: json .pin_count == 8
ASSERT: json .description == "8-bit AVR MCU"
```

### Step 2.4: Verify all pin names present

```
RUN: altium-cli schlib pins work/scenario-03/parts.SchLib --json
ASSERT: exit 0
ASSERT: json .total_pins == 8
```

### Step 2.5: Add capacitor symbol

```
RUN: altium-cli schlib add-component work/scenario-03/parts.SchLib CAP --description "Capacitor"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-03/parts.SchLib CAP 1 "1" 0 50 --direction down --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-03/parts.SchLib CAP 2 "2" 0 -50 --direction up --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-line work/scenario-03/parts.SchLib CAP -20 10 20 10
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-line work/scenario-03/parts.SchLib CAP -20 -10 20 -10
ASSERT: exit 0
```

### Step 2.6: Verify library has 2 components

```
RUN: altium-cli schlib info work/scenario-03/parts.SchLib --json
ASSERT: exit 0
ASSERT: json .component_count == 2
```

### MANUAL CHECKPOINT A

Open `work/scenario-03/parts.SchLib` in Altium → ATtiny85.
**Check:** Rectangle body with 4 pins per side, pin names readable.

---

## Phase 3 — Footprint Library

### Step 3.1: Create PcbLib

```
RUN: altium-cli pcblib create work/scenario-03/fps.PcbLib
ASSERT: exit 0
ASSERT: file exists work/scenario-03/fps.PcbLib
```

### Step 3.2: Create SOIC-8 footprint with dual-row pads

```
RUN: altium-cli pcblib add-footprint work/scenario-03/fps.PcbLib SOIC-8 --description "SOIC-8 150mil body"
ASSERT: exit 0
```

```
RUN: altium-cli pcblib add-dual-row work/scenario-03/fps.PcbLib SOIC-8 8 --pitch 50 --span 244
ASSERT: exit 0
```

### Step 3.3: Verify SOIC-8 has 8 pads

```
RUN: altium-cli pcblib footprint work/scenario-03/fps.PcbLib SOIC-8 --json
ASSERT: exit 0
ASSERT: json .pad_count == 8
```

### Step 3.4: Verify pitch is ~50 mil

```
RUN: altium-cli pcblib measure work/scenario-03/fps.PcbLib SOIC-8 --json
ASSERT: exit 0
ASSERT: json .pitch[0].pitch.mils >= 45
ASSERT: json .pitch[0].pitch.mils <= 55
```

### Step 3.5: Generate 0402 for bypass cap

```
RUN: altium-cli pcblib gen-chip work/scenario-03/fps.PcbLib C0402 --size 0402
ASSERT: exit 0
```

### Step 3.6: Verify 0402 footprint

```
RUN: altium-cli pcblib footprint work/scenario-03/fps.PcbLib C0402 --json
ASSERT: exit 0
ASSERT: json .pad_count == 2
```

### MANUAL CHECKPOINT B

Open `work/scenario-03/fps.PcbLib` in Altium → SOIC-8.
**Check:** 8 pads in two rows of 4, pin 1 marker visible.

---

## Phase 4 — Schematic Entry

### Step 4.1: Create schematic

```
RUN: altium-cli schdoc create work/scenario-03/design.SchDoc
ASSERT: exit 0
```

### Step 4.2: Place MCU

```
RUN: altium-cli edit work/scenario-03/design.SchDoc -c "add-component work/scenario-03/parts.SchLib ATtiny85 1000 1500 U1"
ASSERT: exit 0
```

### Step 4.3: Place bypass cap near MCU

```
RUN: altium-cli edit work/scenario-03/design.SchDoc -c "add-component work/scenario-03/parts.SchLib CAP 700 1200 C1"
ASSERT: exit 0
```

### Step 4.4: Verify 2 components

```
RUN: altium-cli schdoc components work/scenario-03/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 2
```

### Step 4.5: Add VCC power port

```
RUN: altium-cli edit work/scenario-03/design.SchDoc -c "add-power VCC 1000 2000 bar down"
ASSERT: exit 0
```

### Step 4.6: Add GND power port

```
RUN: altium-cli edit work/scenario-03/design.SchDoc -c "add-power GND 1000 800 ground up"
ASSERT: exit 0
```

### Step 4.7: Wire U1.VCC to VCC rail

```
RUN: altium-cli edit work/scenario-03/design.SchDoc -c "route U1.VCC @VCC"
ASSERT: exit 0
ASSERT: stdout contains "Success"
```

### Step 4.8: Wire U1.GND to GND rail

```
RUN: altium-cli edit work/scenario-03/design.SchDoc -c "route U1.GND @GND"
ASSERT: exit 0
ASSERT: stdout contains "Success"
```

### Step 4.9: Wire bypass cap pin 1 to VCC

```
RUN: altium-cli edit work/scenario-03/design.SchDoc -c "route C1.1 @VCC"
ASSERT: exit 0
```

### Step 4.10: Wire bypass cap pin 2 to GND

```
RUN: altium-cli edit work/scenario-03/design.SchDoc -c "route C1.2 @GND"
ASSERT: exit 0
```

### Step 4.11: Add junctions

```
RUN: altium-cli edit work/scenario-03/design.SchDoc -c "add-missing-junctions"
ASSERT: exit 0
```

---

## Phase 5 — Validation

### Step 5.1: Validate — expect warnings for unconnected PB0-PB5 (no load)

```
RUN: altium-cli edit work/scenario-03/design.SchDoc -c "validate"
ASSERT: exit 0 or exit 1
```

*Note:* PB0-PB5 are intentionally unconnected. If validate reports warnings for
floating pins, that is expected and correct.

### Step 5.2: Check netlist has VCC and GND

```
RUN: altium-cli schdoc netlist work/scenario-03/design.SchDoc --json
ASSERT: exit 0
ASSERT: json output includes net name containing "VCC"
ASSERT: json output includes net name containing "GND"
```

### Step 5.3: Verify VCC net connects U1.VCC and C1.1

```
RUN: altium-cli schdoc netlist work/scenario-03/design.SchDoc --filter VCC --json
ASSERT: exit 0
ASSERT: json output includes pin "U1.VCC" or "U1:VCC"
ASSERT: json output includes pin "C1.1" or "C1:1"
```

### Step 5.4: Check BOM

```
RUN: altium-cli schdoc bom work/scenario-03/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 2
```

### Step 5.5: Check power map

```
RUN: altium-cli schdoc power-map work/scenario-03/design.SchDoc --json
ASSERT: exit 0
```

### MANUAL CHECKPOINT C

Open `work/scenario-03/design.SchDoc` in Altium.
**Check:** U1 (ATtiny85) has VCC and GND wired to power ports, C1 bridges VCC-GND.

---

## Phase 6 — Project

### Step 6.1: Create project

```
RUN: altium-cli prjpcb create work/scenario-03/project.PrjPcb --name "MCU Power"
ASSERT: exit 0
```

### Step 6.2: Add documents

```
RUN: altium-cli prjpcb add-document work/scenario-03/project.PrjPcb work/scenario-03/design.SchDoc
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-03/project.PrjPcb work/scenario-03/parts.SchLib
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-03/project.PrjPcb work/scenario-03/fps.PcbLib
ASSERT: exit 0
```

---

## Phase 7 — PCB Setup

### Step 7.1: Create PCB

```
RUN: altium-cli pcbdoc create work/scenario-03/board.PcbDoc
ASSERT: exit 0
```

### Step 7.2: Board outline 15mm x 15mm (591 x 591 mil)

```
RUN: altium-cli pcbdoc set-outline-rect work/scenario-03/board.PcbDoc 591 591
ASSERT: exit 0
```

### Step 7.3: Settings

```
RUN: altium-cli pcbdoc set-settings work/scenario-03/board.PcbDoc --imperial --grid-size 25 --track-width 10
ASSERT: exit 0
```

### Step 7.4: Rules

```
RUN: altium-cli pcbdoc add-rule work/scenario-03/board.PcbDoc "Clearance" --value 8
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-03/board.PcbDoc "MinTrackWidth" --value 8
ASSERT: exit 0
```

### Step 7.5: Add to project

```
RUN: altium-cli prjpcb add-document work/scenario-03/project.PrjPcb work/scenario-03/board.PcbDoc
ASSERT: exit 0
```

---

## Phase 8 — Import

### Step 8.1: Dry-run

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-03/project.PrjPcb --dry-run
ASSERT: exit 0
ASSERT: stdout contains "U1"
ASSERT: stdout contains "C1"
```

### Step 8.2: Import

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-03/project.PrjPcb
ASSERT: exit 0
```

### Step 8.3: Verify 2 components on PCB

```
RUN: altium-cli pcbdoc components work/scenario-03/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes designator "U1"
ASSERT: json output includes designator "C1"
```

### Step 8.4: Verify VCC and GND nets on PCB

```
RUN: altium-cli pcbdoc nets work/scenario-03/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes net "VCC"
ASSERT: json output includes net "GND"
```

### MANUAL CHECKPOINT D

Open `work/scenario-03/board.PcbDoc` in Altium.
**Check:** U1 is SOIC-8 (8 pads, two rows), C1 is 0402 (2 small pads). Ratsnest shows VCC/GND connections.

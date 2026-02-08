# Scenario 09 — LDO Voltage Regulator

3.3V LDO with input/output filtering and an enable pin tied through a resistor.
Tests power-in/power-out rail naming, ceramic cap placement convention, and
SOT-223 footprint generation.

**Parts:** 4 (U1: AMS1117-3.3 SOT-223, C1: 10uF input 0805, C2: 10uF output
0805, R1: 100k enable pull-up 0402)

---

## Phase 1 — Part Selection

### Step 1.1

```
RUN: datasheet-cli search "AMS1117-3.3" --package SOT-223
ASSERT: exit 0
ASSERT: stdout contains "AMS1117" or stdout contains "1117"
```

### Step 1.2

```
RUN: datasheet-cli pinout "AMS1117-3.3"
ASSERT: exit 0
ASSERT: stdout contains "IN" or stdout contains "input"
ASSERT: stdout contains "OUT" or stdout contains "output"
```

**Record:** U1 = AMS1117-3.3, SOT-223. Pins: GND/ADJ(1), VOUT(2), VIN(3), TAB=VOUT. Fixed 3.3V output, 1A max.

---

## Phase 2 — Schematic Library

### Step 2.1: Create

```
RUN: altium-cli schlib create work/scenario-09/parts.SchLib
ASSERT: exit 0
```

### Step 2.2: Generate LDO symbol (4 pins: VIN, VOUT, GND, EN)

```
RUN: altium-cli schlib gen-ic work/scenario-09/parts.SchLib LDO --pins "VIN,VOUT,GND,EN" --description "3.3V LDO Regulator"
ASSERT: exit 0
```

### Step 2.3: Verify 4 pins

```
RUN: altium-cli schlib component work/scenario-09/parts.SchLib LDO --json
ASSERT: exit 0
ASSERT: json .pin_count == 4
```

### Step 2.4: Add passive symbols

```
RUN: altium-cli schlib add-component work/scenario-09/parts.SchLib CAP --description "Capacitor"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-09/parts.SchLib CAP 1 "1" 0 50 --direction down --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-09/parts.SchLib CAP 2 "2" 0 -50 --direction up --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-line work/scenario-09/parts.SchLib CAP -20 10 20 10
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-line work/scenario-09/parts.SchLib CAP -20 -10 20 -10
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-component work/scenario-09/parts.SchLib RES --description "Resistor"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-09/parts.SchLib RES 1 "1" -50 0 --direction right --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-09/parts.SchLib RES 2 "2" 50 0 --direction left --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-rectangle work/scenario-09/parts.SchLib RES -30 -10 30 10
ASSERT: exit 0
```

### Step 2.5: Verify 3 library components

```
RUN: altium-cli schlib info work/scenario-09/parts.SchLib --json
ASSERT: exit 0
ASSERT: json .component_count == 3
```

---

## Phase 3 — Footprint Library

### Step 3.1: Create

```
RUN: altium-cli pcblib create work/scenario-09/fps.PcbLib
ASSERT: exit 0
```

### Step 3.2: SOT-223 (4 pads: 3 small + 1 tab)

```
RUN: altium-cli pcblib add-footprint work/scenario-09/fps.PcbLib SOT-223 --description "SOT-223-3"
ASSERT: exit 0
```

```
RUN: altium-cli pcblib add-dual-row work/scenario-09/fps.PcbLib SOT-223 4 --pitch 90 --span 260
ASSERT: exit 0
```

### Step 3.3: Verify 4 pads

```
RUN: altium-cli pcblib footprint work/scenario-09/fps.PcbLib SOT-223 --json
ASSERT: exit 0
ASSERT: json .pad_count == 4
```

### Step 3.4: 0805 for capacitors

```
RUN: altium-cli pcblib gen-chip work/scenario-09/fps.PcbLib C0805 --size 0805
ASSERT: exit 0
```

### Step 3.5: 0402 for resistor

```
RUN: altium-cli pcblib gen-chip work/scenario-09/fps.PcbLib R0402 --size 0402
ASSERT: exit 0
```

### Step 3.6: Verify 3 footprints

```
RUN: altium-cli pcblib list work/scenario-09/fps.PcbLib --json
ASSERT: exit 0
ASSERT: json .total_footprints == 3
```

### MANUAL CHECKPOINT A

Open `work/scenario-09/fps.PcbLib` in Altium → SOT-223.
**Check:** 3 small pads on one side, 1 large tab pad on the other side.

---

## Phase 4 — Schematic Entry

### Step 4.1: Create

```
RUN: altium-cli schdoc create work/scenario-09/design.SchDoc
ASSERT: exit 0
```

### Step 4.2: Place components

```
RUN: altium-cli edit work/scenario-09/design.SchDoc -c "add-component work/scenario-09/parts.SchLib LDO 1200 1400 U1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-09/design.SchDoc -c "add-component work/scenario-09/parts.SchLib CAP 900 1200 C1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-09/design.SchDoc -c "add-component work/scenario-09/parts.SchLib CAP 1500 1200 C2"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-09/design.SchDoc -c "add-component work/scenario-09/parts.SchLib RES 1000 1700 R1"
ASSERT: exit 0
```

### Step 4.3: Verify 4 components

```
RUN: altium-cli schdoc components work/scenario-09/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 4
```

### Step 4.4: Power rails — separate input and output rail names

```
RUN: altium-cli edit work/scenario-09/design.SchDoc -c "add-net-label VIN_5V 900 1600"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-09/design.SchDoc -c "add-net-label VOUT_3V3 1500 1600"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-09/design.SchDoc -c "add-power GND 1200 800 ground up"
ASSERT: exit 0
```

### Step 4.5: Wire LDO VIN to input rail

```
RUN: altium-cli edit work/scenario-09/design.SchDoc -c "route U1.VIN %VIN_5V"
ASSERT: exit 0
```

### Step 4.6: Wire LDO VOUT to output rail

```
RUN: altium-cli edit work/scenario-09/design.SchDoc -c "route U1.VOUT %VOUT_3V3"
ASSERT: exit 0
```

### Step 4.7: Wire LDO GND

```
RUN: altium-cli edit work/scenario-09/design.SchDoc -c "route U1.GND @GND"
ASSERT: exit 0
```

### Step 4.8: Wire enable through pull-up to VIN

```
RUN: altium-cli edit work/scenario-09/design.SchDoc -c "route R1.1 %VIN_5V"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-09/design.SchDoc -c "route R1.2 U1.EN"
ASSERT: exit 0
```

### Step 4.9: Input cap on VIN rail

```
RUN: altium-cli edit work/scenario-09/design.SchDoc -c "route C1.1 %VIN_5V"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-09/design.SchDoc -c "route C1.2 @GND"
ASSERT: exit 0
```

### Step 4.10: Output cap on VOUT rail

```
RUN: altium-cli edit work/scenario-09/design.SchDoc -c "route C2.1 %VOUT_3V3"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-09/design.SchDoc -c "route C2.2 @GND"
ASSERT: exit 0
```

### Step 4.11: Junctions

```
RUN: altium-cli edit work/scenario-09/design.SchDoc -c "add-missing-junctions"
ASSERT: exit 0
```

---

## Phase 5 — Validation

### Step 5.1

```
RUN: altium-cli edit work/scenario-09/design.SchDoc -c "validate"
ASSERT: exit 0
```

### Step 5.2: VIN_5V net should have 3 pins (U1.VIN, R1.1, C1.1)

```
RUN: altium-cli schdoc netlist work/scenario-09/design.SchDoc --filter VIN_5V --json
ASSERT: exit 0
ASSERT: json net "VIN_5V" has >= 3 pins
```

### Step 5.3: VOUT_3V3 net should have 2 pins (U1.VOUT, C2.1)

```
RUN: altium-cli schdoc netlist work/scenario-09/design.SchDoc --filter VOUT_3V3 --json
ASSERT: exit 0
ASSERT: json net "VOUT_3V3" has >= 2 pins
```

### Step 5.4: BOM

```
RUN: altium-cli schdoc bom work/scenario-09/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 4
```

### Step 5.5: Power map

```
RUN: altium-cli schdoc power-map work/scenario-09/design.SchDoc --json
ASSERT: exit 0
```

### MANUAL CHECKPOINT B

Open `work/scenario-09/design.SchDoc` in Altium.
**Check:** VIN_5V label on left (input) side, VOUT_3V3 on right (output) side. C1 on input, C2 on output. R1 pull-up from VIN to EN.

---

## Phase 6 — Project

### Step 6.1

```
RUN: altium-cli prjpcb create work/scenario-09/project.PrjPcb --name "LDO Regulator"
ASSERT: exit 0
```

### Step 6.2

```
RUN: altium-cli prjpcb add-document work/scenario-09/project.PrjPcb work/scenario-09/design.SchDoc
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-09/project.PrjPcb work/scenario-09/parts.SchLib
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-09/project.PrjPcb work/scenario-09/fps.PcbLib
ASSERT: exit 0
```

---

## Phase 7 — PCB Setup

### Step 7.1

```
RUN: altium-cli pcbdoc create work/scenario-09/board.PcbDoc
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc set-outline-rect work/scenario-09/board.PcbDoc 591 394
ASSERT: exit 0
```

### Step 7.2

```
RUN: altium-cli pcbdoc set-settings work/scenario-09/board.PcbDoc --imperial --grid-size 25 --track-width 12
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-09/board.PcbDoc "Clearance" --value 8
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-09/board.PcbDoc "MinTrackWidth" --value 8
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-09/board.PcbDoc "PowerTrackWidth" --value 20
ASSERT: exit 0
```

### Step 7.3: Verify 3 rules including power track width

```
RUN: altium-cli pcbdoc rules work/scenario-09/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json .total_rules >= 3
```

### Step 7.4

```
RUN: altium-cli prjpcb add-document work/scenario-09/project.PrjPcb work/scenario-09/board.PcbDoc
ASSERT: exit 0
```

---

## Phase 8 — Import

### Step 8.1

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-09/project.PrjPcb --dry-run
ASSERT: exit 0
ASSERT: stdout contains "U1"
ASSERT: stdout contains "C1"
ASSERT: stdout contains "C2"
ASSERT: stdout contains "R1"
```

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-09/project.PrjPcb
ASSERT: exit 0
```

### Step 8.2

```
RUN: altium-cli pcbdoc components work/scenario-09/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes designator "U1"
ASSERT: json output includes designator "C1"
ASSERT: json output includes designator "C2"
ASSERT: json output includes designator "R1"
```

### Step 8.3: Verify power rail nets

```
RUN: altium-cli pcbdoc nets work/scenario-09/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes net "VIN_5V"
ASSERT: json output includes net "VOUT_3V3"
ASSERT: json output includes net "GND"
```

### MANUAL CHECKPOINT C

Open `work/scenario-09/board.PcbDoc` in Altium.
**Check:** U1 is SOT-223 with tab pad. C1 and C2 are 0805. R1 is 0402. Three distinct power nets in ratsnest (VIN_5V, VOUT_3V3, GND).

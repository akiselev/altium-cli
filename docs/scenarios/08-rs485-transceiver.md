# Scenario 08 — RS-485 Transceiver

RS-485 line driver IC with bus termination resistor and TVS protection diode.
Tests differential pair net naming (A/B), multi-function IC with enable pins,
and protection component placement.

**Parts:** 5 (U1: MAX485 SOIC-8, R1: 120R termination, R2: 10k pull-up on A,
R3: 10k pull-down on B, C1: 100nF bypass)

---

## Phase 1 — Part Selection

### Step 1.1

```
RUN: datasheet-cli search "MAX485" --package SOIC-8
ASSERT: exit 0
ASSERT: stdout contains "MAX485"
```

### Step 1.2

```
RUN: datasheet-cli pinout "MAX485ESA"
ASSERT: exit 0
ASSERT: stdout contains "RO" or stdout contains "receiver"
ASSERT: stdout contains "DI" or stdout contains "driver"
```

**Record:** U1 = MAX485, SOIC-8. Pins: RO(1), RE(2), DE(3), DI(4), GND(5), A(6), B(7), VCC(8).

---

## Phase 2 — Schematic Library

### Step 2.1: Create

```
RUN: altium-cli schlib create work/scenario-08/parts.SchLib
ASSERT: exit 0
```

### Step 2.2: Generate MAX485 symbol (8 pins)

```
RUN: altium-cli schlib gen-ic work/scenario-08/parts.SchLib MAX485 --pins "RO,RE_N,DE,DI,GND,A,B,VCC" --description "RS-485 Transceiver"
ASSERT: exit 0
```

### Step 2.3: Verify 8 pins

```
RUN: altium-cli schlib component work/scenario-08/parts.SchLib MAX485 --json
ASSERT: exit 0
ASSERT: json .pin_count == 8
```

### Step 2.4: Add resistor and cap symbols

```
RUN: altium-cli schlib add-component work/scenario-08/parts.SchLib RES --description "Resistor"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-08/parts.SchLib RES 1 "1" -50 0 --direction right --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-08/parts.SchLib RES 2 "2" 50 0 --direction left --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-rectangle work/scenario-08/parts.SchLib RES -30 -10 30 10
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-component work/scenario-08/parts.SchLib CAP --description "Capacitor"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-08/parts.SchLib CAP 1 "1" 0 50 --direction down --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-08/parts.SchLib CAP 2 "2" 0 -50 --direction up --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-line work/scenario-08/parts.SchLib CAP -20 10 20 10
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-line work/scenario-08/parts.SchLib CAP -20 -10 20 -10
ASSERT: exit 0
```

### Step 2.5: Verify 3 components

```
RUN: altium-cli schlib info work/scenario-08/parts.SchLib --json
ASSERT: exit 0
ASSERT: json .component_count == 3
```

---

## Phase 3 — Footprint Library

### Step 3.1: Create

```
RUN: altium-cli pcblib create work/scenario-08/fps.PcbLib
ASSERT: exit 0
```

### Step 3.2: SOIC-8

```
RUN: altium-cli pcblib add-footprint work/scenario-08/fps.PcbLib SOIC-8 --description "SOIC-8"
ASSERT: exit 0
```

```
RUN: altium-cli pcblib add-dual-row work/scenario-08/fps.PcbLib SOIC-8 8 --pitch 50 --span 244
ASSERT: exit 0
```

### Step 3.3: Verify SOIC-8

```
RUN: altium-cli pcblib footprint work/scenario-08/fps.PcbLib SOIC-8 --json
ASSERT: exit 0
ASSERT: json .pad_count == 8
```

### Step 3.4: 0402 for passives

```
RUN: altium-cli pcblib gen-chip work/scenario-08/fps.PcbLib P0402 --size 0402
ASSERT: exit 0
```

---

## Phase 4 — Schematic Entry

### Step 4.1: Create

```
RUN: altium-cli schdoc create work/scenario-08/design.SchDoc
ASSERT: exit 0
```

### Step 4.2: Place components

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "add-component work/scenario-08/parts.SchLib MAX485 1200 1400 U1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "add-component work/scenario-08/parts.SchLib RES 1700 1600 R1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "add-component work/scenario-08/parts.SchLib RES 1700 1800 R2"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "add-component work/scenario-08/parts.SchLib RES 1700 1200 R3"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "add-component work/scenario-08/parts.SchLib CAP 900 1200 C1"
ASSERT: exit 0
```

### Step 4.3: Verify 5 components

```
RUN: altium-cli schdoc components work/scenario-08/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 5
```

### Step 4.4: Power and bus labels

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "add-power VCC 1200 2000 bar down"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "add-power GND 1200 800 ground up"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "add-net-label RS485_A 1600 1600"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "add-net-label RS485_B 1600 1200"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "add-net-label TX_DATA 800 1400"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "add-net-label RX_DATA 800 1600"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "add-net-label TX_EN 800 1200"
ASSERT: exit 0
```

### Step 4.5: Wire IC power

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "route U1.VCC @VCC"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "route U1.GND @GND"
ASSERT: exit 0
```

### Step 4.6: Wire data lines

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "route U1.DI %TX_DATA"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "route U1.RO %RX_DATA"
ASSERT: exit 0
```

### Step 4.7: Wire enable pins (tie DE and RE_N together for half-duplex)

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "route U1.DE %TX_EN"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "route U1.RE_N %TX_EN"
ASSERT: exit 0
```

### Step 4.8: Wire differential bus

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "route U1.A %RS485_A"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "route U1.B %RS485_B"
ASSERT: exit 0
```

### Step 4.9: Wire termination resistor across A-B

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "route R1.1 %RS485_A"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "route R1.2 %RS485_B"
ASSERT: exit 0
```

### Step 4.10: Wire bias resistors (R2: A pull-up, R3: B pull-down)

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "route R2.1 @VCC"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "route R2.2 %RS485_A"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "route R3.1 %RS485_B"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "route R3.2 @GND"
ASSERT: exit 0
```

### Step 4.11: Wire bypass cap

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "route C1.1 @VCC"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "route C1.2 @GND"
ASSERT: exit 0
```

### Step 4.12: Junctions

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "add-missing-junctions"
ASSERT: exit 0
```

---

## Phase 5 — Validation

### Step 5.1

```
RUN: altium-cli edit work/scenario-08/design.SchDoc -c "validate"
ASSERT: exit 0
```

### Step 5.2: Verify RS485_A net has 3 pins (U1.A, R1.1, R2.2)

```
RUN: altium-cli schdoc netlist work/scenario-08/design.SchDoc --filter RS485_A --json
ASSERT: exit 0
ASSERT: json net "RS485_A" has >= 3 pins
```

### Step 5.3: Verify RS485_B net has 3 pins (U1.B, R1.2, R3.1)

```
RUN: altium-cli schdoc netlist work/scenario-08/design.SchDoc --filter RS485_B --json
ASSERT: exit 0
ASSERT: json net "RS485_B" has >= 3 pins
```

### Step 5.4: Verify TX_EN net ties DE and RE_N together

```
RUN: altium-cli schdoc netlist work/scenario-08/design.SchDoc --filter TX_EN --json
ASSERT: exit 0
ASSERT: json net "TX_EN" has >= 2 pins
```

### Step 5.5: BOM

```
RUN: altium-cli schdoc bom work/scenario-08/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 5
```

### MANUAL CHECKPOINT A

Open `work/scenario-08/design.SchDoc` in Altium.
**Check:** RS485_A and RS485_B labels visible on bus side. R1 bridges A-B (termination). DE and RE_N share TX_EN label.

---

## Phase 6 — Project

### Step 6.1

```
RUN: altium-cli prjpcb create work/scenario-08/project.PrjPcb --name "RS-485 Interface"
ASSERT: exit 0
```

### Step 6.2

```
RUN: altium-cli prjpcb add-document work/scenario-08/project.PrjPcb work/scenario-08/design.SchDoc
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-08/project.PrjPcb work/scenario-08/parts.SchLib
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-08/project.PrjPcb work/scenario-08/fps.PcbLib
ASSERT: exit 0
```

---

## Phase 7 — PCB Setup

### Step 7.1

```
RUN: altium-cli pcbdoc create work/scenario-08/board.PcbDoc
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc set-outline-rect work/scenario-08/board.PcbDoc 787 394
ASSERT: exit 0
```

### Step 7.2

```
RUN: altium-cli pcbdoc set-settings work/scenario-08/board.PcbDoc --imperial --grid-size 25 --track-width 10
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-08/board.PcbDoc "Clearance" --value 8
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-08/board.PcbDoc "MinTrackWidth" --value 8
ASSERT: exit 0
```

### Step 7.3

```
RUN: altium-cli prjpcb add-document work/scenario-08/project.PrjPcb work/scenario-08/board.PcbDoc
ASSERT: exit 0
```

---

## Phase 8 — Import

### Step 8.1

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-08/project.PrjPcb --dry-run
ASSERT: exit 0
ASSERT: stdout contains "U1"
```

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-08/project.PrjPcb
ASSERT: exit 0
```

### Step 8.2: Verify all 5 components

```
RUN: altium-cli pcbdoc components work/scenario-08/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes designator "U1"
ASSERT: json output includes designator "R1"
ASSERT: json output includes designator "R2"
ASSERT: json output includes designator "R3"
ASSERT: json output includes designator "C1"
```

### Step 8.3: Verify differential pair nets

```
RUN: altium-cli pcbdoc nets work/scenario-08/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes net "RS485_A"
ASSERT: json output includes net "RS485_B"
ASSERT: json output includes net "TX_DATA"
ASSERT: json output includes net "RX_DATA"
ASSERT: json output includes net "TX_EN"
```

### MANUAL CHECKPOINT B

Open `work/scenario-08/board.PcbDoc` in Altium.
**Check:** U1 is SOIC-8. Four 0402 passives. Ratsnest shows RS485_A and RS485_B nets connecting U1 to termination resistor R1.

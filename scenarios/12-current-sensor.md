# Scenario 12 — INA219 Current Sensor

INA219 high-side current sensor with shunt resistor, I2C bus, and bypass cap.
Tests precision analog + digital mixed design, Kelvin-sense net topology (shunt
resistor has 4 nets: 2 high-current + 2 sense), and I2C address configuration.

**Parts:** 6 (U1: INA219 MSOP-8, R_SHUNT: 100mR 2512, R1: 4.7k SDA pull-up,
R2: 4.7k SCL pull-up, C1: 100nF bypass, J1: 1x4 I2C header)

---

## Phase 1 — Part Selection

### Step 1.1

```
RUN: datasheet-cli search "INA219" --package MSOP-8
ASSERT: exit 0
ASSERT: stdout contains "INA219"
```

### Step 1.2

```
RUN: datasheet-cli pinout "INA219AIDR"
ASSERT: exit 0
ASSERT: stdout contains "IN+" or stdout contains "VIN"
ASSERT: stdout contains "SDA"
```

**Record:** U1 = INA219, MSOP-8. Pins: A1(1), A0(2), SDA(3), SCL(4), GND(5), VS(6), IN-(7), IN+(8). I2C address set by A0/A1.

---

## Phase 2 — Schematic Library

### Step 2.1: Create

```
RUN: altium-cli schlib create work/scenario-12/parts.SchLib
ASSERT: exit 0
```

### Step 2.2: Generate INA219 symbol (8 pins)

```
RUN: altium-cli schlib gen-ic work/scenario-12/parts.SchLib INA219 --pins "A1,A0,SDA,SCL,GND,VS,INM,INP" --description "I2C Current/Power Monitor"
ASSERT: exit 0
```

### Step 2.3: Verify 8 pins

```
RUN: altium-cli schlib component work/scenario-12/parts.SchLib INA219 --json
ASSERT: exit 0
ASSERT: json .pin_count == 8
```

### Step 2.4: Add passive symbols

```
RUN: altium-cli schlib add-component work/scenario-12/parts.SchLib RES --description "Resistor"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-12/parts.SchLib RES 1 "1" -50 0 --direction right --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-12/parts.SchLib RES 2 "2" 50 0 --direction left --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-rectangle work/scenario-12/parts.SchLib RES -30 -10 30 10
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-component work/scenario-12/parts.SchLib CAP --description "Capacitor"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-12/parts.SchLib CAP 1 "1" 0 50 --direction down --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-12/parts.SchLib CAP 2 "2" 0 -50 --direction up --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-line work/scenario-12/parts.SchLib CAP -20 10 20 10
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-line work/scenario-12/parts.SchLib CAP -20 -10 20 -10
ASSERT: exit 0
```

### Step 2.5: Generate 1x4 header

```
RUN: altium-cli schlib gen-ic work/scenario-12/parts.SchLib HDR4 --pins "VCC,GND,SDA,SCL" --description "1x4 I2C Header"
ASSERT: exit 0
```

### Step 2.6: Verify 4 library components

```
RUN: altium-cli schlib info work/scenario-12/parts.SchLib --json
ASSERT: exit 0
ASSERT: json .component_count == 4
```

---

## Phase 3 — Footprint Library

### Step 3.1: Create

```
RUN: altium-cli pcblib create work/scenario-12/fps.PcbLib
ASSERT: exit 0
```

### Step 3.2: MSOP-8 for INA219

```
RUN: altium-cli pcblib add-footprint work/scenario-12/fps.PcbLib MSOP-8 --description "MSOP-8 3x3mm"
ASSERT: exit 0
```

```
RUN: altium-cli pcblib add-dual-row work/scenario-12/fps.PcbLib MSOP-8 8 --pitch 26 --span 197
ASSERT: exit 0
```

### Step 3.3: Verify MSOP-8 has 8 pads

```
RUN: altium-cli pcblib footprint work/scenario-12/fps.PcbLib MSOP-8 --json
ASSERT: exit 0
ASSERT: json .pad_count == 8
```

### Step 3.4: 2512 for shunt resistor (large current-handling package)

```
RUN: altium-cli pcblib gen-chip work/scenario-12/fps.PcbLib R2512 --size 2512
ASSERT: exit 0
```

### Step 3.5: Verify 2512 has 2 pads

```
RUN: altium-cli pcblib footprint work/scenario-12/fps.PcbLib R2512 --json
ASSERT: exit 0
ASSERT: json .pad_count == 2
```

### Step 3.6: Verify 2512 is larger than 0402

```
RUN: altium-cli pcblib measure work/scenario-12/fps.PcbLib R2512 --json
ASSERT: exit 0
ASSERT: json .dimensions.width.mils > 100
```

### Step 3.7: 0402 for pull-ups and bypass

```
RUN: altium-cli pcblib gen-chip work/scenario-12/fps.PcbLib P0402 --size 0402
ASSERT: exit 0
```

### Step 3.8: 1x4 header

```
RUN: altium-cli pcblib add-footprint work/scenario-12/fps.PcbLib HDR-1X4 --description "1x4 header"
ASSERT: exit 0
```

```
RUN: altium-cli pcblib add-pad-row work/scenario-12/fps.PcbLib HDR-1X4 4 --pitch 100
ASSERT: exit 0
```

### Step 3.9: Verify 4 footprints

```
RUN: altium-cli pcblib list work/scenario-12/fps.PcbLib --json
ASSERT: exit 0
ASSERT: json .total_footprints == 4
```

### MANUAL CHECKPOINT A

Open `work/scenario-12/fps.PcbLib` in Altium → R2512.
**Check:** Two large pads — visibly bigger than P0402 footprint. This is the power shunt.

---

## Phase 4 — Schematic Entry

### Step 4.1: Create

```
RUN: altium-cli schdoc create work/scenario-12/design.SchDoc
ASSERT: exit 0
```

### Step 4.2: Place components

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "add-component work/scenario-12/parts.SchLib INA219 1200 1400 U1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "add-component work/scenario-12/parts.SchLib RES 1200 1900 R_SHUNT"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "add-component work/scenario-12/parts.SchLib RES 800 1700 R1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "add-component work/scenario-12/parts.SchLib RES 1000 1700 R2"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "add-component work/scenario-12/parts.SchLib CAP 1500 1200 C1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "add-component work/scenario-12/parts.SchLib HDR4 500 1400 J1"
ASSERT: exit 0
```

### Step 4.3: Verify 6 components

```
RUN: altium-cli schdoc components work/scenario-12/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 6
```

### Step 4.4: Power and bus labels

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "add-power VCC 1200 2200 bar down"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "add-power GND 1200 800 ground up"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "add-net-label SDA 800 1500"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "add-net-label SCL 1000 1500"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "add-net-label SENSE_P 1100 1900"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "add-net-label SENSE_N 1300 1900"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "add-net-label LOAD_IN 900 2000"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "add-net-label LOAD_OUT 1500 2000"
ASSERT: exit 0
```

### Step 4.5: Wire shunt resistor in the high-side current path

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route R_SHUNT.1 %SENSE_P"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route R_SHUNT.2 %SENSE_N"
ASSERT: exit 0
```

### Step 4.6: Connect current path through shunt

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route %LOAD_IN R_SHUNT.1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route R_SHUNT.2 %LOAD_OUT"
ASSERT: exit 0
```

### Step 4.7: Wire INA219 sense inputs to shunt

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route U1.INP %SENSE_P"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route U1.INM %SENSE_N"
ASSERT: exit 0
```

### Step 4.8: Wire INA219 power and bus supply

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route U1.VS @VCC"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route U1.GND @GND"
ASSERT: exit 0
```

### Step 4.9: Wire I2C bus

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route U1.SDA %SDA"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route U1.SCL %SCL"
ASSERT: exit 0
```

### Step 4.10: I2C pull-ups

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route R1.1 @VCC"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route R1.2 %SDA"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route R2.1 @VCC"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route R2.2 %SCL"
ASSERT: exit 0
```

### Step 4.11: Address config — A0 and A1 to GND (address 0x40)

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route U1.A0 @GND"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route U1.A1 @GND"
ASSERT: exit 0
```

### Step 4.12: Bypass cap

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route C1.1 @VCC"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route C1.2 @GND"
ASSERT: exit 0
```

### Step 4.13: Header

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route J1.VCC @VCC"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route J1.GND @GND"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route J1.SDA %SDA"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "route J1.SCL %SCL"
ASSERT: exit 0
```

### Step 4.14: Junctions

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "add-missing-junctions"
ASSERT: exit 0
```

---

## Phase 5 — Validation

### Step 5.1

```
RUN: altium-cli edit work/scenario-12/design.SchDoc -c "validate"
ASSERT: exit 0
```

### Step 5.2: Verify SENSE_P net has 2 pins (R_SHUNT.1, U1.INP)

```
RUN: altium-cli schdoc netlist work/scenario-12/design.SchDoc --filter SENSE_P --json
ASSERT: exit 0
ASSERT: json net "SENSE_P" has >= 2 pins
```

### Step 5.3: Verify SENSE_N net has 2 pins (R_SHUNT.2, U1.INM)

```
RUN: altium-cli schdoc netlist work/scenario-12/design.SchDoc --filter SENSE_N --json
ASSERT: exit 0
ASSERT: json net "SENSE_N" has >= 2 pins
```

### Step 5.4: Verify I2C bus nets

```
RUN: altium-cli schdoc netlist work/scenario-12/design.SchDoc --filter SDA --json
ASSERT: exit 0
ASSERT: json net "SDA" has >= 3 pins
```

```
RUN: altium-cli schdoc netlist work/scenario-12/design.SchDoc --filter SCL --json
ASSERT: exit 0
ASSERT: json net "SCL" has >= 3 pins
```

### Step 5.5: BOM has 6 parts

```
RUN: altium-cli schdoc bom work/scenario-12/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 6
```

### Step 5.6: Total nets should be ~8+ (VCC, GND, SDA, SCL, SENSE_P, SENSE_N, LOAD_IN, LOAD_OUT)

```
RUN: altium-cli schdoc netlist work/scenario-12/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_nets >= 8
```

### MANUAL CHECKPOINT B

Open `work/scenario-12/design.SchDoc` in Altium.
**Check:** Shunt resistor R_SHUNT sits in the high-side current path (LOAD_IN → R_SHUNT → LOAD_OUT). INA219 sense inputs (INP/INM) tap both sides of the shunt via SENSE_P/SENSE_N labels. I2C bus with pull-ups goes to header.

---

## Phase 6 — Project

### Step 6.1

```
RUN: altium-cli prjpcb create work/scenario-12/project.PrjPcb --name "Current Sensor"
ASSERT: exit 0
```

### Step 6.2

```
RUN: altium-cli prjpcb add-document work/scenario-12/project.PrjPcb work/scenario-12/design.SchDoc
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-12/project.PrjPcb work/scenario-12/parts.SchLib
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-12/project.PrjPcb work/scenario-12/fps.PcbLib
ASSERT: exit 0
```

---

## Phase 7 — PCB Setup

### Step 7.1

```
RUN: altium-cli pcbdoc create work/scenario-12/board.PcbDoc
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc set-outline-rect work/scenario-12/board.PcbDoc 787 591
ASSERT: exit 0
```

### Step 7.2

```
RUN: altium-cli pcbdoc set-settings work/scenario-12/board.PcbDoc --imperial --grid-size 25 --track-width 10
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-12/board.PcbDoc "Clearance" --value 6
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-12/board.PcbDoc "MinTrackWidth" --value 6
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-12/board.PcbDoc "PowerTrackWidth" --value 25
ASSERT: exit 0
```

### Step 7.3: Verify power track rule exists

```
RUN: altium-cli pcbdoc rules work/scenario-12/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json .total_rules >= 3
```

### Step 7.4

```
RUN: altium-cli prjpcb add-document work/scenario-12/project.PrjPcb work/scenario-12/board.PcbDoc
ASSERT: exit 0
```

---

## Phase 8 — Import

### Step 8.1

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-12/project.PrjPcb --dry-run
ASSERT: exit 0
ASSERT: stdout contains "U1"
ASSERT: stdout contains "R_SHUNT"
ASSERT: stdout contains "J1"
```

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-12/project.PrjPcb
ASSERT: exit 0
```

### Step 8.2: Verify all 6 components

```
RUN: altium-cli pcbdoc components work/scenario-12/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes designator "U1"
ASSERT: json output includes designator "R_SHUNT"
ASSERT: json output includes designator "R1"
ASSERT: json output includes designator "R2"
ASSERT: json output includes designator "C1"
ASSERT: json output includes designator "J1"
```

### Step 8.3: Verify sense and bus nets

```
RUN: altium-cli pcbdoc nets work/scenario-12/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes net "SENSE_P"
ASSERT: json output includes net "SENSE_N"
ASSERT: json output includes net "SDA"
ASSERT: json output includes net "SCL"
ASSERT: json output includes net "LOAD_IN"
ASSERT: json output includes net "LOAD_OUT"
```

### MANUAL CHECKPOINT C

Open `work/scenario-12/board.PcbDoc` in Altium.
**Check:** R_SHUNT has large 2512 pads (power path). U1 is MSOP-8 (small pads). J1 is through-hole header. Ratsnest shows SENSE_P/SENSE_N Kelvin connections from shunt to INA219. I2C bus ratsnest connects U1 to J1.

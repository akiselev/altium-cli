# Scenario 07 — N-Channel MOSFET LED Driver

An N-channel MOSFET switches an LED on/off from a logic signal. Tests a 3-pin
device (gate, drain, source), power switching topology, and gate pull-down
resistor.

**Parts:** 4 (Q1: 2N7002 SOT-23, R1: 330R LED limit, R2: 10k gate pull-down,
D1: green LED 0805)

---

## Phase 1 — Part Selection

### Step 1.1

```
RUN: datasheet-cli search "2N7002" --package SOT-23
ASSERT: exit 0
ASSERT: stdout contains "2N7002"
```

### Step 1.2

```
RUN: datasheet-cli pinout "2N7002"
ASSERT: exit 0
ASSERT: stdout contains "Gate" or stdout contains "G"
ASSERT: stdout contains "Drain" or stdout contains "D"
```

**Record:** Q1 = 2N7002, SOT-23-3. Pins: G(1), S(2), D(3). Vgs(th) ~ 2.1V.

---

## Phase 2 — Schematic Library

### Step 2.1: Create

```
RUN: altium-cli schlib create work/scenario-07/parts.SchLib
ASSERT: exit 0
```

### Step 2.2: Generate MOSFET symbol (3 pins)

```
RUN: altium-cli schlib gen-ic work/scenario-07/parts.SchLib NMOS --pins "G,S,D" --description "N-Ch MOSFET"
ASSERT: exit 0
```

### Step 2.3: Verify 3 pins

```
RUN: altium-cli schlib component work/scenario-07/parts.SchLib NMOS --json
ASSERT: exit 0
ASSERT: json .pin_count == 3
```

### Step 2.4: Add resistor, LED symbols

```
RUN: altium-cli schlib add-component work/scenario-07/parts.SchLib RES --description "Resistor"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-07/parts.SchLib RES 1 "1" -50 0 --direction right --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-07/parts.SchLib RES 2 "2" 50 0 --direction left --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-rectangle work/scenario-07/parts.SchLib RES -30 -10 30 10
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-component work/scenario-07/parts.SchLib LED --description "LED"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-07/parts.SchLib LED A "Anode" -50 0 --direction right --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-07/parts.SchLib LED K "Cathode" 50 0 --direction left --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-rectangle work/scenario-07/parts.SchLib LED -30 -20 30 20
ASSERT: exit 0
```

### Step 2.5: Verify 3 symbols in library

```
RUN: altium-cli schlib info work/scenario-07/parts.SchLib --json
ASSERT: exit 0
ASSERT: json .component_count == 3
```

---

## Phase 3 — Footprint Library

### Step 3.1: Create

```
RUN: altium-cli pcblib create work/scenario-07/fps.PcbLib
ASSERT: exit 0
```

### Step 3.2: SOT-23-3 for MOSFET

```
RUN: altium-cli pcblib add-footprint work/scenario-07/fps.PcbLib SOT-23-3 --description "SOT-23 3-Lead"
ASSERT: exit 0
```

```
RUN: altium-cli pcblib add-dual-row work/scenario-07/fps.PcbLib SOT-23-3 3 --pitch 38 --span 102
ASSERT: exit 0
```

### Step 3.3: Verify SOT-23-3 has 3 pads

```
RUN: altium-cli pcblib footprint work/scenario-07/fps.PcbLib SOT-23-3 --json
ASSERT: exit 0
ASSERT: json .pad_count == 3
```

### Step 3.4: 0402 for resistors

```
RUN: altium-cli pcblib gen-chip work/scenario-07/fps.PcbLib R0402 --size 0402
ASSERT: exit 0
```

### Step 3.5: 0805 for LED

```
RUN: altium-cli pcblib gen-chip work/scenario-07/fps.PcbLib LED0805 --size 0805
ASSERT: exit 0
```

### Step 3.6: Verify 3 footprints

```
RUN: altium-cli pcblib list work/scenario-07/fps.PcbLib --json
ASSERT: exit 0
ASSERT: json .total_footprints == 3
```

---

## Phase 4 — Schematic Entry

### Step 4.1: Create

```
RUN: altium-cli schdoc create work/scenario-07/design.SchDoc
ASSERT: exit 0
```

### Step 4.2: Place components

```
RUN: altium-cli edit work/scenario-07/design.SchDoc -c "add-component work/scenario-07/parts.SchLib NMOS 1200 1000 Q1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-07/design.SchDoc -c "add-component work/scenario-07/parts.SchLib RES 1200 1600 R1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-07/design.SchDoc -c "add-component work/scenario-07/parts.SchLib RES 800 1000 R2"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-07/design.SchDoc -c "add-component work/scenario-07/parts.SchLib LED 1200 1900 D1"
ASSERT: exit 0
```

### Step 4.3: Verify 4 components

```
RUN: altium-cli schdoc components work/scenario-07/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 4
```

### Step 4.4: Power and signal labels

```
RUN: altium-cli edit work/scenario-07/design.SchDoc -c "add-power VCC 1200 2200 bar down"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-07/design.SchDoc -c "add-power GND 1200 600 ground up"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-07/design.SchDoc -c "add-net-label GATE_IN 600 1000"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-07/design.SchDoc -c "add-net-label LED_DRIVE 1200 1400"
ASSERT: exit 0
```

### Step 4.5: Wire LED circuit — VCC → D1.A → D1.K → R1 → MOSFET drain

```
RUN: altium-cli edit work/scenario-07/design.SchDoc -c "route D1.A @VCC"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-07/design.SchDoc -c "route D1.K R1.1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-07/design.SchDoc -c "route R1.2 %LED_DRIVE"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-07/design.SchDoc -c "route Q1.D %LED_DRIVE"
ASSERT: exit 0
```

### Step 4.6: Wire MOSFET source to GND

```
RUN: altium-cli edit work/scenario-07/design.SchDoc -c "route Q1.S @GND"
ASSERT: exit 0
```

### Step 4.7: Wire gate with pull-down

```
RUN: altium-cli edit work/scenario-07/design.SchDoc -c "route Q1.G %GATE_IN"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-07/design.SchDoc -c "route R2.1 %GATE_IN"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-07/design.SchDoc -c "route R2.2 @GND"
ASSERT: exit 0
```

### Step 4.8: Junctions

```
RUN: altium-cli edit work/scenario-07/design.SchDoc -c "add-missing-junctions"
ASSERT: exit 0
```

---

## Phase 5 — Validation

### Step 5.1

```
RUN: altium-cli edit work/scenario-07/design.SchDoc -c "validate"
ASSERT: exit 0
```

### Step 5.2: Verify LED_DRIVE net connects R1.2 and Q1.D

```
RUN: altium-cli schdoc netlist work/scenario-07/design.SchDoc --filter LED_DRIVE --json
ASSERT: exit 0
ASSERT: json net "LED_DRIVE" has >= 2 pins
```

### Step 5.3: Verify GATE_IN net connects Q1.G and R2.1

```
RUN: altium-cli schdoc netlist work/scenario-07/design.SchDoc --filter GATE_IN --json
ASSERT: exit 0
ASSERT: json net "GATE_IN" has >= 2 pins
```

### Step 5.4: BOM

```
RUN: altium-cli schdoc bom work/scenario-07/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 4
```

### MANUAL CHECKPOINT A

Open `work/scenario-07/design.SchDoc` in Altium.
**Check:** Current path is VCC → D1 → R1 → Q1 drain → Q1 source → GND. Gate has pull-down R2 to GND.

---

## Phase 6 — Project

### Step 6.1

```
RUN: altium-cli prjpcb create work/scenario-07/project.PrjPcb --name "LED Driver"
ASSERT: exit 0
```

### Step 6.2

```
RUN: altium-cli prjpcb add-document work/scenario-07/project.PrjPcb work/scenario-07/design.SchDoc
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-07/project.PrjPcb work/scenario-07/parts.SchLib
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-07/project.PrjPcb work/scenario-07/fps.PcbLib
ASSERT: exit 0
```

---

## Phase 7 — PCB Setup

### Step 7.1

```
RUN: altium-cli pcbdoc create work/scenario-07/board.PcbDoc
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc set-outline-rect work/scenario-07/board.PcbDoc 591 394
ASSERT: exit 0
```

### Step 7.2

```
RUN: altium-cli pcbdoc set-settings work/scenario-07/board.PcbDoc --imperial --grid-size 25 --track-width 10
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-07/board.PcbDoc "Clearance" --value 8
ASSERT: exit 0
```

### Step 7.3

```
RUN: altium-cli prjpcb add-document work/scenario-07/project.PrjPcb work/scenario-07/board.PcbDoc
ASSERT: exit 0
```

---

## Phase 8 — Import

### Step 8.1

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-07/project.PrjPcb --dry-run
ASSERT: exit 0
ASSERT: stdout contains "Q1"
ASSERT: stdout contains "R1"
ASSERT: stdout contains "D1"
```

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-07/project.PrjPcb
ASSERT: exit 0
```

### Step 8.2

```
RUN: altium-cli pcbdoc components work/scenario-07/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes designator "Q1"
ASSERT: json output includes designator "R1"
ASSERT: json output includes designator "R2"
ASSERT: json output includes designator "D1"
```

```
RUN: altium-cli pcbdoc nets work/scenario-07/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes net "GATE_IN"
ASSERT: json output includes net "LED_DRIVE"
ASSERT: json output includes net "VCC"
ASSERT: json output includes net "GND"
```

### MANUAL CHECKPOINT B

Open `work/scenario-07/board.PcbDoc` in Altium.
**Check:** Q1 is SOT-23-3 (3 pads), D1 is 0805, R1 and R2 are 0402. Four distinct ratsnest clusters visible.

# Scenario 06 — Op-Amp Unity-Gain Buffer

Op-amp in unity-gain (voltage follower) configuration with dual supply bypass
caps. Tests analog component wiring, feedback connection (output wired back to
inverting input), and +/- power supply nets.

**Parts:** 4 (U1: OPA340 SOT-23-5, C1: 100nF V+ bypass, C2: 100nF V- bypass,
R1: 100R output series resistor)

---

## Phase 1 — Part Selection

### Step 1.1

```
RUN: datasheet-cli search "OPA340" --package SOT-23-5
ASSERT: exit 0
ASSERT: stdout contains "OPA340"
```

### Step 1.2

```
RUN: datasheet-cli pinout "OPA340NA"
ASSERT: exit 0
ASSERT: stdout contains "IN+" or stdout contains "non-inverting"
ASSERT: stdout contains "OUT"
```

**Record:** U1 = OPA340, SOT-23-5. Pins: OUT(1), VS-(2), IN+(3), IN-(4), VS+(5).

---

## Phase 2 — Schematic Library

### Step 2.1: Create

```
RUN: altium-cli schlib create work/scenario-06/parts.SchLib
ASSERT: exit 0
```

### Step 2.2: Generate op-amp symbol (5 pins)

```
RUN: altium-cli schlib gen-ic work/scenario-06/parts.SchLib OPA340 --pins "OUT,VS_NEG,IN_POS,IN_NEG,VS_POS" --description "CMOS Op-Amp"
ASSERT: exit 0
```

### Step 2.3: Verify 5 pins

```
RUN: altium-cli schlib component work/scenario-06/parts.SchLib OPA340 --json
ASSERT: exit 0
ASSERT: json .pin_count == 5
```

### Step 2.4: Add passive symbols (resistor + capacitor)

```
RUN: altium-cli schlib add-component work/scenario-06/parts.SchLib RES --description "Resistor"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-06/parts.SchLib RES 1 "1" -50 0 --direction right --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-06/parts.SchLib RES 2 "2" 50 0 --direction left --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-rectangle work/scenario-06/parts.SchLib RES -30 -10 30 10
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-component work/scenario-06/parts.SchLib CAP --description "Capacitor"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-06/parts.SchLib CAP 1 "1" 0 50 --direction down --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-06/parts.SchLib CAP 2 "2" 0 -50 --direction up --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-line work/scenario-06/parts.SchLib CAP -20 10 20 10
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-line work/scenario-06/parts.SchLib CAP -20 -10 20 -10
ASSERT: exit 0
```

### Step 2.5: Verify 3 components

```
RUN: altium-cli schlib info work/scenario-06/parts.SchLib --json
ASSERT: exit 0
ASSERT: json .component_count == 3
```

---

## Phase 3 — Footprint Library

### Step 3.1: Create

```
RUN: altium-cli pcblib create work/scenario-06/fps.PcbLib
ASSERT: exit 0
```

### Step 3.2: SOT-23-5 (3 pins one side, 2 the other)

```
RUN: altium-cli pcblib add-footprint work/scenario-06/fps.PcbLib SOT-23-5 --description "SOT-23 5-Lead"
ASSERT: exit 0
```

```
RUN: altium-cli pcblib add-dual-row work/scenario-06/fps.PcbLib SOT-23-5 5 --pitch 38 --span 102
ASSERT: exit 0
```

### Step 3.3: Verify SOT-23-5 has 5 pads

```
RUN: altium-cli pcblib footprint work/scenario-06/fps.PcbLib SOT-23-5 --json
ASSERT: exit 0
ASSERT: json .pad_count == 5
```

### Step 3.4: 0402 for passives

```
RUN: altium-cli pcblib gen-chip work/scenario-06/fps.PcbLib P0402 --size 0402
ASSERT: exit 0
```

### Step 3.5: Verify

```
RUN: altium-cli pcblib list work/scenario-06/fps.PcbLib --json
ASSERT: exit 0
ASSERT: json .total_footprints == 2
```

---

## Phase 4 — Schematic Entry

### Step 4.1: Create schematic

```
RUN: altium-cli schdoc create work/scenario-06/design.SchDoc
ASSERT: exit 0
```

### Step 4.2: Place components

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "add-component work/scenario-06/parts.SchLib OPA340 1200 1400 U1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "add-component work/scenario-06/parts.SchLib RES 1700 1400 R1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "add-component work/scenario-06/parts.SchLib CAP 900 1700 C1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "add-component work/scenario-06/parts.SchLib CAP 900 1100 C2"
ASSERT: exit 0
```

### Step 4.3: Verify 4 components

```
RUN: altium-cli schdoc components work/scenario-06/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 4
```

### Step 4.4: Power symbols for dual supply

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "add-power V+ 1200 2000 bar down"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "add-power V- 1200 800 ground up"
ASSERT: exit 0
```

### Step 4.5: Signal net labels

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "add-net-label VIN 800 1400"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "add-net-label VOUT 2000 1400"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "add-net-label FB 1500 1200"
ASSERT: exit 0
```

### Step 4.6: Wire op-amp power

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "route U1.VS_POS @V+"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "route U1.VS_NEG @V-"
ASSERT: exit 0
```

### Step 4.7: Wire input

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "route U1.IN_POS %VIN"
ASSERT: exit 0
```

### Step 4.8: Wire output through series resistor

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "route U1.OUT R1.1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "route R1.2 %VOUT"
ASSERT: exit 0
```

### Step 4.9: Wire feedback — output back to IN- (unity gain)

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "add-net-label FB 1200 1200"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "route U1.OUT %FB"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "route U1.IN_NEG %FB"
ASSERT: exit 0
```

### Step 4.10: Wire bypass caps

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "route C1.1 @V+"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "route C1.2 @V-"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "route C2.1 @V+"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "route C2.2 @V-"
ASSERT: exit 0
```

### Step 4.11: Junctions

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "add-missing-junctions"
ASSERT: exit 0
```

---

## Phase 5 — Validation

### Step 5.1: Validate

```
RUN: altium-cli edit work/scenario-06/design.SchDoc -c "validate"
ASSERT: exit 0
```

### Step 5.2: Verify feedback net connects OUT and IN-

```
RUN: altium-cli schdoc netlist work/scenario-06/design.SchDoc --filter FB --json
ASSERT: exit 0
ASSERT: json net "FB" has >= 2 pins
```

### Step 5.3: BOM

```
RUN: altium-cli schdoc bom work/scenario-06/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 4
```

### MANUAL CHECKPOINT A

Open `work/scenario-06/design.SchDoc` in Altium.
**Check:** Op-amp output connects back to IN- (feedback path visible). Two bypass caps between V+ and V-.

---

## Phase 6 — Project

### Step 6.1

```
RUN: altium-cli prjpcb create work/scenario-06/project.PrjPcb --name "Op-Amp Buffer"
ASSERT: exit 0
```

### Step 6.2

```
RUN: altium-cli prjpcb add-document work/scenario-06/project.PrjPcb work/scenario-06/design.SchDoc
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-06/project.PrjPcb work/scenario-06/parts.SchLib
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-06/project.PrjPcb work/scenario-06/fps.PcbLib
ASSERT: exit 0
```

---

## Phase 7 — PCB Setup

### Step 7.1

```
RUN: altium-cli pcbdoc create work/scenario-06/board.PcbDoc
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc set-outline-rect work/scenario-06/board.PcbDoc 591 394
ASSERT: exit 0
```

### Step 7.2: Settings and rules

```
RUN: altium-cli pcbdoc set-settings work/scenario-06/board.PcbDoc --imperial --grid-size 25 --track-width 8
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-06/board.PcbDoc "Clearance" --value 6
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-06/board.PcbDoc "MinTrackWidth" --value 6
ASSERT: exit 0
```

### Step 7.3: Add to project

```
RUN: altium-cli prjpcb add-document work/scenario-06/project.PrjPcb work/scenario-06/board.PcbDoc
ASSERT: exit 0
```

---

## Phase 8 — Import

### Step 8.1: Import

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-06/project.PrjPcb --dry-run
ASSERT: exit 0
ASSERT: stdout contains "U1"
ASSERT: stdout contains "R1"
ASSERT: stdout contains "C1"
ASSERT: stdout contains "C2"
```

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-06/project.PrjPcb
ASSERT: exit 0
```

### Step 8.2: Verify

```
RUN: altium-cli pcbdoc components work/scenario-06/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes designator "U1"
ASSERT: json output includes designator "R1"
ASSERT: json output includes designator "C1"
ASSERT: json output includes designator "C2"
```

```
RUN: altium-cli pcbdoc nets work/scenario-06/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes net "VIN"
ASSERT: json output includes net "VOUT"
ASSERT: json output includes net "FB"
```

### MANUAL CHECKPOINT B

Open `work/scenario-06/board.PcbDoc` in Altium.
**Check:** U1 has SOT-23-5 footprint (5 pads). Two 0402 caps and one 0402 resistor. Ratsnest shows feedback loop.

# Scenario 11 — Dual Op-Amp Cascaded Gain Stages

Two inverting amplifier stages built from a single dual op-amp IC. Tests the
largest component count (8 parts), cascaded signal path (output of stage 1 feeds
input of stage 2), and shared power supply across both op-amp sections.

**Parts:** 8 (U1: LM358 SOIC-8, R1-R2: stage 1 gain network, R3-R4: stage 2
gain network, C1: AC coupling between stages, C2: input AC coupling,
C3: bypass cap)

---

## Phase 1 — Part Selection

### Step 1.1

```
RUN: datasheet-cli search "LM358" --package SOIC-8
ASSERT: exit 0
ASSERT: stdout contains "LM358"
```

### Step 1.2

```
RUN: datasheet-cli pinout "LM358DR"
ASSERT: exit 0
ASSERT: stdout contains "OUT" or stdout contains "output"
```

**Record:** U1 = LM358, SOIC-8. Pins: OUT1(1), IN1-(2), IN1+(3), GND(4), IN2+(5), IN2-(6), OUT2(7), VCC(8). Dual op-amp, single-supply.

---

## Phase 2 — Schematic Library

### Step 2.1: Create

```
RUN: altium-cli schlib create work/scenario-11/parts.SchLib
ASSERT: exit 0
```

### Step 2.2: Generate LM358 symbol (8 pins)

```
RUN: altium-cli schlib gen-ic work/scenario-11/parts.SchLib LM358 --pins "OUT1,IN1_NEG,IN1_POS,GND,IN2_POS,IN2_NEG,OUT2,VCC" --description "Dual Op-Amp"
ASSERT: exit 0
```

### Step 2.3: Verify 8 pins

```
RUN: altium-cli schlib component work/scenario-11/parts.SchLib LM358 --json
ASSERT: exit 0
ASSERT: json .pin_count == 8
```

### Step 2.4: Add passive symbols

```
RUN: altium-cli schlib add-component work/scenario-11/parts.SchLib RES --description "Resistor"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-11/parts.SchLib RES 1 "1" -50 0 --direction right --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-11/parts.SchLib RES 2 "2" 50 0 --direction left --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-rectangle work/scenario-11/parts.SchLib RES -30 -10 30 10
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-component work/scenario-11/parts.SchLib CAP --description "Capacitor"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-11/parts.SchLib CAP 1 "1" -50 0 --direction right --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-11/parts.SchLib CAP 2 "2" 50 0 --direction left --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-line work/scenario-11/parts.SchLib CAP -10 -20 -10 20
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-line work/scenario-11/parts.SchLib CAP 10 -20 10 20
ASSERT: exit 0
```

### Step 2.5: Verify 3 components

```
RUN: altium-cli schlib info work/scenario-11/parts.SchLib --json
ASSERT: exit 0
ASSERT: json .component_count == 3
```

---

## Phase 3 — Footprint Library

### Step 3.1: Create

```
RUN: altium-cli pcblib create work/scenario-11/fps.PcbLib
ASSERT: exit 0
```

### Step 3.2: SOIC-8 for LM358

```
RUN: altium-cli pcblib add-footprint work/scenario-11/fps.PcbLib SOIC-8 --description "SOIC-8"
ASSERT: exit 0
```

```
RUN: altium-cli pcblib add-dual-row work/scenario-11/fps.PcbLib SOIC-8 8 --pitch 50 --span 244
ASSERT: exit 0
```

### Step 3.3: 0402 for resistors

```
RUN: altium-cli pcblib gen-chip work/scenario-11/fps.PcbLib R0402 --size 0402
ASSERT: exit 0
```

### Step 3.4: 0603 for coupling caps (slightly larger for audio)

```
RUN: altium-cli pcblib gen-chip work/scenario-11/fps.PcbLib C0603 --size 0603
ASSERT: exit 0
```

### Step 3.5: Verify 3 footprints

```
RUN: altium-cli pcblib list work/scenario-11/fps.PcbLib --json
ASSERT: exit 0
ASSERT: json .total_footprints == 3
```

---

## Phase 4 — Schematic Entry

### Step 4.1: Create

```
RUN: altium-cli schdoc create work/scenario-11/design.SchDoc
ASSERT: exit 0
```

### Step 4.2: Place all 8 components

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "add-component work/scenario-11/parts.SchLib LM358 1200 1400 U1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "add-component work/scenario-11/parts.SchLib CAP 700 1400 C2"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "add-component work/scenario-11/parts.SchLib RES 900 1400 R1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "add-component work/scenario-11/parts.SchLib RES 1100 1700 R2"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "add-component work/scenario-11/parts.SchLib CAP 1500 1400 C1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "add-component work/scenario-11/parts.SchLib RES 1700 1400 R3"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "add-component work/scenario-11/parts.SchLib RES 1900 1700 R4"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "add-component work/scenario-11/parts.SchLib CAP 1000 1100 C3"
ASSERT: exit 0
```

### Step 4.3: Verify 8 components

```
RUN: altium-cli schdoc components work/scenario-11/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 8
```

### Step 4.4: Power and signal labels

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "add-power VCC 1200 2000 bar down"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "add-power GND 1200 800 ground up"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "add-net-label AUDIO_IN 600 1400"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "add-net-label STAGE1_IN 900 1300"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "add-net-label STAGE1_OUT 1400 1400"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "add-net-label STAGE1_FB 1100 1600"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "add-net-label STAGE2_IN 1700 1300"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "add-net-label STAGE2_OUT 2100 1400"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "add-net-label STAGE2_FB 1900 1600"
ASSERT: exit 0
```

### Step 4.5: Wire IC power

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route U1.VCC @VCC"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route U1.GND @GND"
ASSERT: exit 0
```

### Step 4.6: Stage 1 — inverting amplifier on op-amp section 1

Input AC coupling: AUDIO_IN → C2 → R1 → IN1_NEG

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route C2.1 %AUDIO_IN"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route C2.2 %STAGE1_IN"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route R1.1 %STAGE1_IN"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route R1.2 U1.IN1_NEG"
ASSERT: exit 0
```

Non-inverting input to GND (AC ground):

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route U1.IN1_POS @GND"
ASSERT: exit 0
```

Feedback resistor R2 from OUT1 back to IN1_NEG:

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route U1.OUT1 %STAGE1_OUT"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route R2.1 %STAGE1_FB"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route R2.2 %STAGE1_OUT"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route U1.IN1_NEG %STAGE1_FB"
ASSERT: exit 0
```

### Step 4.7: Inter-stage AC coupling

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route C1.1 %STAGE1_OUT"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route C1.2 %STAGE2_IN"
ASSERT: exit 0
```

### Step 4.8: Stage 2 — inverting amplifier on op-amp section 2

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route R3.1 %STAGE2_IN"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route R3.2 U1.IN2_NEG"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route U1.IN2_POS @GND"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route U1.OUT2 %STAGE2_OUT"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route R4.1 %STAGE2_FB"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route R4.2 %STAGE2_OUT"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route U1.IN2_NEG %STAGE2_FB"
ASSERT: exit 0
```

### Step 4.9: Bypass cap

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route C3.1 @VCC"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "route C3.2 @GND"
ASSERT: exit 0
```

### Step 4.10: Junctions

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "add-missing-junctions"
ASSERT: exit 0
```

---

## Phase 5 — Validation

### Step 5.1

```
RUN: altium-cli edit work/scenario-11/design.SchDoc -c "validate"
ASSERT: exit 0
```

### Step 5.2: Verify inter-stage coupling — STAGE1_OUT connects OUT1 and C1 and R2

```
RUN: altium-cli schdoc netlist work/scenario-11/design.SchDoc --filter STAGE1_OUT --json
ASSERT: exit 0
ASSERT: json net "STAGE1_OUT" has >= 3 pins
```

### Step 5.3: Verify stage 2 output

```
RUN: altium-cli schdoc netlist work/scenario-11/design.SchDoc --filter STAGE2_OUT --json
ASSERT: exit 0
ASSERT: json net "STAGE2_OUT" has >= 2 pins
```

### Step 5.4: Total net count (expect ~10: VCC, GND, AUDIO_IN, STAGE1_IN, STAGE1_OUT, STAGE1_FB, STAGE2_IN, STAGE2_OUT, STAGE2_FB, plus internal)

```
RUN: altium-cli schdoc netlist work/scenario-11/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_nets >= 8
```

### Step 5.5: BOM

```
RUN: altium-cli schdoc bom work/scenario-11/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 8
```

### Step 5.6: Signal flow from AUDIO_IN through stages

```
RUN: altium-cli schdoc signal-flow work/scenario-11/design.SchDoc AUDIO_IN
ASSERT: exit 0
```

### MANUAL CHECKPOINT A

Open `work/scenario-11/design.SchDoc` in Altium.
**Check:** Two distinct amplifier stages visible. C1 couples STAGE1_OUT to STAGE2_IN. Each stage has a feedback resistor (R2, R4) from output to inverting input.

---

## Phase 6 — Project

### Step 6.1

```
RUN: altium-cli prjpcb create work/scenario-11/project.PrjPcb --name "Dual Op-Amp Gain"
ASSERT: exit 0
```

### Step 6.2

```
RUN: altium-cli prjpcb add-document work/scenario-11/project.PrjPcb work/scenario-11/design.SchDoc
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-11/project.PrjPcb work/scenario-11/parts.SchLib
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-11/project.PrjPcb work/scenario-11/fps.PcbLib
ASSERT: exit 0
```

---

## Phase 7 — PCB Setup

### Step 7.1

```
RUN: altium-cli pcbdoc create work/scenario-11/board.PcbDoc
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc set-outline-rect work/scenario-11/board.PcbDoc 984 591
ASSERT: exit 0
```

### Step 7.2

```
RUN: altium-cli pcbdoc set-settings work/scenario-11/board.PcbDoc --imperial --grid-size 25 --track-width 8
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-11/board.PcbDoc "Clearance" --value 6
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-11/board.PcbDoc "MinTrackWidth" --value 6
ASSERT: exit 0
```

### Step 7.3

```
RUN: altium-cli prjpcb add-document work/scenario-11/project.PrjPcb work/scenario-11/board.PcbDoc
ASSERT: exit 0
```

---

## Phase 8 — Import

### Step 8.1

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-11/project.PrjPcb --dry-run
ASSERT: exit 0
ASSERT: stdout contains "U1"
ASSERT: stdout contains "R1"
ASSERT: stdout contains "R2"
ASSERT: stdout contains "R3"
ASSERT: stdout contains "R4"
ASSERT: stdout contains "C1"
ASSERT: stdout contains "C2"
ASSERT: stdout contains "C3"
```

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-11/project.PrjPcb
ASSERT: exit 0
```

### Step 8.2: Verify all 8 components

```
RUN: altium-cli pcbdoc components work/scenario-11/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes designator "U1"
ASSERT: json output includes designator "R1"
ASSERT: json output includes designator "R2"
ASSERT: json output includes designator "R3"
ASSERT: json output includes designator "R4"
ASSERT: json output includes designator "C1"
ASSERT: json output includes designator "C2"
ASSERT: json output includes designator "C3"
```

### Step 8.3: Verify signal-path nets

```
RUN: altium-cli pcbdoc nets work/scenario-11/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes net "AUDIO_IN"
ASSERT: json output includes net "STAGE1_OUT"
ASSERT: json output includes net "STAGE2_OUT"
```

### MANUAL CHECKPOINT B

Open `work/scenario-11/board.PcbDoc` in Altium.
**Check:** U1 is SOIC-8. 4x 0402 resistors and 3x 0603 capacitors. Ratsnest shows signal chain flowing through C2 → R1 → U1 → C1 → R3 → U1 → output.

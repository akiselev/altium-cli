# Scenario 10 — SPI Flash Breakout

SPI NOR flash IC with four bus nets (MOSI, MISO, SCK, CS), pull-up on CS, bypass
cap, and a header. Tests a 4-signal bus where one net (CS) has a different
connection topology than the shared bus nets.

**Parts:** 6 (U1: W25Q128 SOIC-8, R1: 10k CS pull-up, R2: 10k HOLD pull-up,
C1: 100nF bypass, C2: 100nF bypass, J1: 1x6 header)

---

## Phase 1 — Part Selection

### Step 1.1

```
RUN: datasheet-cli search "W25Q128" --package SOIC-8
ASSERT: exit 0
ASSERT: stdout contains "W25Q128"
```

### Step 1.2

```
RUN: datasheet-cli pinout "W25Q128JVSIQ"
ASSERT: exit 0
ASSERT: stdout contains "CS" or stdout contains "chip select"
ASSERT: stdout contains "CLK" or stdout contains "SCK"
```

**Record:** U1 = W25Q128, SOIC-8. Pins: CS(1), DO/MISO(2), WP(3), GND(4), DI/MOSI(5), CLK(6), HOLD(7), VCC(8).

---

## Phase 2 — Schematic Library

### Step 2.1: Create

```
RUN: altium-cli schlib create work/scenario-10/parts.SchLib
ASSERT: exit 0
```

### Step 2.2: Generate W25Q128 symbol

```
RUN: altium-cli schlib gen-ic work/scenario-10/parts.SchLib W25Q128 --pins "CS_N,MISO,WP_N,GND,MOSI,SCK,HOLD_N,VCC" --description "128Mbit SPI Flash"
ASSERT: exit 0
```

### Step 2.3: Verify 8 pins

```
RUN: altium-cli schlib component work/scenario-10/parts.SchLib W25Q128 --json
ASSERT: exit 0
ASSERT: json .pin_count == 8
```

### Step 2.4: Add passive and header symbols

```
RUN: altium-cli schlib add-component work/scenario-10/parts.SchLib RES --description "Resistor"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-10/parts.SchLib RES 1 "1" -50 0 --direction right --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-10/parts.SchLib RES 2 "2" 50 0 --direction left --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-rectangle work/scenario-10/parts.SchLib RES -30 -10 30 10
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-component work/scenario-10/parts.SchLib CAP --description "Capacitor"
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-10/parts.SchLib CAP 1 "1" 0 50 --direction down --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-pin work/scenario-10/parts.SchLib CAP 2 "2" 0 -50 --direction up --electrical passive
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-line work/scenario-10/parts.SchLib CAP -20 10 20 10
ASSERT: exit 0
```

```
RUN: altium-cli schlib add-line work/scenario-10/parts.SchLib CAP -20 -10 20 -10
ASSERT: exit 0
```

```
RUN: altium-cli schlib gen-ic work/scenario-10/parts.SchLib HDR6 --pins "VCC,GND,MOSI,MISO,SCK,CS" --description "1x6 SPI Header"
ASSERT: exit 0
```

### Step 2.5: Verify 4 components in library

```
RUN: altium-cli schlib info work/scenario-10/parts.SchLib --json
ASSERT: exit 0
ASSERT: json .component_count == 4
```

---

## Phase 3 — Footprint Library

### Step 3.1: Create

```
RUN: altium-cli pcblib create work/scenario-10/fps.PcbLib
ASSERT: exit 0
```

### Step 3.2: SOIC-8

```
RUN: altium-cli pcblib add-footprint work/scenario-10/fps.PcbLib SOIC-8 --description "SOIC-8"
ASSERT: exit 0
```

```
RUN: altium-cli pcblib add-dual-row work/scenario-10/fps.PcbLib SOIC-8 8 --pitch 50 --span 244
ASSERT: exit 0
```

### Step 3.3: 0402

```
RUN: altium-cli pcblib gen-chip work/scenario-10/fps.PcbLib P0402 --size 0402
ASSERT: exit 0
```

### Step 3.4: 1x6 header

```
RUN: altium-cli pcblib add-footprint work/scenario-10/fps.PcbLib HDR-1X6 --description "1x6 header"
ASSERT: exit 0
```

```
RUN: altium-cli pcblib add-pad-row work/scenario-10/fps.PcbLib HDR-1X6 6 --pitch 100
ASSERT: exit 0
```

### Step 3.5: Verify 3 footprints

```
RUN: altium-cli pcblib list work/scenario-10/fps.PcbLib --json
ASSERT: exit 0
ASSERT: json .total_footprints == 3
```

---

## Phase 4 — Schematic Entry

### Step 4.1: Create

```
RUN: altium-cli schdoc create work/scenario-10/design.SchDoc
ASSERT: exit 0
```

### Step 4.2: Place components

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "add-component work/scenario-10/parts.SchLib W25Q128 1200 1400 U1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "add-component work/scenario-10/parts.SchLib RES 1500 1800 R1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "add-component work/scenario-10/parts.SchLib RES 1500 1600 R2"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "add-component work/scenario-10/parts.SchLib CAP 900 1200 C1"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "add-component work/scenario-10/parts.SchLib CAP 900 1000 C2"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "add-component work/scenario-10/parts.SchLib HDR6 500 1400 J1"
ASSERT: exit 0
```

### Step 4.3: Verify 6 components

```
RUN: altium-cli schdoc components work/scenario-10/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 6
```

### Step 4.4: Power and SPI bus labels

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "add-power VCC 1200 2000 bar down"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "add-power GND 1200 800 ground up"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "add-net-label SPI_MOSI 800 1500"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "add-net-label SPI_MISO 800 1300"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "add-net-label SPI_SCK 800 1100"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "add-net-label SPI_CS_N 1400 1800"
ASSERT: exit 0
```

### Step 4.5: Wire flash IC power

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route U1.VCC @VCC"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route U1.GND @GND"
ASSERT: exit 0
```

### Step 4.6: Wire SPI bus

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route U1.MOSI %SPI_MOSI"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route U1.MISO %SPI_MISO"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route U1.SCK %SPI_SCK"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route U1.CS_N %SPI_CS_N"
ASSERT: exit 0
```

### Step 4.7: CS pull-up to VCC

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route R1.1 @VCC"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route R1.2 %SPI_CS_N"
ASSERT: exit 0
```

### Step 4.8: HOLD pull-up to VCC (keep flash out of hold state)

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route R2.1 @VCC"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route R2.2 U1.HOLD_N"
ASSERT: exit 0
```

### Step 4.9: Tie WP to VCC (disable write protect)

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route U1.WP_N @VCC"
ASSERT: exit 0
```

### Step 4.10: Bypass caps

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route C1.1 @VCC"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route C1.2 @GND"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route C2.1 @VCC"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route C2.2 @GND"
ASSERT: exit 0
```

### Step 4.11: Wire header to SPI bus

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route J1.VCC @VCC"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route J1.GND @GND"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route J1.MOSI %SPI_MOSI"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route J1.MISO %SPI_MISO"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route J1.SCK %SPI_SCK"
ASSERT: exit 0
```

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "route J1.CS %SPI_CS_N"
ASSERT: exit 0
```

### Step 4.12: Junctions

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "add-missing-junctions"
ASSERT: exit 0
```

---

## Phase 5 — Validation

### Step 5.1

```
RUN: altium-cli edit work/scenario-10/design.SchDoc -c "validate"
ASSERT: exit 0
```

### Step 5.2: Verify SPI bus nets each have 2 pins (U1 + J1)

```
RUN: altium-cli schdoc netlist work/scenario-10/design.SchDoc --filter SPI_MOSI --json
ASSERT: exit 0
ASSERT: json net "SPI_MOSI" has >= 2 pins
```

```
RUN: altium-cli schdoc netlist work/scenario-10/design.SchDoc --filter SPI_MISO --json
ASSERT: exit 0
ASSERT: json net "SPI_MISO" has >= 2 pins
```

```
RUN: altium-cli schdoc netlist work/scenario-10/design.SchDoc --filter SPI_SCK --json
ASSERT: exit 0
ASSERT: json net "SPI_SCK" has >= 2 pins
```

### Step 5.3: CS net has 3 pins (U1.CS_N, R1.2, J1.CS)

```
RUN: altium-cli schdoc netlist work/scenario-10/design.SchDoc --filter SPI_CS_N --json
ASSERT: exit 0
ASSERT: json net "SPI_CS_N" has >= 3 pins
```

### Step 5.4: BOM

```
RUN: altium-cli schdoc bom work/scenario-10/design.SchDoc --json
ASSERT: exit 0
ASSERT: json .total_components == 6
```

### MANUAL CHECKPOINT A

Open `work/scenario-10/design.SchDoc` in Altium.
**Check:** Four SPI bus labels (MOSI, MISO, SCK, CS_N) connecting U1 to J1. R1 pull-up on CS_N. R2 pull-up on HOLD_N. WP_N tied to VCC.

---

## Phase 6 — Project

### Step 6.1

```
RUN: altium-cli prjpcb create work/scenario-10/project.PrjPcb --name "SPI Flash Breakout"
ASSERT: exit 0
```

### Step 6.2

```
RUN: altium-cli prjpcb add-document work/scenario-10/project.PrjPcb work/scenario-10/design.SchDoc
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-10/project.PrjPcb work/scenario-10/parts.SchLib
ASSERT: exit 0
```

```
RUN: altium-cli prjpcb add-document work/scenario-10/project.PrjPcb work/scenario-10/fps.PcbLib
ASSERT: exit 0
```

---

## Phase 7 — PCB Setup

### Step 7.1

```
RUN: altium-cli pcbdoc create work/scenario-10/board.PcbDoc
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc set-outline-rect work/scenario-10/board.PcbDoc 787 472
ASSERT: exit 0
```

### Step 7.2

```
RUN: altium-cli pcbdoc set-settings work/scenario-10/board.PcbDoc --imperial --grid-size 25 --track-width 8
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-10/board.PcbDoc "Clearance" --value 6
ASSERT: exit 0
```

```
RUN: altium-cli pcbdoc add-rule work/scenario-10/board.PcbDoc "MinTrackWidth" --value 6
ASSERT: exit 0
```

### Step 7.3

```
RUN: altium-cli prjpcb add-document work/scenario-10/project.PrjPcb work/scenario-10/board.PcbDoc
ASSERT: exit 0
```

---

## Phase 8 — Import

### Step 8.1

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-10/project.PrjPcb --dry-run
ASSERT: exit 0
ASSERT: stdout contains "U1"
ASSERT: stdout contains "J1"
```

```
RUN: altium-cli prjpcb import-to-pcb work/scenario-10/project.PrjPcb
ASSERT: exit 0
```

### Step 8.2: Verify all 6 components

```
RUN: altium-cli pcbdoc components work/scenario-10/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes designator "U1"
ASSERT: json output includes designator "R1"
ASSERT: json output includes designator "R2"
ASSERT: json output includes designator "C1"
ASSERT: json output includes designator "C2"
ASSERT: json output includes designator "J1"
```

### Step 8.3: Verify SPI nets

```
RUN: altium-cli pcbdoc nets work/scenario-10/board.PcbDoc --json
ASSERT: exit 0
ASSERT: json output includes net "SPI_MOSI"
ASSERT: json output includes net "SPI_MISO"
ASSERT: json output includes net "SPI_SCK"
ASSERT: json output includes net "SPI_CS_N"
```

### MANUAL CHECKPOINT B

Open `work/scenario-10/board.PcbDoc` in Altium.
**Check:** U1 is SOIC-8. J1 is through-hole 6-pin header (drill holes visible). Four 0402 passives. Ratsnest shows 4 SPI bus connections between U1 and J1.

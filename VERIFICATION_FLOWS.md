# Altium-CLI Verification Flows

30 manual verification flows for testing all major functionality in `altium-cli`.
Each flow is self-contained and designed to be executed by an agent with shell access.

**Test data paths:**
- Blank SchDoc: `crates/altium-format/data/blank/Sheet1.SchDoc`
- Blank SchLib: `crates/altium-format/data/blank/Schlib1.SchLib`
- Blank PcbLib: `crates/altium-format/data/blank/PcbLib1.PcbLib`
- Sample PcbDoc: `crates/altium-format/data/PCB1.PcbDoc`
- Sample Project: `crates/altium-format/data/Project1.PrjPcb`

**Binary:** `cargo run -p altium-cli --`

---

## Flow 1: SchLib — Create Library and Add Components

**Goal:** Create a new schematic library from scratch, add multiple components with descriptions, and verify they appear in listings.

```bash
# 1. Create a new empty schematic library
cargo run -p altium-cli -- schlib create /tmp/test_flow1.SchLib

# 2. Add three components with descriptions
cargo run -p altium-cli -- schlib add-component /tmp/test_flow1.SchLib "LM7805" -d "5V Linear Regulator"
cargo run -p altium-cli -- schlib add-component /tmp/test_flow1.SchLib "LM358" -d "Dual Op-Amp"
cargo run -p altium-cli -- schlib add-component /tmp/test_flow1.SchLib "ATmega328P" -d "8-bit Microcontroller"

# 3. List components — expect all three to appear
cargo run -p altium-cli -- schlib list /tmp/test_flow1.SchLib

# 4. Verify overview shows correct count and categories
cargo run -p altium-cli -- schlib overview /tmp/test_flow1.SchLib

# 5. Verify info shows library statistics
cargo run -p altium-cli -- schlib info /tmp/test_flow1.SchLib
```

**Pass criteria:** All three components appear in `list` output. Overview shows component count of 3. No errors from any command.

---

## Flow 2: SchLib — Add Pins to Components

**Goal:** Add pins with various electrical types and orientations to a component, then verify pin listing.

```bash
# 1. Create library and add component
cargo run -p altium-cli -- schlib create /tmp/test_flow2.SchLib
cargo run -p altium-cli -- schlib add-component /tmp/test_flow2.SchLib "LM7805" -d "Voltage Regulator"

# 2. Add pins with different electrical types and orientations
cargo run -p altium-cli -- schlib add-pin /tmp/test_flow2.SchLib "LM7805" "1" "VIN" -50 0 -e input -o right
cargo run -p altium-cli -- schlib add-pin /tmp/test_flow2.SchLib "LM7805" "2" "GND" 0 -50 -e power -o up
cargo run -p altium-cli -- schlib add-pin /tmp/test_flow2.SchLib "LM7805" "3" "VOUT" 50 0 -e output -o left

# 3. List pins — expect all three with correct attributes
cargo run -p altium-cli -- schlib pins /tmp/test_flow2.SchLib -c "LM7805"

# 4. View component details
cargo run -p altium-cli -- schlib component /tmp/test_flow2.SchLib "LM7805"

# 5. View with primitives flag
cargo run -p altium-cli -- schlib component /tmp/test_flow2.SchLib "LM7805" --primitives
```

**Pass criteria:** All 3 pins shown with correct designators, names, electrical types, and orientations.

---

## Flow 3: SchLib — Add Graphical Primitives (Rectangle, Line, Polygon)

**Goal:** Add graphical primitives to a component body and verify they appear in the primitives listing.

```bash
# 1. Create library with component
cargo run -p altium-cli -- schlib create /tmp/test_flow3.SchLib
cargo run -p altium-cli -- schlib add-component /tmp/test_flow3.SchLib "IC1" -d "Test IC"

# 2. Add a filled rectangle (component body)
cargo run -p altium-cli -- schlib add-rectangle /tmp/test_flow3.SchLib "IC1" -30 -40 30 40 --filled

# 3. Add a line
cargo run -p altium-cli -- schlib add-line /tmp/test_flow3.SchLib "IC1" -30 0 -40 0

# 4. Add a polygon
cargo run -p altium-cli -- schlib add-polygon /tmp/test_flow3.SchLib "IC1" "0,50 -10,40 10,40"

# 5. Verify all primitives appear
cargo run -p altium-cli -- schlib primitives /tmp/test_flow3.SchLib "IC1"

# 6. Render as ASCII art to visually verify
cargo run -p altium-cli -- schlib render-ascii /tmp/test_flow3.SchLib "IC1"
```

**Pass criteria:** Primitives listing shows rectangle, line, and polygon. ASCII render produces recognizable output without errors.

---

## Flow 4: SchLib — Generate IC Symbol from Pinout

**Goal:** Use the `gen-ic` command to auto-generate an IC symbol from a pin list, then inspect the result.

```bash
# 1. Create library
cargo run -p altium-cli -- schlib create /tmp/test_flow4.SchLib

# 2. Generate IC symbol with named pins
cargo run -p altium-cli -- schlib gen-ic /tmp/test_flow4.SchLib "ATtiny85" --pins "VCC,PB0,PB1,PB2,PB3,PB4,RESET,GND"

# 3. Verify component exists with correct pin count
cargo run -p altium-cli -- schlib component /tmp/test_flow4.SchLib "ATtiny85" --primitives

# 4. List pins to verify all 8 pins
cargo run -p altium-cli -- schlib pins /tmp/test_flow4.SchLib -c "ATtiny85"

# 5. Render ASCII art for visual check
cargo run -p altium-cli -- schlib render-ascii /tmp/test_flow4.SchLib "ATtiny85"

# 6. Verify in overview
cargo run -p altium-cli -- schlib overview /tmp/test_flow4.SchLib
```

**Pass criteria:** 8 pins generated. ASCII render shows an IC box shape with pins on left/right sides. Component appears in overview.

---

## Flow 5: SchLib — Search and JSON Export

**Goal:** Test search functionality and JSON export for a populated library.

```bash
# 1. Create library with multiple components
cargo run -p altium-cli -- schlib create /tmp/test_flow5.SchLib
cargo run -p altium-cli -- schlib add-component /tmp/test_flow5.SchLib "LM7805" -d "5V Regulator"
cargo run -p altium-cli -- schlib add-component /tmp/test_flow5.SchLib "LM7812" -d "12V Regulator"
cargo run -p altium-cli -- schlib add-component /tmp/test_flow5.SchLib "LM358" -d "Dual Op-Amp"
cargo run -p altium-cli -- schlib add-component /tmp/test_flow5.SchLib "NE555" -d "Timer IC"

# 2. Search for "LM" — should find LM7805, LM7812, LM358
cargo run -p altium-cli -- schlib search /tmp/test_flow5.SchLib "LM"

# 3. Search with limit
cargo run -p altium-cli -- schlib search /tmp/test_flow5.SchLib "LM" --limit 1

# 4. Export as JSON
cargo run -p altium-cli -- schlib json /tmp/test_flow5.SchLib

# 5. Export as pretty JSON
cargo run -p altium-cli -- schlib json /tmp/test_flow5.SchLib --pretty
```

**Pass criteria:** Search "LM" returns 3 results. `--limit 1` returns exactly 1. JSON output is valid JSON. Pretty JSON is indented.

---

## Flow 6: SchLib — Add Component from JSON Definition

**Goal:** Define a component in JSON and add it to a library using `add-json`.

```bash
# 1. Create library
cargo run -p altium-cli -- schlib create /tmp/test_flow6.SchLib

# 2. Create a JSON component definition
cat > /tmp/component_def.json << 'EOF'
{
  "name": "LM317",
  "description": "Adjustable Voltage Regulator",
  "pins": [
    {"designator": "1", "name": "ADJ", "x": -50, "y": 0, "electrical": "passive"},
    {"designator": "2", "name": "VOUT", "x": 50, "y": 10, "electrical": "output"},
    {"designator": "3", "name": "VIN", "x": 50, "y": -10, "electrical": "input"}
  ],
  "rectangle": {"x1": -30, "y1": -30, "x2": 30, "y2": 30, "filled": true}
}
EOF

# 3. Add from JSON
cargo run -p altium-cli -- schlib add-json /tmp/test_flow6.SchLib /tmp/component_def.json

# 4. Verify component
cargo run -p altium-cli -- schlib component /tmp/test_flow6.SchLib "LM317" --primitives
cargo run -p altium-cli -- schlib pins /tmp/test_flow6.SchLib -c "LM317"
```

**Pass criteria:** Component LM317 created with 3 pins and a rectangle. Pin attributes match JSON definition.

---

## Flow 7: PcbLib — Create Library and Add Footprints

**Goal:** Create a PCB footprint library, add footprints, and verify listings.

```bash
# 1. Create new PCB library
cargo run -p altium-cli -- pcblib create /tmp/test_flow7.PcbLib

# 2. Add footprints
cargo run -p altium-cli -- pcblib add-footprint /tmp/test_flow7.PcbLib "SOIC-8" -d "8-pin SOIC package"
cargo run -p altium-cli -- pcblib add-footprint /tmp/test_flow7.PcbLib "0805" -d "0805 chip resistor"
cargo run -p altium-cli -- pcblib add-footprint /tmp/test_flow7.PcbLib "QFP-44" -d "44-pin QFP package"

# 3. List all footprints
cargo run -p altium-cli -- pcblib list /tmp/test_flow7.PcbLib

# 4. Overview
cargo run -p altium-cli -- pcblib overview /tmp/test_flow7.PcbLib

# 5. Info
cargo run -p altium-cli -- pcblib info /tmp/test_flow7.PcbLib
```

**Pass criteria:** All 3 footprints listed. Overview and info show correct count.

---

## Flow 8: PcbLib — Add Pads with Various Shapes

**Goal:** Add SMD and through-hole pads to a footprint with different shapes and verify.

```bash
# 1. Create library with footprint
cargo run -p altium-cli -- pcblib create /tmp/test_flow8.PcbLib
cargo run -p altium-cli -- pcblib add-footprint /tmp/test_flow8.PcbLib "DIP-8" -d "8-pin DIP"

# 2. Add through-hole pads
cargo run -p altium-cli -- pcblib add-pad /tmp/test_flow8.PcbLib -f "DIP-8" -d "1" -x -150 -y -150 -w 60 --height 60 -s round --hole 35
cargo run -p altium-cli -- pcblib add-pad /tmp/test_flow8.PcbLib -f "DIP-8" -d "2" -x -150 -y -50 -w 60 --height 60 -s round --hole 35
cargo run -p altium-cli -- pcblib add-pad /tmp/test_flow8.PcbLib -f "DIP-8" -d "3" -x -150 -y 50 -w 60 --height 60 -s round --hole 35
cargo run -p altium-cli -- pcblib add-pad /tmp/test_flow8.PcbLib -f "DIP-8" -d "4" -x -150 -y 150 -w 60 --height 60 -s round --hole 35
cargo run -p altium-cli -- pcblib add-pad /tmp/test_flow8.PcbLib -f "DIP-8" -d "5" -x 150 -y 150 -w 60 --height 60 -s round --hole 35
cargo run -p altium-cli -- pcblib add-pad /tmp/test_flow8.PcbLib -f "DIP-8" -d "6" -x 150 -y 50 -w 60 --height 60 -s round --hole 35
cargo run -p altium-cli -- pcblib add-pad /tmp/test_flow8.PcbLib -f "DIP-8" -d "7" -x 150 -y -50 -w 60 --height 60 -s round --hole 35
cargo run -p altium-cli -- pcblib add-pad /tmp/test_flow8.PcbLib -f "DIP-8" -d "8" -x 150 -y -150 -w 60 --height 60 -s round --hole 35

# 3. List pads
cargo run -p altium-cli -- pcblib pads /tmp/test_flow8.PcbLib -f "DIP-8"

# 4. Verify footprint details
cargo run -p altium-cli -- pcblib footprint /tmp/test_flow8.PcbLib "DIP-8" --primitives
```

**Pass criteria:** 8 pads listed with correct designators, positions, shapes, and hole sizes.

---

## Flow 9: PcbLib — Generate Chip Footprint and Measure

**Goal:** Use `gen-chip` to auto-generate a standard chip footprint and measure it.

```bash
# 1. Create library
cargo run -p altium-cli -- pcblib create /tmp/test_flow9.PcbLib

# 2. Generate standard 0805 chip footprint
cargo run -p altium-cli -- pcblib gen-chip /tmp/test_flow9.PcbLib "0805"

# 3. Measure dimensions
cargo run -p altium-cli -- pcblib measure /tmp/test_flow9.PcbLib "0805"

# 4. List pads to verify pad geometry
cargo run -p altium-cli -- pcblib pads /tmp/test_flow9.PcbLib -f "0805"

# 5. Analyze hole sizes (should be zero for SMD)
cargo run -p altium-cli -- pcblib holes /tmp/test_flow9.PcbLib

# 6. Render ASCII
cargo run -p altium-cli -- pcblib render-ascii /tmp/test_flow9.PcbLib "0805"
```

**Pass criteria:** Footprint generated with 2 SMD pads. Measure shows pad dimensions. Holes report shows no through-holes. ASCII render shows two pads.

---

## Flow 10: PcbLib — Add Pad Rows (Dual Row, Quad, Grid)

**Goal:** Test advanced pad generation commands for common package types.

```bash
# 1. Create library
cargo run -p altium-cli -- pcblib create /tmp/test_flow10.PcbLib

# 2. Add footprint and generate dual-row pads (SOIC-8 style)
cargo run -p altium-cli -- pcblib add-footprint /tmp/test_flow10.PcbLib "SOIC-8" -d "8-pin SOIC"
cargo run -p altium-cli -- pcblib add-dual-row /tmp/test_flow10.PcbLib -f "SOIC-8" --pitch 50 --span 240 --start 1 -n 8 -w 25 --height 60

# 3. Add QFP-style quad pads
cargo run -p altium-cli -- pcblib add-footprint /tmp/test_flow10.PcbLib "QFP-32" -d "32-pin QFP"
cargo run -p altium-cli -- pcblib add-quad-pads /tmp/test_flow10.PcbLib -f "QFP-32" --pitch 31 --span 400 -n 32 -w 60 --height 15

# 4. Add BGA-style grid pads
cargo run -p altium-cli -- pcblib add-footprint /tmp/test_flow10.PcbLib "BGA-16" -d "16-ball BGA"
cargo run -p altium-cli -- pcblib add-pad-grid /tmp/test_flow10.PcbLib -f "BGA-16" --pitch 50 --rows 4 --cols 4 -w 25 --height 25

# 5. Verify pad counts
cargo run -p altium-cli -- pcblib pads /tmp/test_flow10.PcbLib -f "SOIC-8"
cargo run -p altium-cli -- pcblib pads /tmp/test_flow10.PcbLib -f "QFP-32"
cargo run -p altium-cli -- pcblib pads /tmp/test_flow10.PcbLib -f "BGA-16"

# 6. List all footprints
cargo run -p altium-cli -- pcblib list /tmp/test_flow10.PcbLib
```

**Pass criteria:** SOIC-8 has 8 pads, QFP-32 has 32 pads, BGA-16 has 16 pads. All pad counts correct.

---

## Flow 11: PcbLib — SVG and PNG Rendering

**Goal:** Test footprint rendering to SVG and PNG output formats.

```bash
# 1. Create library with a footprint and pads
cargo run -p altium-cli -- pcblib create /tmp/test_flow11.PcbLib
cargo run -p altium-cli -- pcblib add-footprint /tmp/test_flow11.PcbLib "SOIC-8" -d "8-pin SOIC"
cargo run -p altium-cli -- pcblib add-dual-row /tmp/test_flow11.PcbLib -f "SOIC-8" --pitch 50 --span 240 --start 1 -n 8 -w 25 --height 60

# 2. Add silkscreen outline
cargo run -p altium-cli -- pcblib add-silkscreen /tmp/test_flow11.PcbLib -f "SOIC-8" --x1 -80 --y1 -130 --x2 80 --y2 -130 -w 8
cargo run -p altium-cli -- pcblib add-silkscreen /tmp/test_flow11.PcbLib -f "SOIC-8" --x1 80 --y1 -130 --x2 80 --y2 130 -w 8
cargo run -p altium-cli -- pcblib add-silkscreen /tmp/test_flow11.PcbLib -f "SOIC-8" --x1 80 --y1 130 --x2 -80 --y2 130 -w 8
cargo run -p altium-cli -- pcblib add-silkscreen /tmp/test_flow11.PcbLib -f "SOIC-8" --x1 -80 --y1 130 --x2 -80 --y2 -130 -w 8

# 3. Render as SVG
cargo run -p altium-cli -- pcblib render-svg /tmp/test_flow11.PcbLib "SOIC-8" -o /tmp/soic8.svg

# 4. Render as PNG
cargo run -p altium-cli -- pcblib render-png /tmp/test_flow11.PcbLib "SOIC-8" -o /tmp/soic8.png

# 5. Verify output files exist and have content
ls -la /tmp/soic8.svg /tmp/soic8.png
```

**Pass criteria:** SVG and PNG files created, both non-empty. No rendering errors.

---

## Flow 12: PcbLib — Search, JSON Export, and JSON Import

**Goal:** Test search, JSON export, and round-trip JSON import for PcbLib.

```bash
# 1. Create library with several footprints
cargo run -p altium-cli -- pcblib create /tmp/test_flow12.PcbLib
cargo run -p altium-cli -- pcblib gen-chip /tmp/test_flow12.PcbLib "0402"
cargo run -p altium-cli -- pcblib gen-chip /tmp/test_flow12.PcbLib "0603"
cargo run -p altium-cli -- pcblib gen-chip /tmp/test_flow12.PcbLib "0805"
cargo run -p altium-cli -- pcblib gen-chip /tmp/test_flow12.PcbLib "1206"

# 2. Search for "08" — should match 0805
cargo run -p altium-cli -- pcblib search /tmp/test_flow12.PcbLib "08"

# 3. Search with limit
cargo run -p altium-cli -- pcblib search /tmp/test_flow12.PcbLib "0" --limit 2

# 4. Export as JSON
cargo run -p altium-cli -- pcblib json /tmp/test_flow12.PcbLib --full

# 5. Create second library and add footprint from JSON definition
cargo run -p altium-cli -- pcblib create /tmp/test_flow12b.PcbLib
cat > /tmp/footprint_def.json << 'EOF'
{
  "name": "SOT-23",
  "description": "3-pin SOT-23",
  "pads": [
    {"designator": "1", "x": -37, "y": -40, "width": 24, "height": 40, "shape": "rectangular"},
    {"designator": "2", "x": 37, "y": -40, "width": 24, "height": 40, "shape": "rectangular"},
    {"designator": "3", "x": 0, "y": 40, "width": 24, "height": 40, "shape": "rectangular"}
  ]
}
EOF
cargo run -p altium-cli -- pcblib add-json /tmp/test_flow12b.PcbLib /tmp/footprint_def.json

# 6. Verify imported footprint
cargo run -p altium-cli -- pcblib footprint /tmp/test_flow12b.PcbLib "SOT-23" --primitives
```

**Pass criteria:** Search finds correct results. JSON is valid. JSON import creates footprint with 3 pads.

---

## Flow 13: SchDoc — Create and Inspect Blank Schematic

**Goal:** Create a new schematic, inspect it, and verify basic metadata.

```bash
# 1. Create new schematic document
cargo run -p altium-cli -- schdoc create /tmp/test_flow13.SchDoc

# 2. Verify info
cargo run -p altium-cli -- schdoc info /tmp/test_flow13.SchDoc

# 3. Verify stats (should show minimal records)
cargo run -p altium-cli -- schdoc stats /tmp/test_flow13.SchDoc

# 4. Verify overview (empty design)
cargo run -p altium-cli -- schdoc overview /tmp/test_flow13.SchDoc

# 5. List components (should be empty)
cargo run -p altium-cli -- schdoc components /tmp/test_flow13.SchDoc

# 6. Export as JSON
cargo run -p altium-cli -- schdoc json /tmp/test_flow13.SchDoc --pretty
```

**Pass criteria:** Document created successfully. Info shows valid metadata. Components list is empty. JSON is valid.

---

## Flow 14: SchDoc — Edit Operations: Add Components from Library

**Goal:** Add components from a schematic library to a schematic document using the edit command.

```bash
# 1. Create a library with components and pins
cargo run -p altium-cli -- schlib create /tmp/test_flow14.SchLib
cargo run -p altium-cli -- schlib gen-ic /tmp/test_flow14.SchLib "LM7805" --pins "VIN,GND,VOUT"
cargo run -p altium-cli -- schlib gen-ic /tmp/test_flow14.SchLib "RES" --pins "1,2"

# 2. Create schematic and add components from library
cargo run -p altium-cli -- schdoc create /tmp/test_flow14.SchDoc
cargo run -p altium-cli -- edit /tmp/test_flow14.SchDoc -c "add-component /tmp/test_flow14.SchLib LM7805 500 500" -o /tmp/test_flow14_out.SchDoc

# 3. Add another component
cargo run -p altium-cli -- edit /tmp/test_flow14_out.SchDoc -c "add-component /tmp/test_flow14.SchLib RES 800 500" -o /tmp/test_flow14_out2.SchDoc

# 4. Verify components placed
cargo run -p altium-cli -- schdoc components /tmp/test_flow14_out2.SchDoc

# 5. Detailed component info
cargo run -p altium-cli -- schdoc overview /tmp/test_flow14_out2.SchDoc
```

**Pass criteria:** Both components appear in the schematic. Components listing shows designators and positions.

---

## Flow 15: SchDoc — Edit Operations: Wire Routing and Net Labels

**Goal:** Add wires, net labels, and junctions to connect components in a schematic.

```bash
# 1. Create a schematic (use blank template)
cp crates/altium-format/data/blank/Sheet1.SchDoc /tmp/test_flow15.SchDoc

# 2. Add wires
cargo run -p altium-cli -- edit /tmp/test_flow15.SchDoc -c "add-wire 100,100,200,100,200,200" -o /tmp/test_flow15_out.SchDoc

# 3. Add a net label
cargo run -p altium-cli -- edit /tmp/test_flow15_out.SchDoc -c "add-net-label VCC 100 100" -o /tmp/test_flow15_out2.SchDoc

# 4. Add a junction
cargo run -p altium-cli -- edit /tmp/test_flow15_out2.SchDoc -c "add-junction 200 100" -o /tmp/test_flow15_out3.SchDoc

# 5. Add a power port
cargo run -p altium-cli -- edit /tmp/test_flow15_out3.SchDoc -c "add-power GND 200 200 ground down" -o /tmp/test_flow15_out4.SchDoc

# 6. Verify wires
cargo run -p altium-cli -- schdoc wires /tmp/test_flow15_out4.SchDoc

# 7. Verify nets
cargo run -p altium-cli -- schdoc nets /tmp/test_flow15_out4.SchDoc

# 8. Verify junctions
cargo run -p altium-cli -- schdoc junctions /tmp/test_flow15_out4.SchDoc

# 9. Verify power objects
cargo run -p altium-cli -- schdoc power /tmp/test_flow15_out4.SchDoc
```

**Pass criteria:** Wire appears in wires listing. Net label "VCC" appears. Junction and GND power object present.

---

## Flow 16: SchDoc — Edit Operations: Move and Delete Components

**Goal:** Move and delete components in a schematic document.

```bash
# 1. Setup: Create schematic with library and placed component
cargo run -p altium-cli -- schlib create /tmp/test_flow16.SchLib
cargo run -p altium-cli -- schlib gen-ic /tmp/test_flow16.SchLib "U1_IC" --pins "A,B,C,D"
cargo run -p altium-cli -- schdoc create /tmp/test_flow16.SchDoc
cargo run -p altium-cli -- edit /tmp/test_flow16.SchDoc -c "add-component /tmp/test_flow16.SchLib U1_IC 100 100" -o /tmp/test_flow16_placed.SchDoc

# 2. Check initial position
cargo run -p altium-cli -- schdoc components /tmp/test_flow16_placed.SchDoc --verbose

# 3. Move the component
cargo run -p altium-cli -- edit /tmp/test_flow16_placed.SchDoc -c "move U1 500 500" -o /tmp/test_flow16_moved.SchDoc

# 4. Verify moved position
cargo run -p altium-cli -- schdoc components /tmp/test_flow16_moved.SchDoc --verbose

# 5. Delete the component
cargo run -p altium-cli -- edit /tmp/test_flow16_moved.SchDoc -c "delete U1" -o /tmp/test_flow16_deleted.SchDoc

# 6. Verify component is gone
cargo run -p altium-cli -- schdoc components /tmp/test_flow16_deleted.SchDoc
```

**Pass criteria:** Component initially at (100,100). After move, at (500,500). After delete, component list is empty.

---

## Flow 17: SchDoc — Edit Operations: Port and Route Commands

**Goal:** Add ports and use the route command to connect design elements.

```bash
# 1. Setup schematic
cp crates/altium-format/data/blank/Sheet1.SchDoc /tmp/test_flow17.SchDoc

# 2. Add ports
cargo run -p altium-cli -- edit /tmp/test_flow17.SchDoc -c "add-port DATA_IN 100 200 input" -o /tmp/test_flow17_p1.SchDoc
cargo run -p altium-cli -- edit /tmp/test_flow17_p1.SchDoc -c "add-port DATA_OUT 500 200 output" -o /tmp/test_flow17_p2.SchDoc
cargo run -p altium-cli -- edit /tmp/test_flow17_p2.SchDoc -c "add-port CLK 100 300 input" -o /tmp/test_flow17_p3.SchDoc

# 3. Route between two coordinates
cargo run -p altium-cli -- edit /tmp/test_flow17_p3.SchDoc -c "route 100,200 500,200" -o /tmp/test_flow17_routed.SchDoc

# 4. Verify ports
cargo run -p altium-cli -- schdoc ports /tmp/test_flow17_routed.SchDoc

# 5. Verify wires created by routing
cargo run -p altium-cli -- schdoc wires /tmp/test_flow17_routed.SchDoc

# 6. Verify connectivity overview
cargo run -p altium-cli -- schdoc overview /tmp/test_flow17_routed.SchDoc
```

**Pass criteria:** 3 ports present with correct IO types. Route creates wire(s) between specified coordinates.

---

## Flow 18: SchDoc — Edit Validate and Add Missing Junctions

**Goal:** Test the `validate` and `add-missing-junctions` edit operations.

```bash
# 1. Setup schematic with crossing wires
cp crates/altium-format/data/blank/Sheet1.SchDoc /tmp/test_flow18.SchDoc

# 2. Add crossing wires (T-junction, no junction marker)
cargo run -p altium-cli -- edit /tmp/test_flow18.SchDoc -c "add-wire 100,200,300,200" -o /tmp/test_flow18_w1.SchDoc
cargo run -p altium-cli -- edit /tmp/test_flow18_w1.SchDoc -c "add-wire 200,100,200,300" -o /tmp/test_flow18_w2.SchDoc

# 3. Validate — should report missing junctions
cargo run -p altium-cli -- edit /tmp/test_flow18_w2.SchDoc -c "validate" -o /tmp/test_flow18_val.SchDoc

# 4. Add missing junctions automatically
cargo run -p altium-cli -- edit /tmp/test_flow18_w2.SchDoc -c "add-missing-junctions" -o /tmp/test_flow18_junc.SchDoc

# 5. Verify junctions added
cargo run -p altium-cli -- schdoc junctions /tmp/test_flow18_junc.SchDoc

# 6. Re-validate — should pass now
cargo run -p altium-cli -- edit /tmp/test_flow18_junc.SchDoc -c "validate" -o /tmp/test_flow18_val2.SchDoc
```

**Pass criteria:** Validate reports issue at wire crossing. `add-missing-junctions` adds junction at (200,200). Re-validate passes.

---

## Flow 19: SchDoc — Analysis Commands (BOM, Netlist, Power Map)

**Goal:** Test analysis commands on a populated schematic document.

```bash
# 1. Build a schematic with components and connections
cargo run -p altium-cli -- schlib create /tmp/test_flow19.SchLib
cargo run -p altium-cli -- schlib gen-ic /tmp/test_flow19.SchLib "MCU" --pins "VCC,GND,TX,RX,CLK"
cargo run -p altium-cli -- schlib gen-ic /tmp/test_flow19.SchLib "RES" --pins "1,2"
cargo run -p altium-cli -- schdoc create /tmp/test_flow19.SchDoc
cargo run -p altium-cli -- edit /tmp/test_flow19.SchDoc -c "add-component /tmp/test_flow19.SchLib MCU 300 300" -o /tmp/test_flow19_s1.SchDoc
cargo run -p altium-cli -- edit /tmp/test_flow19_s1.SchDoc -c "add-component /tmp/test_flow19.SchLib RES 600 300" -o /tmp/test_flow19_s2.SchDoc
cargo run -p altium-cli -- edit /tmp/test_flow19_s2.SchDoc -c "add-power VCC 300 500 bar up" -o /tmp/test_flow19_s3.SchDoc
cargo run -p altium-cli -- edit /tmp/test_flow19_s3.SchDoc -c "add-power GND 300 100 ground down" -o /tmp/test_flow19_s4.SchDoc

# 2. Generate BOM
cargo run -p altium-cli -- schdoc bom /tmp/test_flow19_s4.SchDoc

# 3. Generate netlist
cargo run -p altium-cli -- schdoc netlist /tmp/test_flow19_s4.SchDoc

# 4. Filter netlist
cargo run -p altium-cli -- schdoc netlist /tmp/test_flow19_s4.SchDoc --filter "VCC"

# 5. Power map
cargo run -p altium-cli -- schdoc power-map /tmp/test_flow19_s4.SchDoc

# 6. Block diagram
cargo run -p altium-cli -- schdoc blocks /tmp/test_flow19_s4.SchDoc
```

**Pass criteria:** BOM lists MCU and RES. Netlist shows connectivity. Power map shows VCC/GND nets. Block diagram shows MCU as major IC.

---

## Flow 20: SchDoc — Hierarchy and Record Inspection

**Goal:** Test hierarchy tree viewing, stats, and component detail inspection.

```bash
# 1. Use the blank schematic as base, add content
cargo run -p altium-cli -- schlib create /tmp/test_flow20.SchLib
cargo run -p altium-cli -- schlib gen-ic /tmp/test_flow20.SchLib "OPAMP" --pins "V+,V-,OUT,VCC,VEE"
cargo run -p altium-cli -- schdoc create /tmp/test_flow20.SchDoc
cargo run -p altium-cli -- edit /tmp/test_flow20.SchDoc -c "add-component /tmp/test_flow20.SchLib OPAMP 400 400" -o /tmp/test_flow20_out.SchDoc

# 2. View hierarchy tree
cargo run -p altium-cli -- schdoc hierarchy /tmp/test_flow20_out.SchDoc

# 3. View hierarchy from specific component
cargo run -p altium-cli -- schdoc hierarchy /tmp/test_flow20_out.SchDoc --from U1

# 4. View hierarchy with depth limit
cargo run -p altium-cli -- schdoc hierarchy /tmp/test_flow20_out.SchDoc --depth 1

# 5. View record stats
cargo run -p altium-cli -- schdoc stats /tmp/test_flow20_out.SchDoc

# 6. View specific component with children
cargo run -p altium-cli -- schdoc component /tmp/test_flow20_out.SchDoc U1 --children

# 7. List pins for the component
cargo run -p altium-cli -- schdoc pins /tmp/test_flow20_out.SchDoc -c U1
```

**Pass criteria:** Hierarchy shows nested structure. Stats shows record type counts. Component detail shows pin children.

---

## Flow 21: PcbDoc — Inspect Existing PCB Document

**Goal:** Read and inspect the sample PCB document using all analysis commands.

```bash
# 1. Overview of sample PCB
cargo run -p altium-cli -- pcbdoc overview crates/altium-format/data/PCB1.PcbDoc

# 2. Document info
cargo run -p altium-cli -- pcbdoc info crates/altium-format/data/PCB1.PcbDoc

# 3. List components
cargo run -p altium-cli -- pcbdoc components crates/altium-format/data/PCB1.PcbDoc

# 4. List layers
cargo run -p altium-cli -- pcbdoc layers crates/altium-format/data/PCB1.PcbDoc

# 5. List all layers
cargo run -p altium-cli -- pcbdoc layers crates/altium-format/data/PCB1.PcbDoc --all

# 6. List nets
cargo run -p altium-cli -- pcbdoc nets crates/altium-format/data/PCB1.PcbDoc

# 7. List rules
cargo run -p altium-cli -- pcbdoc rules crates/altium-format/data/PCB1.PcbDoc

# 8. Board outline
cargo run -p altium-cli -- pcbdoc outline crates/altium-format/data/PCB1.PcbDoc

# 9. Board settings
cargo run -p altium-cli -- pcbdoc settings crates/altium-format/data/PCB1.PcbDoc

# 10. Export JSON
cargo run -p altium-cli -- pcbdoc json crates/altium-format/data/PCB1.PcbDoc --pretty | head -50
```

**Pass criteria:** All commands produce output without errors. Components, nets, layers, and rules are listed.

---

## Flow 22: PcbDoc — Create and Configure Board

**Goal:** Create a new PCB document, set board outline, configure settings, and add design rules.

```bash
# 1. Create new PCB
cargo run -p altium-cli -- pcbdoc create /tmp/test_flow22.PcbDoc

# 2. Set rectangular board outline (50mm x 30mm = ~1969mil x ~1181mil)
cargo run -p altium-cli -- pcbdoc set-outline-rect /tmp/test_flow22.PcbDoc 1969 1181

# 3. Configure board settings (metric, grids)
cargo run -p altium-cli -- pcbdoc set-settings /tmp/test_flow22.PcbDoc --metric --snap-grid 25 --visible-grid 100

# 4. Add design rules
cargo run -p altium-cli -- pcbdoc add-rule /tmp/test_flow22.PcbDoc clearance --value 8
cargo run -p altium-cli -- pcbdoc add-rule /tmp/test_flow22.PcbDoc width --value 10

# 5. Verify outline
cargo run -p altium-cli -- pcbdoc outline /tmp/test_flow22.PcbDoc

# 6. Verify settings
cargo run -p altium-cli -- pcbdoc settings /tmp/test_flow22.PcbDoc

# 7. Verify rules
cargo run -p altium-cli -- pcbdoc rules /tmp/test_flow22.PcbDoc
```

**Pass criteria:** Board created with correct outline dimensions. Settings reflect metric/grid values. Rules listed.

---

## Flow 23: PcbDoc — Place Components and Add Nets

**Goal:** Place components on a PCB, define nets, and add tracks/vias.

```bash
# 1. Create PCB
cargo run -p altium-cli -- pcbdoc create /tmp/test_flow23.PcbDoc
cargo run -p altium-cli -- pcbdoc set-outline-rect /tmp/test_flow23.PcbDoc 2000 1500

# 2. Add nets
cargo run -p altium-cli -- pcbdoc add-net /tmp/test_flow23.PcbDoc "VCC"
cargo run -p altium-cli -- pcbdoc add-net /tmp/test_flow23.PcbDoc "GND"
cargo run -p altium-cli -- pcbdoc add-net /tmp/test_flow23.PcbDoc "SIG1"

# 3. Place components
cargo run -p altium-cli -- pcbdoc place-component /tmp/test_flow23.PcbDoc "U1" 500 750
cargo run -p altium-cli -- pcbdoc place-component /tmp/test_flow23.PcbDoc "R1" 1000 750
cargo run -p altium-cli -- pcbdoc place-component /tmp/test_flow23.PcbDoc "C1" 1500 750

# 4. Add track segment
cargo run -p altium-cli -- pcbdoc add-track /tmp/test_flow23.PcbDoc --start 500,750 --end 1000,750 -w 10 -l top -n "SIG1"

# 5. Add via
cargo run -p altium-cli -- pcbdoc add-via /tmp/test_flow23.PcbDoc --x 750 --y 750 --size 30 --hole 15 -n "SIG1"

# 6. Verify components
cargo run -p altium-cli -- pcbdoc components /tmp/test_flow23.PcbDoc

# 7. Verify nets
cargo run -p altium-cli -- pcbdoc nets /tmp/test_flow23.PcbDoc

# 8. Verify tracks
cargo run -p altium-cli -- pcbdoc tracks /tmp/test_flow23.PcbDoc

# 9. Verify vias
cargo run -p altium-cli -- pcbdoc vias /tmp/test_flow23.PcbDoc
```

**Pass criteria:** 3 components placed, 3 nets defined, track and via present. All commands return data.

---

## Flow 24: PcbDoc — Keepouts, Cutouts, Polygons, and Text

**Goal:** Add board features: keepout regions, cutouts, copper pours, and silkscreen text.

```bash
# 1. Create and configure PCB
cargo run -p altium-cli -- pcbdoc create /tmp/test_flow24.PcbDoc
cargo run -p altium-cli -- pcbdoc set-outline-rect /tmp/test_flow24.PcbDoc 3000 2000
cargo run -p altium-cli -- pcbdoc add-net /tmp/test_flow24.PcbDoc "GND"

# 2. Add keepout region
cargo run -p altium-cli -- pcbdoc add-keepout /tmp/test_flow24.PcbDoc top 100 100 500 500

# 3. Add board cutout (mounting hole area)
cargo run -p altium-cli -- pcbdoc add-cutout /tmp/test_flow24.PcbDoc 2700 100 2900 300

# 4. Add copper polygon pour
cargo run -p altium-cli -- pcbdoc add-polygon /tmp/test_flow24.PcbDoc top "GND" "0,0 3000,0 3000,2000 0,2000"

# 5. Add text on silkscreen
cargo run -p altium-cli -- pcbdoc add-text /tmp/test_flow24.PcbDoc --text "REV A" --x 1500 --y 100 --layer top-overlay --height 50

# 6. Verify all
cargo run -p altium-cli -- pcbdoc keepouts /tmp/test_flow24.PcbDoc
cargo run -p altium-cli -- pcbdoc cutouts /tmp/test_flow24.PcbDoc
cargo run -p altium-cli -- pcbdoc polygons /tmp/test_flow24.PcbDoc
cargo run -p altium-cli -- pcbdoc texts /tmp/test_flow24.PcbDoc
```

**Pass criteria:** Keepout, cutout, polygon, and text all present in their respective listings.

---

## Flow 25: PcbDoc — Design Rule Management (Add, Modify, Delete)

**Goal:** Test the full lifecycle of design rules: add, inspect, modify, and delete.

```bash
# 1. Create PCB
cargo run -p altium-cli -- pcbdoc create /tmp/test_flow25.PcbDoc

# 2. Add several rules
cargo run -p altium-cli -- pcbdoc add-rule /tmp/test_flow25.PcbDoc clearance --value 6
cargo run -p altium-cli -- pcbdoc add-rule /tmp/test_flow25.PcbDoc width --value 8
cargo run -p altium-cli -- pcbdoc add-rule /tmp/test_flow25.PcbDoc via --value 20

# 3. List rules
cargo run -p altium-cli -- pcbdoc rules /tmp/test_flow25.PcbDoc

# 4. Inspect specific rule
cargo run -p altium-cli -- pcbdoc rules /tmp/test_flow25.PcbDoc -v

# 5. Modify a rule
cargo run -p altium-cli -- pcbdoc modify-rule /tmp/test_flow25.PcbDoc "clearance" --value 10

# 6. Verify modification
cargo run -p altium-cli -- pcbdoc rules /tmp/test_flow25.PcbDoc -v

# 7. Delete a rule
cargo run -p altium-cli -- pcbdoc delete-rule /tmp/test_flow25.PcbDoc "via"

# 8. Verify deletion
cargo run -p altium-cli -- pcbdoc rules /tmp/test_flow25.PcbDoc
```

**Pass criteria:** Rules created, listed, modified (clearance from 6 to 10), and deleted (via removed). Final listing shows 2 rules.

---

## Flow 26: PrjPcb — Project Management

**Goal:** Create a project, manage documents and parameters, and validate.

```bash
# 1. Create new project
cargo run -p altium-cli -- prjpcb create /tmp/test_flow26.PrjPcb -n "TestProject"

# 2. Verify info
cargo run -p altium-cli -- prjpcb info /tmp/test_flow26.PrjPcb

# 3. Set project parameters
cargo run -p altium-cli -- prjpcb set-parameter /tmp/test_flow26.PrjPcb "Revision" "A"
cargo run -p altium-cli -- prjpcb set-parameter /tmp/test_flow26.PrjPcb "Author" "TestUser"
cargo run -p altium-cli -- prjpcb set-parameter /tmp/test_flow26.PrjPcb "Company" "ACME Corp"

# 4. List parameters
cargo run -p altium-cli -- prjpcb parameters /tmp/test_flow26.PrjPcb

# 5. Add documents to project
cargo run -p altium-cli -- prjpcb add-document /tmp/test_flow26.PrjPcb "main.SchDoc"
cargo run -p altium-cli -- prjpcb add-document /tmp/test_flow26.PrjPcb "power.SchDoc"
cargo run -p altium-cli -- prjpcb add-document /tmp/test_flow26.PrjPcb "board.PcbDoc"

# 6. List documents
cargo run -p altium-cli -- prjpcb documents /tmp/test_flow26.PrjPcb

# 7. Remove a document
cargo run -p altium-cli -- prjpcb remove-document /tmp/test_flow26.PrjPcb "power.SchDoc"

# 8. Remove a parameter
cargo run -p altium-cli -- prjpcb remove-parameter /tmp/test_flow26.PrjPcb "Company"

# 9. Verify final state
cargo run -p altium-cli -- prjpcb overview /tmp/test_flow26.PrjPcb
cargo run -p altium-cli -- prjpcb documents /tmp/test_flow26.PrjPcb
cargo run -p altium-cli -- prjpcb parameters /tmp/test_flow26.PrjPcb

# 10. Validate project
cargo run -p altium-cli -- prjpcb validate /tmp/test_flow26.PrjPcb

# 11. Export JSON
cargo run -p altium-cli -- prjpcb json /tmp/test_flow26.PrjPcb --pretty
```

**Pass criteria:** Project created. Parameters added/removed. Documents added/removed. Final state shows 2 documents and 2 parameters.

---

## Flow 27: PrjPcb — Read Existing Project and BOM

**Goal:** Read the sample project file and test analysis commands.

```bash
# 1. Overview
cargo run -p altium-cli -- prjpcb overview crates/altium-format/data/Project1.PrjPcb

# 2. Info
cargo run -p altium-cli -- prjpcb info crates/altium-format/data/Project1.PrjPcb

# 3. List documents
cargo run -p altium-cli -- prjpcb documents crates/altium-format/data/Project1.PrjPcb

# 4. Filter documents by type
cargo run -p altium-cli -- prjpcb documents crates/altium-format/data/Project1.PrjPcb -t SchDoc

# 5. Parameters
cargo run -p altium-cli -- prjpcb parameters crates/altium-format/data/Project1.PrjPcb

# 6. JSON export
cargo run -p altium-cli -- prjpcb json crates/altium-format/data/Project1.PrjPcb --pretty
```

**Pass criteria:** All commands produce output without errors. Documents and parameters are listed from the sample project.

---

## Flow 28: Query — Record Selector Syntax

**Goal:** Test all record selector pattern types against a populated schematic.

```bash
# 1. Build a test schematic with known components
cargo run -p altium-cli -- schlib create /tmp/test_flow28.SchLib
cargo run -p altium-cli -- schlib gen-ic /tmp/test_flow28.SchLib "LM7805" --pins "VIN,GND,VOUT"
cargo run -p altium-cli -- schlib gen-ic /tmp/test_flow28.SchLib "RESISTOR" --pins "1,2"
cargo run -p altium-cli -- schdoc create /tmp/test_flow28.SchDoc
cargo run -p altium-cli -- edit /tmp/test_flow28.SchDoc \
  -c "add-component /tmp/test_flow28.SchLib LM7805 300 300" \
  -o /tmp/test_flow28_s1.SchDoc
cargo run -p altium-cli -- edit /tmp/test_flow28_s1.SchDoc \
  -c "add-component /tmp/test_flow28.SchLib RESISTOR 600 300" \
  -o /tmp/test_flow28_s2.SchDoc
cargo run -p altium-cli -- edit /tmp/test_flow28_s2.SchDoc \
  -c "add-component /tmp/test_flow28.SchLib RESISTOR 600 500" \
  -o /tmp/test_flow28_s3.SchDoc
cargo run -p altium-cli -- edit /tmp/test_flow28_s3.SchDoc \
  -c "add-net-label VCC 300 400" -o /tmp/test_flow28_s4.SchDoc

# 2. Query by exact designator
cargo run -p altium-cli -- query /tmp/test_flow28_s4.SchDoc "U1"

# 3. Query by wildcard prefix
cargo run -p altium-cli -- query /tmp/test_flow28_s4.SchDoc "R*"

# 4. Query by part name
cargo run -p altium-cli -- query /tmp/test_flow28_s4.SchDoc '$LM7805'

# 5. Query by net name
cargo run -p altium-cli -- query /tmp/test_flow28_s4.SchDoc "~VCC"

# 6. Query pin by component:pin
cargo run -p altium-cli -- query /tmp/test_flow28_s4.SchDoc "U1:VIN"
```

**Pass criteria:** Each selector returns the expected match(es). `U1` returns 1 component, `R*` returns 2, `$LM7805` returns 1, `~VCC` returns VCC net, `U1:VIN` returns pin.

---

## Flow 29: Query — SchQL CSS-Like Selectors

**Goal:** Test the SchQL query language with attribute selectors, pseudo-selectors, and combinators.

```bash
# (Use the schematic built in Flow 28 — /tmp/test_flow28_s4.SchDoc)

# 1. Select all components
cargo run -p altium-cli -- query /tmp/test_flow28_s4.SchDoc "component"

# 2. Select by ID
cargo run -p altium-cli -- query /tmp/test_flow28_s4.SchDoc "#U1"

# 3. Universal selector
cargo run -p altium-cli -- query /tmp/test_flow28_s4.SchDoc "*"

# 4. Attribute equals
cargo run -p altium-cli -- query /tmp/test_flow28_s4.SchDoc 'component[part=LM7805]'

# 5. Attribute starts-with
cargo run -p altium-cli -- query /tmp/test_flow28_s4.SchDoc 'component[part^=LM]'

# 6. Select pins
cargo run -p altium-cli -- query /tmp/test_flow28_s4.SchDoc "pin"

# 7. Pseudo-selector: power nets
cargo run -p altium-cli -- query /tmp/test_flow28_s4.SchDoc "net:power"

# 8. Count pseudo-selector
cargo run -p altium-cli -- query /tmp/test_flow28_s4.SchDoc "component:count"

# 9. Limit results
cargo run -p altium-cli -- query /tmp/test_flow28_s4.SchDoc "pin:limit(3)"
```

**Pass criteria:** Each query returns semantically correct results. `component` returns 3, `#U1` returns 1, `component:count` returns count of 3, `:limit(3)` returns max 3 results.

---

## Flow 30: Inspect, JSON Output Modes, and Shell Completions

**Goal:** Test cross-cutting features: file inspection, global output flags, and shell completions.

```bash
# 1. Inspect a SchLib
cargo run -p altium-cli -- inspect crates/altium-format/data/blank/Schlib1.SchLib

# 2. Inspect a PcbLib
cargo run -p altium-cli -- inspect crates/altium-format/data/blank/PcbLib1.PcbLib

# 3. Inspect a PcbDoc
cargo run -p altium-cli -- inspect crates/altium-format/data/PCB1.PcbDoc

# 4. Test --json flag on schlib list
cargo run -p altium-cli -- schlib create /tmp/test_flow30.SchLib
cargo run -p altium-cli -- schlib add-component /tmp/test_flow30.SchLib "TestIC" -d "Test"
cargo run -p altium-cli -- schlib list /tmp/test_flow30.SchLib --json

# 5. Test --pretty flag
cargo run -p altium-cli -- schlib list /tmp/test_flow30.SchLib --pretty

# 6. Test --verbose flag on pcbdoc
cargo run -p altium-cli -- pcbdoc components crates/altium-format/data/PCB1.PcbDoc --verbose

# 7. Test --json on pcbdoc
cargo run -p altium-cli -- pcbdoc overview crates/altium-format/data/PCB1.PcbDoc --json

# 8. Generate shell completions
cargo run -p altium-cli -- completions bash > /tmp/altium.bash
cargo run -p altium-cli -- completions zsh > /tmp/altium.zsh
cargo run -p altium-cli -- completions fish > /tmp/altium.fish

# 9. Verify completions files are non-empty
ls -la /tmp/altium.bash /tmp/altium.zsh /tmp/altium.fish

# 10. Run unit tests
cargo test --workspace
```

**Pass criteria:** Inspect shows file structure for all formats. JSON/pretty flags produce valid JSON. Shell completion files generated and non-empty. All unit tests pass.

---

## Summary by Category

| Category | Flows | Coverage |
|----------|-------|----------|
| **SchLib** (create, components, pins, primitives, gen-ic, search, JSON) | 1–6 | 6 flows |
| **PcbLib** (create, footprints, pads, gen-chip, rows, render, JSON) | 7–12 | 6 flows |
| **SchDoc** (create, edit-add, edit-wire, edit-move, edit-port, edit-validate, analysis, hierarchy) | 13–20 | 8 flows |
| **PcbDoc** (inspect, create, components, tracks, keepouts, rules) | 21–25 | 5 flows |
| **PrjPcb** (create, manage, read) | 26–27 | 2 flows |
| **Query** (record selectors, SchQL) | 28–29 | 2 flows |
| **Cross-cutting** (inspect, output flags, completions, tests) | 30 | 1 flow |

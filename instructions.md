# CLAUDE.md

## Purpose

This document defines the end-to-end operating process for an LLM-driven electronics design workflow. It is intended for autonomous or semi-autonomous agents working with:

* `datasheet-cli` for part research, datasheet extraction, and design-rule extraction
* `altium-cli` for library generation, schematic capture support, BOM generation, and PCB data manipulation
* external simulation, calculation, and firmware tooling as needed

The workflow assumes agents can:

* research components
* read and summarize datasheets
* generate symbols and footprints
* create and review schematics
* produce placement guidance and routing constraints
* generate firmware scaffolding
* produce manufacturing outputs
* perform structured reviews at every stage

The workflow does **not** assume blind trust in a single model. Every stage includes explicit review, contradiction checks, and independent verification by other agents or models.

---

# Core Principles

## 1. No silent assumptions

Every nontrivial electrical decision must be stated explicitly:

* input voltage range
* load current
* thermal assumptions
* signal speed
* EMC sensitivity
* environmental conditions
* manufacturing constraints
* firmware update method
* connector orientation and use case
* assembly assumptions

If an assumption is necessary, it must be recorded in the design log.

## 2. Every stage produces artifacts

Each stage must emit concrete outputs, not just narrative text.

Examples:

* requirement matrix
* block diagram
* candidate parts table
* part decision memo
* pin allocation table
* net-class table
* schematic review checklist
* footprint verification report
* PCB placement constraints
* DFM/assembly checklist
* firmware bring-up checklist
* manufacturing release checklist

## 3. Every stage has entry and exit criteria

The agent must not proceed just because it “feels done.” Each stage has explicit completion gates.

## 4. Mandatory independent review

At least two distinct review passes should occur for important stages:

1. a primary design agent that creates the artifact
2. one or more reviewer agents that critique it independently

Ideal pattern:

* **Builder model**: strongest at synthesis and structured output
* **Electrical reviewer model**: strongest at domain correctness and edge cases
* **Adversarial reviewer model**: tries to break assumptions and find omissions
* **Manufacturing reviewer model**: focuses on fab, assembly, test, and reliability
* **Firmware reviewer model**: checks boot mode, pin mux, debugging, programming, and bring-up implications

## 5. Prefer reversible decisions early

During early design:

* delay mechanical lock-in when possible
* prefer common footprints and broad-source parts
* keep optional circuitry stuffed/DNP where uncertainty exists
* add measurement points and configuration options early

## 6. Optimize for debug, not just minimal BOM

A board that is slightly more expensive but easy to validate is often superior.

Prefer including:

* test points
* current-sense jumpers or shunts
* LED indicators
* straps for boot/config
* series resistors for risky interfaces
* stuffing options
* isolation of subsystems

## 7. Treat datasheets as source of truth, but verify interpretation

Never rely on vendor marketing copy alone. Extract requirements from:

* absolute maximum ratings
* recommended operating conditions
* pin descriptions
* application schematics
  n- layout guidance
* thermal information
* timing diagrams
* boot/reset requirements
* programming/debug requirements
* package land pattern recommendations

Cross-check unclear points across:

* datasheet revisions
* reference designs
* eval board schematics/layouts
* application notes
* errata

## 8. Separate facts, calculations, and judgments

For every design decision, record:

* **Facts**: directly supported by datasheet or measurement
* **Calculations**: derived values and assumptions
* **Judgments**: tradeoffs or selected direction

---

# Global Workflow Overview

1. Product framing and requirements
2. System architecture and block diagram
3. Part research and selection
4. Symbol, footprint, and CAD library generation
5. Schematic capture
6. Electrical review and risk analysis
7. PCB stackup, constraints, and floorplanning
8. Placement
9. Routing
10. PCB review: SI, PI, EMC, thermal, and manufacturability
11. Prototype build preparation
12. Firmware scaffolding and bring-up planning
13. Assembly, bring-up, and validation
14. Design iteration
15. Manufacturing release
16. Post-release learnings and library improvement

---

# Repository Structure

Suggested structure:

```text
project/
  CLAUDE.md
  docs/
    requirements.md
    architecture.md
    design_log.md
    risk_register.md
    review/
      01_requirements_review.md
      02_part_selection_review.md
      03_schematic_review.md
      04_layout_review.md
      05_dfm_review.md
      06_firmware_review.md
    prompts/
      01_requirements_prompt.md
      02_architecture_prompt.md
      03_part_selection_prompt.md
      04_symbol_footprint_prompt.md
      05_schematic_prompt.md
      06_schematic_review_prompt.md
      07_floorplan_prompt.md
      08_placement_review_prompt.md
      09_routing_review_prompt.md
      10_firmware_prompt.md
      11_bringup_prompt.md
      12_dfm_release_prompt.md
  data/
    parts/
    datasheets/
    extracted/
    supplier_snapshots/
  cad/
    libraries/
    schematic/
    pcb/
    outputs/
  firmware/
  test/
    bringup/
    validation/
  manufacturing/
    bom/
    pick_place/
    fab/
    assembly_drawings/
```

---

# Stage 1: Product Framing and Requirements

## Goal

Turn a vague idea into a constrained engineering problem.

## Inputs

* product concept
* target user
* rough cost target
* rough size constraints
* rough power source
* rough feature list

## Outputs

* requirements document
* non-requirements list
* risk register v1
* acceptance criteria
* block diagram draft

## Required questions

The agent must force explicit answers for:

### Functional

* What must the product do?
* What is core vs optional?
* What loads, sensors, radios, or interfaces are involved?
* What must happen on power-up, reset, fault, and brownout?
* What are the expected operating modes?

### Electrical

* Supply source and allowed range?
* Peak, average, startup, and sleep current?
* Battery or mains or external DC?
* Isolation needed?
* Any hot-plug or reverse-polarity scenarios?

### Mechanical

* Board dimensions?
* Connectors at edges?
* Mounting holes?
* Enclosure constraints?
* Waterproofing or conformal coating?

### Environmental and reliability

* Indoor/outdoor?
* Temperature range?
* Vibration?
* ESD exposure?
* Moisture?
* Expected lifetime?

### Firmware and UX

* OTA required?
* Debug/programming connector?
* Factory test mode?
* LEDs/buttons?
* Recovery mode?

### Manufacturing and business

* Prototype quantity?
* Expected production volume?
* PCB layer/cost target?
* Assembly constraints?
* Single-source acceptable or not?

## Exit criteria

Do not proceed until the requirements document includes:

* measurable voltage/current ranges
* measurable I/O or performance targets
* clear environmental assumptions
* explicit cost and complexity targets
* known unknowns listed separately

## Common failure modes

* designing before clarifying power budget
* not defining worst-case load conditions
* forgetting manufacturing/test constraints
* ignoring enclosure and connector orientation
* no plan for programming and recovery

## Deliverable template

### Requirements Matrix

| ID | Requirement                             | Type                | Priority | Verification Method | Notes                      |
| -- | --------------------------------------- | ------------------- | -------- | ------------------- | -------------------------- |
| R1 | Device accepts 9–28 V DC input          | Electrical          | Must     | Bench test          | Reverse polarity protected |
| R2 | MCU enters low-power sleep under 500 uA | Electrical/Firmware | Should   | Current measurement | Radio off                  |
| R3 | Board fits 60 x 40 mm                   | Mechanical          | Must     | Mechanical check    | Excluding cable overhang   |

---

# Stage 2: System Architecture and Block Diagram

## Goal

Partition the system before selecting exact parts.

## Outputs

* top-level block diagram
* power tree
* interface map
* high-risk area list
* initial pin budget

## Tasks

1. Identify subsystems:

   * input protection
   * power conversion
   * MCU/SoC
   * sensors
   * comms
   * outputs/actuation
   * storage
   * user interface
   * debug/programming
2. Create power tree with each rail:

   * nominal voltage
   * tolerance
   * peak current
   * sequencing dependencies
3. Map interfaces:

   * UART, I2C, SPI, USB, CAN, RS-485, ADC, PWM, GPIO, etc.
4. Flag special signals:

   * high current
   * high dV/dt
   * clocks
   * RF
   * analog
   * boot strapping pins
5. Identify architectural alternatives.

## Exit criteria

* each subsystem has defined responsibility
* each rail has current estimate
* each interface has designated owner/peripheral
* pin budget closes with margin
* high-risk domains explicitly called out

## Common failure modes

* selecting MCU before pin budgeting
* no explicit power tree
* mixing noisy and sensitive domains carelessly
* not reserving pins for debug, reset, boot, interrupts

---

# Stage 3: Part Research and Selection

## Goal

Select real manufacturable parts with explicit justification.

## Outputs

* candidate parts table per function
* chosen part list with rationale
* sourcing/risk notes
* electrical calculation sheet
* lifecycle/procurement notes

## General rules

* Prefer parts with solid datasheets and reference designs.
* Prefer packages you can assemble/debug at your scale.
* Prefer multi-source or second-source options when practical.
* Prefer common passives and commodity regulators unless a special part is truly justified.
* Check supply availability and lifecycle early.
* Check eval boards or open designs using the part.

## Per-part review checklist

For every selected IC or module, capture:

* manufacturer and exact MPN
* package and pitch
* operating voltage range
* I/O voltage domain
* max current consumption and startup behavior
* thermal limits and package dissipation concerns
* required external components
* mandatory layout guidance
* boot/reset requirements
* configuration pins
* programming/debug needs
* known errata or caveats
* supply-chain status
* alternates

## Selection dimensions

### Technical fit

* meets electrical specs
* enough margin
* correct temp range
* correct interface support
* acceptable package

### Firmware fit

* toolchain support
* known SDK maturity
* bootloader/update path
* debugging support

### Manufacturing fit

* assembly yield risk
* footprint availability
* testability
* moisture sensitivity concerns

### Business fit

* cost
* availability
* lifecycle
* counterfeit risk
* volume pricing slope

## Calculations required

At minimum, the agent should compute or estimate:

* input surge and protection sizing
* regulator dissipation
* inductor sizing
* bulk and decoupling capacitor needs
* resistor divider currents and tolerance impact
* MOSFET gate drive and losses if applicable
* LED resistor values
* pull-up/pull-down strengths
* crystal/load capacitor values if external crystal used
* battery life estimates if battery-powered

## Exit criteria

Do not freeze parts until:

* selected parts cover all functions
* no unresolved voltage-domain mismatch
* regulator thermal dissipation has been checked
* startup/boot dependencies are understood
* every critical part has a sourcing note
* land pattern source is identified or will be generated

## Common failure modes

* picking parts from parametric tables without reading datasheets
* ignoring startup current or inrush
* missing required compensation networks or passives
* overlooking package thermal limits
* using fragile or obscure parts with no sourcing margin

## Deliverable template

### Candidate Parts Table

| Function   | Candidate | Pros                      | Cons                                               | Cost   | Availability | Verdict  |
| ---------- | --------- | ------------------------- | -------------------------------------------------- | ------ | ------------ | -------- |
| 3.3 V buck | TPS54202H | Simple, common, good docs | External catch diode not needed but layout matters | Medium | Good         | Finalist |
| 3.3 V buck | AP63203   | Cheap, integrated, common | Need thermal check                                 | Low    | Good         | Finalist |

### Part Decision Memo

For each selected part:

* Why this part was chosen
* Why alternatives were rejected
* What assumptions it depends on
* What must be validated in prototype

---

# Stage 4: Symbols, Footprints, and CAD Library Generation

## Goal

Create trustworthy CAD library objects.

## Outputs

* symbol
* footprint
* 3D model reference if available
* pin mapping verification report
* land pattern source note

## Rules

* Never trust scraped symbols or footprints without review.
* Symbols must reflect functional readability, not just raw pin order.
* Footprints must be traceable to IPC, datasheet land pattern, or a known trusted library.
* Pin 1, polarity, exposed pad, and mechanical keepouts must be explicit.

## Symbol checklist

* exact MPN in metadata
* correct pin names and numbers
* hidden power pins forbidden unless intentionally justified
* pins grouped logically
* passive pin types marked correctly
* NC pins handled explicitly
* boot/config pins labeled clearly
* alternate functions documented where relevant

## Footprint checklist

* package matches exact datasheet package code
* pitch and pad dimensions verified
* courtyard and solder mask checked
* paste reduction rules considered for thermal pads/QFNs
* exposed pad via strategy noted if used
* polarity and pin-1 marking visible
* connector outline, overhang, and mating clearance checked
* mounting hole dimensions and plating status explicit

## Review requirements

Each library item gets:

1. automated extraction pass
2. human-readable verification table
3. independent reviewer pass against datasheet pages

## Exit criteria

* symbol pin numbers match datasheet exactly
* footprint dimensions match source exactly
* 3D/mechanical assumptions recorded
* high-risk packages independently checked

## Common failure modes

* confusing package variants with same name family
* pin swaps from OCR or PDF parsing errors
* missing exposed pad connectivity
* incorrect connector orientation from top/bottom view confusion

---

# Stage 5: Schematic Capture

## Goal

Encode the design unambiguously and readably.

## Outputs

* schematic sheets
* net labels and port structure
* BOM draft
* rail naming convention
* pin allocation table

## Schematic rules

### Readability

* organize by function, not by placement on board
* one sheet per logical subsystem when complexity warrants
* left-to-right or top-to-bottom signal flow
* consistent rail names
* no spaghetti wiring when ports/net labels are cleaner

### Electrical correctness

* every supply pin connected explicitly
* every regulator has required passives from datasheet
* every MCU boot/reset/programming dependency shown
* all connector pins accounted for
* all unused pins intentionally tied, left NC, or documented
* decoupling caps placed in schematic near consuming IC section
* protection components shown at interfaces

### DFM/test-minded additions

* test points on key rails and buses
* programming header or pads
* current measurement options for key rails
* strap resistors or jumpers where useful
* indicator LEDs where they speed bring-up

## Required schematic sub-checklists

### Power

* input polarity protection
* TVS if needed
* fuse/PTC if needed
* buck/linear regulator components complete
* feedback network correct
* compensation network correct if external
* enable sequencing correct
* bulk and local decoupling present

### MCU/SoC

* all power pins connected
* all grounds connected correctly
* decoupling complete
* reset circuit valid
* boot strapping valid
* crystal/oscillator circuit valid if used
* programming/debug pins accessible
* flash/PSRAM requirements met if external

### Interfaces

* pull-ups/pull-downs correct
* line protection where needed
* level shifting where needed
* termination/biasing for bus interfaces where needed
* connector pinout checked against real cables and mating parts

## Exit criteria

* ERC clean or all exceptions justified
* BOM draft complete enough for review
* pin allocation table reconciled with schematic
* power-up path understood
* every net of consequence named and classified

## Common failure modes

* missing enable pull resistor
* wrong reset polarity or RC timing
* incorrect I2C pull-up values
* forgetting exposed pad to ground
* connectors mirrored accidentally
* debug header present but unusable in-circuit

---

# Stage 6: Electrical Review and Risk Analysis

## Goal

Attempt to prove the schematic wrong before layout begins.

## Outputs

* schematic review report
* issue list
* resolved/unresolved risks
* prototype validation matrix

## Review modes

### Builder self-review

The design agent checks:

* datasheet compliance
* power integrity basics
* missing components
* obvious net/pin issues

### Independent electrical review

A different model reviews only the outputs, not the builder rationale first.

### Adversarial review

Ask a reviewer to find the top 20 ways the board might fail:

* not boot
* brown out
* overheat
* fail EMC
* damage I/O
* be unprogrammable
* be impossible to assemble

### Firmware bring-up review

Review pin mux, boot modes, flashing access, strapping conflicts, interrupts, and logging/debug practicality.

## Required review outputs

* critical issues
* major issues
* minor issues
* assumptions needing bench validation
* recommended schematic improvements

## Exit criteria

* all critical issues resolved
* major unresolved issues explicitly accepted and tracked
* prototype validation tests mapped to risks

---

# Stage 7: PCB Stackup, Constraints, and Floorplanning

## Goal

Convert schematic into a layout strategy before moving parts.

## Outputs

* proposed layer stackup
* net classes
* impedance/width guidance if relevant
* keepout strategy
* placement regions
* return current and grounding plan

## Tasks

1. Choose layer count based on:

   * current
   * density
   * EMI risk
   * routing complexity
   * cost target
2. Define classes:

   * power
   * analog
   * digital low-speed
   * digital high-speed
   * RF
   * high current
   * noisy switching
3. Define board regions:

   * power entry
   * power conversion
   * MCU/control
   * RF/antenna
   * sensors/analog
   * connectors and cable exits
4. Define grounding approach:

   * continuous plane preference
   * split ground only if truly justified
   * chassis/shield connection strategy if applicable
5. Define critical loops and keep them small:

   * buck converter hot loop
   * gate drive loops
   * crystal loops
   * high di/dt returns

## Exit criteria

* layer count justified
* critical nets/classes defined
* placement strategy written before freeform placement begins
* grounding and return-current plan explicit

## Common failure modes

* starting placement without identifying hot loops
* splitting grounds unnecessarily
* putting connectors or antennas where enclosure blocks them
* no reserved area for programming/test access

---

# Stage 8: Placement

## Goal

Place components according to electrical priority, not aesthetics.

## Placement order

1. Board outline, mounting holes, fixed connectors
2. Mechanical no-go zones
3. Power entry and protection
4. Regulators and their passives
5. MCU/SoC and mandatory support parts
6. clocks/crystals
7. memory
8. sensitive analog and sensors
9. user I/O and support circuitry
10. remaining passives and optional parts

## Placement rules

### Power

* place regulator input/output caps per datasheet intent
* minimize hot loop area
* keep feedback away from noisy switch nodes
* keep high current paths short and wide

### MCU

* decouplers adjacent to supply pins
* crystal tight and symmetric if used
* boot/reset/programming pins accessible
* keep noisy power away from analog refs

### Connectors

* confirm board-edge orientation against enclosure use
* verify top/bottom mating direction
* allow cable bend radius and tool access

### RF and analog

* isolate from switching nodes
* protect antenna region
* keep analog front-end compact
* avoid digital traces crossing sensitive regions

### Testability

* test points accessible
* probes can physically reach them
* bed-of-nails or pogo strategy considered if relevant

## Exit criteria

* all critical placement rules met
* no major datasheet placement violation
* assembly access acceptable
* test/programming access preserved

## Common failure modes

* decouplers technically connected but physically too far
* crystal under noisy traces or planes cut incorrectly
* regulator feedback routed through noise
* connectors placed for board convenience rather than actual use

---

# Stage 9: Routing

## Goal

Route to preserve power integrity, signal integrity, and manufacturability.

## Routing order

1. Critical power paths
2. Sensitive analog and clocks
3. High-speed or timing-sensitive interfaces
4. Remaining signals
5. Fill/plane tuning

## Routing rules

### Power integrity

* route current loops first
* use planes where appropriate
* avoid neck-downs on high-current paths
* verify return path continuity
* place vias strategically, not decoratively

### Signal integrity

* keep clocks short
* match lengths only where needed
* avoid stubs on sensitive nets
* do not cross plane splits with return-dependent signals
* control impedance only when actually required

### Noise control

* keep switch node copper tight
* separate switching node from feedback and sensitive traces
* avoid long parallel runs between aggressors and victims

### Manufacturability

* avoid acid traps and awkward slivers
* keep annular rings reasonable
* avoid impossible soldering access for hand-assembled prototypes when that matters
* minimize unnecessary vias under parts unless planned

## Exit criteria

* DRC clean or all exceptions justified
* critical loops and returns reviewed visually
* no unexamined autorouter artifacts
* silkscreen readable enough for assembly/debug

## Common failure modes

* assuming routed means correct
* prioritizing shortest route over quiet route
* hiding weak grounding under copper fills that do not actually return well
* forgetting copper balance or thermal relief implications

---

# Stage 10: PCB Review

## Goal

Review the board as a physical object, not just a CAD file.

## Review categories

### Electrical

* placement/routing consistent with schematic intent
* no obvious SI/PI mistakes
* decoupling physically effective
* thermal escape paths adequate

### Manufacturing

* fab rules compatible with chosen fab
* assembly spacing acceptable
* fiducials if needed
* panelization concerns noted
* stencil/paste concerns noted

### Mechanical

* connector overhangs correct
* mounting clearances correct
* keepouts respected
* enclosure fit assumptions checked

### Bring-up/debug

* test points accessible
* SWD/JTAG/UART reachable
* status LEDs visible
* rails measurable

### Reliability

* creepage/clearance where relevant
* high-stress components have margin
* moisture/contamination concerns considered
* mounting-induced stress minimized

## Exit criteria

* review report complete
* prototype blockers resolved
* manufacturing notes prepared

---

# Stage 11: Prototype Build Preparation

## Outputs

* BOM with manufacturer part numbers
* approved alternates list
* assembly notes
* fab notes
* test point map
* bring-up plan

## Tasks

* mark DNP options clearly
* verify footprints against assembly method
* verify package orientations visually
* verify polarity marks on PCB and assembly docs
* generate centroid/pick-place and compare against expected orientations
* create first-article checklist

## Common failure modes

* BOM references vague distributor SKUs instead of MPNs
* polarity or orientation mismatches between library and pick-place
* assembly house gets insufficient notes about special parts

---

# Stage 12: Firmware Scaffolding and Bring-Up Planning

## Goal

Ensure the hardware is actually operable on day one.

## Outputs

* pin map header/source files
* board support package scaffold
* startup/init sequence plan
* manufacturing test firmware plan
* bring-up checklist

## Required firmware-aware outputs

* exact GPIO assignment table
* boot strapping table
* default pin states at reset
* safe-state behavior for outputs
* flashing/debug procedure
* clock source configuration plan
* peripheral ownership map
* logging/diagnostic output plan

## Bring-up checklist example

1. Visual inspection
2. Check shorts on main rails
3. Power board from current-limited supply
4. Measure each rail without MCU active if possible
5. Verify reset and boot pins
6. Flash minimal LED/UART firmware
7. Validate clocks
8. Validate current in idle and active modes
9. Validate each interface incrementally
10. Validate thermal behavior under worst case

## Common failure modes

* hardware selected pins that conflict with boot mode
* firmware assumes missing pull-ups or wrong polarity
* no manufacturing test fixture strategy

---

# Stage 13: Assembly, Bring-Up, and Validation

## Goal

Turn prototype data into design truth.

## Outputs

* bring-up log
* validation results
* issue tracker updates
* ECO list

## Validation categories

* power-up and startup robustness
* normal mode operation
* low-power behavior
* thermal performance
* EMC-preliminary observations
* programming/recovery behavior
* connector and user interaction tests
* fault injection/basic abuse cases

## Required logging

For every anomaly record:

* symptom
* reproduction steps
* suspected cause
* measurement data
* fix recommendation

---

# Stage 14: Iteration and ECOs

## Rules

* every board change gets rationale
* avoid stealth edits
* compare BOM, netlist, and outputs between revisions
* maintain rev-specific validation notes

---

# Stage 15: Manufacturing Release

## Release contents

* fabrication files
* assembly files
* BOM with approved alternates
* assembly drawings
* test instructions
* programming instructions
* revision notes
* known limitations

## Release gate

Do not release until:

* all review reports attached
* prototype issues dispositioned
* manufacturing outputs regenerated from frozen sources
* part substitutions reviewed for electrical impact

---

# Stage 16: Post-Release Learning Loop

Capture lessons into reusable libraries and prompts:

* footprint fixes
* preferred regulators
* known-good ESP32 support circuits
* common checklist additions
* DFM findings by fab/assembler
* firmware bring-up gotchas

---

# Review Framework

Use multiple agents with explicit roles.

## Role A: Builder

Responsible for generating the primary artifact.

### Responsibilities

* produce structured deliverable
* cite datasheet-derived facts
* list assumptions
* identify open questions

## Role B: Electrical Reviewer

Reviews for correctness and hidden electrical issues.

### Responsibilities

* challenge assumptions
* inspect datasheet compliance
* find missing support circuitry
* check margins and failure modes

## Role C: Adversarial Reviewer

Attempts to break the design.

### Responsibilities

* enumerate likely prototype failures
* identify contradictions and edge cases
* detect overlooked environmental/manufacturing problems

## Role D: Manufacturing Reviewer

Checks prototype and production practicality.

### Responsibilities

* inspect package risk
  n- check hand-assembly feasibility if relevant
* inspect DFM, DFA, testability
* flag sourcing fragility

## Role E: Firmware Reviewer

Checks operability from reset through field updates.

### Responsibilities

* check boot pins
* check flash/debug accessibility
* check default states and safe outputs
* check test firmware feasibility

---

# Standard Review Rubric

For every review, require this output format:

## Summary

* overall risk: low / medium / high
* proceed / proceed with changes / block

## Findings

### Critical

Items likely to cause nonfunctional hardware, damage, unsafe behavior, or severe rework.

### Major

Items likely to degrade reliability, manufacturability, or performance.

### Minor

Readability, cleanup, margin improvements.

## Validation requests

Bench tests or simulations needed to retire uncertainty.

## Confidence assessment

What the reviewer is confident about vs not.

---

# Required Artifacts by Stage

| Stage                    | Required files                                             |
| ------------------------ | ---------------------------------------------------------- |
| Requirements             | `docs/requirements.md`, `docs/risk_register.md`            |
| Architecture             | `docs/architecture.md`                                     |
| Part selection           | `data/parts/selection_table.csv`, `docs/part_decisions.md` |
| Libraries                | `cad/libraries/*`, `docs/library_review.md`                |
| Schematic                | `cad/schematic/*`, `docs/pin_allocation.md`                |
| Schematic review         | `docs/review/03_schematic_review.md`                       |
| Layout planning          | `docs/layout_strategy.md`                                  |
| Placement/routing review | `docs/review/04_layout_review.md`                          |
| Firmware                 | `firmware/board_support/*`, `docs/bringup_plan.md`         |
| Manufacturing            | `manufacturing/*`, `docs/release_checklist.md`             |

---

# Prompt Library

These prompts are intended for agent use. Replace bracketed sections with project-specific data.

## Prompt 1: Requirements Extraction

```text
You are the requirements-definition agent for a new PCB-based electronic product.

Project concept:
[INSERT PRODUCT IDEA]

Your task:
1. Convert the concept into a structured engineering requirements document.
2. Separate hard requirements from preferences and open questions.
3. Produce a requirements matrix with measurable targets.
4. Produce a non-requirements list.
5. Produce an initial risk register.
6. Identify missing information and assumptions explicitly.

Constraints:
- Do not design the circuit yet.
- Do not pick exact parts yet.
- Force specificity for voltage, current, interfaces, environment, size, manufacturing, and firmware update/debug needs.
- State facts, assumptions, and open questions separately.

Output format:
- Executive summary
- Requirements matrix
- Non-requirements
- Assumptions
- Open questions
- Risk register
- Exit checklist
```

## Prompt 2: System Architecture

```text
You are the system-architecture agent for a PCB design.

Inputs:
- Requirements document: [INSERT]

Your task:
1. Propose a subsystem block diagram.
2. Define the power tree.
3. Define the interface map.
4. Produce an initial pin budget.
5. Identify architectural alternatives and tradeoffs.
6. Identify high-risk subsystems and why.

Rules:
- Do not freeze exact parts unless required to explain architecture.
- Call out assumptions and unresolved risks explicitly.
- Optimize for debuggability and manufacturability.

Output format:
- Architecture summary
- Block diagram description
- Power tree table
- Interface map
- Pin budget table
- Alternatives considered
- Risks and unknowns
- Exit checklist
```

## Prompt 3: Part Selection

```text
You are the component-selection agent for a PCB design.

Inputs:
- Requirements: [INSERT]
- Architecture: [INSERT]
- Preferred vendors or sourcing constraints: [INSERT]

Your task:
1. For each major function, propose candidate parts.
2. Compare them across electrical fit, firmware support, package risk, availability, and cost.
3. Recommend final selections.
4. List required external support components for each selected IC.
5. List prototype validation concerns for each selected IC.
6. Highlight sourcing and lifecycle risks.

Rules:
- Read actual datasheet constraints, not marketing summaries.
- Explicitly check voltage domains, startup behavior, thermal limits, and package suitability.
- Prefer common and well-supported parts where possible.

Output format:
- Candidate comparison tables by function
- Selected parts table
- Part decision memo
- Required support circuitry checklist
- Risk notes
- Exit checklist
```

## Prompt 4: Symbol and Footprint Generation

```text
You are the CAD library generation agent.

Inputs:
- Selected parts list: [INSERT]
- Datasheets and package drawings: [INSERT]

Your task:
1. Generate or validate symbols for each selected part.
2. Generate or validate footprints for each selected part.
3. Verify pin numbering, naming, polarity, exposed pad handling, and package dimensions.
4. Produce a verification report for every library item.

Rules:
- Treat package variants as hazardous until proven exact.
- Do not trust OCR or scraped data without cross-checking against datasheet tables and package drawings.
- Hidden power pins are disallowed unless explicitly justified.

Output format:
- Symbol generation log
- Footprint generation log
- Verification table per part
- High-risk items requiring manual review
- Exit checklist
```

## Prompt 5: Schematic Capture

```text
You are the schematic-design agent.

Inputs:
- Requirements: [INSERT]
- Architecture: [INSERT]
- Selected parts: [INSERT]
- Library items: [INSERT]

Your task:
1. Produce the full schematic structure by functional sheets.
2. Define net names, rail names, and subsystem boundaries.
3. Ensure all required support circuitry is present.
4. Add test points, programming access, and debug aids.
5. Produce a BOM draft and pin allocation table.

Rules:
- Readability matters.
- Every power pin must be explicit.
- Every MCU boot/reset/programming requirement must be shown.
- All unused pins must be intentionally handled.
- Include protection and bring-up aids where justified.

Output format:
- Schematic sheet plan
- Detailed circuit description by sheet
- Pin allocation table
- BOM draft
- Test/debug features list
- Assumptions and open issues
- Exit checklist
```

## Prompt 6: Schematic Review

```text
You are an independent electrical reviewer. You did not design the schematic.

Inputs:
- Requirements: [INSERT]
- Architecture: [INSERT]
- Selected parts: [INSERT]
- Schematic description or netlist: [INSERT]

Your task:
1. Find critical, major, and minor issues.
2. Check datasheet compliance.
3. Check power, reset, boot, programming, and interface correctness.
4. Identify missing support circuitry.
5. Identify prototype risks and required bench validation tests.

Be adversarial. Assume the design is wrong until proven otherwise.

Output format:
- Overall assessment
- Critical findings
- Major findings
- Minor findings
- Required validation tests
- Confidence limits
- Go/no-go recommendation
```

## Prompt 7: Floorplanning and Constraints

```text
You are the PCB floorplanning agent.

Inputs:
- Schematic: [INSERT]
- Mechanical constraints: [INSERT]
- Selected parts and package info: [INSERT]

Your task:
1. Propose the layer count and stackup strategy.
2. Define net classes and routing constraints.
3. Define board regions and placement priorities.
4. Identify critical loops, noisy regions, and sensitive regions.
5. Define grounding and return-current strategy.

Output format:
- Stackup recommendation
- Net class table
- Board region plan
- Critical placement constraints
- Return-current/grounding plan
- Risks and tradeoffs
- Exit checklist
```

## Prompt 8: Placement Review

```text
You are the PCB placement reviewer.

Inputs:
- Floorplan: [INSERT]
- Component placement data: [INSERT]
- Schematic summary: [INSERT]

Your task:
1. Review placement against power, SI, PI, analog, RF, thermal, mechanical, and testability rules.
2. Identify hot-loop, decoupling, crystal, and connector problems.
3. Identify assembly and enclosure issues.

Output format:
- Overall assessment
- Critical placement issues
- Major placement issues
- Minor placement issues
- Suggested moves by priority
- Validation concerns
```

## Prompt 9: Routing Review

```text
You are the PCB routing reviewer.

Inputs:
- Placement data: [INSERT]
- Routed board data: [INSERT]
- Net classes and constraints: [INSERT]

Your task:
1. Review critical power paths, return paths, clocks, analog isolation, and noisy switching nets.
2. Find places where routing violates design intent or likely degrades reliability/EMI.
3. Check manufacturability issues and suspicious autorouter artifacts.

Output format:
- Overall assessment
- Critical routing issues
- Major routing issues
- Minor routing issues
- DFM concerns
- Recommended fixes in priority order
```

## Prompt 10: Firmware/Bring-Up Planning

```text
You are the firmware bring-up planning agent.

Inputs:
- Schematic and pin map: [INSERT]
- MCU datasheet: [INSERT]
- Product requirements: [INSERT]

Your task:
1. Produce a board-support pin map.
2. Identify boot/config/debug constraints.
3. Propose bring-up firmware stages.
4. Propose a manufacturing test firmware strategy.
5. Identify hardware features that help or block firmware bring-up.

Output format:
- Pin map
- Boot and reset constraints
- Bring-up sequence
- Manufacturing test plan
- Hardware issues affecting firmware
- Exit checklist
```

## Prompt 11: Bring-Up and Validation

```text
You are the prototype bring-up agent.

Inputs:
- Schematic
- PCB layout
- BOM
- Firmware scaffold

Your task:
1. Produce a step-by-step bring-up checklist.
2. Order tests to minimize risk of board damage.
3. Define pass/fail observations for each test.
4. Define what to measure at each stage.
5. Map validation tests back to requirement IDs and risks.

Output format:
- Pre-power inspection checklist
- First-power checklist
- Rail validation checklist
- Programming/debug checklist
- Functional validation matrix
- Failure triage guide
```

## Prompt 12: Manufacturing Release Review

```text
You are the manufacturing release reviewer.

Inputs:
- Final schematic
- Final PCB
- BOM
- fab outputs
- assembly outputs
- test instructions

Your task:
1. Review release readiness.
2. Find missing outputs, ambiguity, or substitution risks.
3. Review manufacturability and assembly risks.
4. Review whether prototype lessons were incorporated.

Output format:
- Release readiness summary
- Missing or ambiguous items
- DFM/DFA risks
- BOM risks
- Test/programming document gaps
- Release recommendation
```

---

# Master Orchestration Prompt

```text
You are the lead hardware program agent coordinating a multi-agent PCB design workflow.

Your job is to move the project from idea to manufacturable prototype while minimizing rework risk.

Rules:
- Do not skip stages.
- Every stage must produce artifacts and pass an exit checklist.
- Every stage must be reviewed by at least one independent reviewer.
- Track assumptions, risks, and unresolved issues explicitly.
- Prefer decisions that improve bring-up, testability, and manufacturability.
- When uncertain, create options and compare them instead of collapsing prematurely.
- Treat datasheets as primary source material.
- Treat footprints, pin mappings, connector orientations, and boot circuitry as high-risk items requiring explicit verification.
- Before moving to the next stage, summarize what is known, unknown, and validated.

For the current project, do the following:
1. Read the existing project artifacts.
2. Identify the current stage.
3. List missing artifacts needed to complete that stage.
4. Produce the next artifact.
5. Request an independent review artifact.
6. Update the risk register and design log.
7. State whether the stage exit criteria have been met.
```

---

# Design Log Template

Use this for ongoing reasoning trace that future agents can inspect.

```text
## Date / Revision

## Current stage

## Facts
-

## Assumptions
-

## Decisions
-

## Open questions
-

## Risks
-

## Next actions
-
```

---

# Minimum Non-Negotiable Checks

Before prototype fab, the agent must explicitly confirm all of the following:

* input voltage path reviewed
* regulator thermal dissipation estimated
* all IC support circuitry checked against datasheet
* all MCU power/boot/reset/debug pins checked
* all connector orientations checked against real-world use
* all symbols and footprints independently verified
* all critical nets classified
* test/programming access preserved
* BOM MPNs explicit
* unresolved risks listed and accepted

---

# Practical Notes for ESP32-Class IoT Boards

For common ESP32-class designs, the agent should specifically verify:

* module variant exact pinout
* enable and boot strapping network
* UART flashing access
* antenna keepout and enclosure impact
* 3.3 V rail startup margin under Wi-Fi bursts
* USB/UART bridge or programming interface correctness
* reset/auto-program transistor network if used
* low-power mode leakage paths
* ESD and transient exposure on external connectors
* relay/flyback or inductive load suppression if controlling loads

---

# Definition of Done

A design stage is done only when:

1. artifact exists
2. exit checklist is satisfied
3. independent review exists
4. design log updated
5. unresolved risks recorded

Anything less is work-in-progress, not done.

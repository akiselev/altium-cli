# Phase 6: Templates & Builders

**Agents: 3 parallel tracks (6A, 6B, 6C)**
**Blocked by: Phase 3 (record types)**

Templates are code functions that return `RecordOrigin` with Altium-correct defaults. Builders wrap template + setters into a fluent API. These can be built as soon as record types exist.

---

## Track 6A: Schematic Template Functions

**File: `crates/altium-format/src/v2/templates.rs`**

**Reference:**
- `templates/schlib.rs` — existing JSON template system (for default values)
- `_v2_reference/fields/pin.rs` — field/param key inventory
- `_v2_reference/fields/component.rs` — field inventory
- Real Altium files (Synthiam.SchLib) — extract actual default param order

### What to Build

Template functions that return `RecordOrigin` with Altium-correct default parameters in the correct order:

```rust
pub mod templates {
    use super::*;

    /// Default SchPin backing store matching Altium's output for a new pin.
    pub fn sch_pin_default() -> RecordOrigin {
        RecordOrigin::Param(ParamOrigin {
            params: {
                let mut p = ParameterCollection::new();
                // Add params in Altium's canonical order
                // Extract actual order from parsing real Altium files
                p.add_int("RECORD", 2);
                p.add_int("OWNERINDEX", 0);
                p.add_int("OWNERPARTID", 1);
                p.add_int("OWNERPARTDISPLAYMODE", 0);
                p.add_int("SYMBOLINNEREDGE", 0);
                p.add_int("SYMBOLOUTEREDGE", 0);
                p.add_int("SYMBOLINNER", 0);
                p.add_int("SYMBOLOUTER", 0);
                p.add("DESCRIPTION", "");
                p.add_int("FORMALTYPE", 0);
                p.add_int("ELECTRICAL", 4); // Passive
                p.add_int("PINCONGLOMERATE", 0);
                p.add_int("PINLENGTH", 30);
                p.add_int("PINLENGTH_FRAC", 0);
                p.add_int("LOCATION.X", 0);
                p.add_int("LOCATION.Y", 0);
                p.add("NAME", "");
                p.add("DESIGNATOR", "");
                p.add("UNIQUEID", "");
                p
            },
            raw_record_text: String::new(),
        })
    }

    /// Default SchComponent backing store.
    pub fn sch_component_default() -> RecordOrigin {
        RecordOrigin::Param(ParamOrigin {
            params: {
                let mut p = ParameterCollection::new();
                p.add_int("RECORD", 1);
                p.add("LIBREFERENCE", "");
                p.add("COMPONENTDESCRIPTION", "");
                p.add_int("PARTCOUNT", 1);
                p.add_int("DISPLAYMODECOUNT", 1);
                p.add_int("LOCATION.X", 0);
                p.add_int("LOCATION.Y", 0);
                p.add_int("ORIENTATION", 0);
                p.add("UNIQUEID", "");
                // ... all defaults from real Altium output
                p
            },
            raw_record_text: String::new(),
        })
    }

    /// Default SchArc, SchLine, SchRectangle, etc.
    pub fn sch_arc_default() -> RecordOrigin { ... }
    pub fn sch_line_default() -> RecordOrigin { ... }
    pub fn sch_rectangle_default() -> RecordOrigin { ... }
    pub fn sch_wire_default() -> RecordOrigin { ... }
    pub fn sch_bus_default() -> RecordOrigin { ... }
    pub fn sch_junction_default() -> RecordOrigin { ... }
    pub fn sch_net_label_default() -> RecordOrigin { ... }
    pub fn sch_power_default() -> RecordOrigin { ... }
    pub fn sch_port_default() -> RecordOrigin { ... }
    pub fn sch_parameter_default() -> RecordOrigin { ... }
    pub fn sch_designator_default() -> RecordOrigin { ... }
    pub fn sch_sheet_default() -> RecordOrigin { ... }
    pub fn sch_label_default() -> RecordOrigin { ... }
    pub fn sch_text_frame_default() -> RecordOrigin { ... }
    pub fn sch_no_erc_default() -> RecordOrigin { ... }
}
```

### How to Extract Defaults

1. Open `Synthiam.SchLib` with the current v2 code
2. For each record type, print the raw param string
3. Use that as the template — these are the actual params Altium writes

Alternatively, create a utility test that opens a SchLib and prints all params for each record type in insertion order.

### Tests

- `template_sch_pin_has_record_id()` — verify RECORD=2
- `template_sch_pin_roundtrip()` — create from template → read fields → correct defaults
- `template_sch_component_has_record_id()` — verify RECORD=1

### Acceptance Criteria

- [ ] Template function for every schematic record type
- [ ] Params are in Altium's canonical order
- [ ] Templates produce valid `RecordOrigin` that record types can read
- [ ] `cargo check` passes

---

## Track 6B: PCB Template Functions

**File: `crates/altium-format/src/v2/templates.rs` (same file, or split to `templates/pcb.rs`)**

**Reference:**
- `_v2_reference/pcb/pad.rs` — PcbPad binary structure
- Real Altium files (Synthiam.PcbLib) — extract default binary bytes

### What to Build

```rust
pub fn pcb_pad_default() -> RecordOrigin {
    RecordOrigin::Binary(BinaryOrigin {
        raw_block: vec![/* default binary bytes from real Altium file */],
        field_spans: vec![/* decoded spans from parse_pad() */],
    })
}

pub fn pcb_track_default() -> RecordOrigin {
    RecordOrigin::Binary(BinaryOrigin {
        raw_block: vec![
            // 13-byte common header (default layer, no flags, no refs)
            0x00, // layer: none
            0x00, 0x00, // flags
            0xFF, 0xFF, // net: none
            0xFF, 0xFF, // polygon_ref: none
            0xFF, 0xFF, // component_ref: none
            0xFF, 0xFF, // ref4: none
            0xFF, 0xFF, // ref5: none
            // Track-specific fields (all zeros)
            0x00, 0x00, 0x00, 0x00, // start_x
            0x00, 0x00, 0x00, 0x00, // start_y
            0x00, 0x00, 0x00, 0x00, // end_x
            0x00, 0x00, 0x00, 0x00, // end_y
            0x00, 0x00, 0x00, 0x00, // width
            0x00, 0x00,             // subpoly_index
        ],
        field_spans: vec![], // sequential layout — no span map needed
    })
}

pub fn pcb_arc_default() -> RecordOrigin { ... }
pub fn pcb_via_default() -> RecordOrigin { ... }
pub fn pcb_fill_default() -> RecordOrigin { ... }
pub fn pcb_text_default() -> RecordOrigin { ... }
pub fn pcb_footprint_default() -> RecordOrigin {
    // Param-based (Parameters stream metadata)
    RecordOrigin::Param(ParamOrigin {
        params: {
            let mut p = ParameterCollection::new();
            p.add("PATTERN", "");
            p.add("DESCRIPTION", "");
            p.add_int("HEIGHT", 0);
            p
        },
        raw_record_text: String::new(),
    })
}
```

### Acceptance Criteria

- [ ] Template function for each PCB record type
- [ ] Binary templates have correct default byte layouts
- [ ] `cargo check` passes

---

## Track 6C: Document-Level Builders

**File: `crates/altium-format/src/v2/builders.rs`**

**Reference: `docs/v2-plan.md` (Insert / Builder API section)**

### What to Build

1. **`ComponentBuilder`** — for constructing SchLib/SchDoc components:
   ```rust
   pub struct ComponentBuilder {
       component: RecordNode,
       children: Vec<RecordNode>,
   }

   impl ComponentBuilder {
       pub fn new(template: fn() -> RecordOrigin) -> Self {
           Self {
               component: RecordNode::new(template()),
               children: Vec::new(),
           }
       }

       /// Access component record for setting fields.
       pub fn record_mut(&mut self) -> &mut SchComponentRecord {
           // Cast origin to SchComponentRecord
           SchComponentRecord::from_origin_mut(&mut self.component.origin)
       }

       /// Convenience setters that delegate to the record
       pub fn set_lib_reference(&mut self, v: impl Into<LibReference>) {
           self.record_mut().set_lib_reference(v);
       }
       pub fn set_description(&mut self, v: impl Into<Description>) {
           self.record_mut().set_description(v);
       }

       /// Add a child record from a template
       pub fn add_pin(
           &mut self,
           template: fn() -> RecordOrigin,
           build: impl FnOnce(&mut SchPinRecord),
       ) {
           let mut node = RecordNode::new(template());
           let record = SchPinRecord::from_origin_mut(&mut node.origin);
           build(record);
           self.children.push(node);
       }

       pub fn add_arc(&mut self, template: fn() -> RecordOrigin, build: impl FnOnce(&mut SchArcRecord)) { ... }
       pub fn add_line(&mut self, template: fn() -> RecordOrigin, build: impl FnOnce(&mut SchLineRecord)) { ... }
       pub fn add_rectangle(&mut self, template: fn() -> RecordOrigin, build: impl FnOnce(&mut SchRectangleRecord)) { ... }

       /// Consume builder into ComponentGroup
       pub(crate) fn into_group(self) -> ComponentGroup {
           ComponentGroup {
               component: self.component,
               children: self.children,
               original_indices: Vec::new(), // new component has no original position
           }
       }
   }
   ```

2. **`FootprintBuilder`** — for constructing PcbLib footprints:
   ```rust
   pub struct FootprintBuilder {
       metadata: RecordNode,
       primitives: Vec<RecordNode>,
   }

   impl FootprintBuilder {
       pub fn new(template: fn() -> RecordOrigin) -> Self { ... }
       pub fn add_pad(&mut self, template: fn() -> RecordOrigin, build: impl FnOnce(&mut PcbPadRecord)) { ... }
       pub fn add_track(&mut self, template: fn() -> RecordOrigin, build: impl FnOnce(&mut PcbTrackRecord)) { ... }
       pub fn into_group(self) -> FootprintGroup { ... }
   }
   ```

3. **Document-level insertion** (on SchLib, SchDoc, PcbLib):
   ```rust
   impl SchLib {
       pub fn build_component(
           &mut self,
           template: fn() -> RecordOrigin,
           build: impl FnOnce(&mut ComponentBuilder),
       ) -> Result<()> {
           let mut builder = ComponentBuilder::new(template);
           build(&mut builder);
           self.groups.push(builder.into_group());
           Ok(())
       }
   }

   impl PcbLib {
       pub fn build_footprint(
           &mut self,
           template: fn() -> RecordOrigin,
           build: impl FnOnce(&mut FootprintBuilder),
       ) -> Result<()> { ... }
   }
   ```

### Tests

- `component_builder_basic()` — build component with 2 pins, verify structure
- `component_builder_closure()` — build via `SchLib::build_component()`
- `footprint_builder_basic()` — build footprint with pads and tracks

### Acceptance Criteria

- [ ] `ComponentBuilder` creates components with children from templates
- [ ] `FootprintBuilder` creates footprints with primitives from templates
- [ ] Document-level `build_component()` / `build_footprint()` methods work
- [ ] `cargo check` passes

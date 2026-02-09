# API Design Review: altium-format

## Executive Summary

The codebase has grown organically into **two parallel implementations** (v1 and v2) that solve the same problems with incompatible designs. The v1 system uses derive macros and trait-based polymorphism; the v2 system uses a `SchSerializer` trait ported from decompiled C#. Neither is complete alone. Meanwhile, OLE/CFB file access is well-isolated, but the mid-level architecture — how records are represented, how file types expose their contents, and how the ops/CLI layer consumes them — is where the real mess lives.

**Total codebase:** ~75K lines of Rust across `altium-format`.

| Module | Lines | Purpose |
|--------|-------|---------|
| v1 io/ | 6,369 | File I/O (SchLib, PcbLib, SchDoc, PcbDoc, IntLib, PrjPcb) |
| v2 io/ | 2,635 | Duplicate file I/O (SchLib, SchDoc, PcbLib, PcbDoc) |
| v1 records/ | 13,134 | Typed record structs (SchRecord, PcbRecord) |
| v2 fields/ | 1,560 | Duplicate record data structs |
| v2 pcb/ | 3,533 | Duplicate PCB record types |
| v2 serializer/ | 4,770 | SchSerializer trait + ASCII/Binary impls |
| api/ | 2,183 | CFB wrapper + generic dynamic access |
| ops/ | 16,154 | CLI-facing operations (the largest module) |

The v1+v2 duplication alone accounts for ~12K lines of redundant code.

---

# Unification Design: Merging v1 and v2

## Design Principles

1. **v1's ergonomics win** — derive macros, typed records, trait polymorphism
2. **v2's correctness wins** — coordinate system, field names, binary format, extended streams
3. **v2/format_v5 is the spec** — field orderings, names, and types are the ground truth from decompiled C#; they inform the derive attributes but don't live in the final code
4. **No breaking change is off limits** — but existing tests must pass (with adjusted expectations where coordinate values change)
5. **One authoritative type per concept** — no more parallel SchLib vs SchLibV2

---

## Decision 1: Domain-Specific Coordinate Types

### The Problem

Altium uses different coordinate scales for schematic and PCB:
- **Schematic**: 100,000 internal units per mil (confirmed from C# `SchDataSerializerBinary`)
- **PCB**: 10,000 internal units per mil (confirmed from Altium SDK `InternalUnits = 10000`)

v1 uses a single `Coord` type at 10K/mil for everything. This is correct for PCB but **wrong for schematic** — fractional coordinate precision is off by 10x:

```
Parameter: LOCATION.X=200, LOCATION.X_FRAC=50000

v1 (WRONG):  200 * 10,000  + 50,000 =  2,050,000 → 205.0 mils
v2 (RIGHT):  200 * 100,000 + 50,000 = 20,050,000 → 200.5 mils
```

We can't use 100K/mil for PCB because i32 overflows. At 100K/mil, max representable = 21,474 mils = 21.5 inches. PCB boards can be 100+ inches.

### Design

**Two coordinate types, explicitly named by domain:**

```rust
// types/coord.rs — replaces both Coord and V2Coord

/// Schematic coordinate: 100,000 internal units per mil.
///
/// Used by SchLib, SchDoc records. Supports sub-mil precision
/// via the DXP integer+frac split (PinFrac streams, LOCATION.X_FRAC params).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SchCoord(i32);

/// PCB coordinate: 10,000 internal units per mil.
///
/// Used by PcbLib, PcbDoc records. 1 internal unit = 2.54 nanometers.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PcbCoord(i32);
```

Both implement a common trait for conversion methods:

```rust
pub trait AltiumCoord: Copy + Sized {
    const UNITS_PER_MIL: f64;

    fn from_raw(value: i32) -> Self;
    fn to_raw(self) -> i32;

    fn from_mils(mils: f64) -> Self {
        Self::from_raw((mils * Self::UNITS_PER_MIL) as i32)
    }
    fn to_mils(self) -> f64 {
        self.to_raw() as f64 / Self::UNITS_PER_MIL
    }
    fn from_mms(mms: f64) -> Self {
        Self::from_mils(mms / 0.0254)
    }
    fn to_mms(self) -> f64 {
        self.to_mils() * 0.0254
    }
    // ... from_inches, to_inches, etc.
}

impl AltiumCoord for SchCoord {
    const UNITS_PER_MIL: f64 = 100_000.0;
    fn from_raw(v: i32) -> Self { SchCoord(v) }
    fn to_raw(self) -> i32 { self.0 }
}

impl AltiumCoord for PcbCoord {
    const UNITS_PER_MIL: f64 = 10_000.0;
    fn from_raw(v: i32) -> Self { PcbCoord(v) }
    fn to_raw(self) -> i32 { self.0 }
}
```

`SchCoord` also gets binary-split methods (for PinFrac streams):

```rust
impl SchCoord {
    /// Split for binary pin format: whole-mil i16 in Data stream,
    /// fractional remainder i32 in PinFrac stream.
    pub fn to_binary_parts(self) -> (i16, i32) {
        let whole = self.0 / 100_000;
        let frac = self.0 - 100_000 * whole;
        (whole as i16, frac)
    }

    pub fn from_binary_parts(whole: i16, frac: i32) -> Self {
        SchCoord(whole as i32 * 100_000 + frac)
    }
}
```

**Point and rect types become generic:**

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Point<C: AltiumCoord> {
    pub x: C,
    pub y: C,
}

pub type SchPoint = Point<SchCoord>;
pub type PcbPoint = Point<PcbCoord>;

// Same for Rect<C>
pub type SchRect = Rect<SchCoord>;
pub type PcbRect = Rect<PcbCoord>;
```

### Backward Compatibility

Keep `Coord` as a type alias during migration:

```rust
/// Deprecated: use PcbCoord or SchCoord explicitly.
pub type Coord = PcbCoord;
pub type CoordPoint = PcbPoint;
pub type CoordRect = PcbRect;
```

### DXP Frac Conversion Fix

```rust
// Fix: use SchCoord's unit scale
pub fn dxp_frac_to_sch_coord(integer: i32, frac: i32) -> SchCoord {
    SchCoord::from_raw(integer * 100_000 + frac)
}

pub fn sch_coord_to_dxp_frac(coord: SchCoord) -> (i32, i32) {
    (coord.0 / 100_000, coord.0 % 100_000)
}

// Keep old functions as deprecated aliases during migration
#[deprecated(note = "use dxp_frac_to_sch_coord")]
pub fn dxp_frac_to_coord(integer: i32, frac: i32) -> i32 {
    integer * 100_000 + frac  // Fixed scale
}
```

### Test Impact

- `test_dxp_frac_conversion` in `records/sch/common.rs` — values will change:
  - Before: `dxp_frac_to_coord(100, 5000) == 1005000`
  - After: `dxp_frac_to_coord(100, 5000) == 10005000`
  - Update assertion
- `test_coord_conversions` in `types/coord.rs` — unchanged (tests PcbCoord behavior)
- Roundtrip integration tests — raw param values are preserved, so parse→write→parse still matches. The internal representation changes but serialized output is identical.
- Tests that compare Coord values in mils — update expected values where schematic coords are involved

---

## Decision 2: Merge Record Type Field Sets

### Strategy

Use v2/fields/ and v2/serializer/format_v5/ as the **specification** for correct field sets. Port every field into v1-style derive-macro structs. Delete v2 types once ported.

### SchPin: 24 fields → 50+ fields

Current v1 SchPin is missing ~26 fields that v2's PinData has. Expand it:

```rust
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 2, format = "params")]
pub struct SchPin {
    // --- Base fields (NOT via SchGraphicalBase — pins are special) ---
    #[altium(param = "OWNERINDEX")]
    pub owner_index: i32,
    #[altium(param = "OWNERPARTID")]
    pub owner_part_id: i16,
    #[altium(param = "OWNERPARTDISPLAYMODE")]
    pub owner_part_display_mode: u8,

    // --- IEEE symbols ---
    #[altium(param = "SYMBOL_INNEREDGE", default)]
    pub symbol_inner_edge: PinSymbol,
    #[altium(param = "SYMBOL_OUTEREDGE", default)]
    pub symbol_outer_edge: PinSymbol,
    #[altium(param = "SYMBOL_INNER", default)]
    pub symbol_inside: PinSymbol,
    #[altium(param = "SYMBOL_OUTER", default)]
    pub symbol_outside: PinSymbol,

    // --- Core pin properties ---
    #[altium(param = "DESCRIPTION", default)]
    pub description: String,
    #[altium(param = "FORMALTYPE", default)]
    pub formal_type: i32,
    #[altium(param = "ELECTRICAL", default)]
    pub electrical: PinElectricalType,

    // --- PinConglomerate (packed byte) ---
    #[altium(param = "PINCONGLOMERATE", default)]
    pub pin_conglomerate: PinConglomerateFlags,

    // --- Coordinates (SchCoord, with frac support) ---
    #[altium(param = "PINLENGTH", frac)]
    pub pin_length: SchCoord,
    #[altium(param = "LOCATION.X", frac)]
    pub location_x: SchCoord,
    #[altium(param = "LOCATION.Y", frac)]
    pub location_y: SchCoord,
    #[altium(param = "COLOR", default)]
    pub color: u32,

    // --- Strings ---
    #[altium(param = "NAME", default)]
    pub name: String,
    #[altium(param = "DESIGNATOR", default)]
    pub designator: String,
    #[altium(param = "SWAPIDPIN", default)]       // v1 bug: was SWAPIDGROUP
    pub swap_id_pin: String,
    #[altium(param = "SWAPIDPART", default)]
    pub swap_id_part: String,
    #[altium(param = "DEFAULTVALUE", default)]
    pub default_value: String,
    #[altium(param = "SWAPIDPAIR", default)]       // NEW from v2
    pub swap_id_pair: String,

    // --- Name customization (NEW from v2) ---
    #[altium(param = "PIN_NAME_POSITIONMODE", default)]
    pub name_position_mode: i32,
    #[altium(param = "PIN_NAME_CUSTOMROTATIONANCHOR", default)]
    pub name_custom_rotation_anchor: i32,
    #[altium(param = "PIN_NAME_CUSTOMROTATIONRELATIVE", default)]
    pub name_custom_rotation_relative: i32,
    #[altium(param = "PIN_NAME_FONTMODE", default)]
    pub name_font_mode: i32,
    #[altium(param = "PIN_NAME_CUSTOMPOSITIONMARGIN", default)]
    pub name_custom_position_margin: SchCoord,
    #[altium(param = "PIN_NAME_CUSTOMFONTID", default)]
    pub name_custom_font_id: i32,
    #[altium(param = "PIN_NAME_CUSTOMCOLOR", default)]
    pub name_custom_color: u32,

    // --- Designator customization (NEW from v2) ---
    #[altium(param = "PIN_DESIGNATOR_POSITIONMODE", default)]
    pub designator_position_mode: i32,
    #[altium(param = "PIN_DESIGNATOR_CUSTOMROTATIONANCHOR", default)]
    pub designator_custom_rotation_anchor: i32,
    #[altium(param = "PIN_DESIGNATOR_CUSTOMROTATIONRELATIVE", default)]
    pub designator_custom_rotation_relative: i32,
    #[altium(param = "PIN_DESIGNATOR_FONTMODE", default)]
    pub designator_font_mode: i32,
    #[altium(param = "PIN_DESIGNATOR_CUSTOMPOSITIONMARGIN", default)]
    pub designator_custom_position_margin: SchCoord,
    #[altium(param = "PIN_DESIGNATOR_CUSTOMFONTID", default)]
    pub designator_custom_font_id: i32,
    #[altium(param = "PIN_DESIGNATOR_CUSTOMCOLOR", default)]
    pub designator_custom_color: u32,

    // --- Extended (NEW from v2) ---
    #[altium(param = "SYMBOLLINEWIDTH", default)]
    pub symbol_line_width: LineWidth,
    #[altium(param = "PINPACKAGELENGTH", default)]
    pub pin_package_length: SchCoord,
    #[altium(param = "PINPROPAGATIONDELAY", default)]
    pub pin_propagation_delay: f64,
    #[altium(param = "UNIQUEID", default)]
    pub unique_id: String,

    // --- Alternate pin functions (NEW from v2) ---
    #[altium(param = "HIDEPINNAMEASFUNCTION", default)]
    pub hide_pin_name_as_function: bool,
    #[altium(param = "PINSYMBOLICNAME", default)]
    pub pin_symbolic_name: String,
    #[altium(param = "SHOWSYMBOLICNAMEASFUNCTION", default)]
    pub show_symbolic_name_as_function: bool,

    // --- Round-trip preservation ---
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}
```

**Key changes from v1:**
- `swap_id_group` → `swap_id_pin` (v1 bug: wrong field name)
- `formal_type` serialized as actual value (v1 always wrote 0)
- Location fields use `SchCoord` instead of raw `i32`
- ~26 new fields for name/designator customization, extended data
- Pin does NOT use `SchGraphicalBase` — owner fields are serialized directly (per C# `ExportPin`)

### SchComponent: 23 fields → 47+ fields

Similar expansion. Key additions from v2:
- `database_table_name`, `vault_guid`, `item_guid`, `revision_guid`
- `symbol_vault_guid`, `symbol_item_guid`, `symbol_revision_guid`
- `generic_component_template_guid`
- `component_kind: ComponentKind` (with version-aware serialization)
- `custom_display_mode_names: Vec<String>`
- `pins_moveable`, `not_use_library_name`, `not_use_db_table_name`
- `all_pin_count`, `key_component_unique_id`

All new fields get `#[altium(default)]` so they deserialize from existing files (missing params → default value). UnknownFields catches anything we still miss.

### Derive Macro Extensions

The derive macro needs minor extensions to support the merged types:

1. **`SchCoord` support in `frac` attribute** — the frac attribute currently uses `dxp_frac_to_coord(int, frac)` which returns `i32`. Change to return `SchCoord`:
   ```rust
   // In altium-format-derive record.rs generate_from_params():
   // When field type is SchCoord and has frac attribute:
   quote! {
       #field_name: crate::types::SchCoord::from_dxp_frac(
           params.get(#param_name).map(|v| v.as_int_or(0)).unwrap_or(0),
           params.get(#frac_name).map(|v| v.as_int_or(0)).unwrap_or(0),
       ),
   }
   ```

2. **`#[altium(ascii_only)]` attribute** — some v2 fields are only serialized in ASCII mode, skipped in binary. Add attribute to mark these:
   ```rust
   #[altium(param = "PIN_NAME_POSITIONMODE", default, ascii_only)]
   pub name_position_mode: i32,
   ```
   Generated binary serialization skips fields with `ascii_only`.

---

## Decision 3: Pin Extended Data Streams

### The Problem

In SchLib, each component's pin data is split across multiple CFB streams:

```
/{ComponentSection}/
    Data              ← main record data (ASCII params or binary)
    PinFrac           ← fractional coordinate remainders (i32 per pin per coord)
    PinDesc           ← long descriptions
    PinWideText       ← UTF-16 text
    PinMiscData       ← swap IDs
    PinTextData       ← custom text position/color/rotation
    PinSymbolLineWidth
    PinPackageLength
    PinPropagationDelay
    PinFunctionData   ← alternate pin function names
```

v1 ignores all of these. v2 handles them via the SchSerializer's `start_stream`/`end_stream` mechanism.

### Design

Extended data streams are a **file I/O concern**, not a record type concern. The typed `SchPin` struct has fields for all this data (pin_length as SchCoord with full precision, description, etc.). The file I/O layer is responsible for:

1. Reading main Data stream → populate core fields
2. Reading extended streams → populate extended fields
3. Writing main Data stream ← serialize core fields
4. Writing extended streams ← serialize extended fields

This stays in `io/schlib.rs`, not in the derive macros:

```rust
impl SchLib {
    fn read_component<F: Read + Seek>(
        &mut self,
        cf: &mut CompoundFile<F>,
        section_key: &str,
    ) -> Result<SchLibComponent> {
        // 1. Read main Data stream → records
        let records = self.read_data_stream(cf, section_key)?;

        // 2. Collect pins from records
        let pins: Vec<&mut SchPin> = records.iter_mut()
            .filter_map(|r| match r {
                SchRecord::Pin(p) => Some(p),
                _ => None,
            })
            .collect();

        // 3. Read extended streams and merge into pin fields
        self.read_pin_frac(cf, section_key, &mut pins)?;
        self.read_pin_desc(cf, section_key, &mut pins)?;
        self.read_pin_wide_text(cf, section_key, &mut pins)?;
        self.read_pin_misc_data(cf, section_key, &mut pins)?;
        self.read_pin_text_data(cf, section_key, &mut pins)?;
        self.read_pin_symbol_line_width(cf, section_key, &mut pins)?;
        self.read_pin_package_length(cf, section_key, &mut pins)?;
        self.read_pin_propagation_delay(cf, section_key, &mut pins)?;
        self.read_pin_function_data(cf, section_key, &mut pins)?;

        // ...
    }

    fn read_pin_frac<F: Read + Seek>(
        &self,
        cf: &mut CompoundFile<F>,
        section_key: &str,
        pins: &mut [&mut SchPin],
    ) -> Result<()> {
        let path = format!("/{}/PinFrac", section_key);
        if let Ok(data) = read_stream(cf, &path) {
            let mut cursor = Cursor::new(&data);
            for pin in pins.iter_mut() {
                // Each pin has 3 frac values: pin_length, location_x, location_y
                let length_frac = cursor.read_i32::<LittleEndian>()?;
                let loc_x_frac = cursor.read_i32::<LittleEndian>()?;
                let loc_y_frac = cursor.read_i32::<LittleEndian>()?;

                // Merge fractional parts into the SchCoord values
                pin.pin_length = SchCoord::from_binary_parts(
                    pin.pin_length.to_binary_parts().0,
                    length_frac,
                );
                pin.location_x = SchCoord::from_binary_parts(
                    pin.location_x.to_binary_parts().0,
                    loc_x_frac,
                );
                pin.location_y = SchCoord::from_binary_parts(
                    pin.location_y.to_binary_parts().0,
                    loc_y_frac,
                );
            }
        }
        Ok(())
    }

    // Similar for other extended streams...
}
```

**On write**, the reverse: extract extended data from pin fields into separate stream buffers.

### Why Not in the Derive Macro?

Extended data streams span multiple pins within a component — they're positional (pin N's frac data is at offset N*12 in the PinFrac stream). This is fundamentally a container-level concern, not a per-record concern. The derive macro operates on individual records.

---

## Decision 4: Binary Pin Format Fix

### The Bugs

v1's `write_binary_pin` has three bugs vs the C# reference:

1. **Phantom byte after record type** (line 406 in io/schlib.rs):
   ```rust
   data.write_i32::<LittleEndian>(2)?;  // record type
   data.write_u8(0)?;                   // ← PHANTOM BYTE — doesn't exist in C#
   ```

2. **Phantom byte before electrical** (line 420):
   ```rust
   write_pascal_short_string(&mut data, &pin.description)?;
   data.write_u8(0)?;                   // ← PHANTOM BYTE
   data.write_u8(pin.electrical...)?;
   ```

3. **FormalType always 0** — v1 writes 0 instead of the actual value.

### Fix

Replace the hand-written `write_binary_pin` / `read_binary_pin` with derive-macro generated code, using v2's format as the spec:

```rust
#[derive(AltiumRecord)]
#[altium(record_id = 2, format = "binary")]
pub struct SchPinBinary {
    // No phantom bytes — derive macro writes fields in order
    #[altium(param = "OWNERPARTID")]
    pub owner_part_id: i16,
    pub owner_part_display_mode: u8,
    pub symbol_inner_edge: u8,
    pub symbol_outer_edge: u8,
    pub symbol_inside: u8,
    pub symbol_outside: u8,
    #[altium(pascal_string)]
    pub description: String,
    pub formal_type: u8,        // Actual value, not hardcoded 0
    pub electrical: u8,
    pub pin_conglomerate: u8,
    pub pin_length: i16,        // Whole mils (frac in PinFrac stream)
    pub location_x: i16,
    pub location_y: i16,
    pub color: i32,
    #[altium(pascal_string)]
    pub name: String,
    #[altium(pascal_string)]
    pub designator: String,
    #[altium(pascal_string)]
    pub swap_id_pin: String,    // Fixed name
    #[altium(pascal_string)]
    pub swap_id_part_sequence: String,
    #[altium(pascal_string)]
    pub default_value: String,
}
```

The binary struct is an intermediate representation — read it, then convert to the full `SchPin` by merging with extended stream data:

```rust
impl SchPin {
    fn from_binary(bin: &SchPinBinary) -> Self {
        SchPin {
            owner_part_id: bin.owner_part_id,
            location_x: SchCoord::from_binary_parts(bin.location_x, 0), // frac merged later
            location_y: SchCoord::from_binary_parts(bin.location_y, 0),
            pin_length: SchCoord::from_binary_parts(bin.pin_length, 0),
            electrical: PinElectricalType::from_int(bin.electrical as i32),
            formal_type: bin.formal_type as i32,  // Actual value preserved
            swap_id_pin: bin.swap_id_pin.clone(),  // Correct field name
            // ...
        }
    }
}
```

---

## Decision 5: Section Key Collision Avoidance

### The Bug

v1 truncates at 31 chars with no collision detection:
```rust
fn get_section_key_for(name: &str) -> String {
    let mut key = name.replace('/', "_");
    if key.len() > 31 { key.truncate(31); }
    key
}
```

Two components whose names share the first 31 characters get the same section key → data corruption.

### Fix

Replace with v2's `SectionKeyList` (from `v2/io/section_keys.rs`):
- Truncate to 30 characters (matching C# behavior)
- Append numeric suffix on collision (`Name1`, `Name2`, ...)
- Check for space at position 30 (CFB edge case from C#)

This is a direct copy of v2's `SectionKeyList` into the main io module — it's 175 lines, well-tested, and correct:

```rust
// Move from v2/io/section_keys.rs → io/section_keys.rs (or types/)
pub struct SectionKeyList { ... }

// Use in io/schlib.rs and io/pcblib.rs
impl SchLib {
    fn build_section_keys(&self) -> SectionKeyList {
        let mut keys = SectionKeyList::new();
        for comp in &self.components {
            let safe = comp.component.lib_reference.replace('/', "_");
            keys.add_key(&safe, 30);
            for alias in &comp.component.alias_list {
                keys.add_key(&alias.replace('/', "_"), 30);
            }
        }
        keys
    }
}
```

---

## Decision 6: PCB Record Improvements

### Adaptive Trailing Fields

v2's PCB records handle variable-length binary data that v1 can't:

```rust
// v2 approach: Option<T> for fields that may or may not be present
pub struct PcbVia {
    // ... core fields always present ...
    pub thermal_relief_airgap: Option<PcbCoord>,       // only if data len > 35
    pub diameter_by_layer: Option<[PcbCoord; 32]>,     // only if data len >= 203
    pub pos_tolerance: Option<PcbCoord>,               // only if data len >= 310
    pub neg_tolerance: Option<PcbCoord>,
}
```

v1's derive macro reads fields in strict order and fails if data is short.

### Fix: Extend Derive Macro with `#[altium(optional_binary)]`

```rust
#[derive(AltiumRecord)]
#[altium(format = "binary")]
pub struct PcbVia {
    #[altium(flatten)]
    pub common: PcbPrimitiveCommon,
    #[altium(coord_point)]
    pub location: PcbPoint,
    #[altium(coord)]
    pub diameter: PcbCoord,
    #[altium(coord)]
    pub hole_size: PcbCoord,
    pub from_layer: u8,
    pub to_layer: u8,

    // Fields below are optional — only present if binary data is long enough
    #[altium(optional_binary, min_offset = 35)]
    pub thermal_relief_airgap: Option<PcbCoord>,
    #[altium(optional_binary, min_offset = 36)]
    pub thermal_relief_conductor_count: Option<u8>,
    #[altium(optional_binary, min_offset = 37)]
    pub thermal_relief_conductor_width: Option<PcbCoord>,
    // ...

    #[altium(unknown_binary)]
    pub unknown: Vec<u8>,
}
```

The derive macro generates `FromBinary` that:
1. Reads required fields normally
2. For `optional_binary` fields: checks remaining data length, reads if sufficient, sets `None` if not
3. Captures remaining bytes in `unknown_binary`

### New PCB Fields

Port these from v2 into v1 structs:

| Record | New Fields from v2 |
|--------|--------------------|
| PcbTrack | `subpoly_index: u16`, adaptive trailing (user_routed, union_index, layer_enum, keepout) |
| PcbArc | `subpoly_index: u16`, adaptive trailing |
| PcbPad | `pad_mode`, `thermal_connect_mode`, `pad_layer_bitmask`, jumper GUIDs, separated core/stack |
| PcbVia | `via_mode`, `diameter_by_layer`, `soldermask_expansion_back`, tolerances |
| PcbText | (minimal changes) |

### PcbPrimitiveCommon: Add Object References

v2's header includes references v1 ignores:

```rust
pub struct PcbPrimitiveCommon {
    pub layer: Layer,
    pub flags: PcbFlags,
    pub net: u16,          // NEW: net reference (0xFFFF = none)
    pub polygon: u16,      // NEW: polygon reference
    pub component: u16,    // NEW: component reference
    pub ref4: u16,         // NEW: unknown reference
    pub ref5: u16,         // NEW: unknown reference
}
```

---

## Decision 7: The SchSerializer Question

### Keep or Delete?

The 147-method `SchSerializer` trait is a faithful C# port. It has value as a **reference implementation** but is not idiomatic Rust.

**Decision: Keep as internal validation tool, delete as public API.**

1. Keep `v2/serializer/` behind `#[cfg(test)]` or a `v2-compat` feature flag
2. Use it to write **comparison tests**: serialize a record with both the derive-macro path and the SchSerializer path, assert byte-identical output
3. Once all comparison tests pass, the v2 code becomes dead — delete it

```rust
#[cfg(test)]
mod compat_tests {
    use super::*;
    use crate::v2::serializer::format_v5;

    #[test]
    fn pin_ascii_matches_v2() {
        let pin = SchPin { /* test values */ };

        // v1 path: derive-macro generated
        let v1_output = pin.to_params().to_param_string();

        // v2 path: SchSerializer
        let mut ser = AsciiSerializer::new_writer();
        format_v5::export_pin(&mut ser, &pin.to_pin_data())?;
        let v2_output = ser.to_param_string();

        assert_eq!(v1_output, v2_output);
    }
}
```

This gives us confidence that the derive-macro path produces identical output to the proven C# port, then we can delete v2 entirely.

---

## Decision 8: Unified File Types

### Current Duplication

| Concept | v1 | v2 |
|---------|----|----|
| Schematic library | `io::SchLib` | `v2::io::SchLibV2` |
| Schematic document | `io::SchDoc` | `v2::io::SchDocV2` |
| PCB library | `io::PcbLib` | `v2::pcb::io::PcbLib` (shadowed) |
| PCB document | `io::PcbDoc` | `v2::pcb::io::PcbDoc` (shadowed) |

### Design

**One type per file format, in `io/`:**

- `io::SchLib` — the merged type (v1 struct + v2 fixes)
- `io::SchDoc` — merged
- `io::PcbLib` — merged
- `io::PcbDoc` — merged

v2 types (`SchLibV2`, `SchDocV2`) become temporary aliases during migration:

```rust
// During migration only:
pub type SchLibV2 = SchLib;  // v2 integration tests still compile
```

Then delete the aliases once all v2 tests are migrated.

---

## Migration Plan

### Phase 1: Coordinate Types (smallest blast radius)

**Files changed:** ~8

1. Add `SchCoord`, `PcbCoord`, `AltiumCoord` trait to `types/coord.rs`
2. Add `SchPoint`, `PcbPoint`, `SchRect`, `PcbRect`
3. Keep `Coord = PcbCoord` alias
4. Fix `dxp_frac_to_coord` to use 100K scale
5. Update `test_dxp_frac_conversion` assertion

**Tests affected:** 1 test updated (dxp_frac assertion). All other tests pass — PCB code still uses `Coord` (= PcbCoord, unchanged), schematic code uses raw `i32` which round-trips unchanged through params.

### Phase 2: Schematic Record Fields (medium blast radius)

**Files changed:** ~15 (records/sch/*.rs)

1. Expand `SchPin` with v2 fields (all `#[altium(default)]` — backward compatible)
2. Fix field name: `swap_id_group` → `swap_id_pin`
3. Expand `SchComponent` with v2 fields
4. Change schematic coordinate fields from `i32` to `SchCoord`
5. Port SectionKeyList from v2

**Tests affected:**
- `test_pin_roundtrip` — update field name reference
- `test_component_roundtrip` — new fields default, roundtrip unchanged
- Integration roundtrip tests — field values preserved via UnknownFields

### Phase 3: Binary Pin Format Fix

**Files changed:** ~2 (io/schlib.rs)

1. Replace hand-written `read_binary_pin` / `write_binary_pin`
2. Add `SchPinBinary` intermediate struct with derive macro
3. Add pin extended data stream reading (PinFrac, PinDesc, etc.)
4. Add pin extended data stream writing

**Tests affected:**
- Binary roundtrip tests need Synthiam.SchLib with real pin data
- Add new test: `pin_binary_format_matches_v2` (comparison test)

### Phase 4: PCB Record Improvements

**Files changed:** ~12 (records/pcb/*.rs)

1. Add `optional_binary` support to derive macro
2. Expand PcbVia, PcbTrack, PcbArc, PcbPad with v2 fields
3. Add subpoly_index, trailing fields
4. Expand PcbPrimitiveCommon with object references

**Tests affected:**
- PCB roundtrip integration tests — new Optional fields parse as None from existing data, raw bytes preserved via unknown_binary
- Add new test: `pcb_track_adaptive_trailing`

### Phase 5: v2 Comparison Tests

**Files changed:** ~5 (new test files)

1. Write comparison tests for every record type: v1 derive path vs v2 SchSerializer path
2. Verify byte-identical output
3. Fix any discrepancies found

### Phase 6: Delete v2

**Files deleted:** ~55 files, ~12K lines

1. Remove `v2/` module entirely
2. Remove `SchLibV2`, `SchDocV2`, v2 PCB types
3. Move v2 integration tests to use merged types
4. Remove `V2Coord` (superseded by `SchCoord`)
5. Remove `SchSerializer` trait and implementations
6. Remove `format_v5` functions

**Tests migrated:**
- `v2_schlib_roundtrip.rs` → uses `SchLib`
- `v2_schlib_cfb_roundtrip.rs` → uses `SchLib`
- `v2_pcblib_roundtrip.rs` → uses `PcbLib`
- `v2_pcblib_cfb_roundtrip.rs` → uses `PcbLib`
- `v2_schdoc_cfb_roundtrip.rs` → uses `SchDoc`
- `v2_pcbdoc_cfb_roundtrip.rs` → uses `PcbDoc`

---

## Test Strategy

### Invariant: Serialized Output Must Match

The critical invariant is: **reading a file and writing it back produces identical bytes.** This is already tested by the integration roundtrip tests. Since we're changing internal representation (not serialization format), these tests should pass at every phase.

Specifically:
- Parameter strings round-trip unchanged (UnknownFields preserves unknown params)
- Binary blocks round-trip unchanged (unknown_binary preserves unknown bytes)
- New fields get `#[altium(default)]` so missing params → default → not written back (no new params injected)

### New Tests to Add

| Test | What it validates |
|------|-------------------|
| `sch_coord_from_mils` | SchCoord at 100K/mil: `from_mils(1.0).to_raw() == 100_000` |
| `sch_coord_binary_parts` | Split/merge: `from_raw(350_123).to_binary_parts() == (3, 50_123)` |
| `pcb_coord_unchanged` | PcbCoord at 10K/mil: `from_mils(1.0).to_raw() == 10_000` |
| `pin_swap_id_field_name` | `SchPin { swap_id_pin: "X" }.to_params()` contains "SWAPIDPIN=X" |
| `pin_formal_type_preserved` | `SchPin { formal_type: 5 }.to_params()` contains "FORMALTYPE=5" |
| `pin_binary_no_phantom` | Binary pin serialization matches v2 byte-for-byte |
| `pin_frac_roundtrip` | SchLib write → read preserves sub-mil pin coordinates |
| `section_key_collision` | Two 35-char names differing at char 32 get different section keys |
| `pcb_via_short_record` | PcbVia with 31 bytes → optional fields are None |
| `pcb_via_long_record` | PcbVia with 310 bytes → all optional fields populated |

### Existing Tests: Expected Changes

| Test | Change Required |
|------|----------------|
| `test_dxp_frac_conversion` | Update: `dxp_frac_to_coord(100, 5000) == 10_005_000` (was 1_005_000) |
| `test_pin_roundtrip` | Update field name if test references `swap_id_group` |
| `test_designator_roundtrip_record_id` | No change (tests record_id, not coords) |
| `test_designator_survives_schdoc_roundtrip` | No change (tests record survival, not values) |
| v2 roundtrip tests | Phase 6: migrate to use unified types |
| All other 270+ tests | No change expected |

---

## Summary

| Decision | What | Lines Changed | Lines Deleted |
|----------|------|---------------|---------------|
| 1. Coord types | SchCoord (100K) + PcbCoord (10K) | ~200 new | 0 |
| 2. Record fields | Expand SchPin (24→50), SchComponent (23→47) | ~400 changed | ~100 |
| 3. Extended streams | PinFrac/PinDesc/... reading in io/schlib.rs | ~300 new | 0 |
| 4. Binary pin fix | Remove phantom bytes, fix FormalType | ~100 changed | ~80 |
| 5. Section keys | Port SectionKeyList from v2 | ~10 changed | ~30 |
| 6. PCB records | optional_binary, trailing fields, refs | ~300 changed | ~100 |
| 7. Comparison tests | v1 vs v2 output validation | ~200 new | 0 |
| 8. Delete v2 | Remove entire v2/ module | 0 | **~12,500** |
| **Total** | | ~1,500 new/changed | **~12,800 deleted** |

Net result: **~11,300 fewer lines of code**, one authoritative implementation, v2's correctness, v1's ergonomics.

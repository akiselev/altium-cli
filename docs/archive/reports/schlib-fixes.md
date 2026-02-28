# SchLib Validation Findings

Cross-reference of Rust SchLib implementation against C# source (`AD26-dotnet/`) and documentation (`docs/schlib/`, `docs/dxp/`).

**Date**: 2026-02-22
**Scope**: Record structs, binary pin parsing, dispatch coverage, constants, base primitive fields

---

## MISMATCH — Code contradicts C# source

### M1: SchPie has spurious `secondary_radius` field

**Code**: `crates/altium-format/src/sch_records.rs:674-675`
**C# ref**: `FileFormatV5.cs:277-324` (ExportPie/ImportPie), `SchDataPie.cs`, `SchDataArc.cs`

Our `SchPie` struct has:
```rust
#[param(coord, key = SECONDARY_RADIUS, frac_key = SECONDARY_RADIUS_FRAC)]
pub secondary_radius: Coord,
```

The C# `ExportPie`/`ImportPie` does NOT read/write `SecondaryRadius`. `SchDataPie` extends `SchDataArc` which only has `radius`. `SecondaryRadius` only exists on `Ellipse` and `EllipticalArc`.

**Fix**: Remove `secondary_radius` from `SchPie`.

---

### M2: `PartIDLocked` default should be dynamic (value of `DesignatorLocked`)

**Code**: `crates/altium-format/src/sch_records.rs:416`
**C# ref**: `FileFormatV5.cs:3011-3016`

Our code:
```rust
let part_id_locked: bool = params.remove_with_default(PART_ID_LOCKED, false)?;
```

C#:
```csharp
bool argN22 = false;
argSerializer.Import_Boolean(ref argN22, "DesignatorLocked");
schDataComponent.SetDesignatorLocked(argN22);
bool argN23 = false;
argSerializer.Import_Boolean_WithDefault(ref argN23, "PartIDLocked", argN22);
//                                                       default ^^^^^ = DesignatorLocked value
```

When `PartIDLocked` is absent from the file, C# defaults to whatever `DesignatorLocked` was. Our code always defaults to `false`. This matters when `DesignatorLocked=T` but `PartIDLocked` is absent.

The comment in `constants/component.rs:167` already documents this but the parsing code doesn't implement it.

**Fix**: Read `DesignatorLocked` first, then use its value as the default for `PartIDLocked`.

---

### M3: `GraphicallyLocked` always reset to `false` on import in C#

**Code**: `crates/altium-format/src/sch_records.rs:97` (text records), `:151` (binary pins)
**C# ref**: `FileFormatV5.cs:5099` (graphical objects), `FileFormatV5.cs:602` (pins)

C# `ImportGraphicalObject`:
```csharp
schDataGraphicalObject.SetGraphicallyLocked(argValue: false);  // line 5099
```

C# `ImportPin`:
```csharp
schDataPin.SetGraphicallyLocked(argValue: false);  // line 602
```

Both ALWAYS set `GraphicallyLocked = false` on import, ignoring the file value. The value IS written on export (asymmetric round-trip in Altium itself). Our code reads and preserves the actual file value.

**Decision needed** — see A1 below.

---

## GAP — Code is missing something the C# source has

### G1: Missing base record fields: `SelectionMemory`, `UnionIndex`

**Code**: `crates/altium-format/src/sch_records.rs:85-99` (`SchPrimitiveBase`)
**C# ref**: `FileFormatV5.cs:5082-5098` (`ImportGraphicalObject`)

C# reads these for ALL graphical objects:
```csharp
byte argN3 = 0;
argSerializer.Import_Byte(ref argN3, "SelectionMemory");     // line 5094
int argN4 = 0;
argSerializer.Import_LongInt(ref argN4, "UnionIndex");       // line 5097
```

Our `SchPrimitiveBase` does not parse either. Constants already exist (`SELECTION_MEMORY`, `UNION_INDEX` in `record_structure.rs:81,87`).

**Fix**: Add `selection_memory: u8` (default 0) and `union_index: i32` (default 0) to `SchPrimitiveBase`.

---

### G2: Missing base record fields: `IgnoreOnLoad`, `OwnerIndexAdditionalList` (text form)

**Code**: `crates/altium-format/src/sch_records.rs:85-99` (`SchPrimitiveBase`)
**C# ref**: `FileFormatV5.cs:5038-5067` (`ImportDataObject`)

C# reads these for ALL data objects:
```csharp
bool argN3 = true;
argSerializer.Import_Boolean(ref argN3, "OwnerIndexAdditionalList");  // line 5047
bool argN5 = false;
argSerializer.Import_Boolean(ref argN5, "IgnoreOnLoad");              // line 5053
```

We handle `OwnerIndexAdditionalList` for binary pins (in the conglomerate byte) but not for text parameter records. Constants already exist (`OWNER_INDEX_ADDITIONAL_LIST`, `IGNORE_ON_LOAD` in `record_structure.rs:36,66`).

**Fix**: Add `owner_index_additional_list: bool` (default `true`) and `ignore_on_load: bool` (default `false`) to `SchPrimitiveBase`.

---

### G3: Missing base record field: `IsSchematicBlockObject`

**Code**: `crates/altium-format/src/sch_records.rs:85-99`
**C# ref**: `FileFormatV5.cs:5061-5063`

```csharp
bool argN7 = false;
argSerializer.Import_Boolean(ref argN7, "IsSchematicBlockObject");
```

Not parsed by our code.

**Fix**: Add `is_schematic_block_object: bool` (default `false`) to `SchPrimitiveBase`.

---

### G4: `IsSolid` default mismatch on multiple records

**C# ref**: `FileFormatV5.cs` — initial values before `Import_Boolean`

| Record | Our file:line | C# default | Our default |
|--------|--------------|------------|-------------|
| SchRectangle | `sch_records.rs:546` | `true` (line 1668) | `false` |
| SchRoundRectangle | `sch_records.rs:575` | `true` (line 1731) | `false` |
| SchPie | `sch_records.rs:686` | `true` (line 323) | `false` |

Only matters when `IsSolid` is absent from the file. When present, both parse correctly.

**Fix**: Change `default = false` to `default = true` for `is_solid` on these three records.

---

### G5: `CornerXRadius`/`CornerYRadius` default mismatch in SchRoundRectangle

**Code**: `crates/altium-format/src/sch_records.rs:570-571`
**C# ref**: `FileFormatV5.cs:1716-1720`

```csharp
int argN = 20;
argSerializer.Import_Coord(ref argN, "CornerXRadius");
int argN2 = 20;
argSerializer.Import_Coord(ref argN2, "CornerYRadius");
```

C# defaults both to 20 DXP units (= 2,000,000 internal units). Our code defaults to 0 via the `Coord` default.

**Fix**: Set default for `corner_x_radius` and `corner_y_radius` to `Coord::from_internal(20 * 100_000)`.

---

### G6: Documentation bug — `binary-pin-format.md` has wrong field order

**Doc**: `docs/schlib/binary-pin-format.md`
**C# ref**: `FileFormatV5.cs:566-629`

The documentation shows IEEE symbol bytes immediately after the binary code byte (offset 0x01). The actual format (verified in both Rust and C#) has 7 bytes between them:

| Offset | Size | Field |
|--------|------|-------|
| 0x00 | 1 | binary_code (0x02) |
| 0x01 | 4 | owner_index (i32 LE) — **missing from docs** |
| 0x05 | 2 | owner_part_id (i16 LE) — **missing from docs** |
| 0x07 | 1 | owner_part_display_mode (u8) — **missing from docs** |
| 0x08 | 1 | symbol_inner_edge |
| ... | | (rest of fields) |

Our Rust parsing code IS correct and matches C#. Only the docs are wrong.

**Fix**: Update `docs/schlib/binary-pin-format.md` to include the 3 missing fields.

---

## AMBIGUITY — Needs user decision

### A1: `GraphicallyLocked` preservation vs Altium compatibility

See M3. Altium writes `GraphicallyLocked` to the file but always resets it to `false` on import. Our code preserves the file value.

**Options**:
- **(a) Match Altium**: Always set to `false` on parse — behavioral parity
- **(b) Preserve file value**: Keep current behavior — better round-trip fidelity (recommended given our design philosophy)
- **(c) Parse + document**: Read from file but document that Altium ignores it

---

## Summary

| Category   | Count | IDs |
|------------|-------|-----|
| MISMATCH   | 3     | M1, M2, M3 |
| GAP        | 6     | G1, G2, G3, G4, G5, G6 |
| AMBIGUITY  | 1     | A1 |
| **Total**  | **10** |  |

## Validated as Correct (no issues found)

- **Record type enum (V1)**: All 65 `SchRecordType` variants match C# `BinaryFileCode.cs`
- **Constant strings (V2)**: 400+ constants validated, zero mismatches vs `FileFormatConsts.cs`
- **Dispatch coverage (V5)**: All 22 SchLib-relevant record types dispatched
- **Stream names (V6)**: All 14+ stream names match exactly
- **Pin binary parsing (V8)**: All 18 fields match C# field-by-field (order, sizes, encoding)
- **Pin conglomerate bitmask (V2)**: All 7 bits match `SchDataPin` mask definitions
- **Pin sidecar streams (V7)**: All 9 streams present and correctly ordered
- **Coordinate encoding**: DXP units * 100,000 = internal units, correctly applied
- **Parameter key strings (V3)**: Spot-checked across all record types, including intentional quirks (`IsNotAccesible`, `OverideColors`, `SymBol_Inner`)

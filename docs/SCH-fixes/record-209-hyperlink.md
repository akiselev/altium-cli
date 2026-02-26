# RECORD=209 (Note) and RECORD=226 (Hyperlink) Research Report

## Clarification: Record Identity

The task title says "RECORD=209 (Hyperlink)" but this is **incorrect**:

- **RECORD=209** is **Note** (CNote=209, TObjectId.eNote)
- **RECORD=226** is **Hyperlink** (CHyperlink=226, TObjectId.eHyperlink)

Source: `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/BinaryFileCode.cs:237,271`

Both records have issues in the current codebase. This report covers both.

---

## Issue 1: RECORD=209 (Note) Missing Fields

### Current Implementation

File: `crates/altium-format/src/sch_records.rs:1802-1835`

```rust
/// Note record (RECORD=209).
pub(crate) struct SchNote {
    pub base: SchPrimitiveBase,
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub color: Color,
    pub area_color: Color,
    pub text: String,
    pub author: String,
    pub font_id: i32,
    pub text_color: Color,
    pub is_solid: bool,
    pub show_border: bool,
    pub word_wrap: bool,
    pub clip_to_rect: bool,
    pub text_margin: Coord,
    pub unique_id: String,
}
```

### C# Reference (FileFormatV5.cs:2372-2459)

ExportNote field order:
1. ExportGraphicalObject (base fields)
2. Location.X, Location.Y (Coord)
3. Corner.X, Corner.Y (Coord)
4. **LineWidth** (Size/TSize) -- MISSING
5. Color (Color)
6. AreaColor (Color)
7. TextColor (Color)
8. FontID (FontID)
9. IsSolid (Boolean)
10. ShowBorder (Boolean)
11. **Alignment** (HorizontalAlign / THorizontalAlign) -- MISSING
12. WordWrap (Boolean)
13. ClipToRect (Boolean)
14. Text (Export_Text, not Export_DynamicString)
15. TextMargin (Coord, default=500000)
16. **Collapsed** (Boolean) -- MISSING
17. Author (DynamicString)
18. UniqueID (DynamicString)

### Missing Fields

| Field | C# Type | Altium Serializer | Rust Type | Default | Constant |
|-------|---------|-------------------|-----------|---------|----------|
| LineWidth | Rt_Schematic.TSize | Import_Size | `PenWidth` | `PenWidth::Zero` (eZeroSize) | `LINE_WIDTH` |
| Alignment | THorizontalAlign | Import_HorizontalAlign | See note below | `1` (eLeftAlign) | `ALIGNMENT` |
| Collapsed | bool | Import_Boolean | `bool` | `false` | `COLLAPSED` |

### THorizontalAlign Type

C# definition (`AD26-dotnet/Altium.SDK.Interfaces/SCH/THorizontalAlign.cs`):

```csharp
public enum THorizontalAlign {
    eHorizontalCentreAlign,  // 0
    eLeftAlign,              // 1
    eRightAlign              // 2
}
```

**IMPORTANT**: This is NOT the same as `TextJustification` (9 values, 0-8). The current TextFrame
implementation incorrectly uses `TextJustification` for ALIGNMENT -- this is also a bug to fix.

There is currently NO `HorizontalAlign` type in `altium-format-types`. One needs to be added:

```rust
/// Horizontal text alignment (0-2).
/// Maps to C# THorizontalAlign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum HorizontalAlign {
    Center = 0,
    #[default]
    Left = 1,
    Right = 2,
}
```

### Recommended Fix for SchNote

```rust
/// Note record (RECORD=209).
///
/// Field order matches Altium's `ExportNote` (FileFormatV5.cs:2372-2397).
pub(crate) struct SchNote {
    pub base: SchPrimitiveBase,
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub line_width: PenWidth,        // <-- ADD
    pub color: Color,
    pub area_color: Color,
    pub text_color: Color,
    pub font_id: i32,
    pub is_solid: bool,
    pub show_border: bool,
    pub alignment: HorizontalAlign,  // <-- ADD (new type needed)
    pub word_wrap: bool,
    pub clip_to_rect: bool,
    pub text: String,
    pub author: String,
    pub text_margin: Coord,
    pub collapsed: bool,             // <-- ADD
    pub unique_id: String,
}
```

### Note on Default Values from C#

The ImportNote code has quirks:
- `IsSolid` default is `true`, but SetIsSolid is called with hardcoded `true` (ignores import)
- `ShowBorder` default is `false`, but SetShowBorder is called with hardcoded `true` (ignores import)
- `LineWidth` is imported but then SetLineWidth is called with hardcoded `eZeroSize` (ignores import!)
- `Alignment` default is `eLeftAlign` (=1)
- `TextMargin` default is 500000
- `Collapsed` default is `false`

These appear to be bugs in Altium's own code. For parsing we should use the actual imported
values (not the hardcoded overrides), since we're reading what's in the file.

---

## Issue 2: RECORD=226 (Hyperlink) Not Implemented

### Current Status

Hyperlink (RECORD=226, SchRecordType::Hyperlink) is defined in the SchRecordType enum
(`crates/altium-format-types/src/sch.rs:119`) but is **NOT dispatched** in:
- `crates/altium-format/src/schdoc/dispatch.rs` (falls through to UnknownRecordType error)

There is no `SchHyperlink` struct.

### C# Reference (FileFormatV5.cs:935-943)

```csharp
protected override void ExportHyperlink(ISchDataSerializer argSerializer, ISchDataObject argObject) {
    ExportLabel(argSerializer, argObject);
}

protected override void ImportHyperlink(ISchDataSerializer argSerializer, ISchDataObject argObject) {
    ImportLabel(argSerializer, argObject);
}
```

Hyperlink inherits ALL fields from Label, with no additional parameters.

### ISch_Hyperlink Interface

```
ISch_Hyperlink : ISch_Label, ISch_GraphicalObject, ISch_BasicContainer
```

Only additional property vs ISch_Label:
- `GetState_Url()` / `SetState_Url(string)`

But the URL field is already part of Label's export (see ExportLabel line 884:
`argSerializer.Export_DynamicString(schDataLabel.GetUrl(), "URL")`).

### Label Field List (from ImportLabel, FileFormatV5.cs:894-932)

1. ImportGraphicalObject (base)
2. Location.X, Location.Y (Coord)
3. Orientation (RotationBy90)
4. Justification (TextJustification)
5. Color (Color)
6. FontID (FontID)
7. Text (DynamicString) -- included for label/hyperlink, excluded for HarnessLayoutLabel
8. IsMirrored (Boolean)
9. URL (DynamicString)
10. UniqueID (DynamicString)

### Recommended Fix for Hyperlink

Since Hyperlink has identical fields to Label, there are two options:

**Option A (Reuse SchLabel)**: Dispatch RECORD=226 to the same `SchLabel` struct, using a
`Hyperlink(SchLabel)` variant in SchRecord.

**Option B (Dedicated struct)**: Create a `SchHyperlink` with identical fields to SchLabel.

Option A is preferred since the C# code literally delegates to ImportLabel/ExportLabel.

Changes needed:
1. Add `Hyperlink(SchLabel)` variant to `SchRecord` enum
2. Add dispatch in `dispatch.rs`: `SchRecordType::Hyperlink => dispatch!(SchLabel => SchRecord::Hyperlink)`
3. Add `SchRecord::Hyperlink(_) => SchRecordType::Hyperlink` in `record_type_for`
4. Add serialization arm in `to_params` match

---

## Issue 3: TextFrame ALIGNMENT Type Mismatch (Related)

### Current Code

`crates/altium-format/src/sch_records.rs:1249`:
```rust
#[param(key = ALIGNMENT, default = TextJustification::BottomLeft)]
pub alignment: TextJustification,
```

### C# Code (ImportTextFrame, FileFormatV5.cs:1971-1972)

```csharp
THorizontalAlign argN8 = THorizontalAlign.eLeftAlign;
argSerializer.Import_HorizontalAlign(ref argN8, "Alignment");
```

TextFrame uses `THorizontalAlign` (3 values), not `TTextJustification` (9 values).
The current Rust code using `TextJustification` is wrong.

### Fix

Change TextFrame's alignment field from `TextJustification` to `HorizontalAlign` (the new type).

---

## Summary of All Changes Needed

### 1. New Type in `altium-format-types/src/sch.rs`

Add `HorizontalAlign` enum (3 variants: Center=0, Left=1, Right=2) with TryFrom<u8>.

### 2. Fix SchNote (`crates/altium-format/src/sch_records.rs`)

Add three missing fields:
- `line_width: PenWidth` (key=LINE_WIDTH, default=PenWidth::Zero)
- `alignment: HorizontalAlign` (key=ALIGNMENT, default=HorizontalAlign::Left)
- `collapsed: bool` (key=COLLAPSED, default=false)

### 3. Fix SchTextFrame (`crates/altium-format/src/sch_records.rs`)

Change `alignment` type from `TextJustification` to `HorizontalAlign`.
Change default from `TextJustification::BottomLeft` to `HorizontalAlign::Left`.

### 4. Add Hyperlink support (`crates/altium-format/src/sch_records.rs` + `dispatch.rs`)

- Add `Hyperlink(SchLabel)` variant to `SchRecord`
- Add dispatch, record_type_for, and to_params entries

### 5. Constants

All needed constants already exist:
- `ALIGNMENT` in `constants/text.rs:72`
- `COLLAPSED` in `constants/record_structure.rs:349`
- `LINE_WIDTH` in `constants/visual.rs:75`

### 6. Export `HorizontalAlign` from `altium-format-types/src/lib.rs`

Add to the `pub use sch::{ ... }` list.

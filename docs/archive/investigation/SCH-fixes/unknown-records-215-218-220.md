# Unknown Record Types 215, 218, 220 + Blanket (225) COLLAPSED Fix

## Summary

| Record | Name | Status | Blocks |
|--------|------|--------|--------|
| 215 | HarnessConnector | Unimplemented | 18 files |
| 218 | SignalHarness | Unimplemented | 18 files |
| 220 | HighLevelCodeSymbol | Unimplemented | 18 files |
| 225 | Blanket | Missing COLLAPSED param | 18 files |

All four are already defined in `SchRecordType` enum but records 215/218/220 have no
dispatch entry and no Rust struct. Record 225 (Blanket) is implemented but missing the
`COLLAPSED` boolean parameter.

---

## Record 215: HarnessConnector

### Identity
- **RECORD value:** 215
- **SchRecordType variant:** `HarnessConnector`
- **C# data class:** `SchDataHarnessConnector` in `Altium.Sch.DataModel.Objects`
- **C# engine class:** `SchHarnessConnector` in `Altium.Sch.DataModel.EngineObjects`
- **Interface chain:** `ISchDataHarnessConnector` : `ISchDataRectangularEntryContainer` : `ISchDataRectangularGroup` : `ISchDataParametrizedGroup` : `ISchDataGraphicalObject` : `ISchDataContainer` : `ISchDataObject`

### Purpose
Represents a harness connector symbol on the schematic. This is a rectangular container
(like SheetSymbol) that holds HarnessEntry children (RECORD=216) to define signal-level
connections into a harness. It also owns a child HarnessConnectorType object (RECORD=217)
for the connector type label.

### Serialization (from C# `FileFormatV5.ExportHarnessConnector`)

Inherits from `RectangularEntryContainer`, then adds its own fields:

**Inherited from GraphicalObject (base of RectangularEntryContainer):**
- `OWNERINDEX` (i32) - owner index
- `OWNERPARTID` (i32) - owner part ID
- `INDEXINSHEET` (i32) - index in sheet

**Inherited from RectangularEntryContainer:**
- `Location.X` (coord) - X location
- `Location.Y` (coord) - Y location
- `XSize` (coord) - width
- `YSize` (coord) - height
- `LineWidth` (TSize) - pen width
- `Color` (color) - border color
- `AreaColor` (color) - fill color

**HarnessConnector-specific:**
- `PrimaryConnectionPosition` (coord, default 1000000) - wire attach point within connector
- `HarnessConnectorSide` (TLeftRightSide, default eLeftSide) - connector side (left/right)
- `UniqueID` (DynamicString) - unique ID

### Child Records
- **RECORD=216 (HarnessEntry):** Serialized identically to SheetEntry (same as BasicEntryObject). Uses same params: Side, DistanceFromTop, Color, AreaColor, IOType, Name, etc.
- **RECORD=217 (HarnessConnectorType):** Label object with Location.X/Y, Orientation, Justification, Color, FontID, IsHidden, Text, IsMirrored, NotAutoPosition, TextHorzAnchor, TextVertAnchor, UniqueID.

### Constants Available
- `harness::HARNESS_CONNECTOR_SIDE` ("HarnessConnectorSide") - exists
- `harness::PRIMARY_CONNECTION_POSITION` ("PrimaryConnectionPosition") - exists

### Implementation Recommendation
Create `SchHarnessConnector` struct similar to `SchSheetSymbol` but with the
RectangularEntryContainer base + PrimaryConnectionPosition + HarnessConnectorSide + UniqueID.
The struct should be very similar to SheetSymbol but without IsSolid/ShowHiddenFields/SymbolType
and with the harness-specific fields instead.

Since HighLevelCodeSymbol (220) serializes identically to SheetSymbol, and HarnessConnector
serializes identically to RectangularEntryContainer + 3 fields, consider whether these can
share a common base struct.

---

## Record 218: SignalHarness

### Identity
- **RECORD value:** 218
- **SchRecordType variant:** `SignalHarness`
- **C# data class:** `SchDataSignalHarness` in `Altium.Sch.DataModel.Objects`
- **C# engine class:** `SchSignalHarness` in `Altium.Sch.DataModel.EngineObjects`
- **Interface chain:** `ISchDataSignalHarness` : `ISchDataWire` : `ISchDataPolygon` : `ISchDataGraphicalObject` : `ISchDataContainer` : `ISchDataObject`

### Purpose
Represents a signal harness wire on the schematic. This is a polyline-like object (extends
Bus behavior) that connects harness connectors. It is the harness equivalent of a Bus wire.

### Serialization (from C# `FileFormatV5.ExportSignalHarness`)

**IMPORTANT:** SignalHarness does NOT delegate to ExportBus. It has its own serialization
that is essentially identical to Bus but duplicated. Both read exactly the same parameters.

**Inherited from GraphicalObject:**
- `OWNERINDEX` (i32)
- `OWNERPARTID` (i32)
- `INDEXINSHEET` (i32)

**SignalHarness-specific (same layout as Bus):**
- `LineWidth` (TSize) - line width
- `Color` (color) - line color
- `UnderlineColor` (color) - underline color for emphasis
- Vertices: `LOCATIONCOUNT` + indexed `X1`/`Y1`, `X2`/`Y2`, etc. (with ExLocations)
- `UniqueID` (DynamicString)
- `AssignedInterface` (DynamicString) - assigned harness interface name
- `AssignedInterfaceSignal` (DynamicString) - assigned signal within interface

### Constants Available
- `text::UNDERLINE_COLOR` ("UnderlineColor") - exists
- `record_structure::ASSIGNED_INTERFACE` ("AssignedInterface") - exists
- `record_structure::ASSIGNED_INTERFACE_SIGNAL` ("AssignedInterfaceSignal") - exists

### Implementation Recommendation
Create `SchSignalHarness` struct. The parameter layout is identical to Bus except:
1. Bus currently missing `UnderlineColor`, `AssignedInterface`, `AssignedInterfaceSignal` in
   the Rust struct (SchBus only has base, color, line_width, vertices, unique_id)
2. SignalHarness has the exact same parameters

Options:
- **(Recommended)** Create SchSignalHarness as its own struct with all fields (including
  UnderlineColor, AssignedInterface, AssignedInterfaceSignal). Also add the missing fields
  to SchBus since the C# Bus serialization writes them too.
- Alternatively, reuse SchBus struct if the parameter sets are truly identical.

Note: The Bus Rust struct (SchBus) is currently MISSING these parameters compared to C#:
- `UnderlineColor` (color, default 0)
- `AssignedInterface` (DynamicString, default "")
- `AssignedInterfaceSignal` (DynamicString, default "")

These should be added to SchBus as part of this work.

---

## Record 220: HighLevelCodeSymbol

### Identity
- **RECORD value:** 220
- **SchRecordType variant:** `HighLevelCodeSymbol`
- **C# data class:** `SchDataHighLevelCodeSymbol` in `Altium.Sch.DataModel.Objects`
- **Interface chain:** `ISchDataHighLevelCodeSymbol` : `ISchDataSheetSymbol` : `ISchDataRectangularEntryContainer` : `ISchDataRectangularGroup` : `ISchDataParametrizedGroup` : `ISchDataGraphicalObject` : `ISchDataContainer` : `ISchDataObject`

### Purpose
Represents a High-Level Code Symbol on the schematic. This is an Altium feature for
embedding code blocks (VHDL, Verilog) as symbols. It is a subtype of SheetSymbol with
no additional fields -- the serialization is **byte-for-byte identical** to SheetSymbol.

### Serialization (from C# `FileFormatV5.ExportHighLevelCodeSymbol`)

```csharp
protected override void ExportHighLevelCodeSymbol(ISchDataSerializer argSerializer, ISchDataObject argObject)
{
    ExportSheetSymbol(argSerializer, argObject);  // Delegates entirely to SheetSymbol
}
```

**Parameters (identical to SheetSymbol via RectangularEntryContainer + SheetSymbol):**

Inherited from GraphicalObject:
- `OWNERINDEX`, `OWNERPARTID`, `INDEXINSHEET`

Inherited from RectangularEntryContainer:
- `Location.X`, `Location.Y`, `XSize`, `YSize`, `LineWidth`, `Color`, `AreaColor`

SheetSymbol-specific:
- `IsSolid` (bool, default true)
- `ShowHiddenFields` (bool, default false)
- `UniqueID` (String, default "$$$")
- `SymbolType` (DynamicString, mapped via SheetSymbolTypeToString)
- `DesignItemId` (DynamicString)
- `SourceLibraryName` (DynamicString)
- `VaultGUID` (DynamicString)
- `ItemGUID` (DynamicString)
- `RevisionGUID` (DynamicString)
- `RevisionName` (DynamicString)

### Child Records
- **RECORD=221 (HighLevelCodeEntry):** Serialized identically to SheetEntry.
- **RECORD=222 (HighLevelCodeName):** Text label for the code symbol name.
- **RECORD=223 (HighLevelCodeFileName):** Text label for the code file name.

### Implementation Recommendation
Since HighLevelCodeSymbol serializes identically to SheetSymbol, the simplest approach
is to reuse the existing `SchSheetSymbol` struct. In the dispatch table, map
`SchRecordType::HighLevelCodeSymbol` to parse using `SchSheetSymbol` and wrap it in a
`SchRecord::HighLevelCodeSymbol(SchSheetSymbol)` variant.

The same applies to child records 221/222/223 -- they can reuse SheetEntry/SheetName/SheetFileName.

**Note:** The existing SchSheetSymbol is missing these SheetSymbol-specific parameters from C#:
- `ShowHiddenFields` (bool)
- `DesignItemId` (DynamicString)
- `SourceLibraryName` (DynamicString)
- `VaultGUID` (DynamicString)
- `ItemGUID` (DynamicString)
- `RevisionGUID` (DynamicString)
- `RevisionName` (DynamicString)

These should be added to SchSheetSymbol as part of this work if they appear in actual files.

---

## Record 225: Blanket (COLLAPSED parameter missing)

### Identity
- **RECORD value:** 225
- **SchRecordType variant:** `Blanket`
- **C# data class:** `SchDataBlanket` extends `SchDataCollapsiblePolygon`
- **Interface chain:** `ISchDataBlanket` : `ISchDataCollapsiblePolygon` : `ISchDataStraightPolygon` : `ISchDataPolygon` : `ISchDataGraphicalObject` : `ISchDataContainer` : `ISchDataObject`

### Current State
Already implemented as `SchBlanket` in `sch_records.rs` but missing the `COLLAPSED`
parameter from its parent class `SchDataCollapsiblePolygon`.

### The COLLAPSED Parameter

From `SchDataCollapsiblePolygon`:
```csharp
public class SchDataCollapsiblePolygon : SchDataStraightPolygon
{
    private bool collapsed;

    public bool GetCollapsed() => collapsed;
    public void SetCollapsed(bool argValue) { collapsed = argValue; }

    public override void SetDefault(TUnitSystem argUnit)
    {
        base.SetDefault(argUnit);
        collapsed = false;  // default: false
    }
}
```

From `FileFormatV5.ExportBlanket` (line 2751):
```csharp
argSerializer.Export_Boolean(schDataBlanket.GetCollapsed(), "Collapsed");
```

From `FileFormatV5.ImportBlanket` (lines 2786-2788):
```csharp
bool argN4 = false;
argSerializer.Import_Boolean(ref argN4, "Collapsed");
schDataBlanket.SetCollapsed(argN4);
```

### Full Blanket Parameter List (from C#)

Inherited from GraphicalObject:
- `OWNERINDEX`, `OWNERPARTID`, `INDEXINSHEET`

Blanket-specific:
- `Location.X` (coord)
- `Location.Y` (coord)
- `Corner.X` (coord)
- `Corner.Y` (coord)
- `LineWidth` (TSize)
- `Color` (color)
- `AreaColor` (color)
- **`Collapsed` (bool, default false)** -- MISSING from Rust struct
- `LineStyle` (TLineStyle, clamped to < DashDotted)
- Vertices (LOCATIONCOUNT + indexed X/Y with ExLocations)
- `LineStyleExt` (exported via ExportLineStyleExt helper)
- `UniqueID` (DynamicString)

### Fix Required
Add to `SchBlanket` struct:
```rust
#[param(key = COLLAPSED, default = false)]
pub collapsed: bool,
```

The constant `COLLAPSED` already exists at `crates/altium-format-types/src/constants/record_structure.rs:349`.

### Other Records Using COLLAPSED
Several other record types also have COLLAPSED via the CollapsiblePolygon parent:
- **RTFLink (RECORD=241):** `Export_Boolean(GetCollapsed(), "Collapsed")`
- **CompileMask (RECORD=211):** `Export_Boolean(GetCollapsed(), "Collapsed")`
- **Note (RECORD=209):** `Export_Boolean(GetCollapsed(), "Collapsed")`

Status of COLLAPSED in existing Rust structs:
- **CompileMask (RECORD=211):** Already has `collapsed` field -- OK
- **Note (RECORD=209):** **MISSING** `collapsed` field -- needs fix
- **RTFLink (RECORD=241):** Not yet implemented -- will need COLLAPSED when added
- **Blanket (RECORD=225):** **MISSING** `collapsed` field -- needs fix (this report)

---

## Stream Location

All four record types appear in the **main data stream** (`/FileHeader` for SchDoc,
`/Component_N/Data` for SchLib). They are standard schematic objects using the same
`|KEY=VALUE|` pipe-delimited parameter format. They do NOT appear in `/Additional`.

---

## Implementation Priority

1. **Blanket COLLAPSED** (trivial fix, 1 line) -- highest ROI, unblocks validation
2. **HighLevelCodeSymbol** (reuse SheetSymbol, just add dispatch) -- easy
3. **SignalHarness** (new struct, similar to Bus) -- medium
4. **HarnessConnector** (new struct + child records 216/217) -- most work

## Additional Work Discovered

- **SchBus** is missing `UnderlineColor`, `AssignedInterface`, `AssignedInterfaceSignal`
  compared to C# serialization
- **SchSheetSymbol** is missing `ShowHiddenFields`, `DesignItemId`, `SourceLibraryName`,
  `VaultGUID`, `ItemGUID`, `RevisionGUID`, `RevisionName` compared to C# serialization
- **Note/CompileMask/RTFLink** may be missing COLLAPSED parameter

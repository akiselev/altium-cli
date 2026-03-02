# Wirebonds, WirebondTemplates, WirebondBodies

Three CFB sections for IC packaging wire bonding support, gated by
`TStorageFeature::eHasWirebondAtWriteStage` (bit 21).

Feature flags: `PCB.Wirebonding` and `PCB.Wirebonding.3DImprovements`
(from `RT_FeatureNames/Consts.cs`).

## Overview

Wire bonding is used in semiconductor IC packaging to connect die pads to
package bond fingers using thin metal wires. Altium models this with three
interrelated sections:

| Section | Section Index | Delphi Section Class | Description |
|---------|---------------|---------------------|-------------|
| `Wirebonds` | 80 | `Section_Wirebond` | Wire bond primitives (extends Track) |
| `WirebondTemplates` | 81 | (none documented) | Wire loop profile templates |
| `WirebondBodies` | 82 | (none documented) | ComponentBody instances for 3D wire visualization |

All three sections are written when `eHasWirebondAtWriteStage` is set in the
file version info (`IPCB_FileVersionInfoList.AddVersionWirebondsAreUsed()`).

## CFB Storage Layout

Each section follows the standard PcbDoc section pattern:

```
/Wirebonds/
    Header          (4 bytes: u32 LE record count)
    Data            (binary records)
/WirebondTemplates/
    Header          (4 bytes: u32 LE record count)
    Data            (binary records)
/WirebondBodies/
    Header          (4 bytes: u32 LE record count)
    Data            (binary records)
```

## Section 1: Wirebonds

### Inheritance and Type Identity

A wirebond is a specialized track:

```
IPCB_Primitive
  +-- IPCB_Track
        +-- IPCB_Wirebond
```

Delphi class: `TWirebond`, VMT `0x017102b8`, inherits `TTrack`.

- **TObjectId**: Uses the Track object ID (since it inherits TTrack), but is
  distinguished at the section level. The Wirebonds section stores Track-format
  binary records.
- **ViewableObjectId**: `eViewableObject_Wirebond` = 116 (0x74). Created via
  `PcbApi_CreateObjectByViewableObjectId(0x74)`.
- **TGlobalObjId**: `eGlobalWirebond` (enum value in `TGlobalObjId`)

### Section Type

The `Wirebonds` section implements `IPCB_BinarySection` (the standard binary
section interface). Unlike `WirebondTemplates`, it does NOT have a specialized
section interface -- it uses the base binary section like `IPCB_RequiredBinarySection`.

### Binary Record Format

Since `IPCB_Wirebond` extends `IPCB_Track`, the binary record format is the
same as a Track record. The wirebond-specific data is stored in associated
templates and bodies, not in the wirebond record itself.

Track record layout (from existing Track documentation):
- 13-byte common PCB primitive header
- Track-specific fields: x1, y1, x2, y2, width

### Wirebond-Specific Properties (Runtime Only)

These properties are NOT stored in the binary record but are computed/linked at
runtime:

| Property | Source | Type |
|----------|--------|------|
| `WirebondTemplate` | Template link | `IPCB_WirebondTemplate` |
| `PrimitivesAtStart` | Spatial query | `IPCB_PrimitiveList` |
| `PrimitivesAtEnd` | Spatial query | `IPCB_PrimitiveList` |
| `WireBody` | Body link | `IPCB_ComponentBody` |
| `Length3D` | Computed from loop | `int` (Coord) |
| `Count` | Wire loop point count | `int` |
| `Point(start, end, index)` | Wire loop 3D point | `TCoordPoint3D` |

Source: `RT_PCB/IPCB_Wirebond.cs` lines 354-372, `PCB/IPCB_WirebondHelper.cs`

### Delphi API

`PcbApi_QueryWirebond` at address `0x03d5b700` provides get/set access to
wirebond properties from the Delphi side.

## Section 2: WirebondTemplates

### Section Type

`WirebondTemplates` uses the specialized `IPCB_WirebondTemplateSection` interface,
which extends `IPCB_BinarySection`:

```
IPCB_BinarySection
  +-- IPCB_WirebondTemplateSection
```

This is one of the named section specializations in the loading pipeline
(alongside `IPCB_BoardBinarySection`, `IPCB_DimensionsSection`, etc.).

The section adds one method beyond the base:
```csharp
IPCB_WirebondTemplate GetWirebondTemplateByID(string argID);
```

Source: `RT_PCB/IPCB_WirebondTemplateSection.cs`

### Data Model: IPCB_WirebondTemplate

Each template defines a wire loop profile:

```csharp
interface IPCB_WirebondTemplate {
    nint I_ObjectAddress();
    string GetState_StateId();      // Unique template identifier
    string GetState_Name();         // Display name
    IPCB_WireLoop GetState_WireLoop();         // Wire loop geometry
    TWirebondConnectStyle GetState_StartStyle(); // Start connection style
    TWirebondConnectStyle GetState_EndStyle();   // End connection style
}
```

Source: `RT_PCB/IPCB_WirebondTemplate.cs`

### Wire Loop Types

Templates contain a wire loop that defines the 3D wire profile:

```
IPCB_WireLoop
  +-- IPCB_WireLoopJedec     (JEDEC-standard loop profiles)
```

**IPCB_WireLoop** (base):
```csharp
interface IPCB_WireLoop {
    nint I_ObjectAddress();
    TWirebondStandard GetProperty_Standard();  // Which standard
    TCoordPoint GetState_Point(int startHeight, int endHeight, int length, int index);
    int GetState_Count();    // Number of profile points
}
```

**IPCB_WireLoopJedec** (JEDEC profile):
```csharp
interface IPCB_WireLoopJedec : IPCB_WireLoop {
    double GetProperty_StartAngle();   // Alpha angle (degrees, 0-90)
    double GetProperty_EndAngle();     // Beta angle (degrees, 0-90)
    int GetProperty_Height();          // Loop height (Coord units)
}
```

Source: `RT_PCB/IPCB_WireLoop.cs`, `RT_PCB/IPCB_WireLoopJedec.cs`

### Wire Loop Builders

Templates are created via a factory/builder pattern:

```
IPCB_WireLoopBuilder
  +-- IPCB_WireLoopJedecBuilder
```

```csharp
interface IPCB_WireLoopJedecBuilder : IPCB_WireLoopBuilder {
    double GetProperty_StartAngle();
    void SetProperty_StartAngle(double argStartAngle);
    double GetProperty_EndAngle();
    void SetProperty_EndAngle(double argEndAngle);
    int GetProperty_Height();
    void SetProperty_Height(int argHeight);
}
```

Usage (from `PcbTrackDataObject.UpdateWirebondTemplate()`):
```csharp
var factory = board.GetWirebondTemplatesManager().GetTemplateFactory();
var builder = factory.CreateWireLoopBuilder(TWirebondStandard.eWirebondStandardJedecSimplified)
    as IPCB_WireLoopJedecBuilder;
builder.SetProperty_Height(loopHeight);
builder.SetProperty_StartAngle(startAngle);
builder.SetProperty_EndAngle(endAngle);
var template = factory.CreateTemplate(name, startStyle, endStyle, builder.GetWireLoop());
wirebond.SetState_WirebondTemplate(template);
```

Source: `InteractiveProperties.Providers.PCB.DataModel/PcbTrackDataObject.cs` lines 560-569

### Template Management

`IPCB_WirebondTemplatesManager` manages templates at the board level:

```csharp
interface IPCB_WirebondTemplatesManager {
    IPCB_WirebondTemplateFactory GetTemplateFactory();
    IPCB_WirebondTemplate GetTemplateByID(string id);
    ISafeInterfaceList GetTemplates();
    void AddTemplate(IPCB_WirebondTemplate template);
    void DeleteTemplate(string id);
    IPCB_WirebondTemplatesIterator CreateIterator();
}
```

Access: `(board as IPCB_Board2Ex).GetWirebondTemplatesManager()`

Source: `RT_PCB/IPCB_WirebondTemplatesManager.cs`, `PCBInterfaces/IPCB_Board2Ex.cs` line 1260

### Template Binary Format (Likely)

The template section stores parameter-block records (text blocks in the standard
`|KEY=VALUE|` format) with at minimum:
- Template ID (StateId string)
- Template name
- Start style (TWirebondConnectStyle enum byte)
- End style (TWirebondConnectStyle enum byte)
- Wire loop standard (TWirebondStandard enum byte)
- JEDEC parameters: StartAngle (double), EndAngle (double), Height (int Coord)

**NOTE**: The exact binary format is not confirmed from decompilation. The section
implements `IPCB_BinarySection` which typically means the standard Header/Data
format, but the records could be text-parameter blocks or binary structs. Without
test files containing wirebond data, the exact format cannot be verified.

## Section 3: WirebondBodies

### Purpose

Each wirebond has an associated `IPCB_ComponentBody` for 3D visualization of the
wire. This section stores those body records separately from the main
`ComponentBodies6` section.

Access: `IPCB_Wirebond.GetState_WireBody()` returns `IPCB_ComponentBody`

The wire body supports:
- **OverrideColor**: Whether the wire uses a custom color
- **BodyColor3D**: The 3D display color (Win32 COLORREF)

Source: `PcbTrackDataObject.cs` lines 516-558

### Binary Format

The body records are likely in the same format as standard ComponentBody records
from `ComponentBodies6`, since the type is `IPCB_ComponentBody`. The section
presumably uses `IPCB_BinarySection` with standard Header (4 bytes) and Data
streams.

## Key Enumerations

### TWirebondConnectStyle (byte)

Die bond connection type at wire endpoints:

| Value | Name | Description |
|-------|------|-------------|
| 0 | `wcsUnknown` | Unknown/unset |
| 1 | `wcsBall` | Ball bond (thermosonic ball bonding) |
| 2 | `wcsWedge` | Wedge bond (ultrasonic wedge bonding) |

Source: `RT_PCB/TWirebondConnectStyle.cs`

The UI displays these as "Die Bond Type" options. The start style corresponds
to the die-side bond (usually Ball for gold wire, Wedge for aluminum wire).

### TWirebondStandard (byte)

Wire loop profile standard:

| Value | Name | Description |
|-------|------|-------------|
| 0 | `eWirebondStandardUnknown` | Unknown |
| 1 | `eWirebondStandardJedec` | Full JEDEC profile |
| 2 | `eWirebondStandardJedecSimplified` | Simplified JEDEC profile |

Source: `RT_PCB/TWirebondStandard.cs`

The UI always uses `eWirebondStandardJedecSimplified` when creating/modifying
templates (see `PcbTrackDataObject.UpdateWirebondTemplate()`).

## Wirebond Primitive Attributes

From `TPrimitiveAttribute` enum (used for filtering/querying):

| Attribute | Filter Key | Display Name |
|-----------|-----------|--------------|
| `ePrimitiveAttribute_Wirebond_DieBondType` | `DieBondType` | Die Bond Type |
| `ePrimitiveAttribute_Wirebond_LoopHeight` | `LoopHeight` | Loop Height |
| `ePrimitiveAttribute_Wirebond_Length3d` | `Length3D` | Length 3D |
| `ePrimitiveAttribute_Wirebond_Diameter` | `Diameter` | Diameter |
| `ePrimitiveAttribute_Wirebond_AlphaAngle` | `AlphaAngle` | Alpha Angle |
| `ePrimitiveAttribute_Wirebond_BetaAngle` | `BetaAngle` | Beta Angle |

Source: `xPCBTypes/TPrimitiveAttribute.cs` lines 526-531, `xPCBTypes/Consts.cs`

## Wirebond DRC Rule

The wirebond rule type (`eRule_Wirebonding`, TRuleKind = 68) governs wire
bonding design constraints:

**Interface**: `IPCB_WirebondRule` extends `IPCB_Rule`

| Property | Type | Description |
|----------|------|-------------|
| `WireToWireGap` | `int` (Coord) | Minimum gap between adjacent wires |
| `MinWireLength` | `int` (Coord) | Minimum wire length |
| `MaxWireLength` | `int` (Coord) | Maximum wire length |
| `BondFingerSpace` | `int` (Coord) | Bond finger spacing |
| `BondFingerMargin` | `int` (Coord) | Bond finger margin |
| `Angle` | `double` | Wire angle constraint |
| `BondFingerToWireAlignment` | `bool` | Alignment constraint |

Source: `RT_PCB/IPCB_WirebondRule.cs` lines 418-445

Rule string key: `"WireBonding"` (from `Consts.cs` line 1190)

### DRC Defaults

From `PcbIntegrationMapper.SetWirebondDefaultValues()`:
- `WireToWire` = `Constants.Rules.DefaultWireToWire`
- `BondFingerMargin` = `Constants.Rules.DefaultBondFingerMargin`
- `MaxWireLength` = `Constants.Rules.DefaultMaxWireLength`
- `MinWireLength` = `Constants.Rules.DefaultMinWireLength`

### DRC Violations

| Type | Interface | Description |
|------|-----------|-------------|
| `eViolation_WirebondLength` (1) | `IPCB_WirebondLengthViolation` | Wire length out of range |
| `eViolation_WirebondAngleClearance` (5) | `IPCB_WirebondWireToWireViolation` | Wire-to-wire clearance |
| `eViolation_WirebondShortCircuit` (8) | (unknown) | Wire short circuit |

**IPCB_WirebondLengthViolation** extends `IPCB_Violation`:
```csharp
int GetState_ActualLength();
void SetState_ActualLength(int argValue);
```

**IPCB_WirebondWireToWireViolation** extends `IPCB_ClearanceViolation`:
```csharp
int GetState_ActualClosestDistance();
void SetState_ActualClosestDistance(int argValue);
TCoordPoint3D GetProperty_ViolationPt3D1();
TCoordPoint3D GetProperty_ViolationPt3D2();
```

These violations use 3D coordinate points because wire bonding is inherently
a 3D operation (wires loop through the Z axis).

Source: `RT_PCB/IPCB_WirebondLengthViolation.cs`, `RT_PCB/IPCB_WirebondWireToWireViolation.cs`

## Relationship Between the Three Sections

```
+-------------------+       references       +----------------------+
| Wirebonds         |----------------------->| WirebondTemplates    |
| (IPCB_Wirebond)   |  GetState_WirebondTemplate()  | (IPCB_WirebondTemplate)|
+-------------------+                        +----------------------+
        |                                             |
        | GetState_WireBody()                         | GetState_WireLoop()
        v                                             v
+-------------------+                        +----------------------+
| WirebondBodies    |                        | IPCB_WireLoop        |
| (IPCB_ComponentBody)|                      | (IPCB_WireLoopJedec) |
+-------------------+                        +----------------------+
```

1. **Wirebonds** stores the geometric primitives (track-like: start/end points, width/diameter, layer, net)
2. **WirebondTemplates** stores the wire loop profiles (JEDEC standard, angles, height, connection styles)
3. **WirebondBodies** stores the 3D visualization bodies (color, 3D model)

Each wirebond references one template (many wirebonds can share a template) and
has one associated body for 3D rendering.

## Collaborative Editing Object Kinds

From `RT_Comparison.Interfaces/Consts.cs`:
- `ObjectKindPcbWirebond` = `"Wirebond"`
- `ObjectKindPcbWirebondTemplate` = `"WirebondTemplate"`
- `ObjectKindPcbRuleWirebond` = `"RuleWirebond"`

## Existing Type Support in altium-format-types

- `ViewableObjectId::Wirebond` = 116 (`crates/altium-format-types/src/pcb.rs`)
- `ViewableObjectId::RuleWirebonding` = 117
- `TGlobalObjId::eGlobalWirebond` in the .NET enum

## Source References

| File | Description |
|------|-------------|
| `AD26-dotnet/Altium.SDK.Interfaces/PCB/IPCB_Wirebond.cs` | SDK wirebond interface |
| `AD26-dotnet/Altium.SDK.Interfaces/PCB/IPCB_WirebondHelper.cs` | Helper extension methods |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_Wirebond.cs` | RT wirebond interface (full) |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_WirebondTemplate.cs` | Template interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_WirebondTemplateSection.cs` | Section interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_WirebondTemplatesManager.cs` | Template manager |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_WirebondTemplateFactory.cs` | Template factory |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_WirebondRule.cs` | DRC rule interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_WireLoop.cs` | Wire loop base |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_WireLoopJedec.cs` | JEDEC wire loop |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_WireLoopBuilder.cs` | Loop builder base |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_WireLoopJedecBuilder.cs` | JEDEC loop builder |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TWirebondConnectStyle.cs` | Connect style enum |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TWirebondStandard.cs` | Wire standard enum |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_WirebondWireToWireViolation.cs` | DRC violation |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_WirebondLengthViolation.cs` | DRC violation |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_AdditionalObjectFactory.cs` | Factory (CreateWirebond) |
| `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_Board2Ex.cs` | Board extension (template mgr) |
| `AD26-dotnet/InteractiveProperties.Providers.PCB.DataModel/PcbTrackDataObject.cs` | UI data model |
| `AD26-dotnet/InteractiveProperties.Providers.PCB.Views/.../WirebondingProfileViewModel.cs` | UI view model |
| `AD26-dotnet/ConstraintsManager.Module/.../WirebondData.cs` | Rule constraint data |
| `AD26-dotnet/ConstraintsManager.Module/.../PcbIntegrationMapper.cs` | Rule mapping (lines 3017-3034) |
| `AD26-dotnet/Altium.Edp.Interfaces/xPCBTypes/TPrimitiveAttribute.cs` | Primitive attributes |
| `AD26-dotnet/Altium.Edp.Interfaces/xPCBTypes/Consts.cs` | Attribute strings |

## Open Questions

1. **Exact binary record format for WirebondTemplates**: The section implements
   `IPCB_BinarySection` but the records could be text-parameter blocks or custom
   binary. Without test files, the exact format cannot be confirmed.

2. **WirebondBodies record format**: Likely mirrors ComponentBody format but may
   have wirebond-specific extensions.

3. **Template-to-wirebond linkage in binary**: How the wirebond record references
   its template (by index? by StateId string? embedded in an extended record area?).

4. **eWirebondStandardJedec vs eWirebondStandardJedecSimplified**: The UI only
   uses `JedecSimplified`. The full `Jedec` variant may have additional loop
   parameters.

5. **Third violation type**: `eViolation_WirebondShortCircuit` (8) has no
   corresponding interface found in the decompiled code.

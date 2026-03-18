# Altium ECO System: Document Adapters and Change Detection

Research based on decompiled C# source in `AD26-dotnet/`.

## Overview

The ECO (Engineering Change Order) system works through a **compiled adapter model**.
Altium compiles schematic projects into a normalized adapter tree (documents, components,
nets, pins), then compares successive compiled states to detect changes. The comparison
is NOT a direct SchDoc-vs-PcbDoc diff; instead, the schematic is compiled into a
"flattened" document that represents the physical/logical design, and that compiled
state is what gets compared against prior compiled state or against the PCB.

## Class/Interface Hierarchy

### Base Adapter

```
BaseAdapter (abstract)
  IDDMObject, IDMObject
  - Id: Layer3Id
  - IsValidForComparision(): bool     // note: Altium's spelling
  - DM_ObjectKind(): TWorkspaceObjectId
  - DM_Parameters(int): IParameter
  - DM_UniqueId() etc. (on subclasses)
```

Source: `Altium.Sch.Compilation/Altium.Sch.Compilation.Adapters/BaseAdapter.cs`

### Document Adapters

```
DocumentBaseAdapter : BaseAdapter
  IDDMDocument, IDocument, IDMObject, IDXPDocument
  +-- DocumentAdapter : DocumentBaseAdapter         (abstract, wraps SchematicData)
  |     +-- DocumentLogicalAdapter                   (logical/flat view)
  |     +-- DocumentPhysicalAdapter                  (per-instance physical view)
  +-- DocumentFlattenedAdapter : DocumentBaseAdapter (project-wide flattened view)
```

Key members on `DocumentBaseAdapter`:
- `DM_Components(int)` -> `IComponent`
- `DM_Nets(int)` -> `INet`
- `DM_Parts(int)` -> `IPart`
- `DM_GetDocumentForECO()` -> `IDocument`  (returns ECO copy with pin mapping applied)

Source: `Altium.Sch.Compilation/Altium.Sch.Compilation.Adapters/DocumentBaseAdapter.cs`

The `DM_GetDocumentForECO()` method on `DocumentBaseAdapter` calls:
```csharp
public virtual IDocument DM_GetDocumentForECO()
{
    return ECOUtils.CreateDocumentForECO(this);
}
```

### Component Adapter

```
PartAdapter : BaseAdapter
  IDDMPart, IPart, IDMObject
  - PartInfo: IPartInfo
  - Pins: List<NetItemPinAdapter>
  - DM_UniqueId(): string             // = uniqueIdPath + "\\" + uniqueIdName
  - DM_UniqueIdName(): string         // = PartInfo.UniqueId (the object's UNIQUEID parameter)
  - DM_UniqueIdPath(): string         // = hierarchy path for physical, "$$$" for logical
  - DM_PhysicalDesignator(): string   // e.g. "R1"
  - DM_Footprint(): string
  - DM_Comment(): string
  - IsValidForComparision(): bool     // delegates to PartInfo.IsValidForComparison()

ComponentAdapter : PartAdapter
  IDDMComponent, IComponent
  - SubParts: IReadOnlyList<PartAdapter>       // multi-part components have multiple sub-parts
  - flatUniqueId: string                        // used for matching from flat component list
  - DM_UniqueIdFromFlatComponent(): string
  - DM_PhysicalPath(): string                   // hierarchical path string
```

Source: `Altium.Sch.Compilation/Altium.Sch.Compilation.Adapters/ComponentAdapter.cs`

### Net Adapter

```
NetBaseAdapter : BaseAdapter (abstract)
  - Signal: SignalInfo
  - Items: IDMObjectsList              // all net items (pins, labels, ports, etc.)
  - RemovedItems: IDMObjectsList
  - Lines: List<LineAdapter>           // wire segments
  - DirectiveObjects: List<NetItemDirectiveAdapter>

NetAdapter : NetBaseAdapter
  IDDMNet, INet
  - Index: int                         // net index (1-based, assigned during sort)
  - OverrideNetName: string
  - DM_FullNetName(): string           // the canonical net name
  - DM_PinCount(): int
  - DM_Pins(int): INetItem
  - DM_AllNetItems(int): INetItem
  - IsValidForComparision(): bool
```

Source: `Altium.Sch.Compilation/Altium.Sch.Compilation.Adapters/NetAdapter.cs`

### Pin Adapter

```
NetItemAdapter : BaseAdapter (abstract)
  - Info: ObjectInfo
  - HierarchyPath: HierarchyPath
  - OwnerNetLogical: NetBaseAdapter

NetItemPinAdapter : NetItemAdapter
  IDDMPin, INetItem
  - Part: PartAdapter                  // owning part
  - ElectricalType: TPinElectrical
  - IsDuplicate: bool
  - DM_PinNumber(): string            // = Info.PinName (the designator, e.g. "1")
  - DM_PinName(): string              // the descriptive name
  - DM_PartUniqueId(): string
  - DM_ComponentUniqueId(): string
  - DM_MatchesPadName(padName): bool  // checks mapped pin designators against pad name
  - DM_PartSwapId() / DM_PinSwapId() / DM_PairSwapId() / DM_PartPinSwapId()
```

Source: `Altium.Sch.Compilation/Altium.Sch.Compilation.Adapters/NetItemPinAdapter.cs`


## UniqueID Matching Algorithm

### How UniqueID is Constructed

Components are matched between schematic and PCB using a composite UniqueID:

```
UniqueID = UniqueIdPath + "\\" + UniqueIdName
```

Where:
- **UniqueIdName** = `PartInfo.UniqueId` -- the `UNIQUEID` parameter value stored on
  each schematic component (a random string like `ABCDEFGH`).
- **UniqueIdPath** = The hierarchy path. For logical documents, this is the literal
  string `"$$$"`. For physical documents, it is constructed from the hierarchy of
  sheet symbol UniqueIDs along the instantiation path.

Source: `PartAdapter.DefineUniqueId()` (line 296-299 of PartAdapter.cs):
```csharp
private string DefineUniqueId()
{
    return uniqueIdPath + "\\" + uniqueIdName;
}
```

And from the constructor (line 97-99):
```csharp
uniqueIdName = PartInfo.UniqueId;
uniqueIdPath = ((OwnerDocument is DocumentLogicalAdapter) ? "$$$"
    : PartInfo.HierarchyPath.GetUniqueIdsPathString());
uniqueId = DefineUniqueId();
```

### Flat Component UniqueID

`ComponentAdapter` also has `flatUniqueId` which is set during construction:
```csharp
this.flatUniqueId = (!string.IsNullOrEmpty(flatUniqueId)) ? flatUniqueId : DM_UniqueId();
```

This is exposed via `DM_UniqueIdFromFlatComponent()` and used when the component
is accessed from the flattened project view. The flat UniqueID is the canonical
key for cross-domain matching (SchDoc <-> PcbDoc).

### Hierarchy Path

The hierarchy path (`HierarchyPath`) tracks the chain of sheet symbol instantiations.
For a top-level schematic with no hierarchy, the path is empty.
For repeated channels, each physical instance gets a unique path built from the
UniqueIDs of the sheet symbols along the instantiation chain.

`HierarchyPath.GetUniqueIdsPathString()` serializes this as a backslash-separated
string of UniqueIDs.


## Net Matching Algorithm

### Net Identity = Net Name

Nets are matched **by name**, not by any ID. The `NetAdapter` wraps a `SignalInfo`
which provides the computed net name. The canonical name comes from:

```csharp
private static string CalculateFullNetName(string overrideNetName, SignalInfo signal)
{
    if (!string.IsNullOrEmpty(overrideNetName))
        return overrideNetName;
    return signal?.FullNameInfo?.FullName;
}
```

The `FullName` is computed during schematic compilation based on:
- Net labels on connected wires
- Power objects (e.g. VCC, GND)
- Port names and sheet entry names (for hierarchical nets)
- Auto-generated names for unnamed nets (e.g. `NetC1_1`)

### Net Comparison and Sorting

Nets are sorted using `NetAdapter.Compare()`:
1. Base null check
2. Alphanumeric comparison of `DM_FullNetName()`
3. Pin count (descending)
4. Base comparison from `NetBaseAdapter.Compare`

Nets are assigned sequential indices (1-based) after sorting. The
`NetAdaptersComparator` singleton implements `IComparer<NetAdapter>`.

### Signal Types

The signal system distinguishes:
- `eNormal` -- regular net
- `eSub` -- bus member (sub-signal)
- `eWide` -- bus itself
- `eBus` -- bus adapter
- `eHarness` -- harness net

Global signals span multiple sheets; local signals are sheet-scoped.
`GlobalSignalInfo` contains a list of `LocalSignalInfo` objects.


## Pin-to-Net Mapping

### Data Structure

Each `NetAdapter` contains an `IDMObjectsList Items` that holds all `NetItemAdapter`
objects belonging to that net. Pins specifically are typed as `NetItemPinAdapter` and
filtered via `IDMObjectType.ePin`.

The relationship is bidirectional:
- `NetAdapter.DM_Pins(index)` -- gets pins on a net
- `NetItemPinAdapter.Part` -- gets the owning `PartAdapter`
- `NetItemPinAdapter.Info.GlobalSignal` / `.LocalSignal` -- signal the pin belongs to

### Pin Matching for ECO

Pin designators are matched between schematic and PCB using the **mapped pin
designator** system. `NetItemPinAdapter.DM_MatchesPadName()` checks if a PCB pad
name matches a schematic pin:

```csharp
public override bool DM_MatchesPadName(string padName)
{
    string text = DM_PhysicalPartDesignator();
    foreach (string mappedDesignator in UDMUtils.GetMappedPinDesignators(this, "PCBLIB"))
    {
        if ($"{text}-{mappedDesignator}".Equals(padName, OrdinalIgnoreCase))
            return true;
    }
    return false;
}
```

The pad name format is `"{Designator}-{PinDesignator}"` (e.g. `"R1-1"`, `"U1-A3"`).

Pin mapping can differ from the schematic pin name when the component's footprint
model has a pin map. `UDMUtils.GetMappedPinDesignators(pin, "PCBLIB")` resolves
this mapping.


## ECO Snapshot Creation (ECOUtils)

### `ECOUtils.CreateDocumentForECO()`

This is the **snapshot creation** entry point. It takes a compiled `DocumentBaseAdapter`
and produces an ECO-ready copy with pin mapping applied.

Source: `Altium.Sch.Compilation/Altium.Sch.Compilation.AdaptersUtils/ECOUtils.cs`

Algorithm:

1. **Collect nets with pin remapping**:
   - For each net in the base document, iterate its pins
   - For each pin, get `UDMUtils.GetMappedPinDesignators(pin, "PCBLIB")`
   - If the first mapped designator differs from the original `DM_Id()`, the original
     pin is "removed" and a new `NetItemPinAdapterECOCopy` is created with the mapped ID
   - Additional mapped designators (multi-pad pins) create additional ECO pin copies
   - Nets that had pin changes get wrapped in `NetAdapterECOCopy` with modified item lists

2. **Collect components with pin remapping**:
   - Components that own any affected pins get wrapped in `ComponentAdapterECOCopy`
   - The ECO copy replaces the component's pin list: removes original pins, adds mapped pins
   - Unaffected components are returned as-is

3. **Create document ECO copy**:
   - Based on the document type (Logical/Physical/Flattened), creates the corresponding
     ECO copy (`DocumentLogicalAdapterECOCopy`, etc.)
   - The ECO copy overrides `DM_Nets()`, `DM_Components()`, `DM_NetCount()`,
     `DM_ComponentCount()` to return the modified collections

### ECO Copy Classes

```
DocumentLogicalAdapterECOCopy   : DocumentLogicalAdapter
DocumentPhysicalAdapterECOCopy  : DocumentPhysicalAdapter
DocumentFlattenedAdapterECOCopy : DocumentFlattenedAdapter
  -- all override DM_Nets/DM_Components to return ECO collections
  -- all override DM_GetDocumentForECO() to return `this`

NetAdapterECOCopy : NetAdapter
  INetECOCopy
  -- overrides DM_AllNetItems/DM_Pins/counts with modified item list

ComponentAdapterECOCopy : ComponentAdapter
  -- overrides DM_Pins/DM_PinCount with mapped pin list

NetItemPinAdapterECOCopy : NetItemPinAdapter
  -- overrides DM_Id() to return the mapped pin designator
```

The key insight: **pin mapping for PCB footprints is applied at ECO snapshot time**,
not during initial compilation. This separates schematic-domain pin names from
PCB-domain pad names.


## The Diff Algorithm: CompiledDataComparator

### Entry Point

```csharp
// Altium.Sch.Compilation.ModelUtils/CompiledDataComparator.cs
public static ComparisonResult Compare(
    CompiledData compiledData1,
    CompiledData compiledData2,
    ISchematicsCollection schematics1,
    ISchematicsCollection schematics2)
```

This compares two `CompiledData` snapshots and returns a `ComparisonResult`.

### ComparisonResult

```csharp
public class ComparisonResult
{
    public bool Equal { get; set; } = true;

    // Objects (wires, labels, ports, power objects, sheet entries, etc.)
    public List<Layer3Id> AddedObjectsOccurrencesIds
    public List<Layer3Id> UpdatedObjectsOccurrencesIds
    public List<Layer3Id> RemovedObjectsOccurrencesIds

    // Pins
    public List<Layer3Id> AddedPinsOccurrencesIds
    public List<Layer3Id> UpdatedPinsOccurrencesIds
    public List<Layer3Id> RemovedPinsOccurrencesIds

    // Parts (components)
    public List<Layer3Id> AddedPartsOccurrencesIds
    public List<Layer3Id> UpdatedPartsOccurrencesIds
    public List<Layer3Id> RemovedPartsOccurrencesIds
}
```

### Three-Way Comparison

`CompiledDataComparator.Compare()` runs three sub-comparators:

1. **ObjectsComparator** -- Compares connection links, net labels, cross-sheet
   connectors, ports, power objects, sheet entries, directive objects, bus entries,
   harness entries. These are matched by their `Layer2Id` (the underlying schematic
   object identity).

2. **PinsComparator** -- Compares pins by `Layer2Id`. Within the same `Layer2Id`,
   pins are matched by `HierarchyPath` and project variant description. Then the
   actual pin data is compared using `CompareVisitor.GetPinsComparer()`.

3. **PartsComparator** -- Compares parts/components by `Layer2Id`. Within the same
   `Layer2Id`, parts are matched by `HierarchyPath` and variant type. Then compares:
   - Designator info (physical/logical)
   - Footprint (case-insensitive string compare)
   - Parameters (full comparison: name, value, rawValue, uniqueId, description,
     visibility, virtual flag)
   - Variations (variant kind, alternate part, library links, variant parameters)

### Object Matching Strategy

The matching algorithm for all three comparators follows the same pattern:

```
For each Layer2Id key in snapshot1:
    If key exists in snapshot2:
        For each occurrence in snapshot1's list:
            Find unmatched occurrence in snapshot2's list where:
                - occurrence.Id == Layer3Id.Empty (not yet matched)
                - occurrence matches (type-specific equality check)
            If found:
                Assign snapshot1's Layer3Id to snapshot2's occurrence
                Compare details -> if different, add to Updated list
            If not found:
                Add to Removed list
    Else:
        All occurrences are Removed

For remaining unmatched entries in snapshot2 (Layer3Id == Empty):
    Assign new Layer3Id
    Add to Added list
```

This is a **Layer2Id-keyed, occurrence-based diff**. The `Layer2Id` is the stable
identity from the schematic object model (based on the UNIQUEID parameter value).
Multiple occurrences of the same `Layer2Id` can exist in multi-channel designs
(different `HierarchyPath` values).

### Part/Component Comparison Details

`PartsComparator.Compare(PartInfo, PartInfo)` checks:
1. `layer2CompareVisitor` visitor pattern match (structural equality of the part object)
2. `FullDesignatorInfo.IsEquivalentTo()` (designator strings match)
3. `Footprint` equality (case-insensitive)
4. `HierarchyPath` equality
5. Parameter list equality (order-independent, all fields must match)
6. Variations list equality (order-independent)

For `MultiPartInfo` (multi-gate components), it additionally:
- Compares sub-part count
- Matches sub-parts by `CurrentPartId` (part number within the component)
- Verifies `Part.Id` equality between matched sub-parts
- Recursively compares each matched sub-part pair


## Validity Filtering

Before comparison, objects can be excluded via `IsValidForComparision()`:

- **NetAdapter**: delegates to `NetAdapterUtils.IsValidForComparision()` which checks
  items, removed items, and project state
- **PartAdapter**: delegates to `PartInfo.IsValidForComparison()`
- **NetItemPinAdapter**: `Part.IsValidForComparision() && !IsJumperPin()`

Jumper pins are explicitly excluded from ECO comparison.


## Summary: How ECO Changes Are Detected

1. **Compile** the schematic project into `CompiledData` containing parts, pins, and
   net objects, indexed by `Layer2Id`.

2. **Create ECO snapshot** via `ECOUtils.CreateDocumentForECO()` which applies
   footprint pin mapping to produce the PCB-domain view.

3. **Compare** the new `CompiledData` against the previous `CompiledData` using
   `CompiledDataComparator.Compare()`.

4. **Result** is a `ComparisonResult` with three categories of changes (Added/Updated/Removed)
   for three entity types (Objects, Pins, Parts).

5. Each change is identified by `Layer3Id` which can be resolved back to the
   specific adapter object to generate human-readable ECO messages.

The matching keys are:
- **Components**: `Layer2Id` (from UNIQUEID) + `HierarchyPath` + variant info
- **Nets**: Net name (string comparison, case-sensitive)
- **Pins**: `Layer2Id` (from pin UNIQUEID) + `HierarchyPath` + variant description
- **Objects**: `Layer2Id` (from UNIQUEID) + structural equality

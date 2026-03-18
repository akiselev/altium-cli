# Pin Swap, Part Swap, and Back-Annotation in Altium

Research findings from decompiled C# source code in AD26-dotnet/.

## Overview

Altium's swap system operates through the **Engineering Change Order (ECO)** mechanism.
Pin and part swaps are detected by comparing two project compilations (typically the
schematic netlist vs. the PCB netlist), identifying pin-to-net assignment differences,
and generating ECO modifications that update the schematic to match the PCB.

Key classes in `Altium.WorkspaceManager.Changes`:
- `ChangeManager` -- orchestrates ECO creation and execution
- `PinSwapManager` -- detects and rationalizes pin/part swaps from netlist differences
- `ChangeGroup` -- groups changes by component designator
- `NetGroup` -- groups pin changes within a net
- `AddRemovePair` -- a matched add-node + remove-node pair (a pin moving between nets)
- `PinChangePair` -- a matched pair of pin changes within the same net (pin rename)
- `SubPart` -- tracks sub-part identity for part swaps


## Pin Swap Data Flow (PCB to Schematic)

### 1. Detection Phase

The `PinSwapManager.Run()` method drives the entire flow:

```
Build_FastPinList(Project1, target)    -- index all pins in target project (schematic)
Build_FastPinList(Project2, reference) -- index all pins in reference project (PCB)
Build_ModificationLists()              -- collect AddNode/RemoveNode modifications
Build_FindMatchingAddRemoves()         -- pair up add/remove for same physical pin
Build_GroupsByNet()                     -- group pairs by component designator
Build_CreateSubPartList()              -- identify part-swap candidates
Build_FindSubPartIdChanges()           -- detect cyclic sub-part ID changes
RationalizeChanges_PartSwaps()         -- remove add/remove pairs covered by part swaps
Build_PinChangePairs()                 -- if PinSwapBy_Pin, find pin-level changes
RationalizeChanges_PinChangePairs()    -- remove modifications covered by pin changes
RationalizeChanges_AddRemovePairs()    -- filter remaining by SwapIdPin match
CreateECOs_SubPartChanges()            -- emit eModification_ChangeSubPartID ECOs
CreateECOs_PinChangePairs()            -- emit eModification_ChangePinName ECOs
CreateECOs_AddRemovePairs()            -- emit eModification_SwapPin ECOs
```

### 2. Pin Matching

Pins are matched between projects using their **physical designator**: the string
`DM_PhysicalPartDesignator() + "-" + DM_Id()` (e.g., `U1-3`). The `PinSwapManager`
builds a sorted list of all pins indexed by this key and uses binary search to match
pins between the target (schematic) and reference (PCB) compilations.

### 3. AddRemovePair Formation

When the ChangeManager detects a pin that was removed from one net and added to another,
this creates a matching `Modification_RemoveMember` and `Modification_AddMember`. The
`PinSwapManager` pairs these into `AddRemovePair` objects. Each pair represents a single
pin that needs to move from one net to another.

### 4. ECO Generation

For pin swaps that pass validation (swap IDs match, pin is on a schematic document),
the `AddRemovePair.CreateECO()` method creates:

```csharp
new Modification_ChangeObject(
    TargetDocument,
    TModificationKind.eModification_SwapPin,
    Pin_ToRemoveFromNet(),   // INetItem -- the pin being moved
    Net_ToGainPin()          // INet -- the net it should move to
)
```

The ECO is executed against the target schematic document via
`IDocument.ECO_ChangeObject(mode, eModification_SwapPin, pin, newNet)`.

### 5. Swap Validation (RationalizeChanges_AddRemovePairs)

The `RationalizeChanges_AddRemovePairs()` method checks whether a pin swap can
actually be applied to the schematic:

1. Get `SwapIdPin` from both the reference pin and the target pin
2. Both must be non-empty
3. Both must match (same swap group)
4. `CanBeSwappedOnSchematic(pin, net)` must return true

The `CanBeSwappedOnSchematic` check depends on the project setting:
- **PinSwapBy_Pin**: always returns true (direct pin swap)
- **PinSwapBy_Netlabel**: calls `SchUtils.PinIsValidForSwappingOnSchematic()` which
  checks that the pin has a net label on the schematic that can be redirected


## Part Swap Data Flow

### SubPart Detection

Part swaps are detected when ALL pins of a sub-part are moving between nets in a
consistent pattern. The `ChangeGroup.CreateSubPartList()` method:

1. Filters `AddRemovePair` items that pass `ValidForPartSwap()`:
   - Both old and new pins must have non-empty `SwapId_PartPin`
   - Both must have matching `SwapId_Part` values
   - Target document must be a schematic
2. Groups pairs by `PartId_Old()` into `SubPart` objects
3. Sorts within each SubPart by `SwapPartPinId_Old`

### SubPart Cycle Detection

`ChangeGroup.FindSubPartIdChanges()` uses `SubPartCycles.FindCycles()` to detect
cyclic sub-part ID changes. For example, if sub-part A's pins now match sub-part B's
configuration and vice versa, this forms a 2-cycle. Each SubPart gets a
`ReferenceSubPart` pointing to the SubPart it should swap with.

### SubPart ECO

When a SubPart `CanBeSwapped()` (has a ReferenceSubPart that is not itself), it emits:

```csharp
new Modification_ChangeObject(
    TargetDocument,
    TModificationKind.eModification_ChangeSubPartID,
    Part_ChangeTo(),     // the part to change
    Part_Reference()     // the reference part to swap with
)
```

After part swap ECOs are created, the corresponding `AddRemovePair` modifications are
removed from the modification list (since the part swap subsumes the individual pin moves).


## Three Swap ID Fields on Schematic Pins

Each schematic pin (`SchDataPin`) carries three swap identity fields:

### 1. `SwapIdPin` (string)
- **Parameter name in file**: `SwapIdPin`
- **Purpose**: Identifies the pin swap group. Pins with the same `SwapIdPin` within a
  component can be swapped with each other.
- **Used by**: `NetGroup.Build_PinChangePairs()` for matching via `eMatchBy_PinSwapId`,
  and `RationalizeChanges_AddRemovePairs()` for validation.
- **Accessor**: `INetItem.DM_PinSwapId()`

### 2. `SwapIdPartAndPartPin` (string, compound field)
- **Parameter name in file**: `SwapIDPart`
- **Format**: `"<SwapIdPart>|&|<SwapIdPartPin>"` (separator is literal `|&|`)
- **Default value**: `"|&|"` (empty part and empty partpin)
- **Parsing**: `ParseSwapIdPartAndSwapIdPartPin()` splits on first `|&|`
- **Setting SwapIdPart propagates**: When you set `SwapIdPart` on any pin, it is
  automatically set on ALL pins of the same sub-part (same OwnerPartId and DisplayMode)
- **SwapIdPart**: Identifies the part swap group (component-level)
- **SwapIdPartPin**: Identifies the specific pin within a part swap group (pin-level)
- **Accessors**: `INetItem.DM_PartSwapId()` (part), `INetItem.DM_PartPinSwapId()` (partpin)

### 3. `SwapIdPair` / `PairSwapID` (string)
- **Two storage locations**:
  - Main record: parameter `SwapIdPair` (ASCII-only string in pipe-delimited params)
  - SchLib sidecar: `PinMiscData` stream, parameter `PairSwapID` (Unicode)
- **Purpose**: Identifies differential pair swap grouping. Pins with the same `PairSwapID`
  form a swap pair.
- **Accessor**: `INetItem.DM_PairSwapId()`


## PairSwapID Format and Usage

### Storage in SchLib PinMiscData

The `PairSwapID` is stored in the `PinMiscData` sidecar stream of SchLib files. This stream
uses the standard pin sidecar format:

**Export** (`SchDataExporterLibraryV5.AddPinMiscDataData`):
```
For each pin with non-empty SwapIdPair:
  1. Create parameter string: "|PairSwapID=<value>|"
  2. Encode as UTF-16LE (Encoding.Unicode)
  3. Store as embedded object named "<pin_index>"
```

**Import** (`SchDataImporterLibraryV5.UpdatePinsMiscData`):
```
For each embedded object in PinMiscData:
  1. Parse object name as pin index
  2. Read UTF-16LE string
  3. Extract PairSwapID parameter value
  4. Call pin.SetSwapIdPair(value)
```

### Storage in SchDoc/SchLib Main Record

In the main pin record (pipe-delimited parameters), `SwapIdPair` is stored as an
ASCII-only string via `Export_ASCIIOnlyString`/`Import_ASCIIOnlyString`.


## Swap Group Definitions

### Pin Swap Groups

Pins belong to the same pin swap group when they share the same `SwapIdPin` value within
a component. The ECO system only allows swapping pins that have matching `SwapIdPin` values.

A value of `"0"` or empty string means the pin is NOT swappable (treated as empty by
`ChangeManagerUtils.GetFromPin_SwapId_Pin()`).

### Part Swap Groups

Components belong to the same part swap group when they share the same `SwapIdPart` value.
Within a part swap group, individual pins are identified by their `SwapIdPartPin` value.
For a part swap to be valid:
- Both the old and new pin must have non-empty `SwapIdPartPin`
- Both must have matching `SwapIdPart` values

A value of `"0"` or empty string means no swap group membership.

### Pair Swap Groups (Differential)

Pins in the same `PairSwapID`/`SwapIdPair` group can be swapped as differential pairs.

### PCB Component Flags

The PCB component object (`IPCB_Component`) has explicit enable flags:
- `GetState_EnablePinSwapping()` / `SetState_EnablePinSwapping(bool)`
- `GetState_EnablePartSwapping()` / `SetState_EnablePartSwapping(bool)`

These must be enabled for the interactive PCB routing pin-swap feature to work.

### Project-Level Settings

The project controls which swap methods are available:
- `IProject.DM_GetPinSwapBy_Pin()` -- allow direct pin swaps (pin name/number change)
- `IProject.DM_GetPinSwapBy_Netlabel()` -- allow swaps via net label reassignment


## Back-Annotation Mechanism

Back-annotation in Altium is the process of pushing changes from PCB back to schematic.
It works through the same ECO/ChangeManager infrastructure used for forward annotation.

### Flow

1. **Compile both projects**: The schematic and PCB are compiled independently into
   flattened document models (`DM_DocumentFlattened()`).

2. **Difference detection**: The `DifferenceEngine` (in `Altium.WorkspaceManager.Differences`)
   compares the two compiled projects, generating `Difference` objects. Pin-related
   differences include:
   - `eDifference_ExtraNode` -- pin exists in one project but not the other
   - `eDifference_ChangedPin` -- pin properties differ (swap IDs, etc.)

3. **Modification generation**: Each difference generates one or more `IModification`
   objects via the `ChangeManager`:
   - `eModification_AddNode` -- add a pin to a net
   - `eModification_RemoveNode` -- remove a pin from a net
   - `eModification_SwapPin` -- move a pin to a different net
   - `eModification_ChangePinSwapId_Pin` -- update pin swap ID
   - `eModification_ChangeSubPartID` -- swap sub-part assignments
   - `eModification_ChangePinName` -- rename a pin

4. **Pin swap rationalization**: The `PinSwapManager` runs to detect that add/remove
   node pairs actually represent pin swaps or part swaps, and replaces the raw
   add/remove modifications with higher-level swap modifications.

5. **User review**: The ECO dialog presents all modifications to the user for review.
   Each modification shows:
   - Type description (e.g., "Move Pins To Different Nets")
   - Affected object (e.g., "U1-3 NET_A -> NET_B")
   - Verb ("Modify")
   - Target document

6. **ECO execution**: For each approved modification, `Modification_ChangeObject.ECO_Action()`
   calls `TargetDocument.ECO_ChangeObject(mode, kind, objectToChange, referenceObject)`.
   The schematic server handles the actual pin/net reassignment.

### IChangeManager.DM_CreateECO_SwapPin

There is a direct API for creating swap pin ECOs:
```csharp
void DM_CreateECO_SwapPin(
    IDocument argTargetDocument,
    IComponent argTargetComponent,
    INetItem argTargetPin,
    string argNewPinNumber,
    string argOldPinNet,
    string argNewPinNet
)
```

Note: In the current AD26 codebase, this method throws `NotImplementedException` in
`ChangeManager.DM_CreateECO_SwapPin()`. The actual pin swap logic goes through the
`PinSwapManager` path instead.

### PCB-Side Pin Swap (Interactive Routing)

The PCB editor supports interactive pin swapping during routing through
`IPCB_PinPairsManager`:
- `SwapPins(pinA, pinB, oldNetA, oldNetB)` -- performs the swap on the PCB side
- This creates a pending change that will be detected during the next ECO comparison

The PCB component must have `EnablePinSwapping` set to true.


## UniqueID Tracking Through Swaps

### UniqueID Fields on Pins

Each pin carries several identity fields:
- `DM_FullUniqueId()` -- full hierarchical unique ID (document path + object ID)
- `DM_PartUniqueId()` -- unique ID of the owning part/component

### Role in Pin Matching

The `PinSwapManager` does NOT use UniqueIDs for pin matching during swap detection.
Instead, it uses the **physical designator** (`DM_PhysicalPartDesignator() + "-" + DM_Id()`).
This is because after a pin swap, the pin's net assignment changes but its designator
remains the same.

### SchDoc ConnectedObjectUniqueId

`SchDataPin` has a `ConnectedObjectUniqueId` field that tracks which schematic object
(e.g., wire endpoint) the pin connects to. This is a schematic-internal linking mechanism,
not related to swap identity.

### Identity Preservation

During a pin swap (eModification_SwapPin), the pin object itself does not change --
only its net assignment does. The pin retains its UniqueID, designator, and all other
properties. The swap is purely a netlist connectivity change.

During a part swap (eModification_ChangeSubPartID), the sub-part IDs are reassigned
between two sub-parts of the same component. The component's UniqueID is preserved;
only the PartID assignments change.


## SchLib Metadata That Enables/Constrains Swaps

### Pin Record Parameters (in main Data stream)

| Parameter     | Type          | Purpose                                |
|---------------|---------------|----------------------------------------|
| `SwapIdPin`   | String        | Pin swap group identifier              |
| `SwapIDPart`  | DynamicString | Compound: SwapIdPart + SwapIdPartPin   |
| `SwapIdPair`  | ASCII String  | Differential pair swap group           |

### PinMiscData Sidecar Stream

Contains `PairSwapID` parameter in UTF-16LE encoding. Only present for pins that have
a non-empty `SwapIdPair` value. This is an overflow mechanism for the `SwapIdPair` field
(the sidecar stores data that may not fit in the main record's ASCII-only encoding).

### Electrical Type Constraint

Power pins (`eElectricPower`) are excluded from swap consideration. In
`ChangeManagerUtils.GetAllPinsWithSamePartId()`, pins with electrical type `eElectricPower`
are filtered out when building the swap candidate list.

### OwnerPartId Constraint

When setting `SwapIdPart`, the value is propagated to ALL pins with the same
`OwnerPartId` and `DisplayMode` within the component. This ensures all pins of a
sub-part share the same part swap group identifier. Pins with `OwnerPartId == 0`
(shared across all sub-parts) are excluded from this propagation.


## TModificationKind Enum Values (Swap-Related)

| Value | Name | Description |
|-------|------|-------------|
| 57 | `eModification_SwapPin` | Move pin to a different net |
| 58 | `eModification_ChangePinSwapId_Pin` | Update pin's swap ID |
| 65 | `eModification_ChangePin` | Update pin-part swapping information |
| 67 | `eModification_ChangeSubPartID` | Swap sub-part assignments |
| 92 | `eModification_ChangePin` (via ChangePin) | Update pin swap info |

All swap-related modifications belong to `TModificationGroup.eModificationGroup_Component`.


## Key Source Files

| File | Purpose |
|------|---------|
| `Altium.WorkspaceManager.Changes/PinSwapManager.cs` | Core pin/part swap detection and ECO generation |
| `Altium.WorkspaceManager.Changes/ChangeGroup.cs` | Groups changes by component, manages SubParts |
| `Altium.WorkspaceManager.Changes/AddRemovePair.cs` | Paired add/remove modifications for a single pin |
| `Altium.WorkspaceManager.Changes/SubPart.cs` | Sub-part swap tracking with cycle detection |
| `Altium.WorkspaceManager.Changes/NetGroup.cs` | Pin change pairing within a net |
| `Altium.WorkspaceManager.Changes/PinChangePair.cs` | Matched pin change pair (name swap) |
| `Altium.WorkspaceManager.Changes/ChangeManager.cs` | ECO orchestration |
| `Altium.WorkspaceManager.Changes/ChangeManagerUtils.cs` | Swap ID extraction utilities |
| `Altium.WorkspaceManager.Changes/Modification_ChangeObject.cs` | Change object with swap descriptors |
| `Altium.Sch.DataModel.Objects/SchDataPin.cs` | Pin data model with swap ID fields |
| `Altium.Sch.DataModel.EngineObjects/SchPin.cs` | Pin engine with SwapIdPart propagation |
| `Altium.Sch.DataModel.FileFormats/FileFormatV5.cs` | Serialization of swap fields |
| `Altium.Sch.DataModel.ImportExport/SchDataImporterLibraryV5.cs` | PinMiscData import |
| `Altium.Sch.DataModel.ImportExport/SchDataExporterLibraryV5.cs` | PinMiscData export |
| `Altium.SDK.Interfaces/EDP/TModificationKind.cs` | Modification type enum |
| `Altium.SDK.Interfaces/EDP/INetItem.cs` | Pin interface with swap ID accessors |
| `Altium.SDK.Interfaces/PCB/IPCB_PinPairsManager.cs` | PCB-side pin swap execution |
| `Altium.SDK.Interfaces/PCB/IPCB_Component.cs` | Pin/part swap enable flags |

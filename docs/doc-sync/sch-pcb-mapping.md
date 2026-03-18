# SchDoc-PcbDoc Data Model Mapping (ECO System)

Research based on decompiled C# source in AD26-dotnet/.

## Architecture Overview

The Sch-to-PCB synchronization operates through Altium's ECO (Engineering Change Order) system.
The pipeline is:

1. **Schematic Compilation** (`Altium.Sch.Compilation`): Compiles schematic sheets into a
   Unified Data Model (UDM) -- flattens hierarchy, resolves nets, assigns physical designators
2. **Comparison** (`Altium.WorkspaceManager.Comparators`): Compares the compiled schematic UDM
   against the PCB document's UDM, producing a list of differences
3. **ECO Generation**: Differences become ECO actions (add/remove/change objects)
4. **ECO Execution** (`CustomECOImplementation` / `ProjectECO`): ECO actions are applied to the
   target document via the `IECO` interface

Key assemblies:
- `Altium.Sch.Compilation` -- Schematic compiler, produces UDM adapters
- `Altium.WorkspaceManager.Comparators` -- Comparison engine
- `Altium.WorkspaceManager.ProjectServices` -- ECO execution
- `Altium.Edp.Interfaces` -- UDM interfaces (`IPart`, `IComponent`, `INet`, etc.)

---

## 1. Component Mapping (Sch -> PCB)

### Identity and Matching

Components are matched between Sch and PCB using `TComponentMappingMethod`:

| Method | Enum | Description |
|--------|------|-------------|
| By UniqueId | `eMapByUniqueId` | **Default.** Uses `DM_UniqueId()` which is `UniqueIdPath + "\" + UniqueIdName`. UniqueIdName comes from the component's UniqueId in the schematic. UniqueIdPath is the hierarchy path's UniqueIds concatenated. |
| By Designator | `eMapByDesignator` | Fallback. Matches on `DM_PhysicalDesignator()` (e.g., "R1", "U3"). |
| By Any | `eMapByAny` | Tries UniqueId first, then Designator. |

Source: `ListPair_Components.Find_ComponentMatches()` in `Altium.WorkspaceManager.Comparators`.

When UniqueId-based matching fails (broken links), the user is prompted to choose:
- **Automatic match by designators** (runs `DocumentComparator_ComponentSynchronizer` in full-auto mode)
- **Manual match** (same comparator with UI)
- **Abort**

### UDM Component Model

The schematic side uses a two-level hierarchy:

- **`IComponent`** (extends `IPart`): Represents a full component (all sub-parts combined).
  Has `DM_SubParts()` / `DM_SubPartCount()`. This is the entity compared against PCB.
- **`IPart`**: Represents one part of a multi-part component (e.g., one gate of a quad op-amp).
  Has pins, implementations, parameters.

In the comparator, both are wrapped in `Component` / `Part` classes that cache all relevant
properties from the UDM interfaces.

### Properties Compared for Matched Components

From `ListPair_Components.Report_DifferencesForMatchedPair()`:

| Schematic Property | PCB Property | Difference Type |
|---|---|---|
| `DM_PhysicalDesignator()` | Component designator | `CreateDifferenceObject_ComponentDesignator` |
| `DM_Comment()` | Comment parameter | `CreateDifferenceObject_ComponentComment` |
| `DM_Footprint()` + `FootprintRevisionGUID` | PCB footprint name + GUID | `CreateDifferenceObject_ComponentFootprint` |
| `DM_ComponentKind()` | Component kind | `CreateDifferenceObject_ComponentKind` |
| `DM_DesignItemID()` | Design Item ID | `CreateDifferenceObject_ComponentDesignItemID` |
| `DM_Description()` | Description | `CreateDifferenceObject_ComponentDescription` |
| `DM_SourceLibraryName()` | Source library name | `CreateDifferenceObject_ComponentLibrary` |
| `VaultGUID` + `ItemGUID` + `RevisionGUID` | Managed component link | `CreateDifferenceObject_ManagedComponentLibraryLink` |
| Simulation model (name, text, portmap) | Simulation model params | `CreateDifferenceObject_ComponentSimulationModel` |
| `ConfiguratorName` + `ConfigurationParameters` | Configurable footprint params | `CreateDifferenceObject_ConfigurableFootprintParameters` |

### Schematic-Only Properties (NOT transferred to PCB)

These properties exist on the schematic component but are not compared in the ECO:
- `DM_LocationX()` / `DM_LocationY()` -- Schematic placement coordinates
- `DM_Rotation()` -- Schematic symbol rotation
- `DM_Layer()` -- Always "None" from schematic side
- `DM_DisplayMode()` -- Schematic display mode
- `DM_DesignatorLocationX/Y()` -- Schematic designator position
- `DM_ReferenceLocationX/Y()` -- Always 0 from schematic
- `DM_ChannelOffset()` -- Multi-channel index (used for designator prefixing, not synced)
- `DM_DesignatorLocked()` -- Schematic annotation lock state

---

## 2. Net Mapping

### Net Matching

Nets are matched by **pin membership** (perfect match = all pins match), with a fallback
to **net name** matching via `Find_PartialMatchesByName()`.

From `ListPair_Nets`:

1. **Perfect match**: Two nets match if every pin in net A has a corresponding pin in net B
   (same pin ID + same component designator), and vice versa.
2. **Partial match by name**: Unmatched nets are matched by `DM_FullNetName()` (case-insensitive).

### Net Properties Compared

From `ListPair_Nets.Report_DifferencesForMatchedPair()`:

| Property | Difference Type | ECO Modification |
|---|---|---|
| `DM_FullNetName()` | `CreateDifferenceObject_NetName` | `eModification_ChangeNetName` |
| `DM_NetColor()` | `CreateDifferenceObject_NetColor` | `eModification_ChangeNetColor` |

### Net Items (Pins in Nets)

A pin within a net is identified by:
- `DM_Id()` -- Pin designator (e.g., "1", "A3")
- `DM_PhysicalPartDesignator()` -- Owning component designator (e.g., "U1")

The `SearchId` (typically `<designator>-<pin_id>`) is used for fast lookup.

When a pin exists in the schematic net but not in the PCB net (or vice versa), it generates
add/remove node ECO actions (`eModification_AddNode` / `eModification_RemoveNode`).

### Net Classes

From `ListPair_NetClasses`:
- Matched by member nets (perfect match requires identical net membership)
- Member matching uses `DM_FullNetName()` (case-insensitive)
- Net class name changes generate `CreateDifferenceObject_NetClassName`

### Differential Pairs

From `ListPair_DifferentialPairs`:
- Matched by name (from `ObjectClass.Name`)
- Positive and negative net names compared separately
- Generates `CreateDifferenceObject_DifferentialPairPositiveNet` / `NegativeNet`

### xNets, xNet Classes, xSignal Classes

When Constraint Manager flow is enabled (`project.ConstraintManagerFlow`), additional
comparisons are made:
- `ListPair_xNets` -- Extended nets (nets through series components)
- `ListPair_xNetClasses` -- xNet class membership
- `ListPair_xSignalClasses` -- xSignal class membership
- `ListPair_PinPairs` -- Pin pair definitions
- `ListPair_ClearancesMatrixes` -- Clearance matrix data

---

## 3. Pin-to-Pad Mapping

### Pin Matching

Pins are matched in `ListPair_Pin` by:
1. **Pin ID** (`DM_Id()`) -- The pin designator (e.g., "1", "GND")
2. **Physical Part Designator** -- The owning component's designator

A perfect match requires both `DM_Id` and `DM_PhysicalPartDesignator` to be identical
(case-insensitive via ValueId comparison).

### Pin Properties Compared

| Schematic Pin Property | PCB Pad Property | When Compared |
|---|---|---|
| `DM_Id()` | Pad designator | Always (identity) |
| `PinPackageLength` | Package length | `CreateDifferenceObject_PinPackageLength` |
| `PinPropagationDelay` | Propagation delay | `CreateDifferenceObject_PinPropagationDelay` |

### Pin Similarity Scoring

When exact matching fails, partial matching uses a weighted score:
- Pin ID + Designator match: 0.66
- PinSwapId match: 0.165
- PartSwapId match: 0.165

This supports pin swap and part swap scenarios where pins may have been reassigned.

### Pin Swap IDs

Both `PinSwapId` and `PartSwapId` are carried from schematic to PCB for swap group management.
These are compared during pin matching but are NOT themselves ECO-synced -- they're
informational for the matching algorithm.

---

## 4. Parameter Flow Matrix

### Parameters Synced from Sch to PCB

| Parameter | Direction | ECO Type | Notes |
|---|---|---|---|
| **Designator** | Sch -> PCB | `eModification_ChangeComponentDesignator` / `eModification_AnnotateComponent` | Physical designator after compilation |
| **Comment** | Sch -> PCB | `eModification_ChangeComponentComment` | Evaluated value (formulas resolved) |
| **Description** | Sch -> PCB | `eModification_ChangeComponentDescription` | ComponentDescription from schematic |
| **Footprint** | Sch -> PCB | `eModification_ChangeComponentFootPrint` | Current implementation model name |
| **ComponentKind** | Sch -> PCB | `eModification_ChangeComponentKind` | Standard/Mechanical/Graphical |
| **DesignItemID** | Sch -> PCB | `eModification_ChangeComponentDesignItemID` | Library item identifier |
| **SourceLibraryName** | Sch -> PCB | `eModification_ChangeComponentLibrary` | Library source |
| **VaultGUID/ItemGUID/RevisionGUID** | Sch -> PCB | `eModification_ChangeManagedComponentLibraryLink` | Managed component identity |
| **User Parameters** | Sch <-> PCB | `eModification_AddParameter` / `eModification_RemoveParameter` / `eModification_ChangeParameterValue` | Bidirectional when PrimitiveParamsECO feature enabled |
| **Net Name** | Sch -> PCB | `eModification_ChangeNetName` | Via net comparison |
| **Net Color** | Sch -> PCB | `eModification_ChangeNetColor` | Win32 COLORREF |
| **Rules** | Sch -> PCB | `eModification_AddRule` / `eModification_ChangeRule` / `eModification_RemoveRule` | Schematic directives become PCB rules |
| **Rooms** | Sch -> PCB | `eModification_AddRoom` / `eModification_ChangeRoom` / `eModification_RemoveRoom` | From schematic sheet symbols |
| **Net Classes** | Sch -> PCB | `eModification_AddNetClass` / ... | Net class membership |
| **Component Classes** | Sch -> PCB | Various | Component class membership |
| **Differential Pairs** | Sch -> PCB | `eModification_AddDifferentialPair` / ... | Pair definitions |
| **Channel Classes** | Sch -> PCB | Various | Multi-channel classes |
| **Simulation Model** | Sch -> PCB | `eModification_ChangeComponentSimulationModel` | SIM implementation |
| **Configurable Footprint Params** | Sch -> PCB | `eModification_ChangeFootprintParameters` | Via ConfiguratorName system |
| **Pin Package Length** | Sch -> PCB | `eModification_ChangePinPackageLength` | Pin-level |
| **Pin Propagation Delay** | Sch -> PCB | `eModification_ChangePinPropagationDelay` | Pin-level |

### Parameter Sync Direction

For **user parameters**, the direction depends on the `PrimitiveParamsECO` feature flag:
- When enabled (PCB target or both-SCH), parameters are compared with
  `ReportParameterDifferenceDetails.Value | VariantValue | GroupByComponent`
- This means variant-specific parameter values are also compared
- The comparison itself is symmetric (differences are reported regardless of direction),
  but the ECO dialog lets the user choose which direction to apply

### Parameters NOT Synced

- Schematic coordinates (X, Y, rotation) -- PCB has its own placement
- PCB-only properties (layer, component body, 3D model) -- Not in schematic model
- Schematic graphical primitives (lines, arcs, rectangles within symbol) -- Not in PCB model

---

## 5. Footprint Resolution

### How SchDoc References Footprints

In the schematic, a component has one or more **implementations** (`IComponentImplementation`).
The footprint is determined by finding the "current" implementation of type `"PCBLIB"` (or
`"PCADLib"` as fallback):

```
PartAdapter.GetFootprintModel():
  1. Find implementation where DM_IsCurrent() == true AND DM_ModelType() == "PCBLIB"
  2. Fallback: find current implementation where DM_ModelType() == "PCADLib"
```

The footprint name comes from `DM_ModelName()` of the current implementation.

### Implementation Properties

Each implementation (`IComponentImplementation`) carries:
- `DM_ModelName()` -- Footprint name (e.g., "CAPC0805(2012)125_L")
- `DM_ModelType()` -- Type string ("PCBLIB", "SIM", "SCH", "VHDL", "PCADLib")
- `DM_IsCurrent()` -- Whether this is the active implementation for its type
- `DM_Description()` -- Human-readable description
- `DM_PortMap()` -- Pin-to-pad mapping string
- `DM_UseComponentLibrary()` -- Whether to use the component's library link for model lookup
- `DM_DatafileLocation/Entity/Kind()` -- Library file references
- `DM_IntegratedModel()` -- Whether model is from an integrated library
- `DM_DatalinksLocked()` -- Whether data links can be changed (same as UseComponentLibrary)

### Footprint Change Detection

From `ListPair_Components.Report_DifferencesForMatchedPair()`:

```csharp
if (!StringUtils.SameStr(component.Footprint, component2.Footprint) ||
    !StringUtils.SameStr(component.FootprintRevisionGUID, component2.FootprintRevisionGUID))
```

Both the footprint **name** AND the **RevisionGUID** must match. A footprint change is detected
even if the name is the same but the revision has changed (library update scenario).

If the reference component's footprint is empty, it's treated as an "extra target object"
(component exists in PCB but has no schematic footprint reference).

### Managed Component Footprint GUIDs

For managed (vault) components, the footprint is tracked by:
- `DM_FootprintItemGUID()` -- from `IPartProperties`
- `DM_FootprintRevisionGUID()` -- from `IPartProperties`

These GUIDs are set via `DM_SetFootprintItemGUID()` / `DM_SetFootprintRevisionGUID()` and
come from the implementation's model library link.

---

## 6. Multi-Sheet / Hierarchical Handling

### Sheet Compilation

The schematic compiler (`Altium.Sch.Compilation`) handles hierarchy through:

1. **HierarchyPath**: Each component instance tracks its position in the sheet hierarchy.
   This is a chain of unique IDs from the top sheet down through sheet symbols.

2. **Layer2Id / Layer3Id**: Internal identifiers for schematic sheets (Layer2) and
   compiled objects (Layer3).

3. **DocumentPhysicalAdapter**: Each physical instance of a sheet gets its own document
   adapter. For multi-channel designs, a single sheet template may produce multiple physical
   documents.

### Multi-Channel Designators

Multi-channel designs produce multiple physical instances of the same logical component:

- **Logical Designator**: The designator as written in the schematic (e.g., "R1")
- **Physical Designator**: The designator after channel prefixing (e.g., "CH1_R1")
- **FullPhysicalDesignator**: Includes part suffix for multi-part (e.g., "CH1_U1A")
- **ChannelOffset**: Index of the channel instance, set per-document

The `ComponentAdapter` constructor takes a `topSchematicNameProvider` function that returns
the top-level schematic name for building the `DM_PhysicalPath()`, which is the full
hierarchical path as backslash-separated designator strings.

### UniqueId Construction for Hierarchical Designs

```
UniqueId = UniqueIdPath + "\" + UniqueIdName
```

Where:
- `UniqueIdName` = Component's unique ID within its sheet
- `UniqueIdPath` = Hierarchy path's unique IDs concatenated (or `"$$$"` for logical documents)

This ensures that the same logical component instantiated in multiple channels gets
distinct UniqueIds for PCB matching.

### Flat vs Logical vs Physical Documents

The comparator handles three document types:
- `eIdDocumentFlattened` -- All pins from all sub-parts are collected directly
- `eIdDocumentPhysical` -- Uses `FirstSubPart.Pins` for pin collection
- Regular documents -- Uses `component.Pins` directly

For the schematic side, the comparison uses flattened/physical documents. For PCB, it reads
directly from the PCB board via `ReaderPhysicalPCB`.

### Rooms (from Sheet Symbols)

Room definitions in the schematic (typically auto-generated from sheet symbols in
multi-channel designs) are synced to PCB placement rooms:
- `RoomName` -- Room identifier
- `Scope1Expression` -- PCB query expression defining room membership (e.g., `InComponent('CH1_*')`)
- `Layer` -- Target PCB layer

---

## 7. Design Rule Generation from Schematic

### Rule Comparison

`ListPair_Rules` compares schematic-derived rules against PCB rules:

Rules are matched by:
1. **Perfect match**: Same UniqueId + same RuleKind + same attributes + same scope expressions
2. **Partial match** (score 0.8): Same UniqueId + same scope expressions
3. **Constraint Manager match** (score 0.7): Same RuleKind + same scope expressions

### Rule Properties Compared

- `RuleKind` -- PCB rule type (clearance, width, etc.)
- `UniqueId` -- Rule identity
- `Attributes` -- Serialized rule parameters (pipe-delimited KEY=VALUE)
- `Comment` -- Rule description
- `Scope1Expression` / `Scope2Expression` -- PCB query expressions defining rule applicability

### Impedance-Driven Rules

Special handling for routing width rules (`RuleKind == 2`) that are impedance-driven:
- `MinImp`, `MaxImp`, `FavImp` -- Impedance values
- `CheckConnectedCopper` -- Boolean flag
- Compared via extracted parameter values rather than raw attribute string

### DefinedByLogicalDocument Flag

When a PCB rule is matched to a schematic-derived rule, the PCB rule is marked with
`SetState_DefinedByLogicalDocument(true)`. This flag indicates the rule originated from
schematic directives and should be managed by the ECO system rather than edited directly
in PCB.

### Clearance Matrices (Constraint Manager)

When the Constraint Manager flow is enabled, `ListPair_ClearancesMatrixes` and related
constraint collectors handle the transfer of clearance matrix definitions from schematic
to PCB. This includes:
- Clearance filter adapters
- Same-net clearance rules
- Creepage rules
- Z-axis clearance rules

---

## 8. ECO Operations Summary

### IECO Interface

The core ECO interface supports 5 operations, each with 3 modes:

| Operation | Modes |
|---|---|
| `DM_AddObject` | PerformAction / ValidateAction / CheckSupportForAction |
| `DM_RemoveObject` | PerformAction / ValidateAction / CheckSupportForAction |
| `DM_AddMemberToObject` | PerformAction / ValidateAction / CheckSupportForAction |
| `DM_RemoveMemberFromObject` | PerformAction / ValidateAction / CheckSupportForAction |
| `DM_ChangeObject` | PerformAction / ValidateAction / CheckSupportForAction |

### TModificationKind (Key Entries for Sch-PCB)

Component-level:
- `eModification_AddComponent` / `RemoveComponent`
- `eModification_ChangeComponentDesignator` / `AnnotateComponent`
- `eModification_ChangeComponentFootPrint` / `ChangePhysicalFootPrint`
- `eModification_ChangeComponentComment`
- `eModification_ChangeComponentKind`
- `eModification_ChangeComponentDescription`
- `eModification_ChangeComponentLibrary`
- `eModification_ChangeManagedComponentLibraryLink`

Parameter-level:
- `eModification_AddParameter` / `RemoveParameter`
- `eModification_ChangeParameterName` / `ChangeParameterValue` / `ChangeParameterType`
- `eModification_ChangeComponentParameters` (grouped)

Net-level:
- `eModification_AddNet` / `RemoveNet`
- `eModification_ChangeNetName` / `ChangeNetColor`
- `eModification_AddNode` / `RemoveNode`

Class-level:
- Net classes, component classes, differential pair classes, channel classes, class clusters
- Each has Add/Remove/Change/AddMember/RemoveMember variants

Pin-level:
- `eModification_ChangePinPackageLength`
- `eModification_ChangePinPropagationDelay`
- `eModification_SwapPin` / `ChangePinSwapId_Pin`
- `eModification_AddPin` / `RemovePin`

Rule-level:
- `eModification_AddRule` / `RemoveRule` / `ChangeRule`
- `eModification_ClearancesMatrix`

Room-level:
- `eModification_AddRoom` / `RemoveRoom` / `ChangeRoom`

---

## 9. Comparison Flow (DocumentComparator_Synchronizer)

The full Sch-to-PCB synchronization flow:

1. If PCB target: get `DM_GetDocumentForECO()` version of the reference document
2. Create all list pairs via `DocumentComparator_Logical.ReCreateListPairs()`
3. `CheckAndUpdateComponentLinks()`:
   a. Test if any UniqueId links are broken
   b. If broken: prompt user for match-by-designator or abort
   c. If matched by designator: run `DocumentComparator_ComponentSynchronizer`
   d. Post-process: clear, re-compile, re-elaborate, re-create list pairs
4. For each list pair, in order:
   a. Reuse blocks (if Design Reuse 2.0 enabled)
   b. **Components** -- match and report differences
   c. **Component Classes** -- match and report
   d. **Nets** -- match by pin membership, report name/color changes
   e. **Net Classes** -- match and report
   f. **Differential Pairs** -- match and report
   g. **Differential Pair Classes** -- match and report
   h. **Rooms** -- match and report
   i. **Channel Classes** -- match and report
   j. **Rules** -- match and report
   k. **Class Clusters** -- match and report
   l. **Pins** -- match and report package length / propagation delay
   m. (If Constraint Manager): Clearances matrices, pin pairs, xSignal/xNet/xNetClasses,
      constraint manager data

Each list pair runs:
1. `PreProcess()` -- build search lists
2. `Find_PerfectMatches()` -- exact matches
3. `Find_PartialMatches()` -- similarity-scored matches
4. `Report_DifferencesInMatchedPairs()` -- generate difference objects for matched pairs
5. `Report_ExtraObjects()` -- unmatched objects become add/remove actions
6. `PostProcess()` -- cleanup

---

## 10. Key Source Files

| File | Purpose |
|---|---|
| `AD26-dotnet/Altium.Edp.Interfaces/RT_Workspace/IECO.cs` | Core ECO interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_Workspace/TModificationKind.cs` | All ECO modification types |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_Workspace/IPart.cs` | Part interface (component in UDM) |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_Workspace/IComponent.cs` | Full component interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_Workspace/INet.cs` | Net interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_Workspace/IComponentImplementation.cs` | Implementation (footprint link) |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_Workspace/IPartProperties.cs` | Extended part properties (vault GUIDs) |
| `AD26-dotnet/Altium.WorkspaceManager.Comparators/.../DocumentComparator_Logical.cs` | Main comparator, creates all list pairs |
| `AD26-dotnet/Altium.WorkspaceManager.Comparators/.../DocumentComparator_Synchronizer.cs` | Synchronization flow orchestrator |
| `AD26-dotnet/Altium.WorkspaceManager.Comparators/.../ListPair_Components.cs` | Component matching and differencing |
| `AD26-dotnet/Altium.WorkspaceManager.Comparators/.../ListPair_Nets.cs` | Net matching by pin membership |
| `AD26-dotnet/Altium.WorkspaceManager.Comparators/.../ListPair_Pin.cs` | Pin/pad matching |
| `AD26-dotnet/Altium.WorkspaceManager.Comparators/.../ListPair_Rules.cs` | Design rule comparison |
| `AD26-dotnet/Altium.WorkspaceManager.Comparators/.../Part.cs` | Part wrapper (caches UDM properties) |
| `AD26-dotnet/Altium.WorkspaceManager.Comparators/.../Component.cs` | Component wrapper |
| `AD26-dotnet/Altium.WorkspaceManager.Comparators/.../Net.cs` | Net wrapper |
| `AD26-dotnet/Altium.WorkspaceManager.Comparators/.../Pin.cs` | Pin wrapper |
| `AD26-dotnet/Altium.Sch.Compilation/.../ComponentAdapter.cs` | Compiled component adapter |
| `AD26-dotnet/Altium.Sch.Compilation/.../PartAdapter.cs` | Compiled part adapter (1000+ lines) |
| `AD26-dotnet/Altium.Sch.Compilation/.../ImplementationBaseAdapter.cs` | Footprint model resolution |
| `AD26-dotnet/Altium.Sch.Compilation/.../ComponentAdaptersCollector.cs` | Component collection and flattening |
| `AD26-dotnet/Altium.WorkspaceManager.ProjectServices/.../ProjectECO.cs` | Project-level ECO execution |
| `AD26-dotnet/Altium.WorkspaceManager.ProjectServices/.../CustomECOImplementation.cs` | Base ECO implementation |

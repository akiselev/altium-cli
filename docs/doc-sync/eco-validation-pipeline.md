# Altium ECO (Engineering Change Order) Validation Pipeline

Research based on decompiled C# source code from AD26-dotnet.

## Overview

Altium's ECO system synchronizes design data between documents (typically schematic
and PCB, but also project files, harness documents, and multi-board system designs).
The pipeline has three main stages:

1. **Difference Detection** - Compare two documents/projects to find discrepancies
2. **Change Generation** - Convert differences into ordered ECO modifications
3. **Change Execution** - Apply modifications using the three-phase CheckSupport/Validate/Perform pattern

## Architecture

```
DifferenceManager              ChangeManager                IECO implementers
(compare docs)                 (order & execute)            (per-document handlers)
     |                              |                            |
     | Difference objects           | Modification objects       | ECO_* methods
     | (Difference subclasses)      | (Modification subclasses)  | (TECO_Mode dispatch)
     |                              |                            |
     +---> CreateECO() ----+       +---> PerformActions() --+   |
                            |                                |   |
                            +-> ChangeManager.CreateECO_*() |   |
                                                             +-> ECO_Action(mode)
                                                                  calls doc.ECO_*()
```

### Key Types

| Type | Assembly | Role |
|------|----------|------|
| `DifferenceManager` | Altium.WorkspaceManager.Differences | Detects and stores differences between two docs |
| `Difference` (abstract) | " | Base class for a single detected difference |
| `ChangeManager` | Altium.WorkspaceManager.Changes | Stores, orders, and executes ECO modifications |
| `Modification` | " | Base class for a single ECO change action |
| `ModificationList` | " | Sorted container with by-kind indexing |
| `IECO` | RT_Workspace | COM interface implemented by each document type |
| `CustomECOImplementation` | Altium.WorkspaceManager.ProjectServices | Base class for project-level ECO handlers |
| `TECO_Mode` | RT_Workspace | The three-phase enum: Perform, Validate, CheckSupport |
| `TModificationKind` | RT_Workspace | Enum with 161 change types |
| `TModificationStatus` | RT_Workspace | Status tracking: None, Supported, PassedVerification, etc. |

---

## Stage 1: Difference Detection

`DifferenceManager` compares two project "sides" (Project1 vs Project2, typically
schematic vs PCB or old vs new) and builds a list of `Difference` objects.

### Difference Types

Differences are categorized by `TDifferenceKind` (e.g., `eDifference_ExtraComponent`,
`eDifference_NetName`, `eDifference_ComponentFootprint`). Each maps to a
`TModificationKind` via `RT_Workspace.Consts.DifferenceToModificationMap`.

### Filtering

Differences are filtered before being added to the list:

```csharp
// In DifferenceManager.IsDifferenceValidForProject()
switch (project.DM_GetDifferenceLevel(difference.DifferenceKind))
{
    case TDifferenceCheckLevel.eDifferenceCheck_Off:
        return false;   // User disabled this check category
    case TDifferenceCheckLevel.eDifferenceCheck_On:
        return !difference.IsDifferenceInCaseOnly();  // Ignore case-only diffs
    case TDifferenceCheckLevel.eDifferenceCheck_On_CaseSensitive:
        return true;    // Report everything including case changes
}
```

### Synchronize Direction

Each difference has a `TSynchronizeDecision`:
- `eUpdateNone` - No action (user unchecked it)
- `eUpdateDocument1` - Push change into document 1
- `eUpdateDocument2` - Push change into document 2

The decision determines the `SynchronizeAction`:
- `eAddObjectToDocument` / `eRemoveObjectFromDocument`
- `eChangeObjectInDocument1` / `eChangeObjectInDocument2`
- `eAddMemberToObject` / `eRemoveMemberFromObject`
- `eNoAction`

### Difference Subclass Hierarchy

- `ExtraObjectDifference` - Object exists in one doc but not the other (component,
  net, class, rule, room, etc.). Creates AddObject or RemoveObject ECOs.
- `ChangedObjectDifference` - Object exists in both docs but with different values
  (designator, footprint, comment, net name, etc.). Creates ChangeObject ECOs.
- `ExtraMemberDifference` - Membership difference (pin in net, component in class).
  Creates AddMember or RemoveMember ECOs.
- Specialized subclasses: `Difference_ComponentParameters`,
  `Difference_ComponentSimulationModel`, `Difference_ExtraSystemDesignPinInNet`, etc.

---

## Stage 2: Change Generation (Differences to Modifications)

When the user clicks "Create ECO" in the Differences dialog:

```csharp
// DifferenceManager.Action_CreateECOs()
IChangeManager changeManager = CreateChangeManager(Project1, Project2, ...);

foreach (Difference diff in Differences)
{
    if (diff.ECO_Action(TECO_Mode.eECO_CheckSupportForAction))
    {
        diff.CreateECO(changeManager);  // Populates ChangeManager with Modifications
    }
}

changeManager.NormalizePinSwaps(Project1, Project2);  // Post-processing
changeManager.DoEditProperties();  // Show dialog or execute
```

### CreateECO Implementation

Each `Difference` subclass implements `CreateECO(IChangeManager)`:

**ExtraObjectDifference:**
```csharp
switch (SynchronizeAction())
{
    case eAddObjectToDocument:
        changeManager.CreateECO_AddObject(targetDoc, referenceObject);
        break;
    case eRemoveObjectFromDocument:
        changeManager.CreateECO_RemoveObject(targetDoc, objectToRemove);
        break;
}
```

**ChangedObjectDifference:**
```csharp
switch (SynchronizeAction())
{
    case eChangeObjectInDocument1:
        changeManager.CreateECO_ChangeObject(doc1, modificationKind, obj1, obj2);
        break;
    case eChangeObjectInDocument2:
        changeManager.CreateECO_ChangeObject(doc2, modificationKind, obj2, obj1);
        break;
}
```

### Modification Filtering

When a modification is added to the `ChangeManager`, it is checked against the
project's enabled modification levels:

```csharp
// ChangeManager.AddModification()
IProject ownerProject = ChangeManagerUtils.GetOwnerProject(modification);
if (ownerProject != null)
{
    if (ownerProject.GetModificationLevel(modification.GetModificationKind())
        != TModificationLevel.eModificationLevel_On)
    {
        return;  // Silently skip - user disabled this ECO type in project options
    }
}
Modifications.Add(modification);
```

### Modification Subclasses

| Subclass | ECO_Action delegates to |
|----------|------------------------|
| `Modification_AddObject` | `document.ECO_AddObject(mode, referenceObject)` |
| `Modification_RemoveObject` | `document.ECO_RemoveObject(mode, objectToRemove)` |
| `Modification_ChangeObject` | `document.ECO_ChangeObject(mode, kind, objectToChange, referenceObject, options)` |
| `Modification_AddMember` | `document.ECO_AddMemberToObject(mode, member, parent, targetParent)` |
| `Modification_RemoveMember` | `document.ECO_RemoveMemberFromObject(mode, member, parent)` |
| `Modification_ChangeComponentParameters` | `document.ECO_ChangeComponentParameters(...)` |
| `Modification_ChangeFootprintParameters` | `document.ECO_ChangeFootprintParameters(...)` |
| `Modification_AddParameterVariation` | For variant management |
| `Modification_RemoveParameterVariation` | For variant management |
| `Modification_ChangeComponentVariation` | For variant management |
| `Modification_ChangeParameterVariation` | For variant management |

### Pin Swap Normalization

After all modifications are generated, `PinSwapManager.Run()` post-processes the
change list. It identifies paired AddNode/RemoveNode modifications for the same
component and converts them into SwapPin modifications. This is needed because
the difference engine sees "pin X moved from Net A to Net B" as separate
add/remove operations, but the PCB needs them as atomic pin swaps.

---

## Stage 3: Change Execution - The Three-Phase Pattern

### The TECO_Mode Enum

```csharp
public enum TECO_Mode
{
    eECO_PerformAction,           // 0 - Actually apply the change
    eECO_ValidateAction,          // 1 - Check if change would succeed
    eECO_CheckSupportForAction    // 2 - Check if this change type is supported at all
}
```

### Execution Order

The `ChangeManager.PerformActions()` method applies modifications in a **strict
kind-based ordering** defined by `ChangeManagerUtils.ModificationOrder[]`. This is
a 161-element array that maps each `TModificationKind` to its execution priority.

The execution loop iterates through ALL modification kinds in order:

```csharp
for (TModificationKind i = eModification_Unknown; i <= eModification_RemoveNoConnect; i++)
{
    TModificationKind orderedKind = ChangeManagerUtils.ModificationOrder[(int)i];
    if (Modifications.ModificationsByKindCount(orderedKind) == 0)
        continue;

    foreach (Modification mod in Modifications.ModificationsByKind(orderedKind))
    {
        ProcessModification(mod);
    }
}
```

**The ordering array is initialized from a static data blob** (compiled into the
assembly metadata via `RuntimeHelpers.InitializeArray`), so the exact ordering is
not directly visible in the decompiled code. However, the general principle is:

1. **Removes before adds** - Remove obsolete objects first to avoid naming conflicts
2. **Containers before members** - Add/remove classes before adding/removing members
3. **Structural before cosmetic** - Component adds/removes before parameter changes
4. **Dependencies respected** - Nets exist before pins are added to them

Within each kind, modifications are sorted alphanumerically by their
`ModificationDetails` string.

### The Three-Phase ProcessModification

For each modification, the three phases are executed **sequentially as a pipeline**
with early termination:

```csharp
void ProcessModification(Modification modification)
{
    if (!modification.GetEnabled())
        return;

    // Phase 1: CheckSupport
    if (!modification.ECO_Action(TECO_Mode.eECO_CheckSupportForAction))
        return;  // Not supported - skip silently

    int msgCountBefore = messagesManager.MessagesCount();

    // Phase 2: Validate
    if (modification.ECO_Action(TECO_Mode.eECO_ValidateAction))
    {
        modification.SetStatus(eStatusPassedVerification);

        // Phase 3: Perform
        if (modification.ECO_Action(TECO_Mode.eECO_PerformAction))
        {
            modification.SetStatus(eStatusPassedExecution);
        }
        else
        {
            modification.SetStatus(eStatusFailedExecution);
            result = false;
        }
    }
    else
    {
        modification.SetStatus(eStatusFailedVerification);
        result = false;
    }

    // Capture any error messages emitted during the process
    int msgCountAfter = messagesManager.MessagesCount();
    if (msgCountAfter > 0 && msgCountAfter != msgCountBefore)
    {
        modification.ErrorMessage = messagesManager.Messages(msgCountAfter - 1).GetText();
    }
}
```

### Phase 1: CheckSupportForAction

**Purpose:** Determine if the target document's ECO handler recognizes this change
type at all. This is a capability query -- does the document server know how to
handle this kind of modification?

**What it checks:**
- In `ProjectECO`: Almost always returns `true` (the project handler supports all
  its known modification kinds)
- In document-level handlers (PCB, SCH, System Design): Checks whether the document
  type implements the specific ECO operation. For example, a PCB document supports
  AddComponent but might not support AddConstraintGroup.
- The `Modification.ECO_Action()` base class calls `LoadDocument()` to ensure the
  target document is loaded before checking support.

**Behavior when false:** The modification is silently skipped -- no error status is
set, no error message is generated. The modification simply does not appear in the
results.

### Phase 2: ValidateAction

**Purpose:** Check whether this specific modification can be applied given the
current state of the document. This is a pre-flight check.

**What it checks (examples from `ProjectECO`):**
- `ECO_AddMemberToObject`: Validates that `referenceMember` is an
  `IParameterVariation` and `referenceParent` is an `IComponentVariation`
- `ECO_RemoveMemberFromObject`: Same type checks
- `Modify_ParameterName`: Validates `objectToChange` is `IParameterVariation`
- `Modify_ParameterValue`: Validates `objectToChange` is `IParameterVariation`
- `Modify_FullPartUpdate`: Always returns true (broad acceptance)

**General validation pattern:**
- Type checking: Are the objects of the expected types?
- Existence checking: Does the target object still exist in the document?
- Constraint checking: Would the change violate any constraints?

**Behavior when false:** Status is set to `eStatusFailedVerification`. The
modification is marked as failed, an error message is captured from the messages
panel, and execution continues to the next modification (no rollback of previous
modifications).

### Phase 3: PerformAction

**Purpose:** Actually apply the change to the document.

**What it does (examples from `ProjectECO`):**
- `ECO_AddMemberToObject` (Perform): Creates a new `IParameterVariation`, copies
  name and value from reference, updates variant library
- `ECO_RemoveMemberFromObject` (Perform): Finds the matching variation by identity
  comparison, removes it, updates variant library
- `Modify_ParameterName` (Perform): Calls `SetParameterName()` with the new name,
  updates variant library
- `Modify_ParameterValue` (Perform): Calls `SetVariedValue()` with the new value,
  updates variant library
- `Modify_FullPartUpdate` (Perform): Calls `UpdateComponentVariationFromAltPart()`
  to replace the entire part, then updates variant library

**Behavior when false:** Status is set to `eStatusFailedExecution`. Error message
is captured. Execution continues to next modification -- **there is no rollback**.

---

## The IECO Interface and Document Dispatch

Each document type implements `IECO` (COM interface). When a `Modification` calls
`ECO_Action(mode)`, it delegates to the target document:

```csharp
// Modification_ChangeObject.ECO_Action()
public override bool ECO_Action(TECO_Mode aMode)
{
    LoadDocument(aMode, objectToChange);
    return TargetDocument.ECO_ChangeObject(
        aMode, ModificationKind, objectToChange, referenceObject, updateParameterOptions);
}
```

The document routes this to its registered `IECO` implementer. Known implementers:

| Implementer | Assembly | Domain |
|-------------|----------|--------|
| `ProjectECO` | Altium.WorkspaceManager.ProjectServices | Project-level (variants) |
| `EcoImplementer` | Altium.Designer.SystemDesign | Multi-board system design |
| PCB server (Delphi) | via COM interop | PCB documents |
| SCH server (Delphi) | via COM interop | Schematic documents |
| Harness ECO classes | Altium.Har.*.ECO | Harness documents |

### CustomECOImplementation Base Class

`ProjectECO` inherits from `CustomECOImplementation`, which wraps each ECO
operation in try/catch and emits error messages to the Messages panel:

```csharp
public bool DM_ChangeObject(TECO_Mode argMode, TModificationKind argKind,
    IDMObject argObjectToChange, IDMObject argReferenceObject,
    IUpdateParameterOptions argUpdateParameterOptions)
{
    try
    {
        return ECO_ChangeObject(argMode, argKind, argObjectToChange,
            argReferenceObject, argUpdateParameterOptions);
    }
    catch (Exception ex)
    {
        Message_Error("Change object", ex.Message);
        return false;  // Exception -> return false -> FailedExecution
    }
}
```

### Begin/End Bracketing

ECO operations are bracketed with `DM_Begin()` / `DM_End()` calls:

```csharp
// In ChangeManager.PerformActions():
iECO1?.DM_Begin();
iECO2?.DM_Begin();
documents.ForEach(doc => doc.ECO_Begin());

try
{
    // ... execute all modifications ...
}
finally
{
    documents.ForEach(doc => doc.ECO_End());
    iECO1?.DM_End();
    iECO2?.DM_End();
}
```

In `ProjectECO`, Begin/End implements a **suspend/resume notification** pattern:
- `ECO_Begin()` increments a suspension counter
- `ECO_End()` decrements it; when it reaches 0, calls `DM_SetModified()` once
- This batches the "project modified" notification across all ECO operations

---

## Change Ordering and Dependencies

### The ModificationOrder Array

`ChangeManagerUtils.ModificationOrder` is a static array of 161 `TModificationKind`
values that defines the execution order. The array is initialized from embedded
metadata, but the general ordering principle follows dependency order:

**Inferred ordering categories (removes first, then adds):**

1. Remove pins from nets (`eModification_RemoveNode`)
2. Remove class members
3. Remove rules
4. Remove nets
5. Remove components
6. Change operations (designator, footprint, comment, etc.)
7. Add components
8. Add nets
9. Add pins to nets (`eModification_AddNode`)
10. Add/remove classes
11. Add/remove rooms, rules
12. Parameter changes
13. Implementation/model changes
14. Pin swaps
15. System design operations
16. Harness annotations

### Within-Kind Ordering

Within each modification kind, modifications are sorted by `ModificationDetails`
using an alphanumeric comparator (`Utils.AlphaNumericComparator`). This ensures
deterministic ordering (e.g., C1 before C2 before C10).

### Special Case: AddComponent + Reuse Blocks

When `eModification_AddComponent` modifications are processed, if Design Reuse v1
is active, `EmulateVer1ReuseBlocksModifications()` generates additional reuse-block
modifications and processes them inline before the regular add-component
modifications.

---

## Conflict Detection and Resolution

### No Built-in Conflict Detection Between Modifications

The `ChangeManager` does **not** perform cross-modification conflict detection.
Each modification is processed independently in order. If two modifications conflict
(e.g., one removes a net that another tries to add a pin to), the second
modification will fail at the Validate or Perform phase and be marked as
`eStatusFailedVerification` or `eStatusFailedExecution`.

**The ordering is the conflict resolution mechanism**: by processing removes before
adds, and structural changes before detail changes, most natural conflicts are
avoided.

### Pin Swap Normalization

The `PinSwapManager` performs a specific form of conflict resolution after all
modifications are generated but before execution:

1. Collects all `AddNode` and `RemoveNode` modifications
2. Identifies paired add/remove operations on the same component
3. Converts them to `SwapPin` modifications (which are atomic)

This prevents the failure mode where removing pin A from Net1 and adding pin A to
Net2 would temporarily leave pin A unconnected, potentially triggering validation
errors.

### System Design Conflict Management

The `ConflictManager` in `Altium.Designer.SystemDesign.Eco.ConflictManage` handles
a specific conflict type: **pin net name conflicts** in multi-board designs. When
a pin swap changes net assignments, it creates `ConflictPinNetName` objects that
track the old value and can detect whether the conflict has been resolved.

### User-Level Conflict Resolution

The primary conflict resolution mechanism is **the UI dialog**. The
`ChangeManagementDialog` (accessed via `IWSM_DialogManager2.CreateChangeManagementDialog()`)
shows all modifications with their status (Pass/Fail) and allows users to:

- Enable/disable individual modifications via checkboxes
- View error messages for failed modifications
- Re-validate after making changes
- Generate error reports

---

## Error Handling and Rollback

### No Transactional Rollback

The ECO system does **not** implement rollback. If modification #50 out of 100
fails:

- Modifications 1-49 are already applied to the document
- Modification 50 is marked as `eStatusFailedExecution`
- Modifications 51-100 continue to be processed (they may also fail if they depended
  on #50)
- The user sees a report of which modifications succeeded and which failed

### Error Capture

Errors are captured via the DXP Messages Manager:

```csharp
int msgCountBefore = messagesManager.MessagesCount();
// ... execute modification ...
int msgCountAfter = messagesManager.MessagesCount();
if (msgCountAfter != msgCountBefore)
{
    modification.ErrorMessage = messagesManager.Messages(msgCountAfter - 1).GetText();
}
```

The `CustomECOImplementation` base class also catches exceptions and routes them to
`Message_Error()`, which adds them to the Messages panel.

### Exception Handling in PerformActions

The outer loop wraps each `ProcessModification` in a try/catch:

```csharp
try
{
    ProcessModification(modification);
}
catch
{
    Utils.ShowError("ECO Action Failed\n\n" + modification.GetState_ReportString());
    result = false;
}
```

This ensures that an unhandled exception in one modification does not prevent
processing of remaining modifications.

### Status Tracking

Each modification tracks its status through the `TModificationStatus` enum:

| Status | Meaning |
|--------|---------|
| `eStatusNone` | Not yet processed |
| `eStatusIsSupported` | CheckSupport passed (set during ValidateActions only) |
| `eStatusIsNotSupported` | CheckSupport failed |
| `eStatusPassedVerification` | Validate passed |
| `eStatusFailedVerification` | Validate failed |
| `eStatusPassedExecution` | Perform succeeded |
| `eStatusFailedExecution` | Perform failed |

---

## The User-Facing Workflow

### Workflow 1: Differences-Driven ECO (Update PCB from Schematic)

1. User triggers "Design > Update PCB Document" (or similar)
2. `DifferenceManager` compares compiled schematic netlist with PCB document
3. **Differences Dialog** appears showing all detected differences
   - User sets synchronize direction for each difference
   - User can choose "Report", "ECO", or "Explore"
4. User clicks "Create ECO" (`Action_CreateECOs()`)
5. Each difference with `CheckSupportForAction` passes generates a Modification
6. `NormalizePinSwaps()` converts paired add/remove to swaps
7. **Change Management Dialog** appears
   - Shows all modifications with checkboxes
   - "Validate Changes" button runs CheckSupport + Validate phases
   - Status column shows Pass/Fail for each modification
   - Error messages shown for failures
8. User clicks "Execute Changes"
9. `PerformActions()` runs the three-phase pipeline for each enabled modification
10. Results shown in the dialog (green check / red X per modification)
11. Post-execution: affected SCH documents get `DM_ScrapCompile()` called

### Workflow 2: Silent/Programmatic ECO

```csharp
changeManager.ExecuteChanges(argIsSilent: true);
// Calls PerformActions() directly without showing dialog
```

### Workflow 3: Validate-Only

```csharp
changeManager.ValidateActions();
// Runs CheckSupport + Validate but NOT Perform
// Used by the dialog's "Validate Changes" button
```

The validate-only flow skips modifications already at `eStatusPassedExecution`
(already applied) and uses the sorted modification list rather than the
kind-ordered iteration.

---

## Key Source Files

| File | Path |
|------|------|
| IECO interface | `AD26-dotnet/Altium.Edp.Interfaces/RT_Workspace/IECO.cs` |
| TECO_Mode enum | `AD26-dotnet/Altium.Edp.Interfaces/RT_Workspace/TECO_Mode.cs` |
| TModificationKind enum | `AD26-dotnet/Altium.Edp.Interfaces/RT_Workspace/TModificationKind.cs` |
| TModificationStatus enum | `AD26-dotnet/Altium.Edp.Interfaces/RT_Workspace/TModificationStatus.cs` |
| ChangeManager | `AD26-dotnet/Altium.WorkspaceManager.Changes/.../ChangeManager.cs` |
| ChangeManagerUtils | `AD26-dotnet/Altium.WorkspaceManager.Changes/.../ChangeManagerUtils.cs` |
| Modification base | `AD26-dotnet/Altium.WorkspaceManager.Changes/.../Modification.cs` |
| ModificationList | `AD26-dotnet/Altium.WorkspaceManager.Changes/.../ModificationList.cs` |
| DifferenceManager | `AD26-dotnet/Altium.WorkspaceManager.Differences/.../DifferenceManager.cs` |
| Difference base | `AD26-dotnet/Altium.WorkspaceManager.Differences/.../Difference.cs` |
| CustomECOImplementation | `AD26-dotnet/Altium.WorkspaceManager.ProjectServices/.../CustomECOImplementation.cs` |
| ProjectECO | `AD26-dotnet/Altium.WorkspaceManager.ProjectServices/.../ProjectECO.cs` |
| EcoImplementer (SystemDesign) | `AD26-dotnet/Altium.Designer.SystemDesign/.../EcoImplementer.cs` |
| ConflictManager | `AD26-dotnet/Altium.Designer.SystemDesign/.../ConflictManager.cs` |
| PinSwapManager | `AD26-dotnet/Altium.WorkspaceManager.Changes/.../PinSwapManager.cs` |

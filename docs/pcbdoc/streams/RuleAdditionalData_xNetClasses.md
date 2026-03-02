# RuleAdditionalData and xNetClassesSection

Research findings for two PcbDoc CFB sections: `RuleAdditionalData` (section ID 76) and
`xNetClassesSection` (section ID 79).

---

## 1. xNetClassesSection

### Overview

The `xNetClassesSection` stores **extended net class definitions** in the PcbDoc CFB
container. xNets ("extended nets") represent nets that span across series components
(like termination resistors), treating them as a single logical signal for routing and
constraint purposes. An xNetClass groups multiple xNets together for rule scoping.

### CFB Stream

| Property | Value |
|----------|-------|
| **CFB stream name** | `xNetClassesSection` |
| **Delphi section constant** | `Section_xNetClasses` |
| **Delphi class** | `TxNetClassesSection` |
| **Section ID in stream table** | 79 |
| **Base class** | `TPrimitivesSection` (instance size 0xB0 = 176 bytes) |
| **Format** | Standard parameter-block format (`\|KEY=VALUE\|`) |
| **Substreams** | `xNetClassesSection/Data` + `xNetClassesSection/Header` |

### Inheritance & Section Type

`TxNetClassesSection` inherits directly from `TPrimitivesSection` at the base size (0xB0),
the same as `TClassesSection`, `TSignalClassesSection`, etc. This means it uses the
standard read/write method shared by all primitive sections:
- Block-encoded records in the `Data` stream
- 4-byte header in the `Header` stream (record count)
- Each record is a pipe-delimited parameter string

### Data Structure

xNetClasses follow the same structure as `Classes6` records. Each record is a
`|KEY=VALUE|` parameter string describing a class with its members.

Based on the .NET `TClassMemberKind` enum, xNetClasses use two member kinds:
- `eClassMemberKind_xNet` (11) - individual xNet members within the class
- `eClassMemberKind_xNetClass` (12) - the class itself

The `IPCB_ObjectClusteredClass` interface (used by xNetClasses) has a special method:
```csharp
void AddMemberByName(TClassMemberKind argKind, string argName);
```
This differs from regular classes which use `AddMemberByName(string argName)` without
a kind parameter, because xNetClass members are xNets (kind=11) not the same kind as
the class itself (kind=12).

### Expected Parameters

Based on the Classes6 record pattern and the constraint manager code, each xNetClass
record likely contains:

| Parameter | Type | Description |
|-----------|------|-------------|
| `KIND` | integer | `12` (eClassMemberKind_xNetClass) |
| `NAME` | string | Class name |
| `SUPERCLASS` | boolean | Whether this is a "super class" (class of classes) |
| `CLASSMEMBER0`, `CLASSMEMBER1`, ... | string | Member names (xNet names) |
| `CLASSMEMBERCOUNT` | integer | Number of members |
| `UNIQUEID` | string | Unique identifier |

### Relationship to Other Sections

- **Classes6**: Regular object classes (nets, components, layers, etc.) with
  `ClassMemberKind` values 0-8
- **SignalClasses**: Signal class section (kind=10, `eClassMemberKind_Signal`)
- **xNetClassesSection**: Extended net classes (kinds 11-12)

The constraint manager treats xNetClasses specially:
- When exporting to PCB, xNetClasses use `TClassMemberKind.eClassMemberKind_xNetClass`
  as the top-level class kind
- Members within are added using `TClassMemberKind.eClassMemberKind_xNet`
- Scope expressions use `InxNetClass('ClassName')` syntax

### PCB Panel Modes

xNets and xNetClasses have dedicated panel modes:
```csharp
public enum TPanelMode : byte {
    // ... earlier entries ...
    eModexNets = 13,
    eModexNetClasses = 14
}
```

### Workspace Object IDs

```csharp
eIdxNet = 179,      // TWorkspaceObjectId
eIdxNetClass = 180,  // TWorkspaceObjectId
```

### IxNetClass Interface

The workspace-level `IxNetClass` interface extends `IObjectClass`:
```csharp
public interface IxNetClass : IObjectClass, IDMObject {
    string DM_Name();
    int DM_MemberCount();
    string DM_Members(int argIndex);
}
```

Members are xNet names (strings), accessed by index.

### Observation: Not Present in Test Files

None of our 98 test PcbDoc files contain this section. xNetClasses are an advanced
feature typically used in complex designs with series-terminated signals where the
constraint manager has been configured. The section will only be present in PcbDoc
files that have been synchronized with a constraint manager containing xNet definitions.

---

## 2. RuleAdditionalData

### Overview

The `RuleAdditionalData` section stores **extended data for design rules** that goes
beyond what fits in the standard `Rules6` parameter-block format. This is primarily
used for rules that have complex data like clearance matrices, layer-specific clearance
overrides, and other rule-type-specific additional information.

### CFB Stream

| Property | Value |
|----------|-------|
| **CFB stream name** | `RuleAdditionalData` |
| **Delphi section constant** | `Section_RuleAdditionalData` |
| **Section ID in stream table** | 76 |
| **Format** | Parameter-block format (standard `\|KEY=VALUE\|`) |
| **Substreams** | `RuleAdditionalData/Data` + `RuleAdditionalData/Header` |

### Purpose

In the Delphi codebase, certain rule types have `GetState_Data()` / `SetState_Data()`
methods that return a string containing additional configuration data beyond the base
rule parameters. This pattern appears on:

1. **`IPCB_Rule1`** - Extended rule interface with `GetState_Data()` / `SetState_Data()`
2. **`IPCB_ClearanceMatrixConstraint`** - Clearance matrix rules with their own
   `GetState_Data()` / `SetState_Data()`

The standard `Rules6` section stores the base rule parameters (scope expressions, rule
kind, name, priority, etc.) via `Export_ToParameters()`. The `RuleAdditionalData`
section provides a sidecar for storing the "data" property of rules that have it.

### Relationship to Rules6

Each entry in `RuleAdditionalData` is associated with a rule in `Rules6` by index.
The linkage works as follows:

1. `Rules6` contains all design rules as standard parameter-block records
2. `RuleAdditionalData` contains supplementary data for rules that need it
3. The association is by record index (matching order)

### Rule Types with Additional Data

Based on the IPCB interfaces, rule types that have additional data include:

| Rule Type | Interface | Additional Data |
|-----------|-----------|-----------------|
| Clearance Matrix | `IPCB_ClearanceMatrixConstraint` | Per-object-type clearance matrix |
| Clearance with layer rules | `IPCB_ClearanceGapByLayerConstraint` | Per-layer clearance overrides |
| Other advanced rules | `IPCB_Rule1` | Rule-specific data string |

The `cOldRuleSection` array in xPCBTypes Consts defines which 67 rule kinds belong to
the "old" section format. Rules not in this array may use the additional data section.

### Data Format

The additional data is serialized as parameter strings (same `|KEY=VALUE|` encoding
used throughout PcbDoc). The `IPCB_ClearanceConstraint2` interface has:
```csharp
string ExportToParameterString();
void ImportFromParameterString(string argParameterString);
```

This suggests the additional data is a nested parameter string that gets stored as
a record in the `RuleAdditionalData` section.

### Observation: Not Present in Test Files

Like xNetClassesSection, none of our 98 test PcbDoc files contain this section.
`RuleAdditionalData` is a newer format extension. Older PcbDoc files encode all rule
data within the `Rules6` parameter blocks. The section appears in files created by
newer Altium versions (likely AD20+) that use advanced clearance matrix rules or
other rule types with complex additional data.

---

## 3. Related: ConstraintManager Extra Data

Separate from both sections above, the PCB Board has a **Constraint Manager** system
(accessible via `IPCB_BoardConstraintManager`) that stores xNet differential pair data
as serialized strings:

```csharp
public interface IPCB_BoardConstraintManager {
    int GetState_ConstraintManagerExtraDataCount();
    string GetState_ConstraintManagerExtraData(int argIndex);
    void AddConstraintManagerExtraData(string argSerializedData);
    void ClearConstraintManagerExtraDatas();
}
```

### ConstraintManagerData Types

Currently only one type is defined:
```csharp
public enum ConstraintManagerDataType {
    Unknown = 0,
    XNetDiffPair = 1
}
```

### XNetDiffPair Serialization

XNetDiffPair data is stored in the `ConstraintManager` section (not RuleAdditionalData
or xNetClassesSection) as semicolon-separated strings:

```
XNetDiffPair;scope;prefix;positiveSuffix;negativeSuffix;positiveXNetScope;negativeXNetScope
```

Each "scope" is a colon-separated packed string:
```
ScopeType:IsPredefined(0/1):IsClass(0/1):Unknown:Name
```

This data links xNet differential pairs from the constraint manager to the PCB board
and is imported/exported during constraint synchronization.

---

## 4. ClassMemberKind Enum (Complete)

The `TClassMemberKind` enum in `RT_PCB` defines all class member kinds:

```csharp
public enum TClassMemberKind : byte {
    eClassMemberKind_Net = 0,
    eClassMemberKind_Component = 1,
    eClassMemberKind_FromTo = 2,
    eClassMemberKind_Pad = 3,
    eClassMemberKind_Layer = 4,
    eClassMemberKind_DesignChannel = 5,
    eClassMemberKind_DifferentialPair = 6,
    eClassMemberKind_Polygon = 7,
    eClassMemberKind_SplitPlane = 8,
    eClassMemberKind_ObjectClass = 9,    // "Structure Class"
    eClassMemberKind_Signal = 10,
    eClassMemberKind_xNet = 11,
    eClassMemberKind_xNetClass = 12
}
```

**NOTE**: Our Rust `ClassMemberKind` enum currently only goes to `SplitPlane = 8`.
Values 9-12 need to be added:
- 9 = ObjectClass (used for "Structure Class" / class clusters)
- 10 = Signal (used by SignalClasses section)
- 11 = xNet
- 12 = xNetClass

The `cClassClusterMemberKinds` array indicates that class clusters consist of:
- `eClassMemberKind_ObjectClass` (9)
- `eClassMemberKind_xNetClass` (12)

---

## 5. Source Files Referenced

### xNetClasses

| File | Content |
|------|---------|
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TClassMemberKind.cs` | `TClassMemberKind` enum (byte, 0-12) |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TClassMemberKindConsts.cs` | First/Last bounds |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_Workspace/IxNetClass.cs` | `IxNetClass` interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_Workspace/TWorkspaceObjectId.cs` | `eIdxNet`, `eIdxNetClass` |
| `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/TPanelMode.cs` | `eModexNets`, `eModexNetClasses` |
| `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_ObjectClusteredClass.cs` | Clustered class interface |
| `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_Board_SaveLoadParameters.cs` | `CreateDefaultxNetClasses()` |
| `AD26-dotnet/Altium.WorkspaceManager.Comparators/.../ListPair_xNetClasses.cs` | Comparison logic |
| `AD26-dotnet/Altium.Sch.Compilation/.../XNetClassAdapter.cs` | Schematic-side adapter |
| `AD26-dotnet/Altium.Sch.Compilation/.../ConstraintxNetClassAdapter.cs` | Constraint adapter |
| `AD26-dotnet/ConstraintsManager.Module/.../PcbRuleImporterService.cs` | PCB import (xNet handling) |
| `AD26-dotnet/ConstraintsManager.Module/.../PcbRuleExporterService.cs` | PCB export (xNet handling) |

### RuleAdditionalData

| File | Content |
|------|---------|
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_Rule.cs` | Base rule interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_Rule1.cs` | Extended rule with `GetState_Data()` |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_ClearanceMatrixConstraint.cs` | Matrix with `GetState_Data()` |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_BoardConstraintManager.cs` | Extra data API |
| `AD26-dotnet/Altium.Edp.Classes/.../ConstraintManagerBaseData.cs` | Serialization base |
| `AD26-dotnet/Altium.Edp.Classes/.../ConstraintManagerxNetDiffPairData.cs` | XNetDiffPair serialization |
| `AD26-dotnet/Altium.Edp.Classes/.../ConstraintManagerDataUtils.cs` | Deserialization factory |
| `AD26-dotnet/Altium.Edp.Interfaces/.../ConstraintManagerDataType.cs` | Data type enum |
| `AD26-dotnet/Altium.Edp.Interfaces/.../IConstraintManagerxNetDiffPairData.cs` | DiffPair data interface |

### Schematic Constraints (related)

| File | Content |
|------|---------|
| `AD26-dotnet/Altium.Sch.Core/.../ConstraintxNetClass.cs` | Schematic-side xNetClass |
| `AD26-dotnet/Altium.Sch.Core/.../ConstraintxNetClass.cs` (Dto) | XML DTO with `CXNets`/`CXNet` |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_SchConstraintsManager/ISchConstraintsManager.cs` | SCH constraint manager |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_SchConstraintsManager/ISchConstraintxNetClass.cs` | SCH xNetClass interface |
| `AD26-dotnet/Altium.Sch.Core/.../ConstraintsData.cs` | Constraint data storage |

---

## 6. Implementation Notes

### For xNetClassesSection

- Uses standard `TPrimitivesSection` format - can reuse existing section reader
- Records are `|KEY=VALUE|` parameter strings just like `Classes6`
- Need to extend `ClassMemberKind` enum to include values 9-12
- Need to add `ParamSectionKind::xNetClassesSection` variant
- The section is optional - only present when xNetClasses have been defined

### For RuleAdditionalData

- Also uses standard parameter-block format
- Records contain additional data for rules in the `Rules6` section
- Linked by record index to the corresponding rule
- The `GetState_Data()` string from rule objects is serialized here
- The section is optional - only present when rules have additional data

### Absence in Test Files

Neither section appears in any of the 98 test PcbDoc files. To test these sections,
we would need:
- **xNetClassesSection**: A PcbDoc with series-terminated nets and constraint
  manager synchronization
- **RuleAdditionalData**: A PcbDoc created by AD20+ with advanced clearance matrix
  rules or other rules using the extended data format

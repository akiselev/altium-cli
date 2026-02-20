# Altium Schematic .NET Data Model Reference

Comprehensive reference for the decompiled .NET interfaces that define the Altium Designer schematic data model. Sourced from `AD26-dotnet/` assemblies: `Altium.Edp.Interfaces`, `Altium.Sch.Interfaces`, `Altium.Sch.DataModel`, and `Altium.Sch.Core`.

## Table of Contents

1. [Interface Architecture Overview](#interface-architecture-overview)
2. [Interface Hierarchy](#interface-hierarchy)
3. [Object Type Enumeration (TObjectId)](#object-type-enumeration-tobjectid)
4. [Base Interfaces](#base-interfaces)
5. [Graphical Object Interface](#graphical-object-interface)
6. [Component Interface (ISchComponent)](#component-interface-ischcomponent)
7. [Pin Interface (ISchPin)](#pin-interface-ischpin)
8. [Document Interface (ISchDocument)](#document-interface-ischdocument)
9. [Connectivity Objects](#connectivity-objects)
10. [Iterator and Container Patterns](#iterator-and-container-patterns)
11. [Parameter System](#parameter-system)
12. [Unique ID and Handle Management](#unique-id-and-handle-management)
13. [Cross-Document Reference Patterns](#cross-document-reference-patterns)
14. [Key Enumerations](#key-enumerations)
15. [Modern Data Model Interfaces](#modern-data-model-interfaces)
16. [Implications for altium-format Crate](#implications-for-altium-format-crate)

---

## Interface Architecture Overview

The Altium schematic .NET code has **two generations** of interfaces:

### Legacy COM Interfaces (SCHInterfaces namespace)

- Located in `Altium.Edp.Interfaces/SCHInterfaces/`
- COM-visible with `[Guid]` attributes for interop with Delphi runtime
- All property access via `GetState_*` / `SetState_*` method pairs
- Deep inheritance with extensive `new` keyword re-declarations at each level
- Every concrete interface (ISchComponent, ISchPin, ISchDocument, etc.) re-declares ALL inherited methods with `new`, making the vtable layout explicit for COM interop

### Modern Clean Interfaces (Altium.Sch.Interfaces.Objects namespace)

- Located in `Altium.Sch.Interfaces/Altium.Sch.Interfaces.Objects/`
- Clean `Get*`/`Set*` method pairs without the `State_` infix
- No COM attributes
- Thinner interface surface -- only type-specific properties, no re-declaration of inherited methods
- Used by newer Altium.Sch.DataModel and Altium.Sch.Core code

### Namespace Map

| Namespace | Assembly | Purpose |
|-----------|----------|---------|
| `SCHInterfaces` | Altium.Edp.Interfaces | Legacy COM interfaces for all schematic types |
| `RT_SchDataModel` | Altium.Edp.Interfaces | Iterator, serializer, data model factory, base data object |
| `Rt_Schematic` | Altium.Edp.Interfaces | Enums (TObjectId, TSheetStyle, etc.) and value types (TLocation, TCoordRect) |
| `RT_Workspace` | Altium.Edp.Interfaces | Shared enums (TPinElectrical, TComponentKind) |
| `Altium.Sch.Interfaces.Objects` | Altium.Sch.Interfaces | Modern data interfaces |

---

## Interface Hierarchy

```
IBasicContainer                    [4AB6FF85-A596-46D2-AF30-C0C16DF1A952]
  |
  +-- IGraphicalObject             [E3A2D0B2-B339-4BAA-B746-25B31022D6E2]
        |
        +-- IParametrizedGroup     (mixin: parameter management)
        |
        +-- ISchComponent          [F9D2D109-2D4A-4B7E-AD59-E3B474A35BAA]
        +-- ISchPin                [6AEBE7C3-FB5A-4B1F-90A9-6E5597B98DC5]
        +-- ISchDocument           [442F6722-2D44-4106-9C63-6D25B424BEE5]
        +-- IWire                  [C2802C32-C306-4880-B27D-DB1527F9F41B]
        +-- ISchPort               [3C30254F-D913-446E-A7C6-B903B228B4D5]
        +-- ISchNetLabel           [256E091C-ED2A-4ABE-8229-D0B5A9B55DCA]
        +-- ISchPowerObject        [89C6C8BB-4923-4B17-8CCC-4B1ACF56230B]
        +-- ISchSheetSymbol        [AECA2F23-82BC-4CC8-A08D-59385056FE6B]

ISchDataObject                     [3062B336-6254-4857-A343-907D40AEA411]
  |
  +-- ISchDataContainer
  +-- ISchDataGraphicalObject
  +-- ISchDataParametrizedGroup
  +-- ISchDataComponent  (modern)
  +-- ISchDataPin        (modern)
```

All concrete COM interfaces inherit from **IParametrizedGroup : IGraphicalObject : IBasicContainer**. This triple-inheritance is the standard pattern.

---

## Object Type Enumeration (TObjectId)

Source: `Altium.Edp.Interfaces/Rt_Schematic/TObjectId.cs`

The TObjectId enum defines all schematic record types. Values are sequential starting from 0.

### Core Drawing Primitives (0-21)

| Value | Name | Description |
|-------|------|-------------|
| 0 | `eFirstObjectID` | Sentinel (not a real object) |
| 1 | `eClipBoardContainer` | Clipboard container |
| 2 | `eNote` | Text note annotation |
| 3 | `eProbe` | Simulation probe |
| 4 | `eRectangle` | Rectangle primitive |
| 5 | `eLine` | Line primitive |
| 6 | `eConnectionLine` | Connection line (non-electrical) |
| 7 | `eBusEntry` | Bus entry connector |
| 8 | `eArc` | Arc primitive |
| 9 | `eEllipticalArc` | Elliptical arc |
| 10 | `eRoundRectangle` | Round rectangle |
| 11 | `eImage` | Embedded image |
| 12 | `ePie` | Pie/wedge shape |
| 13 | `eTextFrame` | Text frame |
| 14 | `eRichTextDocument` | Rich text document |
| 15 | `eEllipse` | Ellipse primitive |
| 16 | `eJunction` | Wire junction |
| 17 | `ePolygon` | Polygon primitive |
| 18 | `ePolyline` | Polyline primitive |
| 19 | `eWire` | Electrical wire |
| 20 | `eBus` | Bus line |
| 21 | `eBezier` | Bezier curve |

### Labels and Text (22-30)

| Value | Name | Description |
|-------|------|-------------|
| 22 | `eLabel` | Generic text label |
| 23 | `eHyperlink` | Hyperlink annotation |
| 24 | `eNetLabel` | Net label (assigns net name) |
| 25 | `eDesignator` | Component designator (R1, U1, etc.) |
| 26 | `eSchComponent` | Schematic component (symbol) |
| 27 | `eParameter` | Parameter object |
| 28 | `eParameterSet` | Parameter set container |
| 29 | `eParameterList` | Parameter list container |
| 30 | `eSheetName` | Sheet name label |

### Document Structure (31-37)

| Value | Name | Description |
|-------|------|-------------|
| 31 | `eSheetFileName` | Sheet file name |
| 32 | `eSheet` | Sheet document (SchDoc) |
| 33 | `eSchLib` | Schematic library (SchLib) |
| 34 | `eSymbol` | Library symbol |
| 35 | `eNoERC` | No-ERC marker |
| 36 | `eErrorMarker` | Compilation error marker |
| 37 | `ePin` | Component pin |

### Connectivity (38-48)

| Value | Name | Description |
|-------|------|-------------|
| 38 | `ePort` | Hierarchical port |
| 39 | `ePowerObject` | Power port/symbol |
| 40 | `eSheetEntry` | Sheet entry on sheet symbol |
| 41 | `eSheetSymbol` | Hierarchical sheet symbol |
| 42 | `eTemplate` | Document template |
| 43 | `eTaskHolder` | Task holder |
| 44 | `eMapDefiner` | Map definer |
| 45 | `eImplementationMap` | Implementation map |
| 46 | `eImplementation` | Implementation (footprint link) |
| 47 | `eImplementationsList` | Implementations list container |
| 48 | `eCrossSheetConnector` | Cross-sheet connector |

### Open Bus / Harness (49-66)

| Value | Name | Description |
|-------|------|-------------|
| 49 | `eCompileMask` | Compilation mask |
| 50 | `eOpenBusComponent` | Open bus component |
| 51 | `eOpenBusLink` | Open bus link |
| 52 | `eOpenBusDesignator` | Open bus designator |
| 53 | `eHarnessConnector` | Harness connector |
| 54 | `eHarnessEntry` | Harness entry |
| 55 | `eHarnessConnectorType` | Harness connector type |
| 56 | `eSignalHarness` | Signal harness |
| 57 | `eOpenBusPort` | Open bus port |
| 58 | `eHighLevelCodeSymbol` | High level code symbol |
| 59 | `eHighLevelCodeEntry` | High level code entry |
| 60 | `eOpenBusPinGroup` | Open bus pin group |
| 61 | `eBlanket` | Blanket region |
| 62 | `eRTFLink` | RTF link |
| 63 | `eFSMState` | FSM state |
| 64 | `eFSMTransition` | FSM transition |
| 65 | `eCommentThread` | Comment thread |
| 66 | `eCommentThreadNote` | Comment thread note |

### Extended Objects (67-120)

| Value | Name | Description |
|-------|------|-------------|
| 67 | `eFSMNote` | FSM note |
| 68-82 | `eDiagram*` | Diagram module/connector/block/harness objects |
| 83 | `eVirtualParameter` | Virtual parameter |
| 84-103 | `eHarness*` | Harness wiring/layout/component/wire/splice/shield/twist/cable objects |
| 104 | `eImageParameter` | Image parameter |
| 107 | `eHarnessLibrary` | Harness library |
| 108 | `eLineView` | Line view |
| 110 | `eObjectDefinition` | Object definition |
| 113 | `eElectronicsSystemDesignDocument` | Electronics system design document |
| 114 | `eFunctionalBlock` | Functional block |
| 115 | `eFunctionalConnectionLine` | Functional connection line |
| 116 | `eFunctionalTextFrame` | Functional text frame |
| 117 | `eSchematicBlock` | Schematic block |
| 118 | `eReuseSheetSymbol` | Reuse sheet symbol |
| 119 | `eReuseBlockImplementationInfo` | Reuse block implementation info |
| 120 | `eLastObjectId` | Sentinel (not a real object) |

---

## Base Interfaces

### IBasicContainer

Source: `Altium.Edp.Interfaces/SCHInterfaces/IBasicContainer.cs`
GUID: `4AB6FF85-A596-46D2-AF30-C0C16DF1A952`

The root interface for all schematic objects. Every object in a schematic document implements this.

#### Identity and Type

```csharp
TObjectId GetState_ObjectId();                    // Returns the TObjectId enum value
string GetState_UniqueId();                       // 8-char unique ID (e.g., "ABCDEFGH")
void SetState_UniqueId(string argS);
string GetState_UniqueIdInReuseBlock();           // ID within reuse block context
void SetState_UniqueIdInReuseBlock(string argValue);
string GetState_Handle();                         // Runtime handle string
string GetState_IdentifierString();               // Human-readable identifier
string GetState_DescriptionString();              // Human-readable description
```

#### Container Hierarchy

```csharp
IBasicContainer GetState_Container();             // Direct parent container
IBasicContainer GetState_OriginalContainer();     // Original container (before moves)
IBasicContainer GetState_TopMostContainer();      // Root-level container (usually component or doc)
IBasicContainer GetState_OriginalTopMostContainer();
bool GetState_IsInContainer();                    // Whether object is in a container
IGraphicalObject GetState_GraphicalContainer();   // Owning graphical object
```

#### Containment Operations

```csharp
void AddContainedObject(IBasicContainer argObject);
void RemoveContainedObject(IBasicContainer argObject);
void InsertObjectAt(IBasicContainer argObject, int argi);
int I_IndexOfObject(IBasicContainer argObject);
bool IsContainedIn(IBasicContainer argObject);
bool ContainsAsFieldObject(IBasicContainer argObject);
void DeleteAll();
void FreeAllContainedObjects();
void RemoveAndFreeChildObjects(ref TObjectSet argObjectSet);
int ObjectsCount();
int ObjectsCount_AllLevels();                     // Recursive count
```

#### Object Ordering

```csharp
void MoveObjectToIndex(IBasicContainer argObject, int argIndex);
void MoveObjectToEnd(IBasicContainer argObject);
void MoveObjectToBeginning(IBasicContainer argObject);
void MoveObjectBehindReference(IBasicContainer argObjectToMove, IBasicContainer argReferenceObject);
void MoveObjectBeforeReference(IBasicContainer argObjectToMove, IBasicContainer argReferenceObject);
```

#### Replication and Copy

```csharp
IBasicContainer I_Replicate_SetOriginal();        // Clone, link to original
IBasicContainer I_Replicate_ForgetOriginal();     // Clone, no link
IBasicContainer I_Replicate_TransferOriginal();   // Clone, transfer original link
void I_RestoreFromCopy(IBasicContainer argCopyContainer);
void I_CopyTo_ForgetOriginal(IBasicContainer argContainer);
void I_CopyTo_KeepOriginal(IBasicContainer argContainer);
void CopyUniqueIds(IBasicContainer argSourceContainer);
void RestoreOriginal();
IBasicContainer GetState_Original();
```

#### Parameter Management

```csharp
ISchParameter GetState_ParameterByName(string argName);
int GetState_ParameterCount();
string GetState_ParameterString();                // Serialized parameter string
ISchParameter AddParameter();
ISchParameter AddParameter(TParameterType argParamType, string argName, string argValue);
void RemoveParameter(ISchParameter argParameter);
```

#### Save/Load Support

```csharp
int GetState_IndexInSheetForSave();
void SetState_IndexInSheetForSave(int argValue);
int GetState_OwnerIndexForSave();
void SetState_OwnerIndexForSave(int argValue);
bool GetState_OwnerIndexForSaveAdditionalList();
bool GetState_IgnoreOnLoad();
void SetState_IgnoreOnLoad(bool argValue);
bool GetState_IsAccessible();
void SetState_IsAccessible(bool argValue);
bool GetState_IsSchematicBlockObject();
void GetIteratedObjects(TIterationDepth argIterationDepth, ISafeInterfaceList argList);
void AddAllToListForSave(ISafeInterfaceList argOldObjectsList, ISafeInterfaceList argNewObjectsList);
void AddAllToListForSave_Additional(ISafeInterfaceList argList);
```

#### Misc

```csharp
void SetState_Default(TUnitSystem argUnit, bool argIsNewObject, bool argByDataObject);
ISchDocument GetDocument();
ISchDocument GetOwnerDocumentExt();
ISchDataObject GetSchDataObject();                // Bridge to modern data model
void DestroyObject();
void ResetUniqueIds();
bool ObjectAttributesSame(IBasicContainer argG, bool argSkipServerParameters, bool argIgnoreUndoRedoAttributes);
int ReportAttributesDifferences(IBasicContainer argG, bool argIgnoreSpatialAttributes, ref string argDiffDescription);
void BeforePlacing();
void AfterPlacing(bool argCancelled);
```

### ISchDataObject (Modern Base)

Source: `Altium.Edp.Interfaces/RT_SchDataModel/ISchDataObject.cs`
GUID: `3062B336-6254-4857-A343-907D40AEA411`

The modern data model base. Mirrors IBasicContainer's identity/ownership but with cleaner API.

```csharp
ISch_BasicContainer GetOwner();
void SetOwner(ISch_BasicContainer argValue);
ISchDataObject GetOwnerContainer();
void SetOwnerContainer(ISchDataObject argObj);
ISchDataObject GetOriginalContainer();
void SetOriginalContainer(ISchDataObject argValue);
bool GetIsAccessible();
void SetIsAccessible(bool argValue);
int GetIndexInSheetForSave();
void SetIndexInSheetForSave(int argValue);
bool GetIgnoreOnLoad();
void SetIgnoreOnLoad(bool argValue);
bool GetIgnoreOnSave();
void SetIgnoreOnSave(bool argValue);
bool GetIsSchematicBlockObject();
void SetIsSchematicBlockObject(bool argValue);
TObjectId GetObjectID();                          // Same TObjectId enum as legacy
bool GetInContainer();
void SetInContainer(bool argValue);
string GetUniqueId();
void SetUniqueId(string argValue);
string GetUniqueIdInReuseBlock();
void SetUniqueIdInReuseBlock(string argValue);
ISchDataObject GetTopMostContainer();
ISch_BasicContainer GetTopMostContainerOwner();
ISch_BasicContainer GetOriginalTopMostContainerOwner();
string GetHandle();
void SetHandle(string argValue);
ISch_BasicContainer GetOwnerDocumentOwner();
ISch_BasicContainer GetDocumentOwner();
void UpdateOwner(ISchDataObjectList argList);
ISchDataObject Replicate();
void CopyFieldsTo(ISchDataObject argDataObject);
void SetDefault(TUnitSystem argUnit);
bool IsGlobalObject();
bool HandleNeeded();
void CreateHandle();
void DestroyHandle();
```

---

## Graphical Object Interface

### IGraphicalObject

Source: `Altium.Edp.Interfaces/SCHInterfaces/IGraphicalObject.cs`
GUID: `E3A2D0B2-B339-4BAA-B746-25B31022D6E2`

Extends IBasicContainer with spatial and visual properties. All visible schematic objects implement this.

#### Spatial Properties

```csharp
TLocation GetState_Location();                    // Origin point (X, Y in internal units)
void SetState_Location(TLocation argLocation);
TLocation GetState_CenterLocation();              // Computed center
TCoordRect GetState_OwnBoundingRectangle();       // Bounding rectangle
bool HasOwnBoundingRectangle();
string GetState_LocationString();                 // Human-readable location
```

#### Visual Properties

```csharp
uint GetState_Color();                            // Line/outline color (uint32 ARGB)
void SetState_Color(uint argColor);
uint GetState_AreaColor();                        // Fill color
void SetState_AreaColor(uint argColor);
```

#### Part Association

```csharp
int GetState_OwnerPartId();                       // Which part of multi-part component (0 = all parts)
void SetState_OwnerPartId(int argValue);
byte GetState_OwnerPartDisplayMode();             // Which display mode this belongs to
void SetState_OwnerPartDisplayMode(byte argValue);
```

#### Selection and Display State

```csharp
bool GetState_Selection();
void SetState_Selection(bool argB);
bool GetState_EnableDraw();
void SetState_EnableDraw(bool argB);
bool GetState_Disabled();
void SetState_Disabled(bool argB);
bool GetState_Dimmed();
void SetState_Dimmed(bool argB);
bool GetState_Thickened();
void SetState_Thickened(bool argB);
bool GetState_GraphicallyLocked();
void SetState_GraphicallyLocked(bool argValue);
byte GetState_SelectionMemoryFlags();
bool GetState_InSelectionMemory(int argMemoryIndex);
int GetState_UnionIndex();
void SetState_UnionIndex(int argValue);
bool GetState_InPlacementMode();
bool GetState_CompilationMasked();
```

#### Error Display

```csharp
TErrorKind GetState_ErrorKind();
uint GetState_ErrorColor();
bool GetState_DisplayError();
string GetState_ErrorString();
```

#### Transformations

```csharp
void RotateBy90(TLocation argCenter, TRotationBy90 arg);
void MoveByXY(int argX, int argY);
void MoveToXY(int argX, int argY);
void Mirror(TLocation argxis);
void ShiftPositionByXY(int argX, int argY);
```

### ISchDataGraphicalObject (Modern)

Source: `Altium.Sch.Interfaces/Altium.Sch.Interfaces.Objects/ISchDataGraphicalObject.cs`

```csharp
int GetOwnerPartId();
void SetOwnerPartId(int argValue);
byte GetOwnerPartDisplayMode();
void SetOwnerPartDisplayMode(byte argValue);
byte GetSelectionMemoryFlags();
void SetSelectionMemoryFlags(byte argValue);
int GetUnionIndex();
void SetUnionIndex(int argValue);
bool GetGraphicallyLocked();
void SetGraphicallyLocked(bool argValue);
TLocation GetLocation();
void SetLocation(TLocation argValue);
uint GetColor();
void SetColor(uint argValue);
uint GetAreaColor();
void SetAreaColor(uint argValue);
void MarkFontEntryInFontTable();
void MoveByXY(int argX, int argY);
void MoveToXY(int argX, int argY);
```

---

## Component Interface (ISchComponent)

Source: `Altium.Edp.Interfaces/SCHInterfaces/ISchComponent.cs`
GUID: `F9D2D109-2D4A-4B7E-AD59-E3B474A35BAA`
Inherits: `IParametrizedGroup : IGraphicalObject : IBasicContainer`

The most complex interface in the schematic model. Components contain pins, parameters, implementations, and support multi-part/multi-display-mode.

### Display and Part Management

```csharp
// Display modes
byte GetState_DisplayMode();                      // Current display mode index
void SetState_DisplayMode(byte argValue);
int GetState_DisplayModeCount();                  // Number of display modes
void SetState_DisplayModeCount_Check(int argValue);
void SetState_DisplayModeCount_NoCheck(int argCount);
void AddDisplayMode();
void DeleteDisplayMode(byte argMode);
string GetState_CustomDisplayModeName(byte argDisplayMode);
void SetState_CustomDisplayModeName(byte argDisplayMode, string argValue);

// Multi-part
int GetState_CurrentPartID();                     // Current active part (1-based)
void SetState_CurrentPartID(int argValue);
void SetState_CurrentPartID_NoCheck(int argValue);
int GetState_PartCount();                         // Total number of parts
void SetState_PartCount_CheckValidity(int argValue);
void SetState_PartCount_NoCheck(int argPartCount);
void AddPart();
void DeletePart(int argPartId);
string FullPartDesignator(int argPartId);         // e.g., "U1A", "U1B"
string PartIdString(int argPartId);               // Part suffix letter
bool GetState_CanIncrementCurrentPartID();
void IncrementCurrentPartID();
bool GetState_IsSubPartImplementedForDisplayMode(int argSubPartId, int argDisplayMode);
bool IsMultiPartComponent();
bool GetState_HasOnlyCurrentPartInfo();
```

### Component Identity

```csharp
TRotationBy90 GetState_Orientation();             // 0, 90, 180, 270 degrees
void SetState_Orientation(TRotationBy90 argValue);
bool GetState_IsMirrored();
void SetState_IsMirrored(bool argValue);
TComponentKind GetState_ComponentKind();           // Standard, Mechanical, Graphical, NetTie, Jumper
void SetState_ComponentKind(TComponentKind argValue);
string GetState_ComponentDescription();
void SetState_ComponentDescription(string argValue);
```

### Library References

```csharp
string GetState_LibraryPath();                    // Path to source library
void SetState_LibraryPath(string argValue);
string GetState_SourceLibraryName();              // Library file name
void SetState_SourceLibraryName(string argValue);
string GetState_LibReference();                   // Component name in library
void SetState_LibReference(string argValue);
string GetState_SymbolReference();                // Symbol reference name
void SetState_SymbolReference(string argValue);
string GetState_DesignItemId();                   // Design item ID (for managed libraries)
void SetState_DesignItemId(string argValue);
string GetState_DatabaseTableName();              // DbLib table name
void SetState_DatabaseTableName(string argValue);
bool GetState_UseLibraryName();
bool GetState_UseDBTableName();
TLibIdentifierKind GetState_LibIdentifierKind();  // How library is identified
string GetState_LibraryIdentifier();              // Computed library identifier
string GetState_SheetPartFileName();              // Sheet part file name for sheet symbols
string GetState_TargetFileName();                 // Target file for cross-references
```

### Vault/Managed Component Links

```csharp
string GetState_VaultGUID();                      // Workspace/vault GUID
void SetState_VaultGUID(string argValue);
string GetState_ItemGUID();                       // Component item GUID
void SetState_ItemGUID(string argValue);
string GetState_RevisionGUID();                   // Specific revision GUID
void SetState_RevisionGUID(string argValue);
string GetState_SymbolVaultGUID();                // Symbol's vault GUID (can differ)
void SetState_SymbolItemGUID(string argValue);
string GetState_SymbolItemGUID();
string GetState_SymbolRevisionGUID();
void SetState_SymbolRevisionGUID(string argValue);
string GetState_SaveItemGUID();                   // GUID used during save
string GetState_SaveVaultGUID();
```

### Visual Properties

```csharp
bool GetState_ShowHiddenPins();
void SetState_ShowHiddenPins(bool argValue);
bool GetState_DisplayFieldNames();
void SetState_DisplayFieldNames(bool argValue);
bool GetState_ShowHiddenFields();
void SetState_ShowHiddenFields(bool argValue);
bool GetState_DesignatorLocked();
void SetState_DesignatorLocked(bool argValue);
bool GetState_PartIdLocked();
void SetState_PartIdLocked(bool argValue);
bool GetState_PinsMoveable();
void SetState_PinsMoveable(bool argValue);
uint GetState_PinColor();
void SetState_PinColor(uint argValue);
bool GetState_OverideColors();                    // Note: typo in original ("Overide" not "Override")
void SetState_OverideColors(bool argValue);
```

### Pin Access

```csharp
int GetAllPinCount();                             // Total pins across all parts/modes
int GetState_PinsForAllParts_ForCurrentMode_Count();
int GetState_PinsForPart_ForCurrentMode_Count(int argPartId);
int GetState_PinsForPartId_ForMode_Count(int argPartId, byte argMode);
int GetState_PinsForAllParts_ForMode_Count(byte argMode);
int GetState_PinsByDesignator_ForAllModes_Count(string argPinDes);
ISchPin GetState_Pins_Pin(int argIndex);          // Get pin by index
ISchPin GetState_PinByDesignator_ForCurrentMode(string argPinDesignator);
string GetState_NewPinDesignator();               // Next available pin designator
void SetHiddenNetForHiddenPins();
```

### Designator and Comment

```csharp
IDesignator GetState_Designator();                // The designator object (R1, U1, etc.)
ISchParameter GetState_Comment();                 // The comment parameter object
void SetCommentForLoad(ISchParameter argComment);
void SetCommentToLibRefIfNecessary();
void ResetDesignator();
```

### Implementations (Footprint Links)

```csharp
int ImplementationCount();
IImplementation AddImplementation();
IImplementation AddImplementation(string argModelName, string argModelType, string argDescription);
void RemoveImplementation(IImplementation argnImplementation);
void RemoveAllImplementations();
void GetState_AllImplementations(ISafeInterfaceList argList);
IImplementation GetState_ImplementationByModelName(string argS);
IImplementation GetState_ImplementationByModelNameAndType(string argModelName, string argModelType);
IImplementation GetState_ImplementationCurrentByModelType(string argModelType);
void GetState_ImplementationsByModelType(ISafeInterfaceList argList, string argModelType);
void ValidateImplementationsCurrentState();
void RebuildCurrentImplementationsList();
void LockImplementationsIfIntegrated();
void UnlockAllImplementations();
```

### Alias System

```csharp
string GetState_AliasAsText();                    // All aliases as text
int GetState_AliasCount();
string GetState_AliasAt(int argi);
void SetState_AliasAsText(string argValue);
void SetState_AliasAt(int argi, string argValue);
void Alias_Add(string argS);
void Alias_Remove(string argS);
void Alias_Delete(int argi);
void Alias_Clear();
```

### Variant Support

```csharp
IVariantOption GetState_VariantOption();
void SetState_VariantOption(IVariantOption argValue);
ISchComponent GetState_VariantComponent();
void SetVariantComponentFromLibLink(IComponentLibraryLink argLibraryLink);
bool GetState_HasVariantComponent();
```

### Generic Component Templates

```csharp
bool IsGenericComponent();
string GetState_GenericComponentTemplateGuid();
void SetState_GenericComponentTemplateGuid(string argValue);
```

### Subcircuit Support

```csharp
bool IsSubcircuitCapable();
bool IsSubcircuitEnabled();
ISchControl GetSubcircuitControl();
int GetState_MatchingSubcircuitsCount();
```

### Transform Operations (Component-Specific)

```csharp
void RotateBy90DontMoveParams(TLocation argCenter, TRotationBy90 arg);
void RotateBy90DontMoveAutoposParams(TLocation argCenter, TRotationBy90 arg, bool argSkipComplexText);
void MirrorDontMoveParams(TLocation argxis);
void MirrorDontMoveAutoposParams(TLocation argxis, bool argSkipComplexText);
void ResetReferenceLocation();
void ResetOrientation();
void SetState_OrientationWithoutRotating(TRotationBy90 argValue);
```

### Library Operations

```csharp
bool InSheet();                                   // Is this component in a sheet?
bool InLibrary();                                 // Is this component in a library?
bool IsIntegratedComponent();                     // Integrated component (vault-managed)
void UpdateComponentFromLibrary(string argSaveDesignItemID, bool argQuiet, IUpdateParameterOptions argUpdateParameterOptions);
bool UpdatePartFromReferenceFull(ISchComponent argReferencePart, IUpdateParameterOptions argUpdateParameterOptions, ISchCommandManagerExt argCmdManager);
bool UpdatePartFromReferenceSymbol(ISchComponent argReferencePart, bool argUpdateComponentKind, IUpdateParameterOptions argUpdateParameterOptions);
bool IsDifferentFrom(ISchComponent argOriginalComponent);
void ConvertLibraryData_OldToNew();
ISchSheetSymbol CopyAsSheetSymbol();
ISchSheetSymbol ConvertToSheetSymbol(ITransformPinNameToPortName argTransformPinNameToPortName);
```

### ISchDataComponent (Modern)

Source: `Altium.Sch.Interfaces/Altium.Sch.Interfaces.Objects/ISchDataComponent.cs`
Inherits: `ISchDataParametrizedGroup, ISchDataGraphicalObject, ISchDataContainer, ISchDataObject, ISchDataDesignatorOwner, ISchDataCommentOwner, ISchDataComponentKindOwner`

Clean data interface with same properties but via `Get*/Set*` pattern:

```csharp
string KeyComponentUniqueId { get; set; }         // Property-style access
int GetAllPinCount();
byte GetDisplayMode();
void SetDisplayMode(byte argValue);
byte GetDisplayModeCount();
void SetDisplayModeCount(byte argValue);
bool GetShowHiddenPins();
bool GetDisplayFieldNames();
int GetCurrentPartID();
int GetPartCount();
TRotationBy90 GetOrientation();
bool GetDesignatorLocked();
bool GetPartIdLocked();
bool GetPinsMoveable();
uint GetPinColor();
bool GetOverideColors();
bool GetIsMirrored();
bool GetShowHiddenFields();
string GetLibraryPath();
string GetSourceLibraryName();
string GetDatabaseTableName();
bool GetUseLibraryName();
bool GetUseDBTableName();
string GetDesignItemId();
string GetLibReference();
string GetSheetPartFileName();
string GetTargetFileName();
string GetComponentDescription();
string GetVaultGUID();
string GetItemGUID();
string GetRevisionGUID();
string GetSymbolVaultGUID();
string GetSymbolItemGUID();
string GetSymbolRevisionGUID();
bool GetHasOnlyCurrentPartInfo();
ISchDataAliasList GetAliasList();
string GetFootprint();
void SetFootprint(string argValue);
ISchDataObjectList GetParameterList();
ISchDataParameter GetDummyParameter();
ISchDataParameter GetSheetFileNameParameter();
int GetFilePosition();
string GetCustomDisplayModeName(byte argDisplayMode);
void SetCustomDisplayModeName(byte argDisplayMode, string argValue);
string GetGenericComponentTemplateGuid();
void ResetReferenceLocation();
void ResetOrientation();
void FillImplementationListByModelType(string argModelType, ISchDataObjectList argList);
ISchDataImplementation FindCurrentImplementationByModelType(string argModelType);
void ValidateImplementationsCurrentState();
void SetDisplayModeWithCheck(byte argValue);
void SetHiddenNetForHiddenPins();
void AddDisplayMode();
void RebuildCurrentImplementationList();
void UpdatePrimitivesAccessibility();
bool IsSubcircuitCapable();
```

---

## Pin Interface (ISchPin)

Source: `Altium.Edp.Interfaces/SCHInterfaces/ISchPin.cs`
GUID: `6AEBE7C3-FB5A-4B1F-90A9-6E5597B98DC5`
Inherits: `IParametrizedGroup : IGraphicalObject : IBasicContainer`

### Core Pin Properties

```csharp
string GetState_Name();                           // Pin name (e.g., "VCC", "GND", "D0")
void SetState_Name(string argValue);
string GetState_Designator();                     // Pin number/designator (e.g., "1", "A3")
void SetState_Designator(string argValue);
TRotationBy90 GetState_Orientation();             // Pin direction (0=right, 1=up, 2=left, 3=down)
void SetState_Orientation(TRotationBy90 argValue);
int GetState_PinLength();                         // Length in internal units
void SetState_PinLength(int argValue);
int GetState_Width();                             // Line width
void SetState_Width(int argValue);
TLocation GetState_EndLocation();                 // End point (opposite of Location)
```

### Electrical Properties

```csharp
TPinElectrical GetState_Electrical();             // Input, Output, IO, Passive, Power, etc.
void SetState_Electrical(TPinElectrical argValue);
TStdLogicState GetState_FormalType();             // Formal type for signal integrity
void SetState_FormalType(TStdLogicState argValue);
string GetState_DefaultValue();                   // Default logic value
void SetState_DefaultValue(string argValue);
double GetState_PropagationDelay();               // Propagation delay value
void SetState_PropagationDelay(double argValue);
```

### Visibility

```csharp
bool GetState_ShowName();                         // Show pin name text
void SetState_ShowName(bool argValue);
bool GetState_ShowDesignator();                   // Show pin designator text
void SetState_ShowDesignator(bool argValue);
bool GetState_IsHidden();                         // Hidden pin (still electrically connected)
void SetState_IsHidden(bool argValue);
string GetState_HiddenNetName();                  // Net name for hidden pins (e.g., "VCC", "GND")
void SetState_HiddenNetName(string argValue);
string GetState_Description();                    // Pin description text
void SetState_Description(string argValue);
```

### IEEE Symbols

```csharp
TIeeeSymbol GetState_Symbol_Inner();              // Inner symbol (e.g., clock, Schmitt trigger)
void SetState_Symbol_Inner(TIeeeSymbol argValue);
TIeeeSymbol GetState_Symbol_Outer();              // Outer symbol (e.g., dot for active low)
void SetState_Symbol_Outer(TIeeeSymbol argValue);
TIeeeSymbol GetState_Symbol_InnerEdge();          // Inner edge symbol
void SetState_Symbol_InnerEdge(TIeeeSymbol argValue);
TIeeeSymbol GetState_Symbol_OuterEdge();          // Outer edge symbol
void SetState_Symbol_OuterEdge(TIeeeSymbol argValue);
Rt_Schematic.TSize GetState_Symbol_LineWidth();   // IEEE symbol line width
void SetState_Symbol_LineWidth(Rt_Schematic.TSize argValue);
```

### Pin Swap IDs

```csharp
string GetState_SwapIdPart();                     // Part-level swap group
void SetState_SwapIdPart(string argValue);
string GetState_SwapIdPin();                      // Pin-level swap group
void SetState_SwapIdPin(string argValue);
string GetState_SwapIdPartPin();                  // Combined part+pin swap group
void SetState_SwapIdPartPin(string argValue);
string GetState_SwapIdPartAndPartPin();           // Part and part-pin combined
void SetState_SwapIdPartAndPartPin(string argValue);
string GetState_SwapIdPair();                     // Pair swap group
void SetState_SwapIdPair(string argValue);
```

### Pin Name/Designator Custom Formatting

Each of Name and Designator has parallel formatting properties:

```csharp
// Name formatting
TPinItemMode GetState_Name_PositionMode();        // Default or Custom position
TPinItemMode GetState_Name_FontMode();            // Default or Custom font
TPinTextRotationAnchor GetState_Name_CustomPosition_RotationAnchor();
TRotationBy90 GetState_Name_CustomPosition_RotationRelative();
int GetState_Name_CustomPosition_Margin();
int GetState_Name_CustomFontID();
uint GetState_Name_CustomColor();

// Designator formatting (same pattern)
TPinItemMode GetState_Designator_PositionMode();
TPinItemMode GetState_Designator_FontMode();
TPinTextRotationAnchor GetState_Designator_CustomPosition_RotationAnchor();
TRotationBy90 GetState_Designator_CustomPosition_RotationRelative();
int GetState_Designator_CustomPosition_Margin();
int GetState_Designator_CustomFontID();
uint GetState_Designator_CustomColor();
```

### Alternate Pin Functions

```csharp
string GetState_SymbolicName();                   // Symbolic/alternate name
void SetState_SymbolicName(string argValue);
bool GetState_ShowSymbolicNameAsFunction();
void SetState_ShowSymbolicNameAsFunction(bool argValue);
bool GetState_HidePinNameAsFunction();
void SetState_HidePinNameAsFunction(bool argValue);

// Selected functions (runtime active)
int GetSelectedFunctionsCount();
string GetSelectedFunction(int argIndex);
void AddSelectedFunction(string argFunctionName);
void RemoveSelectedFunction(int argIndex);
void RemoveSelectedFunctionByName(string argFunctionName);
void ClearSelectedFunctions();
string GetSelectedFunctionsAsString();

// Defined functions (all available)
int GetDefinedFunctionsCount();
string GetDefinedFunction(int argIndex);
void AddDefinedFunction(string argFunctionName);
void RemoveDefinedFunction(int argIndex);
void RemoveDefinedFunctionByName(string argFunctionName);
void ClearDefinedFunctions();
string GetDefinedFunctionsAsString();
string GetAlternatePinFunctionsName();
```

### Pin Utility

```csharp
ISchComponent OwnerComponent();                   // The component that owns this pin
string FullDesignator();                          // Full designator including part suffix
string GetState_PadDesignator();                  // Pad designator for PCB mapping
int GetState_PinDesignatorSuperscriptFontID();
int GetState_PinPackageLength();                  // Package-level pin length
bool IsBusPin();                                  // Is this a bus pin?
bool Allow_SwapWithPin(ISchPin argPin);
void SwapWithPin(ISchPin argPin);
```

### ISchDataPin (Modern)

Source: `Altium.Sch.Interfaces/Altium.Sch.Interfaces.Objects/ISchDataPin.cs`
Inherits: `ISchDataParametrizedGroup, ISchDataGraphicalObject, ISchDataContainer, ISchDataObject`

```csharp
string ConnectedObjectUniqueId { get; set; }      // Property for connected object tracking
string SymbolicName { get; set; }
bool ShowSymbolicNameAsFunction { get; set; }
string GetName();
string GetDesignator();
TRotationBy90 GetOrientation();
TStdLogicState GetFormalType();
string GetDefaultValue();
string GetDescription();
bool GetShowName();
bool GetShowDesignator();
TPinElectrical GetElectrical();
int GetPinLength();
bool GetIsHidden();
TIeeeSymbol GetSymbolInner();
TIeeeSymbol GetSymbolOuter();
TIeeeSymbol GetSymbolInnerEdge();
TIeeeSymbol GetSymbolOuterEdge();
bool GetHidePinNameAsFunction();
ISchDataPinFunctions GetSelectedFunctions();
ISchDataPinFunctions GetDefinedFunctions();
Rt_Schematic.TSize GetSymbolLineWidth();
string GetSwapIdPin();
string GetSwapIdPartAndPartPin();
string GetSwapIdPair();
int GetPinPackageLength();
string GetHiddenNetName();
// Name custom formatting
uint GetNameCustomColor();
int GetNameCustomFontID();
int GetNameCustomPositionMargin();
TRotationBy90 GetNameCustomRotationRelative();
TPinTextRotationAnchor GetNameCustomRotationAnchor();
TPinItemMode GetNameFontMode();
TPinItemMode GetNamePositionMode();
// Designator custom formatting
uint GetDesignatorCustomColor();
int GetDesignatorCustomFontID();
int GetDesignatorCustomPositionMargin();
TRotationBy90 GetDesignatorCustomRotationRelative();
TPinTextRotationAnchor GetDesignatorCustomRotationAnchor();
TPinItemMode GetDesignatorFontMode();
TPinItemMode GetDesignatorPositionMode();
double GetPropagationDelay();
bool IsConnectedToAnyObject();
TLocation GetEndLocation();
```

---

## Document Interface (ISchDocument)

Source: `Altium.Edp.Interfaces/SCHInterfaces/ISchDocument.cs`
GUID: `442F6722-2D44-4106-9C63-6D25B424BEE5`
Inherits: `IParametrizedGroup : IGraphicalObject : IBasicContainer`

The sheet-level document container. One ISchDocument per .SchDoc file.

### Sheet Size and Style

```csharp
TSheetStyle GetState_SheetStyle();                // Standard size (A4, A3, A, B, Letter, etc.)
void SetState_SheetStyle(TSheetStyle argValue);
TSheetDocumentBorderStyle GetState_DocumentBorderStyle();
void SetState_DocumentBorderStyle(TSheetDocumentBorderStyle argValue);
string GetState_CustomSheetStyle();               // Custom style name
TSheetOrientation GetState_WorkspaceOrientation();
void SetState_WorkspaceOrientation(TSheetOrientation argValue);
bool GetState_UseCustomSheet();                   // Custom vs standard dimensions
void SetState_UseCustomSheet(bool argValue);
int GetState_CustomX();                           // Custom width (internal units)
int GetState_CustomY();                           // Custom height
int GetState_CustomXZones();                      // Number of X reference zones
int GetState_CustomYZones();                      // Number of Y reference zones
int GetState_CustomMarginWidth();                 // Custom margin width
int GetState_SheetSizeX();                        // Effective sheet width
int GetState_SheetSizeY();                        // Effective sheet height
int GetState_SheetZonesX();                       // Effective X zones
int GetState_SheetZonesY();                       // Effective Y zones
int GetState_SheetMarginWidth();                  // Effective margin
```

### Border and Title Block

```csharp
bool GetState_TitleBlockOn();
void SetState_TitleBlockOn(bool argValue);
bool GetState_ReferenceZonesOn();
void SetState_ReferenceZonesOn(bool argValue);
TSheetReferenceZoneStyle GetState_ReferenceZoneStyle();
void SetState_ReferenceZoneStyle(TSheetReferenceZoneStyle argValue);
bool GetState_BorderOn();
void SetState_BorderOn(bool argValue);
int GetState_SheetNumberSpaceSize();
```

### Grid Settings

```csharp
bool GetState_SnapGridOn();
void SetState_SnapGridOn(bool argValue);
int GetState_SnapGridSize();
void SetState_SnapGridSize(int argValue);
bool GetState_VisibleGridOn();
void SetState_VisibleGridOn(bool argValue);
int GetState_VisibleGridSize();
void SetState_VisibleGridSize(int argValue);
bool GetState_HotspotGridOn();
void SetState_HotSpotGridOn(bool argValue);
int GetState_HotspotGridSize();
void SetState_HotSpotGridSize(int argValue);
```

### Template Management

```csharp
bool GetState_ShowTemplateGraphics();
void SetState_ShowTemplateGraphics(bool argValue);
string GetState_TemplateFileName();
void SetState_TemplateFileName(string argValue);
ITemplate GetTemplate();
void MoveTemplateToBeginning();
void SetTemplateInfo(string argVaultGUID, string argItemGUID, string argRevisionGUID, string argVaultHRID, string argRevisionHRID);
string GetTemplateVaultGUID();
string GetTemplateItemGUID();
string GetTemplateRevisionGUID();
string GetTemplateVaultHRID();
string GetTemplateRevisionHRID();
```

### Document Properties

```csharp
TUnit GetState_Unit();                            // DXP or Imperial
void SetState_Unit(TUnit argValue);
TUnitSystem GetState_UnitSystem();
int GetState_SystemFont();
void SetState_SystemFont(int argValue);
int GetState_MinorVersion();                      // File format minor version
void SetState_MinorVersion(int argValue);
string GetState_LoadFormat();
string GetState_DocumentName();
void SetState_DocumentName(string argValue);
bool GetState_SuccessfullyLoaded();
bool GetState_InFileLoad();
bool GetState_ModifiedInLoad();
string GetState_ReleaseVaultGUID();               // Vault GUID for released versions
```

### Object Queries

```csharp
ISafeInterfaceList GetAllObjectsOfKind(TObjectId argObjectID); // All objects of a given type
void UnregisterAllObjectsOfKind(ISafeInterfaceList argList, TObjectId argObjectID);
void UnregisterAndFreeAllObjectsOfKind(TObjectId argObjectID);
```

### Unique ID Management (Document Level)

```csharp
string GenerateUniqueID();                        // Generate a new unique 8-char ID
bool IsIDUnique(string argUniqueId, IBasicContainer argIntf); // Check ID uniqueness
bool SupportsObject(string argUniqueId, Guid argIID, out ISch_BasicContainer argIntf);
```

### View and Navigation

```csharp
void LockViewUpdate();
void UnLockViewUpdate();
void Navigate_HighlightObjectList(ISafeInterfaceList argObjectList, THighlightMethodSet argHighlightMethods, bool argClearExisting);
void Navigate_ZoomOnList(ISafeInterfaceList argObjectList, bool argClearExisting);
void Navigate_SelectList(ISafeInterfaceList argObjectList, bool argClearExisting);
void Navigate_MaskList(ISafeInterfaceList argObjectList, bool argClearExisting);
void Zoom_Selected();
void Zoom_All();
void Zoom_Document();
void Mask_Selected();
void SwitchToEditorView();
bool IsInEditorView();
bool IsEditableInCurrentView(IGraphicalObject argG);
ISCH_GraphicalViewInterface GetGraphicalView();
```

---

## Connectivity Objects

### IWire

Source: `Altium.Edp.Interfaces/SCHInterfaces/IWire.cs`
GUID: `C2802C32-C306-4880-B27D-DB1527F9F41B`
Inherits: `IBasicPolyline : IPolygon : IGraphicalObject : IBasicContainer`

Wires are polylines with electrical connectivity. They extend the polyline interface (vertex management) with wire-specific features.

```csharp
// Wire-specific properties (beyond polyline vertices)
bool GetState_AutoWire();                         // Auto-routing enabled
void SetState_AutoWire(bool argValue);
TLocation GetState_EditingEndPoint();             // Currently editing endpoint
uint GetState_UnderlineColor();                   // Underline highlight color
void SetState_UnderlineColor(uint argValue);
void GetState_CrossOversArray(ISafeInterfaceList argList);  // Crossover points
string GetState_AssignedInterface();              // Assigned interface name
void SetState_AssignedInterface(string argValue);
string GetState_AssignedInterfaceSignal();
void SetState_AssignedInterfaceSignal(string argValue);
bool GetState_CompilationMaskedSegment();
```

### ISchPort

Source: `Altium.Edp.Interfaces/SCHInterfaces/ISchPort.cs`
GUID: `3C30254F-D913-446E-A7C6-B903B228B4D5`
Inherits: `IParametrizedGroup : IGraphicalObject : IBasicContainer`

Hierarchical ports for inter-sheet connectivity.

```csharp
string GetState_Name();                           // Port/net name
void SetState_Name(string argValue);
string GetState_HarnessType();                    // Harness type (if harness port)
void SetState_HarnessType(string argValue);
TPortArrowStyle GetState_Style();                 // Arrow style
void SetState_Style(TPortArrowStyle argValue);
TPortIO GetState_IOType();                        // Unspecified, Input, Output, Bidirectional
void SetState_IOType(TPortIO argValue);
int GetState_Alignment();                         // Text alignment
int GetState_Width();                             // Port width
int GetState_Height();                            // Port height
uint GetState_TextColor();                        // Text color
int GetState_FontId();                            // Font identifier
int GetState_BorderWidth();                       // Border thickness
bool GetState_AutoSize();                         // Auto-size to text
bool GetState_ShowNetName();                      // Show net name label
int GetState_ConnectedEnd();                      // Which end is connected (0 or 1)
```

### ISchNetLabel

Source: `Altium.Edp.Interfaces/SCHInterfaces/ISchNetLabel.cs`
GUID: `256E091C-ED2A-4ABE-8229-D0B5A9B55DCA`
Inherits: `ILabel : IGraphicalObject : IBasicContainer`

Net labels assign net names to wires. They inherit all properties from ILabel:

```csharp
// Inherited from ILabel:
string GetState_Text();                           // The net name
int GetState_FontId();                            // Font
TRotationBy90 GetState_Orientation();
int GetState_xSize();                             // Horizontal size
int GetState_ySize();                             // Vertical size
int GetState_Justification();                     // Text justification
bool GetState_IsMirrored();
string GetState_DisplayString();                  // Displayed string (may differ from text)
string GetState_Formula();                        // Formula expression
```

### ISchPowerObject

Source: `Altium.Edp.Interfaces/SCHInterfaces/ISchPowerObject.cs`
GUID: `89C6C8BB-4923-4B17-8CCC-4B1ACF56230B`
Inherits: `ILabel : IGraphicalObject : IBasicContainer`

Power ports (VCC, GND, etc.) that create global nets.

```csharp
// Power-specific properties:
bool GetState_IsCrossSheetConnector();            // Cross-sheet connector mode
void SetState_IsCrossSheetConnector(bool argValue);
TPowerObjectStyle GetState_Style();               // Visual style (circle, arrow, bar, ground variants)
void SetState_Style(TPowerObjectStyle argValue);
bool GetState_ShowNetName();                      // Show the net name
void SetState_ShowNetName(bool argValue);
// Inherits text (net name) from ILabel
```

### ISchSheetSymbol

Source: `Altium.Edp.Interfaces/SCHInterfaces/ISchSheetSymbol.cs`
GUID: `AECA2F23-82BC-4CC8-A08D-59385056FE6B`
Inherits: `IRectangularEntryContainer : IRectangularGroup : IParametrizedGroup : IGraphicalObject : IBasicContainer`

Sheet symbols represent sub-sheets in hierarchical designs.

```csharp
bool GetState_IsSolid();                          // Filled or outline
void SetState_IsSolid(bool argValue);
bool GetState_ShowHiddenFields();
void SetState_ShowHiddenFields(bool argValue);
TSheetSymbolType GetState_SymbolType();           // Symbol type
void SetState_SymbolType(TSheetSymbolType argValue);
string GetState_DesignItemId();                   // Design item ID (managed libraries)
void SetState_DesignItemId(string argValue);
string GetState_SourceLibraryName();              // Source library
string GetState_VaultGUID();
string GetState_ItemGUID();
string GetState_RevisionGUID();
string GetState_SheetFileName();                  // Referenced sheet file path
void SetState_SheetFileName(string argValue);
string GetState_SheetName();                      // Sheet name
void SetState_SheetName(string argValue);
bool GetState_IsMultichannel();                   // Multi-channel repeat block
void SetState_IsMultichannel(bool argValue);
```

---

## Iterator and Container Patterns

### ISchDataIterator

Source: `Altium.Edp.Interfaces/RT_SchDataModel/ISchDataIterator.cs`
GUID: `77A819E7-726E-478F-9B3D-6C187D4FAA0D`

The iterator is the primary mechanism for traversing schematic objects. It supports filtering by object type, part/display mode, spatial area, and custom attributes.

#### Usage Pattern

```
1. Get iterator from data model: ISchDataModel.GetDataIterator()
2. Configure filters:
   - SetDepth(TLookupDepth)              // Shallow or deep traversal
   - AddObjectIdFilter(TObjectSet)        // Filter by object type(s)
   - AddPartPrimitivesFilter(partId, displayMode)  // Filter by part/mode
   - AddAreaFilter(x1, y1, x2, y2)       // Spatial filter
   - AddFilter(TObjectAttribute, value)   // Attribute filter (bool, int, or string)
   - SetCustomFilter(ISchDataIteratorFilter)  // Custom filter callback
3. Iterate:
   - ISch_BasicContainer obj = iterator.First(container)
   - while (obj != null) { process(obj); obj = iterator.Next(); }
4. Clear: ClearFilterList()
```

#### Filter Methods

```csharp
void SetDepth(TLookupDepth argDepth);
void AddObjectIdFilter(ref TObjectSet argObjectSet);
void AddPartPrimitivesFilter(int argPartId, byte argDisplayMode);
void AddCurrentPartPrimitivesFilter();
void AddCurrentPartPrimitivesWithHiddenPinsFilter();
void AddCurrentDisplayModePrimitivesFilter();
void AddCurrentDisplayModePrimitivesHiddenParametersFilter();
void AddAreaFilter(int argX1, int argY1, int argX2, int argY2);
void ClearFilterList();
void SetCustomFilter(ISchDataIteratorFilter argFilter);
void AddFilter(TObjectAttribute argnAttribute, bool argValue);
void AddFilter(TObjectAttribute argnAttribute, int argValue);
void AddFilter(TObjectAttribute argnAttribute, string argValue);
```

#### Traversal

```csharp
ISch_BasicContainer First(ISchDataObject argContainer);  // Start iteration from container
ISch_BasicContainer Next();                               // Get next matching object
```

### ISchDataModel

Source: `Altium.Edp.Interfaces/RT_SchDataModel/ISchDataModel.cs`
GUID: `972BFF25-B343-4FC3-8077-482C8387033E`

Factory for iterators, serializers, and component info readers.

```csharp
ISchDataIterator GetDataIterator();
ISchDataLibraryIterator GetDataLibraryIterator();
ISchDataSerializer GetSerializer(TSerializerType argType, string argFileName, int argMode);
ISchDataFontManager GetFontManager();
ILibCompInfoReader GetComponentInfoReader(string argLibraryFilePath, int argMultiPartNamingMethod);
ILibCompFullInfoReader GetComponentFullInfoReader(string argLibraryFilePath, int argMultiPartNamingMethod);
void SetHelper(ISchDataModelHelper argHelper);
```

### Container Traversal via IBasicContainer

The IBasicContainer itself supports iteration through:

```csharp
void GetIteratedObjects(TIterationDepth argIterationDepth, ISafeInterfaceList argList);
int ObjectsCount();                               // Direct children count
int ObjectsCount_AllLevels();                     // Recursive count
```

### Save Ordering

Objects maintain explicit save ordering:

```csharp
int GetState_IndexInSheetForSave();               // Position in save stream
int GetState_OwnerIndexForSave();                 // Owner's save index
bool GetState_OwnerIndexForSaveAdditionalList();  // In additional list
void AddAllToListForSave(ISafeInterfaceList argOldObjectsList, ISafeInterfaceList argNewObjectsList);
void AddAllToListForSave_Additional(ISafeInterfaceList argList);
```

---

## Parameter System

Parameters are the primary mechanism for storing key-value metadata on schematic objects.

### TParameterType

Source: `Altium.Edp.Interfaces/Rt_Schematic/TParameterType.cs`

```csharp
enum TParameterType {
    eParameterType_String,    // String value
    eParameterType_Boolean,   // Boolean value
    eParameterType_Integer,   // Integer value
    eParameterType_Float      // Floating-point value
}
```

### Parameter Operations (via IBasicContainer)

```csharp
ISchParameter AddParameter();
ISchParameter AddParameter(TParameterType argParamType, string argName, string argValue);
void RemoveParameter(ISchParameter argParameter);
ISchParameter GetState_ParameterByName(string argName);
int GetState_ParameterCount();
string GetState_ParameterString();                // All parameters serialized as string
void GetState_AllParameters(ISafeInterfaceList argList);
void RemoveAllParameters();
void ResetAllSchParametersPosition();
bool AddImageParameter(string argName, string argValue, out ISch_Parameter argParameter);
```

### IParametrizedGroup

The IParametrizedGroup mixin interface (which ISchComponent, ISchPin, ISchDocument, ISchPort, etc. all inherit) adds:

```csharp
void GetState_AllParameters(ISafeInterfaceList argList);
void ResetAllSchParametersPosition();
bool Import_FromUser_Parameters();
void OffsetParameters_Default();
bool AddImageParameter(string argName, string argValue, out ISch_Parameter argParameter);
```

### Parameter String Serialization

The `GetState_ParameterString()` method returns a pipe-delimited string of all parameters. This is the format used in the binary file's record storage. Components also support:

```csharp
bool SetState_FromParameters(string argParameters);        // Deserialize from parameter string
void SetState_FromDatabaseParameters(string argParameters); // From database
void SetState_FromDesignLibraryParameters(string argParameters); // From design library
```

---

## Unique ID and Handle Management

### Unique IDs

Every schematic object has a unique identifier within its document.

```csharp
// On IBasicContainer / ISchDataObject:
string GetState_UniqueId();                       // 8-char alphanumeric ID
void SetState_UniqueId(string argS);
string GetState_UniqueIdInReuseBlock();           // ID within reuse block context
void SetState_UniqueIdInReuseBlock(string argValue);
void ResetUniqueIds();                            // Regenerate all IDs in container
void CopyUniqueIds(IBasicContainer argSource);    // Copy IDs from another object
```

### Document-Level ID Management

```csharp
// On ISchDocument:
string GenerateUniqueID();                        // Generate a new unique 8-char ID
bool IsIDUnique(string argUniqueId, IBasicContainer argIntf); // Check uniqueness
```

### Handles

Handles are a secondary identification mechanism, used for runtime object tracking.

```csharp
// On IBasicContainer:
string GetState_Handle();

// On ISchDataObject:
string GetHandle();
void SetHandle(string argValue);
bool HandleNeeded();
void CreateHandle();
void DestroyHandle();
```

### Cross-Object References via UniqueId

Pins track connectivity via unique IDs:

```csharp
// On ISchDataPin:
string ConnectedObjectUniqueId { get; set; }      // UniqueId of connected wire/junction
```

Components track key component relationships:

```csharp
// On ISchComponent:
string GetState_AssignedKeyComponent();           // UniqueId of key component
void SetState_AssignedKeyComponent(string argKeyComponentUniqueId);

// On ISchDataComponent:
string KeyComponentUniqueId { get; set; }
```

---

## Cross-Document Reference Patterns

### Hierarchical Design (Sheet Symbols -> Sheets)

Sheet symbols reference sub-sheets via file paths:

```csharp
// ISchSheetSymbol:
string GetState_SheetFileName();                  // Path to referenced .SchDoc
string GetState_SheetName();                      // Logical name
```

Sheet entries on sheet symbols are matched to ports on the sub-sheet by name:

```csharp
// ISchSheetEntry (TObjectId: eSheetEntry):
// Contained within ISchSheetSymbol, matched to ISchPort by name
```

### Library References

Components reference their library source via multiple identifiers:

```
LibraryPath       -> Full path to .SchLib or .IntLib
SourceLibraryName -> Library file name
LibReference      -> Component name within library
DesignItemId      -> Managed component identifier
DatabaseTableName -> DbLib table name
```

### Vault/Managed Component Links

Three-tier GUID system for workspace-managed components:

```
VaultGUID         -> Workspace/vault identifier
ItemGUID          -> Component item within vault
RevisionGUID      -> Specific revision of the item
```

Symbols can have separate vault links (when symbol comes from different vault item):

```
SymbolVaultGUID   -> Symbol's workspace
SymbolItemGUID    -> Symbol item
SymbolRevisionGUID -> Symbol revision
```

### Implementation Links (Footprints, Models)

Components link to PCB footprints and other models through IImplementation objects:

```csharp
// On ISchComponent:
IImplementation AddImplementation(string argModelName, string argModelType, string argDescription);
IImplementation GetState_ImplementationCurrentByModelType(string argModelType);
// argModelType is typically "PCBLIB" for footprints, "SIM" for simulation models
```

### Template Links

Documents reference templates via vault GUIDs:

```csharp
// On ISchDocument:
string GetState_TemplateFileName();
string GetTemplateVaultGUID();
string GetTemplateItemGUID();
string GetTemplateRevisionGUID();
```

---

## Key Enumerations

### TPinElectrical

Source: `Altium.Edp.Interfaces/RT_Workspace/TPinElectrical.cs`

```csharp
enum TPinElectrical {
    eElectricInput,           // 0 - Input pin
    eElectricIO,              // 1 - Bidirectional
    eElectricOutput,          // 2 - Output pin
    eElectricOpenCollector,   // 3 - Open collector
    eElectricPassive,         // 4 - Passive (resistor, capacitor, etc.)
    eElectricHiZ,             // 5 - High impedance
    eElectricOpenEmitter,     // 6 - Open emitter
    eElectricPower            // 7 - Power pin
}
```

### TComponentKind

Source: `Altium.Edp.Interfaces/RT_Workspace/TComponentKind.cs`

```csharp
enum TComponentKind {
    eComponentKind_Standard,       // 0 - Standard component (in BOM)
    eComponentKind_Mechanical,     // 1 - Mechanical component
    eComponentKind_Graphical,      // 2 - Graphical only (no BOM, no netlist)
    eComponentKind_NetTie_BOM,     // 3 - Net tie (in BOM)
    eComponentKind_NetTie_NoBOM,   // 4 - Net tie (not in BOM)
    eComponentKind_Standard_NoBOM, // 5 - Standard but excluded from BOM
    eComponentKind_Jumper          // 6 - Jumper component
}
```

### TPortIO

Source: `Altium.Edp.Interfaces/Rt_Schematic/TPortIO.cs`

```csharp
enum TPortIO {
    ePortUnspecified,  // 0
    ePortOutput,       // 1
    ePortInput,        // 2
    ePortBidirectional // 3
}
```

### TPowerObjectStyle

Source: `Altium.Edp.Interfaces/Rt_Schematic/TPowerObjectStyle.cs`

```csharp
enum TPowerObjectStyle {
    ePowerCircle,           // 0
    ePowerArrow,            // 1
    ePowerBar,              // 2
    ePowerWave,             // 3
    ePowerGndPower,         // 4 - Power ground symbol
    ePowerGndSignal,        // 5 - Signal ground symbol
    ePowerGndEarth,         // 6 - Earth ground symbol
    eGOSTPowerArrow,        // 7 - GOST arrow
    eGOSTPowerGndPower,     // 8 - GOST power ground
    eGOSTPowerGndEarth,     // 9 - GOST earth ground
    eGOSTPowerBar           // 10 - GOST bar
}
```

### TIeeeSymbol

Source: `Altium.Edp.Interfaces/Rt_Schematic/TIeeeSymbol.cs`

```csharp
enum TIeeeSymbol {
    eNoSymbol,                    // 0
    eDot,                         // 1 - Active low (dot)
    eRightLeftSignalFlow,         // 2
    eClock,                       // 3
    eActiveLowInput,              // 4
    eAnalogSignalIn,              // 5
    eNotLogicConnection,          // 6
    eShiftLeft,                   // 7
    ePostponedOutput,             // 8
    eOpenCollector,               // 9
    eHiz,                         // 10 - High impedance
    eHighCurrent,                 // 11
    ePulse,                       // 12
    eSchmitt,                     // 13 - Schmitt trigger
    eOpenCollectorPullUp,         // 14
    eOpenEmitter,                 // 15
    eOpenEmitterPullUp,           // 16
    eDigitalSignalIn,             // 17
    eShiftRight,                  // 18
    eLeftRightSignalFlow,         // 19
    eBidirectionalSignalFlow,     // 20
    eActiveLowOutput,             // 21
    eGOSTOutputLow,               // 22
    eGOSTOutputHighZLow,          // 23
    eGOSTNewLeft,                 // 24
    eGOSTNewRight,                // 25
    eGOSTConnectPoint,            // 26
    eGOSTLogicInput,              // 27
    eGOSTLogicOutput,             // 28
    eGOSTNotConnected,            // 29
    eGOSTOpenCollector,           // 30
    eGOSTOpenCollectorPullUp,     // 31
    eGOSTOpenEmitter,             // 32
    eGOSTOpenEmitterPullUp,       // 33
    eGOSTHiz,                     // 34
    eGOSTPulse                    // 35
}
```

### TPortArrowStyle

Source: `Altium.Edp.Interfaces/Rt_Schematic/TPortArrowStyle.cs`

```csharp
enum TPortArrowStyle {
    ePortNone,            // 0
    ePortLeft,            // 1
    ePortRight,           // 2
    ePortLeftRight,       // 3
    ePortNoneVertical,    // 4
    ePortTop,             // 5
    ePortBottom,          // 6
    ePortTopBottom        // 7
}
```

### TSheetStyle

Source: `Altium.Edp.Interfaces/Rt_Schematic/TSheetStyle.cs`

```csharp
enum TSheetStyle {
    eSheetA4,       // 0
    eSheetA3,       // 1
    eSheetA2,       // 2
    eSheetA1,       // 3
    eSheetA0,       // 4
    eSheetA,        // 5
    eSheetB,        // 6
    eSheetC,        // 7
    eSheetD,        // 8
    eSheetE,        // 9
    eSheetLetter,   // 10
    eSheetLegal,    // 11
    eSheetTabloid,  // 12
    eSheetOrcadA,   // 13
    eSheetOrcadB,   // 14
    eSheetOrcadC,   // 15
    eSheetOrcadD,   // 16
    eSheetOrcadE    // 17
}
```

### TRotationBy90

Used for pin/component orientation:

```
0 = 0 degrees (right/east)
1 = 90 degrees (up/north)
2 = 180 degrees (left/west)
3 = 270 degrees (down/south)
```

---

## Modern Data Model Interfaces

The newer interfaces in `Altium.Sch.Interfaces.Objects` provide a cleaner API surface without COM baggage. They follow a consistent pattern:

### Interface Naming

| Legacy (SCHInterfaces) | Modern (Altium.Sch.Interfaces.Objects) |
|------------------------|----------------------------------------|
| IBasicContainer | ISchDataObject + ISchDataContainer |
| IGraphicalObject | ISchDataGraphicalObject |
| ISchComponent | ISchDataComponent |
| ISchPin | ISchDataPin |
| IParametrizedGroup | ISchDataParametrizedGroup |

### Property Access Pattern

| Legacy | Modern |
|--------|--------|
| `GetState_Name()` | `GetName()` |
| `SetState_Name(value)` | `SetName(value)` |
| `GetState_Orientation()` | `GetOrientation()` |
| `string GetState_UniqueId()` | `string GetUniqueId()` |

Some properties in the modern model use C# property syntax:

```csharp
string ConnectedObjectUniqueId { get; set; }
string KeyComponentUniqueId { get; set; }
string SymbolicName { get; set; }
```

### Additional Modern Interfaces

| Interface | Purpose |
|-----------|---------|
| `ISchDataDesignatorOwner` | Mixin for objects that own a designator |
| `ISchDataCommentOwner` | Mixin for objects that own a comment |
| `ISchDataComponentKindOwner` | Mixin for objects with TComponentKind |
| `ISchDataPinFunctions` | Pin function list management |
| `ISchDataAliasList` | Component alias list |
| `ISchDataImplementation` | Implementation (model) link |
| `ISchDataObjectList` | Generic object list |
| `ISchDataParameter` | Parameter data |

---

## Implications for altium-format Crate

### Record Type Mapping

The TObjectId enum directly maps to the record type byte in the binary SchDoc/SchLib format. Our parser should support all values 0-120.

### Property Coverage

Each `GetState_*` / `SetState_*` pair represents a property that is serialized in the binary file's key-value parameter string. The parameter names in the file typically match the property names (after removing the `GetState_`/`SetState_` prefix).

### Multi-Part Components

Components support:
- `PartCount` parts (1-based indexing)
- `DisplayModeCount` display modes (0-based indexing)
- Each contained primitive has `OwnerPartId` (0 = all parts) and `OwnerPartDisplayMode`
- Pin counts are available per-part-per-mode

### Container Hierarchy

The containment model is:
```
ISchDocument (eSheet)
  +-- ISchComponent (eSchComponent)
  |     +-- ISchPin (ePin)
  |     +-- ISchParameter (eParameter)
  |     +-- IDesignator (eDesignator)
  |     +-- IImplementation (eImplementation)
  |     +-- Drawing primitives (eRectangle, eLine, eArc, etc.)
  +-- IWire (eWire)
  +-- ISchNetLabel (eNetLabel)
  +-- ISchPort (ePort)
  +-- ISchPowerObject (ePowerObject)
  +-- ISchSheetSymbol (eSheetSymbol)
  |     +-- ISheetEntry (eSheetEntry)
  |     +-- ISchParameter (eParameter)
  +-- ISchParameter (eParameter) -- document-level
  +-- Drawing primitives
  +-- ITemplate (eTemplate)
```

### Color Format

Colors are stored as `uint` (uint32). Based on COM convention and Delphi heritage, these are likely BGR format: `0x00BBGGRR`.

### Coordinate System

All coordinates use internal units (1 internal unit = 10 nanometers = 0.01 mil). TLocation is a struct with X and Y integer fields.

### Font References

Objects reference fonts by integer ID (`FontId`). The document maintains a font table accessed via `ISchDataFontManager`. Font entries must be marked during save via `MarkFontEntryInFontTable()`.

### Save Stream Ordering

Objects maintain `IndexInSheetForSave` and `OwnerIndexForSave` values that define the serialization order. The save process collects objects via `AddAllToListForSave` and `AddAllToListForSave_Additional`. The owner index creates the parent-child relationship in the flat record stream.

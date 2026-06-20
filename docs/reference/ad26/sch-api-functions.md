> **AD26 source snapshot; not current implementation status.** Validate against current Rust code before use.

# SchAPI Functions - Delphi DLL Analysis

Comprehensive analysis of all SchAPI_* exported functions found in `AdvSch.dll` (Altium Designer 26).

**Source:** `AdvSch.dll` - 126,386 functions total, 135 SchAPI exports
**No SchAPI functions found in:** `Altium.Sch.DataModel.dll`, nor any `DxpApi_*` pattern functions

---

## Table of All Discovered Functions

### Document & Window Management

| Function | Address | Description |
|----------|---------|-------------|
| `SchAPI_GetCurrentLibraryHandle` | `021dc200` | Gets handle to the currently active schematic library |
| `SchAPI_PartAvailableInLibrary` | `021dc360` | Checks if a part exists in a library |
| `SchAPI_GetCurrentEditorWindowHandle` | `021dc420` | Gets handle to the current editor window |
| `SchAPI_GetCurrentDocumentHandle` | `021dc490` | Gets handle to the active schematic document |
| `SchAPI_GetDocumentHandleFromFileName` | `021dc600` | Resolves a file name to a document handle |
| `SchAPI_GetCurrentDocumentName` | `021dc7b0` | Gets the name of the current document |
| `SchAPI_GetDocumentCountInProject` | `021dcdf0` | Returns count of schematic documents in project |
| `SchAPI_GetDocumentHandleByIndex` | `021dce00` | Gets a document handle by index |
| `SchAPI_GetOpenedDocumentsCount` | `021dce20` | Returns count of currently opened documents |
| `SchAPI_GetOpenedDocumentDetails` | `021dcea0` | Gets details for an opened document |
| `SchAPI_QueryDocumentName` | `021ddd80` | Queries the name of a document by handle |
| `SchAPI_GetWindowHandleFromSheetHandle` | `021e4dd0` | Converts sheet handle to window handle |
| `SchApi_GetSheetHandleFromServerWindowHandle` | `021e4f00` | Converts server window handle to sheet handle |

### Object Creation & Destruction

| Function | Address | Description |
|----------|---------|-------------|
| `SchAPI_CreateObject` | `021dceb0` | Creates a schematic object and adds it to a container |
| `SchAPI_CreateObjectEx` | `021dcf40` | Creates a schematic object (extended, without container) |
| `SchAPI_DestroyObject` | `021dcf60` | Destroys a schematic object, removing from container if needed |
| `SchAPI_AddObjectToContainer` | `021dd040` | Adds an existing object to a container (sheet/component) |
| `SchAPI_ReplicatePartItem` | `021dda20` | Replicates (clones) a part item |
| `SchAPI_DestroyPartItem` | `021dda40` | Destroys a part item |

### Standard Iterators

| Function | Address | Description |
|----------|---------|-------------|
| `SchAPI_CreateIterator` | `021dd250` | Creates a filtered iterator over objects in a container |
| `SchAPI_CreateSimpleIterator` | `021dd2d0` | Creates a simple iterator (no recursive children) |
| `SchAPI_DestroyIterator` | `021dd360` | Destroys an iterator (frees memory) |
| `SchAPI_GetNextObject` | `021dd380` | Gets next object from iterator |
| `SchAPI_GetFirstObject` | `021dd420` | Gets first object from iterator |
| `SchAPI_GetObjectIdFromObjectHandle` | `021dd4c0` | Gets object type ID from an object handle |

### Spatial Iterators

| Function | Address | Description |
|----------|---------|-------------|
| `SchAPI_CreateSpatialIterator` | `021dd510` | Creates a spatial iterator (region-based, multi-type filter) |
| `SchAPI_DestroySpatialIterator` | `021dd620` | Destroys a spatial iterator |
| `SchAPI_GetNextSpatialObject` | `021dd640` | Gets next spatial object (also returns object type) |
| `SchAPI_GetFirstSpatialObject` | `021dd760` | Gets first spatial object (also returns object type) |

### Group Iterators

| Function | Address | Description |
|----------|---------|-------------|
| `SchAPI_CreateGroupIterator` | `021dd880` | Creates an iterator for grouped objects |
| `SchAPI_CreateGroupIteratorIncludeAll` | `021dd890` | Creates group iterator including all groups |
| `SchAPI_CreateSimpleGroupIterator` | `021dd8a0` | Creates a simple group iterator |
| `SchAPI_DestroyGroupIterator` | `021dd8b0` | Destroys a group iterator |
| `SchAPI_GetNextGroupObject` | `021dd8d0` | Gets next object in group |
| `SchAPI_GetFirstGroupObject` | `021dd900` | Gets first object in group |

### Library Component Iteration

| Function | Address | Description |
|----------|---------|-------------|
| `SchAPI_GetNextLibraryComponent` | `021dd930` | Gets next component in library iteration |
| `SchAPI_GetFirstLibraryComponent` | `021dd950` | Gets first component in library iteration |
| `SchAPI_GetLibraryComponentForPart` | `021dd970` | Gets library component associated with a placed part |
| `SchAPI_GetSheetEntryContainer` | `021dd500` | Gets the container (sheet symbol) of a sheet entry |

### Vertex Operations

| Function | Address | Description |
|----------|---------|-------------|
| `SchAPI_QueryVertexCount` | `021dda70` | Queries vertex count for polylines/polygons |
| `SchAPI_QueryVertexAt` | `021ddb60` | Queries vertex at a specific index |

### Commands & Events

| Function | Address | Description |
|----------|---------|-------------|
| `SchAPI_SendCommand` | `021ddce0` | Sends a DXP command string for execution |
| `SchAPI_SendEventMessage` | `021e56a0` | Sends an event message to the schematic system |
| `SchAPI_ProcessGlobalPrimitive` | `021e1370` | Processes a global primitive operation |

### User Interaction / Import

| Function | Address | Description |
|----------|---------|-------------|
| `SchAPI_ImportFromUser` | `021dded0` | Shows UI for user to import data |
| `SchAPI_ImportFromUser_SystemOptions` | `021e5030` | Shows UI for system options import |
| `SchApi_RunColorDialog` | `021e0e40` | Opens color picker dialog |
| `SchApi_RunFontDialog` | `021e10e0` | Opens font selection dialog |
| `SchAPI_RunClientProcessListDialog` | `021e13a0` | Opens client process list dialog |
| `SchAPI_ChooseRectangleByCorners` | `021e42f0` | Interactive rectangle selection |
| `SchAPI_ChooseLocation` | `021e4570` | Interactive location selection |

### Rendering & Display

| Function | Address | Description |
|----------|---------|-------------|
| `SchAPI_RedrawSheetToDC` | `021dc920` | Redraws a sheet to a device context |
| `SchAPI_RedrawSheet` | `021dca30` | Redraws a sheet |
| `SchAPI_RedrawObject` | `021dcb10` | Redraws a single object |
| `SchAPI_GetBoundingRect` | `021dcb40` | Gets bounding rectangle for objects |
| `SchAPI_GetBoundingRectangleForOrcad` | `021dcc90` | Gets bounding rect (OrCAD compatibility) |
| `SchAPI_GetObjectBoundingRectangle` | `021e4760` | Gets bounding rectangle for a specific object |
| `SchAPI_CreatePainter` | `021e5410` | Creates a painter for rendering |
| `SchAPI_DrawComponent` | `021e5450` | Draws a component |
| `SchAPI_DrawComponentByHandle` | `021e5580` | Draws a component by its handle |
| `SchAPI_LockViewUpdate` | `021e5740` | Locks view updates (batch modifications) |
| `SchAPI_UnlockViewUpdate` | `021e5830` | Unlocks view updates |
| `SchAPI_JumpToLocation` | `021e11c0` | Scrolls viewport to a location |
| `SchAPI_GetObjectAtCursor` | `021e4a40` | Gets the object at the current cursor position |
| `SchAPI_GetObjectWithFocus` | `021e4180` | Gets the currently focused object |
| `SchAPI_GetPalette` | `021e1390` | Gets the current color palette |

### Preferences & Options

| Function | Address | Description |
|----------|---------|-------------|
| `SchAPI_QuerySchematicPreferences` | `021ddf80` | Queries global schematic preferences |
| `SchAPI_QuerySystemOptions` | `021dedb0` | Queries system-wide options |
| `SchAPI_QueryDocumentOptions` | `021dfbe0` | Queries/sets document-level options (grid, snap, colors, etc.) |
| `SchAPI_QuerySchLibOptions` | `021e05f0` | Queries schematic library options |

### Font Management

| Function | Address | Description |
|----------|---------|-------------|
| `SchAPI_GetFontSpecification` | `021e3e10` | Gets font specification from font ID |
| `SchAPI_GetFontID` | `021e3ff0` | Gets font ID from font specification |

### Variables

| Function | Address | Description |
|----------|---------|-------------|
| `SchAPI_QueryVariable` | `021e1730` | Queries/sets system variables (grid size, snap size, etc.) |
| `SchAPI_DefaultGroundPowerObjectName` | `021e15e0` | Gets default name for power/ground objects |

### Library Component Management

| Function | Address | Description |
|----------|---------|-------------|
| `SchAPI_GetLibraryComponentHandle` | `021e2f40` | Gets handle for a library component by name |
| `SchAPI_LoadComponentFromLibrary` | `021e3030` | Loads a component from a library file |
| `SchAPI_DestroyLibraryComponentObject` | `021e31e0` | Destroys a loaded library component |
| `SchAPI_GetLibraryComponentGroupNameCount` | `021e3210` | Gets count of group names in a component |
| `SchAPI_GetLibraryComponentGroupNameAt` | `021e3220` | Gets group name at index |
| `SchAPI_AddLibraryComponentGroupName` | `021e3320` | Adds a group name to a component |
| `SchAPI_RemoveLibraryComponentGroupName` | `021e3430` | Removes a group name from a component |
| `SchAPI_GetLibraryComponentAliasCount` | `021e3440` | Gets count of aliases for a component |
| `SchAPI_GetLibraryComponentAliasNameAt` | `021e3480` | Gets alias name at index |
| `SchAPI_AddLibraryComponentAliasName` | `021e35b0` | Adds an alias name |
| `SchAPI_RemoveLibraryComponentAliasName` | `021e3690` | Removes an alias name |
| `SchAPI_ClearLibraryComponentAliasNames` | `021e3770` | Clears all alias names |
| `SchAPI_GetLibraryPartCount` | `021e37a0` | Gets count of parts in a library component |
| `SchAPI_GetLibraryPartContainer` | `021e37e0` | Gets container for a library part |
| `SchAPI_GetCurrentLibraryComponent` | `021e37f0` | Gets the currently active library component |
| `SchApi_QueryComponentLibraryInfo` | `021e38c0` | Queries library info for a component |
| `SchAPI_QueryLibraryComponent` | `021e38d0` | Queries/modifies library component (name, description) |
| `SchAPI_IsCurrentLibraryComponent` | `021e52b0` | Checks if a component is the currently active one |
| `SchAPI_ResetPartTextFieldsLocation` | `021e51e0` | Resets text field positions on a part |

### Misc

| Function | Address | Description |
|----------|---------|-------------|
| `SchApi_GetCopyTemplatePointer` | `021e0e30` | Gets pointer to copy template |

---

### Query Functions - Primitive Properties (SchAPI_Query*)

These functions follow a dual-mode pattern controlled by `param_1`:
- **Read mode** (`param_1 == 0x01`): Reads properties from the object into output parameters
- **Write mode** (`param_1 == 0x00` or `0x02`): Writes properties from input parameters to the object

| Function | Address | Primitive Type |
|----------|---------|---------------|
| `SchAPI_QueryPrimitive` | `021e5940` | Base primitive (type, location, color, locked) |
| `SchAPI_QueryText` | `021e5ba0` | Text string |
| `SchAPI_QueryArc` | `021e6110` | Arc |
| `SchAPI_QueryBezier` | `021e6560` | Bezier curve |
| `SchAPI_QueryBus` | `021e67d0` | Bus |
| `SchAPI_QueryBusEntry` | `021e6a40` | Bus entry |
| `SchAPI_QueryEllipse` | `021e6e20` | Ellipse |
| `SchAPI_QueryEllipticalArc` | `021e72c0` | Elliptical arc |
| `SchAPI_QueryErrorMarker` | `021e7760` | Error marker |
| `SchAPI_QueryImageEx` | `021e7bb0` | Image (extended) |
| `SchAPI_QueryImage` | `021e7d00` | Image |
| `SchAPI_QueryJunction` | `021e82b0` | Junction |
| `SchAPI_QueryLabel` | `021e8650` | Label (net label text) |
| `SchAPI_QueryTextFrame` | `021e8b70` | Text frame |
| `SchAPI_QueryLayoutDirective` | `021e9320` | Layout directive |
| `SchAPI_QueryLine` | `021e9330` | Line |
| `SchAPI_QueryNetLabel` | `021e9770` | Net label |
| `SchAPI_QueryNoERC` | `021e9c90` | No-ERC marker |
| `SchAPI_QuerySchPart` | `021e9f80` | Component (SchPart) |
| `SchAPI_QuerySchPartUniqueId` | `021eb1a0` | Component unique ID |
| `SchAPI_QueryPie` | `021eb350` | Pie shape |
| `SchAPI_QueryPin` | `021eb850` | Pin |
| `SchAPI_QueryPolygon` | `021ec0d0` | Polygon |
| `SchAPI_QueryPolyline` | `021ec3d0` | Polyline |
| `SchAPI_QueryPort` | `021ec690` | Port |
| `SchAPI_QueryPowerObject` | `021eccf0` | Power object (VCC/GND etc) |
| `SchAPI_QueryProcessContainer` | `021ed1f0` | Process container |
| `SchAPI_ProcessContainer_Execute` | `021ed200` | Execute a process container |
| `SchAPI_ProcessContainer_Configure` | `021ed210` | Configure a process container |
| `SchAPI_ProcessContainer_SetDefaults` | `021ed220` | Set process container defaults |
| `SchAPI_QueryRectangle` | `021ed230` | Rectangle |
| `SchAPI_QueryRoundRectangle` | `021edc00` | Round rectangle |
| `SchAPI_QuerySheetSymbol` | `021edc00` | Sheet symbol |
| `SchAPI_QuerySheetEntry` | `021ee220` | Sheet entry |
| `SchAPI_QuerySheetEntryReferenceZone` | `021ee850` | Sheet entry reference zone |
| `SchAPI_QuerySimProbe` | `021eea20` | Simulation probe |
| `SchAPI_QuerySimStimulus` | `021eea30` | Simulation stimulus |
| `SchAPI_QuerySimVector` | `021eea40` | Simulation vector |
| `SchAPI_QuerySymbol` | `021eea50` | Symbol |
| `SchAPI_QueryWire` | `021eee90` | Wire |

### Extended Query Functions (SchAPIQuery_*)

These provide richer property access than the basic SchAPI_Query* functions, with more parameters.

| Function | Address | Description |
|----------|---------|-------------|
| `SchAPIQuery_WorkspaceObject` | `021ef130` | Base workspace object properties |
| `SchAPIQuery_GrafObject` | `021ef130` | Graphical object properties |
| `SchApiQuery_SchComponent` | `021ef430` | Full component query (28 parameters) |
| `SchAPIQuery_ComponentDesignatorLocks` | `021f0720` | Component designator lock state |
| `SchAPIQuery_ComponentImplementationsCount` | `021f0880` | Count of implementations on a component |
| `SchAPIQuery_ComponentImplementationAt` | `021f0950` | Get implementation at index |
| `SchAPIQuery_Pin` | `021f0970` | Extended pin query (28 parameters) |
| `SchAPIQuery_Port` | `021f1a00` | Extended port query |
| `SchAPIQuery_PortCrossReference` | `021f1f70` | Port cross-reference info |
| `SchAPIQuery_SheetSymbol` | `021f2190` | Extended sheet symbol query |
| `SchAPIQuery_SheetSymbolEx` | `021f2640` | Even more extended sheet symbol |
| `SchAPIQuery_Label` | `021f2910` | Extended label query |
| `SchApiQuery_SchParameter` | `021f30d0` | Parameter query (20 parameters) |
| `SchApiQuery_SchParameterCalculatedValue` | `021f3d70` | Calculated parameter value |
| `SchApiQuery_UniqueId` | `021f3eb0` | Unique ID query for any object type |
| `SchApiQuery_SchParameterSet` | `021f4160` | Parameter set query |
| `SchApiQuery_Implementation` | `021f4590` | Implementation details (footprint/model links) |
| `SchApiQuery_ImplementationDatafileCount` | `021f4c60` | Count of datafiles in an implementation |
| `SchApiQuery_ImplementationDatafile` | `021f4db0` | Datafile details |
| `SchApiQuery_EditImplementationMap` | `021f52b0` | Edit the implementation mapping |
| `SchApiQuery_AllowSwapWithPin` | `021f5410` | Check if pin swap is allowed |
| `SchApiQuery_SwapWithPin` | `021f5520` | Perform a pin swap |
| `SchApiQuery_AllowSwapToPart` | `021f56e0` | Check if part swap is allowed |
| `SchApiQuery_SwapToPart` | `021f57a0` | Perform a part swap |
| `SchApiQuery_AddObjectToContainer` | `021f5840` | Add object to container (query API version) |
| `SchApiQuery_DeleteObjectFromContainer` | `021f58b0` | Delete object from container |
| `SchApiQuery_AddImplementationToComponent` | `021f59b0` | Add implementation link |
| `SchApiQuery_DeleteImplementationFromComponent` | `021f5aa0` | Remove implementation link |
| `SchApiQuery_DestroyImplementation` | `021f5c00` | Destroy an implementation object |
| `SchApiQuery_PlaceSchComponent` | `021f5e70` | Place a component on the schematic |
| `SchApiQuery_ParameterHandleByName` | `021f5e70` | Get parameter handle by name |

---

## Detailed Decompiled Code Analysis

### Key Architecture Patterns

#### 1. Dual-Mode Query Pattern

All query functions follow the same bidirectional pattern controlled by the first `char param_1` parameter:

```
param_1 == 0x00: WRITE mode (set properties on object)
param_1 == 0x01: READ mode (get properties from object)
param_1 == 0x02: WRITE mode (alternative write, same as 0x00)
```

This is how the Altium scripting API implements both get/set through a single function.

#### 2. Interface Pointer / Vtable Pattern

All object access uses COM-style vtable dispatch:
```c
(**(code **)(*object + OFFSET))(object, args...)
```

Each offset corresponds to a different method on the object's interface. Key common vtable offsets observed:

| Offset | Purpose | Notes |
|--------|---------|-------|
| `0x48` | SetName / SetText | Sets the primary text/name property |
| `0x78` | SetUniqueId | Sets unique identifier |
| `0x90` | GetObjectId | Returns the object type ID enum |
| `0x110` | GetName / GetText | Gets primary text content |
| `0x160` | GetUniqueId | Gets unique identifier |
| `0x1b8` | AddChild | Adds child object to container |
| `0x1c0` | RemoveChild | Removes child from container |
| `0x238` | ApplyChanges / Commit | Commits modifications to document |
| `0x2f8` | Destroy (standalone) | Destroy without container |
| `0x328` | GetLocation | Gets X,Y coordinates |
| `0x330` | GetColor | Gets line/fill color |
| `0x338` | GetAreaColor | Gets area/fill color |
| `0x340` | GetLocked | Gets locked/hidden state |
| `0x3f0` | SetColor | Sets line/fill color |
| `0x3f8` | SetAreaColor | Sets area/fill color |
| `0x400` | SetLocked | Sets locked state |
| `0x4a8` | GraphicallyInvalidate | Marks object for redraw |
| `0x4f0` | MoveBy (dx, dy) | Moves object by delta |
| `0x500` | MoveTo (point) | Moves object to absolute position |
| `0x5d8` | Get/Set various geometry | Width, radius, etc. |
| `0x5e0` | Get/Set secondary geometry | |
| `0x608` | Get/Set XSize / PortIOType | |
| `0x610` | Get/Set YSize / LineWidth | |
| `0x618` | Get/Set Symbol type | |
| `0x620` | Get/Set Orientation | |
| `0x628` | Get/Set SubPartID / Style | |
| `0x630` | Get/Set Alignment | |
| `0x638` | Get/Set Font ID / rotation | |
| `0x640` | Get/Set electrical type | |
| `0x648` | Get/Set visibility | |
| `0x650` | Get/Set name-visible | |
| `0x658` | Get/Set description-visible | |
| `0x660` | Get/Set misc bool | |
| `0x668` | Get/Set ShowNetName | |
| `0x670` | Get/Set mirror | |
| `0x678` | Get/Set designator-visible | |
| `0x680` | Get/Set DesignatorFontId | |
| `0x688` | Get/Set CommentFontId | |
| `0x690` | Get/Set port name | |
| `0x698` | Get/Set pin-related | |
| `0x6b0` | Get/Set pin name | |
| `0x6b8` | Get/Set various | Library reference, etc. |
| `0x6c0` | Get/Set various | |
| `0x6c8` | Get/Set sheet entry style | |
| `0x6d0` | Get/Set component name | |
| `0x6d8` | Get/Set sheet symbol filename | |
| `0x6e0` | Get/Set sheet symbol child filename | |
| `0x6e8` | Get/Set misc | |
| `0x6f0` | Get container handle (sheet entry) | |
| `0x750` | Get SheetStyle | |
| `0x758` | Get/Set PartCount | |
| `0x760` | Get/Set ShowHiddenPins | |
| `0x768` | Get grid spacing | |
| `0x770` | Get/Set PartIdLocked | |
| `0x778` | Get/Set SubPartCount | |
| `0x780` | Get/Set Orientation (SchPart) | |
| `0x790` | Get/Set snap grid | |
| `0x7a0` | Get/Set misc grid/vis | |
| `0x810` | Get/Set LibraryReference | |
| `0x828` | Get/Set DesignItemId | |
| `0x830` | Get/Set ElectricalGridEnabled | |
| `0x858` | Get UndoManager | |
| `0x870` | GetParameterByName | Takes name string and returns param handle |
| `0x878` | GetDesignator (child param) | Returns the designator parameter |

#### 3. Object Type ID Mapping

`SchAPI_GetObjectIdFromObjectHandle` and spatial iterator results return an object type byte. The function `FUN_021dbf70` converts from internal Delphi enum to SchAPI enum values. The return value `0x3f` means "unknown/invalid". Some observed ID mappings based on the type-check constants in `SchAPI_AddObjectToContainer`:

| Internal ID | Meaning |
|------------|---------|
| `0x20` | Sheet/Document |
| `0x21` | Sheet (alternative) |
| `0x28` | SheetEntry |
| `0x29` | SheetSymbol |

The mapping function `FUN_021db920` converts API object type IDs to internal filter set IDs for iterators.

#### 4. String Handling

Strings use Delphi's `WideString` (UTF-16) via helper functions:
- `FUN_00452710` - Creates/copies a WideString (WStr -> WStr)
- `FUN_00416c90` - Converts WideString to interface string
- `FUN_004522e0` - Assigns string to output parameter
- `FUN_00416c60` - Converts interface string to WideString
- `FUN_00414c50` - Allocates/initializes string buffer
- `FUN_00414bb0` - Frees WideString

#### 5. Reference Counting / Memory Management

- `FUN_0041b530` - Release/clear interface pointer (sets to nil, decrements refcount)
- `FUN_0041b570` - Assign interface pointer (increments refcount, copies)
- `FUN_0041b5c0` - QueryInterface-like: gets interface from object with GUID check
- `FUN_0041b8c0` - OleCheck/SafeCall result handler
- `FUN_00411570` - Free object memory (TObject.Free)

#### 6. Undo/Transaction Management

Write operations wrap modifications in undo transactions:
```c
// Begin transaction
FUN_021c6260(&undoManager);
(**(code **)(*undoManager + 0x18))(undoManager, objectHandle, 0, 2, 0); // BeginModify

// ... modify object properties ...

// End transaction
FUN_021c6260(&undoManager2);
(**(code **)(*undoManager2 + 0x18))(undoManager2, objectHandle, 0, 3, 0); // EndModify
```

Transaction types (4th param):
- `2` = Begin modification
- `3` = End modification

---

### Detailed Function Analysis

#### SchAPI_CreateObject

```
SchAPI_CreateObject(result, container, objectType)
```

Creates an object via `SchAPI_CreateObjectEx` and immediately adds it to the specified container. Wrapper function.

**Parameters:**
- `param_1` (out): Result handle
- `param_2`: Container handle (sheet/component to add to)
- `param_3`: Object type byte

**Returns:** Object handle (via param_1)

#### SchAPI_CreateIterator

```
SchAPI_CreateIterator(containerHandle, filterObjectType) -> iteratorHandle
```

Creates a filtered iterator. The container is looked up via `FUN_011f11b0` (gets an internal iterator manager). If `filterObjectType` is non-zero, `FUN_021db920` maps it to an internal filter ID, which is applied via `FUN_011f16b0`.

**Parameters:**
- `param_1`: Container handle (document/component)
- `param_2`: Object type filter (0 = all types)

**Returns:** Iterator handle (longlong)

#### SchAPI_CreateSimpleIterator

```
SchAPI_CreateSimpleIterator(containerHandle, filterObjectType) -> iteratorHandle
```

Like `CreateIterator` but additionally calls `FUN_011f1480(iterator, 0)` which disables recursive descent into children - only iterates direct children.

#### SchAPI_CreateSpatialIterator

```
SchAPI_CreateSpatialIterator(containerHandle, filterBitmask) -> iteratorHandle
```

Creates an iterator filtered by both spatial region AND a bitmask of object types. The bitmask is encoded in a 9-byte structure passed on the stack. Iterates through 64 possible object types (loop 0..0x3F), converting each set bit via `FUN_021db920` to build the internal type filter.

**Parameters:**
- `param_1`: Container handle
- Stack parameter at offset 0x30: Type filter bitmask (9 bytes)

**Returns:** Iterator handle

#### SchAPI_GetFirstSpatialObject / GetNextSpatialObject

```
SchAPI_GetFirstSpatialObject(result, iteratorHandle, outObjectType) -> objectHandle
SchAPI_GetNextSpatialObject(result, iteratorHandle, outObjectType) -> objectHandle
```

Returns both the object handle AND the object type ID (via `outObjectType`). Default object type if not found is `0x3F` (unknown).

#### SchAPI_QueryPrimitive

```
SchAPI_QueryPrimitive(mode, objectHandle, outType, x, y, color, locked)
```

Queries/sets base primitive properties:
- **Read**: Gets object type, location (x,y), color, locked state
- **Write**: Moves object to (x,y) relative to current position, sets color and locked

#### SchAPI_QueryPin

```
SchAPI_QueryPin(mode, pinHandle, x, y, name, designator, hiddenPin, graphicLocked,
                electricalType, pinOrientation, pinLength, nameVisible, designVisible,
                pinLength2, locked, color)
```

Full pin property query with 16 parameters. In write mode, wraps modifications in undo transactions.

#### SchAPI_QuerySchPart (Component)

```
SchAPI_QuerySchPart(mode, componentHandle, x, y, orientation, mirror, orientation2,
                    locked, partCount, notUseSeparateDesignators, showHiddenPins,
                    designItemId, designator, [many more...])
```

The most complex query function with 28 parameters. Handles component placement, orientation, library references, designators, sub-parts, and footprint model associations. In write mode, performs complex library lookup (via both IntLib path and vault-style model resolution) and handles part swapping between sub-parts.

#### SchAPI_QuerySchPartUniqueId

```
SchAPI_QuerySchPartUniqueId(mode, componentHandle, uniqueId)
```

Simple unique ID get/set. Read mode calls vtable offset `0x160` (GetUniqueId), write mode calls offset `0x78` (SetUniqueId).

#### SchApiQuery_SchComponent (Extended Component)

```
SchApiQuery_SchComponent(mode, componentHandle, uniqueId, x, y, orientation,
                         mirror, color, areaColor, designatorVisibility,
                         showComment, locked, notUseSeparateDesignators,
                         componentOrientation, partCount, pinsMoved, pinMirrored,
                         designatorName, commentText, libraryReference,
                         libraryReference2, designItemId, componentPartId,
                         subPartIndex, currentPartId, designatorHandle,
                         footprintHandle, modelHandle)
```

28-parameter extended component query. Provides access to:
- All base properties (location, orientation, mirror, colors)
- Library references (DesignItemId, LibraryReference)
- Part/sub-part management (PartCount, SubPartIndex, CurrentPartId)
- String properties (Designator, Comment, UniqueId)
- Child object handles (designator parameter, footprint, model)

#### SchApiQuery_SchParameter

```
SchApiQuery_SchParameter(mode, paramHandle, x, y, color, locked,
                         orientation, fontId, justification,
                         name, value, isHidden, uniqueIndex,
                         fontName, displayName, valueText,
                         autoPosition, showName, isSystemParam, isDesignator)
```

20-parameter query for component parameters. Each component has named parameters (like Comment, Description, custom user parameters). In read mode it checks a `isRuntimeCalculated` flag to decide whether to read the raw value or the calculated/resolved value.

#### SchApiQuery_UniqueId

```
SchApiQuery_UniqueId(mode, objectHandle, uniqueIdString)
```

Universal unique ID query that works on multiple object types. Tries three different interface casts sequentially (offsets at UNK_021f4130, UNK_021f4140, UNK_021f4150) to find one that supports unique IDs. All three use vtable offset `0x160` to get the ID.

#### SchApiQuery_Implementation

```
SchApiQuery_Implementation(mode, implHandle, name, description,
                           isCurrent, isLocked, isIntegrated, isVault,
                           modelType, modelPath, modelName, enabled)
```

Queries implementation (footprint/model) links on components. Parameters include:
- Name and description
- State flags (current, locked, integrated, vault)
- Model type string, path, and name
- Enabled state

#### SchAPI_QueryVariable

```
SchAPI_QueryVariable(resultString, variableIndex) -> bool
```

Large function that queries or sets system variables by name. Parses a variable specification with `Category` and `Variable` fields. Known categories:

- **Category = "Get"**: Reads the variable value
  - `Variable = "SnapGridSize"` -> Gets snap grid size
  - `Variable = "VisibleGridSize"` -> Gets visible grid size
  - `Variable = "UsingElectricalGrid"` -> Gets electrical grid state
  - `Variable = "HotSpotGridOn"` -> Gets hotspot grid state
  - `Variable = "IsLibraryDocument"` -> Gets if current doc is a library
  - `Variable = "Power"` (with `Unit` sub-param) -> Gets power-related units
    - Unit names map to indices (0-17+ different unit types)
    - Queries: Name, Text, X, Y, OffsetX for each power symbol

- **Category = "Set"**: Writes the variable value
  - Same variables as Get, with values passed through the `Value` field

#### SchAPI_QueryDocumentOptions

```
SchAPI_QueryDocumentOptions(mode, docHandle, sheetStyle, visibleGridEnabled,
                            snapGridEnabled, electricalGridEnabled,
                            gridSize, color, useCustomSheet,
                            areaColor, showBorder, templateName,
                            showTemplateGraphics, snapSize, showParameters,
                            customSheetWidth, customSheetHeight,
                            showReferenceZones, xZoneCount, yZoneCount,
                            xMargin, yMargin, zoneWidth, ...[8 zone names]...,
                            titleBlockPosition, orientation, fontIdString, headerSize)
```

35-parameter query for document-level options including:
- Sheet style and size
- Grid settings (visible, snap, electrical)
- Colors
- Custom sheet dimensions
- Reference zone configuration
- Title block settings
- Template information

#### SchAPI_SendCommand

```
SchAPI_SendCommand(commandString, param2, param3, param4) -> 1
```

Sends a command string to the DXP command processor via `FUN_00895590`. Always returns 1. The command string is a WideString processed through the standard string conversion pipeline.

#### SchAPI_LoadComponentFromLibrary

```
SchAPI_LoadComponentFromLibrary(result, libraryPath, componentName) -> objectHandle
```

Loads a component definition from a library file. Takes the library file path and the component name as WideString parameters. Uses `FUN_021d6290` to get a library loader interface, then calls its method at offset `0x18` to perform the load.

---

## Common Helper Functions Referenced

| Function | Purpose |
|----------|---------|
| `FUN_0041b530` | Release interface / clear pointer |
| `FUN_0041b570` | Assign interface (addref + copy) |
| `FUN_0041b5c0` | QueryInterface with GUID check |
| `FUN_0041b8c0` | OleCheck / HRESULT check |
| `FUN_00411570` | TObject.Free |
| `FUN_004604c0` | Cast/verify object to expected interface type |
| `FUN_004604f0` | Cast/verify (alternative, may be more permissive) |
| `FUN_00452710` | WideString copy/create |
| `FUN_004522e0` | WideString assign to output |
| `FUN_00416c90` | WideString to interface string |
| `FUN_00416c60` | Interface string to WideString |
| `FUN_00414c50` | Allocate string buffer |
| `FUN_00414bb0` | Free WideString |
| `FUN_0044f940` | Integer to string conversion |
| `FUN_0044fc00` | String to integer conversion |
| `FUN_0044e720` | Wide string comparison |
| `FUN_00409680` | Setup variant / parameter record |
| `FUN_004096b0` | Cleanup variant / parameter record |
| `FUN_0087bad0` | Parse key=value from parameter string |
| `FUN_0087bb90` | Write key=value to parameter string |
| `FUN_021c6260` | Get undo manager interface |
| `FUN_021c64b0` | Get document options interface |
| `FUN_021d67f0` | Get owner/container for an object |
| `FUN_021d6290` | Get library loader interface |
| `FUN_021db920` | Map SchAPI object type ID to internal filter ID |
| `FUN_021dbf70` | Map internal object type to SchAPI type ID |
| `FUN_008954d0` | Get ServerModule/Application interface |
| `FUN_00892d50` | Get ServerDocumentManager |
| `FUN_011f11b0` | Create iterator from manager |
| `FUN_011f16b0` | Set type filter on iterator |
| `FUN_011f1480` | Set iterator options (recursive flag) |
| `FUN_011f1b40` | Iterator.First() |
| `FUN_011f1c00` | Iterator.Next() |
| `FUN_01c103a0` | Look up component in IntLib by name |
| `FUN_01c0fd30` | Load component from library file |

---

## Cross-Cutting Observations

1. **All 135 SchAPI functions are in AdvSch.dll only** - none in the .NET DLLs. The .NET layer accesses schematic data through entirely different interfaces.

2. **No DxpApi_* functions exist in AdvSch.dll** - the DXP cross-cutting API is likely in a separate DLL (possibly Client.dll or DXP.dll).

3. **Object handles are interface pointers** - They are COM-style interface pointers with reference counting. The vtable offsets are consistent across object types for shared properties (location at 0x328, color at 0x330, etc.).

4. **The API is deeply vtable-driven** - Nearly all property access goes through vtable dispatch, meaning the actual implementation lives in the concrete object classes, not in the API layer.

5. **Iterator management is centralized** - All iterator types (standard, simple, spatial, group) share common infrastructure at FUN_011f*.

6. **Undo management is pervasive** - Write operations consistently bracket modifications with begin/end transaction calls.

7. **String encoding is UTF-16** (Delphi WideString) throughout the API.

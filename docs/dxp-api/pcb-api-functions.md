# PcbApi Delphi Functions (Advpcb.dll)

Research conducted via ghidra-cli on the `altium26` project, program `Advpcb.dll`.

All PcbApi functions are exported exclusively from `Advpcb.dll`. No PcbApi functions were found in `Altium.PCB.DataModel.dll`, `Altium.PCB.DataModel.X.dll`, or `Altium.PCB.BinaryLoader.dll`.

## Table of Contents

1. [Summary of All Functions](#summary-of-all-functions)
2. [Object Type IDs](#object-type-ids)
3. [Layer ID Mapping](#layer-id-mapping)
4. [Common Parameter Patterns](#common-parameter-patterns)
5. [Detailed Function Analysis](#detailed-function-analysis)
   - [Iterator Functions](#iterator-functions)
   - [Object Factory Functions](#object-factory-functions)
   - [Query Functions - Primitives](#query-functions---primitives)
   - [Query Functions - Board & Layers](#query-functions---board--layers)
   - [Query Functions - Components](#query-functions---components)
   - [Query Functions - Rules](#query-functions---rules)
   - [Query Functions - Dimensions](#query-functions---dimensions)
   - [Container Management](#container-management)
   - [Export / Painter Functions](#export--painter-functions)
   - [Library Reader Functions](#library-reader-functions)
   - [Event & Robot Functions](#event--robot-functions)
   - [Undo/Redo Functions](#undoredo-functions)
   - [Miscellaneous Functions](#miscellaneous-functions)

---

## Summary of All Functions

Total: ~290 PcbApi_* functions + 4 PCBAPI_* functions

### Iterator & Traversal

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_CreateIterator | 03d1fb90 | Create iterator for traversing PCB objects |
| PcbApi_DestroyIterator | 03d1fe60 | Destroy/free an iterator |
| PcbApi_Iterator_ProcessSpecialLayers | 03d1fe80 | Configure iterator for special layer processing |
| PcbApi_Iterator_ApplyEmbeddedBoardArrayFilter | 03d1feb0 | Apply embedded board array filter to iterator |
| PcbApi_GetNextObject | 03d1ff00 | Get next object from iterator (vtable call +0x28) |
| PcbApi_GetFirstObject | 03d1ff30 | Get first object from iterator (vtable call +0x20) |
| PcbApi_CreateSpatialIterator | 03d1ff60 | Create spatial (region-based) iterator |
| PcbApi_DestroySpatialIterator | 03d20040 | Destroy spatial iterator |
| PcbApi_GetNextSpatialObject | 03d20060 | Get next object from spatial iterator |
| PcbApi_GetFirstSpatialObject | 03d200e0 | Get first object from spatial iterator |

### Current Document & Board

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_GetCurrentComponent | 03d200e0 | Get currently selected/active component |
| PcbApi_GetCurrentBoardHandle | 03d202a0 | Get handle to current board document |
| PcbApi_GetBoardHandleFromFullFileName | 03d20460 | Get board handle from full file path |
| PcbApi_GetBoardHandleFromFileName | 03d205f0 | Get board handle from filename |
| PcbApi_LoadBoardByFullFileName | 03d20660 | Load a board document by full path |
| PcbApi_CloseDocumentByFullFileName | 03d20820 | Close a board document by full path |
| PcbApi_SetCurrentBoardHandle | 03d52d10 | Set the current active board |
| PcbApi_SetBoardIsFullyLoaded | 03d590c0 | Mark board as fully loaded |

### Cursor & UI Interaction

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_GetObjectAtCursor | 03d20940 | Get object at current cursor position |
| PcbApi_GetObjectAtCursor_2 | 03d20d60 | Extended version of GetObjectAtCursor |
| PcbApi_ChooseRectangleByCorners | 03d21160 | Interactive rectangle selection |
| PcbApi_ChooseLocation | 03d214b0 | Interactive point selection |
| PcbApi_QueryCursorLocation | 03d21780 | Query current cursor position |
| PcbApi_GetObjectAtXYAskUserIfAmbiguous | 03d58310 | Get object at XY, prompt user if ambiguous |
| PcbApi_GetObjectAtXYAskUserIfAmbiguous_2 | 03d585d0 | Extended version |
| PcbApi_QueryFocusedPrimitive | 03d58930 | Get currently focused primitive |

### Progress / Status

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_Percent_Init | 03d21c20 | Initialize progress bar |
| PcbApi_Percent_Update | 03d21f20 | Update progress bar |
| PcbApi_Percent_UpdateByNumber | 03d22220 | Update progress bar by number |
| PcbApi_Percent_Finish | 03d224d0 | Finish/close progress bar |
| PcbApi_ProcessStatus_UpdatePercent | 03d22840 | Update process status percentage |

### Object Factory

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_ReplicateObject | 03d228e0 | Clone/replicate an existing object |
| PcbApi_CreateObject | 03d22900 | Create a new PCB object by type ID |
| PcbApi_CreateDimensionObject | 03d22b10 | Create a dimension object by subtype |
| PcbApi_DestroyObject | 03d22c70 | Destroy a PCB object |
| PcbApi_CreateRuleObject | 03d22c70 | Create a design rule object |
| PcbApi_CreateClassObject | 03d22c90 | Create a class (net class, component class) |
| PcbApi_CreateClassObjectEx | 03d22d00 | Extended class creation |
| PcbApi_CreateBoardOutline | 03d2dec0 | Create board outline |
| PcbApi_UpdateBoardOutline | 03d2df10 | Update board outline |
| PcbApi_CreateRule_FromParameters | 03d59270 | Create rule from parameter string |
| PcbApi_CreateLibComponent | 03d594e0 | Create library component |
| PcbApi_DestroyLibComponent | 03d594f0 | Destroy library component |
| PcbApi_CreateObjectByViewableObjectId | 03d5b700 | Create object by viewable ID |

### Object Properties / Utility

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_GetPalette | 03d22d30 | Get color palette |
| PcbApi_FindDominantRuleForObject | 03d22d40 | Find dominant rule for single object |
| PcbApi_FindDominantRuleForUpdatedObject | 03d22d50 | Find dominant rule for modified object |
| PcbApi_FindDominantRuleForObjectPair | 03d22d80 | Find dominant rule for pair of objects |
| PcbApi_AnalyzeNet | 03d22da0 | Analyze a net |
| PcbApi_ShowObject | 03d22dc0 | Make object visible |
| PcbApi_HideObject | 03d22e00 | Make object invisible |
| PcbApi_GetObjectBitField2 | 03d22e40 | Get object bitfield 2 |
| PcbApi_GetObjectBitField3 | 03d22e80 | Get object bitfield 3 |
| PcbApi_GetObjectIdFromObjectHandle | 03d22ec0 | Get object type ID from handle |
| PcbApi_GetObjectDescriptionFromObjectHandle | 03d22ee0 | Get human-readable description |
| PcbApi_PushStateOfObject | 03d23520 | Push object state for undo |
| PcbApi_PopStateOfObject | 03d23550 | Pop/restore object state |
| PcbApi_QueryObjectSetState_Default | 03d3baf0 | Reset object to defaults |
| PcbApi_QueryObjectImport_FromUser | 03d3baf0 | Import object from user interaction |
| PcbApi_QueryObjectGraphicallyInvalidate | 03d3bc80 | Mark object for graphical refresh |
| PcbApi_QueryObjectBoundingRectangle | 03d3bc80 | Get object bounding rectangle |
| PcbApi_QueryObjectBoundingRectangleChildren | 03d3bd00 | Get bounding rectangle including children |
| PcbApi_QueryObjectParameters | 03d58ec0 | Get/set object parameter collection |
| PcbApi_GetObjectFromPrimitive | 03d5a330 | Get high-level object from primitive |
| PcbApi_IsBoardRegion | 03d5a3f0 | Check if object is a board region |
| PcbApi_GetViewableObjectIdFromObjectHandle | 03d5b6d0 | Get viewable object ID |
| PcbApi_InvertObject | 03d58850 | Invert object (mirror) |
| PcbApi_ScreenDisplay | 03d58890 | Screen display refresh |
| PcbApi_SetOwnerBoard | 03d588e0 | Set owning board for object |
| PcbApi_GetDesignatorDisplayString | 03d591e0 | Get designator display string |
| PcbApi_GrowPolyshape | 03d59100 | Grow/expand a polyshape |

### Container Management

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_AddObjectToContainer | 03d22f70 | Add object to board/component container |
| PcbApi_DeleteObjectFromContainer | 03d232b0 | Delete object from container |
| PcbApi_DeleteObjectFromComponent | 03d234c0 | Delete object from component |

### Net / Class Management

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_CleanNet | 03d23580 | Clean up a net |
| PcbApi_RunNetAnalyser | 03d235b0 | Run net analyzer |
| PcbApi_LoadComponentFromLibrary | 03d235e0 | Load component from library |
| PcbApi_AddMemberToClass | 03d23800 | Add member to class |
| PcbApi_GetMemberFromClassAt | 03d238d0 | Get member at index from class |
| PcbApi_RemoveAllMembersFromClass | 03d23a20 | Remove all members from class |

### Query Functions - Core Primitives

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_QueryPrimitive | 03d254f0 | Get/set common primitive properties |
| PcbApi_QueryTrack | 03d2f240 | Get/set track properties |
| PcbApi_QueryVia | 03d2f710 | Get/set via properties |
| PcbApi_QueryVia_2 | 03d31920 | Extended via query |
| PcbApi_QueryViaHeight | 03d319d0 | Get via height |
| PcbApi_QueryFill | 03d31a20 | Get/set fill properties |
| PcbApi_QueryRegion | 03d31c20 | Get/set region properties |
| PcbApi_QueryArc | 03d31e70 | Get/set arc properties |
| PcbApi_QueryText | 03d32090 | Get/set text properties |
| PcbApi_QueryTrueTypeTextInfo | 03d32540 | Get TrueType text rendering info |
| PcbApi_QueryBarCodeTextInfo | 03d327d0 | Get barcode text info |
| PcbApi_ConvertLegendText2Polygon | 03d327d0 | Convert legend text to polygon |
| PcbApi_FreeGPCPolygon | 03d328c0 | Free GPC polygon memory |
| PcbApi_QueryCoordinate | 03d2f010 | Get/set coordinate object properties |
| PcbApi_QueryWirebond | 03d5b700 | Get/set wirebond properties |

### Query Functions - Pad

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_QueryPad | 03d255f0 | Get/set pad properties (37 params!) |
| PcbApi_QueryPadConnectToLayer | 03d26830 | Query pad connection to layer |
| PcbApi_QueryPadConnectToLayer_2 | 03d27240 | Extended pad-layer connection query |
| PcbApi_QueryPadCache | 03d27270 | Get pad cache data |
| PcbApi_QueryPadCacheFull | 03d27370 | Get full pad cache |
| PcbApi_QueryPadCacheExt | 03d27710 | Extended pad cache query |
| PcbApi_QueryPadHoleTolerance | 03d27980 | Get pad hole tolerance |
| PcbApi_QueryPadOrViaShapeOnLayer | 03d27c60 | Get pad/via shape on specific layer |
| PcbApi_QueryPadOrViaShapeOnLayer_2 | 03d28660 | Extended shape-on-layer query |
| PcbApi_QueryPadShapeInfoOnLayer | 03d28680 | Get detailed pad shape info on layer |
| PcbApi_QueryPadShapeInfoOnLayerExt | 03d28a30 | Extended pad shape info on layer |
| PcbApi_SetCRPctExtOnLayer | 03d28ee0 | Set corner radius percentage ext |
| PcbApi_SetCRPctEnabledOnLayer | 03d28ee0 | Enable/disable CR% on layer |
| PcbApi_SetCRSizeOnLayer | 03d28f50 | Set corner radius size on layer |
| PcbApi_QueryPadOrViaSizeOnLayer | 03d29310 | Get pad/via size on layer |
| PcbApi_QueryPadOrViaSizeOnLayer_2 | 03d29d10 | Extended size-on-layer |
| PcbApi_QueryPadHoleSizeOnLayer | 03d29d30 | Get pad hole size on layer |
| PcbApi_QueryPadCornerRadiusOnLayer | 03d29dd0 | Get pad corner radius on layer |
| PcbApi_QueryPadCornerRadiusOnLayer_2 | 03d2a7d0 | Extended corner radius on layer |
| PcbApi_QueryPadCRPercentageOnLayer | 03d2a840 | Get corner radius percentage |
| PcbApi_QueryPadCRPercentageOnLayer_2 | 03d2b240 | Extended CR% on layer |
| PcbApi_SetPadShapeOnLayer | 03d2b2d0 | Set pad shape on specific layer |
| PcbApi_SetPadShapeOnLayer_2 | 03d2bcd0 | Extended set shape on layer |
| PcbApi_SetPadSizeOnLayer | 03d2bd70 | Set pad size on specific layer |
| PcbApi_SetPadSizeOnLayer_2 | 03d2c770 | Extended set size on layer |
| PcbApi_SetCRPctOnLayer | 03d2c800 | Set CR% on layer |
| PcbApi_SetCRPctOnLayer_2 | 03d2d200 | Extended set CR% |
| PcbApi_QueryPadStackMode | 03d2d220 | Get pad stack mode |
| PcbApi_QueryViaStackMode | 03d2d290 | Get via stack mode |
| PcbApi_QueryViaStackSizeOnLayer | 03d2d300 | Get via stack size on layer |
| PcbApi_QueryPad_PwrGnd | 03d2d370 | Query power/ground pad info |
| PcbApi_QueryViaPadOnPlaneLayer | 03d5b360 | Get via pad on plane layer |
| PcbApi_QueryViaHoleTolerance | 03d27a00 | Get via hole tolerance |
| PcbApi_QueryViaBackDrill | 03d27a80 | Get via back drill info |

### Query Functions - Polygon

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_QueryPolygon | 03d32f80 | Get/set polygon properties + vertices |
| PcbApi_QueryPolygonEx | 03d33120 | Extended polygon query |
| PcbApi_QueryPolygonSegmentCount | 03d332c0 | Get polygon segment count |
| PcbApi_QuerySplitPlane | 03d33340 | Get/set split plane properties |
| PcbApi_ConvertRegionShapeToPolyUtilsShape | 03d5b3b0 | Convert region to poly utils shape |

### Query Functions - Embedded Board

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_QueryEmbeddedBoard | 03d328e0 | Get/set embedded board properties |
| PcbApi_QueryEmbeddedBoardExt | 03d32bb0 | Extended embedded board query |
| PcbApi_QueryEmbedded | 03d42c70 | General embedded query |

### Query Functions - Component

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_QueryComponent | 03d36340 | Get/set component properties |
| PcbApi_QueryComponent_2 | 03d36f20 | Extended component query |
| PcbApi_QueryComponentEx | 03d33530 | Extra component fields (4 params) |
| PcbApi_QueryComponent_ApplyAutopositionNameComment | 03d33610 | Apply auto-position for name/comment |
| PcbApi_QueryLibComponent | 03d36fd0 | Get/set library component properties |
| PcbApi_QueryComponentUniqueId | 03d371d0 | Get component unique ID |
| PcbApi_QueryComponentSourceLinks | 03d37260 | Get component source library links |
| PcbApi_QueryComponentVaultLinks | 03d3a8c0 | Get component vault links |
| PcbApi_GetComponentSchParameter | 03d3ab40 | Get schematic parameter from component |
| PcbApi_QueryDefaultPCB3DModel | 03d3b7f0 | Get default 3D model |
| PcbApi_QueryComponentKind | 03d3b8e0 | Get component kind |
| PcbApi_QueryComponentChannelOffset | 03d3b950 | Get channel offset |
| PcbApi_QueryComponent_GetObjectAt | 03d3b9c0 | Get child object at index |
| PcbApi_QueryRemoveCavity | 03d3ba30 | Query remove cavity |
| PcbApi_QueryRebuidCavity | 03d3ba80 | Query rebuild cavity |

### Query Functions - Dimension

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_QueryDimension | 03d2df80 | Get/set common dimension properties |
| PcbApi_QueryExtDimension | 03d2e660 | Extended dimension query |
| PcbApi_References_Count | 03d2e6d0 | Get dimension reference count |
| PcbApi_References_At | 03d2e820 | Get dimension reference at index |
| PcbApi_References_Add | 03d2e890 | Add dimension reference |
| PcbApi_TextLocations_Count | 03d2e900 | Get text location count |
| PcbApi_TextLocations_At | 03d2e9e0 | Get text location at index |
| PcbApi_TextLocations_Add | 03d2ea40 | Add text location |
| PcbApi_QueryLinearDimension | 03d2ead0 | Get linear dimension properties |
| PcbApi_QueryAngularDimension | 03d2eb80 | Get angular dimension properties |
| PcbApi_QueryRadialDimension | 03d2ec10 | Get radial dimension properties |
| PcbApi_QueryLeaderDimension | 03d2ece0 | Get leader dimension properties |
| PcbApi_QueryDatumDimension | 03d2ed70 | Get datum dimension properties |
| PcbApi_QueryBaselineDimension | 03d2ee00 | Get baseline dimension properties |
| PcbApi_QueryCenterDimension | 03d2ee90 | Get center dimension properties |
| PcbApi_QueryOriginalDimension | 03d2eef0 | Get original dimension properties |
| PcbApi_QueryLinearDiameterDimension | 03d2ef80 | Get linear diameter dim properties |
| PcbApi_QueryRadialDiameterDimension | 03d2f010 | Get radial diameter dim properties |

### Query Functions - Net

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_QueryNet | 03d25390 | Get/set net properties |
| PcbApi_QueryNetRules | 03d3bd30 | Get rules for a net |
| PcbApi_QueryClass | 03d23a20 | Get/set class properties |
| PcbApi_QueryFromTo | 03d23cd0 | Get from-to pair properties |
| PcbApi_QueryFromTo_2 | 03d25150 | Extended from-to query |
| PcbApi_QueryManualFromTo | 03d251b0 | Get manual from-to properties |

### Query Functions - Board

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_QueryBoard | 03d3bf40 | Get/set board-level properties |
| PcbApi_QueryBoardGrids | 03d3c730 | Get board grids |
| PcbApi_QueryBoardGridsEx | 03d3c940 | Extended board grids |
| PcbApi_QueryBoardLayerInfo | 03d3ccf0 | Get layer info (name, type, etc.) |
| PcbApi_QueryBoardLayerInfo_2 | 03d3d710 | Extended layer info |
| PcbApi_QueryBoardAdvancedPlacerOptions | 03d3d740 | Get advanced placer options |
| PcbApi_QueryBoardAdvancedPlacerOptionsExt | 03d3d8a0 | Extended placer options |
| PcbApi_QueryBoardAdvancedRouterOptions | 03d3d930 | Get advanced router options |
| PcbApi_QueryBoardDesignRuleCheckerOptions | 03d3d940 | Get DRC options |
| PcbApi_QueryBoardSpecctraRouterOptions | 03d3da00 | Get Specctra router options |
| PcbApi_QueryBoardWindowRectangle | 03d3e390 | Get board window rectangle |
| PcbApi_QueryBoardSplitPlaneMode | 03d3e690 | Get split plane mode |
| PcbApi_QueryBoardInternalPlaneNets | 03d3e6d0 | Get internal plane net assignments |
| PcbApi_QueryBoardLayerPairsCount | 03d3e970 | Get number of layer pairs |
| PcbApi_QueryBoardLayerPairAt | 03d3ea10 | Get layer pair at index |
| PcbApi_QueryBoardLayerPairBackDrill | 03d3ebd0 | Get back drill info for layer pair |
| PcbApi_GetDrillTableLayerPairFromItsObject | 03d3ec50 | Get drill table layer pair |
| PcbApi_QueryBoardEngineeringChangeOrderOptions | 03d3ed70 | Get ECO options |
| PcbApi_QueryBoardOutputOptions | 03d3ee10 | Get output options |
| PcbApi_QueryBoardOutputOptionsPlotLayers | 03d3fee0 | Get plot layer options |
| PcbApi_QueryBoardOutputOptionsFlipLayers | 03d3ffe0 | Get flip layer options |
| PcbApi_QueryBoardGerberOptions | 03d40330 | Get Gerber output options |
| PcbApi_QueryBoardPrinterOptions | 03d40860 | Get printer options |
| PcbApi_QueryBoardPrinterOptionsPlotterPen | 03d40a70 | Get plotter pen options |
| PcbApi_QueryBoardPrinterOptionsCompositeLayer | 03d41490 | Get composite layer print options |
| PcbApi_QueryBoardPrinterOptionsCompositeLayer_2 | 03d414d0 | Extended composite layer options |
| PcbApi_QueryBoardShowDrillSymbolsDlg | 03d414d0 | Show drill symbols dialog |
| PcbApi_QueryBoardGetDrillSymbolsConfiguration | 03d41580 | Get drill symbols config |
| PcbApi_QueryBoardShowHoleSymbols | 03d41650 | Show hole symbols |
| PcbApi_QueryBoardShowHoleSymbols_ByLayerPair | 03d41650 | Show hole symbols by layer pair |
| PcbApi_QueryBoardShowHoleSymbols_ByLayerPairEx | 03d416c0 | Extended hole symbols by pair |
| PcbApi_QueryBoardShowHoleSymbolsEx | 03d41770 | Extended hole symbols |
| PcbApi_QueryBoardShowAllHoleSymbolsWithCurrentSettings | 03d418d0 | Show all holes current settings |
| PcbApi_QueryBoardHideHoleSymbols | 03d41910 | Hide hole symbols |
| PcbApi_QueryBoardDrillDrawLegendPresent | 03d41a20 | Check drill legend presence |
| PcbApi_QueryBoardMechanicalLayerKindMapping | 03d41b00 | Get mech layer kind mapping |
| PcbApi_QueryBoardDrillSymbolIndex | 03d41b90 | Get drill symbol index |
| PcbApi_QueryBoardDrillSymbolIndexByObjectHandle | 03d41bd0 | Get drill symbol by object |
| PcbApi_QueryBoardInteractiveRoutingOptions | 03d51e10 | Get interactive routing options |
| PcbApi_QueryBoardInteractiveRoutingOptions_2 | 03d52a90 | Extended routing options |
| PcbApi_QueryBoardExternalDielectrics | 03d49fd0 | Get external dielectric info |
| PcbApi_QuerySheet | 03d3bd90 | Get/set sheet properties |

### Layer Stack Functions

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_QueryLayer | 03d451b0 | Get/set layer properties |
| PcbApi_QueryLayer_2 | 03d46ff0 | Extended layer query |
| PcbApi_QueryInternalPlane | 03d474b0 | Get/set internal plane properties |
| PcbApi_QueryInternalPlane_2 | 03d492f0 | Extended internal plane query |
| PcbApi_QueryLayerDielectric | 03d49570 | Get/set dielectric layer properties |
| PcbApi_QueryLayerDielectric_2 | 03d49f90 | Extended dielectric query |
| PcbApi_InsertLayerInStack | 03d4a4e0 | Insert layer into stack |
| PcbApi_InsertLayerInStack_2 | 03d4b8a0 | Extended insert layer |
| PcbApi_RemoveLayerFromStack | 03d4b9c0 | Remove layer from stack |
| PcbApi_RemoveLayerFromStack_2 | 03d4c3b0 | Extended remove layer |
| PcbApi_ViewLayerSet_BeginUpdate | 03d5a910 | Begin layer set update batch |
| PcbApi_ViewLayerSet_EndUpdate | 03d5a9c0 | End layer set update batch |
| PcbApi_PlaneManager_LayerIsPourable | 03d5b300 | Check if layer is pourable |

### Query Functions - Rules

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_QueryRule | 03d42fb0 | Get/set design rule properties |
| PcbApi_QueryRule_2 | 03d44730 | Extended rule query |
| PcbApi_QueryRuleEx | 03d44870 | Extra rule properties |
| PcbApi_QueryRuleScopeDescriptions | 03d44a40 | Get rule scope descriptions |
| PcbApi_Scope2_Includes | 03d44bd0 | Check if scope 2 includes object |
| PcbApi_Scope1_Includes | 03d44c10 | Check if scope 1 includes object |
| PcbApi_QueryRuleExt | 03d44c10 | Extended rule properties |
| PcbApi_QueryRuleExtEx | 03d44cb0 | Extra extended rule props |
| PcbApi_SetState_DeleteAllSubScopes | 03d44da0 | Delete all sub-scopes |
| PcbApi_SetState_AddSubscope | 03d44df0 | Add a sub-scope |
| PcbApi_GetState_SubScopesCount | 03d44e50 | Get sub-scope count |
| PcbApi_GetState_SubscopeAt | 03d44e60 | Get sub-scope at index |
| PcbApi_QueryViolation | 03d4c3d0 | Get design rule violation |
| PcbApi_QueryRuleParallelSegmentConstraint | 03d4c4f0 | Parallel segment constraint |
| PcbApi_QueryRuleClearanceConstraint | 03d4c570 | Clearance constraint |
| PcbApi_QueryRuleMaxMinLengthConstraint | 03d4c5d0 | Max/min length constraint |
| PcbApi_QueryRuleMaxMinWidthConstraint | 03d4c650 | Max/min width constraint |
| PcbApi_QueryRuleMaxMinWidthConstraintExt | 03d4c6d0 | Extended width constraint |
| PcbApi_QueryRuleMaxMinWidthConstraintByLayer | 03d4c820 | Width constraint by layer |
| PcbApi_QueryRuleMaxMinWidthConstraintByLayer_2 | 03d4d240 | Extended by-layer width |
| PcbApi_QueryRuleRoutingCornerStyleRule | 03d4d270 | Routing corner style |
| PcbApi_QueryRuleRoutingViaStyleRule | 03d4d310 | Routing via style |
| PcbApi_QueryRuleRoutingViaStyleRuleExt | 03d4d3b0 | Extended via style rule |
| PcbApi_QueryRuleRoutingLayersRule | 03d4f370 | Routing layers rule |
| PcbApi_QueryRuleRoutingTopologyRule | 03d4f3f0 | Routing topology rule |
| PcbApi_QueryRuleRoutingPriorityRule | 03d4f450 | Routing priority rule |
| PcbApi_QueryRulePowerPlaneConnectStyleRule | 03d4f4b0 | Power plane connect style |
| PcbApi_QueryRulePolygonConnectStyleRule | 03d4f610 | Polygon connect style |
| PcbApi_QueryRulePasteMaskExpansionRule | 03d4f750 | Paste mask expansion |
| PcbApi_QueryRuleSolderMaskExpansionRule | 03d4f7b0 | Solder mask expansion |
| PcbApi_QueryRuleSolderMaskExpansionRuleFull | 03d4f800 | Full solder mask expansion |
| PcbApi_QueryRulePowerPlaneExpansionRule | 03d4f8c0 | Power plane expansion |
| PcbApi_QueryRuleDaisyChainStubLengthConstraint | 03d4f920 | Daisy chain stub length |
| PcbApi_QueryRuleMatchedNetLengthsConstraint | 03d4f980 | Matched net lengths |
| PcbApi_QueryRuleShortCircuitConstraint | 03d4fa40 | Short circuit constraint |
| PcbApi_QueryRuleBrokenNetRule | 03d4faa0 | Broken net rule |
| PcbApi_QueryRuleViasUnderSMDConstraint | 03d4fae0 | Vias under SMD constraint |
| PcbApi_QueryRuleMaximumViaCountRule | 03d4fb40 | Maximum via count |
| PcbApi_QueryRuleMinimumAnnularRingRule | 03d4fba0 | Minimum annular ring |
| PcbApi_QueryRuleAcuteAngleRule | 03d4fc00 | Acute angle rule |
| PcbApi_QueryRuleConfinementConstraint | 03d4fc60 | Confinement constraint |
| PcbApi_QueryRuleSMDToCorner | 03d4fdd0 | SMD to corner rule |
| PcbApi_QueryRuleComponentClearanceConstraint | 03d4fdd0 | Component clearance |
| PcbApi_QueryRuleComponentRotationsRule | 03d4fe60 | Component rotations |
| PcbApi_QueryRulePermittedLayersRule | 03d4fee0 | Permitted layers |
| PcbApi_QueryRuleSignalStimulus | 03d50230 | Signal stimulus |
| PcbApi_QueryRuleMaxOvershootRise | 03d50350 | Max overshoot rise |
| PcbApi_QueryRuleMaxOvershootFall | 03d503e0 | Max overshoot fall |
| PcbApi_QueryRuleMaxUndershootRise | 03d50470 | Max undershoot rise |
| PcbApi_QueryRuleMaxUndershootFall | 03d50500 | Max undershoot fall |
| PcbApi_QueryRuleMaxMinImpedance | 03d50590 | Max/min impedance |
| PcbApi_QueryRuleMinSignalTopValue | 03d50640 | Min signal top value |
| PcbApi_QueryRuleMaxSignalBaseValue | 03d506d0 | Max signal base value |
| PcbApi_QueryRuleFlightTime_RisingEdge | 03d50760 | Flight time rising edge |
| PcbApi_QueryRuleFlightTime_FallingEdge | 03d507f0 | Flight time falling edge |
| PcbApi_QueryRuleMaxSlope_RisingEdge | 03d50880 | Max slope rising edge |
| PcbApi_QueryRuleMaxSlope_FallingEdge | 03d50910 | Max slope falling edge |
| PcbApi_QueryRuleSupplyNets | 03d509a0 | Supply nets |
| PcbApi_QueryRuleTestpointUsage | 03d50a30 | Testpoint usage |
| PcbApi_QueryRuleTestpointStyle | 03d50ae0 | Testpoint style |
| PcbApi_QueryRuleTestpointStyleExtended | 03d50c90 | Extended testpoint style |
| PcbApi_QueryRuleDiffPairsRoutingRuleRule | 03d5a420 | Diff pairs routing rule |
| PcbApi_QueryRuleCreepage | 03d5a5a0 | Creepage rule |
| PcbApi_QueryRuleWirebond | 03d5a640 | Wirebond rule |
| PcbApi_QueryRuleReturnPath | 03d5a780 | Return path rule |

### System Options

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_RunColorDialog | 03d41bd0 | Run color selection dialog |
| PcbApi_QuerySystemOptions | 03d41be0 | Get system options |
| PcbApi_QuerySystemOptionsPlaceArray | 03d41f10 | Get place array options |
| PcbApi_QuerySystemOptionsPorts | 03d420e0 | Get port options |
| PcbApi_QuerySystemOptionsMiscellaneous | 03d421b0 | Get misc options |
| PcbApi_QuerySystemOptionsMiscellaneousEx | 03d42880 | Extended misc options |
| PcbApi_GetState_ComponentTypeMappingsCount | 03d42970 | Get component type mapping count |
| PcbApi_SetState_DeleteAllComponentTypeMappings | 03d42980 | Delete all type mappings |
| PcbApi_SetState_AddComponentTypeMapping | 03d42990 | Add component type mapping |
| PcbApi_GetState_ComponentTypeMappingAt | 03d42a00 | Get type mapping at index |
| PcbApi_QuerySystemGetGlobalDimension | 03d42a00 | Get global dimension settings |
| PcbApi_QuerySystemGetGlobalPrimitive | 03d42b30 | Get global primitive settings |
| PcbApi_QuerySystemOptionsLayerDrawingOrder | 03d50e20 | Get layer drawing order |
| PcbApi_QuerySystemOptionsLayerDrawingOrder_2 | 03d51840 | Extended layer drawing order |
| PcbApi_QuerySystemOptionsPolygon | 03d51850 | Get polygon options |
| PcbApi_QuerySystemOptionsProtection | 03d51910 | Get protection options |
| PcbApi_GetSystemOptions | 03d5a170 | Get system options handle |

### Test Functions

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_TestTrack | 03d53000 | Test track validity |
| PcbApi_TestArc | 03d53a90 | Test arc validity |
| PcbApi_TestFill | 03d54500 | Test fill validity |
| PcbApi_TestPad | 03d54f90 | Test pad validity |
| PcbApi_TestVia | 03d55ab0 | Test via validity |
| PcbApi_TestText | 03d56ef0 | Test text validity |
| PcbApi_ConvertTextToStrokeArray | 03d57aa0 | Convert text to stroke array |

### Command / Event System

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_SendCommand | 03d52bf0 | Send command string |
| PcbApi_GetCurrentEditorWindow | 03d52c90 | Get current editor window |
| PcbApi_EventRouter_SendMessage | 03d57dd0 | Send event message |
| PcbApi_EventRouter_SendMessageEx | 03d57e00 | Extended event message |
| PcbApi_GetState_ProcessDepth | 03d57e60 | Get process depth |
| PcbApi_SetState_ProcessDepth | 03d57f20 | Set process depth |

### Robot Functions

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_AddRobotToRobotsList | 03d57ff0 | Register robot |
| PcbApi_RemoveRobotFromRobotsList | 03d58000 | Unregister robot |
| PcbApi_GetRobotsCount | 03d58010 | Get robot count |
| PcbApi_GetRobotAt | 03d58020 | Get robot at index |
| PcbApi_CreateRobot | 03d58020 | Create new robot |
| PcbApi_DeleteRobot | 03d580a0 | Delete robot |
| PcbApi_GetRobotObjectHandleByName | 03d580a0 | Get robot by name |
| PcbApi_GetRobotNameAndCallback | 03d58110 | Get robot name and callback |
| PcbApi_RunEventHandler | 03d58130 | Run event handler |
| PcbApi_CreateEventHandler | 03d58150 | Create event handler |
| PcbApi_DestroyEventHandler | 03d582d0 | Destroy event handler |
| PcbApi_EventHandlerPerformanceIsPoor | 03d582f0 | Check handler performance |

### Undo/Redo

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_ClearUndoRedo | 03d52d20 | Clear undo/redo history |
| PcbApi_NewUndo | 03d52d30 | Begin new undo group |
| PcbApi_EndUndo | 03d52d40 | End undo group |
| PcbApi_DoUndo | 03d52d50 | Execute undo |
| PcbApi_DoRedo | 03d52d60 | Execute redo |

### DRC / Clearance Checking

| Function | Address | Description |
|----------|---------|-------------|
| PCBAPI_GetUnaryRuleForPrimitive | 03d52d60 | Get unary rule for primitive |
| PCBAPI_GetBinaryRuleForPrimitive | 03d52da0 | Get binary rule for pair |
| PCBAPI_CheckPrimitivesOverlapWithClearance | 03d52e00 | Check overlap with clearance |
| PCBAPI_ReleasePrimPrimDistanceChecker | 03d52e70 | Release distance checker |

### Export / Painter

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_CreatePainter | 03d58940 | Create painter for output |
| PcbApi_Export_ToPainter | 03d589d0 | Export object to painter |
| PcbApi_Export_ToPainter_ByHandle | 03d58af0 | Export by handle to painter |
| PcbApi_GetLayerPolygonShapesForOutput | 03d5a1c0 | Get layer polygon shapes for output |
| PcbApi_GetLayerPolygonShapesForOutputEx | 03d5a210 | Extended polygon shapes for output |

### Library Reader

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_CreateLibReader | 03d58bb0 | Create library file reader |
| PcbApi_GetLibReaderCount | 03d58e00 | Get count of items in lib reader |
| PcbApi_GetLibReaderPattern | 03d58e20 | Get library reader pattern |
| PcbApi_DestroyLibReader | 03d58eb0 | Destroy library reader |

### Net Analyzer / Trace

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_CreateNetAnalyzer | 03d594f0 | Create net analyzer |
| PcbApi_FreeNetAnalyzer | 03d59510 | Free net analyzer |
| PcbApi_RunNetAnalyzer | 03d59530 | Run net analyzer |
| PcbApi_GetTraceCount | 03d59550 | Get trace count |
| PcbApi_GetTrace | 03d59570 | Get trace |
| PcbApi_GetTraceCornerCount | 03d59590 | Get trace corner count |
| PcbApi_GetTraceCornerProperties | 03d59630 | Get trace corner properties |
| PcbApi_GetTraceCornerProperties_2 | 03d5a0a0 | Extended corner properties |
| PcbApi_GetTraceTerminations | 03d5a0f0 | Get trace terminations |

### Drill Manager

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_DrillManager_NeedsToUpdateClassifier | 03d5b570 | Check if classifier needs update |
| PcbApi_DrillManager_UpdateClassifierBegin | 03d5b5c0 | Begin classifier update |
| PcbApi_DrillManager_UpdateClassifierEnd | 03d5b630 | End classifier update |

### Miscellaneous

| Function | Address | Description |
|----------|---------|-------------|
| PcbApi_GetWaivedViolationCommentSingleton | 03d5a170 | Get waived violation comment |
| PcbApi_UpdateDiePadsForCurrentComponent | 03d5b900 | Update die pads |

---

## Object Type IDs

From `PcbApi_CreateObject` and `PcbApi_GetObjectIdFromObjectHandle`, the type check (`FUN_0469e1e0`) returns:

| ID (hex) | ID (dec) | Type | Evidence |
|----------|----------|------|----------|
| 0x01 | 1 | Arc | QueryArc checks `cVar1 == '\x01'` |
| 0x02 | 2 | Pad | QueryPad checks `cVar1 == '\x02'` |
| 0x03 | 3 | Via | QueryVia uses object ID 3 |
| 0x04 | 4 | Track | QueryTrack checks `cVar1 == '\x04'` |
| 0x05 | 5 | Text | QueryText checks `cVar1 == '\x05'` |
| 0x06 | 6 | Fill | QueryFill checks `cVar1 == '\x06'` |
| 0x07 | 7 | (Component Body?) | CreateObject case 7 |
| 0x08 | 8 | Net | QueryNet checks `cVar1 == '\x08'`; also used in container logic |
| 0x09 | 9 | Component | QueryComponent checks for '\x09'; ComponentUniqueId too |
| 0x0A | 10 | Class | CreateObject case 10 (0x0A) |
| 0x0B | 11 | Region | QueryRegion checks `cVar1 == '\x0b'` |
| 0x0D | 13 | Polygon | Used in iterator logic |
| 0x0E | 14 | Coordinate | QueryCoordinate checks `cVar1 == '\x0e'` |
| 0x10 | 16 | (Dimension?) | QueryObjectParameters special case for '\x10' |
| 0x11 | 17 | (Unknown 17) | CreateObject case 0x11 |
| 0x12 | 18 | (Unknown 18) | CreateObject case 0x12 |
| 0x14 | 20 | (Unknown 20) | CreateObject case 0x14 |
| 0x16 | 22 | (Embedded Board?) | CreateObject case 0x16; iterator checks |
| 0x19 | 25 | Board | QueryBoard checks `cVar1 == '\x19'`; CreateIterator too |

### CreateObject Factory Mapping

From `PcbApi_CreateObject` decompilation, `FUN_0469d2c0` is the factory (Delphi `TObject.Create`):

| param_1 | Type | VMT Address |
|---------|------|-------------|
| 1 | Arc | 0137d300 |
| 2 | Pad | 045c4070 |
| 3 | Via | 0462ae98 |
| 4 | Track | 0133ac80 |
| 5 | Text | 0128e250 |
| 6 | Fill | 0133ba00 |
| 7 | (type 7) | 013a8b50 |
| 8 | Net | 01379430 |
| 9 | Component | 0445d8b0 |
| 10 | Class | 013551d8 |
| 13 | (type 13/Dim?) | 019659c0 |
| 14 | (type 14) | 020e6f30 |
| 17 | (type 17) | 01a33e00 |
| 18 | (type 18) | 01f8efe0 |
| 20 | (type 20) | 017a5800 |
| 22 | (type 22) | 01361be0 |

### CreateDimensionObject Factory Mapping

| param_1 | Dimension Type | VMT Address |
|---------|---------------|-------------|
| 1 | Linear | 01963dc8 |
| 2 | Angular | 019641b0 |
| 3 | Radial | 019645c8 |
| 4 | Leader | 01964930 |
| 5 | Datum | 01964dd0 |
| 6 | Baseline | 01965028 |
| 7 | Center | 01965768 |
| 8 | (type 8/Ordinate?) | 019659c0 |
| 9 | (type 9) | 01965dc8 |
| 10 | (type 10) | 01965f58 |

---

## Layer ID Mapping

The massive switch statements in functions like `PcbApi_QueryVia`, `PcbApi_QueryComponent`, `PcbApi_QueryBoardLayerInfo`, and `PcbApi_QueryRule` all use the same byte-to-internal-layer conversion pattern. The layer byte IDs map to internal layer values via helper functions:

| Byte ID | Function | Layer |
|---------|----------|-------|
| 0x00 | FUN_00fd7ca0() | No Layer / Unknown (default) |
| 0x01 | FUN_00fd7dd0() | Top Layer |
| 0x02 | FUN_00fd7e10() | Mid Layer 1 |
| 0x03 | FUN_00fd7e50() | Mid Layer 2 |
| 0x04 | FUN_00fd7e90() | Mid Layer 3 |
| 0x05 | FUN_00fd7ed0() | Mid Layer 4 |
| 0x06 | FUN_00fd7f10() | Mid Layer 5 |
| 0x07 | FUN_00fd7f50() | Mid Layer 6 |
| 0x08 | FUN_00fd7f90() | Mid Layer 7 |
| 0x09 | FUN_00fd7fd0() | Mid Layer 8 |
| 0x0A | FUN_00fd8010() | Mid Layer 9 |
| 0x0B | FUN_00fd8050() | Mid Layer 10 |
| 0x0C | FUN_00fd8090() | Mid Layer 11 |
| 0x0D | FUN_00fd80d0() | Mid Layer 12 |
| 0x0E | FUN_00fd8110() | Mid Layer 13 |
| 0x0F | FUN_00fd8150() | Mid Layer 14 |
| 0x10 | FUN_00fd8190() | Mid Layer 15 |
| 0x11 | FUN_00fd81d0() | Mid Layer 16 |
| 0x12 | FUN_00fd8210() | Mid Layer 17 |
| 0x13 | FUN_00fd8250() | Mid Layer 18 |
| 0x14 | FUN_00fd8290() | Mid Layer 19 |
| 0x15 | FUN_00fd82d0() | Mid Layer 20 |
| 0x16 | FUN_00fd8310() | Mid Layer 21 |
| 0x17 | FUN_00fd8350() | Mid Layer 22 |
| 0x18 | FUN_00fd8390() | Mid Layer 23 |
| 0x19 | FUN_00fd83d0() | Mid Layer 24 |
| 0x1A | FUN_00fd8410() | Mid Layer 25 |
| 0x1B | FUN_00fd8450() | Mid Layer 26 |
| 0x1C | FUN_00fd8490() | Mid Layer 27 |
| 0x1D | FUN_00fd84d0() | Mid Layer 28 |
| 0x1E | FUN_00fd8510() | Mid Layer 29 |
| 0x1F | FUN_00fd8550() | Mid Layer 30 |
| 0x20 | FUN_00fd8590() | Bottom Layer |
| 0x21-0x26 | FUN_00fd85d0..8710 | Unknown (post-bottom copper) |
| 0x27 | FUN_00fda2a0(1) | Internal Plane 1 |
| 0x28 | FUN_00fda2a0(2) | Internal Plane 2 |
| 0x29 | FUN_00fda2a0(3) | Internal Plane 3 |
| 0x2A | FUN_00fda2a0(4) | Internal Plane 4 |
| ... | FUN_00fda2a0(N) | Internal Plane N |
| 0x36 | FUN_00fda2a0(0x10) | Internal Plane 16 |
| 0x37 | FUN_00fd8b50() | Top Overlay |
| 0x38 | FUN_00fd8b90() | Bottom Overlay |
| 0x39 | FUN_00fda410(1) | Mechanical 1 |
| 0x3A | FUN_00fda410(2) | Mechanical 2 |
| ... | FUN_00fda410(N) | Mechanical N |
| 0x48 | FUN_00fda410(0x10) | Mechanical 16 |
| 0x49 | FUN_00fd8fd0() | Top Solder |
| 0x4A | FUN_00fd9010() | Bottom Solder |
| 0x4B | FUN_00fd9050() | Top Paste |
| 0x4C | FUN_00fd9090() | Bottom Paste |
| 0x4D | FUN_00fd92d0() | Drill Guide |
| 0x4E | FUN_00fd9350() | Keep Out Layer |
| 0x4F | FUN_00fd9400() | Multi Layer |
| 0x50 | FUN_00fd9440() | Drill Drawing |
| 0x51 | FUN_00fd9480() | (Unknown 0x51) |
| 0x52 | FUN_00fd94c0() | (Unknown 0x52) |

The reverse conversion (`FUN_00fd9a10` + `FUN_00fd79c0`) converts internal layer back to byte ID.

---

## Common Parameter Patterns

### Get/Set Pattern (param_1 direction flag)

Almost all Query functions use the same get/set pattern:

```
param_1 == 0x00  -> SET mode (write values from caller to object)
param_1 == 0x01  -> GET mode (read values from object to caller)
param_1 == 0x02  -> SET mode with "commit" (same as 0x00 but auto-applies)
```

When `param_1 == '\x02'`, it is converted to `'\x00'` internally (same as SET), but may trigger additional side effects like `FUN_0469f280` (graphical invalidation/recalc).

### Common Primitive Fields

All primitives share common fields accessed through the same helper functions:

| Getter | Setter | Field |
|--------|--------|-------|
| FUN_0469da80 | FUN_0469d7d0 | Index/Selection (u16) |
| FUN_0469db40 | FUN_0469dba0 | Net (u16) |
| FUN_0469e260 | FUN_0469e290 | Component (u32) |
| FUN_0469ea90 | FUN_0469eab0 | Polygon ID / Net handle (u64) |
| FUN_046a0020 | FUN_046a0040 | Locked (u8/bool) |
| FUN_0469e6e0 | FUN_0469e700 | UserRouted / Attr1 (u64) |
| FUN_0469e6a0 | FUN_0469e6c0 | Attr2 (u64) - used by track/arc/fill |
| FUN_0469e720 | FUN_0469e740 | Attr3 (u64) - used by track/text |
| FUN_0469e760 | FUN_0469e780 | Attr4 (u64) - used by track/text |
| FUN_0469e1e0 | N/A | GetObjectTypeId (read-only) |
| FUN_0469f280 | N/A | GraphicallyInvalidate |
| FUN_0469f7e0 | N/A | GetDescription |
| FUN_0469f8f0 | N/A | ImportParameters (from ParamCollection) |
| FUN_0469f910 | N/A | ExportParameters (to ParamCollection) |

### Return Value Convention

All Query functions return 0 on success, 1 on failure (null handle or wrong type).

---

## Detailed Function Analysis

### Iterator Functions

#### PcbApi_CreateIterator (0x03d1fb90)

```
Handle PcbApi_CreateIterator(Handle container, char objectTypeFilter, bool processSpecialLayers, int layerFilter)
```

Creates an iterator for traversing PCB objects within a container. The container can be a board (type 0x19), component, or other container. The `objectTypeFilter` is a char (e.g., '\0' for all types, specific char for one type). The `layerFilter` parameter is an internal layer ID; if it differs from the "no layer" value (FUN_00fd7ca0), a layer filter string is built and applied.

For board-type containers (0x19), if the board has an embedded child document, it creates a special embedded board iterator. Otherwise, it creates a standard iterator via `FUN_046828e0`.

#### PcbApi_GetFirstObject / GetNextObject (0x03d1ff30 / 0x03d1ff00)

Simple vtable dispatchers:
- `GetFirstObject` calls vtable offset +0x20
- `GetNextObject` calls vtable offset +0x28

Returns object handle, or 0/null when iteration is complete.

#### PcbApi_DestroyIterator (0x03d1fe60)

Calls `FUN_00411b30` (Delphi `TObject.Free`) on the iterator handle.

### Object Factory Functions

#### PcbApi_CreateObject (0x03d22900)

```
Handle PcbApi_CreateObject(byte objectTypeId)
```

Factory function that creates PCB objects. Uses `FUN_0469d2c0` (Delphi class constructor) with VMT pointers for each type. Supports types 1-10, 13, 14, 17, 18, 20, 22. Returns 0 for unsupported types. See [Object Type IDs](#object-type-ids) for the mapping.

#### PcbApi_CreateDimensionObject (0x03d22b10)

```
Handle PcbApi_CreateDimensionObject(byte dimensionSubtype)
```

Creates dimension objects with subtypes 1-10. Uses the same factory pattern as CreateObject.

### Query Functions - Primitives

#### PcbApi_QueryPrimitive (0x03d254f0)

```
int PcbApi_QueryPrimitive(char mode, Handle obj, u16* index, u16* net, u32* component,
                           u64* netHandle, u64* attr1, u8* locked)
```

Gets/sets the 6 common primitive fields shared by all PCB objects. The `mode` parameter controls direction (see Common Parameter Patterns).

#### PcbApi_QueryTrack (0x03d2f240)

```
int PcbApi_QueryTrack(char mode, Handle obj, u16* index, u16* net, u32* component,
                       u64* netHandle, u8* locked, u64* attr1,
                       u32* x1, u32* y1, u32* x2, u32* y2, u32* width,
                       u64* attr2, u64* attr3, u64* attr4)
```

Track-specific fields (getters starting at FUN_0133b5d0):
- x1, y1, x2, y2: Start and end coordinates (int32, Altium internal units)
- width: Track width (int32)
- Plus 3 additional attribute fields (attr2/attr3/attr4)

Object type ID check: '\x04' (Track)

#### PcbApi_QueryArc (0x03d31e70)

```
int PcbApi_QueryArc(char mode, Handle obj, u16* index, u16* net, u32* component,
                     u64* netHandle, u8* locked, u64* attr1,
                     u32* centerX, u32* centerY, u32* radius,
                     u64* startAngle, u64* endAngle, u32* width, u64* attr2)
```

Arc-specific fields (getters starting at FUN_0137de50):
- centerX, centerY: Arc center coordinates
- radius: Arc radius
- startAngle, endAngle: Start/end angles (f64 as u64)
- width: Arc line width

Object type ID check: '\x01' (Arc)

#### PcbApi_QueryFill (0x03d31a20)

```
int PcbApi_QueryFill(char mode, Handle obj, u16* index, u16* net, u32* component,
                      u64* netHandle, u8* locked, u64* attr1,
                      u32* x1, u32* y1, u32* x2, u32* y2,
                      u64* attr2, u64* rotation)
```

Fill-specific fields (getters starting at FUN_01255900):
- x1, y1: Corner 1 coordinates
- x2, y2: Corner 2 coordinates
- rotation: via vtable call at +0x60

Object type ID check: '\x06' (Fill)

#### PcbApi_QueryRegion (0x03d31c20)

```
int PcbApi_QueryRegion(char mode, Handle obj, u8* regionKind, u16* index, u16* net,
                        u32* component, u64* netHandle, u8* locked,
                        u64* attr1, u64* attr2, u64* outlineData)
```

Region fields:
- regionKind: Region subtype (FUN_01352f00/FUN_01352f20)
- outlineData: Region outline/shape data (FUN_01352d80)
- For regions with associated polygon objects, queries child polygon info

Object type ID check: '\x0b' (Region)

#### PcbApi_QueryText (0x03d32090)

```
int PcbApi_QueryText(char mode, Handle obj, u16* index, u16* net, u32* component,
                      u64* netHandle, u8* locked, u64* attr1,
                      u32* x, u32* y, u32* height, u16* font,
                      u64* rotation, u8* mirrored, str* textValue, u32* width,
                      u64* attr3, u64* attr4)
```

Text-specific fields (getters starting at FUN_01293560):
- x, y: Text position
- height: Text height
- font: Font ID
- rotation: Text rotation angle
- mirrored: Mirror flag
- textValue: The actual text string (Delphi string, requires special handling)
- width: Text stroke width
- Has special TrueType vs. stroke font distinction (FUN_01294720)

Object type ID check: '\x05' (Text)

#### PcbApi_QueryPad (0x03d255f0)

The most complex query function with **37 parameters**!

```
int PcbApi_QueryPad(char mode, Handle obj,
    u16* index, u16* net, u32* component, u64* netHandle, u8* locked, u64* attr1,
    u32* x, u32* y,                    // Position
    u32* topXSize, u32* topYSize,      // Top layer pad size
    u32* midXSize, u32* midYSize,      // Mid layer pad size
    u32* botXSize, u32* botYSize,      // Bottom layer pad size
    u32* holeSize,                      // Hole diameter
    u8* topShape, u8* midShape, u8* botShape,  // Pad shapes
    str* padName,                       // Pad designator string
    u64* rotation,                      // Pad rotation
    u8* plated, u8* padMode,           // Plated flag, pad stack mode
    str* pasteMaskExpansion,            // Paste expansion
    str* solderMaskExpansion,           // Solder expansion
    u32* pasteMaskMode,                 // Paste mask expansion mode
    str* someString,                    // TBD
    u8* thermalReliefAngle,            // Thermal relief
    u8* connectionStyle,               // Connection style to planes
    u32* holeSizeSlot,                 // Slot hole size
    u64* someAttr,                     // TBD
    u32* cornerRadius1,                // Corner radius
    u32* cornerRadius2,                // Corner radius
    u32* cornerRadiusMode,             // CR mode
    u8* padJunctionType,               // Junction type
    u8* viaFence                       // Via fence flag
)
```

Object type ID check: '\x02' (Pad)

#### PcbApi_QueryVia (0x03d2f710)

Very large function (~8000+ bytes). Contains triple layer-conversion code for:
- param_5: Start layer byte -> internal layer
- param_13: End layer byte -> internal layer
- param_14: Third layer byte -> internal layer

The core query is delegated to helper function `FUN_03d2f490` which does the actual get/set of via fields.

After the helper call, the internal layer values are converted back to byte IDs via `FUN_00fd9a10` + `FUN_00fd79c0`.

### Query Functions - Board & Layers

#### PcbApi_QueryBoard (0x03d3bf40)

```
int PcbApi_QueryBoard(char mode, Handle boardObj, u64* docHandle, str* fileName,
                       u8* snapToGrid, u32* snapGridX, u32* snapGridY,
                       int* viewX, int* viewY, u8* boardType,
                       int* defaultLayer, u64* someAttr)
```

Object type ID check: '\x19' (Board)

Complex function with many COM interface vtable calls. Reads/writes board metadata, view state, snap grid settings, document handle, and filename. Uses multiple levels of COM interface queries (QueryInterface pattern visible in `FUN_004626d0`).

#### PcbApi_QueryBoardLayerInfo (0x03d3ccf0)

Takes a layer byte ID (param_3), converts it to internal layer value, then delegates to helper `FUN_03d3ca90` which reads/writes layer name, type, and other info.

### Query Functions - Components

#### PcbApi_QueryComponent (0x03d36340)

Has the standard layer-conversion prologue for the component layer parameter, then delegates to helper `FUN_03d35c70`.

#### PcbApi_QueryComponentEx (0x03d33530)

```
int PcbApi_QueryComponentEx(byte mode, Handle obj, u32* height, u8* designLocked,
                             u8* patternLocked, u32* channelOffset)
```

Gets/sets additional component properties:
- height: Component height (3D)
- designLocked / patternLocked: Lock flags
- channelOffset: Channel offset for multi-channel designs

Object type ID check: '\x09' (Component)

#### PcbApi_QueryComponentUniqueId (0x03d371d0)

```
int PcbApi_QueryComponentUniqueId(Handle out, Handle componentObj)
```

Checks if object is a component (type 9), then copies unique ID string.

### Query Functions - Rules

#### PcbApi_QueryRule (0x03d42fb0)

Massive function with layer-conversion prologues for two layer parameters (param_8 and param_9). Contains 25 parameters covering rule name, type, scope expressions, enabled state, layer restrictions, priority, and more. Delegates to helper `FUN_03d42e20`.

### Container Management

#### PcbApi_AddObjectToContainer (0x03d22f70)

```
int PcbApi_AddObjectToContainer(Handle container, Handle object)
```

Adds a PCB object to a container (board or component). Has different handling for:
- Board containers (type 0x19): If board has embedded child doc, wraps the object via COM interfaces. Otherwise calls `FUN_0470f110` directly.
- Primitive containers: Based on the container type, assigns the object to different fields:
  - Type 8 (Net): Sets via `FUN_0469eab0` (net handle field)
  - Type 9 (Component): Sets via `FUN_0469e700` (attr1 field)
  - Type 10/22 (Class/EmbeddedBoard): Sets via `FUN_0469e6c0` (attr2 field)

### Export / Painter Functions

#### PcbApi_CreatePainter (0x03d58940)

Creates a painter object for output/export operations. Uses `FUN_012024b0` to get the factory, then calls vtable +0x118 to create the painter with the provided options parameter.

#### PcbApi_Export_ToPainter (0x03d589d0)

```
void PcbApi_Export_ToPainter(Handle painter, Handle filename1, Handle filename2)
```

Exports data through a painter interface. Gets the painter's COM interface via `FUN_0041c5d0`, then calls vtable +0x18 with two filename/path parameters.

### Library Reader Functions

#### PcbApi_CreateLibReader (0x03d58bb0)

```
Handle PcbApi_CreateLibReader(str libraryPath)
```

Creates a library reader for a PCB library file. Uses `FUN_00fd41a0` to get the library manager, calls vtable +0x158 to get a document handle, then vtable +0x98 to open the library. Iterates through all items in the library (vtable +0x40 for count, +0x38 for item name at index) and collects them into a list object.

### Event & Robot Functions

The "Robot" API is Altium's internal plugin/automation system. Functions like `PcbApi_CreateRobot`, `PcbApi_CreateEventHandler`, etc. allow automation plugins to register callbacks for PCB editor events.

### Miscellaneous Functions

#### PcbApi_QueryObjectParameters (0x03d58ec0)

```
bool PcbApi_QueryObjectParameters(byte mode, Handle obj, Handle paramCollectionName)
```

Gets or sets object parameters via a named parameter collection. For mode flags < 8, it may directly import parameters (`FUN_0469f8f0`). Otherwise, it exports parameters first (`FUN_0469f910`), handles special dimension prefix formatting for type 0x10, then saves via vtable +0x58.

---

## Cross-Reference Patterns

### Common Helper Functions Used Across PcbApi

| Function | Purpose |
|----------|---------|
| FUN_0469d2c0 | Delphi TObject.Create (class factory) |
| FUN_0469e1e0 | GetObjectTypeId - returns object type byte |
| FUN_0469d7d0 / FUN_0469da80 | Set/Get Index (u16) |
| FUN_0469dba0 / FUN_0469db40 | Set/Get NetId (u16) |
| FUN_0469e290 / FUN_0469e260 | Set/Get Component ref (u32) |
| FUN_0469eab0 / FUN_0469ea90 | Set/Get NetHandle/PolygonId (u64) |
| FUN_046a0040 / FUN_046a0020 | Set/Get Locked flag (u8) |
| FUN_0469e700 / FUN_0469e6e0 | Set/Get UserRouted/Attr1 (u64) |
| FUN_0469e6c0 / FUN_0469e6a0 | Set/Get Attr2 (u64) |
| FUN_0469e740 / FUN_0469e720 | Set/Get Attr3 (u64) |
| FUN_0469e780 / FUN_0469e760 | Set/Get Attr4 (u64) |
| FUN_0469f280 | GraphicallyInvalidate (trigger redraw) |
| FUN_0469f8f0 | ImportParameters (from ParamCollection into object) |
| FUN_0469f910 | ExportParameters (from object to ParamCollection) |
| FUN_0469f7e0 | GetDescription (Delphi string result) |
| FUN_004174b0 | Delphi string copy (AnsiString) |
| FUN_004174e0 | Delphi string assign out |
| FUN_004152b0 | Delphi string clear |
| FUN_004152f0 | Delphi array/string cleanup |
| FUN_00415210 | Delphi string finalize |
| FUN_004153a0 | Delphi string AddRef |
| FUN_0041c540 | COM interface Release |
| FUN_0041c580 | COM interface assign |
| FUN_0041c5d0 | COM QueryInterface |
| FUN_0041c650 | COM AddRef |
| FUN_0041c980 | COM HRESULT check (OleCheck) |
| FUN_00411b30 | TObject.Free |
| FUN_00411fe0 | TObject.InheritsFrom / is-a check |
| FUN_004101a0 | Memory set (fills bytes) |
| FUN_00fd7ca0 | Get "No Layer" / default layer value |
| FUN_00fd9a10 | Internal layer to layer-info struct |
| FUN_00fd79c0 | Layer-info struct to byte ID |
| FUN_00fda2a0(N) | Get Internal Plane N layer |
| FUN_00fda410(N) | Get Mechanical N layer |

---

## Key Insights for altium-cli Implementation

1. **Object Type IDs** directly correspond to binary record types in PcbDoc files. The IDs match what's stored in the binary format.

2. **Layer Byte IDs (0x00-0x52)** are the API-level encoding. The internal representation uses a different struct-based format. The binary files likely store the byte IDs directly.

3. **The Get/Set pattern** (mode 0=SET, 1=GET, 2=SET+commit) means all Query functions are bidirectional. Our Rust implementation needs both read and write support for round-tripping.

4. **Parameter serialization** (`QueryObjectParameters`) goes through a `ParamCollection` (key=value pairs), which is the same format used in the ASCII parameter blocks within binary files.

5. **The factory** (`PcbApi_CreateObject`) shows the canonical list of PCB object types and their VMT addresses, which can be used to trace type-specific methods.

6. **Component source/vault links** (`QueryComponentSourceLinks`, `QueryComponentVaultLinks`) have dedicated complex query functions, suggesting these are stored as structured sub-records.

7. **Polygon segments** are stored in a flat array of 0x25 (37) byte records (`PcbApi_QueryPolygon` uses `param_18 + iVar4 * 0x25`).

8. **Pad stack** handling is extremely complex with per-layer shape, size, and corner radius support, reflecting the pad stack model in the binary format.

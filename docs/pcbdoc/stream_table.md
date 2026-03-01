# PcbDoc Section Name Table

Complete mapping of all PcbDoc CFB storage names, extracted from the Delphi binary
(`Altium.PCB.BinaryLoader.dll` in Ghidra project `altium26`) and cross-referenced with
the C# decompiled source (`AD26-dotnet/`).

## Sources

- **Delphi**: Section name table at `0x01bb4820`–`0x01bb5230` in `Altium.PCB.BinaryLoader.dll`
- **C#**: `TObjectId.cs`, `TStorageFeature.cs`, `Consts.cs` in `AD26-dotnet/Altium.Edp.Interfaces/`
- **C# master list**: PcbLib V6 storage names at `0x0186b700` (88 entries, superset of PcbDoc)
- **Delphi violation RTTI**: Violation section classes at `0x01993900`–`0x0199f300`

## CFB V3 Name Limit

OLE/CFB V3 limits storage names to **31 characters**. Names longer than 31 chars are
truncated in the actual CFB file. This affects several DRC violation storage names.

## Naming Conventions

PcbDoc V6 uses `Name6` suffix for most sections. PcbLib uses the same names without the
`6` suffix (e.g., `Arcs6` → `Arcs`). A few sections use the same name in both formats.

Key naming discrepancies between Delphi internal names and CFB storage names:

| Delphi Internal | CFB Storage | Note |
|-----------------|-------------|------|
| `Section_LetterGeometry` | `LettersGeometry` | Singular → plural, dropped "Section_" |
| `Section_CustomShape` | `CustomShapes` | Singular → plural |
| `Section_SharedUnions` | `SharedUnion` | Plural → singular (param-grouped format) |
| `Section_PrimitivesGUIDs` | `PrimitiveGuids` | Different casing and pluralization |
| `Section_PinPairs` | `PinPairsSection` | "Section" suffix instead of prefix |
| `TBackdrillViolation` (Delphi) | `TBackDrillViolation` (CFB) | Casing difference: lowercase d vs capital D |

---

## I. Primitive Sections (Binary Record Streams)

Binary PCB primitive records dispatched by `PcbObjectId`. Each section's `Data` stream
contains `[u8 object_id][u32 length][payload]` framed records.

| CFB Storage Name | Delphi Internal | Address | Object ID | Implemented |
|-----------------|-----------------|---------|-----------|-------------|
| `Arcs6` | `Section_Arcs` | 0x01bb4d15 | 1 (`eArcObject`) | Yes |
| `Pads6` | `Section_Pads` | 0x01bb4c7b | 2 (`ePadObject`) | Yes |
| `Vias6` | `Section_Vias` | 0x01bb4c6e | 3 (`eViaObject`) | Yes |
| `Tracks6` | `Section_Tracks` | 0x01bb4d22 | 4 (`eTrackObject`) | Yes |
| `Texts6` | `Section_Texts` | 0x01bb4c1f | 5 (`eTextObject`) | Yes |
| `Fills6` | `Section_Fills` | 0x01bb4d07 | 6 (`eFillObject`) | Yes |
| `Regions6` | `Section_Regions` | 0x01bb4c99 | 11 (`eRegionObject`) | Yes |
| `ShapeBasedRegions6` | `Section_Regions` | 0x01bb4c99 | 11 (variant) | Yes |
| `ComponentBodies6` | `Section_ComponentBody` | 0x01bb4c09 | 12 (`eComponentBodyObject`) | Yes |
| `ShapeBasedComponentBodies6` | `Section_ComponentBody` | 0x01bb4c09 | 12 (variant) | Yes |
| `BoardRegions` | `BoardRegionObj` | 0x01bb4877 | 11 (variant) | Yes |
| `Texts` | `Section_Texts` | 0x01bb4c1f | 5 (legacy) | Yes |
| `SplitPlaneRegions6` | — | — | 22 (`eSplitPlaneObject`) | No |

## II. Standard Parameter Sections

`|KEY=VALUE|` text parameter records with `[u32 length][payload]` framing.

### Core Sections

| CFB Storage Name | Delphi Internal | Address | Implemented |
|-----------------|-----------------|---------|-------------|
| `Board6` | `Section_Board` | 0x01bb4d31 | Yes |
| `Nets6` | `Section_Nets` | 0x01bb4cbc | Yes |
| `Components6` | `Section_Components` | 0x01bb4ca9 | Yes |
| `Polygons6` | `Section_Polygons` | 0x01bb4c88 | Yes |
| `Classes6` | `Section_Classes` | 0x01bb4bb8 | Yes |
| `DifferentialPairs6` | `Section_DifferentialPairs` | 0x01bb4cdd | Yes |
| `FromTos6` | `Section_FromTos` | 0x01bb4cf7 | Yes |
| `EmbeddedBoards6` | `Section_EmbeddedBoards` | 0x01bb4bde | Yes |
| `Embeddeds6` | `Section_Embeddeds` | 0x01bb4ba6 | Yes |
| `SmartUnions` | `Section_SmartUnions` | 0x01bb4ee8 | Yes |
| `WaivedViolations` | `Section_WaivedViolations` | 0x01bb4f31 | Yes |
| `SignalClasses` | `Section_SignalClasses` | 0x01bb4e74 | Yes |
| `PinPairsSection` | `Section_PinPairs` | 0x01bb4ed7 | Yes |

### Options Sections

| CFB Storage Name | Delphi Internal | Address | Implemented |
|-----------------|-----------------|---------|-------------|
| `Advanced Placer Options6` | `Section_AdvancedPlacer` | 0x01bb4c57 | Yes |
| `Advanced Router Options6` | `Section_AdvancedRouter` | — | Yes |
| `Design Rule Checker Options6` | `Section_DesignRuleChecker` | 0x01bb4c3d | Yes |
| `Pin Swap Options6` | `Section_PinSwap` | 0x01bb4c2d | Yes |

### Sidecar / Metadata Sections

| CFB Storage Name | Delphi Internal | Address | Implemented |
|-----------------|-----------------|---------|-------------|
| `UniqueIDPrimitiveInformation` | `Section_UniqueIDPrimitiveInformation` | 0x01bb4a2d | Yes |
| `ExtendedPrimitiveInformation` | `Section_ExtendedPrimitiveInformation` | 0x01bb4a52 | Yes |
| `PadViaLibrary` | `Section_PadViaLibrary` | 0x01bb4ade | Yes |
| `PadViaLibraryCache` | `Section_PadViaLibrary` | 0x01bb4ade | Yes |
| `PadViaLibraryLinks` | `Section_PadViaLibrary` | 0x01bb4ade | Yes |
| `FileVersionInfo` | `Section_FileVersionInfo` | 0x01bb4b54 | Yes |
| `Textures` | `Section_Textures` | 0x01bb4af4 | Yes |
| `ModelsNoEmbed` | `Section_Models` | 0x01bb4ab4 | Yes |
| `CustomShapes` | `Section_CustomShape` | 0x01bb4ee0 | Yes |
| `CustomMaskShapes` | `Section_CustomMaskShapes` | 0x01bb4929 | Yes |
| `CornerRadiusChamfer` | `Section_CornerRadiusChamfer` | 0x01bb48f5 | Yes |
| `ViaStructures` | `Section_ViaStructures` | 0x01bb4a17 | Yes |
| `ViaStructureManager` | `Section_ViaStructures` | 0x01bb4a17 | Yes |

## III. Prefixed Parameter Sections

`[u16 prefix][u32 length][|KEY=VALUE| payload]` framing.

| CFB Storage Name | Delphi Internal | Address | Implemented |
|-----------------|-----------------|---------|-------------|
| `Rules6` | `Section_Rules` | 0x01bb4b98 | Yes |
| `NewRules6` | `Section_Rules` | 0x01bb4b98 | Yes |
| `Dimensions6` | `Section_Dimensions` | 0x01bb4886 | Yes |
| `Coordinates6` | `Section_Coordinates` | 0x01bb4bf5 | Yes |

## IV. Binary Length-Prefixed Sections

Fixed-size binary records with `[u32 length][payload]` framing.

| CFB Storage Name | Delphi Internal | Address | Record Size | Implemented |
|-----------------|-----------------|---------|-------------|-------------|
| `Connections6` | `Section_Connections` | 0x01bb4cc9 | 43 bytes | Yes |

## V. Special Format Sections

Sections with unique parsing formats that don't fit the standard categories.

| CFB Storage Name | Delphi Internal | Format | Implemented |
|-----------------|-----------------|--------|-------------|
| `WideStrings6` | `Section_WideStrings` | `[u32 index][u32 byte_len][UTF-16LE]` entries | Yes |
| `UnionNames` | `Section_UnionNames` | `[u32 count]` then `[u32 index][u32 byte_len][UTF-16LE]` per entry | Yes |
| `UnionRelations` | `Section_UnionRelations` | Binary `[i32 parent][i32 child]` pairs | Yes |
| `UnionFeatures` | `Section_UnionFeatures` | `[u32 index][u32 len][param payload]` indexed records | Yes |
| `SharedUnion` | `Section_SharedUnions` | Grouped: header with `HIDDENPRIMITIVESCOUNT=N` + N detail blocks | Yes |
| `PrimitiveParameters` | `Section_PrimitiveParameters` | Grouped: component header with `COUNT=N` + N param blocks | Yes |
| `PrimitiveGuids` | `Section_PrimitivesGUIDs` | Per-primitive GUID entries | Yes |
| `Models` | `Section_Models` | Metadata + numbered STEP blobs (`/Models/{Header,Data,0,1,...}`) | Yes |
| `EmbeddedFonts6` | `Section_EmbeddedFonts` | Binary font data | Yes |
| `LayerKindMapping` | `Section_LayerKindMapping` | Version string + key-value mapping | Yes |
| `ConstraintManager` | `Section_ConstraintManager` | XML payload | Yes |
| `DrillManager` | `DrillManager.HoleSizeInfo.Serialize` | Binary drill symbol config | Yes |
| `LettersGeometry` | `Section_LetterGeometry` | TrueType glyph tessellation cache (3 streams) | Yes |

## VI. File Headers (Streams, Not Storages)

| Stream Path | Format | Implemented |
|-------------|--------|-------------|
| `/FileHeader` | Legacy UTF-16LE identification (24 bytes) | Yes |
| `/FileHeaderSix` | V6 pascal-block: version string + f64 + GUID (75 bytes) | Yes |

## VII. DRC Violation Sections

All DRC violation sections use standard parameter record format (`|KEY=VALUE|`).
The CFB storage name is the Delphi class name, truncated to 31 chars for CFB V3.

### Currently Implemented (38 types)

| CFB Storage Name | Delphi Class | Implemented |
|-----------------|--------------|-------------|
| `TAcuteAngleViolation` | `TAcuteAngleViolationSection` | Yes |
| `TBackDrillViolation` | `TBackdrillViolationSection` | Yes |
| `TBoardOutlineClearanceViolation` | `TBoardOutlineClearanceViolationSection` | Yes |
| `TClearanceViolation` | `TClearanceViolationSection` | Yes |
| `TComponentClearanceViolation` | `TComponentClearanceViolationSection` | Yes |
| `TCreepageViolation` | `TCreepageViolationSection` | Yes |
| `TDiffPairsViolation` | `TDiffPairsViolationSection` | Yes |
| `TDisconnectedSubnetsViolation` | `TDisconnectedSubnetsViolationSection` | Yes |
| `THoleToHoleViolation` | `THoleToHoleViolationSection` | Yes |
| `TMatchedNetLengthsViolation` | `TMatchedNetLengthsViolationSection` | Yes |
| `TMaximumViaCountViolation` | `TMaximumViaCountViolationSection` | Yes |
| `TMaxMinComponentHeightViolation` | `TMaxMinComponentHeightViolationSection` | Yes |
| `TMaxMinLengthViolation` | `TMaxMinLengthViolationSection` | Yes |
| `TMaxMinPadSlotWidthViolation` | `TMaxMinPadSlotWidthViolationSection` | Yes |
| `TMaxMinViaHoleSizeViolation` | `TMaxMinViaHoleSizeViolationSection` | Yes |
| `TMinimumAnnularRingViolation` | `TMinimumAnnularRingViolationSection` | Yes |
| `TMinSolderMaskSliverViolation` | `TMinSolderMaskSliverViolationSection` | Yes |
| `TMinWidthViolation` | `TMinWidthViolationSection` | Yes |
| `TModifiedPolygonViolation` | `TUnpouredPolygonViolationSection` | Yes |
| `TNetAntennaeViolation` | `TNetAntennaeViolationSection` | Yes |
| `TPadUnderSMDViolation` | `TPadUnderSMDViolationSection` | Yes |
| `TParallelSegmentViolation` | `TParallelSegmentViolationSection` | Yes |
| `TReturnPathViolation` | `TReturnPathViolationSection` | Yes |
| `TRoutingNeckDownViolation` | `TRoutingNeckDownViolationSection` | Yes |
| `TRoutingViaStyleViolation` | `TRoutingViaStyleViolationSection` | Yes |
| `TShortCircuitViolation` | `TShortCircuitViolationSection` | Yes |
| `TSilkToBoardRegionClearanceViol` | `TSilkToBoardRegionClearanceViolationSection` | Yes (truncated) |
| `TSilkToSilkClearanceViolation` | `TSilkToSilkClearanceViolationSection` | Yes |
| `TSilkToSolderMaskClearanceViola` | `TSilkToSolderMaskClearanceViolationSection` | Yes (truncated) |
| `TSMDNeckDownViolation` | `TSMDNeckDownViolationSection` | Yes |
| `TSMDPADEntryViolation` | `TSMDPADEntryViolationSection` | Yes |
| `TSMDToCornerViolation` | `TSMDToCornerViolationSection` | Yes |
| `TTestPointViolation` | `TTestPointViolationSection` | Yes |
| `TUnconnectedPinViolation` | `TUnconnectedPinViolationSection` | Yes |
| `TViaUnderSMDViolation` | `TViaUnderSMDViolationSection` | Yes |
| `TWirebondLengthViolation` | `TWirebondLengthViolationSection` | Yes |
| `TWirebondWireToWireViolation` | `TWirebondWireToWireViolationSection` | Yes |
| `TZAxisClearanceViolation` | `TZAxisClearanceViolationSection` | Yes |

### Not Yet Implemented (additional violation types from Delphi RTTI)

| CFB Storage Name | Delphi Class | CFB Truncated? |
|-----------------|--------------|----------------|
| `TComponentClearance1Violation` | `TComponentClearance1ViolationSection` | No (29 chars) |
| `TDaisyChainStubViolation` | `TDaisyChainStubViolationSection` | No |
| `TEmptyParallelSegmentViolatio` | `TEmptyParallelSegmentViolationSection` | Yes (38→31) |
| `TMaxMinCounterHoleSizeViolati` | `TMaxMinCounterHoleSizeViolationSection` | Yes (39→31) |
| `TMaxMinPadRndHoleSizeViolat` | `TMaxMinPadRoundHoleSizeViolationSection` | Yes (40→31) |
| `TMaxMinPadSlotHeightViolation` | `TMaxMinPadSlotHeightViolationSection` | No (30 chars) |
| `TMaxMinPadSqHoleSizeViolation` | `TMaxMinPadSquareHoleSizeViolationSection` | Yes (41→31) |
| `TMaxWidthArcViolation` | `TMaxWidthArcViolationSection` | No |
| `TMaxWidthTrackViolation` | `TMaxWidthTrackViolationSection` | No |
| `TMinWidthStubArcViolation` | `TMinWidthStubArcViolationSection` | No |
| `TMinWidthStubFillViolation` | `TMinWidthStubFillViolationSection` | No |
| `TMinWidthStubPadViolation` | `TMinWidthStubPadViolationSection` | No |
| `TMinWidthStubTrackViolation` | `TMinWidthStubTrackViolationSection` | No |
| `TMinWidthStubViaViolation` | `TMinWidthStubViaViolationSection` | No |
| `TPadLayerPairsViolation` | `TPadLayerPairsViolationSection` | No |
| `TRoomConfinementViolation` | `TRoomConfinementViolationSection` | No |
| `TRoutingTopologyViolation` | `TRoutingTopologyViolationSection` | No |
| `TSMDToPlaneViolation` | `TSMDToPlaneViolationSection` | No |
| `TStarvedThermalViolation` | `TStarvedThermalViolationSection` | No |
| `TUnplatedPadViolation` | `TUnplatedPadViolationSection` | No |
| `TUnpouredPolygonViolation` | `TUnpouredPolygonViolationSection` | No |
| `TViaLayerPairsViolation` | `TViaLayerPairsViolationSection` | No |
| `TWirebondMarginViolation` | `TWirebondMarginViolationSection` | No |

### Testpoint Violation Sub-Types (16 types, none implemented)

These are sub-types of `TTestPointViolation`, each with its own CFB storage.

| CFB Storage Name | Delphi Class |
|-----------------|--------------|
| `TTPUIllegalTentedViolation` | `TTPUIllegalTentedViolationSection` |
| `TTPUMissingStyleRuleViolation` | `TTPUMissingStyleRuleViolationSection` |
| `TTPUMissingSingleTPViolation` | `TTPUMissingSingleTPViolationSection` |
| `TTPUIllegalTPViolation` | `TTPUIllegalTPViolationSection` |
| `TTPUMissingLeafTPViolation` | `TTPUMissingLeafTPViolationSection` |
| `TTPUIllegalNonLeafTPViolation` | `TTPUIllegalNonLeafTPViolationSection` |
| `TTPSMinMaxHoleSizeViolation` | `TTPSMinMaxHoleSizeViolationSection` |
| `TTPSMinMaxShapeSizeViolation` | `TTPSMinMaxShapeSizeViolationSection` |
| `TTPSIllegalTPSideViolation` | `TTPSIllegalTPSideViolationSection` |
| `TTPSIllegalObjSideViolation` | `TTPSIllegalObjSideViolationSection` |
| `TTPSOffGridViolation` | `TTPSOffGridViolationSection` |
| `TTPSIllegalUnderCompViolation` | `TTPSIllegalUnderCompViolationSection` |
| `TTPSMinSpacingViolation` | `TTPSMinSpacingViolationSection` |
| `TTPSCompBodyClrncViolation` | `TTPSCompBodyClrncViolationSection` |
| `TTPSBoardEdgeClrncViolation` | `TTPSBoardEdgeClrncViolationSection` |

## VIII. Sections Not Yet Observed in Test Files

These sections exist in the Delphi binary / C# master list but have not been
encountered in any test file in `data/pcbdoc/`. They may require specific Altium
features to be enabled (see TStorageFeature below).

| CFB Storage Name | Delphi Internal | Category |
|-----------------|-----------------|----------|
| `SplitPlaneRegions6` | — | Primitive (split plane) |
| `3DRoutingData` | `Section_3DRouting` | 3D routing |
| `3DRoutingXYZData` | — | 3D routing |
| `3DRoutingSurfaceData` | — | 3D routing |
| `3DRoutingSketchesData` | — | 3D routing |
| `MechanicalPrimitives` | `Section_MechanicalPrimitives` | Mech layer primitives |
| `CounterHolesSection` | `Section_CounterHoles` | Counter-bore/sink holes |
| `CounterHolesPresetsSection` | — | Counter-hole presets |
| `LayerToLayerMapping` | `Section_LayerToLayerMapping` | Layer mapping |
| `CustomReliefs` | `Section_CustomRelief` | Custom thermal relief |
| `RuleAdditionalData` | `Section_RuleAdditionalData` | Rule extensions |
| `xNetClassesSection` | `Section_xNetClasses` | xNet classes |
| `Wirebonds` | `Section_Wirebond` | Wirebonding |
| `WirebondTemplates` | — | Wirebond templates |
| `WirebondBodies` | — | Wirebond bodies |
| `DiePadsInfo` | `Section_DiePadsInfo` | Die pad info |
| `RegionHoles` | — | Region hole data |
| `ViaInstancing` | `Section_ViaInstance` | Via instancing |
| `ExtendedPrimitiveIndices` | `Section_ExtendedPrimitiveIndices` | Extended indices |
| `LayerStackSection` | — | Layer stack |
| `Testpoint Options` | — | Testpoint config |
| `SimbeorCacheSection` | `Section_SimberianCache` | Simberian SI cache |
| `ZAxisClearanceCache` | `Section_ZAxisClearanceCache` | Z-axis cache |
| `ConnectivityGraphCache` | — | Connectivity cache |
| `ComponentCache` | — | Component cache |
| `GeometryZeroCache` | — | Geometry cache |
| `PadViaCacheLibraryLinksSection` | — | PadVia cache links |

## IX. TStorageFeature Flags

From `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TStorageFeature.cs`. These flags in
`Board6` control which sections are written during save.

| Value | Feature Flag | Controls Section(s) |
|-------|-------------|---------------------|
| 0 | `eHasImpedanceProfileCount` | Board6 impedance data |
| 1 | `eHasPrintedElectronicLayers` | Board6 PE layers |
| 2 | `eHasMicroVias` | ViaStructures |
| 3 | `eHasCustomThermalReliefsAtWriteStage` | CustomReliefs |
| 4 | `eHasSystemParametersAtWriteStage` | System parameters |
| 5 | `eHasShapeBasedRegions` | ShapeBasedRegions6 |
| 6 | `eHasShapeBasedCompBodies` | ShapeBasedComponentBodies6 |
| 7 | `eHasRF20IsUsedAtWriteStage` | RF features |
| 8 | `eHasIPC4761ViaTypesAtWriteStage` | Via types |
| 9 | `eHasCustomPadShapesAtWriteStage` | CustomShapes |
| 10 | `eHasRotatedAnyAngleEmbeddedBoardArrayAtWriteStage` | EmbeddedBoards |
| 11 | `eHasFootprintParametersAtWriteStage` | PrimitiveParameters |
| 12 | `eHasCustomReliefInfosAtWriteStage` | CustomReliefs |
| 13 | `eHasClearanceByLayerRuleAtWriteStage` | Layer rules |
| 14 | `eHasMatrixRuleAtWriteStage` | Matrix rules |
| 15 | `eHasTHPadPasteInfosAtWriteStage` | Paste info |
| 16 | `eHasCustomMaskInfosAtWriteStage` | CustomMaskShapes |
| 17 | `eHasPolygonsWithNeckWidthFromRule` | Polygon neck width |
| 18 | `eHasNeckDownRuleAtWriteStage` | Neck down rules |
| 19 | `eHasSingleLayerModeAtWriteStage` | Single layer mode |
| 20 | `eHasCustomPadShapesDonutAtWriteStage` | Donut pad shapes |
| 21 | `eHasWirebondAtWriteStage` | Wirebonds, WirebondTemplates, WirebondBodies |
| 22 | `eHasDiffpairPhaseMatching` | DifferentialPairs phase |
| 23 | `eHasExtendedGroupIndicesAreUsed` | Extended group indices |
| 24 | `eHasIncreasedSignalLayers` | Signal layer count |
| 25 | `eHasZAxisClearanceRuleAtWriteStage` | ZAxisClearanceCache |

## X. TObjectId Enum

The `u8` type byte used to dispatch binary primitive records. Shared between PcbDoc
and PcbLib.

| Value | Enum | Display Name | Has Section |
|-------|------|-------------|-------------|
| 0 | `eNoObject` | NoObject | — |
| 1 | `eArcObject` | Arc | Arcs6 |
| 2 | `ePadObject` | Pad | Pads6 |
| 3 | `eViaObject` | Via | Vias6 |
| 4 | `eTrackObject` | Track | Tracks6 |
| 5 | `eTextObject` | Text | Texts6, Texts |
| 6 | `eFillObject` | Fill | Fills6 |
| 7 | `eConnectionObject` | Connection | Connections6 |
| 8 | `eNetObject` | Net | Nets6 |
| 9 | `eComponentObject` | Component | Components6 |
| 10 | `ePolyObject` | Poly | Polygons6 |
| 11 | `eRegionObject` | PolyRegion | Regions6, ShapeBasedRegions6, BoardRegions |
| 12 | `eComponentBodyObject` | ComponentBody | ComponentBodies6, ShapeBasedComponentBodies6 |
| 13 | `eDimensionObject` | Dimension | Dimensions6 |
| 14 | `eCoordinateObject` | Coordinate | Coordinates6 |
| 15 | `eClassObject` | Class | Classes6 |
| 16 | `eRuleObject` | Rule | Rules6, NewRules6 |
| 17 | `eFromToObject` | FromTo | FromTos6 |
| 18 | `eDifferentialPairObject` | DifferentialPair | DifferentialPairs6 |
| 19 | `eViolationObject` | Violation | T*Violation sections |
| 20 | `eEmbeddedObject` | Embedded | Embeddeds6 |
| 21 | `eEmbeddedBoardObject` | EmbeddedBoard | EmbeddedBoards6 |
| 22 | `eSplitPlaneObject` | SplitPlane | SplitPlaneRegions6 |
| 23 | `eTraceObject` | Trace | — (interactive routing) |
| 24 | `eSpareViaObject` | SpareVia | — (interactive routing) |
| 25 | `eBoardObject` | Board | Board6 |
| 26 | `eBoardOutlineObject` | BoardOutline | — (within Board6) |

## XI. Summary Statistics

| Category | Total | Implemented |
|----------|-------|-------------|
| Primitive sections | 13 | 12 |
| Standard parameter sections | 30 | 30 |
| Prefixed parameter sections | 4 | 4 |
| Binary length-prefixed sections | 1 | 1 |
| Special format sections | 13 | 13 |
| File headers | 2 | 2 |
| DRC violation sections (implemented) | 38 | 38 |
| DRC violation sections (not implemented) | 23 | 0 |
| Testpoint violation sub-types | 15 | 0 |
| Not yet observed sections | 27 | 0 |
| **Total known sections** | **~166** | **~100** |

# PcbDoc Enumerations and Constants

All enumerations and constants used by PcbDoc binary records. These are shared with PcbLib
(see `docs/pcblib/enumerations.md` for the PcbLib-specific perspective).

## TObjectId (Primitive Type Discriminant)

The object ID byte is the first field in every binary primitive record. Two variants exist
in the .NET source with the same numeric values but different names at indices 7 and 17.

### Pcbtypes.TObjectId (storage/canonical)

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TObjectId.cs`

| Value | C# Name | Rust (`PcbObjectId`) | Description |
|-------|---------|---------------------|-------------|
| 0 | `eIgnoreObject` | `NoObject` | Null/sentinel |
| 1 | `eArcObject` | `Arc` | Circular arc |
| 2 | `ePadObject` | `Pad` | Component pad |
| 3 | `eViaObject` | `Via` | Plated through-hole via |
| 4 | `eTrackObject` | `Track` | Routed line segment |
| 5 | `eTextObject` | `Text` | Text string |
| 6 | `eFillObject` | `Fill` | Solid rectangular fill |
| 7 | `eFromToObject` | `Connection` | Connection/ratsnest endpoint |
| 8 | `eNetObject` | `Net` | Net grouping |
| 9 | `eComponentObject` | `Component` | Component (footprint instance) |
| 10 | `ePolygonObject` | `Polygon` | Copper pour polygon |
| 11 | `eRegionObject` | `Region` | Region (copper, cutout, cavity) |
| 12 | `eComponentBodyObject` | `ComponentBody` | 3D body attached to component |
| 13 | `eDimensionObject` | `Dimension` | Dimension annotation |
| 14 | `eCoordinateObject` | `Coordinate` | Coordinate annotation |
| 15 | `eClassObject` | `Class` | Object class definition |
| 16 | `eRuleObject` | `Rule` | Design rule definition |
| 17 | `eManualFromToObject` | `FromTo` | Manual FromTo definition |
| 18 | `eDifferentialPairObject` | `DifferentialPair` | Differential pair definition |
| 19 | `eViolationObject` | `Violation` | DRC violation marker |
| 20 | `eEmbeddedObject` | `Embedded` | Embedded object (generic) |
| 21 | `eEmbeddedBoardObject` | `EmbeddedBoard` | Embedded board panel |
| 22 | `eSplitPlaneObject` | `SplitPlane` | Split plane region |
| 23 | `eTraceObject` | `Trace` | Trace (routed path group) |
| 24 | `eSpareViaObject` | `SpareVia` | Spare via |
| 25 | `eBoardObject` | `Board` | Board document root |
| 26 | `eBoardOutlineObject` | `BoardOutline` | Board outline shape |

### RT_PCB.TObjectId (runtime)

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TObjectId.cs`

Identical numeric values, but two naming differences:

| Index | Pcbtypes Name | RT_PCB Name | Notes |
|-------|--------------|-------------|-------|
| 0 | `eIgnoreObject` | `eNoObject` | Different sentinel name |
| 7 | `eFromToObject` | `eConnectionObject` | Ratsnest endpoint |
| 10 | `ePolygonObject` | `ePolyObject` | Shortened name |
| 17 | `eManualFromToObject` | `eFromToObject` | Name swapped with index 7 |

**Constants from `RT_PCB.Consts`:**
- `FirstObjectId = eArcObject` (1)
- `LastObjectId = eSplitPlaneObject` (22) -- objects 23-26 not iterable via this range

**Rust implementation:** `altium_format_types::pcb::PcbObjectId`

---

## TAdvPCBFileFormatVersion

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TAdvPCBFileFormatVersion.cs`

| Value | C# Name | Rust (`PcbFileFormatVersion`) | Description |
|-------|---------|------------------------------|-------------|
| 0 | `ePCBFileFormatNone` | `None` | Unknown/invalid |
| 1 | `eAdvPCBFormat_Binary_V3` | `BinaryV3` | Protel 99 binary |
| 2 | `eAdvPCBFormat_Library_V3` | `LibraryV3` | Protel 99 library |
| 3 | `eAdvPCBFormat_ASCII_V3` | `AsciiV3` | Protel 99 ASCII |
| 4 | `eAdvPCBFormat_Binary_V4` | `BinaryV4` | DXP binary |
| 5 | `eAdvPCBFormat_Library_V4` | `LibraryV4` | DXP library |
| 6 | `eAdvPCBFormat_ASCII_V4` | `AsciiV4` | DXP ASCII |
| 7 | `eAdvPCBFormat_Binary_V5` | `BinaryV5` | Altium Designer binary |
| 8 | `eAdvPCBFormat_Library_V5` | `LibraryV5` | Altium Designer library |
| 9 | `eAdvPCBFormat_ASCII_V5` | `AsciiV5` | Altium Designer ASCII |
| 10 | `eAdvPCBFormat_Binary_V6` | `BinaryV6` | Modern AD binary (PcbDoc) |
| 11 | `eAdvPCBFormat_Library_V6` | `LibraryV6` | Modern AD library (PcbLib) |
| 12 | `eAdvPCBFormat_ASCII_V6` | `AsciiV6` | Modern AD ASCII |
| 13 | `eAdvPCBFormat_Binary_V6_CS` | `BinaryV6CS` | CircuitStudio variant |
| 14 | `eAdvPCBFormat_Binary_V6_CM` | `BinaryV6CM` | CircuitMaker variant |
| 15 | `eAdvPCBFormat_Binary_V6_PCBWorks` | `BinaryV6PCBWorks` | PCBWorks variant |
| 16 | `eAdvPCBFormat_PadViaLibrary_V6` | `PadViaLibraryV6` | Pad/Via library |

**PcbDoc targets:** `BinaryV6` (10) for board documents, `LibraryV6` (11) for footprint libraries.

**Rust implementation:** `altium_format_types::pcb::PcbFileFormatVersion`

---

## TStorageFeature

Feature flags stored as a bitset in the Board6 section header. Each flag indicates the
presence of optional data sections or format extensions.

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TStorageFeature.cs`

| Value | C# Name | Description |
|-------|---------|-------------|
| 0 | `eHasImpedanceProfileCount` | Impedance profile data present |
| 1 | `eHasPrintedElectronicLayers` | Printed electronics layers |
| 2 | `eHasMicroVias` | Micro-via support |
| 3 | `eHasCustomThermalReliefsAtWriteStage` | Custom thermal relief data |
| 4 | `eHasSystemParametersAtWriteStage` | System parameter block |
| 5 | `eHasShapeBasedRegions` | Shape-based region format |
| 6 | `eHasShapeBasedCompBodies` | Shape-based component bodies |
| 7 | `eHasRF20IsUsedAtWriteStage` | RF 2.0 feature used |
| 8 | `eHasIPC4761ViaTypesAtWriteStage` | IPC-4761 via structure types |
| 9 | `eHasCustomPadShapesAtWriteStage` | Custom pad shapes present |
| 10 | `eHasRotatedAnyAngleEmbeddedBoardArrayAtWriteStage` | Rotated embedded board arrays |
| 11 | `eHasFootprintParametersAtWriteStage` | Footprint parameters section |
| 12 | `eHasCustomReliefInfosAtWriteStage` | Custom relief info data |
| 13 | `eHasClearanceByLayerRuleAtWriteStage` | Layer-specific clearance rules |
| 14 | `eHasMatrixRuleAtWriteStage` | Matrix-style clearance rule |
| 15 | `eHasTHPadPasteInfosAtWriteStage` | Through-hole pad paste info |
| 16 | `eHasCustomMaskInfosAtWriteStage` | Custom mask expansion data |
| 17 | `eHasPolygonsWithNeckWidthFromRule` | Polygon neck width from rule |
| 18 | `eHasNeckDownRuleAtWriteStage` | Neck-down rule data |
| 19 | `eHasSingleLayerModeAtWriteStage` | Single layer mode data |
| 20 | `eHasCustomPadShapesDonutAtWriteStage` | Donut pad shape |
| 21 | `eHasWirebondAtWriteStage` | Wirebond data |
| 22 | `eHasDiffpairPhaseMatching` | Diff-pair phase matching |
| 23 | `eHasExtendedGroupIndicesAreUsed` | Extended group indices (>255) |
| 24 | `eHasIncreasedSignalLayers` | Extended signal layers (>32) |
| 25 | `eHasZAxisClearanceRuleAtWriteStage` | Z-axis clearance rule |

---

## Layer IDs

### TV6_Layer (Binary V6 Layer Byte)

The V6 layer byte is a `u8` stored in every primitive record's layer field. This is the
encoding used in the binary file format.

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TV6_Layer.cs`

#### Signal Layers (0-32)

| Value | C# Name | Layer Name | Description |
|-------|---------|-----------|-------------|
| 0 | `eV6_NoLayer` | NoLayer | Null/unassigned |
| 1 | `eV6_TopLayer` | TopLayer | Top copper |
| 2-31 | `eV6_MidLayer1`..`eV6_MidLayer30` | MidLayer1..30 | Mid signal layers |
| 32 | `eV6_BottomLayer` | BottomLayer | Bottom copper |

#### Mask Layers (33-38)

| Value | C# Name | Layer Name | Description |
|-------|---------|-----------|-------------|
| 33 | `eV6_TopOverlay` | TopOverlay | Top silkscreen |
| 34 | `eV6_BottomOverlay` | BottomOverlay | Bottom silkscreen |
| 35 | `eV6_TopPaste` | TopPaste | Top paste mask |
| 36 | `eV6_BottomPaste` | BottomPaste | Bottom paste mask |
| 37 | `eV6_TopSolder` | TopSolder | Top solder mask |
| 38 | `eV6_BottomSolder` | BottomSolder | Bottom solder mask |

#### Internal Plane Layers (39-54)

| Value | C# Name | Layer Name |
|-------|---------|-----------|
| 39-54 | `eV6_InternalPlane1`..`eV6_InternalPlane16` | InternalPlane1..16 |

#### Drill & Keepout Layers (55-56)

| Value | C# Name | Layer Name |
|-------|---------|-----------|
| 55 | `eV6_DrillGuide` | DrillGuide |
| 56 | `eV6_KeepOutLayer` | KeepOutLayer |

#### Mechanical Layers (57-72)

| Value | C# Name | Layer Name |
|-------|---------|-----------|
| 57-72 | `eV6_Mechanical1`..`eV6_Mechanical16` | Mechanical1..16 |

#### Special Layers (73-82)

| Value | C# Name | Layer Name | Description |
|-------|---------|-----------|-------------|
| 73 | `eV6_DrillDrawing` | DrillDrawing | Drill drawing output |
| 74 | `eV6_MultiLayer` | MultiLayer | All copper layers |
| 75 | `eV6_ConnectLayer` | ConnectLayer | Ratsnest display |
| 76 | `eV6_BackGroundLayer` | BackGroundLayer | Background display |
| 77 | `eV6_DRCErrorLayer` | DRCErrorLayer | DRC error markers |
| 78 | `eV6_HighlightLayer` | HighlightLayer | Highlight display |
| 79 | `eV6_GridColor1` | GridColor1 | Grid display |
| 80 | `eV6_GridColor10` | GridColor10 | Fine grid display |
| 81 | `eV6_PadHoleLayer` | PadHoleLayer | Pad hole display |
| 82 | `eV6_ViaHoleLayer` | ViaHoleLayer | Via hole display |

**Rust implementation:** `altium_format_types::pcb::V6Layer`

### TLayerConstant (V7 Named Layer Constants)

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TLayerConstant.cs`

Same mapping as TV6_Layer bytes 0-82, plus additional symbolic entries:

| Value | Name | Description |
|-------|------|-------------|
| 83 | `cDRCDetailLayer` | DRC detail overlay |
| 84-90 | `cHighlightLayer`..`cBottomPadMasterPlot` | System layers |
| 91 | `cV7_MidLayers` | Symbolic: all mid layers |
| 92 | `cAllLayers` | Symbolic: all layers |
| 93 | `cSignalLayers` | Symbolic: signal layers only |
| 94 | `cInternalPlaneLayers` | Symbolic: internal planes only |
| 95 | `cElectricalLayers` | Symbolic: all electrical |
| 96 | `cMechanicalLayers` | Symbolic: all mechanical |
| 97 | `cDielectricLayers` | Symbolic: dielectric only |

### V7Layer (Extended 32-bit Layer ID)

V7 uses a 32-bit structured layer ID for extended layer support:

```
Byte 0-1 (u16): Species (layer-specific index)
Byte 2   (u8):  Genus (layer category)
Byte 3   (u8):  Family (0=misc, 1=electrical, 2=dielectric, 4=mechanical)
```

When genus=0 and family=0, species low byte matches the V6 layer byte (backward-compatible).

**Layer family constants from `RT_PCB.Consts`:**

| Value | Constant | Description |
|-------|----------|-------------|
| 0 | `MISC_LAYER_FAMILY` | Miscellaneous layers |
| 1 | `ELECTRICAL_LAYER_FAMILY` | Signal/plane layers |
| 2 | `DIELECTRIC_LAYER_FAMILY` | Dielectric layers |
| 4 | `MECHANICAL_LAYER_FAMILY` | Mechanical layers |

**Layer flag constants:**

| Value | Constant | Description |
|-------|----------|-------------|
| 0 | `STANDARD_LAYER_FLAGS` | Standard layer |
| 1 | `EXTENDED_LAYER_FLAGS` | Extended format |
| 257 | `SIGNAL_LAYER_FLAGS` | Signal layer |
| 258 | `MID_LAYER_FLAGS` | Mid signal layer |
| 260 | `INTERNAL_PLANE_LAYER_FLAGS` | Internal plane |
| 513 | `DIELECTRIC_LAYER_FLAGS` | Dielectric layer |
| 1024 | `MECHANICAL_LAYER_FLAGS` | Mechanical layer |

**Layer count limits:**

| Constant | Value | Description |
|----------|-------|-------------|
| `MAX_SIGNAL_LAYER` | 65535 | Max signal layers (extended) |
| `MAX_MID_LAYER` | 126 | Max mid signal layers |
| `MAX_INTERNAL_PLANE_LAYER` | 65535 | Max internal planes (extended) |
| `MAX_MECHANICAL_LAYER` | 65535 | Max mechanical layers (extended) |
| `MAX_MECHANICAL_LAYER_LEGACY` | 32 | Legacy mechanical layer limit |
| `MAX_MECHANICAL_LAYER_UNLIM` | 1024 | Unlimited mode limit |
| `MAX_SIGNAL_LAYER_S09` | 32 | Pre-S09 signal layer limit |
| `MAX_MID_LAYER_S09` | 30 | Pre-S09 mid layer limit |
| `MAX_INTERNAL_PLANE_LAYER_S09` | 16 | Pre-S09 internal plane limit |

**Rust implementation:** `altium_format_types::pcb::V7Layer`

### TMechanicalLayerKind

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TMechanicalLayerKind.cs`

Classifies the purpose of each mechanical layer:

| Value | C# Name | Description |
|-------|---------|-------------|
| 0 | `mlUndefined` | No assigned kind |
| 1 | `mlAssemblyTop` | Top assembly drawing |
| 2 | `mlAssemblyBottom` | Bottom assembly drawing |
| 3 | `mlAssemblyNotes` | Assembly notes |
| 4 | `mlBoard` | Board outline |
| 5 | `mlCoatingTop` | Top conformal coating |
| 6 | `mlCoatingBottom` | Bottom conformal coating |
| 7 | `mlComponentCenterTop` | Top component centers |
| 8 | `mlComponentCenterBottom` | Bottom component centers |
| 9 | `mlComponentOutlineTop` | Top component outlines |
| 10 | `mlComponentOutlineBottom` | Bottom component outlines |
| 11 | `mlCourtyardTop` | Top courtyard |
| 12 | `mlCourtyardBottom` | Bottom courtyard |
| 13 | `mlDesignatorTop` | Top designators |
| 14 | `mlDesignatorBottom` | Bottom designators |
| 15 | `mlDimensions` | Dimensions |
| 16 | `mlDimensionsTop` | Top dimensions |
| 17 | `mlDimensionsBottom` | Bottom dimensions |
| 18 | `mlFabNotes` | Fabrication notes |
| 19 | `mlGluePointsTop` | Top glue points |
| 20 | `mlGluePointsBottom` | Bottom glue points |
| 21 | `mlGoldPlatingTop` | Top gold plating |
| 22 | `mlGoldPlatingBottom` | Bottom gold plating |
| 23 | `mlValueTop` | Top value labels |
| 24 | `mlValueBottom` | Bottom value labels |
| 25 | `mlVCut` | V-score cut lines |
| 26 | `ml3DBodyTop` | Top 3D body layer |
| 27 | `ml3DBodyBottom` | Bottom 3D body layer |
| 28 | `mlRouteToolPath` | Route tool path |
| 29 | `mlSheet` | Drawing sheet border |
| 30 | `mlBoardShape` | Board shape definition |
| 31 | `mlOverlayTop` | Top overlay extension |
| 32 | `mlOverlayBottom` | Bottom overlay extension |
| 33 | `mlSolderTop` | Top solder mask extension |
| 34 | `mlSolderBottom` | Bottom solder mask extension |
| 35 | `mlPasteTop` | Top paste extension |
| 36 | `mlPasteBottom` | Bottom paste extension |
| 37 | `mlTentingTop` | Top tenting |
| 38 | `mlTentingBottom` | Bottom tenting |
| 39 | `mlCoveringTop` | Top covering |
| 40 | `mlCoveringBottom` | Bottom covering |
| 41 | `mlPluggingTop` | Top plugging |
| 42 | `mlPluggingBottom` | Bottom plugging |
| 43 | `mlFilling` | Via filling |
| 44 | `mlCapping` | Via capping |
| 45 | `mlDiePadsTop` | Top die pads |
| 46 | `mlDiePadsBottom` | Bottom die pads |
| 47 | `mlWirebondingTop` | Top wirebonding |
| 48 | `mlWirebondingBottom` | Bottom wirebonding |

---

## Pad and Via Enumerations

### TShape (Pad/Via Shape)

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TShape.cs`

| Value | C# Name | Rust (`PadShape`) | Description |
|-------|---------|------------------|-------------|
| 0 | `eNoShape` | `NoShape` | No shape (placeholder) |
| 1 | `eRounded` | `Round` | Circular |
| 2 | `eRectangular` | `Rectangular` | Sharp-corner rectangle |
| 3 | `eOctagonal` | `Octagonal` | Octagonal |
| 4 | `eCircleShape` | `Circle` | Circle (alias for arc-based) |
| 5 | `eArcShape` | `Arc` | Arc shape |
| 6 | `eTerminator` | `Terminator` | Terminator shape |
| 7 | `eRoundRectShape` | `RoundRect` | Rounded rectangle |
| 8 | `eRotatedRectShape` | `RotatedRect` | Rotated rectangle |
| 9 | `eRoundedRectangular` | `RoundedRectangular` | Rounded rectangular (alias) |
| 10 | `eCustomShape` | `Custom` | Custom pad shape |

**Rust implementation:** `altium_format_types::pcb::PadShape`

### TShapeSubKind

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TShapeSubKind.cs`

| Value | C# Name | Rust (`PadShapeSubKind`) | Description |
|-------|---------|------------------------|-------------|
| 0 | `eNoKind` | `NoKind` | Default/none |
| 1 | `eOctagonalFinger` | `OctagonalFinger` | Octagonal finger pad |
| 2 | `eRoundedFinger` | `RoundedFinger` | Rounded finger pad |
| 3 | `eRoundedRectangle` | `RoundedRectangle` | Rounded rectangle |
| 4 | `eChamferedRectangle` | `ChamferedRectangle` | Chamfered rectangle |
| 5 | `eDonut` | `Donut` | Donut (ring) shape |

**Rust implementation:** `altium_format_types::pcb::PadShapeSubKind`

### TPadMode (Pad Stack Mode)

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TPadMode.cs`

| Value | C# Name | Rust (`PadStackMode`) | Description |
|-------|---------|----------------------|-------------|
| 0 | `ePadMode_Simple` | `Simple` | Same shape/size on all layers |
| 1 | `ePadMode_LocalStack` | `LocalStack` | Top/mid/bottom definitions |
| 2 | `ePadMode_ExternalStack` | `ExternalStack` | Per-layer definitions |

**Rust implementation:** `altium_format_types::pcb::PadStackMode`

### TExtendedHoleType (Hole Shape)

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TExtendedHoleType.cs`

| Value | C# Name | Rust (`HoleType`) | Description |
|-------|---------|------------------|-------------|
| 0 | `eRoundHole` | `Round` | Circular drill hole |
| 1 | `eSquareHole` | `Square` | Square drill hole |
| 2 | `eSlotHole` | `Slot` | Slotted drill hole |

**Rust implementation:** `altium_format_types::pcb::HoleType`

### TExtendedDrillType

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TExtendedDrillType.cs`

| Value | C# Name | Rust (`DrillType`) | Description |
|-------|---------|-------------------|-------------|
| 0 | `eDrilledHole` | `Drilled` | Mechanical drill |
| 1 | `ePunchedHole` | `Punched` | Punched hole |
| 2 | `eLaserDrilledHole` | `LaserDrilled` | Laser drilled |
| 3 | `ePlasmaDrilledHole` | `PlasmaDrilled` | Plasma drilled |

**Rust implementation:** `altium_format_types::pcb::DrillType`

### TDrillLayerPairType

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TDrillLayerPairType.cs`

| Value | C# Name | Rust (`DrillLayerPairType`) | Description |
|-------|---------|---------------------------|-------------|
| 0 | `Regular` | `Regular` | Standard through-hole drill |
| 1 | `MicroViaDrill` | `MicroViaDrill` | Micro-via laser drill |
| 2 | `Backdrill` | `Backdrill` | Back-drill operation |
| 3 | `CounterHole` | `CounterHole` | Counter-bore/counter-sink |

**Rust implementation:** `altium_format_types::pcb::DrillLayerPairType`

### TViaType

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TViaType.cs`

| Value | C# Name | Description |
|-------|---------|-------------|
| 0 | `InvalidVia` | Invalid/unset |
| 1 | `Thru` | Through-hole via |
| 2 | `Blind` | Blind via |
| 3 | `Buried` | Buried via |
| 4 | `BackdrillHole` | Back-drill via |
| 5 | `MicroVia` | Micro-via |
| 6 | `SkipVia` | Skip via |

### TViaStructureType (IPC-4761)

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TViaStructureType.cs`

| Value | C# Name | Display String | Description |
|-------|---------|---------------|-------------|
| 0 | `eViaStructureType_None` | None | No via protection |
| 1 | `eViaStructureType_1A_Tenting` | Type 1a | Tenting (side A) |
| 2 | `eViaStructureType_1B_Tenting` | Type 1b | Tenting (side B) |
| 3 | `eViaStructureType_2A_TentingAndCovering` | Type 2a | Tenting + covering (A) |
| 4 | `eViaStructureType_2B_TentingAndCovering` | Type 2b | Tenting + covering (B) |
| 5 | `eViaStructureType_3A_Plugging` | Type 3a | Plugging (A) |
| 6 | `eViaStructureType_3B_Plugging` | Type 3b | Plugging (B) |
| 7 | `eViaStructureType_4A_PluggingAndCovering` | Type 4a | Plugging + covering (A) |
| 8 | `eViaStructureType_4B_PluggingAndCovering` | Type 4b | Plugging + covering (B) |
| 9 | `eViaStructureType_5_Filling` | Type 5 | Filling |
| 10 | `eViaStructureType_6A_FillingAndCovering` | Type 6a | Filling + covering (A) |
| 11 | `eViaStructureType_6B_FillingAndCovering` | Type 6b | Filling + covering (B) |
| 12 | `eViaStructureType_7_FillingAndCapping` | Type 7 | Filling + capping |

### TMaskExpansionMode

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TMaskExpansionMode.cs`

| Value | C# Name | Rust (`MaskExpansionMode`) | Description |
|-------|---------|--------------------------|-------------|
| 0 | `eMaskExpansionMode_NoMask` | `NoMask` | No mask opening |
| 1 | `eMaskExpansionMode_Rule` | `Rule` | Expansion from design rule |
| 2 | `eMaskExpansionMode_Manual` | `Manual` | User-specified expansion |

**Rust implementation:** `altium_format_types::pcb::MaskExpansionMode`

### TBoardSide

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TBoardSide.cs`

| Value | C# Name | Rust (`BoardSide`) | Description |
|-------|---------|-------------------|-------------|
| 0 | `eBoardSide_Top` | `Top` | Top side of board |
| 1 | `eBoardSide_Bottom` | `Bottom` | Bottom side of board |

**Rust implementation:** `altium_format_types::pcb::BoardSide`

---

## Text Enumerations

### TTextKind

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TTextKind.cs`

| Value | C# Name | Rust (`TextKind`) | Description |
|-------|---------|------------------|-------------|
| 0 | `eText_StrokeFont` | `StrokeFont` | Vector/stroke font |
| 1 | `eText_TrueTypeFont` | `TrueTypeFont` | TrueType font |
| 2 | `eText_BarCode` | `Barcode` | Barcode rendering |

**Rust implementation:** `altium_format_types::pcb::TextKind`

### TBarcodeKind

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TBarcodeKind.cs`

| Value | C# Name | Rust (`BarcodeKind`) | Description |
|-------|---------|---------------------|-------------|
| 0 | `eBarcode39` | `Code39` | Code 39 barcode |
| 1 | `eBarCode128` | `Code128` | Code 128 barcode |

Note: The Rust implementation adds `QrCode` (2) and `DataMatrix` (3) which are not in the
.NET Pcbtypes enum but may exist in newer code paths.

**Rust implementation:** `altium_format_types::pcb::BarcodeKind`

### TTextAlignment

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TTextAlignment.cs`

| Value | C# Name | Description |
|-------|---------|-------------|
| 0 | `eNoneAlign` | No alignment |
| 1 | `eCentreAlign` | Center aligned |
| 2 | `eLeftAlign` | Left aligned |
| 3 | `eRightAlign` | Right aligned |
| 4 | `eTopAlign` | Top aligned |
| 5 | `eBottomAlign` | Bottom aligned |

### TTextAutoposition

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TTextAutoposition.cs`

| Value | C# Name | Description |
|-------|---------|-------------|
| 0 | `eAutoPos_Manual` | Manual positioning |
| 1 | `eAutoPos_TopLeft` | Auto top-left |
| 2 | `eAutoPos_CenterLeft` | Auto center-left |
| 3 | `eAutoPos_BottomLeft` | Auto bottom-left |
| 4 | `eAutoPos_TopCenter` | Auto top-center |
| 5 | `eAutoPos_CenterCenter` | Auto center-center |
| 6 | `eAutoPos_BottomCenter` | Auto bottom-center |
| 7 | `eAutoPos_TopRight` | Auto top-right |
| 8 | `eAutoPos_CenterRight` | Auto center-right |
| 9 | `eAutoPos_BottomRight` | Auto bottom-right |

**Rust implementation:** `altium_format_types::common::TextAutoPosition` (shared with schematic)

---

## Dimension Enumerations

### TDimensionKind

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TDimensionKind.cs`

| Value | C# Name | Rust (`DimensionKind`) | Description |
|-------|---------|----------------------|-------------|
| 0 | `eNoDimension` | `NoDimension` | No dimension (sentinel) |
| 1 | `eLinearDimension` | `Linear` | Linear distance |
| 2 | `eAngularDimension` | `Angular` | Angle measurement |
| 3 | `eRadialDimension` | `Radial` | Radius measurement |
| 4 | `eLeaderDimension` | `Leader` | Leader line annotation |
| 5 | `eDatumDimension` | `Datum` | Datum dimension |
| 6 | `eBaselineDimension` | `Baseline` | Baseline dimension chain |
| 7 | `eCenterDimension` | `Center` | Center mark |
| 8 | `eOriginalDimension` | `Original` | Original/ordinate |
| 9 | `eLinearDiameterDimension` | `LinearDiameter` | Linear diameter |
| 10 | `eRadialDiameterDimension` | `RadialDiameter` | Radial diameter |

**Rust implementation:** `altium_format_types::pcb::DimensionKind`

### TDimensionUnit

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TDimensionUnit.cs`

| Value | C# Name | Rust (`DimensionUnit`) | Description |
|-------|---------|----------------------|-------------|
| 0 | `eMils` | `Mils` | Thousandths of an inch |
| 1 | `eInches` | `Inches` | Inches |
| 2 | `eMillimeters` | `Millimeters` | Millimeters |
| 3 | `eCentimeters` | `Centimeters` | Centimeters |
| 4 | `eDegrees` | `Degrees` | Degrees (angular) |
| 5 | `eRadians` | `Radians` | Radians (angular) |
| 6 | `eAutomaticUnit` | `Automatic` | Auto-select unit |

**Rust implementation:** `altium_format_types::pcb::DimensionUnit`

### TDimensionTextPosition

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TDimensionTextPosition.cs`

| Value | C# Name | Rust (`DimensionTextPosition`) | Description |
|-------|---------|-------------------------------|-------------|
| 0 | `eTextAuto` | `Auto` | Automatic placement |
| 1 | `eTextCenter` | `Center` | Centered on dimension |
| 2 | `eTextTop` | `Top` | Above dimension line |
| 3 | `eTextBottom` | `Bottom` | Below dimension line |
| 4 | `eTextRight` | `Right` | Right of dimension |
| 5 | `eTextLeft` | `Left` | Left of dimension |
| 6 | `eTextInsideRight` | `InsideRight` | Inside, right aligned |
| 7 | `eTextInsideLeft` | `InsideLeft` | Inside, left aligned |
| 8 | `eTextUniDirectional` | `UniDirectional` | Unidirectional reading |
| 9 | `eTextManual` | `Manual` | Manual placement |

**Rust implementation:** `altium_format_types::pcb::DimensionTextPosition`

### TDimensionArrowPosition

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TDimensionArrowPosition.cs`

| Value | C# Name | Rust (`DimensionArrowPosition`) | Description |
|-------|---------|--------------------------------|-------------|
| 0 | `eInside` | `Inside` | Arrows inside extension lines |
| 1 | `eOutside` | `Outside` | Arrows outside extension lines |

**Rust implementation:** `altium_format_types::pcb::DimensionArrowPosition`

---

## Region and Polygon Enumerations

### TRegionKind

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TRegionKind.cs`

| Value | C# Name | Rust (`RegionKind`) | Description |
|-------|---------|-------------------|-------------|
| 0 | `eRegionKind_Copper` | `Copper` | Copper fill region |
| 1 | `eRegionKind_Cutout` | `Cutout` | Polygon cutout |
| 2 | `eRegionKind_NamedRegion` | `Named` | Named region (keepout, etc.) |
| 3 | `eRegionKind_BoardCutout` | `BoardCutout` | Board cutout |

Note: The Rust implementation adds `Cavity` (4) which exists in newer format versions.

**Rust implementation:** `altium_format_types::pcb::RegionKind`

### TPolygonType (TPCBPolygonType)

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TPolygonType.cs`

| Value | C# Name | Rust (`PolygonType`) | Description |
|-------|---------|---------------------|-------------|
| 0 | `eSignalLayerPolygon` | `SignalLayer` | Signal layer copper pour |
| 1 | `eSplitPlanePolygon` | `SplitPlane` | Split plane polygon |
| 2 | `eCoverlayOutlinePolygon` | `CoverlayOutline` | Coverlay outline |

**Rust implementation:** `altium_format_types::pcb::PolygonType`

### TPolyHatchStyle

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TPolyHatchStyle.cs`

| Value | C# Name | Rust (`PolyHatchStyle`) | Description |
|-------|---------|------------------------|-------------|
| 0 | `ePolyHatch90` | `Hatch90` | 90-degree crosshatch |
| 1 | `ePolyHatch45` | `Hatch45` | 45-degree crosshatch |
| 2 | `ePolyVHatch` | `VerticalHatch` | Vertical hatching |
| 3 | `ePolyHHatch` | `HorizontalHatch` | Horizontal hatching |
| 4 | `ePolyNoHatch` | `NoHatch` | No hatch pattern |
| 5 | `ePolySolid` | `Solid` | Solid fill (default) |

**Rust implementation:** `altium_format_types::pcb::PolyHatchStyle`

### TPolySegmentType

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TPolySegmentType.cs`

| Value | C# Name | Rust (`PolySegmentType`) | Description |
|-------|---------|-------------------------|-------------|
| 0 | `ePolySegmentLine` | `Line` | Straight line segment |
| 1 | `ePolySegmentArc` | `Arc` | Arc segment |

**Rust implementation:** `altium_format_types::pcb::PolySegmentType`

### TPolygonPourOver

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TPolygonPourOver.cs`

| Value | C# Name | Description |
|-------|---------|-------------|
| 0 | `ePolygonPourOver_None` | Do not pour over same-net objects |
| 1 | `ePolygonPourOver_SameNet` | Pour over all same-net objects |
| 2 | `ePolygonPourOver_SameNetPolygons` | Pour over same-net polygons only |

### TPolygonRepourMode

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TPolygonRepourMode.cs`

| Value | C# Name | Rust (`PolygonRepourMode`) | Description |
|-------|---------|--------------------------|-------------|
| 0 | `eNeverRepour` | `Never` | Never auto-repour |
| 1 | `eThresholdRepour` | `Threshold` | Repour on threshold change |
| 2 | `eAlwayRepour` | `Always` | Always repour (note typo in original) |

**Rust implementation:** `altium_format_types::pcb::PolygonRepourMode`

### TPolygonReliefAngle

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TPolygonReliefAngle.cs`

| Value | C# Name | Rust (`PolygonReliefAngle`) | Description |
|-------|---------|---------------------------|-------------|
| 0 | `ePolygonReliefAngle_45` | `Angle45` | 45-degree thermal spokes |
| 1 | `ePolygonReliefAngle_90` | `Angle90` | 90-degree thermal spokes |
| 2 | `ePolygonReliefAngle_0` | -- | 0-degree thermal spokes |
| 3 | `ePolygonReliefAngle_135` | -- | 135-degree thermal spokes |

Note: The Rust implementation only defines values 0-1; values 2-3 may need to be added.

### TPlaneConnectStyle (Polygon Connection)

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TPlaneConnectStyle.cs`

| Value | C# Name | Description |
|-------|---------|-------------|
| 0 | `eReliefConnectToPlane` | Thermal relief connection |
| 1 | `eDirectConnectToPlane` | Direct copper connection |
| 2 | `eNoConnect` | No connection |

### TPlaneConnectionStyle

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TPlaneConnectionStyle.cs`

| Value | C# Name | Rust (`PlaneConnectionStyle`) | Description |
|-------|---------|------------------------------|-------------|
| 0 | `ePlaneNoConnect` | `NoConnect` | No connection to plane |
| 1 | `ePlaneReliefConnect` | `Relief` | Thermal relief |
| 2 | `ePlaneDirectConnect` | `Direct` | Direct copper |

Note: different ordering from `TPlaneConnectStyle` above (NoConnect is 0 here vs 2).

**Rust implementation:** `altium_format_types::pcb::PlaneConnectionStyle`

---

## Component and 3D Model Enumerations

### T3DModelType

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/T3DModelType.cs`

| Value | C# Name | Description |
|-------|---------|-------------|
| 0 | `e3DModelType_Extruded` | Extruded 2D outline |
| 1 | `e3DModelType_Generic` | STEP/STP 3D model file |
| 2 | `e3DModelType_Cylinder` | Parametric cylinder |
| 3 | `e3DModelType_Sphere` | Parametric sphere |

### TModelDisplayMode

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TModelDisplayMode.cs`

| Value | C# Name | Description |
|-------|---------|-------------|
| 0 | `dmSolid` | Solid rendering |
| 1 | `dmTransparent` | Transparent rendering |
| 2 | `dmWireframe` | Wireframe rendering |
| 3 | `dmHide` | Hidden |
| 4 | `dmNotAssigned` | Not assigned |

### TComponentType

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TComponentType.cs`

| Value | C# Name | Description |
|-------|---------|-------------|
| 0 | `eBJT` | Bipolar junction transistor |
| 1 | `eCapactitor` | Capacitor (note typo in original) |
| 2 | `eConnector` | Connector |
| 3 | `eDiode` | Diode |
| 4 | `eIC` | Integrated circuit |
| 5 | `eInductor` | Inductor |
| 6 | `eResistor` | Resistor |

---

## Routing and Connectivity Enumerations

### TCornerStyle (Routing Corner Style)

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TCornerStyle.cs`

| Value | C# Name | Rust (`CornerStyle`) | Description |
|-------|---------|---------------------|-------------|
| 0 | `eCornerStyle_90` | `Degree90` | 90-degree corners |
| 1 | `eCornerStyle_45` | `Degree45` | 45-degree mitered corners |
| 2 | `eCornerStyle_Round` | `Round` | Round/arc corners |

**Rust implementation:** `altium_format_types::pcb::CornerStyle`

### TRoutingCornerStyle

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TRoutingCornerStyle.cs`

| Value | C# Name | Description |
|-------|---------|-------------|
| 0 | `eRoutingCornerStyle_90` | 90-degree routing |
| 1 | `eRoutingCornerStyle_45` | 45-degree routing |
| 2 | `eRoutingCornerStyle_Any` | Any-angle routing |

---

## Class and Rule Enumerations

### TClassMemberKind

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TClassMemberKind.cs`

| Value | C# Name | Rust (`ClassMemberKind`) | Description |
|-------|---------|-------------------------|-------------|
| 0 | Net | `Net` | Net class |
| 1 | Component | `Component` | Component class |
| 2 | FromTo | `FromTo` | FromTo class |
| 3 | Pad | `Pad` | Pad class |
| 4 | Layer | `Layer` | Layer class |
| 5 | DesignChannel | `DesignChannel` | Design channel class |
| 6 | DifferentialPair | `DifferentialPair` | Differential pair class |
| 7 | Polygon | `Polygon` | Polygon class |
| 8 | SplitPlane | `SplitPlane` | Split plane class |

**Rust implementation:** `altium_format_types::pcb::ClassMemberKind`

### TRuleKind

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TRuleKind.cs`

70 rule kinds (0-69). See the Rust implementation for the full mapping.

**Key differences between RT_PCB and Pcbtypes variants:**

| Index | RT_PCB Name | Pcbtypes Name | Notes |
|-------|------------|---------------|-------|
| 12 | `eRule_PowerPlaneClearance` | `eRule_PowerPlaneExpansion` | Different naming |
| 60 | (not present, shifted) | `eRule_Pcad` | Pcbtypes has extra entry |
| 61 | `eRule_None` | `eRule_SMDPADEntry` | Shifted indices |

The RT_PCB version is the authoritative one used by the runtime and matches the Rust
implementation.

**Rust implementation:** `altium_format_types::pcb::RuleKind`

---

## Layer Stack Enumerations

### TDielectricType

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TDielectricType.cs`

| Value | C# Name | Rust (`DielectricType`) | Description |
|-------|---------|------------------------|-------------|
| 0 | `eNoDielectric` | `NoDielectric` | Not a dielectric layer |
| 1 | `eCore` | `Core` | Core material |
| 2 | `ePrePreg` | `PrePreg` | Pre-preg material |
| 3 | `eSurfaceMaterial` | `SurfaceMaterial` | Surface material |

**Rust implementation:** `altium_format_types::pcb::DielectricType`

---

## Miscellaneous Enumerations

### TConfinementStyle

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TConfinementStyle.cs`

| Value | C# Name | Description |
|-------|---------|-------------|
| 0 | `eConfineIn` | Confine objects inside region |
| 1 | `eConfineOut` | Confine objects outside region |

### TDrillSymbol

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TDrillSymbol.cs`

| Value | C# Name | Description |
|-------|---------|-------------|
| 0 | `eSymbols` | Drill chart uses symbols |
| 1 | `eNumbers` | Drill chart uses numbers |
| 2 | `eLetters` | Drill chart uses letters |

### TSingleLayerMode

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TSingleLayerMode.cs`

| Value | C# Name | Description |
|-------|---------|-------------|
| 0 | `eGrayScaleOtherLayers` | Gray out non-active layers |
| 1 | `eMonochromeOtherLayers` | Monochrome non-active layers |
| 2 | `eHideOtherLayers` | Hide non-active layers |
| 3 | `eNotSingleLayerMode` | Normal multi-layer view |

---

## Coordinate Constants

From `RT_PCB.Consts`:

| Constant | Value | Description |
|----------|-------|-------------|
| `InternalUnits` / `k1Mil` | 10,000 | Internal units per mil |
| `k1Inch` | 10,000,000 | Internal units per inch |
| `kMaxCoord` | 999,990,000 | Maximum coordinate (99,999 mils) |
| `kMinCoord` | 0 | Minimum coordinate |
| `cMMsInMil` | 0.0254 | Millimeters per mil |
| `kMaxStrokes` | 2,000 | Maximum stroke font strokes |
| `kMaxPolySize` | 5,000 | Maximum polygon vertex count |
| `kMaxPadName` | 20 | Maximum pad name length |
| `MaxFreeStringLength` | 254 | Maximum free string length |

**Rust implementation:** `altium_format_types::pcb::constants`

---

## Cross-Reference: altium-format-types Rust Types

| C# Enum | Rust Type | Module |
|---------|-----------|--------|
| `TObjectId` | `PcbObjectId` | `altium_format_types::pcb` |
| `TV6_Layer` | `V6Layer` | `altium_format_types::pcb` |
| V7 layer struct | `V7Layer` | `altium_format_types::pcb` |
| `TAdvPCBFileFormatVersion` | `PcbFileFormatVersion` | `altium_format_types::pcb` |
| `TShape` | `PadShape` | `altium_format_types::pcb` |
| `TShapeSubKind` | `PadShapeSubKind` | `altium_format_types::pcb` |
| `TPadMode` | `PadStackMode` | `altium_format_types::pcb` |
| `TExtendedHoleType` | `HoleType` | `altium_format_types::pcb` |
| `TExtendedDrillType` | `DrillType` | `altium_format_types::pcb` |
| `TDrillLayerPairType` | `DrillLayerPairType` | `altium_format_types::pcb` |
| `TTextKind` | `TextKind` | `altium_format_types::pcb` |
| `TBarcodeKind` | `BarcodeKind` | `altium_format_types::pcb` |
| `TDimensionKind` | `DimensionKind` | `altium_format_types::pcb` |
| `TDimensionUnit` | `DimensionUnit` | `altium_format_types::pcb` |
| `TDimensionTextPosition` | `DimensionTextPosition` | `altium_format_types::pcb` |
| `TDimensionArrowPosition` | `DimensionArrowPosition` | `altium_format_types::pcb` |
| `TRegionKind` | `RegionKind` | `altium_format_types::pcb` |
| `TPolygonType` | `PolygonType` | `altium_format_types::pcb` |
| `TPolyHatchStyle` | `PolyHatchStyle` | `altium_format_types::pcb` |
| `TPolySegmentType` | `PolySegmentType` | `altium_format_types::pcb` |
| `TPolygonRepourMode` | `PolygonRepourMode` | `altium_format_types::pcb` |
| `TPolygonReliefAngle` | `PolygonReliefAngle` | `altium_format_types::pcb` |
| `TPlaneConnectionStyle` | `PlaneConnectionStyle` | `altium_format_types::pcb` |
| `TMaskExpansionMode` | `MaskExpansionMode` | `altium_format_types::pcb` |
| `TBoardSide` | `BoardSide` | `altium_format_types::pcb` |
| `TCornerStyle` | `CornerStyle` | `altium_format_types::pcb` |
| `TDielectricType` | `DielectricType` | `altium_format_types::pcb` |
| `TClassMemberKind` | `ClassMemberKind` | `altium_format_types::pcb` |
| `TRuleKind` | `RuleKind` | `altium_format_types::pcb` |
| `TTextAutoposition` | `TextAutoPosition` | `altium_format_types::common` |
| flags word | `PcbFlags` | `altium_format_types::pcb` |
| -- | `TentingMode` | `altium_format_types::pcb` |

### Enums Not Yet in altium-format-types

The following enums from the .NET source are documented above but do not yet have Rust
counterparts in `altium_format_types::pcb`:

- `TStorageFeature` -- feature bitset
- `TMechanicalLayerKind` -- mechanical layer purpose classification
- `TViaType` -- via type (thru/blind/buried/micro)
- `TViaStructureType` -- IPC-4761 via protection types
- `TModelDisplayMode` -- 3D model display mode
- `T3DModelType` -- 3D model geometry type
- `TComponentType` -- component classification
- `TPolygonPourOver` -- polygon pour-over mode
- `TTextAlignment` -- text alignment
- `TConfinementStyle` -- confinement rule direction
- `TDrillSymbol` -- drill chart symbol mode
- `TSingleLayerMode` -- single layer view mode
- `TLayerPartition` -- V7 layer partitioning
- `TRoutingCornerStyle` -- interactive routing corner mode
- `TPlaneConnectStyle` -- polygon-to-plane connection style

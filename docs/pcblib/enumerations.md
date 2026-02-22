# Enumerations

All enumerations used by PCB primitives in PcbLib (and PcbDoc). These are shared across
both document types.

## TObjectId (Primitive Type)

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TObjectId.cs`

| Value | Name | Found in PcbLib | Description |
|-------|------|:---:|-------------|
| 0 | `eIgnoreObject` | No | Null/sentinel |
| 1 | `eArcObject` | Yes | Circular arc |
| 2 | `ePadObject` | Yes | Component pad |
| 3 | `eViaObject` | Yes | Plated through-hole via |
| 4 | `eTrackObject` | Yes | Routed line segment |
| 5 | `eTextObject` | Yes | Text string |
| 6 | `eFillObject` | Yes | Solid rectangular fill |
| 7 | `eFromToObject` | No | Connection/ratsnest endpoint |
| 8 | `eNetObject` | No | Net grouping |
| 9 | `eComponentObject` | No | Component instance |
| 10 | `ePolygonObject` | No | Copper pour polygon |
| 11 | `eRegionObject` | Yes | Region (copper, cutout, keepout) |
| 12 | `eComponentBodyObject` | Yes | 3D component body |
| 13 | `eDimensionObject` | No | Dimension annotation |
| 14 | `eCoordinateObject` | No | Coordinate annotation |
| 15 | `eClassObject` | No | Object class |
| 16 | `eRuleObject` | No | Design rule |
| 17 | `eManualFromToObject` | No | Manual FromTo |
| 18 | `eDifferentialPairObject` | No | Differential pair |
| 19 | `eViolationObject` | No | DRC violation |
| 20 | `eEmbeddedObject` | No | Embedded object |
| 21 | `eEmbeddedBoardObject` | No | Embedded board |
| 22 | `eSplitPlaneObject` | No | Split plane region |
| 23 | `eTraceObject` | No | Routed path group |
| 24 | `eSpareViaObject` | No | Spare via |
| 25 | `eBoardObject` | No | Board document root |
| 26 | `eBoardOutlineObject` | No | Board outline |

## PCB Layers

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/` + `altium-format-types`

| Value | Layer Name | Category |
|-------|-----------|----------|
| 0 | Top Signal (Copper) | Signal |
| 1-30 | Mid Signal Layers 1-30 | Signal |
| 31 | Bottom Signal (Copper) | Signal |
| 32 | Top Overlay (Silkscreen) | Mask |
| 33 | Bottom Overlay | Mask |
| 34 | Top Paste | Mask |
| 35 | Bottom Paste | Mask |
| 36 | Top Solder Mask | Mask |
| 37 | Bottom Solder Mask | Mask |
| 38-53 | Internal Planes 1-16 | Plane |
| 54 | Drill Guide | Drill |
| 55 | Keep-Out Layer | Mechanical |
| 56-71 | Mechanical Layers 1-16 | Mechanical |
| 72 | Drill Drawing | Drill |
| 73 | Multi-Layer | Special |

**PcbLib footprint context**: Footprint primitives commonly use layers 0 (Top), 31 (Bottom),
32 (Top Overlay/Silkscreen), 34 (Top Paste), 36 (Top Solder Mask), and mechanical layers
(56+) for courtyard. Multi-layer (73) is used for through-hole pads.

## Pad Shapes (TShape)

| Value | Name | Description |
|-------|------|-------------|
| 0 | NoShape | No pad (placeholder) |
| 1 | Round | Circular |
| 2 | Rectangular | Sharp corners |
| 3 | Octagonal | Octagonal |
| 4 | RoundRect | Rounded rectangle |
| 5 | RotatedRect | Rotated rectangle |

## Pad Stack Mode (TStackMode)

| Value | Name | Description |
|-------|------|-------------|
| 0 | Simple | One size/shape for all layers |
| 1 | TopMiddleBottom | Three sizes: top, mid, bottom |
| 2 | FullStack | Independent per each of 32 layers |

## Pad Hole Shape

| Value | Name | Description |
|-------|------|-------------|
| 0 | Round | Circular drill hole |
| 1 | Square | Square drill hole |
| 2 | Slot | Slotted drill hole |

## Text Kind (TTextKind)

| Value | Name | Description |
|-------|------|-------------|
| 0 | Stroke | Stroke/vector font |
| 1 | TrueType | TrueType font |
| 2 | BarCode | Barcode |

## Text Justification

9-position grid:

| Value | Name |
|-------|------|
| 0 | BottomLeft |
| 1 | BottomCenter |
| 2 | BottomRight |
| 3 | MiddleLeft |
| 4 | MiddleCenter |
| 5 | MiddleRight |
| 6 | TopLeft |
| 7 | TopCenter |
| 8 | TopRight |

## PcbFlags (u16 bitmask)

| Bit | Flag | Description |
|-----|------|-------------|
| 0 | UNLOCKED | Primitive can be moved/edited |
| 1 | TENTING_TOP | Solder mask tenting on top |
| 2 | TENTING_BOTTOM | Solder mask tenting on bottom |
| 3 | FABRICATION_TOP | Fabrication output on top |
| 4 | FABRICATION_BOTTOM | Fabrication output on bottom |
| 5 | KEEPOUT | Keepout region marker |

## Region Kind

| Value | Name | Description |
|-------|------|-------------|
| 0 | Copper | Copper region |
| 1 | Cutout | Board cutout |
| 2 | CopperKeepout | Copper keepout area |
| 3 | Cavity | Board cavity |

(The exact values need verification from the decompiled code.)

## Mask Expansion Mode

| Value | Name | Description |
|-------|------|-------------|
| 0 | Auto | Calculated from design rules |
| 1 | Manual | User-specified value |
| 2 | Rule | From specific rule |

## 3D Model Type (T3DModelType)

| Value | Name | Description |
|-------|------|-------------|
| 0 | Extruded | Extruded 2D outline |
| 1 | Generic | STEP/STP file |
| 2 | Cylinder | Parametric cylinder |
| 3 | Sphere | Parametric sphere |

## File Format Version (TAdvPCBFileFormatVersion)

| Value | Name | Description |
|-------|------|-------------|
| 0 | None | Unknown/invalid |
| 2 | Library_V3 | Protel 99 SE library |
| 5 | Library_V4 | DXP library |
| 8 | Library_V5 | Altium Designer library |
| 11 | Library_V6 | Modern AD library (our target) |

## Storage Features (TStorageFeature)

Relevant feature flags for PcbLib:

| Value | Flag | Description |
|-------|------|-------------|
| 5 | `eHasShapeBasedRegions` | Shape-based region format |
| 6 | `eHasShapeBasedCompBodies` | Shape-based component bodies |
| 9 | `eHasCustomPadShapesAtWriteStage` | Custom pad shapes |
| 11 | `eHasFootprintParametersAtWriteStage` | Footprint parameters |
| 24 | `eHasIncreasedSignalLayers` | > 32 signal layers |

See `docs/dxp/pcb-files.md` section 10.2 for the complete list.

# UNIMPLEMENTED

Stream-level implementation gaps in `altium-format` v2.

Data sources used:

- `uv run --script scripts/ole-inspect.py scan data --json > /tmp/ole-scan.json`
- `crates/altium-format/src/v2/documents/*.rs` code audit

Normalization used in this file:

- `<Item>` means per-component/per-footprint storage name normalized away.
- `<index>` means numeric stream names normalized away.

Implemented stream patterns used for diff:

- `.schdoc`: `FileHeader`
- `.schlib`: `FileHeader`, `SectionKeys`, `<Item>/Data`
- `.pcblib`: `<Item>/Parameters`, `<Item>/Header`, `<Item>/Data`
- `.pcbdoc`: none
- `.prjpcb`: none

Total normalized missing stream patterns: `129`.

### `.schdoc` missing streams (2)
| Stream pattern | Files |
|---|---:|
| `Additional` | 9 |
| `Storage` | 9 |

### `.schlib` missing streams (6)
| Stream pattern | Files |
|---|---:|
| `<Item>/PinPackageLength` | 199 |
| `<Item>/PinSymbolLineWidth` | 199 |
| `<Item>/PinFrac` | 167 |
| `<Item>/PinTextData` | 16 |
| `Storage` | 3 |
| `<Item>/Redirection` | 1 |

### `.pcblib` missing streams (25)
| Stream pattern | Files |
|---|---:|
| `<Item>/WideStrings` | 763 |
| `<Item>/UniqueIDPrimitiveInformation/Data` | 276 |
| `<Item>/UniqueIDPrimitiveInformation/Header` | 276 |
| `Library/Models/<index>` | 121 |
| `FileHeader` | 3 |
| `Library/Data` | 3 |
| `Library/EmbeddedFonts` | 3 |
| `Library/Header` | 3 |
| `Library/Models/Data` | 3 |
| `Library/Models/Header` | 3 |
| `Library/ModelsNoEmbed/Data` | 3 |
| `Library/ModelsNoEmbed/Header` | 3 |
| `Library/Textures/Data` | 3 |
| `Library/Textures/Header` | 3 |
| `Library/ComponentParamsTOC/Data` | 2 |
| `Library/ComponentParamsTOC/Header` | 2 |
| `Library/LayerKindMapping/Data` | 2 |
| `Library/LayerKindMapping/Header` | 2 |
| `Library/PadViaLibrary/Data` | 2 |
| `Library/PadViaLibrary/Header` | 2 |
| `<Item>/PrimitiveGuids/Data` | 1 |
| `<Item>/PrimitiveGuids/Header` | 1 |
| `<Item>/ExtendedPrimitiveInformation/Data` | 1 |
| `<Item>/ExtendedPrimitiveInformation/Header` | 1 |
| `SectionKeys` | 1 |

### `.pcbdoc` missing streams (95)
| Stream pattern | Files |
|---|---:|
| `Models/<index>` | 44 |
| `Advanced Placer Options6/Data` | 2 |
| `Advanced Placer Options6/Header` | 2 |
| `Arcs6/Data` | 2 |
| `Arcs6/Header` | 2 |
| `Board6/Data` | 2 |
| `Board6/Header` | 2 |
| `BoardRegions/Data` | 2 |
| `BoardRegions/Header` | 2 |
| `Classes6/Data` | 2 |
| `Classes6/Header` | 2 |
| `ComponentBodies6/Data` | 2 |
| `ComponentBodies6/Header` | 2 |
| `Components6/Data` | 2 |
| `Components6/Header` | 2 |
| `Connections6/Data` | 2 |
| `Connections6/Header` | 2 |
| `Coordinates6/Data` | 2 |
| `Coordinates6/Header` | 2 |
| `Design Rule Checker Options6/Data` | 2 |
| `Design Rule Checker Options6/Header` | 2 |
| `DifferentialPairs6/Data` | 2 |
| `DifferentialPairs6/Header` | 2 |
| `Dimensions6/Data` | 2 |
| `Dimensions6/Header` | 2 |
| `EmbeddedBoards6/Data` | 2 |
| `EmbeddedBoards6/Header` | 2 |
| `EmbeddedFonts6/Data` | 2 |
| `EmbeddedFonts6/Header` | 2 |
| `Embeddeds6/Data` | 2 |
| `Embeddeds6/Header` | 2 |
| `ExtendedPrimitiveInformation/Data` | 2 |
| `ExtendedPrimitiveInformation/Header` | 2 |
| `FileHeader` | 2 |
| `FileHeaderSix` | 2 |
| `FileVersionInfo/Data` | 2 |
| `FileVersionInfo/Header` | 2 |
| `Fills6/Data` | 2 |
| `Fills6/Header` | 2 |
| `FromTos6/Data` | 2 |
| `FromTos6/Header` | 2 |
| `LayerKindMapping/Data` | 2 |
| `LayerKindMapping/Header` | 2 |
| `Models/Data` | 2 |
| `Models/Header` | 2 |
| `ModelsNoEmbed/Data` | 2 |
| `ModelsNoEmbed/Header` | 2 |
| `Nets6/Data` | 2 |
| `Nets6/Header` | 2 |
| `PadViaLibrary/Data` | 2 |
| `PadViaLibrary/Header` | 2 |
| `PadViaLibraryCache/Data` | 2 |
| `PadViaLibraryCache/Header` | 2 |
| `PadViaLibraryLinks/Data` | 2 |
| `PadViaLibraryLinks/Header` | 2 |
| `Pads6/Data` | 2 |
| `Pads6/Header` | 2 |
| `Pin Swap Options6/Data` | 2 |
| `Pin Swap Options6/Header` | 2 |
| `PinPairsSection/Data` | 2 |
| `PinPairsSection/Header` | 2 |
| `Polygons6/Data` | 2 |
| `Polygons6/Header` | 2 |
| `PrimitiveParameters/Data` | 2 |
| `PrimitiveParameters/Header` | 2 |
| `Regions6/Data` | 2 |
| `Regions6/Header` | 2 |
| `Rules6/Data` | 2 |
| `Rules6/Header` | 2 |
| `ShapeBasedComponentBodies6/Data` | 2 |
| `ShapeBasedComponentBodies6/Header` | 2 |
| `ShapeBasedRegions6/Data` | 2 |
| `ShapeBasedRegions6/Header` | 2 |
| `SignalClasses/Data` | 2 |
| `SignalClasses/Header` | 2 |
| `SmartUnions/Data` | 2 |
| `SmartUnions/Header` | 2 |
| `Texts/Data` | 2 |
| `Texts/Header` | 2 |
| `Texts6/Data` | 2 |
| `Texts6/Header` | 2 |
| `Textures/Data` | 2 |
| `Textures/Header` | 2 |
| `Tracks6/Data` | 2 |
| `Tracks6/Header` | 2 |
| `UnionNames/Data` | 2 |
| `UnionNames/Header` | 2 |
| `UniqueIDPrimitiveInformation/Data` | 2 |
| `UniqueIDPrimitiveInformation/Header` | 2 |
| `Vias6/Data` | 2 |
| `Vias6/Header` | 2 |
| `WaivedViolations/Data` | 2 |
| `WaivedViolations/Header` | 2 |
| `WideStrings6/Data` | 2 |
| `WideStrings6/Header` | 2 |

### `.prjpcb` missing streams (1)
| Stream pattern | Files |
|---|---:|
| `(raw)` | 1 |

# PrjPcb File Format Reference

## Overview

The `.PrjPcb` file is the **sole Altium file type that is NOT a CFB compound document**.
It is a plain-text INI-style file (UTF-8 with BOM) that describes a PCB project — its
constituent documents, build configurations, ERC settings, output jobs, annotation rules,
and ECO comparison options.

**C# entry point:** `PrjPcbReader.Create(path).Read()` → `PrjPcbContent`

**Key files in AD26-dotnet:**

| File | Purpose |
|------|---------|
| `Altium.Sch.Data.Project/…/PrjPcbReader.cs` | Reader (delegates to PrjPcbContent) |
| `Altium.Sch.Data.Project/…/PrjPcbContent.cs` | Parser + accessor methods |
| `Altium.Sch.Data.Project/…/PrjPcbConsts.cs` | All string constant key names |
| `Altium.Sch.Base/…/PrjContentBase.cs` | Base class: line parsing, state machine |
| `Altium.Sch.Data.Project/…/ProjectOptions.cs` | Immutable output data class |

**No .NET writer exists.** Writing is handled by the Delphi side via COM (`IProjectProperties.DM_Set*`).

---

## Parsing Mechanics

1. File read as `string[]` lines via `FileHelper.ReadFileSafe()`
2. `PrjContentBase.Parse()` iterates lines:
   - `[SectionName]` headers → update state machine
   - `Key=Value` lines → split on **first** `=`, trim both sides, store in case-insensitive `ValueMap`
3. Numbered sections (`[Document1]`, `[Document2]`, …) detected by prefix match — `[Document` matches all
4. Boolean encoding: `1` = true, `0` = false (integer booleans, not string "True"/"False")
5. All key comparisons: `OrdinalIgnoreCase`

### State Machine

The parser tracks which section type it's in via `ProjectFileState`:

| Prefix Match | State | Notes |
|---|---|---|
| `[Document` | `eDocument` | Numbered: `[Document1]`, `[Document2]`, … |
| `[DeviceSheetFolder` | `eDeviceSheetFolder` | Numbered |
| `[DeviceSheet` | `eDeviceSheet` | Numbered |
| `[LibraryUpdateOptions]` | `eLibraryUpdateOptions` | Single section, has sub-records |
| `[` (anything else) | `eUnknown` | Keys go to flat `ValueMap` |

The `eDocument`, `eDeviceSheetFolder`, `eDeviceSheet`, and `eLibraryUpdateOptions` states
collect data into dedicated lists. All other sections dump keys into a single flat `ValueMap`
(case-insensitive `Dictionary<string, string>`).

### Pipe-Delimited Sub-Values

Some INI values embed the same `|KEY=VALUE|` parameter format used by CFB-based files:
- `PrinterOptions=Record=PrinterOptions|Copies=1|Duplex=1|…`
- `PageOptions=Record=PageOptions|CenterHorizontal=True|…`
- `ComparisonOptions0=Kind=Net|MinPercent=75|MinMatch=3|…`

These can reuse the existing `ParameterCollection` parser for the embedded sub-values.

---

## Complete Section Reference

### `[Design]` — Project Settings

The main section. All keys go into the flat `ValueMap`.

#### Core Project Options

| Key | Type | Default | C# Const | Enum |
|---|---|---|---|---|
| `Version` | string | `"1.0"` | — | File format version |
| `HierarchyMode` | int | 0 | `HierarchyMode` | `TFlattenMode` |
| `ChannelRoomNamingStyle` | int | 0 | `ChannelRoomNamingStyle` | `TChannelRoomNamingStyle` |
| `ChannelDesignatorFormatString` | string | `"$Component_$RoomName"` | `ChannelDesignatorFormatString` | Format keywords below |
| `ChannelRoomLevelSeperator` | string | `"_"` | `ChannelRoomLevelSeparator` | Note: Altium typo "Seperator" |

**ChannelDesignatorFormatString keywords:**
`$Component`, `$RoomName`, `$ChannelPrefix`, `$ChannelIndex`, `$ChannelAlpha`,
`$SheetDesignator`, `$SheetNumber`, `$DocumentNumber`, `$ComponentPrefix`, `$ComponentIndex`

#### Net Naming

| Key | Type | Default | C# Const |
|---|---|---|---|
| `AllowPortNetNames` | bool(int) | 0 | `AllowPortNetNames` |
| `AllowSheetEntryNetNames` | bool(int) | 1 | `AllowSheetEntryNetNames` |
| `NetlistSinglePinNets` | bool(int) | 0 | `NetlistSinglePinNets` |
| `AppendSheetNumberToLocalNets` | bool(int) | 0 | `AppendSheetNumberToLocalNets` |
| `NameNetsHierarchically` | bool(int) | 0 | `NameNetsHierarchically` |
| `PowerPortNamesTakePriority` | bool(int) | 0 | `PowerPortNamesTakePriority` |

#### Pin Swap

| Key | Type | Default | C# Const |
|---|---|---|---|
| `PinSwapBy_Netlabel` | bool(int) | 0 | `PinSwapByNetlabel` |
| `PinSwapBy_Pin` | bool(int) | 0 | `PinSwapByPin` |

#### Cross-References

| Key | Type | Default | C# Const | Enum |
|---|---|---|---|---|
| `CrossRefSheetStyle` | int | 2 | `CrossRefSheetStyle` | `TCrossRefSheetStyle` |
| `CrossRefLocationStyle` | int | 1 | `CrossRefLocationStyle` | `TCrossRefLocationStyle` |
| `CrossRefPorts` | int | 3 | `CrossRefPorts` | `TCrossRefPorts` |
| `CrossRefCrossSheets` | bool(int) | 1 | `CrossRefCrossSheets` | |
| `CrossRefSheetEntries` | bool(int) | 0 | `CrossRefSheetEntries` | |
| `CrossRefFollowFromMainSettings` | bool(int) | 1 | `CrossRefFollowFromMainSettings` | |

#### Sheet Numbering

| Key | Type | Default | C# Const |
|---|---|---|---|
| `AutoSheetNumbering` | bool(int) | 0 | `AutoSheetNumbering` |
| `AutoCrossReferences` | int | -1 | `AutoCrossReferences` |
| `NewIndexingOfSheetSymbols` | bool(int) | 0 | `NewIndexingOfSheetSymbols` |

`AutoCrossReferences`: -1 = undefined, 0 = disabled, 1 = enabled

#### Error Reporting

| Key | Type | Default | C# Const |
|---|---|---|---|
| `ReportSuppressedErrorsInMessages` | bool(int) | 0 | — |

#### Build / Output

| Key | Type | Default |
|---|---|---|
| `ReleasesFolder` | string | `""` |
| `OpenOutputs` | bool(int) | 1 |
| `ArchiveProject` | bool(int) | 0 |
| `TimestampOutput` | bool(int) | 0 |
| `SeparateFolders` | bool(int) | 0 |
| `TemplateLocationPath` | string | `""` |
| `DefaultConfiguration` | string | `"Sources"` |
| `OutputPath` | string | `""` |
| `LogFolderPath` | string | `""` |
| `IncludeDesignInRelease` | bool(int) | 0 |

#### PCB Defaults

| Key | Type | Default |
|---|---|---|
| `DefaultPcbProtel` | bool(int) | 1 |
| `DefaultPcbPcad` | bool(int) | 0 |

#### Misc

| Key | Type | Default | Notes |
|---|---|---|---|
| `UserID` | string | `"0xFFFFFFFF"` | Hex string |
| `ReorderDocumentsOnCompile` | bool(int) | 1 | |
| `PushECOToAnnotationFile` | bool(int) | 1 | |
| `DItemRevisionGUID` | string | `""` | |
| `FSMCodingStyle` | string | `"eFMSDropDownList_OneProcess"` | |
| `FSMEncodingStyle` | string | `"eFMSDropDownList_OneHot"` | |
| `IsProjectConflictPreventionWarningsEnabled` | bool(int) | 0 | |
| `ConstraintManagerFlow` | bool(int) | 0 | |
| `IsVirtualBomDocumentRemoved` | bool(int) | 0 | |
| `ManagedProjectGUID` | string | `""` | |

---

### `[Preferences]`

| Key | Type |
|---|---|
| `PrefsVaultGUID` | string |
| `PrefsRevisionGUID` | string |

---

### `[Document{N}]` — Per-Document Entries

Numbered sections: `[Document1]`, `[Document2]`, etc.

| Key | Type | Default | Notes |
|---|---|---|---|
| `DocumentPath` | string | — | Relative path from project dir |
| `DocumentUniqueId` | string | — | 8-char ID (e.g. `"IIEGGIJT"`) |
| `AnnotationEnabled` | bool(int) | 1 | |
| `AnnotateStartValue` | int | 1 | |
| `AnnotationIndexControlEnabled` | bool(int) | 0 | |
| `AnnotateSuffix` | string | `""` | |
| `AnnotateScope` | string | `"All"` | `"All"`, `"Ignore Selected Parts"`, `"Only Selected Parts"` |
| `AnnotateOrder` | int | -1 | |
| `DoLibraryUpdate` | bool(int) | 1 | |
| `DoDatabaseUpdate` | bool(int) | 1 | |
| `ClassGenCCAutoEnabled` | bool(int) | 1 | |
| `ClassGenCCAutoRoomEnabled` | bool(int) | 0 | |
| `ClassGenNCAutoScope` | string | `"None"` | `"None"`, `"Local Nets Only"`, `"All Nets"` |
| `GenerateClassCluster` | bool(int) | 0 | |
| `DItemRevisionGUID` | string | `""` | |

The `DocumentUniqueId` values cross-reference to the `UniqueID` in each document's
FileHeader stream (see `docs/dxp/file-headers.md` § UniqueID Cross-Reference).

---

### `[Configuration{N}]` — Build Configurations

| Key | Type | Notes |
|---|---|---|
| `Name` | string | e.g. `"Sources"` |
| `ParameterCount` | int | |
| `ConstraintFileCount` | int | |
| `ReleaseItemId` | string | |
| `Variant` | string | e.g. `"[No Variations]"` |
| `OutputJobsCount` | int | |
| `ContentTypeGUID` | string | GUID |
| `ConfigurationType` | string | e.g. `"Source"` |

---

### `[OutputGroup{N}]` — Output Job Groups

The blank project has 10 output groups:

| N | Name |
|---|---|
| 1 | Netlist Outputs |
| 2 | Simulator Outputs |
| 3 | Documentation Outputs |
| 4 | Assembly Outputs |
| 5 | Fabrication Outputs |
| 6 | Report Outputs |
| 7 | Other Outputs |
| 8 | Validation Outputs |
| 9 | Export Outputs |
| 10 | PostProcess Outputs |

#### Group-level keys

| Key | Type |
|---|---|
| `Name` | string |
| `Description` | string |
| `TargetPrinter` | string |
| `PrinterOptions` | pipe-delimited |

#### Per-output keys (indexed 1-based)

| Key Pattern | Type | Notes |
|---|---|---|
| `OutputType{M}` | string | Generator type identifier |
| `OutputName{M}` | string | Display name |
| `OutputDocumentPath{M}` | string | |
| `OutputVariantName{M}` | string | e.g. `"[No Variations]"` |
| `OutputDefault{M}` | bool(int) | |
| `PageOptions{M}` | pipe-delimited | Only present for some outputs |

---

### `[Modification Levels]`

Format: `Type{N}=level` (1-based)

- N = `TDifferenceKind` enum value + 1
- level = `TModificationLevel` (0=Off, 1=On)
- Blank project: 161 entries, all set to `1`

---

### `[Difference Levels]`

Format: `Type{N}=level` (1-based)

- N = `TDifferenceKind` enum value + 1
- level = `TDifferenceCheckLevel` (0=Off, 1=On, 2=On_CaseSensitive)
- Blank project: 88 entries, all set to `1`

---

### `[Electrical Rules Check]`

Format: `Type{N}=level` (1-based)

- N = `TErrorKind` enum value + 1
- level = `TErrorLevel` (0=NoReport, 1=Warning, 2=Error, 3=Fatal)
- Special named keys (outside the Type{N} pattern):
  - `MultiChannelAlternate=0`
  - `AlternateItemFail=3`
- Blank project: 165 Type entries + 2 named entries

---

### `[ERC Connection Matrix]`

17×17 matrix of `TErrorLevel` values encoded as single characters.

Format: `L{N}=<17-char string>` where N = 1..17 (row index, 1-based)

Characters: `N`=NoReport(0), `W`=Warning(1), `E`=Error(2), `F`=Fatal(3)

Rows/columns correspond to `TConnectionCode` (0..16):

| Index | Code | Category |
|---|---|---|
| 0 | `eCC_PinInput` | Pin |
| 1 | `eCC_PinIO` | Pin |
| 2 | `eCC_PinOutput` | Pin |
| 3 | `eCC_PinOpenCollector` | Pin |
| 4 | `eCC_PinPassive` | Pin |
| 5 | `eCC_PinHiZ` | Pin |
| 6 | `eCC_PinOpenEmitter` | Pin |
| 7 | `eCC_PinPower` | Pin |
| 8 | `eCC_PortInput` | Port |
| 9 | `eCC_PortOutput` | Port |
| 10 | `eCC_PortBidirectional` | Port |
| 11 | `eCC_PortUnspecified` | Port |
| 12 | `eCC_SheetEntryInput` | Sheet Entry |
| 13 | `eCC_SheetEntryOutput` | Sheet Entry |
| 14 | `eCC_SheetEntryBidirectional` | Sheet Entry |
| 15 | `eCC_SheetEntryUnspecified` | Sheet Entry |
| 16 | `eCC_UnConnected` | Unconnected |

The matrix is symmetric. Example row from blank project:
```
L3=NWEENEEEENEWNEEWN
```
This is row 2 (0-indexed: `eCC_PinOutput`). Reading left-to-right:
N(Input), W(IO), E(Output), E(OC), N(Passive), E(HiZ), E(OE), E(Power), …

---

### `[Annotate]`

| Key | Type | Enum |
|---|---|---|
| `SortOrder` | int | `TSortOrder` |
| `SortLocation` | int | `TSortLocation` |
| `ReplaceSubparts` | int | `TReplaceSubparts` (0=Off, 1=On) |
| `MatchParameter{N}` | string | Parameter name for matching |
| `MatchStrictly{N}` | bool(int) | Strict match for MatchParameter{N} |
| `PhysicalNamingFormat` | string | |
| `GlobalIndexSortOrder` | int | `TSortOrder` |
| `GlobalIndexSortLocation` | int | `TSortLocation` |

---

### `[PrjClassGen]`

| Key | Type | Default |
|---|---|---|
| `CompClassManualEnabled` | bool(int) | 0 |
| `CompClassManualRoomEnabled` | bool(int) | 0 |
| `NetClassAutoBusEnabled` | bool(int) | 1 |
| `NetClassAutoCompEnabled` | bool(int) | 0 |
| `NetClassAutoNamedHarnessEnabled` | bool(int) | 0 |
| `NetClassManualEnabled` | bool(int) | 1 |
| `NetClassSeparateForBusSections` | bool(int) | 0 |

---

### `[LibraryUpdateOptions]`

#### Global flags

| Key | Type |
|---|---|
| `SelectedOnly` | bool(int) |
| `UpdateVariants` | bool(int) |
| `UpdateToLatestRevision` | bool(int) |
| `PartTypes` | int |
| `FullReplace` | bool(int) |
| `UpdateDesignatorLock` | bool(int) |
| `UpdatePartIDLock` | bool(int) |
| `PreserveParameterLocations` | bool(int) |
| `PreserveParameterVisibility` | bool(int) |
| `DoGraphics` | bool(int) |
| `DoParameters` | bool(int) |
| `DoModels` | bool(int) |
| `AddParameters` | bool(int) |
| `RemoveParameters` | bool(int) |
| `AddModels` | bool(int) |
| `RemoveModels` | bool(int) |
| `UpdateCurrentModels` | bool(int) |

#### Per-component records (indexed, groups of 6)

```
ComponentLibIdentifierKind{N}=<kind>
ComponentLibraryIdentifier{N}=<identifier>
ComponentDesignItemID{N}=<id>
ComponentSymbolReference{N}=<ref>
ComponentUpdate{N}=<0|1>
ComponentIsDeviceSheet{N}=<0|1>
```

---

### `[DatabaseUpdateOptions]`

| Key | Type |
|---|---|
| `SelectedOnly` | bool(int) |
| `UpdateVariants` | bool(int) |
| `UpdateToLatestRevision` | bool(int) |
| `PartTypes` | int |

---

### `[Comparison Options]`

Pipe-delimited comparison rules, indexed from 0:

```
ComparisonOptions0=Kind=Net|MinPercent=75|MinMatch=3|ShowMatch=0|UseName=-1|InclAllRules=0
ComparisonOptions1=Kind=Net Class|MinPercent=75|MinMatch=3|ShowMatch=0|UseName=-1|InclAllRules=0
ComparisonOptions2=Kind=Component Class|…
ComparisonOptions3=Kind=Rule|…
ComparisonOptions4=Kind=Differential Pair|MinPercent=50|MinMatch=1|…
ComparisonOptions5=Kind=Structure Class|…
```

Sub-keys within each pipe-delimited value:

| Sub-Key | Type | Notes |
|---|---|---|
| `Kind` | string | Entity kind being compared |
| `MinPercent` | int | Minimum similarity percentage |
| `MinMatch` | int | Minimum match count |
| `ShowMatch` | bool(int) | Show matching entries |
| `UseName` | int | -1=auto, 0=no, 1=yes |
| `InclAllRules` | int | Include all rules |

---

### `[SmartPDF]`

```
PageOptions=Record=PageOptions|CenterHorizontal=True|…
```

Same pipe-delimited `PageOptions` record format as `[OutputGroup{N}]`.

---

### Sections Not in Blank Project (parsed by code)

#### `[ProjectVariant{N}]`

```
[ProjectVariant1]
UniqueID=<GUID>
Description=<string>
OverwritePCBFootprint=<0|1>
Variation1=Designator=<des>|UniqueId=<uid>|Kind=<kind>|AlternatePart=<part>|AltLibLink_*=<value>
ParamVariation1=ParameterName=<name>|VariantValue=<value>
```

`TVariationKind`: 0=None, 1=NotFitted, 2=Alternate

#### `[Parameter{N}]`

```
[Parameter1]
Name=<parameter name>
Value=<parameter value>
```

If `Value` is `"*"`, treated as empty string. Sections with `_` in name
(e.g. `[Parameter1_2]`) are variant-specific parameters.

#### `[DiffPairSuffix{N}]`

```
[DiffPairSuffix1]
Positive=<suffix>
Negative=<suffix>
```

#### `[NetInfos]`

```
[NetInfos]
Net1=NetName=<name>|NetColor=<color>
```

Color format: named color string or `$XXXXXXXX` hex prefix.

#### `[UniqueIdsMappings]`

```
[UniqueIdsMappings]
Mapping1=SchHandle=<handle>|UniqueIdMapping=<mapping>
```

#### `[DeviceSheetFolder{N}]`

```
[DeviceSheetFolder1]
Path=<directory path>
IncludeSubFolders=<0|1>
```

---

## Enum Reference

### `TFlattenMode` (HierarchyMode)

| Value | Variant | Description |
|---|---|---|
| 0 | `eFlatten_Smart` | Auto-detect hierarchy |
| 1 | `eFlatten_Flat` | Flat design |
| 2 | `eFlatten_Hierarchical_GlobalPorts` | Hierarchical with global ports |
| 3 | `eFlatten_Global` | Global scope |
| 4 | `eFlatten_Hierarchical_Strict` | Strict hierarchical |

### `TChannelRoomNamingStyle` (ChannelRoomNamingStyle)

| Value | Variant |
|---|---|
| 0 | `FlatNumericWithNames` |
| 1 | `FlatAlphaWithNames` |
| 2 | `NumericNamePath` |
| 3 | `AlphaNamePath` |
| 4 | `MixedNamePath` |

### `TCrossRefSheetStyle` (CrossRefSheetStyle)

| Value | Variant |
|---|---|
| 0 | None |
| 1 | Name |
| 2 | Number |

### `TCrossRefLocationStyle` (CrossRefLocationStyle)

| Value | Variant |
|---|---|
| 0 | None |
| 1 | Zone |
| 2 | XY |

### `TCrossRefPorts` (CrossRefPorts)

| Value | Variant |
|---|---|
| 0 | Disabled |
| 1 | SheetEntry |
| 2 | Ports |
| 3 | SheetEntryAndPorts |

### `TSortOrder` (SortOrder, GlobalIndexSortOrder)

| Value | Variant |
|---|---|
| 0 | UpThenAcross |
| 1 | DownThenAcross |
| 2 | AcrossThenUp |
| 3 | AcrossThenDown |

### `TSortLocation` (SortLocation, GlobalIndexSortLocation)

| Value | Variant |
|---|---|
| 0 | Designator |
| 1 | Part |

### `TErrorLevel` (ERC + Connection Matrix)

| Value | Variant | Matrix Char |
|---|---|---|
| 0 | NoReport | `N` |
| 1 | Warning | `W` |
| 2 | Error | `E` |
| 3 | Fatal | `F` |

### `TDifferenceCheckLevel` (Difference Levels)

| Value | Variant |
|---|---|
| 0 | Off |
| 1 | On |
| 2 | On_CaseSensitive |

### `TModificationLevel` (Modification Levels)

| Value | Variant |
|---|---|
| 0 | Off |
| 1 | On |

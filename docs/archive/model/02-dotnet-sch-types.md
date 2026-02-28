# .NET Schematic Types and Data Model (AD26 Decompiled)

## Source Assemblies

The schematic data model is spread across these key assemblies:

| Assembly | Purpose |
|----------|---------|
| `Altium.Edp.Interfaces` (namespace `Rt_Schematic`) | Core enums, interfaces, constants |
| `Altium.Sch.Interfaces` | `ISchData*` interfaces for all schematic object types |
| `Altium.Sch.DataModel` | `SchData*` implementation classes (data objects) + engine objects + file format code |
| `Altium.Sch.Base` | Constants, collections, geometry, defaults |
| `Altium.Sch.Core` | Commands, collectors, constraints (higher-level operations) |
| `Altium.Sch.Layer2Base` | Layer 2 engine (connectivity, routing, interactive editing) |

---

## 1. TObjectId Enum (Record Type IDs)

Defined in `Altium.Edp.Interfaces/Rt_Schematic/TObjectId.cs`. This is the **master enum** that identifies every schematic record type. Values are zero-based sequential (no explicit integer assignments - the ordinal position IS the numeric ID).

```csharp
namespace Rt_Schematic;
public enum TObjectId
{
    eFirstObjectID,          // 0 - generic container
    eClipBoardContainer,     // 1
    eNote,                   // 2
    eProbe,                  // 3
    eRectangle,              // 4
    eLine,                   // 5
    eConnectionLine,         // 6
    eBusEntry,               // 7
    eArc,                    // 8
    eEllipticalArc,          // 9
    eRoundRectangle,         // 10
    eImage,                  // 11
    ePie,                    // 12
    eTextFrame,              // 13
    eRichTextDocument,       // 14
    eEllipse,                // 15
    eJunction,               // 16
    ePolygon,                // 17
    ePolyline,               // 18
    eWire,                   // 19
    eBus,                    // 20
    eBezier,                 // 21
    eLabel,                  // 22
    eHyperlink,              // 23
    eNetLabel,               // 24
    eDesignator,             // 25
    eSchComponent,           // 26
    eParameter,              // 27
    eParameterSet,           // 28
    eParameterList,          // 29
    eSheetName,              // 30
    eSheetFileName,          // 31
    eSheet,                  // 32
    eSchLib,                 // 33
    eSymbol,                 // 34
    eNoERC,                  // 35
    eErrorMarker,            // 36
    ePin,                    // 37
    ePort,                   // 38
    ePowerObject,            // 39
    eSheetEntry,             // 40
    eSheetSymbol,            // 41
    eTemplate,               // 42
    eTaskHolder,             // 43
    eMapDefiner,             // 44
    eImplementationMap,      // 45
    eImplementation,         // 46
    eImplementationsList,    // 47
    eCrossSheetConnector,    // 48
    eCompileMask,            // 49
    eOpenBusComponent,       // 50
    eOpenBusLink,            // 51
    eOpenBusDesignator,      // 52
    eHarnessConnector,       // 53
    eHarnessEntry,           // 54
    eHarnessConnectorType,   // 55
    eSignalHarness,          // 56
    eOpenBusPort,            // 57
    eHighLevelCodeSymbol,    // 58
    eHighLevelCodeEntry,     // 59
    eOpenBusPinGroup,        // 60
    eBlanket,                // 61
    eRTFLink,                // 62
    eFSMState,               // 63
    eFSMTransition,          // 64
    eCommentThread,          // 65
    eCommentThreadNote,      // 66
    eFSMNote,                // 67
    eDiagramModule,          // 68
    eDiagramModuleName,      // 69
    eDiagramModuleSource,    // 70
    eDiagramConnector,       // 71
    eDiagramBlock,           // 72
    eDiagramHarness,         // 73
    eDiagramHarnessName,     // 74
    eDiagramHarnessSource,   // 75
    eDiagramConnectorLink,   // 76
    eDiagramPin,             // 77
    eVirtualParameter,       // 78
    eHarnessWiringDiagram,   // 79
    eHarnessLayoutDrawing,   // 80
    eHarnessComponent,       // 81
    eHarnessWire,            // 82
    eHarnessSplice,          // 83
    eHarnessLayoutLabel,     // 84
    eHarnessLayoutConnectionPoint, // 85
    eHarnessBundle,          // 86
    eHarnessLogicalSignal,   // 87
    eHarnessPin,             // 88
    eHarnessWireLabel,       // 89
    eHarnessWireData,        // 90
    eHarnessSpliceData,      // 91
    eHarnessShield,          // 92
    eHarnessTwist,           // 93
    eHarnessNoConnect,       // 94
    eHarnessNoConnectData,   // 95
    eHarnessShieldData,      // 96
    eHarnessTwistData,       // 97
    eHarnessCable,           // 98
    eHarnessCableData,       // 99
    eImageParameter,         // 100
    eHarnessAssociatedParts, // 101
    eHarnessLibrary,         // 102
    eLineView,               // 103
    eHarnessCovering,        // 104
    eObjectDefinition,       // 105
    eHarnessWireBreak,       // 106
    eAssociatedObjects,      // 107
    eElectronicsSystemDesignDocument, // 108
    eFunctionalBlock,        // 109
    eFunctionalConnectionLine, // 110
    eFunctionalTextFrame,    // 111
    eSchematicBlock,         // 112
    eReuseSheetSymbol,       // 113
    eReuseBlockImplementationInfo, // 114
    eLastObjectId            // 115 (sentinel)
}
```

**NOTE**: The `RECORD` field in the binary/text format uses the same numeric values (0-based ordinal of this enum). However, in the legacy binary V4 format, a separate mapping is used - see `SchDataImporterSheetV4Binary.cs`.

### TExtendedObjectId

For specializations of a base TObjectId:

```csharp
public enum TExtendedObjectId
{
    xNoObjectId,         // 0 - normal object
    xIntelligentWire,    // 1 - auto-routing wire (extends eWire)
    xReviewGraphicalObject, // 2 - review/comment graphical object
    xNativeImport        // 3 - native import sheet (extends eSheet)
}
```

---

## 2. Class Inheritance Hierarchy

The data model has a clear layered inheritance structure:

```
SchDataObject (base: TObjectId, owner, uniqueId, container references)
  +-- SchDataContainer (adds child object list, Add/Remove/Iterate)
       +-- SchDataGraphicalObject (adds location, color, areaColor, ownerPartId, displayMode)
            |-- SchDataLabel (text, fontID, orientation, justification, url)
            |     |-- SchDataComplexText (formulaText)
            |     |     |-- SchDataParameter (name, isHidden, autoPosition, showName)
            |     |     |     +-- SchDataDesignator
            |     |     |     +-- SchDataImageParameter (image data + params)
            |     |     +-- SchDataSheetName
            |     |     +-- SchDataSheetFileName
            |     |-- SchDataNetLabel (extends Label)
            |     |-- SchDataHyperlink
            |     +-- SchDataPower (powerObjectStyle, showNetName)
            |           +-- SchDataCrossSheetConnector
            |-- SchDataRectangle (corner location, lineWidth, solid, transparent)
            |     |-- SchDataRoundRectangle (cornerRadiusX/Y)
            |     |-- SchDataTextFrame (text, fontID, wordWrap, showBorder, alignment)
            |     |     |-- SchDataNote (author, collapsed)
            |     |     +-- SchDataFunctionalTextFrame
            |     |-- SchDataImage (embedded image data, keepAspect)
            |     +-- SchDataFunctionalBlock
            |-- SchDataCircle (radius, lineWidth, solid)
            |     |-- SchDataEllipse (secondaryRadius)
            |     +-- SchDataJunction (junctionSize)
            |-- SchDataArc (radius, lineWidth, startAngle, endAngle)
            |     +-- SchDataPie (solid)
            |-- SchDataEllipticalArc (radius, secondaryRadius, lineWidth, angles)
            |-- SchDataLine (cornerX/Y, lineWidth, lineStyle, startShape, endShape)
            |     +-- SchDataConnectionLine
            |-- SchDataBusEntry (corner location)
            |-- SchDataPolygon (vertices, lineWidth, solid, transparent)
            |     |-- SchDataStraightPolygon
            |     |     +-- SchDataPolyline (startShape, endShape, shapeSize)
            |     +-- SchDataWire (underlineColor, connectedJunctions)
            |           |-- SchDataBus
            |           |-- SchDataSignalHarness
            |           +-- SchDataParametrizedWire (abstract: adds parametrized group capabilities)
            |                 +-- SchDataFunctionalConnectionLine
            |-- SchDataBezier (vertices, lineWidth)
            |-- SchDataBlanket (vertices, name)
            |-- SchDataNoERC (active, suppressSpecific)
            |-- SchDataErrorMarker (errorKind)
            |-- SchDataCompileMask
            |-- SchDataClipBoardContainer
            |-- SchDataRichTextDocument (stream data)
            |-- SchDataRTFLink
            |-- SchDataCommentThread
            |-- SchDataCommentThreadNote
            |-- SchDataLineView
            |-- SchDataObjectDefinition
            |-- SchDataParametrizedGroup (abstract: children + parameter management)
            |     |-- SchDataPin (name, designator, electrical, orientation, pinLength, hidden, IEEE symbols, swap IDs)
            |     |-- SchDataComponent (libReference, libraryPath, orientation, mirrored, partCount, displayMode, comment, designator, footprint, vault/item GUIDs)
            |     |-- SchDataPort (name, style, IOType, alignment, width, height, fontID)
            |     |-- SchDataParameterSet (style)
            |     |     +-- SchDataProbe
            |     |-- SchDataParameterList
            |     |-- SchDataTaskHolder (process, instanceName, configuration)
            |     |-- SchDataSymbol
            |     |-- SchDataHighLevelCodeSymbol
            |     |-- SchDataTemplate
            |     |-- SchDataMapDefiner
            |     |-- SchDataImplementation (modelName, modelType, isCurrent)
            |     |-- SchDataImplementationMap
            |     |-- SchDataImplementationList
            |     |-- SchDataRectangularGroup (xSize, ySize)
            |     |     |-- SchDataRectangularEntryContainer (entry management)
            |     |     |     |-- SchDataSheetSymbol (fileName, entries)
            |     |     |     +-- SchDataHarnessConnector
            |     |     +-- SchDataOpenBusPinGroup
            |     |-- SchDataDocument (grid, borders, template, displayUnit, sheet properties)
            |     |     |-- SchDataSheet (unique ID management, reuse blocks)
            |     |     +-- SchDataLibrary (componentList, currentComponent, description)
            |     +-- SchDataSchematicBlock (reuse block)
            +-- SchDataBasicEntry (side, distanceFromTop, name, IOType, fontID)
                  |-- SchDataSheetEntry
                  +-- SchDataHarnessEntry
```

---

## 3. Key Enums

### TPinElectrical (pin electrical type)
```csharp
public enum TPinElectrical {
    eElectricInput,       // 0
    eElectricIO,          // 1
    eElectricOutput,      // 2
    eElectricOpenCollector, // 3
    eElectricPassive,     // 4
    eElectricHiZ,         // 5
    eElectricOpenEmitter, // 6
    eElectricPower        // 7
}
```

### TRotationBy90
```csharp
public enum TRotationBy90 {
    eRotate0,   // 0
    eRotate90,  // 1
    eRotate180, // 2
    eRotate270  // 3
}
```

### TTextJustification
```csharp
public enum TTextJustification {
    eJustify_BottomLeft,   // 0
    eJustify_BottomCenter, // 1
    eJustify_BottomRight,  // 2
    eJustify_CenterLeft,   // 3
    eJustify_Center,       // 4
    eJustify_CenterRight,  // 5
    eJustify_TopLeft,      // 6
    eJustify_TopCenter,    // 7
    eJustify_TopRight      // 8
}
```

### TIeeeSymbol (IEEE pin symbols)
```csharp
public enum TIeeeSymbol {
    eNoSymbol,           // 0
    eDot,                // 1
    eRightLeftSignalFlow, // 2
    eClock,              // 3
    eActiveLowInput,     // 4
    eAnalogSignalIn,     // 5
    eNotLogicConnection, // 6
    eShiftRight,         // 7
    ePostPonedOutput,    // 8
    eOpenCollector,      // 9
    eHiz,                // 10
    eHighCurrent,        // 11
    ePulse,              // 12
    eSchmitt,            // 13
    eDelay,              // 14
    eGroupLine,          // 15
    eGroupBin,           // 16
    eActiveLowOutput,    // 17
    ePiSymbol,           // 18
    eGreaterEqual,       // 19
    eLessEqual,          // 20
    eSigma,              // 21
    eOpenCollectorPullUp, // 22
    eOpenEmitter,        // 23
    eOpenEmitterPullUp,  // 24
    eDigitalSignalIn,    // 25
    eAnd,                // 26
    eInvertor,           // 27
    eOr,                 // 28
    eXor,                // 29
    eShiftLeft,          // 30
    eInputOutput,        // 31
    eOpenCircuitOutput,  // 32
    eLeftRightSignalFlow, // 33
    eBidirectionalSignalFlow, // 34
    eInternalPullUp,     // 35
    eInternalPullDown    // 36
}
```

### TComponentKind
```csharp
public enum TComponentKind {
    eComponentKind_Standard,      // 0
    eComponentKind_Mechanical,    // 1
    eComponentKind_Graphical,     // 2
    eComponentKind_NetTie_BOM,    // 3
    eComponentKind_NetTie_NoBOM,  // 4
    eComponentKind_Standard_NoBOM, // 5
    eComponentKind_Jumper         // 6
}
```

### TStdLogicState (VHDL formal type)
```csharp
public enum TStdLogicState {
    eStdLogic_Unitialized,    // 0
    eStdLogic_ForcingUnknown, // 1
    eStdLogic_Forcing0,       // 2
    eStdLogic_Forcing1,       // 3
    eStdLogic_HiZ,            // 4
    eStdLogic_WeakUnknown,    // 5
    eStdLogic_Weak0,          // 6
    eStdLogic_Weak1,          // 7
    eStdLogic_DontCare        // 8
}
```

### TPortArrowStyle
```csharp
public enum TPortArrowStyle {
    ePortNone,          // 0
    ePortLeft,          // 1
    ePortRight,         // 2
    ePortLeftRight,     // 3
    ePortNoneVertical,  // 4
    ePortTop,           // 5
    ePortBottom,        // 6
    ePortTopBottom      // 7
}
```

### TPortIO
```csharp
public enum TPortIO {
    ePortUnspecified,   // 0
    ePortOutput,        // 1
    ePortInput,         // 2
    ePortBidirectional  // 3
}
```

### TPowerObjectStyle
```csharp
public enum TPowerObjectStyle {
    ePowerCircle,       // 0
    ePowerArrow,        // 1
    ePowerBar,          // 2
    ePowerWave,         // 3
    ePowerGndPower,     // 4
    ePowerGndSignal,    // 5
    ePowerGndEarth,     // 6
    eGOSTPowerArrow,    // 7
    eGOSTPowerGndPower, // 8
    eGOSTPowerGndEarth, // 9
    eGOSTPowerBar       // 10
}
```

### TCrossSheetConnectorStyle
```csharp
public enum TCrossSheetConnectorStyle {
    eCrossSheetLeft,  // 0
    eCrossSheetRight  // 1
}
```

### TLineStyle
```csharp
public enum TLineStyle {
    eLineStyleSolid,       // 0
    eLineStyleDashed,      // 1
    eLineStyleDotted,      // 2
    eLineStyleDashDotted   // 3
}
```

### TLineShape (line endpoint shapes)
```csharp
public enum TLineShape {
    eLineShapeNone,       // 0
    eLineShapeArrow,      // 1
    eLineShapeSolidArrow, // 2
    eLineShapeTail,       // 3
    eLineShapeSolidTail,  // 4
    eLineShapeCircle,     // 5
    eLineShapeSquare      // 6
}
```

### TSize (pen/border width)
```csharp
public enum TSize {
    eZeroSize, // 0
    eSmall,    // 1
    eMedium,   // 2
    eLarge     // 3
}
```

### TSheetStyle
```csharp
public enum TSheetStyle {
    eSheetA4,     eSheetA3,     eSheetA2,     eSheetA1,   eSheetA0,
    eSheetA,      eSheetB,      eSheetC,      eSheetD,    eSheetE,
    eSheetLetter, eSheetLegal,  eSheetTabloid,
    eSheetOrcadA, eSheetOrcadB, eSheetOrcadC, eSheetOrcadD, eSheetOrcadE
}
```

### TSheetOrientation
```csharp
public enum TSheetOrientation { eLandscape, ePortrait }
```

### TSheetDocumentBorderStyle
```csharp
public enum TSheetDocumentBorderStyle { eSheetStandard, eSheetAnsi }
```

### TFileFormatVersion
```csharp
public enum TFileFormatVersion : byte { ffv4, ffv5 }
```

---

## 4. Data Object Fields (per type)

### SchDataObject (base for everything)
- `TObjectId objectId` - type discriminant
- `ISch_BasicContainer owner` - engine object owner
- `bool isAccessible`
- `bool inContainer`, `SchDataObject ownerContainer`
- `int ownerIndexForSave`, `int indexInSheetForSave`
- `bool ignoreOnLoad`, `bool ignoreOnSave`
- `string uniqueId`, `string uniqueIdInReuseBlock`

### SchDataContainer (extends SchDataObject)
- `List<ISchDataObject> objectList` - child objects
- `int generalField` - general purpose field
- `string wiringDiagramOriginUniqueId`

### SchDataGraphicalObject (extends SchDataContainer)
- `TLocation location` (X, Y as int)
- `int ownerPartId` (which part of a multi-part component, -1 if none)
- `byte ownerPartDisplayMode`
- `bool graphicallyLocked`
- `byte selectionMemoryFlags`
- `int unionIndex`
- `uint color` (border/line color)
- `uint areaColor` (fill color)

### SchDataLabel (record 22)
- `int fontID`
- `string text`
- `string formulaText`
- `TTextJustification justification`
- `bool isMirrored`
- `TRotationBy90 orientation`
- `string url`

### SchDataParameter (record 27)
- Inherits label fields plus:
- `string name`
- `bool isHidden`
- `bool showName`
- `TAutoposition autoPosition`

### SchDataPin (record 37)
- `string name`, `string designator`, `string description`
- `TPinElectrical electrical`
- `TRotationBy90 orientation`
- `int pinLength`
- `bool isHidden`, `bool showName`, `bool showDesignator`
- `string hiddenNetName`, `string defaultValue`
- `TStdLogicState formalType` (VHDL formal type)
- `TIeeeSymbol symbolInner/Outer/InnerEdge/OuterEdge`
- `TSize symbolLineWidth`
- `string swapIdPin/Pair/PartAndPartPin`
- `int pinPackageLength`, `double pinPropagationDelay`
- Pin name/designator custom position and font fields (positionMode, fontMode, customFontID, customColor, customPositionMargin, customRotation*)
- `ISchDataPinFunctions definedFunctions/selectedFunctions`
- `string connectedObjectUniqueId`

### SchDataComponent (record 26)
- `string libraryPath`, `string libReference`, `string sourceLibraryName`
- `TRotationBy90 orientation`
- `bool isMirrored`
- `int partCount`, `int currentPartID`
- `byte displayMode`, `byte displayModeCount`
- `TComponentKind componentKind`
- `bool showHiddenPins`, `bool showHiddenFields`, `bool displayFieldNames`
- `bool designatorLocked`, `bool partIdLocked`, `bool pinsMoveable`
- `bool overideColors`, `uint pinColor`
- `string footprint`, `string componentDescription`
- `string designItemId`, `string databaseTableName`
- `bool useLibraryName`, `bool useDBTableName`
- `string targetFileName`, `string sheetPartFileName`
- `string vaultGUID/itemGUID/revisionGUID`
- `string symbolVaultGUID/symbolItemGUID/symbolRevisionGUID`
- `ISchDataDesignator designator` (child field object)
- `ISchDataParameter comment` (child field object)
- `ISchDataAliasList aliasList`
- `int allPinCount`, `int filePosition`

### SchDataPort (record 38)
- `string name`
- `TPortArrowStyle style`
- `TPortIO portIOType`
- `THorizontalAlign alignment`
- `int width`, `int height`
- `int fontID`
- `uint textColor`
- `TSize borderWidth`
- `bool autoSize`
- `string harnessType`
- `string objectDefinitionId`

### SchDataWire (record 19)
- Inherits from SchDataPolygon (vertices, lineWidth, solid)
- `uint underlineColor`
- `IReadOnlyList<string> connectedJunctionsUniqueIds`
- `string assignedInterface`, `string assignedInterfaceSignal`

### SchDataRectangle (record 4)
- Corner location (X, Y), lineWidth, solid, transparent

### SchDataSheetSymbol (record 41)
- Inherits SchDataRectangularEntryContainer
- `xSize, ySize` (width/height)
- `string fileName`
- Sheet entries as children

### SchDataDocument (records 32, 33)
- `TUnit displayUnit`
- `TSheetStyle sheetStyle`
- `TSheetOrientation workspaceOrientation`
- `TSheetDocumentBorderStyle documentBorderStyle`
- `bool useCustomSheet`, `int customX/Y/XZones/YZones/MarginWidth`
- `bool borderOn`, `bool referenceZonesOn`, `bool titleBlockOn`
- `bool snapGridOn/visibleGridOn/hotSpotGridOn`
- `int snapGridSize/visibleGridSize/hotSpotGridSize`
- `bool showTemplateGraphics`, `string templateFileName`
- `int systemFont`, `int minorVersion`
- `IDictionary<string, ISchDataObject> handledObjectList` (UniqueID -> object map)

---

## 5. Object Factory (SchDataModel.CreateDataModelObject)

The `SchDataModel.CreateDataModelObject()` method is the central factory. It maps TObjectId to concrete SchData* classes. Key mappings:

| TObjectId | Data Class |
|-----------|------------|
| eSheet (32) | SchDataSheet |
| eSchLib (33) | SchDataLibrary |
| eSchComponent (26) | SchDataComponent |
| ePin (37) | SchDataPin |
| eWire (19) | SchDataWire |
| eBus (20) | SchDataBus (extends SchDataWire) |
| ePort (38) | SchDataPort |
| ePowerObject (39) | SchDataPower |
| eNetLabel (24) | SchDataNetLabel |
| eLabel (22) | SchDataLabel |
| eParameter (27) | SchDataParameter |
| eDesignator (25) | SchDataDesignator |
| eRectangle (4) | SchDataRectangle |
| eRoundRectangle (10) | SchDataRoundRectangle |
| eImage (11) | SchDataImage |
| eTextFrame (13) | SchDataTextFrame |
| eNote (2) | SchDataNote |
| eArc (8) | SchDataArc |
| eEllipticalArc (9) | SchDataEllipticalArc |
| ePie (12) | SchDataPie |
| eEllipse (15) | SchDataEllipse |
| eJunction (16) | SchDataJunction |
| ePolygon (17) | SchDataPolygon |
| ePolyline (18) | SchDataPolyline |
| eBezier (21) | SchDataBezier |
| eLine (5) | SchDataLine |
| eConnectionLine (6) | SchDataConnectionLine |
| eBusEntry (7) | SchDataBusEntry |
| eSheetSymbol (41) | SchDataSheetSymbol |
| eSheetEntry (40) | SchDataSheetEntry |
| eNoERC (35) | SchDataNoERC |
| eProbe (3) | SchDataProbe |
| eParameterSet (28) | SchDataParameterSet |
| eParameterList (29) | SchDataParameterList |
| eTemplate (42) | SchDataTemplate |
| eImplementation (46) | SchDataImplementation |
| eImplementationMap (45) | SchDataImplementationMap |
| eImplementationsList (47) | SchDataImplementationList |
| eSymbol (34) | SchDataSymbol |
| eHarnessConnector (53) | SchDataHarnessConnector |
| eHarnessEntry (54) | SchDataHarnessEntry |
| eHarnessConnectorType (55) | SchDataHarnessConnectorType |
| eSignalHarness (56) | SchDataSignalHarness |
| eBlanket (61) | SchDataBlanket |
| eCompileMask (49) | SchDataCompileMask |
| eCrossSheetConnector (48) | SchDataCrossSheetConnector |
| eSchematicBlock (112) | SchDataSchematicBlock |

---

## 6. Serialization System

### File Format Versions
- **V4** (`TFileFormatVersion.ffv4`): Legacy binary format, used by `SchDataImporterSheetV4Binary` and `FileFormatV4`
- **V5** (`TFileFormatVersion.ffv5`): Current text-based format (pipe-delimited key=value pairs), used by `FileFormatV5`

### Serialization Architecture

The serialization is handled by:
1. **`ISchDataSerializer`** - interface providing Import/Export methods for each data type
2. **`FileFormatV5`** (extends `FileFormatBase`) - contains Import/Export method pairs for every object type
3. **`SchDataImporterSheetV5`** / **`SchDataImporterDocumentV5`** - top-level document importers

### Serializer Methods (ISchDataSerializer)
The serializer provides typed import/export for each field type:
- `Import_Coord` / `Export_Coord` - coordinate values (integers in internal units)
- `Import_Color` / `Export_Color` - uint32 color values
- `Import_Boolean` / `Export_Boolean`
- `Import_Size` / `Export_Size` - TSize enum
- `Import_Angle` / `Export_Angle` - double angle values
- `Import_DynamicString` / `Export_DynamicString` - variable-length strings
- `Import_ASCIIOnlyString` / `Export_ASCIIOnlyString`
- `Import_ASCIIOnlyLongInt` / `Export_ASCIIOnlyLongInt`
- `Import_Instruction` - reads a "RECORD" field from the serialized data

### Serialization Pattern (FileFormatV5)

Each object type has paired `ImportXxx` / `ExportXxx` methods. Example for Arc:

```csharp
protected override void ImportArc(ISchDataSerializer serializer, ISchDataObject obj) {
    ImportGraphicalObject(serializer, obj);  // base fields
    if (obj is ISchDataArc arc) {
        TLocation location = default;
        serializer.Import_Coord(ref location.X, "Location.X");
        serializer.Import_Coord(ref location.Y, "Location.Y");
        arc.SetLocation(location);
        int radius = 0;
        serializer.Import_Coord(ref radius, "Radius");
        arc.SetRadius(radius);
        TSize lineWidth = TSize.eZeroSize;
        serializer.Import_Size(ref lineWidth, "LineWidth");
        arc.SetLineWidth(lineWidth);
        double startAngle = 0.0;
        serializer.Import_Angle(ref startAngle, "StartAngle");
        arc.SetStartAngle(startAngle);
        double endAngle = 0.0;
        serializer.Import_Angle(ref endAngle, "EndAngle");
        arc.SetEndAngle(endAngle);
        uint color = 0;
        serializer.Import_Color(ref color, "Color");
        arc.SetColor(color);
        string uniqueId = "";
        serializer.Import_DynamicString(ref uniqueId, "UniqueID");
        arc.SetUniqueId(uniqueId);
    }
}
```

The field names in these serializer calls (`"Location.X"`, `"Radius"`, etc.) correspond **exactly** to the key names used in the pipe-delimited text format records.

### Import Document Code
- The imported document binary code for SchDoc is **31** (from `SchDataImporterSheetV5.GetImportedDocumentBinaryCode()`)

---

## 7. SchDoc vs SchLib Structure

### SchDoc (Schematic Document)
- Root object: `SchDataSheet` (TObjectId = eSheet, record 32)
- Contains flat list of graphical objects (wires, components, labels, etc.) as children
- Components in SchDoc are placed instances referencing library symbols
- Components have `libraryPath` and `libReference` to link back to library

### SchLib (Schematic Library)
- Root object: `SchDataLibrary` (TObjectId = eSchLib, record 33)
- Contains a `componentList` (`ISchDataObjectList`) of component definitions
- Each component (`SchDataComponent`) contains its pins, graphical primitives, parameters
- Has a `currentComponent` pointer for the active component being edited
- Library metadata: `description`, `folderGUID`, `lifeCycleDefinitionGUID`, `revisionNamingSchemeGUID`

Both share the base `SchDataDocument` class which provides:
- Sheet/grid settings
- Template management
- Object handle (UniqueID) management via `handledObjectList` dictionary
- Font management

---

## 8. Coordinate System and Units

From `Rt_Schematic.Consts`:
- **Base unit**: `cBaseUnit = 100000` (internal units per DXP unit)
- **Internal precision**: `cInternalPrecision = 10000`
- Coordinates are stored as **integers** in internal units
- Conversion constants: `c1_00MM = 393701` (1mm = 393701 internal units)
- This means **1 internal unit = ~2.54 nanometers** (100000 units = 1 DXP unit = 2.54mm/100 = 0.0254mm)
- Max workspace: 6500 DXP units = 650,000,000 internal units

### Special Characters
- `C_SCH_VERTICAL_BAR = '|'` (124) - field separator in V5 text format
- `C_SCH_SPECIAL_DELIMITER_CHAR = '\u008e'` (142) - special delimiter
- `C_SCH_UTF8_PREFIX = "%UTF8%"` - UTF-8 encoding marker
- `cCoordFractionalPartSaveExtensionName = "_Frac"` - fractional coordinate extension

---

## 9. Container/Ownership Model

Objects form a tree via the container system:
- Each `SchDataObject` has an `ownerContainer` (parent in the tree)
- `SchDataContainer` maintains `objectList` (children)
- The `ownerIndexForSave` field stores the parent's index in the flat save list
- During save, the tree is flattened: `AddToListForSave()` recursively serializes children after parent
- Some objects go to an "additional list" (like compile masks) - controlled by `AdditionalObjectObjectIdSet`

### Field Objects
Some objects have "field objects" - child objects that are structurally part of the parent but stored as separate records:
- `SchDataComponent` has `designator` and `comment` as field objects
- These are iterated via `GetFieldObjects()` override
- Field objects get their own records in the serialized format

---

## 10. Important Constants and Arrays

From `Rt_Schematic.Consts` (the large constants file):
- `SchPrimitiveArray` - set of "primitive" schematic objects
- `SpatialObjectsArray` - objects that participate in spatial queries
- `VirtualObjectsSet` - virtual objects not directly saved
- `ReuseBlockEditableObjectsArray` - objects editable inside reuse blocks
- `cAlwaysOnTopObjectsIds` - objects always drawn on top
- `cDefaultPinLength` - default pin length per unit system
- `cDefaultPortWidth` - default port width per unit system
- `cDefaultElectricalGridSize_3` - electrical grid sizes
- `cDefaultCustomSizeX_Sheet` / `cDefaultCustomSizeY_Sheet` - default custom sheet sizes

---

## 11. TObjectAttribute Enum

This massive enum (`Rt_Schematic.TObjectAttribute`) defines every queryable/settable attribute on schematic objects. It has 210+ members covering all aspects: colors, locations, pin properties, component properties, text properties, port/power/sheet attributes, harness attributes, etc. Key groups:

- **General**: ObjectId, Color, AreaColor, LocationX/Y, CornerLocationX/Y, Width, Height
- **Text**: StringText, FontId, Orientation, HorizontalJustification, VerticalJustification
- **Line**: LineStyle, LineWidth, StartLineShape, EndLineShape
- **Pin**: PinDesignator, PinElectrical, PinLength, PinIeeeSymbol*, PinFormalType, PinSwapId*
- **Component**: SchComponentLibraryName, SchComponentLibReference, SchComponentDesignator, SchComponentPartId, SchComponentKind
- **Port**: PortArrowStyle, PortIOType
- **Power**: PowerObjectStyle, PowerObjectShowNetName
- **Sheet**: SheetFileName, SheetName, SheetEntrySide

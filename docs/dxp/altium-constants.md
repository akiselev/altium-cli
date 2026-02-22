AllLayers

AllLayers = [MinLayer..eConnectLayer];
AllObjects

AllObjects = [FirstObjectId..LastObjectId];
AllPrimitives

AllPrimitives = [ eArcObject ,
eViaObject ,
eTrackObject ,
eTextObject ,
eFillObject ,
ePadObject ,
eComponentObject ,
eNetObject ,
ePolyObject ,
eDimensionObject ,
eCoordinateObject ,
eEmbeddedObject ,
eEmbeddedBoardObject,
eFromToObject ,
eConnectionObject,
ePolyRegionObject,
eComponentBodyObject
];

cAdvPCB
cAdvPCB = 'AdvPCB';

cBoardSideStrings constant
cBoardSideStrings : Array [TBoardSide] Of String[20] =
('Top Side','Bottom Side');
cComonentCollisionCheckModeStrings constant
cComponentCollisionCheckModeStings : Array [TComponentCollisionCheckMode] Of String[22]= ('Quick Check Mode','Multi-Layer Check Mode','Full Check Mode','Component Body Mode');
cDefaultLayerDrawingOrder constant
cDefaultLayerDrawingOrder : TDrawingOrderArray = (
eBackGroundLayer,
eMultiLayer,
eTopOverlay,
eBottomOverlay,
eConnectLayer,
eNoLayer,
eTopLayer,
eMidLayer1,
eMidLayer2,
eMidLayer3,
eMidLayer4,
eMidLayer5,
eMidLayer6,
eMidLayer7,
eMidLayer8,
eMidLayer9,
eMidLayer10,
eMidLayer11,
eMidLayer12,
eMidLayer13,
eMidLayer14,
eMidLayer15,
eMidLayer16,
eMidLayer17,
eMidLayer18,
eMidLayer19,
eMidLayer20,
eMidLayer21,
eMidLayer22,
eMidLayer23,
eMidLayer24,
eMidLayer25,
eMidLayer26,
eMidLayer27,
eMidLayer28,
eMidLayer29,
eMidLayer30,
eBottomLayer,
eTopPaste,
eBottomPaste,
eTopSolder,
eBottomSolder,
eInternalPlane1,
eInternalPlane2,
eInternalPlane3,
eInternalPlane4,
eInternalPlane5,
eInternalPlane6,
eInternalPlane7,
eInternalPlane8,
eInternalPlane9,
eInternalPlane10,
eInternalPlane11,
eInternalPlane12,
eInternalPlane13,
eInternalPlane14,
eInternalPlane15,
eInternalPlane16,
eDrillGuide,
eKeepOutLayer,
eMechanical1,
eMechanical2,
eMechanical3,
eMechanical4,
eMechanical5,
eMechanical6,
eMechanical7,
eMechanical8,
eMechanical9,
eMechanical10,
eMechanical11,
eMechanical12,
eMechanical13,
eMechanical14,
eMechanical15,
eMechanical16,
eMechanical17,
eMechanical18,
eMechanical19,
eMechanical20,
eMechanical21,
eMechanical22,
eMechanical23,
eMechanical24,
eMechanical25,
eMechanical26,
eMechanical27,
eMechanical28,
eMechanical29,
eMechanical30,
eMechanical31,
eMechanical32,
eDrillDrawing,
eGridColor1,
eBackGroundLayer,
eBackGroundLayer,
eBackGroundLayer,
eBackGroundLayer,
eBackGroundLayer);
cDir_NONE

cDir_NONE = [];
cDir_ANY

cDir_ANY = [eDir_N..eDir_NW];
cDir_Diagonal

cDir_Diagonal = [eDir_NE, eDir_SE, eDir_SW, eDir_NW];
cDir_HorVert

cDir_HorVert = cDir_ANY - cDir_Diagonal;
cLayerStrings

cLayerStrings : Array[TLayer] Of String
= ( 'NoLayer' ,
'TopLayer' ,
'MidLayer1' ,
'MidLayer2' ,
'MidLayer3' ,
'MidLayer4' ,
'MidLayer5' ,
'MidLayer6' ,
'MidLayer7' ,
'MidLayer8' ,
'MidLayer9' ,
'MidLayer10' ,
'MidLayer11' ,
'MidLayer12' ,
'MidLayer13' ,
'MidLayer14' ,
'MidLayer15' ,
'MidLayer16' ,
'MidLayer17' ,
'MidLayer18' ,
'MidLayer19' ,
'MidLayer20' ,
'MidLayer21' ,
'MidLayer22' ,
'MidLayer23' ,
'MidLayer24' ,
'MidLayer25' ,
'MidLayer26' ,
'MidLayer27' ,
'MidLayer28' ,
'MidLayer29' ,
'MidLayer30' ,
'BottomLayer' ,
'TopOverlay' ,
'BottomOverlay' ,
'TopPaste' ,
'BottomPaste' ,
'TopSolder' ,
'BottomSolder' ,
'InternalPlane1' ,
'InternalPlane2' ,
'InternalPlane3' ,
'InternalPlane4' ,
'InternalPlane5' ,
'InternalPlane6' ,
'InternalPlane7' ,
'InternalPlane8' ,
'InternalPlane9' ,
'InternalPlane10',
'InternalPlane11',
'InternalPlane12',
'InternalPlane13',
'InternalPlane14',
'InternalPlane15',
'InternalPlane16',
'DrillGuide' ,
'KeepOutLayer' ,
'Mechanical1' ,
'Mechanical2' ,
'Mechanical3' ,
'Mechanical4' ,
'Mechanical5' ,
'Mechanical6' ,
'Mechanical7' ,
'Mechanical8' ,
'Mechanical9' ,
'Mechanical10' ,
'Mechanical11' ,
'Mechanical12' ,
'Mechanical13' ,
'Mechanical14' ,
'Mechanical15' ,
'Mechanical16' ,
'Mechanical17' ,
'Mechanical18' ,
'Mechanical19' ,
'Mechanical20' ,
'Mechanical21' ,
'Mechanical22' ,
'Mechanical23' ,
'Mechanical24' ,
'Mechanical25' ,
'Mechanical26' ,
'Mechanical27' ,
'Mechanical28' ,
'Mechanical29' ,
'Mechanical30' ,
'Mechanical31' ,
'Mechanical32' ,
'DrillDrawing' ,
'MultiLayer' ,
'ConnectLayer' ,
'BackGroundLayer',
'DRCErrorLayer' ,
'HighlightLayer' ,
'GridColor1' ,
'GridColor10' ,
'PadHoleLayer' ,
'ViaHoleLayer');
cMaxTestPointStyle

cMaxTestPointStyle = eNewTHTop;
cMinTestPointStyle

cMinTestPointStyle = eExistingSMDBottom;
cMidLayers

cMidLayers : Set Of TLayer = [eMidLayer1 .. eMidLayer30];
cMinLayer_WidthRule

cMinLayer_WidthRule = eTopLayer;
cMaxLayer_WidthRule

cMaxLayer_WidthRule = eBottomLayer;
cRoutingWidthModeStrings

cRoutingWidthModeStrings : Array[\TRoutingWidthMode] Of String[20]
= ('User Choice' , //eRoutingWidth_Default
'Rule Minimum' , //eRoutingWidth_Min
'Rule Preferred', //eRoutingWidth_Preferred
'Rule Maximum' //eRoutingWidth_Max);
cRuleIdStrings

cRuleIdStrings : Array [TRuleKind] Of String[21]
= ( 'Clearance' ,
'ParallelSegment' ,
'Width' ,
'Length' ,
'MatchedLengths' ,
'StubLength' ,
'PlaneConnect' ,
'RoutingTopology' ,
'RoutingPriority' ,
'RoutingLayers' ,
'RoutingCorners' ,
'RoutingVias' ,
'PlaneClearance' ,
'SolderMaskExpansion' ,
'PasteMaskExpansion' ,
'ShortCircuit' ,
'UnRoutedNet' ,
'ViasUnderSMD' ,
'MaximumViaCount' ,
'MinimumAnnularRing' ,
'PolygonConnect' ,
'AcuteAngle' ,
'RoomDefinition' ,
'SMDToCorner' ,
'ComponentClearance' ,
'ComponentOrientations',
'PermittedLayers' ,
'NetsToIgnore' ,
'SignalStimulus' ,
'OvershootFalling' ,
'OvershootRising' ,
'UndershootFalling' ,
'UndershootRising' ,
'MaxMinImpedance' ,
'SignalTopValue' ,
'SignalBaseValue' ,
'FlightTimeRising' ,
'FlightTimeFalling' ,
'LayerStack' ,
'SlopeRising' ,
'SlopeFalling' ,
'SupplyNets' ,
'HoleSize' ,
'Testpoint' ,
'TestPointUsage' ,
'UnConnectedPin' ,
'SMDToPlane' ,
'SMDNeckDown' ,
'LayerPairs' ,
'FanoutControl' ,
'Height',
'DiffPairsRouting'
);
cTextAutopositionStrings

cTextAutopositionStrings : Array[TTextAutoPosition] Of String[20]
= ( 'Manual' ,
'Left-Above' ,
'Left-Center' ,
'Left-Below' ,
'Center-Above',
'Center' ,
'Center-Below',
'Right-Above' ,
'Right-Center',
'Right-Below');
cTestPointPriorityHigh

cTestPointPriorityHigh = Ord(cMinTestPointStyle);
cTestPointPriorityLow

cTestPointPriorityLow = Ord(cMaxTestPointStyle);
cWidthRuleLayers

cWidthRuleLayers = [cMinLayer_WidthRule..cMaxLayer_WidthRule];
FirstObjectId

FirstObjectId = eArcObject;
InternalUnits

InternalUnits = 10000;
InternalPlanes

InternalPlanes : Set Of TLayer = [eInternalPlane1..eInternalPlane16];
kDiameterSymbolANSI

kDiameterSymbolANSI = #$F8;
kDiameterSymbolUnicode

kDiameterSymbolUnicode = #$3A6;
kDegreeSymbol

kDegreeSymbol = #$B0;
k1Inch

k1Inch = 1000 * InternalUnits;
Notes
1 mil = 10000 internal units
1 inch = 1000 mils
1 inch = 2.54 cm
1 inch = 25.4 mm and 1 cm = 10 mm
PCB object's coordinates are usually in mils or mm depending on the board's current measurement units.
kDefaultArcResolution

kDefaultArcResolution = k1Mil Div 2;
Notes
1 mil = 10000 internal units
1 inch = 1000 mils
1 inch = 2.54 cm
1 inch = 25.4 mm and 1 cm = 10 mm
PCB object's coordinates are usually in mils or mm depending on the board's current measurement units.
k1Mil

k1Mil = 1 * InternalUnits;
Notes
1 mil = 10000 internal units
1 inch = 1000 mils
1 inch = 2.54 cm
1 inch = 25.4 mm and 1 cm = 10 mm
PCB object's coordinates are usually in mils or mm depending on the board's current measurement units.
kMaxCoord

kMaxCoord = 99999 * InternalUnits;
kMinCoord

kMinCoord = 0 * InternalUnits;
kMaxInternalPlane

kMaxInternalPlane = eInternalPlane16;
kMinInternalPlane

kMinInternalPlane = eInternalPlane1;
kMaxPolySize

kMaxPolySize = 5000;
LastObjectId

LastObjectId = eEmbeddedBoardObject;
kMaxStrokes

kMaxStrokes = 2000;
MaxLayer

MaxLayer = eViaHoleLayer;
Notes
Refer to Layer2String and String2Layer functions in the PCB Functions topic.
MaxBoardLayer

MaxBoardLayer = eMultiLayer;
MaxLogicalTextSize

MaxLogicalTextSize = k1Inch;
MaxRouteLayer

MaxRouteLayer = eBottomLayer;
MaxMechanicalLayer constant

MaxMechanicalLayer = eMechanical32;
MechanicalLayers

MechanicalLayers : Set Of TLayer = [eMechanical1..eMechanical32];
MinLayer

MinLayer = eTopLayer;

Notes
Refer to Layer2String and String2Layer functions in the PCB Functions topic.
MinMechanicalLayer constant
MinMechanicalLayer = eMechanical1;
Numbers

Numbers : Set Of Char = ['0'..'9'];
WideStringObjects
WideStringObjects = [ eTextObject, eDimensionObject, eCoordinateObject, eComponentObject];
PCB Messages

Overview
The PCB Messages are messages that are broadcasted by the PCB Editor server. There are different types of messages that describe a specific action within the PCB server.
Normally the PCB message constants are used for the IPCB_ServerInterface.SendMessageToRobots method.
Syntax
PCBM_NullMessage = 0;
PCBM_BeginModify = 1;
PCBM_BoardRegisteration = 2;
PCBM_EndModify = 3;
PCBM_CancelModify = 4;
PCBM_Create = 5;
PCBM_Destroy = 6;
PCBM_ProcessStart = 7;
PCBM_ProcessEnd = 8;
PCBM_ProcessCancel = 9;
PCBM_YieldToRobots = 10;
PCBM_CycleEnd = 11;
PCBM_CycleStart = 12;
PCBM_SystemInvalid = 13;
PCBM_SystemValid = 14;
PCBM_ViewUpdate = 15;
PCBM_UnDoRegister = 16;

c_BroadCast = Nil;
c_NoEventData = Nil;
c_FromSystem = Nil;
See also
SendMessageToRobots method
SignalLayers
SignalLayers : Set Of TLayer = [eTopLayer.. eBottomLayer]; 
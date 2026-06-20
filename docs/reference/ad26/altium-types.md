> **AD26 source snapshot; not current implementation status.** Validate against current Rust code before use.

The enumerated types are used for many of the PCB object interfaces methods which are covered in this section.
For example the IPCB_Board interface has a LayerIsUsed [L : TLayer] : Boolean property. You can use this Enumerated Types section below to check what the range is for the TLayer type.
TAdvPCBFileFormatVersion

TAdvPCBFileFormatVersion =
(ePCBFileFormatNone,
eAdvPCBFormat_Binary_V3,
eAdvPCBFormat_Library_V3,
eAdvPCBFormat_ASCII_V3,
eAdvPCBFormat_Binary_V4,
eAdvPCBFormat_Library_V4,
eAdvPCBFormat_ASCII_V4,
eAdvPCBFormat_Binary_V5,
eAdvPCBFormat_Library_V5,
eAdvPCBFormat_ASCII_V5);
TAngle

Double type.
TApertureUse

TApertureUse = ( eNoApertureUse,
eMultiUse,
eDrawUse,
eFlashUse);
TAutoPanMode

TAutoPanMode = ( eNoAutoPan
eReCentre
eFixedJump
eShiftAccellerator
eShiftDeccellerator
eBallistic
eAdaptive);
TAutoPanUnit

TAutoPanUnit = ( eAutoPanByMils
eAutoPanByPixels);
TBaud

TBaud = ( eBaud110 ,
eBaud150 ,
eBaud300 ,
eBaud600 ,
eBaud1200 ,
eBaud2400 ,
eBaud4800 ,
eBaud9600 ,
eBaud19200
);
TBGAFanoutDirection

TBGAFanoutDirection = ( eBGAFanoutDirection_Out ,
eBGAFanoutDirection_NE ,
eBGAFanoutDirection_SE ,
eBGAFanoutDirection_SW ,
eBGAFanoutDirection_NW ,
eBGAFanoutDirection_In
);
TBGAFanoutViaMode

TBGAFanoutViaMode = ( eBGAFanoutVia_Closest , eBGAFanoutVia_Centered
);
TBoardSide type

TBoardSide = ( eBoardSide_Top, eBoardSide_Bottom);
TCacheState

TCacheState = ( eCacheInvalid,
eCacheValid,
eCacheManual);
TChangeScope

TChangeScope = ( eChangeNone ,
eChangeThisItem ,
eChangeAllPrimitives ,
eChangeAllFreePrimitives ,
eChangeComponentDesignators ,
eChangeComponentComments ,
eChangeLibraryAllComponents ,
eChangeCancelled
);
TClassMemberKind

TClassMemberKind = (eClassMemberKind_Net,
eClassMemberKind_Component,
eClassMemberKind_FromTo,
eClassMemberKind_Pad,
eClassMemberKind_Layer,
eClassMemberKind_DesignChannel,
eClassMemberKind_DifferentialPair
);
TComponentCollisionCheckMode

TComponentCollisionCheckMode
= (eQuickCheck,
eMultiLayerCheck,
eFullCheck,
eComponentBodyCheck,
);
TComponentMoveKind

TComponentMoveKind = ( eNoComponentMoveNoAction
eJumpToComponent
eMoveComponentToCursor
);
TComponentStyle

TComponentStyle = ( eComponentStyle_Unknown ,
eComponentStyle_Small ,
eComponentStyle_SmallSMT ,
eComponentStyle_Edge ,
eComponentStyle_DIP ,
eComponentStyle_SIP ,
eComponentStyle_SMSIP ,
eComponentStyle_SMDIP ,
eComponentStyle_LCC ,
eComponentStyle_BGA ,
eComponentStyle_PGA
);
TComponentType

TComponentType = ( eBJT ,
eCapactitor ,
eConnector ,
eDiode ,
eIC ,
eInductor ,
eResistor
);
TConfinementStyle

TConfinementStyle = ( eConfineIn, eConfineOut);
TConnectionMode

TConnectionMode = ( eRatsNestConnection eBrokenNetMarker );
TCoord

TCoord = Integer;
TCoordPoint

TCoordPoint = Record
x,
y : TCoord;
End;
TCoordRect

TCoordRect = Record
Case Integer of
0 :(left,bottom,right,top : TCoord);
1 :(x1,y1,x2,y2 : TCoord);
2 :(Location1,Location2 : TCoordPoint);
End;
Note TPoint is a Borland Delphi defined type in the Types.pas unit.
TCopyMode

TCopyMode = ( eFullCopy, eFieldCopy);
TCornerStyle

TCornerStyle = ( eCornerStyle_90, eCornerStyle_45, eCornerStyle_Round);
TDaisyChainStyle

TDaisyChainStyle = ( eDaisyChainLoad , eDaisyChainTerminator , eDaisyChainSource );
TDataBits

TDataBits = ( eDataBits5 ,
eDataBits6 ,
eDataBits7 ,
eDataBits8
);
TDielectricType

TDielectricType = (eNoDielectric,
eCore,
ePrePreg,
eSurfaceMaterial);
TDimensionArrowPosition

TDimensionArrowPosition = ( eInside,eOutside);
TDimensionReference

TDimensionReference = Record
Primitive : IPCB_Primitive;
Point : TCoordPoint;
Anchor : Integer;
End;
TDimensionTextPosition

TDimensionTextPosition
= ( eTextAuto ,
eTextCenter ,
eTextTop ,
eTextBottom ,
eTextRight ,
eTextLeft ,
eTextInsideRight ,
eTextInsideLeft ,
eTextUniDirectional ,
eTextManual
);
TDimensionKind

TDimensionKind = ( eNoDimension ,
eLinearDimension ,
eAngularDimension ,
eRadialDimension ,
eLeaderDimension ,
eDatumDimension ,
eBaselineDimension ,
eCenterDimension ,
eOriginalDimension ,
eLinearDiameterDimension ,
eRadialDiameterDimension
);
TDimensionUnit

TDimensionUnit = ( eMils,
eInches,
eMillimeters,
eCentimeters,
eDegrees,
eRadians,
eAutomaticUnit);
TDirection

TDirection = (eDir_N ,
eDir_NE,
eDir_E ,
eDir_SE,
eDir_S ,
eDir_SW,
eDir_W ,
eDir_NW);
TDirectionSet

TDirectionSet = Set Of TDirection;
Display

TDisplay = ( eOverWrite
eHide
eShow
eInvert
eHighLight
);
TDrawingOrderArray

Type TDrawingOrderArray = Array [0..Ord(MaxLayer)] Of TLayer;
TDrawMode

TDrawMode = ( eDrawFull,
eDrawDraft,
eDrawHidden);
DrillSymbol

TDrillSymbol = ( eSymbols ,
eNumbers ,
eLetters
); {Used by gerber and Print/ Plot}
TDXColorMode

TDXColorMode = ( eDXPrimitive_DisplayNormal ,
eDXPrimitive_DisplayDimm ,
eDXPrimitive_DisplayHighlight ,
eDXPrimitive_DisplayTransparent ,
eDXPrimitive_DisplayGrayScale ,
eDXPrimitive_DisplayMonochrome ,
eDXPrimitive_DisplayDRCError ,
eDXPrimitive_DisplayGrayScaleTransparent ,
eDXPrimitive_DisplayMonochromeTRansparent,
eDXPrimitive_DisplayDimmTransparent ,
eDXPrimitive_DisplayHighlightTransparent
);
DynamicString

TDynamicString = AnsiString;
TEditingAction

TEditingAction=(eEditAction_Focus,
eEditAction_Move,
eEditAction_Change,
eEditAction_Delete,
eEditAction_Select,
eEditAction_NonGraphicalSelect,
eEditAction_Measure,
eEditAction_Dimension);
TEmbeddedBoardOriginMode

TEmbeddedBoardOriginMode = (eOriginMode_BoardOrigin,
eOriginMode_BottomLeft);
TEnabledRoutingLayers

TEnabledRoutingLayers = Array [eTopLayer..eBottomLayer] Of Boolean;
TExtendedDrillType

TExtendedDrillType = ( eDrilledHole,
ePunchedHole,
eLaserDrilledHole,
ePlasmaDrilledHole
);
TExtended Hole Type

TExtendedHoleType = ( eRoundHole,
eSquareHole,
eSlotHole
);
TTestpointAllowedSide

TTestpointAllowedSide
= ( eAllowTopSide ,
eAllowBottomSide ,
eAllowThruHoleTop ,
eAllowThruHoleBottom
);
TFanoutDirection

TFanoutDirection = (eFanoutDirection_None,
eFanoutDirection_InOnly,
eFanoutDirection_OutOnly,
eFanoutDirection_InThenOut,
eFanoutDirection_OutThenIn,
eFanoutDirection_Alternating);
TFanoutStyle

TFanoutStyle = (eFanoutStyle_Auto,
eFanoutStyle_Rows,
eFanoutStyle_Staggered,
eFanoutStyle_BGA,
eFanoutStyle_UnderPads);
TFontName

TFontName = Array [0..LF_FACESIZE
- 1] Of WideChar;
TFontID

TFontID = SmallInt;
TFullFontName

TFullFontName = Array [0..LF_FULLFACESIZE
- 1] Of WideChar;
TFromToDisplayMode

TFromToDisplayMode = ( eFromToDisplayMode_Automatic,
eFromToDisplayMode_Hide,
eFromToDisplayMode_Show);
TGraphicsCursor

TGraphicsCursor = ( eCurShapeCross90,
eCurShapeBigCross,
eCurShapeCross45);
THandshaking

THandshaking = ( eHandshakingNone ,
eHandshakingXonXOff ,
eHandshakingHardwire
);
TInteractiveRouteMode

TInteractiveRouteMode
= ( eIgnoreObstacle
eAvoidObstacle
ePushObstacle
);
TIterationMethod

TIterationMethod = ( eProcessAll, eProcessFree, eProcessComponent);
TLayer

TLayer = (eNoLayer ,
eTopLayer ,
eMidLayer1 ,
eMidLayer2 ,
eMidLayer3 ,
eMidLayer4 ,
eMidLayer5 ,
eMidLayer6 ,
eMidLayer7 ,
eMidLayer8 ,
eMidLayer9 ,
eMidLayer10 ,
eMidLayer11 ,
eMidLayer12 ,
eMidLayer13 ,
eMidLayer14 ,
eMidLayer15 ,
eMidLayer16 ,
eMidLayer17 ,
eMidLayer18 ,
eMidLayer19 ,
eMidLayer20 ,
eMidLayer21 ,
eMidLayer22 ,
eMidLayer23 ,
eMidLayer24 ,
eMidLayer25 ,
eMidLayer26 ,
eMidLayer27 ,
eMidLayer28 ,
eMidLayer29 ,
eMidLayer30 ,
eBottomLayer ,
eTopOverlay ,
eBottomOverlay ,
eTopPaste ,
eBottomPaste ,
eTopSolder ,
eBottomSolder ,
eInternalPlane1 ,
eInternalPlane2 ,
eInternalPlane3 ,
eInternalPlane4 ,
eInternalPlane5 ,
eInternalPlane6 ,
eInternalPlane7 ,
eInternalPlane8 ,
eInternalPlane9 ,
eInternalPlane10 ,
eInternalPlane11 ,
eInternalPlane12 ,
eInternalPlane13 ,
eInternalPlane14 ,
eInternalPlane15 ,
eInternalPlane16 ,
eDrillGuide ,
eKeepOutLayer ,
eMechanical1 ,
eMechanical2 ,
eMechanical3 ,
eMechanical4 ,
eMechanical5 ,
eMechanical6 ,
eMechanical7 ,
eMechanical8 ,
eMechanical9 ,
eMechanical10 ,
eMechanical11 ,
eMechanical12 ,
eMechanical13 ,
eMechanical14 ,
eMechanical15 ,
eMechanical16 ,
eDrillDrawing ,
eMultiLayer ,
eConnectLayer ,
eBackGroundLayer ,
eDRCErrorLayer ,
eHighlightLayer ,
eGridColor1 ,
eGridColor10 ,
ePadHoleLayer ,
eViaHoleLayer
);
TLayerSet

TLayerSet = Set of TLayer;
See also
TLayer
TLayerStackStyle

TLayerStackStyle = ( eLayerStack_Pairs ,
eLayerStacks_InsidePairs,
eLayerStackBuildup);
TLengthenerStyle

TLengthenerStyle = (eLengthenerStyle_90,
eLengthenerStyle_45,
eLengthenerStyle_Round);
TLogicalDrawingMode

TLogicalDrawingMode = ( eDisplaySolid ,
eDisplayHollow ,
eDisplaySelected ,
eDisplayDRC ,
eDisplayFocused ,
eDisplayMultiFocused ,
eDisplayHollowDashed
);
TMechanicalLayerPair

TMechanicalLayerPair = Record
Layer1 : TLayer;
Layer2 : TLayer;
End;
TMirrorOperation

TMirrorOperation = (eHMirror,eVMirror);
TNetScope

TNetScope = (eNetScope_DifferentNetsOnly,
eNetScope_SameNetOnly,
eNetScope_AnyNet);
TNetTopology

TNetTopology = ( eNetTopology_Shortest ,
eNetTopology_Horizontal ,
eNetTopology_Vertical ,
eNetTopology_DaisyChain_Simple ,
eNetTopology_DaisyChain_MidDriven ,
eNetTopology_DaisyChain_Balanced ,
eNetTopology_Starburst
);
TObjectCreationMode

TObjectCreationMode = ( eCreate_Default, eCreate_GlobalCopy);
TObjectId

TObjectId = ( eNoObject ,
eArcObject ,
ePadObject ,
eViaObject ,
eTrackObject ,
eTextObject ,
eFillObject ,
eConnectionObject ,
eNetObject ,
eComponentObject ,
ePolyObject ,
eRegionObject ,
eComponentBodyObject,
eDimensionObject ,
eCoordinateObject ,
eClassObject ,
eRuleObject ,
eFromToObject ,
eDifferentialPairObject,
eViolationObject ,
eEmbeddedObject ,
eEmbeddedBoardObject,
eTraceObject ,
eSpareViaObject ,
eBoardObject ,
eBoardOutlineObject,
);

Note, the eTraceObject and eSpareViaObject values are for internal use only and are not used directly with PCB documents (these values are used for Signal Integrity and Situs auto routing modules).
TObjectSet

TObjectSet = Set of TObjectId;
See also
TObjectId
TOptionsObjectId

TOptionsObjectId = (eAbstractOptions,
eOutputOptions,
ePrinterOptions,
eGerberOptions,
eAdvancedPlacerOptions,
eDesignRuleCheckerOptions,
eSpecctraRouterOptions,
eAdvancedRouterOptions,
eEngineeringChangeOrderOptions,
eInteractiveRoutingOptions,
eSystemOptions,
ePinSwapOptions);
TOutputDriverType

TOutputDriverType = ( eUnknownDriver ,
eProtelGerber ,
eProtelPlot_Composite ,
eProtelPlot_Final ,
eStandardDriver_Composite ,
eStandardDriver_Final
);
TOutputPort

TOutputPort = (eOutputPortCom1,
eOutputPortCom2,
eOutputPortCom3,
eOutputPortCom4,
eOutputPortFile);
TPadCache

TPadCache = Record
PlaneConnectionStyle : TPlaneConnectionStyle;
ReliefConductorWidth : TCoord;
ReliefEntries : SmallInt;
ReliefAirGap : TCoord;
PowerPlaneReliefExpansion : TCoord;
PowerPlaneClearance : TCoord;
PasteMaskExpansion : TCoord;
SolderMaskExpansion : TCoord;
Planes : Word;
PlaneConnectionStyleValid : TCacheState;
ReliefConductorWidthValid : TCacheState;
ReliefEntriesValid : TCacheState;
ReliefAirGapValid : TCacheState;
PowerPlaneReliefExpansionValid : TCacheState;
PasteMaskExpansionValid : TCacheState;
SolderMaskExpansionValid : TCacheState;
PowerPlaneClearanceValid : TCacheState;
PlanesValid : TCacheState;
End;
TPadMode

TPadMode = ( ePadMode_Simple,
ePadMode_LocalStack,
ePadMode_ExternalStack);
TParity

TParity = (eParityNone,
eParityEven,
eParityOdd,
eParityMark,
eParitySpace);
TPCBDragMode

TPcbDragMode = ( eDragNone
eDragAllTracks
eDragConnectedTracks);
TPCBObjectHandle

TPCBObjectHandle = Pointer;
TPCBString

TPCBString = WideString;
TPlaceTrackMode

TPlaceTrackMode = ( ePlaceTrackNone,
ePlaceTrackAny,
ePlaceTrack9090,
ePlaceTrack4590,
ePlaceTrack90Arc);
TPlaneConnectionStyle

TPlaneConnectionStyle = ( ePlaneNoConnect,
ePlaneReliefConnect,
ePlaneDirectConnect);
TPlaneConnectStyle

TPlaneConnectStyle = (eReliefConnectToPlane,
eDirectConnectToPlane,
eNoConnect);
TPlaneDrawMode

TPlaneDrawMode = ( ePlaneDrawoOutlineLayerColoured // <- Protel 99 SE style.
ePlaneDrawOutlineNetColoured,
ePlaneDrawInvertedNetColoured);
TPlotLayer

TPlotLayer = (eNullPlot,
eTopLayerPlot,
eMidLayer1Plot,
eMidLayer2Plot,
eMidLayer3Plot,
eMidLayer4Plot,
eMidLayer5Plot,
eMidLayer6Plot,
eMidLayer7Plot,
eMidLayer8Plot,
eMidLayer9Plot,
eMidLayer10Plot,
eMidLayer11Plot,
eMidLayer12Plot,
eMidLayer13Plot,
eMidLayer14Plot,
eMidLayer15Plot,
eMidLayer16Plot,
eMidLayer17Plot,
eMidLayer18Plot,
eMidLayer19Plot,
eMidLayer20Plot,
eMidLayer21Plot,
eMidLayer22Plot,
eMidLayer23Plot,
eMidLayer24Plot,
eMidLayer25Plot,
eMidLayer26Plot,
eMidLayer27Plot,
eMidLayer28Plot,
eMidLayer29Plot,
eMidLayer30Plot,
eBottomLayerPlot,
eTopOverlayPlot,
eBottomOverlayPlot,
eTopPastePlot,
eBottomPastePlot,
eTopSolderPlot,
eBottomSolderPlot,
eInternalPlane1Plot,
eInternalPlane2Plot,
eInternalPlane3Plot,
eInternalPlane4Plot,
eInternalPlane5Plot,
eInternalPlane6Plot,
eInternalPlane7Plot,
eInternalPlane8Plot,
eInternalPlane9Plot,
eInternalPlane10Plot,
eInternalPlane11Plot,
eInternalPlane12Plot,
eInternalPlane13Plot,
eInternalPlane14Plot,
eInternalPlane15Plot,
eInternalPlane16Plot,
eDrillGuide_Top_BottomPlot,
eDrillGuide_Top_Mid1Plot,
eDrillGuide_Mid2_Mid3Plot,
eDrillGuide_Mid4_Mid5Plot,
eDrillGuide_Mid6_Mid7Plot,
eDrillGuide_Mid8_Mid9Plot,
eDrillGuide_Mid10_Mid11Plot,
eDrillGuide_Mid12_Mid13Plot,
eDrillGuide_Mid14_Mid15Plot,
eDrillGuide_Mid16_Mid17Plot,
eDrillGuide_Mid18_Mid19Plot,
eDrillGuide_Mid20_Mid21Plot,
eDrillGuide_Mid22_Mid23Plot,
eDrillGuide_Mid24_Mid25Plot,
eDrillGuide_Mid26_Mid27Plot,
eDrillGuide_Mid28_Mid29Plot,
eDrillGuide_Mid30_BottomPlot,
eDrillGuide_SpecialPlot,
eKeepOutLayerPlot,
eMechanical1Plot,
eMechanical2Plot,
eMechanical3Plot,
eMechanical4Plot,
eMechanical5Plot,
eMechanical6Plot,
eMechanical7Plot,
eMechanical8Plot,
eMechanical9Plot,
eMechanical10Plot,
eMechanical11Plot,
eMechanical12Plot,
eMechanical13Plot,
eMechanical14Plot,
eMechanical15Plot,
eMechanical16Plot,
eDrillDrawing_Top_BottomPlot,
eDrillDrawing_Top_Mid1Plot,
eDrillDrawing_Mid2_Mid3Plot,
eDrillDrawing_Mid4_Mid5Plot,
eDrillDrawing_Mid6_Mid7Plot,
eDrillDrawing_Mid8_Mid9Plot,
eDrillDrawing_Mid10_Mid11Plot,
eDrillDrawing_Mid12_Mid13Plot,
eDrillDrawing_Mid14_Mid15Plot,
eDrillDrawing_Mid16_Mid17Plot,
eDrillDrawing_Mid18_Mid19Plot,
eDrillDrawing_Mid20_Mid21Plot,
eDrillDrawing_Mid22_Mid23Plot,
eDrillDrawing_Mid24_Mid25Plot,
eDrillDrawing_Mid26_Mid27Plot,
eDrillDrawing_Mid28_Mid29Plot,
eDrillDrawing_Mid30_BottomPlot,
eDrillDrawing_SpecialPlot,
eTopPadMasterPlot,
eBottomPadMasterPlot);
TPlotPolygonProc

TPlotPolygonProc = Procedure(APoly : PGPC_Polygon) Of Object;
TPlotterLanguage

TPlotterLanguage = ( ePlotterLanguageHPGL,
ePlotterLanguageDMPL);
TPolygonReliefAngle

TPolygonReliefAngle = ( ePolygonReliefAngle_45,
ePolygonReliefAngle_90);
TPolygonRepourMode

TPolygonRepourMode = ( eNeverRepour
eThresholdRepour
eAlwaysRepour);
TPolygonType

TPolygonType = ( eSignalLayerPolygon,
eSplitPlanePolygon);
TPolyHatchStyle

TPolyHatchStyle = ( ePolyHatch90,
ePolyHatch45,
ePolyVHatch,
ePolyHHatch,
ePolyNoHatch,
ePolySolid);
TPolyRegionKind

TPolyRegionKind = ( ePolyRegionKind_Copper,
ePolyRegionKind_Cutout,
ePolyRegionKind_NamedRegion);
TPolySegmentType

TPolySegmentType = ( ePolySegmentLine,
ePolySegmentArc);
TPrinterBatch

TPrinterBatch = ( ePlotPerSheet,
ePanelize);
TPrinterComposite

TPrinterComposite = ( eColorComposite,
eMonoComposite);
TRouteLayer

TRouteLayer = (eRLLayerNotUsed,
eRLRouteHorizontal,
eRLRouteVertical,
eRLRouteSingleLayer,
eRLRoute_1_OClock,
eRLRoute_2_OClock,
eRLRoute_4_OClock,
eRLRoute_5_OClock,
eRLRoute_45_Up,
eRLRoute_45_Down,
eRLRouteFanout,
eRLRouteAuto);
TRouteVia

TRouteVia = (eViaThruHole,
eViaBlindBuriedPair,
eViaBlindBuriedAny,
eViaNone);
TRoutingWidthMode

TRoutingWidthMode = (eRoutingWidth_Default,
eRoutingWidth_Min,
eRoutingWidth_Preferred,
eRoutingWidth_Max);
TRuleKind

TRuleKind = ( eRule_Clearance,
eRule_ParallelSegment,
eRule_MaxMinWidth,
eRule_MaxMinLength,
eRule_MatchedLengths,
eRule_DaisyChainStubLength,
eRule_PowerPlaneConnectStyle,
eRule_RoutingTopology,
eRule_RoutingPriority,
eRule_RoutingLayers,
eRule_RoutingCornerStyle,
eRule_RoutingViaStyle,
eRule_PowerPlaneClearance,
eRule_SolderMaskExpansion,
eRule_PasteMaskExpansion,
eRule_ShortCircuit,
eRule_BrokenNets,
eRule_ViasUnderSMD,
eRule_MaximumViaCount,
eRule_MinimumAnnularRing,
eRule_PolygonConnectStyle,
eRule_AcuteAngle,
eRule_ConfinementConstraint,
eRule_SMDToCorner,
eRule_ComponentClearance,
eRule_ComponentRotations,
eRule_PermittedLayers,
eRule_NetsToIgnore,
eRule_SignalStimulus,
eRule_Overshoot_FallingEdge,
eRule_Overshoot_RisingEdge,
eRule_Undershoot_FallingEdge,
eRule_Undershoot_RisingEdge,
eRule_MaxMinImpedance,
eRule_SignalTopValue,
eRule_SignalBaseValue,
eRule_FlightTime_RisingEdge,
eRule_FlightTime_FallingEdge,
eRule_LayerStack,
eRule_MaxSlope_RisingEdge,
eRule_MaxSlope_FallingEdge,
eRule_SupplyNets,
eRule_MaxMinHoleSize,
eRule_TestPointStyle,
eRule_TestPointUsage,
eRule_UnconnectedPin,
eRule_SMDToPlane,
eRule_SMDNeckDown,
eRule_LayerPair,
eRule_FanoutControl,
eRule_MaxMinHeight,
eRule_DifferentialPairsRouting);
TRuleLayerKind

TRuleLayerKind = (eRuleLayerKind_SameLayer,
eRuleLayerKind_AdjacentLayer);
TSameNamePadstackReplacementMode

TSameNamePadstackReplacementMode
= ( eAskUser
eReplaceOne
eReplaceAll
eRenameOne
eRenameAll
eKeepOneExisting
eKeepAllExisting
);
TScopeId

ScopeId = (eScope1, eScope2);
TScopeKind

TScopeKind = ( eScopeKindBoard,
{Lowest Precedence}
eScopeKindLayerClass,
eScopeKindLayer,
eScopeKindObjectKind,
eScopeKindFootprint,
eScopeKindComponentClass,
eScopeKindComponent,
eScopeKindNetClass,
eScopeKindNet,
eScopeKindFromToClass,
eScopeKindFromTo,
eScopeKindPadClass,
eScopeKindPadSpec,
eScopeKindViaSpec,
eScopeKindFootprintPad,
eScopeKindPad,
eScopeKindRegion
{Highest Precedence});
TScopeObjectId

TScopeObjectId = ( eRuleObject_None,
eRuleObject_Wire,
eRuleObject_Pin,
eRuleObject_Smd,
eRuleObject_Via,
eRuleObject_Fill,
eRuleObject_Polygon,
eRuleObject_KeepOut);
TShape

TShape = (eNoShape,
eRounded,
eRectangular,
eOctagonal,
eCircleShape,
eArcShape,
eTerminator,
eRoundRectShape,
eRotatedRectShape
eRoundedRectangular
);
TSignalLevel

TSignalLevel = ( eLowLevel,
eHighLevel);
TSortBy

TSortBy = (eSortByAXThenAY,
eSortByAXThenDY,
eSortByAYThenAX,
eSortByDYThenAX,
eSortByName);
TSmartRouteMode

TSmartRouteMode = (eSRIgnoreObstacle,
eSRAvoidObstacle,
eSRWalkAroundObstacle,
eSRPushObstacle);
TStimulusType

TStimulusType = (eConstantLevel,
eSinglePulse,
ePeriodicPulse);
TStopBits

TStopBits = ( eStopBits1 ,
eStopBits1_5 ,
eDataBits2
);
TString (PCB)

TString = ShortString;
TStrokeArray

TStrokeArray = Array[1..kMaxStrokes] Of TStrokeRecord;
TStrokeRecord

TStrokeRecord = Record
X1, Y1, X2, Y2 : TCoord;
End;
TTestPointStyle

TTestPointStyle = (eExistingSMDBottom,
eExistingTHPadBottom,
eExistingTHViaBottom,
eNewSMDBottom,
eNewTHBottom,
eExistingSMDTop,
eExistingTHPadTop,
eExistingTHViaTop,
eNewSMDTop,
eNewTHTop);
TTestpointValid

TTestpointValid = ( eRequire,
eInvalid,
eIgnore);
TTextAlignment

TTextAlignment = ( eNoneAlign ,
eCentreAlign ,
eLeftAlign ,
eRightAlign ,
eTopAlign ,
eBottomAlign
);
TTextAutoposition

TTextAutoposition = ( eAutoPos_Manual,
eAutoPos_TopLeft,
eAutoPos_CenterLeft,
eAutoPos_BottomLeft,
eAutoPos_TopCenter,
eAutoPos_CenterCenter,
eAutoPos_BottomCenter,
eAutoPos_TopRight,
eAutoPos_CenterRight,
eAutoPos_BottomRight);
TUnit

TUnit = (eMetric, eImperial);
TUnitStyle

TUnitStyle = ( eNoUnits,
eYesUnits,
eParenthUnits);
TViewableObjectID

TViewableObjectID = (eViewableObject_None ,
eViewableObject_Arc ,
eViewableObject_Pad ,
eViewableObject_Via ,
eViewableObject_Track ,
eViewableObject_Text ,
eViewableObject_Fill ,
eViewableObject_Connection ,
eViewableObject_Net ,
eViewableObject_Component ,
eViewableObject_Poly ,
eViewableObject_LinearDimension ,
eViewableObject_AngularDimension ,
eViewableObject_RadialDimension ,
eViewableObject_LeaderDimension ,
eViewableObject_DatumDimension ,
eViewableObject_BaselineDimension ,
eViewableObject_CenterDimension ,
eViewableObject_OriginalDimension ,
eViewableObject_LinearDiameterDimension ,
eViewableObject_RadialDiameterDimension ,
eViewableObject_Coordinate ,
eViewableObject_Class ,
eViewableObject_Rule_Clearance ,
eViewableObject_Rule_ParallelSegment ,
eViewableObject_Rule_MaxMinWidth ,
eViewableObject_Rule_MaxMinLength ,
eViewableObject_Rule_MatchedLengths ,
eViewableObject_Rule_DaisyChainStubLength ,
eViewableObject_Rule_PowerPlaneConnectStyle,
eViewableObject_Rule_RoutingTopology ,
eViewableObject_Rule_RoutingPriority ,
eViewableObject_Rule_RoutingLayers ,
eViewableObject_Rule_RoutingCornerStyle ,
eViewableObject_Rule_RoutingViaStyle ,
eViewableObject_Rule_PowerPlaneClearance ,
eViewableObject_Rule_SolderMaskExpansion ,
eViewableObject_Rule_PasteMaskExpansion ,
eViewableObject_Rule_ShortCircuit ,
eViewableObject_Rule_BrokenNets ,
eViewableObject_Rule_ViasUnderSMD ,
eViewableObject_Rule_MaximumViaCount ,
eViewableObject_Rule_MinimumAnnularRing ,
eViewableObject_Rule_PolygonConnectStyle ,
eViewableObject_Rule_AcuteAngle ,
eViewableObject_Rule_ConfinementConstraint ,
eViewableObject_Rule_SMDToCorner ,
eViewableObject_Rule_ComponentClearance ,
eViewableObject_Rule_ComponentRotations ,
eViewableObject_Rule_PermittedLayers ,
eViewableObject_Rule_NetsToIgnore ,
eViewableObject_Rule_SignalStimulus ,
eViewableObject_Rule_Overshoot_FallingEdge ,
eViewableObject_Rule_Overshoot_RisingEdge ,
eViewableObject_Rule_Undershoot_FallingEdge,
eViewableObject_Rule_Undershoot_RisingEdge ,
eViewableObject_Rule_MaxMinImpedance ,
eViewableObject_Rule_SignalTopValue ,
eViewableObject_Rule_SignalBaseValue ,
eViewableObject_Rule_FlightTime_RisingEdge ,
eViewableObject_Rule_FlightTime_FallingEdge,
eViewableObject_Rule_LayerStack ,
eViewableObject_Rule_MaxSlope_RisingEdge ,
eViewableObject_Rule_MaxSlope_FallingEdge ,
eViewableObject_Rule_SupplyNets ,
eViewableObject_Rule_MaxMinHoleSize ,
eViewableObject_Rule_TestPointStyle ,
eViewableObject_Rule_TestPointUsage ,
eViewableObject_Rule_UnconnectedPin ,
eViewableObject_Rule_SMDToPlane ,
eViewableObject_Rule_SMDNeckDown ,
eViewableObject_Rule_LayerPair ,
eViewableObject_Rule_FanoutControl ,
eViewableObject_Rule_MaxMinHeight ,
eViewableObject_Rule_DifferentialPairs ,
eViewableObject_FromTo ,
eViewableObject_DifferentialPair ,
eViewableObject_Violation ,
eViewableObject_Board ,
eViewableObject_BoardOutline ,
eViewableObject_Group ,
eViewableObject_Clipboard ,
eViewableObject_SplitPlane,
eViewableObject_EmbeddedBoard,
eViewableObject_Region,
eViewableObject_ComponentBody);

Notes
This TViewableObjectID type is mainly used by the Inspector and List views in Altium Designer and is an extension of TObjectID type.
TWidthArray

TWidthArray = Array[cMinLayer_WidthRule..cMaxLayer_WidthRule] Of TCoord

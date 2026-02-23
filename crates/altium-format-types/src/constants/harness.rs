// Harness parameters: connectivity, physical dimensions, covering, colors,
// connectors, lengths, layout, logical signals, and system design links.
//
// These constants cover all harness-specific record types (RECORD 104-131,
// 215-218) and their associated parameters.

// ---------------------------------------------------------------------------
// Connectivity: connected object IDs
// ---------------------------------------------------------------------------

/// UniqueID of connected object.
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessSplice, HarnessWireLabel, HarnessWireBreak, HarnessPin
pub const CONNECTED_OBJECT_UNIQUE_ID: &str = "ConnectedObjectUniqueId";

/// Wire/bundle start endpoint UniqueID.
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessWire, HarnessWireData, HarnessBundle
///
/// **Gotcha:** note the trailing capital "ID" (not "Id").
pub const END_VERTEX1_CONNECTED_OBJECT_UNIQUE_ID: &str = "EndVertex1ConnectedObjectUniqueID";

/// Wire/bundle end endpoint UniqueID.
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessWire, HarnessWireData, HarnessBundle
///
/// **Gotcha:** note the trailing capital "ID" (not "Id").
pub const END_VERTEX2_CONNECTED_OBJECT_UNIQUE_ID: &str = "EndVertex2ConnectedObjectUniqueID";

// ---------------------------------------------------------------------------
// Connected-list parameters (count + per-item UniqueIDs)
//
// All use `ExportConnectedObjectsUniqueIds(ids, serializer, countKey, itemKey)`.
// The count is serialized first, then each item UniqueID is indexed
// (e.g., `ConnectedWireUniqueId0`, `ConnectedWireUniqueId1`, ...).
// ---------------------------------------------------------------------------

/// Count of connected wire UniqueIDs.
///
/// **Wire type:** i32
/// **Used by:** HarnessSplice, HarnessPin, HarnessShield, HarnessTwist,
/// HarnessNoConnect, and their Data variants
pub const CONNECTED_WIRES_UNIQUE_IDS_COUNT: &str = "ConnectedWiresUniqueIdsCount";

/// Connected wire UniqueID (indexed).
///
/// **Wire type:** DynamicString
/// **Used by:** (same as count)
pub const CONNECTED_WIRE_UNIQUE_ID: &str = "ConnectedWireUniqueId";

/// Count of connected pin wire UniqueIDs.
///
/// **Wire type:** i32
/// **Used by:** HarnessShield, HarnessShieldData
pub const CONNECTED_PIN_WIRES_UNIQUE_IDS_COUNT: &str = "ConnectedPinWiresUniqueIdsCount";

/// Connected pin wire UniqueID (indexed).
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessShield, HarnessShieldData
pub const CONNECTED_PIN_WIRE_UNIQUE_ID: &str = "ConnectedPinWireUniqueId";

/// Count of connected bundle UniqueIDs.
///
/// **Wire type:** i32
/// **Used by:** HarnessLayoutConnectionPoint
pub const CONNECTED_BUNDLES_UNIQUE_IDS_COUNT: &str = "ConnectedBundlesUniqueIdsCount";

/// Connected bundle UniqueID (indexed).
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessLayoutConnectionPoint
pub const CONNECTED_BUNDLE_UNIQUE_ID: &str = "ConnectedBundleUniqueId";

/// Count of connected inline splice UniqueIDs.
///
/// **Wire type:** i32
/// **Used by:** HarnessWire, HarnessWireData
pub const CONNECTED_INLINE_SPLICES_UNIQUE_IDS_COUNT: &str = "ConnectedInlineSplicesUniqueIdsCount";

/// Connected inline splice UniqueID (indexed).
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessWire, HarnessWireData
pub const CONNECTED_INLINE_SPLICE_UNIQUE_ID: &str = "ConnectedInlineSpliceUniqueId";

/// Count of connected wire label UniqueIDs.
///
/// **Wire type:** i32
/// **Used by:** HarnessWire
pub const CONNECTED_WIRE_LABELS_UNIQUE_IDS_COUNT: &str = "ConnectedWireLabelsUniqueIdsCount";

/// Connected wire label UniqueID (indexed).
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessWire
pub const CONNECTED_WIRE_LABEL_UNIQUE_ID: &str = "ConnectedWireLabelUniqueId";

/// Count of connected shield UniqueIDs.
///
/// **Wire type:** i32
/// **Used by:** HarnessWire, HarnessWireData
pub const CONNECTED_SHIELDS_UNIQUE_IDS_COUNT: &str = "ConnectedShieldsUniqueIdsCount";

/// Connected shield UniqueID (indexed).
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessWire, HarnessWireData
pub const CONNECTED_SHIELD_UNIQUE_ID: &str = "ConnectedShieldUniqueId";

/// Count of connected twist UniqueIDs.
///
/// **Wire type:** i32
/// **Used by:** HarnessWire, HarnessWireData
pub const CONNECTED_TWISTS_UNIQUE_IDS_COUNT: &str = "ConnectedTwistsUniqueIdsCount";

/// Connected twist UniqueID (indexed).
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessWire, HarnessWireData
pub const CONNECTED_TWIST_UNIQUE_ID: &str = "ConnectedTwistUniqueId";

/// Count of connected cable UniqueIDs.
///
/// **Wire type:** i32
/// **Used by:** HarnessWire, HarnessWireData
pub const CONNECTED_CABLES_UNIQUE_IDS_COUNT: &str = "ConnectedCablesUniqueIdsCount";

/// Connected cable UniqueID (indexed).
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessWire, HarnessWireData
pub const CONNECTED_CABLE_UNIQUE_ID: &str = "ConnectedCableUniqueId";

/// Count of bundles to route through.
///
/// **Wire type:** i32
/// **Used by:** routing through bundles
pub const BUNDLES_TO_GO_THROUGH_UNIQUE_IDS_COUNT: &str = "BundlesToGoThroughUniqueIdsCount";

/// Bundle to route through UniqueID (indexed).
///
/// **Wire type:** DynamicString
/// **Used by:** routing through bundles
pub const BUNDLE_TO_GO_THROUGH_UNIQUE_ID: &str = "BundleToGoThroughUniqueId";

// ---------------------------------------------------------------------------
// Physical dimensions
// ---------------------------------------------------------------------------

/// Physical length from bundle start to covering start (harness units).
///
/// **Wire type:** i64
/// **Used by:** HarnessCovering (RECORD=128)
pub const PHYSICAL_START_DISTANCE: &str = "PhysicalStartDistance";

/// Physical length from bundle end to covering end (harness units).
///
/// **Wire type:** i64
/// **Used by:** HarnessCovering (RECORD=128)
pub const PHYSICAL_END_DISTANCE: &str = "PhysicalEndDistance";

/// Actual physical length of covering (harness units).
///
/// **Wire type:** i64
/// **Used by:** HarnessCovering (RECORD=128)
pub const PHYSICAL_LENGTH: &str = "PhysicalLength";

/// Visual thickness (clamped by `CoveringThicknessClamper`).
///
/// **Wire type:** u8
/// **Used by:** HarnessCovering (RECORD=128)
pub const THICKNESS: &str = "Thickness";

// ---------------------------------------------------------------------------
// Visual offsets
// ---------------------------------------------------------------------------

/// Visual offset from start (DXP display coords).
///
/// **Wire type:** i32
/// **Used by:** HarnessCovering (RECORD=128)
pub const START_POINT_DISTANCE: &str = "StartPointDistance";

/// Visual offset from end (DXP display coords).
///
/// **Wire type:** i32
/// **Used by:** HarnessCovering (RECORD=128)
pub const END_POINT_DISTANCE: &str = "EndPointDistance";

/// Visual fill pattern for covering (THarnessBrush enum).
///
/// **Wire type:** u8
/// **Used by:** HarnessCovering (RECORD=128)
///
/// **Gotcha:** the on-disk key is `"HarnessLayoutBraidBrush"` (not
/// "CoveringBrush" as you might expect).
pub const HARNESS_LAYOUT_COVERING_BRUSH: &str = "HarnessLayoutBraidBrush";

// ---------------------------------------------------------------------------
// Covering covered items
// ---------------------------------------------------------------------------

/// Number of covered items in a covering.
///
/// **Wire type:** i32
/// **Used by:** HarnessCovering (RECORD=128)
pub const COVERED_ITEMS_COUNT: &str = "CoveredItemsCount";

/// Type of covered item (TObjectId: eHarnessBundle or eHarnessComponent).
///
/// **Wire type:** u8 (indexed as `CoveredItemType0`, ...)
/// **Used by:** HarnessCovering (RECORD=128)
pub const COVERED_ITEM_TYPE: &str = "CoveredItemType";

/// UniqueId of covered item.
///
/// **Wire type:** DynamicString (indexed as `CoveredItemId0`, ...)
/// **Used by:** HarnessCovering (RECORD=128)
pub const COVERED_ITEM_ID: &str = "CoveredItemId";

/// First pin designator (only for eHarnessComponent type).
///
/// **Wire type:** DynamicString (indexed as `CoveredItemFirstPin0`, ...)
/// **Used by:** HarnessCovering (RECORD=128)
pub const COVERED_ITEM_FIRST_PIN: &str = "CoveredItemFirstPin";

/// Last pin designator (only for eHarnessComponent type).
///
/// **Wire type:** DynamicString (indexed as `CoveredItemLastPin0`, ...)
/// **Used by:** HarnessCovering (RECORD=128)
pub const COVERED_ITEM_LAST_PIN: &str = "CoveredItemLastPin";

// ---------------------------------------------------------------------------
// Color names (human-readable)
// ---------------------------------------------------------------------------

/// Primary color name (e.g., `"Red"`).
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessWireBreak only
pub const PRIMARY_COLOR_NAME: &str = "PrimaryColorName";

/// Secondary color name.
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessWireBreak only
pub const SECONDARY_COLOR_NAME: &str = "SecondaryColorName";

/// Tertiary color name.
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessWireBreak only
pub const TERTIARY_COLOR_NAME: &str = "TertiaryColorName";

/// Border color name.
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessWireBreak only
pub const BORDER_COLOR_NAME: &str = "BorderColorName";

// ---------------------------------------------------------------------------
// Connector
// ---------------------------------------------------------------------------

/// Connector side (TLeftRightSide enum).
///
/// **Wire type:** u8
/// **Used by:** HarnessConnector (RECORD=215)
pub const HARNESS_CONNECTOR_SIDE: &str = "HarnessConnectorSide";

/// Harness type name.
///
/// **Wire type:** DynamicString
/// **Used by:** Port (RECORD=18)
pub const HARNESS_TYPE: &str = "HarnessType";

/// Wire attach point within connector.
///
/// **Wire type:** coord (i32)
/// **Used by:** HarnessConnector (RECORD=215)
pub const PRIMARY_CONNECTION_POSITION: &str = "PrimaryConnectionPosition";

// ---------------------------------------------------------------------------
// Origin
// ---------------------------------------------------------------------------

/// Corresponding wiring-diagram pin UniqueID.
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessPin (RECORD=113) in layout context
pub const WIRING_DIAGRAM_ORIGIN_UNIQUE_ID: &str = "WiringDiagramOriginUniqueId";

// ---------------------------------------------------------------------------
// Length
// ---------------------------------------------------------------------------

/// Harness length unit (THarnessLengthUnit enum).
///
/// **Wire type:** u8
/// **Used by:** HarnessDocument
///
/// Values: mm, cm, m, in, ft. Default: eMillimeter.
pub const HARNESS_LENGTH_UNIT: &str = "HarnessLengthUnit";

/// Wire length type (THarnessWireLengthType enum).
///
/// **Wire type:** u8
/// **Used by:** HarnessBundleSubLineData
///
/// Values: 0=Calculated, 1=UserDefined, 2=MCADCoDesigner.
pub const LENGTH_TYPE: &str = "LengthType";

/// Physical length in harness units (i64).
///
/// **Wire type:** i64
/// **Used by:** HarnessBundle/SubLine
pub const LENGTH_LONG: &str = "LengthLong";

/// Offset added to calculated length.
///
/// **Wire type:** i64
/// **Used by:** HarnessBundleSubLineData
pub const LENGTH_OFFSET: &str = "LengthOffset";

/// Graphically drawn length.
///
/// **Wire type:** i64
/// **Used by:** HarnessBundle/SubLine
pub const DRAWN_LENGTH: &str = "DrawnLength";

/// User-specified length override.
///
/// **Wire type:** i64
/// **Used by:** HarnessBundleSubLineData
pub const USER_LENGTH: &str = "UserLength";

// ---------------------------------------------------------------------------
// Bundle flags
// ---------------------------------------------------------------------------

/// Draw break/gap symbol on bundle.
///
/// **Wire type:** bool
/// **Used by:** HarnessBundle (RECORD=111)
pub const SHOW_BREAK_SYMBOL: &str = "ShowBreakSymbol";

/// Bundle length is set manually (not calculated).
///
/// **Wire type:** bool
/// **Used by:** HarnessBundle (RECORD=111)
pub const IS_LENGTH_SET_MANUALLY: &str = "IsLengthSetManually";

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

/// Default designator position X coordinate.
///
/// **Wire type:** coord (i32)
/// **Used by:** harness layout objects
///
/// **Gotcha:** note the dot notation `DefaultDesignatorPosition.X`.
pub const DEFAULT_DESIGNATOR_POSITION_X: &str = "DefaultDesignatorPosition.X";

/// Default designator position Y coordinate.
///
/// **Wire type:** coord (i32)
/// **Used by:** harness layout objects
pub const DEFAULT_DESIGNATOR_POSITION_Y: &str = "DefaultDesignatorPosition.Y";

// ---------------------------------------------------------------------------
// Logical signals
// ---------------------------------------------------------------------------

/// First component in logical signal connection.
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessLogicalSignal (RECORD=112)
pub const HARNESS_LOGICAL_SIGNAL_CONNECTION_1_COMP: &str = "HarnessLogicalSignalConnection1Comp";

/// Second component in logical signal connection.
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessLogicalSignal (RECORD=112)
pub const HARNESS_LOGICAL_SIGNAL_CONNECTION_2_COMP: &str = "HarnessLogicalSignalConnection2Comp";

/// First pin in logical signal connection.
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessLogicalSignal (RECORD=112)
pub const HARNESS_LOGICAL_SIGNAL_CONNECTION_1_PIN: &str = "HarnessLogicalSignalConnection1Pin";

/// Second pin in logical signal connection.
///
/// **Wire type:** DynamicString
/// **Used by:** HarnessLogicalSignal (RECORD=112)
pub const HARNESS_LOGICAL_SIGNAL_CONNECTION_2_PIN: &str = "HarnessLogicalSignalConnection2Pin";

// ---------------------------------------------------------------------------
// System design
// ---------------------------------------------------------------------------

/// System design UniqueID for harness integration.
///
/// **Wire type:** DynamicString
/// **Used by:** harness-ESD integration
///
/// **Gotcha:** the on-disk key is `"SystemDesignUniqueId"` (not
/// `"HarnessSystemDesignUniqueId"` as the C# field name suggests).
pub const HARNESS_SYSTEM_DESIGN_UNIQUE_ID: &str = "SystemDesignUniqueId";

// ---------------------------------------------------------------------------
// Object definition
// ---------------------------------------------------------------------------

/// Object definition ID linking to `ObjectDefinitions` stream.
///
/// **Wire type:** DynamicString
/// **Used by:** Port, PowerPort, ObjectDefinition (RECORD=129)
pub const OBJECT_DEFINITION_ID: &str = "ObjectDefinitionId";

/// Object definition content hash for change detection.
///
/// **Wire type:** DynamicString
/// **Used by:** ObjectDefinition (RECORD=129)
pub const OBJECT_DEFINITION_HASH: &str = "ObjectDefinitionHash";

/// Associated object type for harness objects.
///
/// **Wire type:** u8
/// **Used by:** AssociatedObjects (RECORD=131)
///
/// Values: 0=Crimp, 1=Seal, 2=Plug, 3=Other.
pub const ASSOCIATED_OBJECT_TYPE: &str = "AssociatedObjectType";

/// Instance label for functional connections.
///
/// **Wire type:** DynamicString
/// **Used by:** FunctionalConnectionLine (RECORD=134) in ESD
pub const INSTANCE_LABEL: &str = "InstanceLabel";

/// Instance name for functional connections.
///
/// **Wire type:** DynamicString
/// **Used by:** FunctionalConnectionLine (RECORD=134) in ESD
pub const INSTANCE_NAME: &str = "InstanceName";

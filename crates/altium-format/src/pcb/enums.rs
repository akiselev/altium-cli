//! PCB-specific enums for the v2 API.
//!
//! All enums are validated against the Altium SDK Delphi types and Ghidra
//! decompilation. They use `#[altium_enum]` to generate `AltiumEnum` and
//! `ParamCodec` trait implementations.

use altium_format_derive::altium_enum;

// ── PCB Object ID ────────────────────────────────────────────────────────

/// PCB record type ID -- used in binary framing `u8 type` byte.
///
/// This enum is referenced by the `#[altium_record(object_id = ...)]` macro
/// to determine the RECORD_ID constant for PCB record types.
#[altium_enum(fallback = "NoObject")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PcbObjectId {
    NoObject = 0,
    Arc = 1,
    Pad = 2,
    Via = 3,
    Track = 4,
    Text = 5,
    Fill = 6,
    Connection = 7,
    Net = 8,
    Component = 9,
    Polygon = 10,
    Region = 11,
    ComponentBody = 12,
    Dimension = 13,
    Coordinate = 14,
    Class = 15,
    Rule = 16,
    FromTo = 17,
    DifferentialPair = 18,
    Violation = 19,
    Embedded = 20,
    EmbeddedBoard = 21,
    SplitPlane = 22,
    SpareVia = 23,
    Board = 24,
    BoardOutline = 25,
}

// ── Layer ────────────────────────────────────────────────────────────────

/// PCB layer ID -- `TLayer` from SDK.
///
/// Values 0-82 for standard layers. Mechanical17-32 use extended mapping in AD26.
#[altium_enum(fallback = "NoLayer")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TLayer {
    NoLayer = 0,
    TopLayer = 1,
    MidLayer1 = 2,
    MidLayer2 = 3,
    MidLayer3 = 4,
    MidLayer4 = 5,
    MidLayer5 = 6,
    MidLayer6 = 7,
    MidLayer7 = 8,
    MidLayer8 = 9,
    MidLayer9 = 10,
    MidLayer10 = 11,
    MidLayer11 = 12,
    MidLayer12 = 13,
    MidLayer13 = 14,
    MidLayer14 = 15,
    MidLayer15 = 16,
    MidLayer16 = 17,
    MidLayer17 = 18,
    MidLayer18 = 19,
    MidLayer19 = 20,
    MidLayer20 = 21,
    MidLayer21 = 22,
    MidLayer22 = 23,
    MidLayer23 = 24,
    MidLayer24 = 25,
    MidLayer25 = 26,
    MidLayer26 = 27,
    MidLayer27 = 28,
    MidLayer28 = 29,
    MidLayer29 = 30,
    MidLayer30 = 31,
    BottomLayer = 32,
    TopOverlay = 33,
    BottomOverlay = 34,
    TopPaste = 35,
    BottomPaste = 36,
    TopSolder = 37,
    BottomSolder = 38,
    InternalPlane1 = 39,
    InternalPlane2 = 40,
    InternalPlane3 = 41,
    InternalPlane4 = 42,
    InternalPlane5 = 43,
    InternalPlane6 = 44,
    InternalPlane7 = 45,
    InternalPlane8 = 46,
    InternalPlane9 = 47,
    InternalPlane10 = 48,
    InternalPlane11 = 49,
    InternalPlane12 = 50,
    InternalPlane13 = 51,
    InternalPlane14 = 52,
    InternalPlane15 = 53,
    InternalPlane16 = 54,
    DrillGuide = 55,
    KeepOutLayer = 56,
    Mechanical1 = 57,
    Mechanical2 = 58,
    Mechanical3 = 59,
    Mechanical4 = 60,
    Mechanical5 = 61,
    Mechanical6 = 62,
    Mechanical7 = 63,
    Mechanical8 = 64,
    Mechanical9 = 65,
    Mechanical10 = 66,
    Mechanical11 = 67,
    Mechanical12 = 68,
    Mechanical13 = 69,
    Mechanical14 = 70,
    Mechanical15 = 71,
    Mechanical16 = 72,
    DrillDrawing = 73,
    MultiLayer = 74,
    ConnectLayer = 75,
    BackGroundLayer = 76,
    DRCErrorLayer = 77,
    HighlightLayer = 78,
    GridColor1 = 79,
    GridColor10 = 80,
    PadHoleLayer = 81,
    ViaHoleLayer = 82,
}

impl TLayer {
    /// Whether this is a signal layer (Top through Bottom, 1-32).
    pub fn is_signal(self) -> bool {
        let v = self as u8;
        v >= 1 && v <= 32
    }

    /// Whether this is an internal plane layer.
    pub fn is_internal_plane(self) -> bool {
        let v = self as u8;
        v >= 39 && v <= 54
    }

    /// Whether this is a mechanical layer.
    pub fn is_mechanical(self) -> bool {
        let v = self as u8;
        v >= 57 && v <= 72
    }
}

// ── Shape ────────────────────────────────────────────────────────────────

/// Pad/Via shape -- `TShape` from SDK.
#[altium_enum(fallback = "NoShape")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TShape {
    NoShape = 0,
    Rounded = 1,
    Rectangular = 2,
    Octagonal = 3,
    CircleShape = 4,
    ArcShape = 5,
    Terminator = 6,
    RoundRectShape = 7,
    RotatedRectShape = 8,
    RoundedRectangular = 9,
}

// ── Pad Mode ─────────────────────────────────────────────────────────────

/// Pad stack mode -- `TPadMode` from SDK.
#[altium_enum(fallback = "Simple")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TPadMode {
    /// Simple: same shape on all layers.
    Simple = 0,
    /// LocalStack (SDK: ePadMode_LocalStack).
    LocalStack = 1,
    /// ExternalStack (SDK: ePadMode_ExternalStack).
    ExternalStack = 2,
}

// ── Cache State ──────────────────────────────────────────────────────────

/// Expansion mode for paste/solder mask -- `TCacheState` from SDK.
#[altium_enum(fallback = "Invalid")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TCacheState {
    /// eCacheInvalid -- no override (None).
    Invalid = 0,
    /// eCacheValid -- use rule value.
    Valid = 1,
    /// eCacheManual -- manual override.
    Manual = 2,
}

// ── Plane Connection Style ───────────────────────────────────────────────

/// Plane connection style -- `TPlaneConnectStyle` from SDK.
///
/// Binary files use THIS ordering (Relief=0, Direct=1, NoConnect=2),
/// NOT `TPlaneConnectionStyle` (NoConnect=0, Relief=1, Direct=2).
#[altium_enum(fallback = "Relief")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TPlaneConnectStyle {
    Relief = 0,
    Direct = 1,
    NoConnect = 2,
}

// ── Polygon Hatch Style ─────────────────────────────────────────────────

/// Polygon fill style -- `TPolyHatchStyle` from SDK.
#[altium_enum(fallback = "Hatch90")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TPolyHatchStyle {
    Hatch90 = 0,
    Hatch45 = 1,
    VHatch = 2,
    HHatch = 3,
    NoHatch = 4,
    Solid = 5,
}

// ── Polygon Region Kind ──────────────────────────────────────────────────

/// Region kind -- `TPolyRegionKind` from SDK.
#[altium_enum(fallback = "Copper")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TPolyRegionKind {
    Copper = 0,
    Cutout = 1,
    NamedRegion = 2,
    DashedOutline = 3,
    CavityDefinition = 4,
}

// ── Polygon Type ─────────────────────────────────────────────────────────

/// Polygon type -- `TPolygonType` from SDK.
#[altium_enum(fallback = "SignalLayer")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TPolygonType {
    SignalLayer = 0,
    SplitPlane = 1,
}

// ── Dimension Kind ───────────────────────────────────────────────────────

/// Dimension type -- `TDimensionKind` from SDK.
#[altium_enum(fallback = "NoDimension")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TDimensionKind {
    NoDimension = 0,
    Linear = 1,
    Angular = 2,
    Radial = 3,
    Leader = 4,
    Datum = 5,
    Baseline = 6,
    Center = 7,
    Original = 8,
    LinearDiameter = 9,
    RadialDiameter = 10,
}

// ── Dimension Unit ───────────────────────────────────────────────────────

/// Dimension display unit -- `TDimensionUnit` from SDK.
#[altium_enum(fallback = "Mils")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TDimensionUnit {
    Mils = 0,
    Inches = 1,
    Millimeters = 2,
    Centimeters = 3,
    Degrees = 4,
    Radians = 5,
    Automatic = 6,
}

// ── Connection Mode ──────────────────────────────────────────────────────

/// Connection mode -- `TConnectionMode` from SDK.
#[altium_enum(fallback = "RatsNest")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TConnectionMode {
    RatsNest = 0,
    BrokenNetMarker = 1,
}

// ── Extended Hole Type ───────────────────────────────────────────────────

/// Hole shape -- `TExtendedHoleType` from SDK.
#[altium_enum(fallback = "Round")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TExtendedHoleType {
    Round = 0,
    Square = 1,
    Slot = 2,
}

// ── Extended Drill Type ──────────────────────────────────────────────────

/// Drill method -- `TExtendedDrillType` from SDK.
#[altium_enum(fallback = "Drilled")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TExtendedDrillType {
    Drilled = 0,
    Punched = 1,
    LaserDrilled = 2,
    PlasmaDrilled = 3,
}

// ── Text Autoposition ────────────────────────────────────────────────────

/// Text auto-positioning -- `TTextAutoposition` from SDK.
#[altium_enum(fallback = "Manual")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TTextAutoposition {
    Manual = 0,
    TopLeft = 1,
    CenterLeft = 2,
    BottomLeft = 3,
    TopCenter = 4,
    CenterCenter = 5,
    BottomCenter = 6,
    TopRight = 7,
    CenterRight = 8,
    BottomRight = 9,
}

// ── Dielectric Type ──────────────────────────────────────────────────────

/// Layer stack dielectric -- `TDielectricType` from SDK.
#[altium_enum(fallback = "NoDielectric")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TDielectricType {
    NoDielectric = 0,
    Core = 1,
    PrePreg = 2,
    SurfaceMaterial = 3,
}

// ── Layer Stack Style ────────────────────────────────────────────────────

/// Layer stack style -- `TLayerStackStyle` from SDK.
#[altium_enum(fallback = "Pairs")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TLayerStackStyle {
    Pairs = 0,
    InsidePairs = 1,
    Buildup = 2,
}

// ── Board Side ───────────────────────────────────────────────────────────

/// Board side -- `TBoardSide` from SDK.
#[altium_enum(fallback = "Top")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TBoardSide {
    Top = 0,
    Bottom = 1,
}

// ── Unit ─────────────────────────────────────────────────────────────────

/// Display unit -- `TUnit` from SDK.
#[altium_enum(fallback = "Metric")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TUnit {
    Metric = 0,
    Imperial = 1,
}

// ── Text Type ────────────────────────────────────────────────────────────

/// Text rendering type.
#[altium_enum(fallback = "Stroke")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextType {
    Stroke = 0,
    TrueType = 1,
    Barcode = 2,
}

// ── Stroke Font ──────────────────────────────────────────────────────────

/// Stroke font type -- `TStrokeFont` from SDK.
#[altium_enum(fallback = "Default")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum StrokeFont {
    Default = 0,
    SansSerif = 1,
    Serif = 2,
    Proportional1 = 3,
    Proportional2 = 4,
    Proportional3 = 5,
    Proportional4 = 6,
    Proportional5 = 7,
}

// ── Rule Kind ────────────────────────────────────────────────────────────

/// Design rule kind -- `TRuleKind` from SDK.
///
/// 52 rule types (0-51).
#[altium_enum(fallback = "Clearance")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TRuleKind {
    Clearance = 0,
    ParallelSegment = 1,
    MaxMinWidth = 2,
    MaxMinLength = 3,
    MatchedLengths = 4,
    DaisyChainStubLength = 5,
    PowerPlaneConnectStyle = 6,
    RoutingTopology = 7,
    RoutingPriority = 8,
    RoutingLayers = 9,
    RoutingCornerStyle = 10,
    RoutingViaStyle = 11,
    PowerPlaneClearance = 12,
    SolderMaskExpansion = 13,
    PasteMaskExpansion = 14,
    ShortCircuit = 15,
    BrokenNets = 16,
    ViasUnderSMD = 17,
    MaximumViaCount = 18,
    MinimumAnnularRing = 19,
    PolygonConnectStyle = 20,
    AcuteAngle = 21,
    ConfinementConstraint = 22,
    SMDToCorner = 23,
    ComponentClearance = 24,
    ComponentRotations = 25,
    PermittedLayers = 26,
    NetsToIgnore = 27,
    SignalStimulus = 28,
    OvershootFallingEdge = 29,
    OvershootRisingEdge = 30,
    UndershootFallingEdge = 31,
    UndershootRisingEdge = 32,
    MaxMinImpedance = 33,
    SignalTopValue = 34,
    SignalBaseValue = 35,
    FlightTimeRisingEdge = 36,
    FlightTimeFallingEdge = 37,
    LayerStack = 38,
    MaxSlopeRisingEdge = 39,
    MaxSlopeFallingEdge = 40,
    SupplyNets = 41,
    MaxMinHoleSize = 42,
    TestPointStyle = 43,
    TestPointUsage = 44,
    UnconnectedPin = 45,
    SMDToPlane = 46,
    SMDNeckDown = 47,
    LayerPair = 48,
    FanoutControl = 49,
    MaxMinHeight = 50,
    DifferentialPairsRouting = 51,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::AltiumEnum;

    #[test]
    fn pcb_object_id_from_int() {
        assert_eq!(PcbObjectId::from_int(0), PcbObjectId::NoObject);
        assert_eq!(PcbObjectId::from_int(1), PcbObjectId::Arc);
        assert_eq!(PcbObjectId::from_int(4), PcbObjectId::Track);
        assert_eq!(PcbObjectId::from_int(25), PcbObjectId::BoardOutline);
    }

    #[test]
    fn pcb_object_id_fallback() {
        assert_eq!(PcbObjectId::from_int(99), PcbObjectId::NoObject);
    }

    #[test]
    fn pcb_object_id_to_int() {
        assert_eq!(PcbObjectId::Track.to_int(), 4);
        assert_eq!(PcbObjectId::Pad.to_int(), 2);
        assert_eq!(PcbObjectId::Component.to_int(), 9);
    }

    #[test]
    fn tlayer_from_int() {
        assert_eq!(TLayer::from_int(0), TLayer::NoLayer);
        assert_eq!(TLayer::from_int(1), TLayer::TopLayer);
        assert_eq!(TLayer::from_int(32), TLayer::BottomLayer);
        assert_eq!(TLayer::from_int(74), TLayer::MultiLayer);
    }

    #[test]
    fn tlayer_fallback() {
        assert_eq!(TLayer::from_int(99), TLayer::NoLayer);
    }

    #[test]
    fn tlayer_predicates() {
        assert!(TLayer::TopLayer.is_signal());
        assert!(TLayer::BottomLayer.is_signal());
        assert!(TLayer::MidLayer1.is_signal());
        assert!(!TLayer::TopOverlay.is_signal());

        assert!(TLayer::InternalPlane1.is_internal_plane());
        assert!(!TLayer::TopLayer.is_internal_plane());

        assert!(TLayer::Mechanical1.is_mechanical());
        assert!(TLayer::Mechanical16.is_mechanical());
        assert!(!TLayer::TopLayer.is_mechanical());
    }

    #[test]
    fn tshape_roundtrip() {
        assert_eq!(TShape::from_int(1), TShape::Rounded);
        assert_eq!(TShape::from_int(2), TShape::Rectangular);
        assert_eq!(TShape::Rounded.to_int(), 1);
    }

    #[test]
    fn tpad_mode_roundtrip() {
        assert_eq!(TPadMode::from_int(0), TPadMode::Simple);
        assert_eq!(TPadMode::from_int(1), TPadMode::LocalStack);
        assert_eq!(TPadMode::from_int(2), TPadMode::ExternalStack);
    }

    #[test]
    fn trule_kind_roundtrip() {
        assert_eq!(TRuleKind::from_int(0), TRuleKind::Clearance);
        assert_eq!(TRuleKind::from_int(51), TRuleKind::DifferentialPairsRouting);
        assert_eq!(TRuleKind::DifferentialPairsRouting.to_int(), 51);
    }

    #[test]
    fn text_type_roundtrip() {
        assert_eq!(TextType::from_int(0), TextType::Stroke);
        assert_eq!(TextType::from_int(1), TextType::TrueType);
        assert_eq!(TextType::from_int(2), TextType::Barcode);
    }

    #[test]
    fn stroke_font_roundtrip() {
        assert_eq!(StrokeFont::from_int(0), StrokeFont::Default);
        assert_eq!(StrokeFont::from_int(1), StrokeFont::SansSerif);
        assert_eq!(StrokeFont::from_int(7), StrokeFont::Proportional5);
    }
}

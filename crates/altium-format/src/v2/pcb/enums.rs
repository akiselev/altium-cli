//! PCB enums validated against Altium SDK Delphi types and Ghidra decompilation.

use serde::{Deserialize, Serialize};

/// Helper macro for dense enums with contiguous discriminants.
macro_rules! dense_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident : $max:literal {
            $($(#[$vmeta:meta])* $variant:ident = $val:expr),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[repr(u8)]
        $vis enum $name {
            $($(#[$vmeta])* $variant = $val),+
        }

        impl $name {
            pub fn from_u8(v: u8) -> Option<Self> {
                if v <= $max {
                    Some(unsafe { std::mem::transmute(v) })
                } else {
                    None
                }
            }
        }
    };
}

// ── Layer ────────────────────────────────────────────────────────────────

/// PCB layer ID — `TLayer` from SDK.
///
/// Values 0-82 for standard layers. Mechanical17-32 use extended mapping in AD26.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 82 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }

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

dense_enum! {
    /// Pad/Via shape — `TShape` from SDK.
    pub enum TShape: 9 {
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
}

// ── Pad Mode ─────────────────────────────────────────────────────────────

dense_enum! {
    /// Pad stack mode — `TPadMode` from SDK (SDK names).
    pub enum TPadMode: 2 {
        /// Simple: same shape on all layers.
        Simple = 0,
        /// LocalStack (SDK: ePadMode_LocalStack, v1 called "TopMiddleBottom").
        LocalStack = 1,
        /// ExternalStack (SDK: ePadMode_ExternalStack, v1 called "FullStack").
        ExternalStack = 2,
    }
}

// ── Cache State ──────────────────────────────────────────────────────────

dense_enum! {
    /// Expansion mode for paste/solder mask — `TCacheState` from SDK.
    pub enum TCacheState: 2 {
        /// eCacheInvalid — no override (None).
        Invalid = 0,
        /// eCacheValid — use rule value.
        Valid = 1,
        /// eCacheManual — manual override.
        Manual = 2,
    }
}

// ── Plane Connection Style ───────────────────────────────────────────────

dense_enum! {
    /// Plane connection style — `TPlaneConnectStyle` from SDK.
    ///
    /// Binary files use THIS ordering (Relief=0, Direct=1, NoConnect=2),
    /// NOT `TPlaneConnectionStyle` (NoConnect=0, Relief=1, Direct=2).
    pub enum TPlaneConnectStyle: 2 {
        Relief = 0,
        Direct = 1,
        NoConnect = 2,
    }
}

// ── Polygon Hatch Style ─────────────────────────────────────────────────

dense_enum! {
    /// Polygon fill style — `TPolyHatchStyle` from SDK.
    pub enum TPolyHatchStyle: 5 {
        Hatch90 = 0,
        Hatch45 = 1,
        VHatch = 2,
        HHatch = 3,
        NoHatch = 4,
        Solid = 5,
    }
}

// ── Polygon Region Kind ──────────────────────────────────────────────────

dense_enum! {
    /// Region kind — `TPolyRegionKind` from SDK.
    pub enum TPolyRegionKind: 4 {
        Copper = 0,
        Cutout = 1,
        NamedRegion = 2,
        DashedOutline = 3,
        CavityDefinition = 4,
    }
}

// ── Polygon Type ─────────────────────────────────────────────────────────

dense_enum! {
    /// Polygon type — `TPolygonType` from SDK.
    pub enum TPolygonType: 1 {
        SignalLayer = 0,
        SplitPlane = 1,
    }
}

// ── Dimension Kind ───────────────────────────────────────────────────────

dense_enum! {
    /// Dimension type — `TDimensionKind` from SDK.
    pub enum TDimensionKind: 10 {
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
}

// ── Dimension Unit ───────────────────────────────────────────────────────

dense_enum! {
    /// Dimension display unit — `TDimensionUnit` from SDK.
    pub enum TDimensionUnit: 6 {
        Mils = 0,
        Inches = 1,
        Millimeters = 2,
        Centimeters = 3,
        Degrees = 4,
        Radians = 5,
        Automatic = 6,
    }
}

// ── Connection Mode ──────────────────────────────────────────────────────

dense_enum! {
    /// Connection mode — `TConnectionMode` from SDK.
    pub enum TConnectionMode: 1 {
        RatsNest = 0,
        BrokenNetMarker = 1,
    }
}

// ── Extended Hole Type ───────────────────────────────────────────────────

dense_enum! {
    /// Hole shape — `TExtendedHoleType` from SDK.
    pub enum TExtendedHoleType: 2 {
        Round = 0,
        Square = 1,
        Slot = 2,
    }
}

// ── Extended Drill Type ──────────────────────────────────────────────────

dense_enum! {
    /// Drill method — `TExtendedDrillType` from SDK.
    pub enum TExtendedDrillType: 3 {
        Drilled = 0,
        Punched = 1,
        LaserDrilled = 2,
        PlasmaDrilled = 3,
    }
}

// ── Text Autoposition ────────────────────────────────────────────────────

dense_enum! {
    /// Text auto-positioning — `TTextAutoposition` from SDK.
    pub enum TTextAutoposition: 9 {
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
}

// ── Dielectric Type ──────────────────────────────────────────────────────

dense_enum! {
    /// Layer stack dielectric — `TDielectricType` from SDK.
    pub enum TDielectricType: 3 {
        NoDielectric = 0,
        Core = 1,
        PrePreg = 2,
        SurfaceMaterial = 3,
    }
}

// ── Layer Stack Style ────────────────────────────────────────────────────

dense_enum! {
    /// Layer stack style — `TLayerStackStyle` from SDK.
    pub enum TLayerStackStyle: 2 {
        Pairs = 0,
        InsidePairs = 1,
        Buildup = 2,
    }
}

// ── Board Side ───────────────────────────────────────────────────────────

dense_enum! {
    /// Board side — `TBoardSide` from SDK.
    pub enum TBoardSide: 1 {
        Top = 0,
        Bottom = 1,
    }
}

// ── Unit ─────────────────────────────────────────────────────────────────

dense_enum! {
    /// Display unit — `TUnit` from SDK.
    pub enum TUnit: 1 {
        Metric = 0,
        Imperial = 1,
    }
}

// ── Text Type ────────────────────────────────────────────────────────────

dense_enum! {
    /// Text rendering type.
    pub enum TextType: 2 {
        Stroke = 0,
        TrueType = 1,
        Barcode = 2,
    }
}

// ── Stroke Font ──────────────────────────────────────────────────────────

dense_enum! {
    /// Stroke font type — `TStrokeFont` from SDK.
    pub enum StrokeFont: 7 {
        Default = 0,
        SansSerif = 1,
        Serif = 2,
        Proportional1 = 3,
        Proportional2 = 4,
        Proportional3 = 5,
        Proportional4 = 6,
        Proportional5 = 7,
    }
}

// ── Rule Kind ────────────────────────────────────────────────────────────

/// Design rule kind — `TRuleKind` from SDK.
///
/// 52 rule types (0-51). String IDs from `cRuleIdStrings`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl TRuleKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 51 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }

    /// Returns the parametric string ID for this rule kind.
    pub fn to_string_id(self) -> &'static str {
        crate::v2::pcb::constants::RULE_ID_STRINGS[self as usize]
    }

    /// Looks up a rule kind from its parametric string ID.
    pub fn from_string_id(s: &str) -> Option<Self> {
        crate::v2::pcb::constants::RULE_ID_STRINGS
            .iter()
            .position(|&id| id.eq_ignore_ascii_case(s))
            .and_then(|i| Self::from_u8(i as u8))
    }
}

// ── PCB Object ID ────────────────────────────────────────────────────────

dense_enum! {
    /// PCB record type ID — used in binary framing `u8 type` byte.
    pub enum PcbRecordType: 25 {
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
}

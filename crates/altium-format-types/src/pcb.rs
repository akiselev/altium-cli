use crate::InvalidEnumValue;

/// PCB record type discriminant (byte in binary format).
///
/// Values 0-26 from the Delphi/C# TObjectId enum.
/// No catch-all variant -- unknown values are parse errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
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
    Trace = 23,
    SpareVia = 24,
    Board = 25,
    BoardOutline = 26,
}

impl TryFrom<u8> for PcbObjectId {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::NoObject),
            1 => Ok(Self::Arc),
            2 => Ok(Self::Pad),
            3 => Ok(Self::Via),
            4 => Ok(Self::Track),
            5 => Ok(Self::Text),
            6 => Ok(Self::Fill),
            7 => Ok(Self::Connection),
            8 => Ok(Self::Net),
            9 => Ok(Self::Component),
            10 => Ok(Self::Polygon),
            11 => Ok(Self::Region),
            12 => Ok(Self::ComponentBody),
            13 => Ok(Self::Dimension),
            14 => Ok(Self::Coordinate),
            15 => Ok(Self::Class),
            16 => Ok(Self::Rule),
            17 => Ok(Self::FromTo),
            18 => Ok(Self::DifferentialPair),
            19 => Ok(Self::Violation),
            20 => Ok(Self::Embedded),
            21 => Ok(Self::EmbeddedBoard),
            22 => Ok(Self::SplitPlane),
            23 => Ok(Self::Trace),
            24 => Ok(Self::SpareVia),
            25 => Ok(Self::Board),
            26 => Ok(Self::BoardOutline),
            _ => Err(InvalidEnumValue {
                type_name: "PcbObjectId",
                value: v as i64,
            }),
        }
    }
}

impl PcbObjectId {
    /// Maps the PRIMITIVEOBJECTID string from UniqueIDPrimitiveInformation to PcbObjectId.
    ///
    /// These strings appear in PcbLib sidecar streams as human-readable type names.
    pub fn from_primitive_object_id_str(s: &str) -> Option<Self> {
        match s {
            "Arc" => Some(Self::Arc),
            "Pad" => Some(Self::Pad),
            "Via" => Some(Self::Via),
            "Track" => Some(Self::Track),
            "Text" => Some(Self::Text),
            "Fill" => Some(Self::Fill),
            "Region" => Some(Self::Region),
            "ComponentBody" => Some(Self::ComponentBody),
            _ => None,
        }
    }
}

impl std::fmt::Display for PcbObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoObject => write!(f, "NoObject"),
            Self::Arc => write!(f, "Arc"),
            Self::Pad => write!(f, "Pad"),
            Self::Via => write!(f, "Via"),
            Self::Track => write!(f, "Track"),
            Self::Text => write!(f, "Text"),
            Self::Fill => write!(f, "Fill"),
            Self::Connection => write!(f, "Connection"),
            Self::Net => write!(f, "Net"),
            Self::Component => write!(f, "Component"),
            Self::Polygon => write!(f, "Polygon"),
            Self::Region => write!(f, "Region"),
            Self::ComponentBody => write!(f, "ComponentBody"),
            Self::Dimension => write!(f, "Dimension"),
            Self::Coordinate => write!(f, "Coordinate"),
            Self::Class => write!(f, "Class"),
            Self::Rule => write!(f, "Rule"),
            Self::FromTo => write!(f, "FromTo"),
            Self::DifferentialPair => write!(f, "DifferentialPair"),
            Self::Violation => write!(f, "Violation"),
            Self::Embedded => write!(f, "Embedded"),
            Self::EmbeddedBoard => write!(f, "EmbeddedBoard"),
            Self::SplitPlane => write!(f, "SplitPlane"),
            Self::Trace => write!(f, "Trace"),
            Self::SpareVia => write!(f, "SpareVia"),
            Self::Board => write!(f, "Board"),
            Self::BoardOutline => write!(f, "BoardOutline"),
        }
    }
}

/// Viewable object ID from the C# `TViewableObjectID` enum (byte, 0-124).
///
/// This is an extended object classification used in UI-facing contexts such as
/// the PrimitiveGuids sidecar stream. It is a superset of `PcbObjectId` — the
/// first 11 values match, but after that the numbering diverges (e.g. dimension
/// subtypes, rule subtypes, groups, etc.).
///
/// Unlike `PcbObjectId` which identifies PCB binary record types, this enum
/// classifies objects for display/selection/GUID tracking purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(u8)]
pub enum ViewableObjectId {
    None = 0,
    Arc = 1,
    Pad = 2,
    Via = 3,
    Track = 4,
    Text = 5,
    Fill = 6,
    Connection = 7,
    Net = 8,
    Component = 9,
    Poly = 10,
    LinearDimension = 11,
    AngularDimension = 12,
    RadialDimension = 13,
    LeaderDimension = 14,
    DatumDimension = 15,
    BaselineDimension = 16,
    CenterDimension = 17,
    OriginalDimension = 18,
    LinearDiameterDimension = 19,
    RadialDiameterDimension = 20,
    Coordinate = 21,
    Class = 22,
    RuleClearance = 23,
    RuleParallelSegment = 24,
    RuleMaxMinWidth = 25,
    RuleMaxMinLength = 26,
    RuleMatchedLengths = 27,
    RuleDaisyChainStubLength = 28,
    RulePowerPlaneConnectStyle = 29,
    RuleRoutingTopology = 30,
    RuleRoutingPriority = 31,
    RuleRoutingLayers = 32,
    RuleRoutingCornerStyle = 33,
    RuleRoutingViaStyle = 34,
    RulePowerPlaneClearance = 35,
    RuleSolderMaskExpansion = 36,
    RulePasteMaskExpansion = 37,
    RuleShortCircuit = 38,
    RuleBrokenNets = 39,
    RuleViasUnderSmd = 40,
    RuleMaximumViaCount = 41,
    RuleMinimumAnnularRing = 42,
    RulePolygonConnectStyle = 43,
    RuleAcuteAngle = 44,
    RuleConfinementConstraint = 45,
    RuleSmdToCorner = 46,
    RuleComponentClearance = 47,
    RuleComponentRotations = 48,
    RulePermittedLayers = 49,
    RuleNetsToIgnore = 50,
    RuleSignalStimulus = 51,
    RuleOvershootFallingEdge = 52,
    RuleOvershootRisingEdge = 53,
    RuleUndershootFallingEdge = 54,
    RuleUndershootRisingEdge = 55,
    RuleMaxMinImpedance = 56,
    RuleSignalTopValue = 57,
    RuleSignalBaseValue = 58,
    RuleFlightTimeRisingEdge = 59,
    RuleFlightTimeFallingEdge = 60,
    RuleLayerStack = 61,
    RuleMaxSlopeRisingEdge = 62,
    RuleMaxSlopeFallingEdge = 63,
    RuleSupplyNets = 64,
    RuleMaxMinHoleSize = 65,
    RuleTestPointStyle = 66,
    RuleTestPointUsage = 67,
    RuleUnconnectedPin = 68,
    RuleSmdToPlane = 69,
    RuleSmdNeckDown = 70,
    RuleLayerPair = 71,
    RuleFanoutControl = 72,
    RuleMaxMinHeight = 73,
    RuleDifferentialPairs = 74,
    RuleHoleToHoleClearance = 75,
    RuleMinimumSolderMaskSliver = 76,
    RuleSilkToSolderMaskClearance = 77,
    RuleSilkToSilkClearance = 78,
    RuleNetAntennae = 79,
    FromTo = 80,
    DifferentialPair = 81,
    Violation = 82,
    Board = 83,
    BoardOutline = 84,
    Group = 85,
    Clipboard = 86,
    SplitPlane = 87,
    EmbeddedBoard = 88,
    Region = 89,
    ComponentBody = 90,
    RuleAssyTestPointStyle = 91,
    RuleAssyTestPointUsage = 92,
    OwnerDraw = 93,
    DrillTable = 94,
    ViaStitching = 95,
    LayerStackTable = 96,
    Viewport = 97,
    BoardRegion = 98,
    RuleSilkToBoardRegion = 99,
    AccordionObject = 100,
    OleObject = 101,
    RuleSmdPadEntry = 102,
    ViaShielding = 103,
    RuleUnpouredPolygon = 104,
    MultilineText = 105,
    RuleBoardOutlineClearance = 106,
    CoverlayPoly = 107,
    PinPair = 108,
    RuleBackDrilling = 109,
    StackedVia = 110,
    StaggeredVia = 111,
    RuleCreepage = 112,
    RuleReturnPath = 113,
    Rectangle = 114,
    RuleRoutingNeckDown = 115,
    Wirebond = 116,
    RuleWirebonding = 117,
    ReuseBlock = 118,
    RuleZAxisClearance = 119,
}

impl TryFrom<u8> for ViewableObjectId {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::None),
            1 => Ok(Self::Arc),
            2 => Ok(Self::Pad),
            3 => Ok(Self::Via),
            4 => Ok(Self::Track),
            5 => Ok(Self::Text),
            6 => Ok(Self::Fill),
            7 => Ok(Self::Connection),
            8 => Ok(Self::Net),
            9 => Ok(Self::Component),
            10 => Ok(Self::Poly),
            11 => Ok(Self::LinearDimension),
            12 => Ok(Self::AngularDimension),
            13 => Ok(Self::RadialDimension),
            14 => Ok(Self::LeaderDimension),
            15 => Ok(Self::DatumDimension),
            16 => Ok(Self::BaselineDimension),
            17 => Ok(Self::CenterDimension),
            18 => Ok(Self::OriginalDimension),
            19 => Ok(Self::LinearDiameterDimension),
            20 => Ok(Self::RadialDiameterDimension),
            21 => Ok(Self::Coordinate),
            22 => Ok(Self::Class),
            23 => Ok(Self::RuleClearance),
            24 => Ok(Self::RuleParallelSegment),
            25 => Ok(Self::RuleMaxMinWidth),
            26 => Ok(Self::RuleMaxMinLength),
            27 => Ok(Self::RuleMatchedLengths),
            28 => Ok(Self::RuleDaisyChainStubLength),
            29 => Ok(Self::RulePowerPlaneConnectStyle),
            30 => Ok(Self::RuleRoutingTopology),
            31 => Ok(Self::RuleRoutingPriority),
            32 => Ok(Self::RuleRoutingLayers),
            33 => Ok(Self::RuleRoutingCornerStyle),
            34 => Ok(Self::RuleRoutingViaStyle),
            35 => Ok(Self::RulePowerPlaneClearance),
            36 => Ok(Self::RuleSolderMaskExpansion),
            37 => Ok(Self::RulePasteMaskExpansion),
            38 => Ok(Self::RuleShortCircuit),
            39 => Ok(Self::RuleBrokenNets),
            40 => Ok(Self::RuleViasUnderSmd),
            41 => Ok(Self::RuleMaximumViaCount),
            42 => Ok(Self::RuleMinimumAnnularRing),
            43 => Ok(Self::RulePolygonConnectStyle),
            44 => Ok(Self::RuleAcuteAngle),
            45 => Ok(Self::RuleConfinementConstraint),
            46 => Ok(Self::RuleSmdToCorner),
            47 => Ok(Self::RuleComponentClearance),
            48 => Ok(Self::RuleComponentRotations),
            49 => Ok(Self::RulePermittedLayers),
            50 => Ok(Self::RuleNetsToIgnore),
            51 => Ok(Self::RuleSignalStimulus),
            52 => Ok(Self::RuleOvershootFallingEdge),
            53 => Ok(Self::RuleOvershootRisingEdge),
            54 => Ok(Self::RuleUndershootFallingEdge),
            55 => Ok(Self::RuleUndershootRisingEdge),
            56 => Ok(Self::RuleMaxMinImpedance),
            57 => Ok(Self::RuleSignalTopValue),
            58 => Ok(Self::RuleSignalBaseValue),
            59 => Ok(Self::RuleFlightTimeRisingEdge),
            60 => Ok(Self::RuleFlightTimeFallingEdge),
            61 => Ok(Self::RuleLayerStack),
            62 => Ok(Self::RuleMaxSlopeRisingEdge),
            63 => Ok(Self::RuleMaxSlopeFallingEdge),
            64 => Ok(Self::RuleSupplyNets),
            65 => Ok(Self::RuleMaxMinHoleSize),
            66 => Ok(Self::RuleTestPointStyle),
            67 => Ok(Self::RuleTestPointUsage),
            68 => Ok(Self::RuleUnconnectedPin),
            69 => Ok(Self::RuleSmdToPlane),
            70 => Ok(Self::RuleSmdNeckDown),
            71 => Ok(Self::RuleLayerPair),
            72 => Ok(Self::RuleFanoutControl),
            73 => Ok(Self::RuleMaxMinHeight),
            74 => Ok(Self::RuleDifferentialPairs),
            75 => Ok(Self::RuleHoleToHoleClearance),
            76 => Ok(Self::RuleMinimumSolderMaskSliver),
            77 => Ok(Self::RuleSilkToSolderMaskClearance),
            78 => Ok(Self::RuleSilkToSilkClearance),
            79 => Ok(Self::RuleNetAntennae),
            80 => Ok(Self::FromTo),
            81 => Ok(Self::DifferentialPair),
            82 => Ok(Self::Violation),
            83 => Ok(Self::Board),
            84 => Ok(Self::BoardOutline),
            85 => Ok(Self::Group),
            86 => Ok(Self::Clipboard),
            87 => Ok(Self::SplitPlane),
            88 => Ok(Self::EmbeddedBoard),
            89 => Ok(Self::Region),
            90 => Ok(Self::ComponentBody),
            91 => Ok(Self::RuleAssyTestPointStyle),
            92 => Ok(Self::RuleAssyTestPointUsage),
            93 => Ok(Self::OwnerDraw),
            94 => Ok(Self::DrillTable),
            95 => Ok(Self::ViaStitching),
            96 => Ok(Self::LayerStackTable),
            97 => Ok(Self::Viewport),
            98 => Ok(Self::BoardRegion),
            99 => Ok(Self::RuleSilkToBoardRegion),
            100 => Ok(Self::AccordionObject),
            101 => Ok(Self::OleObject),
            102 => Ok(Self::RuleSmdPadEntry),
            103 => Ok(Self::ViaShielding),
            104 => Ok(Self::RuleUnpouredPolygon),
            105 => Ok(Self::MultilineText),
            106 => Ok(Self::RuleBoardOutlineClearance),
            107 => Ok(Self::CoverlayPoly),
            108 => Ok(Self::PinPair),
            109 => Ok(Self::RuleBackDrilling),
            110 => Ok(Self::StackedVia),
            111 => Ok(Self::StaggeredVia),
            112 => Ok(Self::RuleCreepage),
            113 => Ok(Self::RuleReturnPath),
            114 => Ok(Self::Rectangle),
            115 => Ok(Self::RuleRoutingNeckDown),
            116 => Ok(Self::Wirebond),
            117 => Ok(Self::RuleWirebonding),
            118 => Ok(Self::ReuseBlock),
            119 => Ok(Self::RuleZAxisClearance),
            _ => Err(InvalidEnumValue {
                type_name: "ViewableObjectId",
                value: v as i64,
            }),
        }
    }
}

/// V6 layer ID (byte, 0-82). Used in the binary file format.
///
/// No catch-all -- unknown layer bytes are parse errors.
/// Byte values follow the Delphi binary format mapping (ground truth).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum V6Layer {
    #[default]
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
    DrcErrorLayer = 77,
    HighlightLayer = 78,
    GridColor1 = 79,
    GridColor10 = 80,
    PadHoleLayer = 81,
    ViaHoleLayer = 82,
}

impl TryFrom<u8> for V6Layer {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::NoLayer),
            1 => Ok(Self::TopLayer),
            2 => Ok(Self::MidLayer1),
            3 => Ok(Self::MidLayer2),
            4 => Ok(Self::MidLayer3),
            5 => Ok(Self::MidLayer4),
            6 => Ok(Self::MidLayer5),
            7 => Ok(Self::MidLayer6),
            8 => Ok(Self::MidLayer7),
            9 => Ok(Self::MidLayer8),
            10 => Ok(Self::MidLayer9),
            11 => Ok(Self::MidLayer10),
            12 => Ok(Self::MidLayer11),
            13 => Ok(Self::MidLayer12),
            14 => Ok(Self::MidLayer13),
            15 => Ok(Self::MidLayer14),
            16 => Ok(Self::MidLayer15),
            17 => Ok(Self::MidLayer16),
            18 => Ok(Self::MidLayer17),
            19 => Ok(Self::MidLayer18),
            20 => Ok(Self::MidLayer19),
            21 => Ok(Self::MidLayer20),
            22 => Ok(Self::MidLayer21),
            23 => Ok(Self::MidLayer22),
            24 => Ok(Self::MidLayer23),
            25 => Ok(Self::MidLayer24),
            26 => Ok(Self::MidLayer25),
            27 => Ok(Self::MidLayer26),
            28 => Ok(Self::MidLayer27),
            29 => Ok(Self::MidLayer28),
            30 => Ok(Self::MidLayer29),
            31 => Ok(Self::MidLayer30),
            32 => Ok(Self::BottomLayer),
            33 => Ok(Self::TopOverlay),
            34 => Ok(Self::BottomOverlay),
            35 => Ok(Self::TopPaste),
            36 => Ok(Self::BottomPaste),
            37 => Ok(Self::TopSolder),
            38 => Ok(Self::BottomSolder),
            39 => Ok(Self::InternalPlane1),
            40 => Ok(Self::InternalPlane2),
            41 => Ok(Self::InternalPlane3),
            42 => Ok(Self::InternalPlane4),
            43 => Ok(Self::InternalPlane5),
            44 => Ok(Self::InternalPlane6),
            45 => Ok(Self::InternalPlane7),
            46 => Ok(Self::InternalPlane8),
            47 => Ok(Self::InternalPlane9),
            48 => Ok(Self::InternalPlane10),
            49 => Ok(Self::InternalPlane11),
            50 => Ok(Self::InternalPlane12),
            51 => Ok(Self::InternalPlane13),
            52 => Ok(Self::InternalPlane14),
            53 => Ok(Self::InternalPlane15),
            54 => Ok(Self::InternalPlane16),
            55 => Ok(Self::DrillGuide),
            56 => Ok(Self::KeepOutLayer),
            57 => Ok(Self::Mechanical1),
            58 => Ok(Self::Mechanical2),
            59 => Ok(Self::Mechanical3),
            60 => Ok(Self::Mechanical4),
            61 => Ok(Self::Mechanical5),
            62 => Ok(Self::Mechanical6),
            63 => Ok(Self::Mechanical7),
            64 => Ok(Self::Mechanical8),
            65 => Ok(Self::Mechanical9),
            66 => Ok(Self::Mechanical10),
            67 => Ok(Self::Mechanical11),
            68 => Ok(Self::Mechanical12),
            69 => Ok(Self::Mechanical13),
            70 => Ok(Self::Mechanical14),
            71 => Ok(Self::Mechanical15),
            72 => Ok(Self::Mechanical16),
            73 => Ok(Self::DrillDrawing),
            74 => Ok(Self::MultiLayer),
            75 => Ok(Self::ConnectLayer),
            76 => Ok(Self::BackGroundLayer),
            77 => Ok(Self::DrcErrorLayer),
            78 => Ok(Self::HighlightLayer),
            79 => Ok(Self::GridColor1),
            80 => Ok(Self::GridColor10),
            81 => Ok(Self::PadHoleLayer),
            82 => Ok(Self::ViaHoleLayer),
            _ => Err(InvalidEnumValue {
                type_name: "V6Layer",
                value: v as i64,
            }),
        }
    }
}

impl V6Layer {
    /// Signal layers: TopLayer (1) through BottomLayer (32).
    pub fn is_signal(self) -> bool {
        let b = self as u8;
        b >= 1 && b <= 32
    }

    /// Mid signal layers: MidLayer1 (2) through MidLayer30 (31).
    pub fn is_mid_signal(self) -> bool {
        let b = self as u8;
        b >= 2 && b <= 31
    }

    /// Internal plane layers 1-16 (bytes 39-54).
    pub fn is_internal_plane(self) -> bool {
        let b = self as u8;
        b >= 39 && b <= 54
    }

    /// Mechanical layers 1-16 (bytes 57-72).
    pub fn is_mechanical(self) -> bool {
        let b = self as u8;
        b >= 57 && b <= 72
    }

    /// Copper-carrying layers (signal layers + MultiLayer + InternalPlanes).
    pub fn is_copper(self) -> bool {
        self.is_signal() || self == V6Layer::MultiLayer || self.is_internal_plane()
    }

    /// Overlay (silkscreen) layers.
    pub fn is_overlay(self) -> bool {
        self == V6Layer::TopOverlay || self == V6Layer::BottomOverlay
    }

    /// Solder mask layers.
    pub fn is_solder_mask(self) -> bool {
        self == V6Layer::TopSolder || self == V6Layer::BottomSolder
    }

    /// Paste mask layers.
    pub fn is_paste_mask(self) -> bool {
        self == V6Layer::TopPaste || self == V6Layer::BottomPaste
    }

    /// Internal plane number (1-16) if this is an internal plane layer.
    pub fn internal_plane_number(self) -> Option<u8> {
        let b = self as u8;
        if b >= 39 && b <= 54 {
            Some(b - 38)
        } else {
            None
        }
    }

    /// Mechanical layer number (1-16) if this is a mechanical layer.
    pub fn mechanical_number(self) -> Option<u8> {
        let b = self as u8;
        if b >= 57 && b <= 72 {
            Some(b - 56)
        } else {
            None
        }
    }

    /// Human-readable layer name string matching the Altium cLayerStrings mapping.
    pub fn to_string_name(self) -> &'static str {
        match self {
            V6Layer::NoLayer => "NoLayer",
            V6Layer::TopLayer => "TopLayer",
            V6Layer::MidLayer1 => "MidLayer1",
            V6Layer::MidLayer2 => "MidLayer2",
            V6Layer::MidLayer3 => "MidLayer3",
            V6Layer::MidLayer4 => "MidLayer4",
            V6Layer::MidLayer5 => "MidLayer5",
            V6Layer::MidLayer6 => "MidLayer6",
            V6Layer::MidLayer7 => "MidLayer7",
            V6Layer::MidLayer8 => "MidLayer8",
            V6Layer::MidLayer9 => "MidLayer9",
            V6Layer::MidLayer10 => "MidLayer10",
            V6Layer::MidLayer11 => "MidLayer11",
            V6Layer::MidLayer12 => "MidLayer12",
            V6Layer::MidLayer13 => "MidLayer13",
            V6Layer::MidLayer14 => "MidLayer14",
            V6Layer::MidLayer15 => "MidLayer15",
            V6Layer::MidLayer16 => "MidLayer16",
            V6Layer::MidLayer17 => "MidLayer17",
            V6Layer::MidLayer18 => "MidLayer18",
            V6Layer::MidLayer19 => "MidLayer19",
            V6Layer::MidLayer20 => "MidLayer20",
            V6Layer::MidLayer21 => "MidLayer21",
            V6Layer::MidLayer22 => "MidLayer22",
            V6Layer::MidLayer23 => "MidLayer23",
            V6Layer::MidLayer24 => "MidLayer24",
            V6Layer::MidLayer25 => "MidLayer25",
            V6Layer::MidLayer26 => "MidLayer26",
            V6Layer::MidLayer27 => "MidLayer27",
            V6Layer::MidLayer28 => "MidLayer28",
            V6Layer::MidLayer29 => "MidLayer29",
            V6Layer::MidLayer30 => "MidLayer30",
            V6Layer::BottomLayer => "BottomLayer",
            V6Layer::TopOverlay => "TopOverlay",
            V6Layer::BottomOverlay => "BottomOverlay",
            V6Layer::TopPaste => "TopPaste",
            V6Layer::BottomPaste => "BottomPaste",
            V6Layer::TopSolder => "TopSolder",
            V6Layer::BottomSolder => "BottomSolder",
            V6Layer::InternalPlane1 => "InternalPlane1",
            V6Layer::InternalPlane2 => "InternalPlane2",
            V6Layer::InternalPlane3 => "InternalPlane3",
            V6Layer::InternalPlane4 => "InternalPlane4",
            V6Layer::InternalPlane5 => "InternalPlane5",
            V6Layer::InternalPlane6 => "InternalPlane6",
            V6Layer::InternalPlane7 => "InternalPlane7",
            V6Layer::InternalPlane8 => "InternalPlane8",
            V6Layer::InternalPlane9 => "InternalPlane9",
            V6Layer::InternalPlane10 => "InternalPlane10",
            V6Layer::InternalPlane11 => "InternalPlane11",
            V6Layer::InternalPlane12 => "InternalPlane12",
            V6Layer::InternalPlane13 => "InternalPlane13",
            V6Layer::InternalPlane14 => "InternalPlane14",
            V6Layer::InternalPlane15 => "InternalPlane15",
            V6Layer::InternalPlane16 => "InternalPlane16",
            V6Layer::DrillGuide => "DrillGuide",
            V6Layer::KeepOutLayer => "KeepOutLayer",
            V6Layer::Mechanical1 => "Mechanical1",
            V6Layer::Mechanical2 => "Mechanical2",
            V6Layer::Mechanical3 => "Mechanical3",
            V6Layer::Mechanical4 => "Mechanical4",
            V6Layer::Mechanical5 => "Mechanical5",
            V6Layer::Mechanical6 => "Mechanical6",
            V6Layer::Mechanical7 => "Mechanical7",
            V6Layer::Mechanical8 => "Mechanical8",
            V6Layer::Mechanical9 => "Mechanical9",
            V6Layer::Mechanical10 => "Mechanical10",
            V6Layer::Mechanical11 => "Mechanical11",
            V6Layer::Mechanical12 => "Mechanical12",
            V6Layer::Mechanical13 => "Mechanical13",
            V6Layer::Mechanical14 => "Mechanical14",
            V6Layer::Mechanical15 => "Mechanical15",
            V6Layer::Mechanical16 => "Mechanical16",
            V6Layer::DrillDrawing => "DrillDrawing",
            V6Layer::MultiLayer => "MultiLayer",
            V6Layer::ConnectLayer => "ConnectLayer",
            V6Layer::BackGroundLayer => "BackGroundLayer",
            V6Layer::DrcErrorLayer => "DRCErrorLayer",
            V6Layer::HighlightLayer => "HighlightLayer",
            V6Layer::GridColor1 => "GridColor1",
            V6Layer::GridColor10 => "GridColor10",
            V6Layer::PadHoleLayer => "PadHoleLayer",
            V6Layer::ViaHoleLayer => "ViaHoleLayer",
        }
    }

    /// Reverse lookup from layer name string to V6Layer.
    pub fn from_string_name(name: &str) -> Option<Self> {
        constants::LAYER_STRINGS
            .iter()
            .position(|&s| s.eq_ignore_ascii_case(name))
            .and_then(|i| V6Layer::try_from(i as u8).ok())
    }
}

impl std::fmt::Display for V6Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_string_name())
    }
}

/// V7 extended layer ID (32-bit structured).
///
/// Layout (from Delphi/C# struct with explicit field offsets):
/// ```text
/// Byte 0-1 (u16): Species (layer-specific index)
/// Byte 2   (u8):  Genus (layer category)
/// Byte 3   (u8):  Family (copper/dielectric/etc.)
/// ```
///
/// When genus=0 and family=0, the species low byte matches V6 layer IDs
/// (backward-compatible).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct V7Layer(u32);

impl V7Layer {
    pub fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub fn raw(self) -> u32 {
        self.0
    }

    pub fn species(self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }

    pub fn genus(self) -> u8 {
        ((self.0 >> 16) & 0xFF) as u8
    }

    pub fn family(self) -> u8 {
        ((self.0 >> 24) & 0xFF) as u8
    }

    /// Convert to V6 layer if this is a legacy-compatible layer (genus=0, family=0).
    /// Returns Err with the raw u32 value if not convertible.
    pub fn to_v6(self) -> Result<V6Layer, u32> {
        if self.genus() == 0 && self.family() == 0 {
            let species_low = (self.species() & 0xFF) as u8;
            V6Layer::try_from(species_low).map_err(|_| self.0)
        } else {
            Err(self.0)
        }
    }

    /// Create from V6 layer.
    pub fn from_v6(layer: V6Layer) -> Self {
        Self(layer as u32)
    }
}

/// PCB primitive flags bitmask (u16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PcbFlags(u16);

impl PcbFlags {
    pub fn new(raw: u16) -> Self {
        Self(raw)
    }

    pub fn selected(self) -> bool {
        self.0 & 0x01 != 0
    }

    pub fn locked(self) -> bool {
        self.0 & 0x10 != 0
    }

    pub fn union_member(self) -> bool {
        self.0 & 0x80 != 0
    }

    pub fn raw(self) -> u16 {
        self.0
    }
}

/// Pad/via shape (0-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PadShape {
    NoShape = 0,
    #[default]
    Round = 1,
    Rectangular = 2,
    Octagonal = 3,
    Circle = 4,
    Arc = 5,
    Terminator = 6,
    RoundRect = 7,
    RotatedRect = 8,
    RoundedRectangular = 9,
    Custom = 10,
}

impl TryFrom<u8> for PadShape {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::NoShape),
            1 => Ok(Self::Round),
            2 => Ok(Self::Rectangular),
            3 => Ok(Self::Octagonal),
            4 => Ok(Self::Circle),
            5 => Ok(Self::Arc),
            6 => Ok(Self::Terminator),
            7 => Ok(Self::RoundRect),
            8 => Ok(Self::RotatedRect),
            9 => Ok(Self::RoundedRectangular),
            10 => Ok(Self::Custom),
            _ => Err(InvalidEnumValue {
                type_name: "PadShape",
                value: v as i64,
            }),
        }
    }
}

impl std::fmt::Display for PadShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Pad shape sub-kind (0-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PadShapeSubKind {
    #[default]
    NoKind = 0,
    OctagonalFinger = 1,
    RoundedFinger = 2,
    RoundedRectangle = 3,
    ChamferedRectangle = 4,
    Donut = 5,
}

impl TryFrom<u8> for PadShapeSubKind {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::NoKind),
            1 => Ok(Self::OctagonalFinger),
            2 => Ok(Self::RoundedFinger),
            3 => Ok(Self::RoundedRectangle),
            4 => Ok(Self::ChamferedRectangle),
            5 => Ok(Self::Donut),
            _ => Err(InvalidEnumValue {
                type_name: "PadShapeSubKind",
                value: v as i64,
            }),
        }
    }
}

impl std::fmt::Display for PadShapeSubKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Pad stack mode (0-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PadStackMode {
    #[default]
    Simple = 0,
    LocalStack = 1,
    ExternalStack = 2,
}

impl TryFrom<u8> for PadStackMode {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Simple),
            1 => Ok(Self::LocalStack),
            2 => Ok(Self::ExternalStack),
            _ => Err(InvalidEnumValue {
                type_name: "PadStackMode",
                value: v as i64,
            }),
        }
    }
}

impl std::fmt::Display for PadStackMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Hole type (0-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum HoleType {
    #[default]
    Round = 0,
    Square = 1,
    Slot = 2,
}

impl TryFrom<u8> for HoleType {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Round),
            1 => Ok(Self::Square),
            2 => Ok(Self::Slot),
            _ => Err(InvalidEnumValue {
                type_name: "HoleType",
                value: v as i64,
            }),
        }
    }
}

/// Drill type (0-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum DrillType {
    #[default]
    Drilled = 0,
    Punched = 1,
    LaserDrilled = 2,
    PlasmaDrilled = 3,
}

impl TryFrom<u8> for DrillType {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Drilled),
            1 => Ok(Self::Punched),
            2 => Ok(Self::LaserDrilled),
            3 => Ok(Self::PlasmaDrilled),
            _ => Err(InvalidEnumValue {
                type_name: "DrillType",
                value: v as i64,
            }),
        }
    }
}

/// Drill layer pair type (0-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum DrillLayerPairType {
    #[default]
    Regular = 0,
    MicroViaDrill = 1,
    Backdrill = 2,
    CounterHole = 3,
}

impl TryFrom<u8> for DrillLayerPairType {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Regular),
            1 => Ok(Self::MicroViaDrill),
            2 => Ok(Self::Backdrill),
            3 => Ok(Self::CounterHole),
            _ => Err(InvalidEnumValue {
                type_name: "DrillLayerPairType",
                value: v as i64,
            }),
        }
    }
}

/// PCB text kind (0-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum TextKind {
    #[default]
    StrokeFont = 0,
    TrueTypeFont = 1,
    Barcode = 2,
}

impl TryFrom<u8> for TextKind {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::StrokeFont),
            1 => Ok(Self::TrueTypeFont),
            2 => Ok(Self::Barcode),
            _ => Err(InvalidEnumValue {
                type_name: "TextKind",
                value: v as i64,
            }),
        }
    }
}

/// Text autoposition setting controlling where text anchors relative to a component.
///
/// Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TTextAutoposition.cs`
/// `TTextAutoposition = (eAutoPos_Manual, eAutoPos_TopLeft, ..., eAutoPos_BottomRight)`
///
/// Used by `IPCB_Text::GetState_TTFInvertedTextJustify` and
/// `IPCB_Text::GetState_MultilineTextAutoPosition`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum TextAutoposition {
    #[default]
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

impl TryFrom<u8> for TextAutoposition {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Manual),
            1 => Ok(Self::TopLeft),
            2 => Ok(Self::CenterLeft),
            3 => Ok(Self::BottomLeft),
            4 => Ok(Self::TopCenter),
            5 => Ok(Self::CenterCenter),
            6 => Ok(Self::BottomCenter),
            7 => Ok(Self::TopRight),
            8 => Ok(Self::CenterRight),
            9 => Ok(Self::BottomRight),
            _ => Err(InvalidEnumValue {
                type_name: "TextAutoposition",
                value: v as i64,
            }),
        }
    }
}

/// Barcode kind (0-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum BarcodeKind {
    #[default]
    Code39 = 0,
    Code128 = 1,
    QrCode = 2,
    DataMatrix = 3,
}

impl TryFrom<u8> for BarcodeKind {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Code39),
            1 => Ok(Self::Code128),
            2 => Ok(Self::QrCode),
            3 => Ok(Self::DataMatrix),
            _ => Err(InvalidEnumValue {
                type_name: "BarcodeKind",
                value: v as i64,
            }),
        }
    }
}

/// Barcode render mode (0-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum BarcodeRenderMode {
    #[default]
    ByMinWidth = 0,
    ByFullWidth = 1,
}

impl TryFrom<u8> for BarcodeRenderMode {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::ByMinWidth),
            1 => Ok(Self::ByFullWidth),
            _ => Err(InvalidEnumValue {
                type_name: "BarcodeRenderMode",
                value: v as i64,
            }),
        }
    }
}

/// Polygon hatch style (0-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PolyHatchStyle {
    Hatch90 = 0,
    Hatch45 = 1,
    VerticalHatch = 2,
    HorizontalHatch = 3,
    NoHatch = 4,
    #[default]
    Solid = 5,
}

impl TryFrom<u8> for PolyHatchStyle {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Hatch90),
            1 => Ok(Self::Hatch45),
            2 => Ok(Self::VerticalHatch),
            3 => Ok(Self::HorizontalHatch),
            4 => Ok(Self::NoHatch),
            5 => Ok(Self::Solid),
            _ => Err(InvalidEnumValue {
                type_name: "PolyHatchStyle",
                value: v as i64,
            }),
        }
    }
}

/// Polygon type (0-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PolygonType {
    #[default]
    SignalLayer = 0,
    SplitPlane = 1,
    CoverlayOutline = 2,
}

impl TryFrom<u8> for PolygonType {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::SignalLayer),
            1 => Ok(Self::SplitPlane),
            2 => Ok(Self::CoverlayOutline),
            _ => Err(InvalidEnumValue {
                type_name: "PolygonType",
                value: v as i64,
            }),
        }
    }
}

/// Region kind (0-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum RegionKind {
    #[default]
    Copper = 0,
    Cutout = 1,
    Named = 2,
    BoardCutout = 3,
    Cavity = 4,
}

impl TryFrom<u8> for RegionKind {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Copper),
            1 => Ok(Self::Cutout),
            2 => Ok(Self::Named),
            3 => Ok(Self::BoardCutout),
            4 => Ok(Self::Cavity),
            _ => Err(InvalidEnumValue {
                type_name: "RegionKind",
                value: v as i64,
            }),
        }
    }
}

/// Plane connection style (0-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PlaneConnectionStyle {
    #[default]
    NoConnect = 0,
    Relief = 1,
    Direct = 2,
}

impl TryFrom<u8> for PlaneConnectionStyle {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::NoConnect),
            1 => Ok(Self::Relief),
            2 => Ok(Self::Direct),
            _ => Err(InvalidEnumValue {
                type_name: "PlaneConnectionStyle",
                value: v as i64,
            }),
        }
    }
}

// TextAutoPosition is defined in crate::common (shared across PCB and schematic).

/// Mask expansion mode (from ExtendedPrimitiveInformation sidecar).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum MaskExpansionMode {
    #[default]
    NoMask = 0,
    Rule = 1,
    Manual = 2,
}

impl TryFrom<u8> for MaskExpansionMode {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::NoMask),
            1 => Ok(Self::Rule),
            2 => Ok(Self::Manual),
            _ => Err(InvalidEnumValue {
                type_name: "MaskExpansionMode",
                value: v as i64,
            }),
        }
    }
}

impl std::fmt::Display for MaskExpansionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoMask => write!(f, "NoMask"),
            Self::Rule => write!(f, "Rule"),
            Self::Manual => write!(f, "Manual"),
        }
    }
}

/// Cache state for pad cache fields (TV6_PadCache validity flags).
///
/// From `TCacheState.cs` in AD26-dotnet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum TCacheState {
    #[default]
    Invalid = 0,
    Valid = 1,
    Manual = 2,
}

impl TryFrom<u8> for TCacheState {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Invalid),
            1 => Ok(Self::Valid),
            2 => Ok(Self::Manual),
            _ => Err(InvalidEnumValue {
                type_name: "TCacheState",
                value: v as i64,
            }),
        }
    }
}

/// Tenting mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum TentingMode {
    #[default]
    None = 0,
    Top = 1,
    Bottom = 2,
    Both = 3,
}

impl TryFrom<u8> for TentingMode {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::None),
            1 => Ok(Self::Top),
            2 => Ok(Self::Bottom),
            3 => Ok(Self::Both),
            _ => Err(InvalidEnumValue {
                type_name: "TentingMode",
                value: v as i64,
            }),
        }
    }
}

impl std::fmt::Display for TentingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Top => write!(f, "Top"),
            Self::Bottom => write!(f, "Bottom"),
            Self::Both => write!(f, "Both"),
        }
    }
}

/// Daisy chain connection style for pads.
///
/// Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TDaisyChainStyle.cs`
/// `TDaisyChainStyle = (eDaisyChainLoad, eDaisyChainTerminator, eDaisyChainSource)`
///
/// Used by `IPCB_Pad3::GetState_DaisyChainStyle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum DaisyChainStyle {
    #[default]
    Load = 0,
    Terminator = 1,
    Source = 2,
}

impl TryFrom<u8> for DaisyChainStyle {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Load),
            1 => Ok(Self::Terminator),
            2 => Ok(Self::Source),
            _ => Err(InvalidEnumValue {
                type_name: "DaisyChainStyle",
                value: v as i64,
            }),
        }
    }
}

/// Dimension kind (0-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum DimensionKind {
    #[default]
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

impl TryFrom<u8> for DimensionKind {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::NoDimension),
            1 => Ok(Self::Linear),
            2 => Ok(Self::Angular),
            3 => Ok(Self::Radial),
            4 => Ok(Self::Leader),
            5 => Ok(Self::Datum),
            6 => Ok(Self::Baseline),
            7 => Ok(Self::Center),
            8 => Ok(Self::Original),
            9 => Ok(Self::LinearDiameter),
            10 => Ok(Self::RadialDiameter),
            _ => Err(InvalidEnumValue {
                type_name: "DimensionKind",
                value: v as i64,
            }),
        }
    }
}

/// PCB file format version (0-16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(u8)]
pub enum PcbFileFormatVersion {
    None = 0,
    BinaryV3 = 1,
    LibraryV3 = 2,
    AsciiV3 = 3,
    BinaryV4 = 4,
    LibraryV4 = 5,
    AsciiV4 = 6,
    BinaryV5 = 7,
    LibraryV5 = 8,
    AsciiV5 = 9,
    BinaryV6 = 10,
    LibraryV6 = 11,
    AsciiV6 = 12,
    BinaryV6CS = 13,
    BinaryV6CM = 14,
    BinaryV6PCBWorks = 15,
    PadViaLibraryV6 = 16,
}

impl Default for PcbFileFormatVersion {
    fn default() -> Self {
        Self::None
    }
}

impl TryFrom<u8> for PcbFileFormatVersion {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::None),
            1 => Ok(Self::BinaryV3),
            2 => Ok(Self::LibraryV3),
            3 => Ok(Self::AsciiV3),
            4 => Ok(Self::BinaryV4),
            5 => Ok(Self::LibraryV4),
            6 => Ok(Self::AsciiV4),
            7 => Ok(Self::BinaryV5),
            8 => Ok(Self::LibraryV5),
            9 => Ok(Self::AsciiV5),
            10 => Ok(Self::BinaryV6),
            11 => Ok(Self::LibraryV6),
            12 => Ok(Self::AsciiV6),
            13 => Ok(Self::BinaryV6CS),
            14 => Ok(Self::BinaryV6CM),
            15 => Ok(Self::BinaryV6PCBWorks),
            16 => Ok(Self::PadViaLibraryV6),
            _ => Err(InvalidEnumValue {
                type_name: "PcbFileFormatVersion",
                value: v as i64,
            }),
        }
    }
}

/// Board side (top/bottom).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum BoardSide {
    #[default]
    Top = 0,
    Bottom = 1,
}

impl TryFrom<u8> for BoardSide {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Top),
            1 => Ok(Self::Bottom),
            _ => Err(InvalidEnumValue {
                type_name: "BoardSide",
                value: v as i64,
            }),
        }
    }
}

/// Corner routing style (0-2). From TCornerStyle in altium-types.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum CornerStyle {
    #[default]
    Degree90 = 0,
    Degree45 = 1,
    Round = 2,
}

impl TryFrom<u8> for CornerStyle {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Degree90),
            1 => Ok(Self::Degree45),
            2 => Ok(Self::Round),
            _ => Err(InvalidEnumValue {
                type_name: "CornerStyle",
                value: v as i64,
            }),
        }
    }
}

/// Dielectric layer type (0-4). From TDielectricType in TDielectricType.cs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum DielectricType {
    #[default]
    NoDielectric = 0,
    Core = 1,
    PrePreg = 2,
    SurfaceMaterial = 3,
    Film = 4,
}

impl TryFrom<u8> for DielectricType {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::NoDielectric),
            1 => Ok(Self::Core),
            2 => Ok(Self::PrePreg),
            3 => Ok(Self::SurfaceMaterial),
            4 => Ok(Self::Film),
            _ => Err(InvalidEnumValue {
                type_name: "DielectricType",
                value: v as i64,
            }),
        }
    }
}

/// Layer stack style (0-3). From TLayerStackStyle in TLayerStackStyle.cs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum LayerStackStyle {
    #[default]
    Pairs = 0,
    InsidePairs = 1,
    Buildup = 2,
    Custom = 3,
}

impl TryFrom<u8> for LayerStackStyle {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Pairs),
            1 => Ok(Self::InsidePairs),
            2 => Ok(Self::Buildup),
            3 => Ok(Self::Custom),
            _ => Err(InvalidEnumValue {
                type_name: "LayerStackStyle",
                value: v as i64,
            }),
        }
    }
}

/// Component placement type (0-2). From TComponentPlacementType in TComponentPlacementType.cs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum ComponentPlacementType {
    #[default]
    None = 0,
    BodyUp = 1,
    BodyDown = 2,
}

impl TryFrom<u8> for ComponentPlacementType {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::None),
            1 => Ok(Self::BodyUp),
            2 => Ok(Self::BodyDown),
            _ => Err(InvalidEnumValue {
                type_name: "ComponentPlacementType",
                value: v as i64,
            }),
        }
    }
}

/// Design rule kind. From TRuleKind in altium-types.md and pcb-dotnet-model.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum RuleKind {
    #[default]
    Clearance = 0,
    ParallelSegment = 1,
    Width = 2,
    Length = 3,
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
    ViasUnderSmd = 17,
    MaximumViaCount = 18,
    MinimumAnnularRing = 19,
    PolygonConnectStyle = 20,
    AcuteAngle = 21,
    ConfinementConstraint = 22,
    SmdToCorner = 23,
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
    FabricationTestpointStyle = 43,
    FabricationTestpointUsage = 44,
    UnconnectedPin = 45,
    SmdToPlane = 46,
    SmdNeckDown = 47,
    LayerPair = 48,
    FanoutControl = 49,
    MaxMinHeight = 50,
    DifferentialPairsRouting = 51,
    HoleToHoleClearance = 52,
    MinimumSolderMaskSliver = 53,
    SilkToSolderMaskClearance = 54,
    SilkToSilkClearance = 55,
    NetAntennae = 56,
    AssyTestPointStyle = 57,
    AssyTestPointUsage = 58,
    SilkToBoardRegionClearance = 59,
    SmdEntry = 60,
    None = 61,
    UnpouredPolygon = 62,
    BoardOutlineClearance = 63,
    BackDrilling = 64,
    Creepage = 65,
    ReturnPath = 66,
    RoutingNeckDown = 67,
    WireBonding = 68,
    ZAxisClearance = 69,
}

impl TryFrom<u8> for RuleKind {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Clearance),
            1 => Ok(Self::ParallelSegment),
            2 => Ok(Self::Width),
            3 => Ok(Self::Length),
            4 => Ok(Self::MatchedLengths),
            5 => Ok(Self::DaisyChainStubLength),
            6 => Ok(Self::PowerPlaneConnectStyle),
            7 => Ok(Self::RoutingTopology),
            8 => Ok(Self::RoutingPriority),
            9 => Ok(Self::RoutingLayers),
            10 => Ok(Self::RoutingCornerStyle),
            11 => Ok(Self::RoutingViaStyle),
            12 => Ok(Self::PowerPlaneClearance),
            13 => Ok(Self::SolderMaskExpansion),
            14 => Ok(Self::PasteMaskExpansion),
            15 => Ok(Self::ShortCircuit),
            16 => Ok(Self::BrokenNets),
            17 => Ok(Self::ViasUnderSmd),
            18 => Ok(Self::MaximumViaCount),
            19 => Ok(Self::MinimumAnnularRing),
            20 => Ok(Self::PolygonConnectStyle),
            21 => Ok(Self::AcuteAngle),
            22 => Ok(Self::ConfinementConstraint),
            23 => Ok(Self::SmdToCorner),
            24 => Ok(Self::ComponentClearance),
            25 => Ok(Self::ComponentRotations),
            26 => Ok(Self::PermittedLayers),
            27 => Ok(Self::NetsToIgnore),
            28 => Ok(Self::SignalStimulus),
            29 => Ok(Self::OvershootFallingEdge),
            30 => Ok(Self::OvershootRisingEdge),
            31 => Ok(Self::UndershootFallingEdge),
            32 => Ok(Self::UndershootRisingEdge),
            33 => Ok(Self::MaxMinImpedance),
            34 => Ok(Self::SignalTopValue),
            35 => Ok(Self::SignalBaseValue),
            36 => Ok(Self::FlightTimeRisingEdge),
            37 => Ok(Self::FlightTimeFallingEdge),
            38 => Ok(Self::LayerStack),
            39 => Ok(Self::MaxSlopeRisingEdge),
            40 => Ok(Self::MaxSlopeFallingEdge),
            41 => Ok(Self::SupplyNets),
            42 => Ok(Self::MaxMinHoleSize),
            43 => Ok(Self::FabricationTestpointStyle),
            44 => Ok(Self::FabricationTestpointUsage),
            45 => Ok(Self::UnconnectedPin),
            46 => Ok(Self::SmdToPlane),
            47 => Ok(Self::SmdNeckDown),
            48 => Ok(Self::LayerPair),
            49 => Ok(Self::FanoutControl),
            50 => Ok(Self::MaxMinHeight),
            51 => Ok(Self::DifferentialPairsRouting),
            52 => Ok(Self::HoleToHoleClearance),
            53 => Ok(Self::MinimumSolderMaskSliver),
            54 => Ok(Self::SilkToSolderMaskClearance),
            55 => Ok(Self::SilkToSilkClearance),
            56 => Ok(Self::NetAntennae),
            57 => Ok(Self::AssyTestPointStyle),
            58 => Ok(Self::AssyTestPointUsage),
            59 => Ok(Self::SilkToBoardRegionClearance),
            60 => Ok(Self::SmdEntry),
            61 => Ok(Self::None),
            62 => Ok(Self::UnpouredPolygon),
            63 => Ok(Self::BoardOutlineClearance),
            64 => Ok(Self::BackDrilling),
            65 => Ok(Self::Creepage),
            66 => Ok(Self::ReturnPath),
            67 => Ok(Self::RoutingNeckDown),
            68 => Ok(Self::WireBonding),
            69 => Ok(Self::ZAxisClearance),
            _ => Err(InvalidEnumValue {
                type_name: "RuleKind",
                value: v as i64,
            }),
        }
    }
}

impl std::fmt::Display for RuleKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Clearance => write!(f, "Clearance"),
            Self::ParallelSegment => write!(f, "ParallelSegment"),
            Self::Width => write!(f, "Width"),
            Self::Length => write!(f, "Length"),
            Self::MatchedLengths => write!(f, "MatchedLengths"),
            Self::DaisyChainStubLength => write!(f, "DaisyChainStubLength"),
            Self::PowerPlaneConnectStyle => write!(f, "PowerPlaneConnectStyle"),
            Self::RoutingTopology => write!(f, "RoutingTopology"),
            Self::RoutingPriority => write!(f, "RoutingPriority"),
            Self::RoutingLayers => write!(f, "RoutingLayers"),
            Self::RoutingCornerStyle => write!(f, "RoutingCornerStyle"),
            Self::RoutingViaStyle => write!(f, "RoutingViaStyle"),
            Self::PowerPlaneClearance => write!(f, "PowerPlaneClearance"),
            Self::SolderMaskExpansion => write!(f, "SolderMaskExpansion"),
            Self::PasteMaskExpansion => write!(f, "PasteMaskExpansion"),
            Self::ShortCircuit => write!(f, "ShortCircuit"),
            Self::BrokenNets => write!(f, "BrokenNets"),
            Self::ViasUnderSmd => write!(f, "ViasUnderSmd"),
            Self::MaximumViaCount => write!(f, "MaximumViaCount"),
            Self::MinimumAnnularRing => write!(f, "MinimumAnnularRing"),
            Self::PolygonConnectStyle => write!(f, "PolygonConnectStyle"),
            Self::AcuteAngle => write!(f, "AcuteAngle"),
            Self::ConfinementConstraint => write!(f, "ConfinementConstraint"),
            Self::SmdToCorner => write!(f, "SmdToCorner"),
            Self::ComponentClearance => write!(f, "ComponentClearance"),
            Self::ComponentRotations => write!(f, "ComponentRotations"),
            Self::PermittedLayers => write!(f, "PermittedLayers"),
            Self::NetsToIgnore => write!(f, "NetsToIgnore"),
            Self::SignalStimulus => write!(f, "SignalStimulus"),
            Self::OvershootFallingEdge => write!(f, "OvershootFallingEdge"),
            Self::OvershootRisingEdge => write!(f, "OvershootRisingEdge"),
            Self::UndershootFallingEdge => write!(f, "UndershootFallingEdge"),
            Self::UndershootRisingEdge => write!(f, "UndershootRisingEdge"),
            Self::MaxMinImpedance => write!(f, "MaxMinImpedance"),
            Self::SignalTopValue => write!(f, "SignalTopValue"),
            Self::SignalBaseValue => write!(f, "SignalBaseValue"),
            Self::FlightTimeRisingEdge => write!(f, "FlightTimeRisingEdge"),
            Self::FlightTimeFallingEdge => write!(f, "FlightTimeFallingEdge"),
            Self::LayerStack => write!(f, "LayerStack"),
            Self::MaxSlopeRisingEdge => write!(f, "MaxSlopeRisingEdge"),
            Self::MaxSlopeFallingEdge => write!(f, "MaxSlopeFallingEdge"),
            Self::SupplyNets => write!(f, "SupplyNets"),
            Self::MaxMinHoleSize => write!(f, "MaxMinHoleSize"),
            Self::FabricationTestpointStyle => write!(f, "FabricationTestpointStyle"),
            Self::FabricationTestpointUsage => write!(f, "FabricationTestpointUsage"),
            Self::UnconnectedPin => write!(f, "UnconnectedPin"),
            Self::SmdToPlane => write!(f, "SmdToPlane"),
            Self::SmdNeckDown => write!(f, "SmdNeckDown"),
            Self::LayerPair => write!(f, "LayerPair"),
            Self::FanoutControl => write!(f, "FanoutControl"),
            Self::MaxMinHeight => write!(f, "MaxMinHeight"),
            Self::DifferentialPairsRouting => write!(f, "DifferentialPairsRouting"),
            Self::HoleToHoleClearance => write!(f, "HoleToHoleClearance"),
            Self::MinimumSolderMaskSliver => write!(f, "MinimumSolderMaskSliver"),
            Self::SilkToSolderMaskClearance => write!(f, "SilkToSolderMaskClearance"),
            Self::SilkToSilkClearance => write!(f, "SilkToSilkClearance"),
            Self::NetAntennae => write!(f, "NetAntennae"),
            Self::AssyTestPointStyle => write!(f, "AssyTestPointStyle"),
            Self::AssyTestPointUsage => write!(f, "AssyTestPointUsage"),
            Self::SilkToBoardRegionClearance => write!(f, "SilkToBoardRegionClearance"),
            Self::SmdEntry => write!(f, "SmdEntry"),
            Self::None => write!(f, "None"),
            Self::UnpouredPolygon => write!(f, "UnpouredPolygon"),
            Self::BoardOutlineClearance => write!(f, "BoardOutlineClearance"),
            Self::BackDrilling => write!(f, "BackDrilling"),
            Self::Creepage => write!(f, "Creepage"),
            Self::ReturnPath => write!(f, "ReturnPath"),
            Self::RoutingNeckDown => write!(f, "RoutingNeckDown"),
            Self::WireBonding => write!(f, "WireBonding"),
            Self::ZAxisClearance => write!(f, "ZAxisClearance"),
        }
    }
}

/// Dimension measurement unit (0-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum DimensionUnit {
    #[default]
    Mils = 0,
    Inches = 1,
    Millimeters = 2,
    Centimeters = 3,
    Degrees = 4,
    Radians = 5,
    Automatic = 6,
}

impl TryFrom<u8> for DimensionUnit {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Mils),
            1 => Ok(Self::Inches),
            2 => Ok(Self::Millimeters),
            3 => Ok(Self::Centimeters),
            4 => Ok(Self::Degrees),
            5 => Ok(Self::Radians),
            6 => Ok(Self::Automatic),
            _ => Err(InvalidEnumValue {
                type_name: "DimensionUnit",
                value: v as i64,
            }),
        }
    }
}

impl std::fmt::Display for DimensionUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Dimension text position (0-9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum DimensionTextPosition {
    #[default]
    Auto = 0,
    Center = 1,
    Top = 2,
    Bottom = 3,
    Right = 4,
    Left = 5,
    InsideRight = 6,
    InsideLeft = 7,
    UniDirectional = 8,
    Manual = 9,
}

impl TryFrom<u8> for DimensionTextPosition {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Auto),
            1 => Ok(Self::Center),
            2 => Ok(Self::Top),
            3 => Ok(Self::Bottom),
            4 => Ok(Self::Right),
            5 => Ok(Self::Left),
            6 => Ok(Self::InsideRight),
            7 => Ok(Self::InsideLeft),
            8 => Ok(Self::UniDirectional),
            9 => Ok(Self::Manual),
            _ => Err(InvalidEnumValue {
                type_name: "DimensionTextPosition",
                value: v as i64,
            }),
        }
    }
}

impl std::fmt::Display for DimensionTextPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Dimension arrow position (0-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum DimensionArrowPosition {
    #[default]
    Inside = 0,
    Outside = 1,
}

impl TryFrom<u8> for DimensionArrowPosition {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Inside),
            1 => Ok(Self::Outside),
            _ => Err(InvalidEnumValue {
                type_name: "DimensionArrowPosition",
                value: v as i64,
            }),
        }
    }
}

impl std::fmt::Display for DimensionArrowPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Class member kind (0-8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum ClassMemberKind {
    #[default]
    Net = 0,
    Component = 1,
    FromTo = 2,
    Pad = 3,
    Layer = 4,
    DesignChannel = 5,
    DifferentialPair = 6,
    Polygon = 7,
    SplitPlane = 8,
}

impl TryFrom<u8> for ClassMemberKind {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Net),
            1 => Ok(Self::Component),
            2 => Ok(Self::FromTo),
            3 => Ok(Self::Pad),
            4 => Ok(Self::Layer),
            5 => Ok(Self::DesignChannel),
            6 => Ok(Self::DifferentialPair),
            7 => Ok(Self::Polygon),
            8 => Ok(Self::SplitPlane),
            _ => Err(InvalidEnumValue {
                type_name: "ClassMemberKind",
                value: v as i64,
            }),
        }
    }
}

impl std::fmt::Display for ClassMemberKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Polygon segment type (0-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PolySegmentType {
    #[default]
    Line = 0,
    Arc = 1,
}

impl TryFrom<u8> for PolySegmentType {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Line),
            1 => Ok(Self::Arc),
            _ => Err(InvalidEnumValue {
                type_name: "PolySegmentType",
                value: v as i64,
            }),
        }
    }
}

impl std::fmt::Display for PolySegmentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Polygon repour mode (0-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PolygonRepourMode {
    #[default]
    Never = 0,
    Threshold = 1,
    Always = 2,
}

impl TryFrom<u8> for PolygonRepourMode {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Never),
            1 => Ok(Self::Threshold),
            2 => Ok(Self::Always),
            _ => Err(InvalidEnumValue {
                type_name: "PolygonRepourMode",
                value: v as i64,
            }),
        }
    }
}

impl std::fmt::Display for PolygonRepourMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Polygon thermal relief angle (0-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PolygonReliefAngle {
    #[default]
    Angle45 = 0,
    Angle90 = 1,
}

impl TryFrom<u8> for PolygonReliefAngle {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Angle45),
            1 => Ok(Self::Angle90),
            _ => Err(InvalidEnumValue {
                type_name: "PolygonReliefAngle",
                value: v as i64,
            }),
        }
    }
}

impl std::fmt::Display for PolygonReliefAngle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Edge kind in a TPolySegment record (ShapeBasedRegions6/ShapeBasedComponentBodies6).
///
/// C# enum: TPolySegmentType (byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PolySegmentKind {
    Line = 0,
    Arc = 1,
}

impl TryFrom<u8> for PolySegmentKind {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Line),
            1 => Ok(Self::Arc),
            _ => Err(InvalidEnumValue {
                type_name: "PolySegmentKind",
                value: v as i64,
            }),
        }
    }
}

impl std::fmt::Display for PolySegmentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// PCB coordinate constants.
pub mod constants {
    /// Internal units per mil (1 mil = 10,000 internal units).
    pub const INTERNAL_UNITS: i32 = 10_000;

    /// 1 mil in internal units.
    pub const K1_MIL: i32 = 10_000;

    /// 1 inch in internal units (1 inch = 1000 mils).
    pub const K1_INCH: i32 = 10_000_000;

    /// 1 mm in internal units (nearest integer: 1 mm = 39.3701 mils * 10000).
    pub const K1_MM: i32 = 393_701;

    /// Maximum coordinate value (99999 mils).
    pub const MAX_COORD: i32 = 99999 * 10_000;

    /// Minimum coordinate value.
    pub const MIN_COORD: i32 = 0;

    /// Layer string names indexed by V6 layer byte value (0-82).
    /// Matches cLayerStrings from Altium's Delphi API, reordered to the binary byte mapping.
    pub const LAYER_STRINGS: [&str; 83] = [
        "NoLayer",         // 0
        "TopLayer",        // 1
        "MidLayer1",       // 2
        "MidLayer2",       // 3
        "MidLayer3",       // 4
        "MidLayer4",       // 5
        "MidLayer5",       // 6
        "MidLayer6",       // 7
        "MidLayer7",       // 8
        "MidLayer8",       // 9
        "MidLayer9",       // 10
        "MidLayer10",      // 11
        "MidLayer11",      // 12
        "MidLayer12",      // 13
        "MidLayer13",      // 14
        "MidLayer14",      // 15
        "MidLayer15",      // 16
        "MidLayer16",      // 17
        "MidLayer17",      // 18
        "MidLayer18",      // 19
        "MidLayer19",      // 20
        "MidLayer20",      // 21
        "MidLayer21",      // 22
        "MidLayer22",      // 23
        "MidLayer23",      // 24
        "MidLayer24",      // 25
        "MidLayer25",      // 26
        "MidLayer26",      // 27
        "MidLayer27",      // 28
        "MidLayer28",      // 29
        "MidLayer29",      // 30
        "MidLayer30",      // 31
        "BottomLayer",     // 32
        "TopOverlay",      // 33
        "BottomOverlay",   // 34
        "TopPaste",        // 35
        "BottomPaste",     // 36
        "TopSolder",       // 37
        "BottomSolder",    // 38
        "InternalPlane1",  // 39
        "InternalPlane2",  // 40
        "InternalPlane3",  // 41
        "InternalPlane4",  // 42
        "InternalPlane5",  // 43
        "InternalPlane6",  // 44
        "InternalPlane7",  // 45
        "InternalPlane8",  // 46
        "InternalPlane9",  // 47
        "InternalPlane10", // 48
        "InternalPlane11", // 49
        "InternalPlane12", // 50
        "InternalPlane13", // 51
        "InternalPlane14", // 52
        "InternalPlane15", // 53
        "InternalPlane16", // 54
        "DrillGuide",      // 55
        "KeepOutLayer",    // 56
        "Mechanical1",     // 57
        "Mechanical2",     // 58
        "Mechanical3",     // 59
        "Mechanical4",     // 60
        "Mechanical5",     // 61
        "Mechanical6",     // 62
        "Mechanical7",     // 63
        "Mechanical8",     // 64
        "Mechanical9",     // 65
        "Mechanical10",    // 66
        "Mechanical11",    // 67
        "Mechanical12",    // 68
        "Mechanical13",    // 69
        "Mechanical14",    // 70
        "Mechanical15",    // 71
        "Mechanical16",    // 72
        "DrillDrawing",    // 73
        "MultiLayer",      // 74
        "ConnectLayer",    // 75
        "BackGroundLayer", // 76
        "DRCErrorLayer",   // 77
        "HighlightLayer",  // 78
        "GridColor1",      // 79
        "GridColor10",     // 80
        "PadHoleLayer",    // 81
        "ViaHoleLayer",    // 82
    ];
}

/// IPC-4761 via structure type for via fill/plug/tent classification.
///
/// Corresponds to `TViaStructureType` from `RT_PCB/TViaStructureType.cs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum ViaStructureType {
    #[default]
    None = 0,
    /// Type 1A: Tenting, applied from top.
    Type1ATenting = 1,
    /// Type 1B: Tenting, applied from bottom.
    Type1BTenting = 2,
    /// Type 2A: Tenting and covering, applied from top.
    Type2ATentingAndCovering = 3,
    /// Type 2B: Tenting and covering, applied from bottom.
    Type2BTentingAndCovering = 4,
    /// Type 3A: Plugging, applied from top.
    Type3APlugging = 5,
    /// Type 3B: Plugging, applied from bottom.
    Type3BPlugging = 6,
    /// Type 4A: Plugging and covering, applied from top.
    Type4APluggingAndCovering = 7,
    /// Type 4B: Plugging and covering, applied from bottom.
    Type4BPluggingAndCovering = 8,
    /// Type 5: Filling.
    Type5Filling = 9,
    /// Type 6A: Filling and covering, applied from top.
    Type6AFillingAndCovering = 10,
    /// Type 6B: Filling and covering, applied from bottom.
    Type6BFillingAndCovering = 11,
    /// Type 7: Filling and capping.
    Type7FillingAndCapping = 12,
}

impl TryFrom<u8> for ViaStructureType {
    type Error = crate::InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::None),
            1 => Ok(Self::Type1ATenting),
            2 => Ok(Self::Type1BTenting),
            3 => Ok(Self::Type2ATentingAndCovering),
            4 => Ok(Self::Type2BTentingAndCovering),
            5 => Ok(Self::Type3APlugging),
            6 => Ok(Self::Type3BPlugging),
            7 => Ok(Self::Type4APluggingAndCovering),
            8 => Ok(Self::Type4BPluggingAndCovering),
            9 => Ok(Self::Type5Filling),
            10 => Ok(Self::Type6AFillingAndCovering),
            11 => Ok(Self::Type6BFillingAndCovering),
            12 => Ok(Self::Type7FillingAndCapping),
            _ => Err(crate::InvalidEnumValue {
                type_name: "ViaStructureType",
                value: v as i64,
            }),
        }
    }
}

impl std::fmt::Display for ViaStructureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// Schematic-specific enums and type definitions for Altium Designer schematic files.
//
// All enums are #[non_exhaustive]: unknown discriminant values are parse errors,
// but future Altium versions may add new variants that we add here over time.

use crate::InvalidEnumValue;

/// Schematic record type discriminant (RECORD=N parameter in pipe-delimited text format).
///
/// Maps RECORD=N integer values to their corresponding record types.
/// No catch-all/unknown variant -- an unrecognized RECORD value is a parse error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(i32)]
pub enum SchRecordType {
    Component = 1,
    Pin = 2,
    Symbol = 3,
    Label = 4,
    Bezier = 5,
    Polyline = 6,
    Polygon = 7,
    Ellipse = 8,
    Pie = 9,
    RoundRectangle = 10,
    EllipticalArc = 11,
    Arc = 12,
    Line = 13,
    Rectangle = 14,
    SheetSymbol = 15,
    SheetEntry = 16,
    PowerObject = 17,
    Port = 18,
    SimProbe = 19,
    SimVector = 20,
    SimStimulus = 21,
    NoErc = 22,
    ErrorMarker = 23,
    LayoutDirective = 24,
    NetLabel = 25,
    Bus = 26,
    Wire = 27,
    TextFrame = 28,
    Junction = 29,
    Image = 30,
    Sheet = 31,
    SheetName = 32,
    SheetFileName = 33,
    Designator = 34,
    PartType = 35,
    PartDescription = 36,
    BusEntry = 37,
    SheetPartFileName = 38,
    Template = 39,
    TaskHolder = 40,
    Parameter = 41,
    SchComponent = 42,
    ParameterSet = 43,
    ImplementationList = 44,
    Implementation = 45,
    ImplementationMap = 46,
    MapDefiner = 47,
    ParameterList = 48,
    // --- Harness records (104-138) ---
    HarnessWiringDiagram = 104,
    HarnessLayoutDrawing = 105,
    HarnessComponent = 106,
    HarnessWire = 107,
    HarnessSplice = 108,
    HarnessLayoutLabel = 109,
    HarnessLayoutConnectionPoint = 110,
    HarnessBundle = 111,
    HarnessLogicalSignal = 112,
    HarnessPin = 113,
    HarnessWireLabel = 114,
    HarnessWireData = 115,
    HarnessSpliceData = 116,
    HarnessShield = 117,
    HarnessTwist = 118,
    HarnessNoConnect = 119,
    HarnessNoConnectData = 120,
    HarnessShieldData = 121,
    HarnessTwistData = 122,
    HarnessCable = 123,
    HarnessCableData = 124,
    HarnessAssociatedParts = 125,
    LineView = 126,
    HarnessLibrary = 127,
    HarnessCovering = 128,
    ObjectDefinition = 129,
    HarnessWireBreak = 130,
    AssociatedObjects = 131,
    ElectronicsSystemDesignDocument = 132,
    FunctionalBlock = 133,
    FunctionalConnectionLine = 134,
    FunctionalTextFrame = 135,
    SchematicBlock = 136,
    ReuseSheetSymbol = 137,
    ReuseBlockImplementationInfo = 138,
    // --- Extended records (200+) ---
    SchLib = 200,
    Note = 209,
    Probe = 210,
    CompileMask = 211,
    OpenBusComponent = 212,
    OpenBusLink = 213,
    OpenBusDesignator = 214,
    HarnessConnector = 215,
    HarnessEntry = 216,
    HarnessConnectorType = 217,
    SignalHarness = 218,
    OpenBusPort = 219,
    HighLevelCodeSymbol = 220,
    HighLevelCodeEntry = 221,
    HighLevelCodeName = 222,
    HighLevelCodeFileName = 223,
    OpenBusPinGroup = 224,
    Blanket = 225,
    Hyperlink = 226,
    PinDesignator = 230,
    PinName = 231,
    RichTextDocument = 240,
    RtfLink = 241,
    FSMState = 242,
    FSMTransition = 243,
    FSMNote = 244,
}

impl TryFrom<i32> for SchRecordType {
    type Error = InvalidEnumValue;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Component),
            2 => Ok(Self::Pin),
            3 => Ok(Self::Symbol),
            4 => Ok(Self::Label),
            5 => Ok(Self::Bezier),
            6 => Ok(Self::Polyline),
            7 => Ok(Self::Polygon),
            8 => Ok(Self::Ellipse),
            9 => Ok(Self::Pie),
            10 => Ok(Self::RoundRectangle),
            11 => Ok(Self::EllipticalArc),
            12 => Ok(Self::Arc),
            13 => Ok(Self::Line),
            14 => Ok(Self::Rectangle),
            15 => Ok(Self::SheetSymbol),
            16 => Ok(Self::SheetEntry),
            17 => Ok(Self::PowerObject),
            18 => Ok(Self::Port),
            19 => Ok(Self::SimProbe),
            20 => Ok(Self::SimVector),
            21 => Ok(Self::SimStimulus),
            22 => Ok(Self::NoErc),
            23 => Ok(Self::ErrorMarker),
            24 => Ok(Self::LayoutDirective),
            25 => Ok(Self::NetLabel),
            26 => Ok(Self::Bus),
            27 => Ok(Self::Wire),
            28 => Ok(Self::TextFrame),
            29 => Ok(Self::Junction),
            30 => Ok(Self::Image),
            31 => Ok(Self::Sheet),
            32 => Ok(Self::SheetName),
            33 => Ok(Self::SheetFileName),
            34 => Ok(Self::Designator),
            35 => Ok(Self::PartType),
            36 => Ok(Self::PartDescription),
            37 => Ok(Self::BusEntry),
            38 => Ok(Self::SheetPartFileName),
            39 => Ok(Self::Template),
            40 => Ok(Self::TaskHolder),
            41 => Ok(Self::Parameter),
            42 => Ok(Self::SchComponent),
            43 => Ok(Self::ParameterSet),
            44 => Ok(Self::ImplementationList),
            45 => Ok(Self::Implementation),
            46 => Ok(Self::ImplementationMap),
            47 => Ok(Self::MapDefiner),
            48 => Ok(Self::ParameterList),
            104 => Ok(Self::HarnessWiringDiagram),
            105 => Ok(Self::HarnessLayoutDrawing),
            106 => Ok(Self::HarnessComponent),
            107 => Ok(Self::HarnessWire),
            108 => Ok(Self::HarnessSplice),
            109 => Ok(Self::HarnessLayoutLabel),
            110 => Ok(Self::HarnessLayoutConnectionPoint),
            111 => Ok(Self::HarnessBundle),
            112 => Ok(Self::HarnessLogicalSignal),
            113 => Ok(Self::HarnessPin),
            114 => Ok(Self::HarnessWireLabel),
            115 => Ok(Self::HarnessWireData),
            116 => Ok(Self::HarnessSpliceData),
            117 => Ok(Self::HarnessShield),
            118 => Ok(Self::HarnessTwist),
            119 => Ok(Self::HarnessNoConnect),
            120 => Ok(Self::HarnessNoConnectData),
            121 => Ok(Self::HarnessShieldData),
            122 => Ok(Self::HarnessTwistData),
            123 => Ok(Self::HarnessCable),
            124 => Ok(Self::HarnessCableData),
            125 => Ok(Self::HarnessAssociatedParts),
            126 => Ok(Self::LineView),
            127 => Ok(Self::HarnessLibrary),
            128 => Ok(Self::HarnessCovering),
            129 => Ok(Self::ObjectDefinition),
            130 => Ok(Self::HarnessWireBreak),
            131 => Ok(Self::AssociatedObjects),
            132 => Ok(Self::ElectronicsSystemDesignDocument),
            133 => Ok(Self::FunctionalBlock),
            134 => Ok(Self::FunctionalConnectionLine),
            135 => Ok(Self::FunctionalTextFrame),
            136 => Ok(Self::SchematicBlock),
            137 => Ok(Self::ReuseSheetSymbol),
            138 => Ok(Self::ReuseBlockImplementationInfo),
            200 => Ok(Self::SchLib),
            209 => Ok(Self::Note),
            210 => Ok(Self::Probe),
            211 => Ok(Self::CompileMask),
            212 => Ok(Self::OpenBusComponent),
            213 => Ok(Self::OpenBusLink),
            214 => Ok(Self::OpenBusDesignator),
            215 => Ok(Self::HarnessConnector),
            216 => Ok(Self::HarnessEntry),
            217 => Ok(Self::HarnessConnectorType),
            218 => Ok(Self::SignalHarness),
            219 => Ok(Self::OpenBusPort),
            220 => Ok(Self::HighLevelCodeSymbol),
            221 => Ok(Self::HighLevelCodeEntry),
            222 => Ok(Self::HighLevelCodeName),
            223 => Ok(Self::HighLevelCodeFileName),
            224 => Ok(Self::OpenBusPinGroup),
            225 => Ok(Self::Blanket),
            226 => Ok(Self::Hyperlink),
            230 => Ok(Self::PinDesignator),
            231 => Ok(Self::PinName),
            240 => Ok(Self::RichTextDocument),
            241 => Ok(Self::RtfLink),
            242 => Ok(Self::FSMState),
            243 => Ok(Self::FSMTransition),
            244 => Ok(Self::FSMNote),
            _ => Err(InvalidEnumValue {
                type_name: "SchRecordType",
                value: value as i64,
            }),
        }
    }
}

impl std::fmt::Display for SchRecordType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Component => write!(f, "Component"),
            Self::Pin => write!(f, "Pin"),
            Self::Symbol => write!(f, "Symbol"),
            Self::Label => write!(f, "Label"),
            Self::Bezier => write!(f, "Bezier"),
            Self::Polyline => write!(f, "Polyline"),
            Self::Polygon => write!(f, "Polygon"),
            Self::Ellipse => write!(f, "Ellipse"),
            Self::Pie => write!(f, "Pie"),
            Self::RoundRectangle => write!(f, "RoundRectangle"),
            Self::EllipticalArc => write!(f, "EllipticalArc"),
            Self::Arc => write!(f, "Arc"),
            Self::Line => write!(f, "Line"),
            Self::Rectangle => write!(f, "Rectangle"),
            Self::SheetSymbol => write!(f, "SheetSymbol"),
            Self::SheetEntry => write!(f, "SheetEntry"),
            Self::PowerObject => write!(f, "PowerObject"),
            Self::Port => write!(f, "Port"),
            Self::SimProbe => write!(f, "SimProbe"),
            Self::SimVector => write!(f, "SimVector"),
            Self::SimStimulus => write!(f, "SimStimulus"),
            Self::NoErc => write!(f, "NoERC"),
            Self::ErrorMarker => write!(f, "ErrorMarker"),
            Self::LayoutDirective => write!(f, "LayoutDirective"),
            Self::NetLabel => write!(f, "NetLabel"),
            Self::Bus => write!(f, "Bus"),
            Self::Wire => write!(f, "Wire"),
            Self::TextFrame => write!(f, "TextFrame"),
            Self::Junction => write!(f, "Junction"),
            Self::Image => write!(f, "Image"),
            Self::Sheet => write!(f, "Sheet"),
            Self::SheetName => write!(f, "SheetName"),
            Self::SheetFileName => write!(f, "SheetFileName"),
            Self::Designator => write!(f, "Designator"),
            Self::PartType => write!(f, "PartType"),
            Self::PartDescription => write!(f, "PartDescription"),
            Self::BusEntry => write!(f, "BusEntry"),
            Self::SheetPartFileName => write!(f, "SheetPartFileName"),
            Self::Template => write!(f, "Template"),
            Self::TaskHolder => write!(f, "TaskHolder"),
            Self::Parameter => write!(f, "Parameter"),
            Self::SchComponent => write!(f, "SchComponent"),
            Self::ParameterSet => write!(f, "ParameterSet"),
            Self::ImplementationList => write!(f, "ImplementationList"),
            Self::Implementation => write!(f, "Implementation"),
            Self::ImplementationMap => write!(f, "ImplementationMap"),
            Self::MapDefiner => write!(f, "MapDefiner"),
            Self::ParameterList => write!(f, "ParameterList"),
            Self::HarnessWiringDiagram => write!(f, "HarnessWiringDiagram"),
            Self::HarnessLayoutDrawing => write!(f, "HarnessLayoutDrawing"),
            Self::HarnessComponent => write!(f, "HarnessComponent"),
            Self::HarnessWire => write!(f, "HarnessWire"),
            Self::HarnessSplice => write!(f, "HarnessSplice"),
            Self::HarnessLayoutLabel => write!(f, "HarnessLayoutLabel"),
            Self::HarnessLayoutConnectionPoint => write!(f, "HarnessLayoutConnectionPoint"),
            Self::HarnessBundle => write!(f, "HarnessBundle"),
            Self::HarnessLogicalSignal => write!(f, "HarnessLogicalSignal"),
            Self::HarnessPin => write!(f, "HarnessPin"),
            Self::HarnessWireLabel => write!(f, "HarnessWireLabel"),
            Self::HarnessWireData => write!(f, "HarnessWireData"),
            Self::HarnessSpliceData => write!(f, "HarnessSpliceData"),
            Self::HarnessShield => write!(f, "HarnessShield"),
            Self::HarnessTwist => write!(f, "HarnessTwist"),
            Self::HarnessNoConnect => write!(f, "HarnessNoConnect"),
            Self::HarnessNoConnectData => write!(f, "HarnessNoConnectData"),
            Self::HarnessShieldData => write!(f, "HarnessShieldData"),
            Self::HarnessTwistData => write!(f, "HarnessTwistData"),
            Self::HarnessCable => write!(f, "HarnessCable"),
            Self::HarnessCableData => write!(f, "HarnessCableData"),
            Self::HarnessAssociatedParts => write!(f, "HarnessAssociatedParts"),
            Self::LineView => write!(f, "LineView"),
            Self::HarnessLibrary => write!(f, "HarnessLibrary"),
            Self::HarnessCovering => write!(f, "HarnessCovering"),
            Self::ObjectDefinition => write!(f, "ObjectDefinition"),
            Self::HarnessWireBreak => write!(f, "HarnessWireBreak"),
            Self::AssociatedObjects => write!(f, "AssociatedObjects"),
            Self::ElectronicsSystemDesignDocument => write!(f, "ElectronicsSystemDesignDocument"),
            Self::FunctionalBlock => write!(f, "FunctionalBlock"),
            Self::FunctionalConnectionLine => write!(f, "FunctionalConnectionLine"),
            Self::FunctionalTextFrame => write!(f, "FunctionalTextFrame"),
            Self::SchematicBlock => write!(f, "SchematicBlock"),
            Self::ReuseSheetSymbol => write!(f, "ReuseSheetSymbol"),
            Self::ReuseBlockImplementationInfo => write!(f, "ReuseBlockImplementationInfo"),
            Self::SchLib => write!(f, "SchLib"),
            Self::Note => write!(f, "Note"),
            Self::Probe => write!(f, "Probe"),
            Self::CompileMask => write!(f, "CompileMask"),
            Self::OpenBusComponent => write!(f, "OpenBusComponent"),
            Self::OpenBusLink => write!(f, "OpenBusLink"),
            Self::OpenBusDesignator => write!(f, "OpenBusDesignator"),
            Self::HarnessConnector => write!(f, "HarnessConnector"),
            Self::HarnessEntry => write!(f, "HarnessEntry"),
            Self::HarnessConnectorType => write!(f, "HarnessConnectorType"),
            Self::SignalHarness => write!(f, "SignalHarness"),
            Self::OpenBusPort => write!(f, "OpenBusPort"),
            Self::HighLevelCodeSymbol => write!(f, "HighLevelCodeSymbol"),
            Self::HighLevelCodeEntry => write!(f, "HighLevelCodeEntry"),
            Self::HighLevelCodeName => write!(f, "HighLevelCodeName"),
            Self::HighLevelCodeFileName => write!(f, "HighLevelCodeFileName"),
            Self::OpenBusPinGroup => write!(f, "OpenBusPinGroup"),
            Self::Blanket => write!(f, "Blanket"),
            Self::Hyperlink => write!(f, "Hyperlink"),
            Self::PinDesignator => write!(f, "PinDesignator"),
            Self::PinName => write!(f, "PinName"),
            Self::RichTextDocument => write!(f, "RichTextDocument"),
            Self::RtfLink => write!(f, "RtfLink"),
            Self::FSMState => write!(f, "FSMState"),
            Self::FSMTransition => write!(f, "FSMTransition"),
            Self::FSMNote => write!(f, "FSMNote"),
        }
    }
}

/// Font definition from SchSheet RECORD=31.
#[derive(Debug, Clone)]
pub struct SchFont {
    pub id: i32,      // 1-based index
    pub name: String, // e.g., "Times New Roman"
    pub size: i32,    // point size
    pub rotation: i32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikeout: bool,
}

/// Indexed display style entry from SchLib FileHeader style table.
#[derive(Debug, Clone, Default)]
pub struct SchDisplayStyle {
    pub id: i32, // 1-based index
    pub gradient_depth: Option<i32>,
    pub shadow_opacity: Option<i32>,
    pub shadow_distance: Option<crate::Coord>,
    pub shadow_blur: Option<crate::Coord>,
    pub shadow_angle_in_degrees: Option<i32>,
    pub glow_color: Option<crate::Color>,
    pub glow_opacity: Option<i32>,
    pub glow_size: Option<i32>,
    pub reflection_depth: Option<i32>,
    pub reflection_opacity: Option<i32>,
    pub transparency_enabled: Option<bool>,
    pub transparency_amount: Option<i32>,
    pub corner_radius_mode: Option<i32>,
    pub corner_radius_value: Option<i32>,
}

/// Sheet display settings from SchLib FileHeader and SchDoc Sheet (RECORD=31).
///
/// These settings control grid configuration, sheet sizing, border/reference zone
/// display, and editor preferences. All fields are optional because any may be
/// absent from a given file (Altium uses built-in defaults).
///
/// Coord fields combine the integer part and `_Frac` companion into a single
/// value: `raw = integer * 100_000 + frac`.
#[derive(Debug, Clone, Default)]
pub struct SchDisplaySettings {
    // Grid settings
    pub snap_grid_on: Option<bool>,
    pub snap_grid_size: Option<crate::Coord>,
    pub visible_grid_on: Option<bool>,
    pub visible_grid_size: Option<crate::Coord>,
    pub hot_spot_grid_on: Option<bool>,
    pub hot_spot_grid_size: Option<crate::Coord>,

    // Sheet size
    pub sheet_style: Option<SheetStyle>,
    pub use_custom_sheet: Option<bool>,
    pub custom_x: Option<crate::Coord>,
    pub custom_y: Option<crate::Coord>,

    // Border and title block
    pub border_on: Option<bool>,
    pub title_block_on: Option<bool>,
    pub document_border_style: Option<SheetBorderStyle>,
    pub reference_zones_on: Option<bool>,
    pub reference_zone_style: Option<SheetReferenceZoneStyle>,
    pub custom_x_zones: Option<i32>,
    pub custom_y_zones: Option<i32>,
    pub custom_margin_width: Option<crate::Coord>,
    pub sheet_number_space_size: Option<i32>,

    // Display options
    pub workspace_orientation: Option<SheetOrientation>,
    pub show_hidden_pins: Option<bool>,
    pub show_template_graphics: Option<bool>,
    pub always_show_cd: Option<bool>,

    // Template
    pub template_file_name: Option<String>,

    // Document settings
    pub display_unit: Option<i32>,
    pub system_font: Option<i32>,
    pub use_mbcs: Option<bool>,
    pub is_boc: Option<bool>,

    // Colors
    pub area_color: Option<crate::Color>,
    pub styles: Vec<SchDisplayStyle>,

    // Version
    pub file_version_info: Option<String>,
}

/// Pin electrical type (0-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PinElectricalType {
    Input = 0,
    InputOutput = 1,
    Output = 2,
    OpenCollector = 3,
    #[default]
    Passive = 4,
    HiZ = 5,
    OpenEmitter = 6,
    Power = 7,
}

impl TryFrom<u8> for PinElectricalType {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Input),
            1 => Ok(Self::InputOutput),
            2 => Ok(Self::Output),
            3 => Ok(Self::OpenCollector),
            4 => Ok(Self::Passive),
            5 => Ok(Self::HiZ),
            6 => Ok(Self::OpenEmitter),
            7 => Ok(Self::Power),
            _ => Err(InvalidEnumValue {
                type_name: "PinElectricalType",
                value: value as i64,
            }),
        }
    }
}

impl std::fmt::Display for PinElectricalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// IEEE pin symbol types (0-36).
pub type PinSymbol = IeeeSymbol;

/// IEEE pin symbol types (0-36).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum IeeeSymbol {
    #[default]
    NoSymbol = 0,
    Dot = 1,
    RightLeftSignalFlow = 2,
    Clock = 3,
    ActiveLowInput = 4,
    AnalogSignalIn = 5,
    NotLogicConnection = 6,
    ShiftRight = 7,
    PostponedOutput = 8,
    OpenCollector = 9,
    HiZ = 10,
    HighCurrent = 11,
    Pulse = 12,
    Schmitt = 13,
    Delay = 14,
    GroupLine = 15,
    GroupBin = 16,
    ActiveLowOutput = 17,
    PiSymbol = 18,
    GreaterEqual = 19,
    LessEqual = 20,
    Sigma = 21,
    OpenCollectorPullUp = 22,
    OpenEmitter = 23,
    OpenEmitterPullUp = 24,
    DigitalSignalIn = 25,
    And = 26,
    Invertor = 27,
    Or = 28,
    Xor = 29,
    ShiftLeft = 30,
    InputOutput = 31,
    OpenCircuitOutput = 32,
    LeftRightSignalFlow = 33,
    BidirectionalSignalFlow = 34,
    InternalPullUp = 35,
    InternalPullDown = 36,
}

impl TryFrom<u8> for IeeeSymbol {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::NoSymbol),
            1 => Ok(Self::Dot),
            2 => Ok(Self::RightLeftSignalFlow),
            3 => Ok(Self::Clock),
            4 => Ok(Self::ActiveLowInput),
            5 => Ok(Self::AnalogSignalIn),
            6 => Ok(Self::NotLogicConnection),
            7 => Ok(Self::ShiftRight),
            8 => Ok(Self::PostponedOutput),
            9 => Ok(Self::OpenCollector),
            10 => Ok(Self::HiZ),
            11 => Ok(Self::HighCurrent),
            12 => Ok(Self::Pulse),
            13 => Ok(Self::Schmitt),
            14 => Ok(Self::Delay),
            15 => Ok(Self::GroupLine),
            16 => Ok(Self::GroupBin),
            17 => Ok(Self::ActiveLowOutput),
            18 => Ok(Self::PiSymbol),
            19 => Ok(Self::GreaterEqual),
            20 => Ok(Self::LessEqual),
            21 => Ok(Self::Sigma),
            22 => Ok(Self::OpenCollectorPullUp),
            23 => Ok(Self::OpenEmitter),
            24 => Ok(Self::OpenEmitterPullUp),
            25 => Ok(Self::DigitalSignalIn),
            26 => Ok(Self::And),
            27 => Ok(Self::Invertor),
            28 => Ok(Self::Or),
            29 => Ok(Self::Xor),
            30 => Ok(Self::ShiftLeft),
            31 => Ok(Self::InputOutput),
            32 => Ok(Self::OpenCircuitOutput),
            33 => Ok(Self::LeftRightSignalFlow),
            34 => Ok(Self::BidirectionalSignalFlow),
            35 => Ok(Self::InternalPullUp),
            36 => Ok(Self::InternalPullDown),
            _ => Err(InvalidEnumValue {
                type_name: "IeeeSymbol",
                value: value as i64,
            }),
        }
    }
}

/// VHDL formal type / std_logic state (0-8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum StdLogicState {
    #[default]
    Uninitialized = 0,
    ForcingUnknown = 1,
    Forcing0 = 2,
    Forcing1 = 3,
    HighZ = 4,
    WeakUnknown = 5,
    Weak0 = 6,
    Weak1 = 7,
    DontCare = 8,
}

impl TryFrom<u8> for StdLogicState {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Uninitialized),
            1 => Ok(Self::ForcingUnknown),
            2 => Ok(Self::Forcing0),
            3 => Ok(Self::Forcing1),
            4 => Ok(Self::HighZ),
            5 => Ok(Self::WeakUnknown),
            6 => Ok(Self::Weak0),
            7 => Ok(Self::Weak1),
            8 => Ok(Self::DontCare),
            _ => Err(InvalidEnumValue {
                type_name: "StdLogicState",
                value: value as i64,
            }),
        }
    }
}

/// Pen/border width (0-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PenWidth {
    #[default]
    Zero = 0,
    Small = 1,
    Medium = 2,
    Large = 3,
}

impl TryFrom<u8> for PenWidth {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::Small),
            2 => Ok(Self::Medium),
            3 => Ok(Self::Large),
            _ => Err(InvalidEnumValue {
                type_name: "PenWidth",
                value: value as i64,
            }),
        }
    }
}

/// Line style (0-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum LineStyle {
    #[default]
    Solid = 0,
    Dashed = 1,
    Dotted = 2,
    DashDotted = 3,
}

impl TryFrom<u8> for LineStyle {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Solid),
            1 => Ok(Self::Dashed),
            2 => Ok(Self::Dotted),
            3 => Ok(Self::DashDotted),
            _ => Err(InvalidEnumValue {
                type_name: "LineStyle",
                value: value as i64,
            }),
        }
    }
}

/// Line endpoint shape (0-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum LineShape {
    #[default]
    None = 0,
    Arrow = 1,
    SolidArrow = 2,
    Tail = 3,
    SolidTail = 4,
    Circle = 5,
    Square = 6,
}

impl TryFrom<u8> for LineShape {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Arrow),
            2 => Ok(Self::SolidArrow),
            3 => Ok(Self::Tail),
            4 => Ok(Self::SolidTail),
            5 => Ok(Self::Circle),
            6 => Ok(Self::Square),
            _ => Err(InvalidEnumValue {
                type_name: "LineShape",
                value: value as i64,
            }),
        }
    }
}

/// Text justification (0-8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum TextJustification {
    #[default]
    BottomLeft = 0,
    BottomCenter = 1,
    BottomRight = 2,
    CenterLeft = 3,
    Center = 4,
    CenterRight = 5,
    TopLeft = 6,
    TopCenter = 7,
    TopRight = 8,
}

impl TryFrom<u8> for TextJustification {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::BottomLeft),
            1 => Ok(Self::BottomCenter),
            2 => Ok(Self::BottomRight),
            3 => Ok(Self::CenterLeft),
            4 => Ok(Self::Center),
            5 => Ok(Self::CenterRight),
            6 => Ok(Self::TopLeft),
            7 => Ok(Self::TopCenter),
            8 => Ok(Self::TopRight),
            _ => Err(InvalidEnumValue {
                type_name: "TextJustification",
                value: value as i64,
            }),
        }
    }
}

/// Power object visual style (0-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PowerObjectStyle {
    #[default]
    Circle = 0,
    Arrow = 1,
    Bar = 2,
    Wave = 3,
    GndPower = 4,
    GndSignal = 5,
    GndEarth = 6,
    GostArrow = 7,
    GostGndPower = 8,
    GostGndEarth = 9,
    GostBar = 10,
}

impl TryFrom<u8> for PowerObjectStyle {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Circle),
            1 => Ok(Self::Arrow),
            2 => Ok(Self::Bar),
            3 => Ok(Self::Wave),
            4 => Ok(Self::GndPower),
            5 => Ok(Self::GndSignal),
            6 => Ok(Self::GndEarth),
            7 => Ok(Self::GostArrow),
            8 => Ok(Self::GostGndPower),
            9 => Ok(Self::GostGndEarth),
            10 => Ok(Self::GostBar),
            _ => Err(InvalidEnumValue {
                type_name: "PowerObjectStyle",
                value: value as i64,
            }),
        }
    }
}

/// Port arrow direction style (0-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PortArrowStyle {
    #[default]
    None = 0,
    Left = 1,
    Right = 2,
    LeftRight = 3,
    NoneVertical = 4,
    Top = 5,
    Bottom = 6,
    TopBottom = 7,
}

impl TryFrom<u8> for PortArrowStyle {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Left),
            2 => Ok(Self::Right),
            3 => Ok(Self::LeftRight),
            4 => Ok(Self::NoneVertical),
            5 => Ok(Self::Top),
            6 => Ok(Self::Bottom),
            7 => Ok(Self::TopBottom),
            _ => Err(InvalidEnumValue {
                type_name: "PortArrowStyle",
                value: value as i64,
            }),
        }
    }
}

/// Port I/O direction (0-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PortIoType {
    #[default]
    Unspecified = 0,
    Output = 1,
    Input = 2,
    Bidirectional = 3,
}

impl TryFrom<u8> for PortIoType {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Unspecified),
            1 => Ok(Self::Output),
            2 => Ok(Self::Input),
            3 => Ok(Self::Bidirectional),
            _ => Err(InvalidEnumValue {
                type_name: "PortIoType",
                value: value as i64,
            }),
        }
    }
}

/// Sheet size standard (0-17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum SheetStyle {
    #[default]
    A4 = 0,
    A3 = 1,
    A2 = 2,
    A1 = 3,
    A0 = 4,
    A = 5,
    B = 6,
    C = 7,
    D = 8,
    E = 9,
    Letter = 10,
    Legal = 11,
    Tabloid = 12,
    OrcadA = 13,
    OrcadB = 14,
    OrcadC = 15,
    OrcadD = 16,
    OrcadE = 17,
}

impl TryFrom<u8> for SheetStyle {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::A4),
            1 => Ok(Self::A3),
            2 => Ok(Self::A2),
            3 => Ok(Self::A1),
            4 => Ok(Self::A0),
            5 => Ok(Self::A),
            6 => Ok(Self::B),
            7 => Ok(Self::C),
            8 => Ok(Self::D),
            9 => Ok(Self::E),
            10 => Ok(Self::Letter),
            11 => Ok(Self::Legal),
            12 => Ok(Self::Tabloid),
            13 => Ok(Self::OrcadA),
            14 => Ok(Self::OrcadB),
            15 => Ok(Self::OrcadC),
            16 => Ok(Self::OrcadD),
            17 => Ok(Self::OrcadE),
            _ => Err(InvalidEnumValue {
                type_name: "SheetStyle",
                value: value as i64,
            }),
        }
    }
}

/// Sheet orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum SheetOrientation {
    #[default]
    Landscape = 0,
    Portrait = 1,
}

impl TryFrom<u8> for SheetOrientation {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Landscape),
            1 => Ok(Self::Portrait),
            _ => Err(InvalidEnumValue {
                type_name: "SheetOrientation",
                value: value as i64,
            }),
        }
    }
}

/// Sheet border style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum SheetBorderStyle {
    #[default]
    Standard = 0,
    Ansi = 1,
}

impl TryFrom<u8> for SheetBorderStyle {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Standard),
            1 => Ok(Self::Ansi),
            _ => Err(InvalidEnumValue {
                type_name: "SheetBorderStyle",
                value: value as i64,
            }),
        }
    }
}

/// Text horizontal anchor mode.
///
/// **Wire type:** u8
/// **Used by:** Text (RECORD=17), TextFrame (RECORD=28)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum TextHorzAnchor {
    #[default]
    None = 0,
    Both = 1,
    Left = 2,
    Right = 3,
}

impl TryFrom<u8> for TextHorzAnchor {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Both),
            2 => Ok(Self::Left),
            3 => Ok(Self::Right),
            _ => Err(InvalidEnumValue {
                type_name: "TextHorzAnchor",
                value: value as i64,
            }),
        }
    }
}

/// Text vertical anchor mode.
///
/// **Wire type:** u8
/// **Used by:** Text (RECORD=17), TextFrame (RECORD=28)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum TextVertAnchor {
    #[default]
    None = 0,
    Both = 1,
    Top = 2,
    Bottom = 3,
}

impl TryFrom<u8> for TextVertAnchor {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Both),
            2 => Ok(Self::Top),
            3 => Ok(Self::Bottom),
            _ => Err(InvalidEnumValue {
                type_name: "TextVertAnchor",
                value: value as i64,
            }),
        }
    }
}

/// Parameter read-only state controlling which parts are editable.
///
/// **Wire type:** u8
/// **Used by:** Parameter (RECORD=41)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum ParameterReadOnlyState {
    #[default]
    None = 0,
    Name = 1,
    Value = 2,
    NameAndValue = 3,
}

impl TryFrom<u8> for ParameterReadOnlyState {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Name),
            2 => Ok(Self::Value),
            3 => Ok(Self::NameAndValue),
            _ => Err(InvalidEnumValue {
                type_name: "ParameterReadOnlyState",
                value: value as i64,
            }),
        }
    }
}

/// Parameter value type discriminator.
///
/// **Wire type:** u8
/// **Used by:** Parameter (RECORD=41)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum ParameterType {
    #[default]
    String = 0,
    Boolean = 1,
    Integer = 2,
    Float = 3,
}

impl TryFrom<u8> for ParameterType {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::String),
            1 => Ok(Self::Boolean),
            2 => Ok(Self::Integer),
            3 => Ok(Self::Float),
            _ => Err(InvalidEnumValue {
                type_name: "ParameterType",
                value: value as i64,
            }),
        }
    }
}

/// Parameter set visual style.
///
/// **Wire type:** u8
/// **Used by:** ParameterSet (RECORD=43)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum ParameterSetStyle {
    #[default]
    Large = 0,
    Tiny = 1,
}

impl TryFrom<u8> for ParameterSetStyle {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Large),
            1 => Ok(Self::Tiny),
            _ => Err(InvalidEnumValue {
                type_name: "ParameterSetStyle",
                value: value as i64,
            }),
        }
    }
}

/// No-ERC marker visual style.
///
/// **Wire type:** u8
/// **Used by:** NoERC (RECORD=22)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum NoErcSymbol {
    #[default]
    CrossThin = 0,
    CrossThick = 1,
    CrossSmall = 2,
    CheckBox = 3,
    Triangle = 4,
}

impl TryFrom<u8> for NoErcSymbol {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::CrossThin),
            1 => Ok(Self::CrossThick),
            2 => Ok(Self::CrossSmall),
            3 => Ok(Self::CheckBox),
            4 => Ok(Self::Triangle),
            _ => Err(InvalidEnumValue {
                type_name: "NoErcSymbol",
                value: value as i64,
            }),
        }
    }
}

/// Object side / edge placement.
///
/// **Wire type:** u8
/// **Used by:** SheetEntry (RECORD=16), HarnessConnector (RECORD=215)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum LeftRightSide {
    #[default]
    Left = 0,
    Right = 1,
    Top = 2,
    Bottom = 3,
}

impl TryFrom<u8> for LeftRightSide {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Left),
            1 => Ok(Self::Right),
            2 => Ok(Self::Top),
            3 => Ok(Self::Bottom),
            _ => Err(InvalidEnumValue {
                type_name: "LeftRightSide",
                value: value as i64,
            }),
        }
    }
}

/// Sheet symbol sub-type.
///
/// **Wire type:** u8
/// **Used by:** SheetSymbol (RECORD=15)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum SheetSymbolType {
    #[default]
    Normal = 0,
    DeviceSheet = 1,
    DesignItem = 2,
}

impl TryFrom<u8> for SheetSymbolType {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Normal),
            1 => Ok(Self::DeviceSheet),
            2 => Ok(Self::DesignItem),
            _ => Err(InvalidEnumValue {
                type_name: "SheetSymbolType",
                value: value as i64,
            }),
        }
    }
}

/// Visible grid rendering style.
///
/// **Wire type:** u8
/// **Used by:** Sheet (RECORD=31)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum VisibleGridStyle {
    #[default]
    DotGrid = 0,
    LineGrid = 1,
}

impl TryFrom<u8> for VisibleGridStyle {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::DotGrid),
            1 => Ok(Self::LineGrid),
            _ => Err(InvalidEnumValue {
                type_name: "VisibleGridStyle",
                value: value as i64,
            }),
        }
    }
}

/// Sheet border reference zone label style.
///
/// **Wire type:** u8
/// **Used by:** Sheet (RECORD=31)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum SheetReferenceZoneStyle {
    #[default]
    Default = 0,
    Asme = 1,
}

impl TryFrom<u8> for SheetReferenceZoneStyle {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Default),
            1 => Ok(Self::Asme),
            _ => Err(InvalidEnumValue {
                type_name: "SheetReferenceZoneStyle",
                value: value as i64,
            }),
        }
    }
}

/// Harness connector visual kind.
///
/// **Wire type:** u8
/// **Used by:** HarnessConnector (RECORD=215)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum ConnectorKind {
    #[default]
    Poly = 0,
    Arrow = 1,
    Round = 2,
}

impl TryFrom<u8> for ConnectorKind {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Poly),
            1 => Ok(Self::Arrow),
            2 => Ok(Self::Round),
            _ => Err(InvalidEnumValue {
                type_name: "ConnectorKind",
                value: value as i64,
            }),
        }
    }
}

/// Harness connector gender/state.
///
/// **Wire type:** u8
/// **Used by:** HarnessConnector (RECORD=215)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum ConnectorState {
    #[default]
    Unknown = 0,
    Male = 1,
    Female = 2,
}

impl TryFrom<u8> for ConnectorState {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Unknown),
            1 => Ok(Self::Male),
            2 => Ok(Self::Female),
            _ => Err(InvalidEnumValue {
                type_name: "ConnectorState",
                value: value as i64,
            }),
        }
    }
}

/// Harness covering visual fill pattern.
///
/// **Wire type:** u8
/// **Used by:** HarnessCovering (RECORD=128)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum HarnessBrush {
    #[default]
    None = 0,
    BlackWeave = 1,
    YellowWeave = 2,
    RedWeave = 3,
}

impl TryFrom<u8> for HarnessBrush {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::BlackWeave),
            2 => Ok(Self::YellowWeave),
            3 => Ok(Self::RedWeave),
            _ => Err(InvalidEnumValue {
                type_name: "HarnessBrush",
                value: value as i64,
            }),
        }
    }
}

/// Harness document length measurement unit.
///
/// **Wire type:** u8
/// **Used by:** HarnessDocument
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum HarnessLengthUnit {
    #[default]
    Millimeter = 0,
    Centimeter = 1,
    Meter = 2,
    Inch = 3,
    Foot = 4,
}

impl TryFrom<u8> for HarnessLengthUnit {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Millimeter),
            1 => Ok(Self::Centimeter),
            2 => Ok(Self::Meter),
            3 => Ok(Self::Inch),
            4 => Ok(Self::Foot),
            _ => Err(InvalidEnumValue {
                type_name: "HarnessLengthUnit",
                value: value as i64,
            }),
        }
    }
}

/// How a harness wire/bundle length was determined.
///
/// **Wire type:** u8
/// **Used by:** HarnessBundleSubLineData
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum HarnessWireLengthType {
    #[default]
    Calculated = 0,
    UserDefined = 1,
    McadCoDesigner = 2,
}

impl TryFrom<u8> for HarnessWireLengthType {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Calculated),
            1 => Ok(Self::UserDefined),
            2 => Ok(Self::McadCoDesigner),
            _ => Err(InvalidEnumValue {
                type_name: "HarnessWireLengthType",
                value: value as i64,
            }),
        }
    }
}

/// Harness cavity associated part type.
///
/// **Wire type:** u8
/// **Used by:** AssociatedObjects (RECORD=131)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum HarnessCavityPartType {
    #[default]
    Crimp = 0,
    Seal = 1,
    Plug = 2,
    Other = 3,
}

impl TryFrom<u8> for HarnessCavityPartType {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Crimp),
            1 => Ok(Self::Seal),
            2 => Ok(Self::Plug),
            3 => Ok(Self::Other),
            _ => Err(InvalidEnumValue {
                type_name: "HarnessCavityPartType",
                value: value as i64,
            }),
        }
    }
}

/// Harness layout connection point visual style.
///
/// **Wire type:** u8
/// **Used by:** HarnessLayoutConnectionPoint (RECORD=110)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum HarnessConnectionPointStyle {
    #[default]
    Circle = 0,
    Square = 1,
    Insulator = 2,
}

impl TryFrom<u8> for HarnessConnectionPointStyle {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Circle),
            1 => Ok(Self::Square),
            2 => Ok(Self::Insulator),
            _ => Err(InvalidEnumValue {
                type_name: "HarnessConnectionPointStyle",
                value: value as i64,
            }),
        }
    }
}

/// Harness shield visual style.
///
/// **Wire type:** u8
/// **Used by:** HarnessShield (RECORD=117)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum HarnessShieldStyle {
    #[default]
    Shield = 0,
    ShieldWithConnection = 1,
}

impl TryFrom<u8> for HarnessShieldStyle {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Shield),
            1 => Ok(Self::ShieldWithConnection),
            _ => Err(InvalidEnumValue {
                type_name: "HarnessShieldStyle",
                value: value as i64,
            }),
        }
    }
}

/// Harness splice visual style.
///
/// **Wire type:** u8
/// **Used by:** HarnessSplice (RECORD=108)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum HarnessSpliceStyle {
    #[default]
    Circle = 0,
    Inline = 1,
}

impl TryFrom<u8> for HarnessSpliceStyle {
    type Error = InvalidEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Circle),
            1 => Ok(Self::Inline),
            _ => Err(InvalidEnumValue {
                type_name: "HarnessSpliceStyle",
                value: value as i64,
            }),
        }
    }
}

// RotationBy90 is defined in crate::common (shared across PCB and schematic).

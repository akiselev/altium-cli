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
    NoErc = 22,
    ErrorMarker = 23,
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
    BusEntry = 37,
    Template = 39,
    TaskHolder = 40,
    Parameter = 41,
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
    HarnessConnector = 215,
    HarnessEntry = 216,
    HarnessConnectorType = 217,
    SignalHarness = 218,
    HighLevelCodeSymbol = 220,
    HighLevelCodeEntry = 221,
    Blanket = 225,
    Hyperlink = 226,
    RichTextDocument = 240,
    RtfLink = 241,
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
            22 => Ok(Self::NoErc),
            23 => Ok(Self::ErrorMarker),
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
            37 => Ok(Self::BusEntry),
            39 => Ok(Self::Template),
            40 => Ok(Self::TaskHolder),
            41 => Ok(Self::Parameter),
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
            215 => Ok(Self::HarnessConnector),
            216 => Ok(Self::HarnessEntry),
            217 => Ok(Self::HarnessConnectorType),
            218 => Ok(Self::SignalHarness),
            220 => Ok(Self::HighLevelCodeSymbol),
            221 => Ok(Self::HighLevelCodeEntry),
            225 => Ok(Self::Blanket),
            226 => Ok(Self::Hyperlink),
            240 => Ok(Self::RichTextDocument),
            241 => Ok(Self::RtfLink),
            _ => Err(InvalidEnumValue { type_name: "SchRecordType", value: value as i64 }),
        }
    }
}

impl std::fmt::Display for SchRecordType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
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
            _ => Err(InvalidEnumValue { type_name: "PinElectricalType", value: value as i64 }),
        }
    }
}

impl std::fmt::Display for PinElectricalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

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
            _ => Err(InvalidEnumValue { type_name: "IeeeSymbol", value: value as i64 }),
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
            _ => Err(InvalidEnumValue { type_name: "StdLogicState", value: value as i64 }),
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
            _ => Err(InvalidEnumValue { type_name: "PenWidth", value: value as i64 }),
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
            _ => Err(InvalidEnumValue { type_name: "LineStyle", value: value as i64 }),
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
            _ => Err(InvalidEnumValue { type_name: "LineShape", value: value as i64 }),
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
            _ => Err(InvalidEnumValue { type_name: "TextJustification", value: value as i64 }),
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
            _ => Err(InvalidEnumValue { type_name: "PowerObjectStyle", value: value as i64 }),
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
            _ => Err(InvalidEnumValue { type_name: "PortArrowStyle", value: value as i64 }),
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
            _ => Err(InvalidEnumValue { type_name: "PortIoType", value: value as i64 }),
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
            _ => Err(InvalidEnumValue { type_name: "SheetStyle", value: value as i64 }),
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
            _ => Err(InvalidEnumValue { type_name: "SheetOrientation", value: value as i64 }),
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
            _ => Err(InvalidEnumValue { type_name: "SheetBorderStyle", value: value as i64 }),
        }
    }
}

// RotationBy90 is defined in crate::common (shared across PCB and schematic).

//! Shared enums ported from decompiled Altium C#.
//!
//! Each enum maps 1:1 to its C# counterpart with discriminant values
//! matching the serialized integer representation.

/// Record/object type identifier — `TObjectId` from C#.
///
/// 121 values (0..=120), matching the enum in `Altium.Edp.Interfaces/TObjectId.cs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ObjectId {
    FirstObjectId = 0,
    ClipBoardContainer = 1,
    Note = 2,
    Probe = 3,
    Rectangle = 4,
    Line = 5,
    ConnectionLine = 6,
    BusEntry = 7,
    Arc = 8,
    EllipticalArc = 9,
    RoundRectangle = 10,
    Image = 11,
    Pie = 12,
    TextFrame = 13,
    RichTextDocument = 14,
    Ellipse = 15,
    Junction = 16,
    Polygon = 17,
    Polyline = 18,
    Wire = 19,
    Bus = 20,
    Bezier = 21,
    Label = 22,
    Hyperlink = 23,
    NetLabel = 24,
    Designator = 25,
    SchComponent = 26,
    Parameter = 27,
    ParameterSet = 28,
    ParameterList = 29,
    SheetName = 30,
    SheetFileName = 31,
    Sheet = 32,
    SchLib = 33,
    Symbol = 34,
    NoERC = 35,
    ErrorMarker = 36,
    Pin = 37,
    Port = 38,
    PowerObject = 39,
    SheetEntry = 40,
    SheetSymbol = 41,
    Template = 42,
    TaskHolder = 43,
    MapDefiner = 44,
    ImplementationMap = 45,
    Implementation = 46,
    ImplementationsList = 47,
    CrossSheetConnector = 48,
    CompileMask = 49,
    OpenBusComponent = 50,
    OpenBusLink = 51,
    OpenBusDesignator = 52,
    HarnessConnector = 53,
    HarnessEntry = 54,
    HarnessConnectorType = 55,
    SignalHarness = 56,
    OpenBusPort = 57,
    HighLevelCodeSymbol = 58,
    HighLevelCodeEntry = 59,
    OpenBusPinGroup = 60,
    Blanket = 61,
    RTFLink = 62,
    FSMState = 63,
    FSMTransition = 64,
    CommentThread = 65,
    CommentThreadNote = 66,
    FSMNote = 67,
    DiagramModule = 68,
    DiagramModuleName = 69,
    DiagramModuleSource = 70,
    DiagramConnector = 71,
    DiagramBlock = 72,
    DiagramHarness = 73,
    DiagramHarnessName = 74,
    DiagramHarnessSource = 75,
    DiagramConnectorLink = 76,
    DiagramPin = 77,
    VirtualParameter = 78,
    HarnessWiringDiagram = 79,
    HarnessLayoutDrawing = 80,
    HarnessComponent = 81,
    HarnessWire = 82,
    HarnessSplice = 83,
    HarnessLayoutLabel = 84,
    HarnessLayoutConnectionPoint = 85,
    HarnessBundle = 86,
    HarnessLogicalSignal = 87,
    HarnessPin = 88,
    HarnessWireLabel = 89,
    HarnessWireData = 90,
    HarnessSpliceData = 91,
    HarnessShield = 92,
    HarnessTwist = 93,
    HarnessNoConnect = 94,
    HarnessNoConnectData = 95,
    HarnessShieldData = 96,
    HarnessTwistData = 97,
    HarnessCable = 98,
    HarnessCableData = 99,
    ImageParameter = 100,
    HarnessAssociatedParts = 101,
    HarnessLibrary = 102,
    LineView = 103,
    HarnessCovering = 104,
    ObjectDefinition = 105,
    HarnessWireBreak = 106,
    AssociatedObjects = 107,
    ElectronicsSystemDesignDocument = 108,
    FunctionalBlock = 109,
    FunctionalConnectionLine = 110,
    FunctionalTextFrame = 111,
    SchematicBlock = 112,
    ReuseSheetSymbol = 113,
    ReuseBlockImplementationInfo = 114,
    LastObjectId = 115,
}

impl ObjectId {
    /// Convert from integer discriminant.
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 115 {
            // SAFETY: all values 0..=115 are valid discriminants
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Pin electrical type — `TPinElectrical` from C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PinElectrical {
    #[default]
    Input = 0,
    IO = 1,
    Output = 2,
    OpenCollector = 3,
    Passive = 4,
    HiZ = 5,
    OpenEmitter = 6,
    Power = 7,
}

impl PinElectrical {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 7 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Rotation by 90-degree increments — `TRotationBy90` from C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum RotationBy90 {
    #[default]
    Rotate0 = 0,
    Rotate90 = 1,
    Rotate180 = 2,
    Rotate270 = 3,
}

impl RotationBy90 {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 3 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Line style — `TLineStyle` from C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum LineStyle {
    #[default]
    Solid = 0,
    Dashed = 1,
    Dotted = 2,
    DashDotted = 3,
}

impl LineStyle {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 3 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// IEEE symbol type — `TIeeeSymbol` from C# (used for pin symbols).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum IeeeSymbol {
    #[default]
    None = 0,
    Dot = 1,
    RightLeftSignalFlow = 2,
    Clock = 3,
    ActiveLowInput = 4,
    AnalogSignalIn = 5,
    NotLogicConnection = 6,
    PostponedOutput = 8,
    OpenCollector = 9,
    HiZ = 10,
    HighCurrent = 11,
    Pulse = 12,
    Schmitt = 13,
    OpenCollectorPullup = 17,
    OpenEmitter = 22,
    OpenEmitterPullup = 23,
    ShiftLeft = 24,
    OpenOutput = 25,
    LeftRightSignalFlow = 33,
    BiDirectionalSignalFlow = 34,
}

impl IeeeSymbol {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::None,
            1 => Self::Dot,
            2 => Self::RightLeftSignalFlow,
            3 => Self::Clock,
            4 => Self::ActiveLowInput,
            5 => Self::AnalogSignalIn,
            6 => Self::NotLogicConnection,
            8 => Self::PostponedOutput,
            9 => Self::OpenCollector,
            10 => Self::HiZ,
            11 => Self::HighCurrent,
            12 => Self::Pulse,
            13 => Self::Schmitt,
            17 => Self::OpenCollectorPullup,
            22 => Self::OpenEmitter,
            23 => Self::OpenEmitterPullup,
            24 => Self::ShiftLeft,
            25 => Self::OpenOutput,
            33 => Self::LeftRightSignalFlow,
            34 => Self::BiDirectionalSignalFlow,
            _ => Self::None,
        }
    }
}

/// Port arrow style — `TPortArrowStyle` from C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
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

impl PortArrowStyle {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 7 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Port I/O direction — `TPortIO` from C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PortIO {
    #[default]
    Unspecified = 0,
    Output = 1,
    Input = 2,
    Bidirectional = 3,
}

impl PortIO {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 3 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Power object style — `TPowerObjectStyle` from C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
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
    GOSTArrow = 7,
    GOSTGndPower = 8,
    GOSTGndEarth = 9,
    GOSTBar = 10,
}

impl PowerObjectStyle {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 10 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Text justification — `TTextJustification` from C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
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

impl TextJustification {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 8 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Sheet style — `TSheetStyle` from C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
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

impl SheetStyle {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 17 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Pin item mode — `TPinItemMode` from C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PinItemMode {
    #[default]
    Default = 0,
    Custom = 1,
}

impl PinItemMode {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 1 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Pin text rotation anchor — used for name/designator custom positioning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PinTextRotationAnchor {
    #[default]
    Pin = 0,
    Component = 1,
}

impl PinTextRotationAnchor {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 1 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Component kind — `TComponentKind` from C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ComponentKind {
    #[default]
    Standard = 0,
    Mechanical = 1,
    Graphical = 2,
    StandardNoBOM = 5,
    Jumper = 6,
}

impl ComponentKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Standard),
            1 => Some(Self::Mechanical),
            2 => Some(Self::Graphical),
            5 => Some(Self::StandardNoBOM),
            6 => Some(Self::Jumper),
            _ => None,
        }
    }
}

/// Size (line width) — `TSize` from C# `Rt_Schematic` namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum Size {
    Smallest = 0,
    #[default]
    Small = 1,
    Medium = 2,
    Large = 3,
}

impl Size {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 3 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// NoERC symbol type — `TNoERCSymbol` from C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum NoERCSymbol {
    #[default]
    CrossThin = 0,
    CrossThick = 1,
    CrossSmall = 2,
    CheckBox = 3,
    Triangle = 4,
}

impl NoERCSymbol {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 4 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Serializer type — `TSerializerType` from C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SerializerType {
    Parametric = 0,
    ParametricAscii = 1,
    Ascii = 2,
    Binary = 3,
    ParametricJSON = 4,
}

/// Parameter type — `TParameterType` from C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ParameterType {
    #[default]
    String = 0,
    Integer = 1,
}

impl ParameterType {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 1 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Parameter read-only state — `TParameter_ReadOnlyState` from C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ParameterReadOnlyState {
    #[default]
    ReadWrite = 0,
    ReadOnly = 1,
}

impl ParameterReadOnlyState {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 1 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Formal type for pins — `TStdLogicState` from C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum StdLogicState {
    #[default]
    DontCare = 0,
    Low = 1,
    High = 2,
    HighZ = 3,
    Weak = 4,
    WeakLow = 5,
    WeakHigh = 6,
    Unknown = 7,
    Uninitialized = 8,
}

impl StdLogicState {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 8 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Horizontal alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum HorizontalAlign {
    #[default]
    Left = 0,
    Center = 1,
    Right = 2,
}

impl HorizontalAlign {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 2 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Line shape for polylines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
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

impl LineShape {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 6 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Left/right side indicator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum LeftRightSide {
    #[default]
    Left = 0,
    Right = 1,
}

impl LeftRightSide {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 1 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Parameter set style — `TParameterSetStyle` from C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ParameterSetStyle {
    #[default]
    Name = 0,
    Flat = 1,
}

impl ParameterSetStyle {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 1 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Text horizontal anchor — `TTextHorzAnchor` from C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum TextHorzAnchor {
    #[default]
    None = 0,
    Left = 1,
    Center = 2,
    Right = 3,
}

impl TextHorzAnchor {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 3 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// Text vertical anchor — `TTextVertAnchor` from C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum TextVertAnchor {
    #[default]
    None = 0,
    Top = 1,
    Center = 2,
    Bottom = 3,
}

impl TextVertAnchor {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 3 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }
}

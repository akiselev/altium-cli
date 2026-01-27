//! Schematic record type identifiers.

/// Schematic record type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SchRecordId {
    Component = 1,
    Pin = 2,
    Symbol = 3,
    Label = 4,
    Bezier = 5,
    Polyline = 6,
    Polygon = 7,
    Ellipse = 8,
    Pie = 9,
    EllipticalArc = 11,
    Arc = 12,
    Line = 13,
    Rectangle = 14,
    PowerObject = 17,
    Port = 18,
    NoErc = 22,
    NetLabel = 25,
    Bus = 26,
    Wire = 27,
    TextFrame = 28,
    Junction = 29,
    Image = 30,
    SheetHeader = 31,
    Designator = 34,
    BusEntry = 37,
    Parameter = 41,
    WarningSign = 43,
    ImplementationList = 44,
    Implementation = 45,
    MapDefinerList = 46,
    MapDefiner = 47,
    ImplementationParameters = 48,
    TextFrameVariant = 209,
}

impl SchRecordId {
    /// Convert from u8 (returns None for unknown IDs).
    pub fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            1 => SchRecordId::Component,
            2 => SchRecordId::Pin,
            3 => SchRecordId::Symbol,
            4 => SchRecordId::Label,
            5 => SchRecordId::Bezier,
            6 => SchRecordId::Polyline,
            7 => SchRecordId::Polygon,
            8 => SchRecordId::Ellipse,
            9 => SchRecordId::Pie,
            11 => SchRecordId::EllipticalArc,
            12 => SchRecordId::Arc,
            13 => SchRecordId::Line,
            14 => SchRecordId::Rectangle,
            17 => SchRecordId::PowerObject,
            18 => SchRecordId::Port,
            22 => SchRecordId::NoErc,
            25 => SchRecordId::NetLabel,
            26 => SchRecordId::Bus,
            27 => SchRecordId::Wire,
            28 => SchRecordId::TextFrame,
            29 => SchRecordId::Junction,
            30 => SchRecordId::Image,
            31 => SchRecordId::SheetHeader,
            34 => SchRecordId::Designator,
            37 => SchRecordId::BusEntry,
            41 => SchRecordId::Parameter,
            43 => SchRecordId::WarningSign,
            44 => SchRecordId::ImplementationList,
            45 => SchRecordId::Implementation,
            46 => SchRecordId::MapDefinerList,
            47 => SchRecordId::MapDefiner,
            48 => SchRecordId::ImplementationParameters,
            209 => SchRecordId::TextFrameVariant,
            _ => return None,
        })
    }

    /// Convert to u8.
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

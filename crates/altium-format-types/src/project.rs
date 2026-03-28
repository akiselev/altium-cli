use std::fmt;
use std::str::FromStr;

use crate::InvalidEnumValue;

/// Netlist flattening mode for PCB project compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(i32)]
pub enum FlattenMode {
    Smart = 0,
    Flat = 1,
    HierarchicalGlobalPorts = 2,
    Global = 3,
    HierarchicalStrict = 4,
}

impl TryFrom<i32> for FlattenMode {
    type Error = InvalidEnumValue;
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Smart),
            1 => Ok(Self::Flat),
            2 => Ok(Self::HierarchicalGlobalPorts),
            3 => Ok(Self::Global),
            4 => Ok(Self::HierarchicalStrict),
            _ => Err(InvalidEnumValue {
                type_name: "FlattenMode",
                value: v as i64,
            }),
        }
    }
}

impl fmt::Display for FlattenMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Smart => write!(f, "Smart"),
            Self::Flat => write!(f, "Flat"),
            Self::HierarchicalGlobalPorts => write!(f, "HierarchicalGlobalPorts"),
            Self::Global => write!(f, "Global"),
            Self::HierarchicalStrict => write!(f, "HierarchicalStrict"),
        }
    }
}

/// Naming style for channel room designators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(i32)]
pub enum ChannelRoomNamingStyle {
    FlatNumericWithNames = 0,
    FlatNumeric = 1,
    FullyQualified = 2,
    FullyQualifiedShort = 3,
    MixedNamePath = 4,
}

impl TryFrom<i32> for ChannelRoomNamingStyle {
    type Error = InvalidEnumValue;
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::FlatNumericWithNames),
            1 => Ok(Self::FlatNumeric),
            2 => Ok(Self::FullyQualified),
            3 => Ok(Self::FullyQualifiedShort),
            4 => Ok(Self::MixedNamePath),
            _ => Err(InvalidEnumValue {
                type_name: "ChannelRoomNamingStyle",
                value: v as i64,
            }),
        }
    }
}

impl fmt::Display for ChannelRoomNamingStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FlatNumericWithNames => write!(f, "FlatNumericWithNames"),
            Self::FlatNumeric => write!(f, "FlatNumeric"),
            Self::FullyQualified => write!(f, "FullyQualified"),
            Self::FullyQualifiedShort => write!(f, "FullyQualifiedShort"),
            Self::MixedNamePath => write!(f, "MixedNamePath"),
        }
    }
}

/// Cross-reference sheet style for schematic cross-references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(i32)]
pub enum CrossRefSheetStyle {
    None = 0,
    Name = 1,
    Number = 2,
}

impl TryFrom<i32> for CrossRefSheetStyle {
    type Error = InvalidEnumValue;
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::None),
            1 => Ok(Self::Name),
            2 => Ok(Self::Number),
            _ => Err(InvalidEnumValue {
                type_name: "CrossRefSheetStyle",
                value: v as i64,
            }),
        }
    }
}

impl fmt::Display for CrossRefSheetStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Name => write!(f, "Name"),
            Self::Number => write!(f, "Number"),
        }
    }
}

/// Cross-reference location style for schematic cross-references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(i32)]
pub enum CrossRefLocationStyle {
    None = 0,
    Zone = 1,
    XY = 2,
}

impl TryFrom<i32> for CrossRefLocationStyle {
    type Error = InvalidEnumValue;
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::None),
            1 => Ok(Self::Zone),
            2 => Ok(Self::XY),
            _ => Err(InvalidEnumValue {
                type_name: "CrossRefLocationStyle",
                value: v as i64,
            }),
        }
    }
}

impl fmt::Display for CrossRefLocationStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Zone => write!(f, "Zone"),
            Self::XY => write!(f, "XY"),
        }
    }
}

/// Cross-reference port participation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(i32)]
pub enum CrossRefPorts {
    Disabled = 0,
    SheetEntry = 1,
    Ports = 2,
    SheetEntryAndPorts = 3,
}

impl TryFrom<i32> for CrossRefPorts {
    type Error = InvalidEnumValue;
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Disabled),
            1 => Ok(Self::SheetEntry),
            2 => Ok(Self::Ports),
            3 => Ok(Self::SheetEntryAndPorts),
            _ => Err(InvalidEnumValue {
                type_name: "CrossRefPorts",
                value: v as i64,
            }),
        }
    }
}

impl fmt::Display for CrossRefPorts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => write!(f, "Disabled"),
            Self::SheetEntry => write!(f, "SheetEntry"),
            Self::Ports => write!(f, "Ports"),
            Self::SheetEntryAndPorts => write!(f, "SheetEntryAndPorts"),
        }
    }
}

/// Sort order for annotation and BOM generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(i32)]
pub enum SortOrder {
    UpThenAcross = 0,
    DownThenAcross = 1,
    AcrossThenUp = 2,
    AcrossThenDown = 3,
}

impl TryFrom<i32> for SortOrder {
    type Error = InvalidEnumValue;
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::UpThenAcross),
            1 => Ok(Self::DownThenAcross),
            2 => Ok(Self::AcrossThenUp),
            3 => Ok(Self::AcrossThenDown),
            _ => Err(InvalidEnumValue {
                type_name: "SortOrder",
                value: v as i64,
            }),
        }
    }
}

impl fmt::Display for SortOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UpThenAcross => write!(f, "UpThenAcross"),
            Self::DownThenAcross => write!(f, "DownThenAcross"),
            Self::AcrossThenUp => write!(f, "AcrossThenUp"),
            Self::AcrossThenDown => write!(f, "AcrossThenDown"),
        }
    }
}

/// Sort location anchor for annotation ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(i32)]
pub enum SortLocation {
    Designator = 0,
    Part = 1,
}

impl TryFrom<i32> for SortLocation {
    type Error = InvalidEnumValue;
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Designator),
            1 => Ok(Self::Part),
            _ => Err(InvalidEnumValue {
                type_name: "SortLocation",
                value: v as i64,
            }),
        }
    }
}

impl fmt::Display for SortLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Designator => write!(f, "Designator"),
            Self::Part => write!(f, "Part"),
        }
    }
}

/// ERC error level, used in violation matrix entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(i32)]
pub enum ErrorLevel {
    NoReport = 0,
    Warning = 1,
    Error = 2,
    Fatal = 3,
}

impl ErrorLevel {
    /// Parse the single-character matrix encoding used in PrjPcb ERC matrices.
    pub fn from_matrix_char(ch: char) -> Option<Self> {
        match ch {
            'N' => Some(Self::NoReport),
            'W' => Some(Self::Warning),
            'E' => Some(Self::Error),
            'F' => Some(Self::Fatal),
            _ => None,
        }
    }

    /// Return the single-character matrix encoding for this level.
    pub fn to_matrix_char(self) -> char {
        match self {
            Self::NoReport => 'N',
            Self::Warning => 'W',
            Self::Error => 'E',
            Self::Fatal => 'F',
        }
    }
}

impl TryFrom<i32> for ErrorLevel {
    type Error = InvalidEnumValue;
    fn try_from(v: i32) -> Result<Self, <Self as TryFrom<i32>>::Error> {
        match v {
            0 => Ok(Self::NoReport),
            1 => Ok(Self::Warning),
            2 => Ok(Self::Error),
            3 => Ok(Self::Fatal),
            _ => Err(InvalidEnumValue {
                type_name: "ErrorLevel",
                value: v as i64,
            }),
        }
    }
}

impl fmt::Display for ErrorLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoReport => write!(f, "NoReport"),
            Self::Warning => write!(f, "Warning"),
            Self::Error => write!(f, "Error"),
            Self::Fatal => write!(f, "Fatal"),
        }
    }
}

/// Difference check sensitivity level for comparator rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(i32)]
pub enum DifferenceCheckLevel {
    Off = 0,
    On = 1,
    OnCaseSensitive = 2,
}

impl TryFrom<i32> for DifferenceCheckLevel {
    type Error = InvalidEnumValue;
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Off),
            1 => Ok(Self::On),
            2 => Ok(Self::OnCaseSensitive),
            _ => Err(InvalidEnumValue {
                type_name: "DifferenceCheckLevel",
                value: v as i64,
            }),
        }
    }
}

impl fmt::Display for DifferenceCheckLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Off => write!(f, "Off"),
            Self::On => write!(f, "On"),
            Self::OnCaseSensitive => write!(f, "OnCaseSensitive"),
        }
    }
}

/// Variation kind for assembly variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(i32)]
pub enum VariationKind {
    None = 0,
    NotFitted = 1,
    Alternate = 2,
}

impl TryFrom<i32> for VariationKind {
    type Error = InvalidEnumValue;
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::None),
            1 => Ok(Self::NotFitted),
            2 => Ok(Self::Alternate),
            _ => Err(InvalidEnumValue {
                type_name: "VariationKind",
                value: v as i64,
            }),
        }
    }
}

impl fmt::Display for VariationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::NotFitted => write!(f, "NotFitted"),
            Self::Alternate => write!(f, "Alternate"),
        }
    }
}

/// Scope for document-level annotation operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DocAnnotationScope {
    All,
    IgnoreSelected,
    OnlySelected,
}

impl fmt::Display for DocAnnotationScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => write!(f, "All"),
            Self::IgnoreSelected => write!(f, "Ignore Selected Parts"),
            Self::OnlySelected => write!(f, "Only Selected Parts"),
        }
    }
}

impl FromStr for DocAnnotationScope {
    type Err = InvalidEnumValue;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "All" => Ok(Self::All),
            "Ignore Selected Parts" => Ok(Self::IgnoreSelected),
            "Only Selected Parts" => Ok(Self::OnlySelected),
            _ => Err(InvalidEnumValue {
                type_name: "DocAnnotationScope",
                value: 0,
            }),
        }
    }
}

/// Scope for automatic net class assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DocAutoNetClassScope {
    None,
    LocalOnly,
    All,
}

impl fmt::Display for DocAutoNetClassScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::LocalOnly => write!(f, "Local Nets Only"),
            Self::All => write!(f, "All Nets"),
        }
    }
}

impl FromStr for DocAutoNetClassScope {
    type Err = InvalidEnumValue;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "None" => Ok(Self::None),
            "Local Nets Only" => Ok(Self::LocalOnly),
            "All Nets" => Ok(Self::All),
            _ => Err(InvalidEnumValue {
                type_name: "DocAutoNetClassScope",
                value: 0,
            }),
        }
    }
}

/// Connection code used as row/column index in the ERC violation matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(i32)]
pub enum ConnectionCode {
    PinInput = 0,
    PinBidirectional = 1,
    PinOutput = 2,
    PinOpenCollector = 3,
    PinPassive = 4,
    PinHiZ = 5,
    PinOpenEmitter = 6,
    PinPower = 7,
    SheetEntryInput = 8,
    SheetEntryBidirectional = 9,
    SheetEntryOutput = 10,
    PortUnspecified = 11,
    PinUnspecified = 12,
    SheetEntryUnspecified = 13,
    PortInput = 14,
    PortOutput = 15,
    Unconnected = 16,
}

impl TryFrom<i32> for ConnectionCode {
    type Error = InvalidEnumValue;
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::PinInput),
            1 => Ok(Self::PinBidirectional),
            2 => Ok(Self::PinOutput),
            3 => Ok(Self::PinOpenCollector),
            4 => Ok(Self::PinPassive),
            5 => Ok(Self::PinHiZ),
            6 => Ok(Self::PinOpenEmitter),
            7 => Ok(Self::PinPower),
            8 => Ok(Self::SheetEntryInput),
            9 => Ok(Self::SheetEntryBidirectional),
            10 => Ok(Self::SheetEntryOutput),
            11 => Ok(Self::PortUnspecified),
            12 => Ok(Self::PinUnspecified),
            13 => Ok(Self::SheetEntryUnspecified),
            14 => Ok(Self::PortInput),
            15 => Ok(Self::PortOutput),
            16 => Ok(Self::Unconnected),
            _ => Err(InvalidEnumValue {
                type_name: "ConnectionCode",
                value: v as i64,
            }),
        }
    }
}

impl fmt::Display for ConnectionCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PinInput => write!(f, "PinInput"),
            Self::PinBidirectional => write!(f, "PinBidirectional"),
            Self::PinOutput => write!(f, "PinOutput"),
            Self::PinOpenCollector => write!(f, "PinOpenCollector"),
            Self::PinPassive => write!(f, "PinPassive"),
            Self::PinHiZ => write!(f, "PinHiZ"),
            Self::PinOpenEmitter => write!(f, "PinOpenEmitter"),
            Self::PinPower => write!(f, "PinPower"),
            Self::SheetEntryInput => write!(f, "SheetEntryInput"),
            Self::SheetEntryBidirectional => write!(f, "SheetEntryBidirectional"),
            Self::SheetEntryOutput => write!(f, "SheetEntryOutput"),
            Self::PortUnspecified => write!(f, "PortUnspecified"),
            Self::PinUnspecified => write!(f, "PinUnspecified"),
            Self::SheetEntryUnspecified => write!(f, "SheetEntryUnspecified"),
            Self::PortInput => write!(f, "PortInput"),
            Self::PortOutput => write!(f, "PortOutput"),
            Self::Unconnected => write!(f, "Unconnected"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_mode_round_trip() {
        for (i, expected) in [
            (0, FlattenMode::Smart),
            (1, FlattenMode::Flat),
            (2, FlattenMode::HierarchicalGlobalPorts),
            (3, FlattenMode::Global),
            (4, FlattenMode::HierarchicalStrict),
        ] {
            assert_eq!(FlattenMode::try_from(i).unwrap(), expected);
        }
        assert!(FlattenMode::try_from(99).is_err());
    }

    #[test]
    fn channel_room_naming_style_round_trip() {
        for (i, expected) in [
            (0, ChannelRoomNamingStyle::FlatNumericWithNames),
            (1, ChannelRoomNamingStyle::FlatNumeric),
            (2, ChannelRoomNamingStyle::FullyQualified),
            (3, ChannelRoomNamingStyle::FullyQualifiedShort),
            (4, ChannelRoomNamingStyle::MixedNamePath),
        ] {
            assert_eq!(ChannelRoomNamingStyle::try_from(i).unwrap(), expected);
        }
        assert!(ChannelRoomNamingStyle::try_from(99).is_err());
    }

    #[test]
    fn cross_ref_sheet_style_round_trip() {
        for (i, expected) in [
            (0, CrossRefSheetStyle::None),
            (1, CrossRefSheetStyle::Name),
            (2, CrossRefSheetStyle::Number),
        ] {
            assert_eq!(CrossRefSheetStyle::try_from(i).unwrap(), expected);
        }
        assert!(CrossRefSheetStyle::try_from(99).is_err());
    }

    #[test]
    fn cross_ref_location_style_round_trip() {
        for (i, expected) in [
            (0, CrossRefLocationStyle::None),
            (1, CrossRefLocationStyle::Zone),
            (2, CrossRefLocationStyle::XY),
        ] {
            assert_eq!(CrossRefLocationStyle::try_from(i).unwrap(), expected);
        }
        assert!(CrossRefLocationStyle::try_from(99).is_err());
    }

    #[test]
    fn cross_ref_ports_round_trip() {
        for (i, expected) in [
            (0, CrossRefPorts::Disabled),
            (1, CrossRefPorts::SheetEntry),
            (2, CrossRefPorts::Ports),
            (3, CrossRefPorts::SheetEntryAndPorts),
        ] {
            assert_eq!(CrossRefPorts::try_from(i).unwrap(), expected);
        }
        assert!(CrossRefPorts::try_from(99).is_err());
    }

    #[test]
    fn sort_order_round_trip() {
        for (i, expected) in [
            (0, SortOrder::UpThenAcross),
            (1, SortOrder::DownThenAcross),
            (2, SortOrder::AcrossThenUp),
            (3, SortOrder::AcrossThenDown),
        ] {
            assert_eq!(SortOrder::try_from(i).unwrap(), expected);
        }
        assert!(SortOrder::try_from(99).is_err());
    }

    #[test]
    fn sort_location_round_trip() {
        for (i, expected) in [(0, SortLocation::Designator), (1, SortLocation::Part)] {
            assert_eq!(SortLocation::try_from(i).unwrap(), expected);
        }
        assert!(SortLocation::try_from(99).is_err());
    }

    #[test]
    fn error_level_round_trip() {
        for (i, expected) in [
            (0, ErrorLevel::NoReport),
            (1, ErrorLevel::Warning),
            (2, ErrorLevel::Error),
            (3, ErrorLevel::Fatal),
        ] {
            assert_eq!(ErrorLevel::try_from(i).unwrap(), expected);
        }
        assert!(ErrorLevel::try_from(99).is_err());
    }

    #[test]
    fn error_level_matrix_chars() {
        assert_eq!(
            ErrorLevel::from_matrix_char('N'),
            Some(ErrorLevel::NoReport)
        );
        assert_eq!(ErrorLevel::from_matrix_char('W'), Some(ErrorLevel::Warning));
        assert_eq!(ErrorLevel::from_matrix_char('E'), Some(ErrorLevel::Error));
        assert_eq!(ErrorLevel::from_matrix_char('F'), Some(ErrorLevel::Fatal));
        assert_eq!(ErrorLevel::from_matrix_char('X'), None);

        assert_eq!(ErrorLevel::NoReport.to_matrix_char(), 'N');
        assert_eq!(ErrorLevel::Warning.to_matrix_char(), 'W');
        assert_eq!(ErrorLevel::Error.to_matrix_char(), 'E');
        assert_eq!(ErrorLevel::Fatal.to_matrix_char(), 'F');
    }

    #[test]
    fn difference_check_level_round_trip() {
        for (i, expected) in [
            (0, DifferenceCheckLevel::Off),
            (1, DifferenceCheckLevel::On),
            (2, DifferenceCheckLevel::OnCaseSensitive),
        ] {
            assert_eq!(DifferenceCheckLevel::try_from(i).unwrap(), expected);
        }
        assert!(DifferenceCheckLevel::try_from(99).is_err());
    }

    #[test]
    fn variation_kind_round_trip() {
        for (i, expected) in [
            (0, VariationKind::None),
            (1, VariationKind::NotFitted),
            (2, VariationKind::Alternate),
        ] {
            assert_eq!(VariationKind::try_from(i).unwrap(), expected);
        }
        assert!(VariationKind::try_from(99).is_err());
    }

    #[test]
    fn doc_annotation_scope_from_str_display() {
        assert_eq!(
            "All".parse::<DocAnnotationScope>().unwrap(),
            DocAnnotationScope::All
        );
        assert_eq!(
            "Ignore Selected Parts"
                .parse::<DocAnnotationScope>()
                .unwrap(),
            DocAnnotationScope::IgnoreSelected
        );
        assert_eq!(
            "Only Selected Parts".parse::<DocAnnotationScope>().unwrap(),
            DocAnnotationScope::OnlySelected
        );
        assert!("unknown".parse::<DocAnnotationScope>().is_err());

        assert_eq!(DocAnnotationScope::All.to_string(), "All");
        assert_eq!(
            DocAnnotationScope::IgnoreSelected.to_string(),
            "Ignore Selected Parts"
        );
        assert_eq!(
            DocAnnotationScope::OnlySelected.to_string(),
            "Only Selected Parts"
        );
    }

    #[test]
    fn doc_auto_net_class_scope_from_str_display() {
        assert_eq!(
            "None".parse::<DocAutoNetClassScope>().unwrap(),
            DocAutoNetClassScope::None
        );
        assert_eq!(
            "Local Nets Only".parse::<DocAutoNetClassScope>().unwrap(),
            DocAutoNetClassScope::LocalOnly
        );
        assert_eq!(
            "All Nets".parse::<DocAutoNetClassScope>().unwrap(),
            DocAutoNetClassScope::All
        );
        assert!("unknown".parse::<DocAutoNetClassScope>().is_err());

        assert_eq!(DocAutoNetClassScope::None.to_string(), "None");
        assert_eq!(
            DocAutoNetClassScope::LocalOnly.to_string(),
            "Local Nets Only"
        );
        assert_eq!(DocAutoNetClassScope::All.to_string(), "All Nets");
    }

    #[test]
    fn connection_code_round_trip() {
        for (i, expected) in [
            (0, ConnectionCode::PinInput),
            (1, ConnectionCode::PinBidirectional),
            (2, ConnectionCode::PinOutput),
            (3, ConnectionCode::PinOpenCollector),
            (4, ConnectionCode::PinPassive),
            (5, ConnectionCode::PinHiZ),
            (6, ConnectionCode::PinOpenEmitter),
            (7, ConnectionCode::PinPower),
            (8, ConnectionCode::SheetEntryInput),
            (9, ConnectionCode::SheetEntryBidirectional),
            (10, ConnectionCode::SheetEntryOutput),
            (11, ConnectionCode::PortUnspecified),
            (12, ConnectionCode::PinUnspecified),
            (13, ConnectionCode::SheetEntryUnspecified),
            (14, ConnectionCode::PortInput),
            (15, ConnectionCode::PortOutput),
            (16, ConnectionCode::Unconnected),
        ] {
            assert_eq!(ConnectionCode::try_from(i).unwrap(), expected);
        }
        assert!(ConnectionCode::try_from(99).is_err());
    }
}

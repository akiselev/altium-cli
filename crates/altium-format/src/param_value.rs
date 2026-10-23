//! Conversion traits between raw Altium parameter strings and typed Rust values.
//! `FromParamValue`: parse a string value for a named key into `T`.
//! `ToParamValue`: serialize `T` back to the Altium string representation.
//! `bool` uses Altium's T/F encoding, not Rust's true/false.
use altium_format_types::{
    BgaFanoutDirection, BgaFanoutViaMode, ClearanceConstraintMode, Color,
    ComponentCollisionCheckMode, ComponentKind, ComponentOrientationFlags, ConfinementStyle, Coord,
    CornerStyle, FanoutDirection, FanoutStyle, HorizontalAlign, IeeeSymbol, LeftRightSide,
    LengthenerStyle, LineShape, LineStyle, NetScope, NetTopology, ObjectClearanceId,
    ParameterReadOnlyState, ParameterType, PenWidth, PlaneConnectionStyle, PolygonReliefAngle,
    RotationBy90, RouteVia, RuleKind, RuleLayerKind, SheetSymbolType, TestpointValid,
    TextHorzAnchor, TextJustification, TextVertAnchor, UniqueId,
    sch::{PortArrowStyle, PortIoType, PowerObjectStyle},
};

use crate::{AltiumFormatError, Result};

pub(crate) trait FromParamValue: Sized {
    fn from_param_value(key: &str, value: &str) -> Result<Self>;
}

pub(crate) trait ToParamValue {
    fn to_param_value(&self) -> String;
}

impl FromParamValue for String {
    fn from_param_value(_key: &str, value: &str) -> Result<Self> {
        Ok(value.to_owned())
    }
}

impl ToParamValue for String {
    fn to_param_value(&self) -> String {
        self.clone()
    }
}

impl FromParamValue for bool {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        match value {
            "T" | "TRUE" => Ok(true),
            "F" | "FALSE" => Ok(false),
            other => Err(AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: format!("expected T/F/TRUE/FALSE, got {other:?}"),
            }),
        }
    }
}

impl ToParamValue for bool {
    fn to_param_value(&self) -> String {
        if *self {
            "TRUE".to_owned()
        } else {
            "FALSE".to_owned()
        }
    }
}

macro_rules! impl_int_param_value {
    ($($t:ty),+) => {
        $(
            impl FromParamValue for $t {
                fn from_param_value(key: &str, value: &str) -> Result<Self> {
                    value.parse::<$t>().map_err(|e| AltiumFormatError::InvalidParamValue {
                        key: key.to_owned(),
                        detail: e.to_string(),
                    })
                }
            }

            impl ToParamValue for $t {
                fn to_param_value(&self) -> String {
                    self.to_string()
                }
            }
        )+
    };
}

impl_int_param_value!(i8, u8, i16, u16, i32, u32);

// f64 is handled separately because Altium parameter strings sometimes contain leading
// whitespace in float values (e.g. "MODEL.3D.ROTX= 3.3E-0314"). Rust's f64::parse()
// rejects whitespace, so we trim before parsing. Integer types are kept strict.
impl FromParamValue for f64 {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        value
            .trim()
            .parse::<f64>()
            .map_err(|e| AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: e.to_string(),
            })
    }
}

impl ToParamValue for f64 {
    fn to_param_value(&self) -> String {
        format!("{self:.6}")
    }
}

impl FromParamValue for Coord {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        let raw: i32 = value.parse().map_err(|e: std::num::ParseIntError| {
            AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: e.to_string(),
            }
        })?;
        Ok(Coord::from_internal(raw))
    }
}

impl ToParamValue for Coord {
    fn to_param_value(&self) -> String {
        self.to_internal().to_string()
    }
}

// usize is excluded from impl_int_param_value! because its width is platform-dependent
// (32-bit or 64-bit depending on target); used for Weight and count fields.
impl FromParamValue for usize {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        value
            .parse::<usize>()
            .map_err(|e| AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: e.to_string(),
            })
    }
}

impl ToParamValue for usize {
    fn to_param_value(&self) -> String {
        self.to_string()
    }
}

// Color is stored as a decimal Win32 COLORREF integer (0x00BBGGRR) in parameter strings.
impl FromParamValue for Color {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        let raw: i32 = value.parse().map_err(|e: std::num::ParseIntError| {
            AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: e.to_string(),
            }
        })?;
        Ok(Color::new(raw))
    }
}

impl ToParamValue for Color {
    fn to_param_value(&self) -> String {
        self.raw().to_string()
    }
}

// Enum types that are stored as decimal integer discriminants in parameter strings.
// Each enum must implement TryFrom<u8> with Error = InvalidEnumValue.
macro_rules! impl_enum_param_value {
    ($($t:ty),+ $(,)?) => {
        $(
            impl FromParamValue for $t {
                fn from_param_value(key: &str, value: &str) -> Result<Self> {
                    let raw = u8::from_param_value(key, value)?;
                    Self::try_from(raw).map_err(|e: altium_format_types::InvalidEnumValue| {
                        AltiumFormatError::InvalidParamValue {
                            key: key.to_owned(),
                            detail: e.to_string(),
                        }
                    })
                }
            }

            impl ToParamValue for $t {
                fn to_param_value(&self) -> String {
                    (*self as u8).to_string()
                }
            }
        )+
    };
}

impl_enum_param_value!(
    PenWidth,
    LineStyle,
    LineShape,
    HorizontalAlign,
    TextJustification,
    RotationBy90,
    ComponentKind,
    IeeeSymbol,
    ParameterReadOnlyState,
    ParameterType,
    TextHorzAnchor,
    TextVertAnchor,
    PowerObjectStyle,
    PortArrowStyle,
    PortIoType,
    LeftRightSide,
);

// ── String-keyed enum macro ─────────────────────────────────────────────────
// For enums serialized as string identifiers (e.g., RULEKIND="Clearance") rather
// than integer discriminants. Used by DRC parameters.
macro_rules! impl_string_enum_param_value {
    ($t:ty, $($variant:ident => $s:literal),+ $(,)?) => {
        impl FromParamValue for $t {
            #[allow(unreachable_patterns)]
            fn from_param_value(key: &str, value: &str) -> Result<Self> {
                match value {
                    $($s => Ok(<$t>::$variant),)+
                    _ => Err(AltiumFormatError::InvalidParamValue {
                        key: key.to_owned(),
                        detail: format!("unknown {} string: {:?}", stringify!($t), value),
                    }),
                }
            }
        }
        impl ToParamValue for $t {
            #[allow(unreachable_patterns)]
            fn to_param_value(&self) -> String {
                match self {
                    $(<$t>::$variant => $s,)+
                    // #[non_exhaustive] enums require a wildcard; unreachable at runtime
                    // because we define arms for every variant.
                    other => panic!(
                        "ToParamValue: unhandled {} variant {other:?} — add a match arm",
                        stringify!($t),
                    ),
                }.to_owned()
            }
        }
    };
}

// All 70 RuleKind string mappings from cRuleIdStrings (Consts.cs L445-515).
impl_string_enum_param_value!(RuleKind,
    Clearance => "Clearance",
    ParallelSegment => "ParallelSegment",
    Width => "Width",
    Length => "Length",
    MatchedLengths => "MatchedLengths",
    DaisyChainStubLength => "StubLength",
    PowerPlaneConnectStyle => "PlaneConnect",
    RoutingTopology => "RoutingTopology",
    RoutingPriority => "RoutingPriority",
    RoutingLayers => "RoutingLayers",
    RoutingCornerStyle => "RoutingCorners",
    RoutingViaStyle => "RoutingVias",
    PowerPlaneClearance => "PlaneClearance",
    SolderMaskExpansion => "SolderMaskExpansion",
    PasteMaskExpansion => "PasteMaskExpansion",
    ShortCircuit => "ShortCircuit",
    BrokenNets => "UnRoutedNet",
    ViasUnderSmd => "ViasUnderSMD",
    MaximumViaCount => "MaximumViaCount",
    MinimumAnnularRing => "MinimumAnnularRing",
    PolygonConnectStyle => "PolygonConnect",
    AcuteAngle => "AcuteAngle",
    ConfinementConstraint => "RoomDefinition",
    SmdToCorner => "SMDToCorner",
    ComponentClearance => "ComponentClearance",
    ComponentRotations => "ComponentOrientations",
    PermittedLayers => "PermittedLayers",
    NetsToIgnore => "NetsToIgnore",
    SignalStimulus => "SignalStimulus",
    OvershootFallingEdge => "OvershootFalling",
    OvershootRisingEdge => "OvershootRising",
    UndershootFallingEdge => "UndershootFalling",
    UndershootRisingEdge => "UndershootRising",
    MaxMinImpedance => "MaxMinImpedance",
    SignalTopValue => "SignalTopValue",
    SignalBaseValue => "SignalBaseValue",
    FlightTimeRisingEdge => "FlightTimeRising",
    FlightTimeFallingEdge => "FlightTimeFalling",
    LayerStack => "LayerStack",
    MaxSlopeRisingEdge => "SlopeRising",
    MaxSlopeFallingEdge => "SlopeFalling",
    SupplyNets => "SupplyNets",
    MaxMinHoleSize => "HoleSize",
    FabricationTestpointStyle => "FabricationTestpoint",
    FabricationTestpointUsage => "FabricationTestPointUsage",
    UnconnectedPin => "UnConnectedPin",
    SmdToPlane => "SMDToPlane",
    SmdNeckDown => "SMDNeckDown",
    LayerPair => "LayerPairs",
    FanoutControl => "FanoutControl",
    MaxMinHeight => "Height",
    DifferentialPairsRouting => "DiffPairsRouting",
    HoleToHoleClearance => "HoleToHoleClearance",
    MinimumSolderMaskSliver => "MinimumSolderMaskSliver",
    SilkToSolderMaskClearance => "SilkToSolderMaskClearance",
    SilkToSilkClearance => "SilkToSilkClearance",
    NetAntennae => "NetAntennae",
    AssyTestPointStyle => "AssemblyTestpoint",
    AssyTestPointUsage => "AssemblyTestPointUsage",
    SilkToBoardRegionClearance => "SilkToBoardRegionClearance",
    SmdEntry => "SMDEntry",
    None => "PCAD Rule",
    UnpouredPolygon => "UnpouredPolygon",
    BoardOutlineClearance => "BoardOutlineClearance",
    BackDrilling => "BackDrilling",
    Creepage => "Creepage",
    ReturnPath => "ReturnPath",
    RoutingNeckDown => "RoutingNeckDown",
    WireBonding => "WireBonding",
    ZAxisClearance => "ZAxisClearance",
);

impl_string_enum_param_value!(NetScope,
    DifferentNetsOnly => "DifferentNets",
    SameNetOnly => "SameNetOnly",
    AnyNet => "AnyNet",
    DifferentDiffPairsOnly => "DifferentPairs",
    SameDiffPairOnly => "SameDiffPairs",
);

impl_string_enum_param_value!(RuleLayerKind,
    SameLayer => "SameLayer",
    AdjacentLayers => "AdjacentLayers",
);

impl_string_enum_param_value!(NetTopology,
    Shortest => "Shortest",
    Horizontal => "Horizontal",
    Vertical => "Vertical",
    DaisyChainSimple => "Daisy_Simple",
    DaisyChainMidDriven => "Daisy_MidDriven",
    DaisyChainBalanced => "Daisy_Balanced",
    Starburst => "Starburst",
);

impl_string_enum_param_value!(RouteVia,
    ThruHole => "Through Hole",
    BlindBuriedPair => "Blind Buried (Adjacent Layers)",
    BlindBuriedAny => "Blind Buried (Any Layer Pair)",
    None => "xxx",
);

impl_string_enum_param_value!(PolygonReliefAngle,
    Angle45 => "45 Angle",
    Angle90 => "90 Angle",
    Angle0 => "0 Angle",
    Angle135 => "135 Angle",
);

impl_string_enum_param_value!(PlaneConnectionStyle,
    Relief => "Relief",
    Direct => "Direct",
    NoConnect => "NoConnect",
);

impl_string_enum_param_value!(ConfinementStyle,
    ConfineIn => "ConfineIn",
    ConfineOut => "ConfineOut",
);

impl_string_enum_param_value!(ClearanceConstraintMode,
    SingleClearance => "SingleClearance",
    ObjectsClearance => "ObjectsClearance",
);

impl_string_enum_param_value!(FanoutStyle,
    Auto => "Auto",
    Rows => "Rows",
    Staggered => "Staggered",
    Bga => "BGA",
    UnderPads => "UnderPads",
);

impl_string_enum_param_value!(FanoutDirection,
    None => "None",
    InOnly => "InOnly",
    OutOnly => "OutOnly",
    InThenOut => "InThenOut",
    OutThenIn => "OutThenIn",
    Alternating => "Alternating",
);

impl_string_enum_param_value!(BgaFanoutDirection,
    Out => "Out",
    In => "In",
);

impl_string_enum_param_value!(BgaFanoutViaMode,
    Centered => "Centered",
    Offset => "Offset",
    Closest => "Closest",
);

// LengthenerStyle: string values from xPCBTypes.Consts.cLengthenerStyleStrings.
impl_string_enum_param_value!(LengthenerStyle,
    Degree90 => "90-Degree",
    Degree45 => "45-Degree",
    Round => "Round",
    Mitered90 => "Mitered",
);

impl FromParamValue for CornerStyle {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        match value {
            "90-Degree" => Ok(Self::Degree90),
            "45-Degree" => Ok(Self::Degree45),
            "Rounded" | "Round" => Ok(Self::Round),
            _ => Err(AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: format!("unknown CornerStyle string: {:?}", value),
            }),
        }
    }
}

impl ToParamValue for CornerStyle {
    fn to_param_value(&self) -> String {
        match self {
            Self::Degree90 => "90-Degree",
            Self::Degree45 => "45-Degree",
            Self::Round => "Rounded",
            other => {
                panic!("ToParamValue: unhandled CornerStyle variant {other:?} — add a match arm")
            }
        }
        .to_owned()
    }
}

// ComponentCollisionCheckMode serializes as integer string (not string name).
impl_enum_param_value!(ComponentCollisionCheckMode, TestpointValid);

// ComponentOrientationFlags serializes as a signed decimal integer bitmask
// in the "AllowedRotations" parameter (from IPCB_ComponentRotationsRule).
impl FromParamValue for ComponentOrientationFlags {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        let raw = i32::from_param_value(key, value)?;
        Ok(Self(raw))
    }
}

impl ToParamValue for ComponentOrientationFlags {
    fn to_param_value(&self) -> String {
        self.0.to_string()
    }
}

// ObjectClearanceId: helper methods (from_clearance_string / to_clearance_string) are
// defined in altium-format-types. Used by ClearanceMatrix parser below.

// ── MilCoord ────────────────────────────────────────────────────────────────
// A newtype around Coord for parameters serialized as "7mil", "11.811mil", etc.
// Used exclusively by DRC rule/violation parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct MilCoord(pub Coord);

impl FromParamValue for MilCoord {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        let trimmed = value.strip_suffix("mil").unwrap_or(value);
        let normalized = trimmed.replace(',', ".");
        let mils: f64 = normalized.parse().map_err(|e: std::num::ParseFloatError| {
            AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: format!("cannot parse '{}' as mil value: {}", value, e),
            }
        })?;
        Ok(MilCoord(Coord::from_mils_f64(mils)))
    }
}

impl ToParamValue for MilCoord {
    fn to_param_value(&self) -> String {
        let mils = self.0.to_mils();
        // Altium formats with up to 4 decimal places, stripping trailing zeros.
        let s = format!("{:.4}", mils);
        let s = s.trim_end_matches('0');
        let s = s.trim_end_matches('.');
        format!("{s}mil")
    }
}

// ── SignalValue ─────────────────────────────────────────────────────────────
// A float stored as text by the signal-integrity rule kinds and SupplyNets
// (AD 26 reads these keys as strings, rule-params.tsv). The corpus stores a sign
// slot (space or '-') and four significant digits: " 51.00", "-1.000",
// " 0.000", " 12.00", " 1.500". Parsing is strict (trimmed, finite f64).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct SignalValue(pub f64);

impl FromParamValue for SignalValue {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        let v = f64::from_param_value(key, value)?;
        if !v.is_finite() {
            return Err(AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: format!("non-finite value '{value}'"),
            });
        }
        Ok(SignalValue(v))
    }
}

impl ToParamValue for SignalValue {
    fn to_param_value(&self) -> String {
        let a = self.0.abs();
        let int_digits = if a < 1.0 {
            1
        } else {
            a.log10().floor() as i32 + 1
        };
        let decimals = (4 - int_digits).max(0) as usize;
        let sign = if self.0 < 0.0 { '-' } else { ' ' };
        format!("{sign}{a:.decimals$}")
    }
}

// ── ExactMilCoord ───────────────────────────────────────────────────────────
// A length stored as "<decimal>mil" (e.g. "1901.8898mil", "-3mil", "0mil") in
// Polygons6 and Board6 vertex records. One internal unit is 1/10,000 mil, so a
// value with at most four fractional digits is an exact integer number of units.
// Parsed with integer arithmetic (no f64 rounding); the "mil" suffix is required,
// more than four fractional digits and out-of-range values are errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ExactMilCoord(pub Coord);

/// Internal units per mil.
const UNITS_PER_MIL: i64 = 10_000;
/// Maximum number of fractional mil digits that are an exact number of units.
const MAX_MIL_FRACTION_DIGITS: usize = 4;

fn parse_exact_mils(value: &str) -> Option<Coord> {
    let number = value.strip_suffix("mil")?;
    let (negative, digits) = match number.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, number),
    };
    let (int_part, frac_part) = match digits.split_once('.') {
        Some((i, f)) => (i, f),
        None => (digits, ""),
    };
    if int_part.is_empty()
        || !int_part.bytes().all(|b| b.is_ascii_digit())
        || !frac_part.bytes().all(|b| b.is_ascii_digit())
        || frac_part.len() > MAX_MIL_FRACTION_DIGITS
        || (digits.contains('.') && frac_part.is_empty())
    {
        return None;
    }
    let int_units = int_part.parse::<i64>().ok()?.checked_mul(UNITS_PER_MIL)?;
    let frac_units = if frac_part.is_empty() {
        0
    } else {
        let scale = 10i64.pow((MAX_MIL_FRACTION_DIGITS - frac_part.len()) as u32);
        frac_part.parse::<i64>().ok()? * scale
    };
    let magnitude = int_units.checked_add(frac_units)?;
    let units = if negative { -magnitude } else { magnitude };
    i32::try_from(units).ok().map(Coord::from_internal)
}

impl FromParamValue for ExactMilCoord {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        parse_exact_mils(value).map(ExactMilCoord).ok_or_else(|| {
            AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: format!(
                    "expected '<decimal>mil' with at most {MAX_MIL_FRACTION_DIGITS} fractional \
                     digits within the i32 unit range, got {value:?}"
                ),
            }
        })
    }
}

impl ToParamValue for ExactMilCoord {
    fn to_param_value(&self) -> String {
        let units = i64::from(self.0.to_internal());
        let sign = if units < 0 { "-" } else { "" };
        let magnitude = units.abs();
        let int_part = magnitude / UNITS_PER_MIL;
        let frac_part = magnitude % UNITS_PER_MIL;
        if frac_part == 0 {
            format!("{sign}{int_part}mil")
        } else {
            let frac = format!("{frac_part:04}");
            format!("{sign}{int_part}.{}mil", frac.trim_end_matches('0'))
        }
    }
}

// ── ClearanceMatrix ─────────────────────────────────────────────────────────
// Sparse symmetric clearance matrix indexed by ObjectClearanceId pairs.
// Serialized as semicolon-delimited "Type1-Type2:value" entries.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ClearanceMatrix {
    entries: indexmap::IndexMap<(ObjectClearanceId, ObjectClearanceId), Coord>,
}

impl ClearanceMatrix {
    fn normalize(
        a: ObjectClearanceId,
        b: ObjectClearanceId,
    ) -> (ObjectClearanceId, ObjectClearanceId) {
        if (a as u8) <= (b as u8) {
            (a, b)
        } else {
            (b, a)
        }
    }
}

impl ClearanceMatrix {
    /// The stored pairs, in stored order (each pair once, lower id first).
    pub(crate) fn entries(
        &self,
    ) -> impl Iterator<Item = (ObjectClearanceId, ObjectClearanceId, Coord)> + '_ {
        self.entries.iter().map(|(&(a, b), &gap)| (a, b, gap))
    }
}

impl FromParamValue for ClearanceMatrix {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        let mut matrix = ClearanceMatrix::default();
        if value.is_empty() {
            return Ok(matrix);
        }
        for entry in value.split(';') {
            if entry.is_empty() {
                return Err(AltiumFormatError::InvalidParamValue {
                    key: key.to_owned(),
                    detail: format!("empty clearance entry in {value:?}"),
                });
            }
            let (pair_str, val_str) =
                entry
                    .split_once(':')
                    .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                        key: key.to_owned(),
                        detail: format!("expected 'Type1-Type2:value', got {entry:?}"),
                    })?;
            let (type1_str, type2_str) =
                pair_str
                    .split_once('-')
                    .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                        key: key.to_owned(),
                        detail: format!("expected 'Type1-Type2', got {pair_str:?}"),
                    })?;
            let type1 = ObjectClearanceId::from_clearance_string(type1_str).map_err(|_| {
                AltiumFormatError::InvalidParamValue {
                    key: key.to_owned(),
                    detail: format!("unknown clearance object type: {type1_str:?}"),
                }
            })?;
            let type2 = ObjectClearanceId::from_clearance_string(type2_str).map_err(|_| {
                AltiumFormatError::InvalidParamValue {
                    key: key.to_owned(),
                    detail: format!("unknown clearance object type: {type2_str:?}"),
                }
            })?;
            let raw: i32 = Some(val_str)
                .filter(|v| !v.starts_with('+'))
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                    key: key.to_owned(),
                    detail: format!("invalid clearance value: {val_str:?}"),
                })?;
            let norm = Self::normalize(type1, type2);
            if matrix
                .entries
                .insert(norm, Coord::from_internal(raw))
                .is_some()
            {
                return Err(AltiumFormatError::InvalidParamValue {
                    key: key.to_owned(),
                    detail: format!("clearance pair {pair_str:?} is given twice"),
                });
            }
        }
        Ok(matrix)
    }
}

impl ToParamValue for ClearanceMatrix {
    fn to_param_value(&self) -> String {
        self.entries
            .iter()
            .map(|(&(a, b), &v)| {
                format!(
                    "{}-{}:{}",
                    a.to_clearance_string(),
                    b.to_clearance_string(),
                    v.to_internal()
                )
            })
            .collect::<Vec<_>>()
            .join(";")
    }
}

// Angle value that serializes with exactly 3 decimal places (matching Altium's N3 format).
// C#: StrUtils.DoubleToString(value, "N3") always produces "180.000", "45.000", etc.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SchAngle(pub f64);

impl ToParamValue for SchAngle {
    fn to_param_value(&self) -> String {
        format!("{:.3}", self.0)
    }
}

impl FromParamValue for SchAngle {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        let v: f64 = value.parse().map_err(|e: std::num::ParseFloatError| {
            AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: format!("invalid angle: {e}"),
            }
        })?;
        // Rust's f64 parser accepts "NaN"/"inf"/"infinity", which Altium
        // never writes. Rejecting them here keeps every stored angle finite:
        // a NaN would break typed equality (PartialEq) everywhere, and
        // serde_json silently serializes non-finite floats as null,
        // producing an undecodable state payload instead of an error.
        if !v.is_finite() {
            return Err(AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: format!("invalid angle: non-finite value {value:?}"),
            });
        }
        Ok(SchAngle(v))
    }
}

impl std::fmt::Display for SchAngle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Format with up to 4 decimal places, stripping trailing zeros.
        let s = format!("{:.4}", self.0);
        let s = s.trim_end_matches('0');
        let s = s.trim_end_matches('.');
        write!(f, "{}", s)
    }
}

impl Default for SchAngle {
    fn default() -> Self {
        SchAngle(0.0)
    }
}

// UniqueId is stored as an 8-char uppercase alpha string in parameter values.
impl FromParamValue for UniqueId {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        value
            .parse::<UniqueId>()
            .map_err(|e| AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: e.to_string(),
            })
    }
}

impl ToParamValue for UniqueId {
    fn to_param_value(&self) -> String {
        self.as_str().to_owned()
    }
}

// SheetSymbolType is serialized as display strings in SchDoc text records.
// C# mapping (SchDataUtils.{StringTo,ToString}SheetSymbolType):
// Normal | Device Sheet | Design Item
impl FromParamValue for SheetSymbolType {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        if value.eq_ignore_ascii_case("Normal") {
            return Ok(SheetSymbolType::Normal);
        }
        if value.eq_ignore_ascii_case("Device Sheet") {
            return Ok(SheetSymbolType::DeviceSheet);
        }
        if value.eq_ignore_ascii_case("Design Item") {
            return Ok(SheetSymbolType::DesignItem);
        }

        // Accept numeric fallback for robustness in mixed-version files.
        if let Ok(raw) = value.parse::<u8>() {
            return SheetSymbolType::try_from(raw).map_err(|e| {
                AltiumFormatError::InvalidParamValue {
                    key: key.to_owned(),
                    detail: e.to_string(),
                }
            });
        }

        Err(AltiumFormatError::InvalidParamValue {
            key: key.to_owned(),
            detail: format!("expected one of Normal/Device Sheet/Design Item or u8, got {value:?}"),
        })
    }
}

impl ToParamValue for SheetSymbolType {
    fn to_param_value(&self) -> String {
        match self {
            SheetSymbolType::Normal => "Normal".to_owned(),
            SheetSymbolType::DeviceSheet => "Device Sheet".to_owned(),
            SheetSymbolType::DesignItem => "Design Item".to_owned(),
            _ => (*self as u8).to_string(),
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn clearance_matrix_keeps_order_and_rejects_repeated_pairs() {
        let m = ClearanceMatrix::from_param_value(
            "OBJECTCLEARANCES",
            "ClearanceObj_Track-ClearanceObj_Arc:50000;ClearanceObj_Hole-ClearanceObj_Hole:0",
        )
        .unwrap();
        let entries: Vec<_> = m.entries().collect();
        assert_eq!(
            entries,
            [
                (
                    ObjectClearanceId::Arc,
                    ObjectClearanceId::Track,
                    Coord::from_internal(50000)
                ),
                (
                    ObjectClearanceId::Hole,
                    ObjectClearanceId::Hole,
                    Coord::from_internal(0)
                ),
            ]
        );
        let err = ClearanceMatrix::from_param_value(
            "OBJECTCLEARANCES",
            "ClearanceObj_Track-ClearanceObj_Arc:1;ClearanceObj_Arc-ClearanceObj_Track:2",
        )
        .unwrap_err();
        assert!(err.to_string().contains("twice"), "{err}");
        for bad in [
            "ClearanceObj_Track-ClearanceObj_Arc:1;",
            "ClearanceObj_Track-ClearanceObj_Arc:1;;ClearanceObj_Via-ClearanceObj_Via:1",
            "ClearanceObj_Track-ClearanceObj_Arc:+1",
            "ClearanceObj_Track-ClearanceObj_Sheet:1",
        ] {
            assert!(
                ClearanceMatrix::from_param_value("OBJECTCLEARANCES", bad).is_err(),
                "{bad}"
            );
        }
    }

    use super::*;

    #[test]
    fn signal_value_round_trips_corpus_forms() {
        for raw in [" 51.00", "-1.000", " 0.000", " 12.00", " 1.500", " 4.000"] {
            let v = SignalValue::from_param_value("MAXIMUM", raw).unwrap();
            assert_eq!(v.to_param_value(), raw, "value {raw:?}");
        }
        assert!(SignalValue::from_param_value("MAXIMUM", "abc").is_err());
        assert!(SignalValue::from_param_value("MAXIMUM", "inf").is_err());
    }

    #[test]
    fn exact_mil_coord_parses_corpus_forms_exactly() {
        let cases = [
            ("1901.8898mil", 19_018_898),
            ("5836.89mil", 58_368_900),
            ("0mil", 0),
            ("19.685mil", 196_850),
            ("-3mil", -30_000),
            ("-0.0001mil", -1),
            ("214748.3647mil", i32::MAX),
            ("-214748.3648mil", i32::MIN),
        ];
        for (text, units) in cases {
            let parsed = ExactMilCoord::from_param_value("VX0", text).unwrap();
            assert_eq!(parsed.0.to_internal(), units, "{text}");
        }
    }

    #[test]
    fn exact_mil_coord_rejects_everything_else() {
        for text in [
            "1901",
            "1901.8898",
            "1901.88981mil",
            "214748.3648mil",
            "1.mil",
            ".5mil",
            "mil",
            "1,5mil",
            " 1mil",
            "1e3mil",
            "1901.8898mm",
        ] {
            let err = ExactMilCoord::from_param_value("VX0", text).unwrap_err();
            assert!(err.to_string().contains("VX0"), "{text}: {err}");
        }
    }

    #[test]
    fn exact_mil_coord_writes_altium_form() {
        for (units, text) in [
            (19_018_898, "1901.8898mil"),
            (58_368_900, "5836.89mil"),
            (0, "0mil"),
            (-30_000, "-3mil"),
            (-1, "-0.0001mil"),
        ] {
            assert_eq!(
                ExactMilCoord(Coord::from_internal(units)).to_param_value(),
                text
            );
            let back = ExactMilCoord::from_param_value("VX0", text).unwrap();
            assert_eq!(back.0.to_internal(), units);
        }
    }

    #[test]
    fn sch_angle_rejects_non_finite_values_at_the_parse_boundary() {
        for value in ["NaN", "nan", "inf", "-inf", "infinity", "Infinity"] {
            let error = SchAngle::from_param_value("STARTANGLE", value)
                .unwrap_err()
                .to_string();
            assert!(error.contains("non-finite"), "{value}: {error}");
        }
        assert_eq!(
            SchAngle::from_param_value("STARTANGLE", "180.000").unwrap(),
            SchAngle(180.0)
        );
    }
}

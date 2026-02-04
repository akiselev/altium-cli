//! PCB design rule types for Altium PCB files.
//!
//! Design rules define constraints for PCB layout such as clearance,
//! trace width, routing layers, etc.
//!
//! **DEPRECATED**: Use `v2::pcb::PcbRuleV2` with `v2::pcb::io` instead.

use std::fmt;
use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::error::Result;
use crate::io::reader::decode_windows_1252;
use crate::io::writer::encode_windows_1252;
use crate::types::{Coord, ParameterCollection};

/// Design rule kind identifiers.
///
/// These correspond to Altium's internal rule type IDs.
/// Based on DXP API TRuleKind enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u16)]
pub enum RuleKind {
    /// Clearance constraints between objects.
    #[default]
    Clearance = 0,
    /// Parallel segment constraints for differential pairs.
    ParallelSegment = 1,
    /// Trace width constraints.
    Width = 2,
    /// Maximum/minimum trace length.
    Length = 3,
    /// Matched length constraints for bus signals.
    MatchedLengths = 4,
    /// Daisy chain stub length limits.
    StubLength = 5,
    /// Connection style to power planes.
    PlaneConnect = 6,
    /// Routing topology rules (e.g., daisy chain, star).
    RoutingTopology = 7,
    /// Priority of routing for specific nets.
    RoutingPriority = 8,
    /// Allowed routing layers.
    RoutingLayers = 9,
    /// Corner styles (45 deg, 90 deg, etc.).
    RoutingCorners = 10,
    /// Via types and sizes allowed.
    RoutingVias = 11,
    /// Clearance to split planes.
    PlaneClearance = 12,
    /// Solder mask expansion settings.
    SolderMaskExpansion = 13,
    /// Paste mask expansion settings.
    PasteMaskExpansion = 14,
    /// Short circuit allowance.
    ShortCircuit = 15,
    /// Checks for unrouted nets.
    UnRoutedNet = 16,
    /// Via placement under SMD pads.
    ViasUnderSMD = 17,
    /// Maximum via count per net.
    MaximumViaCount = 18,
    /// Minimum annular ring width.
    MinimumAnnularRing = 19,
    /// Polygon connection style (Relief/Direct).
    PolygonConnect = 20,
    /// Acute angle constraints.
    AcuteAngle = 21,
    /// Room definition constraints.
    RoomDefinition = 22,
    /// SMD to corner clearance.
    SMDToCorner = 23,
    /// Clearance between 3D bodies/courtyards.
    ComponentClearance = 24,
    /// Allowed component orientations.
    ComponentOrientations = 25,
    /// Permitted layers for objects.
    PermittedLayers = 26,
    /// Nets to ignore in DRC.
    NetsToIgnore = 27,
    /// Signal stimulus for simulation.
    SignalStimulus = 28,
    /// Overshoot on falling edge (signal integrity).
    OvershootFalling = 29,
    /// Overshoot on rising edge (signal integrity).
    OvershootRising = 30,
    /// Undershoot on falling edge (signal integrity).
    UndershootFalling = 31,
    /// Undershoot on rising edge (signal integrity).
    UndershootRising = 32,
    /// Maximum/minimum impedance (signal integrity).
    MaxMinImpedance = 33,
    /// Signal top voltage value (signal integrity).
    SignalTopValue = 34,
    /// Signal base voltage value (signal integrity).
    SignalBaseValue = 35,
    /// Flight time on rising edge (signal integrity).
    FlightTimeRising = 36,
    /// Flight time on falling edge (signal integrity).
    FlightTimeFalling = 37,
    /// Layer stack definition.
    LayerStack = 38,
    /// Maximum slope on rising edge (signal integrity).
    SlopeRising = 39,
    /// Maximum slope on falling edge (signal integrity).
    SlopeFalling = 40,
    /// Supply net definitions.
    SupplyNets = 41,
    /// Allowed hole size range.
    HoleSize = 42,
    /// Testpoint style for fabrication.
    TestPointStyle = 43,
    /// Testpoint usage settings.
    TestPointUsage = 44,
    /// Unconnected pin check.
    UnconnectedPin = 45,
    /// SMD to plane clearance.
    SMDToPlane = 46,
    /// SMD neck-down settings.
    SMDNeckDown = 47,
    /// Drill pair definitions.
    LayerPairs = 48,
    /// Fanout settings for BGAs/LCCs.
    FanoutControl = 49,
    /// Component height checks.
    Height = 50,
    /// Differential pair routing settings.
    DiffPairsRouting = 51,
    /// Minimum clearance between holes.
    HoleToHoleClearance = 52,
    /// Minimum solder mask sliver width.
    MinimumSolderMaskSliver = 53,
    /// Silkscreen to mask clearance.
    SilkToSolderMaskClearance = 54,
    /// Silkscreen to silkscreen clearance.
    SilkToSilkClearance = 55,
    /// Checks for stub tracks (antennae).
    NetAntennae = 56,
    /// Testpoint usage for assembly.
    AssemblyTestpoint = 57,
    /// Assembly testpoint usage settings.
    AssemblyTestPointUsage = 58,
    /// Silkscreen to board region clearance.
    SilkToBoardRegionClearance = 59,
    /// Checks for unpoured polygons.
    UnpouredPolygon = 62,
    /// Unknown rule kind.
    Unknown(u16),
}

impl RuleKind {
    /// Create a RuleKind from its numeric ID.
    pub fn from_id(id: u16) -> Self {
        match id {
            0 => RuleKind::Clearance,
            1 => RuleKind::ParallelSegment,
            2 => RuleKind::Width,
            3 => RuleKind::Length,
            4 => RuleKind::MatchedLengths,
            5 => RuleKind::StubLength,
            6 => RuleKind::PlaneConnect,
            7 => RuleKind::RoutingTopology,
            8 => RuleKind::RoutingPriority,
            9 => RuleKind::RoutingLayers,
            10 => RuleKind::RoutingCorners,
            11 => RuleKind::RoutingVias,
            12 => RuleKind::PlaneClearance,
            13 => RuleKind::SolderMaskExpansion,
            14 => RuleKind::PasteMaskExpansion,
            15 => RuleKind::ShortCircuit,
            16 => RuleKind::UnRoutedNet,
            17 => RuleKind::ViasUnderSMD,
            18 => RuleKind::MaximumViaCount,
            19 => RuleKind::MinimumAnnularRing,
            20 => RuleKind::PolygonConnect,
            21 => RuleKind::AcuteAngle,
            22 => RuleKind::RoomDefinition,
            23 => RuleKind::SMDToCorner,
            24 => RuleKind::ComponentClearance,
            25 => RuleKind::ComponentOrientations,
            26 => RuleKind::PermittedLayers,
            27 => RuleKind::NetsToIgnore,
            28 => RuleKind::SignalStimulus,
            29 => RuleKind::OvershootFalling,
            30 => RuleKind::OvershootRising,
            31 => RuleKind::UndershootFalling,
            32 => RuleKind::UndershootRising,
            33 => RuleKind::MaxMinImpedance,
            34 => RuleKind::SignalTopValue,
            35 => RuleKind::SignalBaseValue,
            36 => RuleKind::FlightTimeRising,
            37 => RuleKind::FlightTimeFalling,
            38 => RuleKind::LayerStack,
            39 => RuleKind::SlopeRising,
            40 => RuleKind::SlopeFalling,
            41 => RuleKind::SupplyNets,
            42 => RuleKind::HoleSize,
            43 => RuleKind::TestPointStyle,
            44 => RuleKind::TestPointUsage,
            45 => RuleKind::UnconnectedPin,
            46 => RuleKind::SMDToPlane,
            47 => RuleKind::SMDNeckDown,
            48 => RuleKind::LayerPairs,
            49 => RuleKind::FanoutControl,
            50 => RuleKind::Height,
            51 => RuleKind::DiffPairsRouting,
            52 => RuleKind::HoleToHoleClearance,
            53 => RuleKind::MinimumSolderMaskSliver,
            54 => RuleKind::SilkToSolderMaskClearance,
            55 => RuleKind::SilkToSilkClearance,
            56 => RuleKind::NetAntennae,
            57 => RuleKind::AssemblyTestpoint,
            58 => RuleKind::AssemblyTestPointUsage,
            59 => RuleKind::SilkToBoardRegionClearance,
            62 => RuleKind::UnpouredPolygon,
            other => RuleKind::Unknown(other),
        }
    }

    /// Get the numeric ID of this rule kind.
    pub fn to_id(self) -> u16 {
        match self {
            RuleKind::Clearance => 0,
            RuleKind::ParallelSegment => 1,
            RuleKind::Width => 2,
            RuleKind::Length => 3,
            RuleKind::MatchedLengths => 4,
            RuleKind::StubLength => 5,
            RuleKind::PlaneConnect => 6,
            RuleKind::RoutingTopology => 7,
            RuleKind::RoutingPriority => 8,
            RuleKind::RoutingLayers => 9,
            RuleKind::RoutingCorners => 10,
            RuleKind::RoutingVias => 11,
            RuleKind::PlaneClearance => 12,
            RuleKind::SolderMaskExpansion => 13,
            RuleKind::PasteMaskExpansion => 14,
            RuleKind::ShortCircuit => 15,
            RuleKind::UnRoutedNet => 16,
            RuleKind::ViasUnderSMD => 17,
            RuleKind::MaximumViaCount => 18,
            RuleKind::MinimumAnnularRing => 19,
            RuleKind::PolygonConnect => 20,
            RuleKind::AcuteAngle => 21,
            RuleKind::RoomDefinition => 22,
            RuleKind::SMDToCorner => 23,
            RuleKind::ComponentClearance => 24,
            RuleKind::ComponentOrientations => 25,
            RuleKind::PermittedLayers => 26,
            RuleKind::NetsToIgnore => 27,
            RuleKind::SignalStimulus => 28,
            RuleKind::OvershootFalling => 29,
            RuleKind::OvershootRising => 30,
            RuleKind::UndershootFalling => 31,
            RuleKind::UndershootRising => 32,
            RuleKind::MaxMinImpedance => 33,
            RuleKind::SignalTopValue => 34,
            RuleKind::SignalBaseValue => 35,
            RuleKind::FlightTimeRising => 36,
            RuleKind::FlightTimeFalling => 37,
            RuleKind::LayerStack => 38,
            RuleKind::SlopeRising => 39,
            RuleKind::SlopeFalling => 40,
            RuleKind::SupplyNets => 41,
            RuleKind::HoleSize => 42,
            RuleKind::TestPointStyle => 43,
            RuleKind::TestPointUsage => 44,
            RuleKind::UnconnectedPin => 45,
            RuleKind::SMDToPlane => 46,
            RuleKind::SMDNeckDown => 47,
            RuleKind::LayerPairs => 48,
            RuleKind::FanoutControl => 49,
            RuleKind::Height => 50,
            RuleKind::DiffPairsRouting => 51,
            RuleKind::HoleToHoleClearance => 52,
            RuleKind::MinimumSolderMaskSliver => 53,
            RuleKind::SilkToSolderMaskClearance => 54,
            RuleKind::SilkToSilkClearance => 55,
            RuleKind::NetAntennae => 56,
            RuleKind::AssemblyTestpoint => 57,
            RuleKind::AssemblyTestPointUsage => 58,
            RuleKind::SilkToBoardRegionClearance => 59,
            RuleKind::UnpouredPolygon => 62,
            RuleKind::Unknown(id) => id,
        }
    }

    /// Get the name of this rule kind as it appears in Altium files.
    pub fn name(&self) -> &'static str {
        match self {
            RuleKind::Clearance => "Clearance",
            RuleKind::ParallelSegment => "ParallelSegment",
            RuleKind::Width => "Width",
            RuleKind::Length => "Length",
            RuleKind::MatchedLengths => "MatchedLengths",
            RuleKind::StubLength => "StubLength",
            RuleKind::PlaneConnect => "PlaneConnect",
            RuleKind::RoutingTopology => "RoutingTopology",
            RuleKind::RoutingPriority => "RoutingPriority",
            RuleKind::RoutingLayers => "RoutingLayers",
            RuleKind::RoutingCorners => "RoutingCorners",
            RuleKind::RoutingVias => "RoutingVias",
            RuleKind::PlaneClearance => "PlaneClearance",
            RuleKind::SolderMaskExpansion => "SolderMaskExpansion",
            RuleKind::PasteMaskExpansion => "PasteMaskExpansion",
            RuleKind::ShortCircuit => "ShortCircuit",
            RuleKind::UnRoutedNet => "UnRoutedNet",
            RuleKind::ViasUnderSMD => "ViasUnderSMD",
            RuleKind::MaximumViaCount => "MaximumViaCount",
            RuleKind::MinimumAnnularRing => "MinimumAnnularRing",
            RuleKind::PolygonConnect => "PolygonConnect",
            RuleKind::AcuteAngle => "AcuteAngle",
            RuleKind::RoomDefinition => "RoomDefinition",
            RuleKind::SMDToCorner => "SMDToCorner",
            RuleKind::ComponentClearance => "ComponentClearance",
            RuleKind::ComponentOrientations => "ComponentOrientations",
            RuleKind::PermittedLayers => "PermittedLayers",
            RuleKind::NetsToIgnore => "NetsToIgnore",
            RuleKind::SignalStimulus => "SignalStimulus",
            RuleKind::OvershootFalling => "OvershootFalling",
            RuleKind::OvershootRising => "OvershootRising",
            RuleKind::UndershootFalling => "UndershootFalling",
            RuleKind::UndershootRising => "UndershootRising",
            RuleKind::MaxMinImpedance => "MaxMinImpedance",
            RuleKind::SignalTopValue => "SignalTopValue",
            RuleKind::SignalBaseValue => "SignalBaseValue",
            RuleKind::FlightTimeRising => "FlightTimeRising",
            RuleKind::FlightTimeFalling => "FlightTimeFalling",
            RuleKind::LayerStack => "LayerStack",
            RuleKind::SlopeRising => "SlopeRising",
            RuleKind::SlopeFalling => "SlopeFalling",
            RuleKind::SupplyNets => "SupplyNets",
            RuleKind::HoleSize => "HoleSize",
            RuleKind::TestPointStyle => "Testpoint",
            RuleKind::TestPointUsage => "TestPointUsage",
            RuleKind::UnconnectedPin => "UnConnectedPin",
            RuleKind::SMDToPlane => "SMDToPlane",
            RuleKind::SMDNeckDown => "SMDNeckDown",
            RuleKind::LayerPairs => "LayerPairs",
            RuleKind::FanoutControl => "FanoutControl",
            RuleKind::Height => "Height",
            RuleKind::DiffPairsRouting => "DiffPairsRouting",
            RuleKind::HoleToHoleClearance => "HoleToHoleClearance",
            RuleKind::MinimumSolderMaskSliver => "MinimumSolderMaskSliver",
            RuleKind::SilkToSolderMaskClearance => "SilkToSolderMaskClearance",
            RuleKind::SilkToSilkClearance => "SilkToSilkClearance",
            RuleKind::NetAntennae => "NetAntennae",
            RuleKind::AssemblyTestpoint => "AssemblyTestpoint",
            RuleKind::AssemblyTestPointUsage => "AssemblyTestPointUsage",
            RuleKind::SilkToBoardRegionClearance => "SilkToBoardRegionClearance",
            RuleKind::UnpouredPolygon => "UnpouredPolygon",
            RuleKind::Unknown(_) => "Unknown",
        }
    }

    /// Parse a rule kind from its name string.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Clearance" => Some(RuleKind::Clearance),
            "ParallelSegment" => Some(RuleKind::ParallelSegment),
            "Width" => Some(RuleKind::Width),
            "Length" => Some(RuleKind::Length),
            "MatchedLengths" => Some(RuleKind::MatchedLengths),
            "StubLength" => Some(RuleKind::StubLength),
            "PlaneConnect" => Some(RuleKind::PlaneConnect),
            "RoutingTopology" => Some(RuleKind::RoutingTopology),
            "RoutingPriority" => Some(RuleKind::RoutingPriority),
            "RoutingLayers" => Some(RuleKind::RoutingLayers),
            "RoutingCorners" => Some(RuleKind::RoutingCorners),
            "RoutingVias" => Some(RuleKind::RoutingVias),
            "PlaneClearance" => Some(RuleKind::PlaneClearance),
            "SolderMaskExpansion" => Some(RuleKind::SolderMaskExpansion),
            "PasteMaskExpansion" => Some(RuleKind::PasteMaskExpansion),
            "ShortCircuit" => Some(RuleKind::ShortCircuit),
            "UnRoutedNet" => Some(RuleKind::UnRoutedNet),
            "ViasUnderSMD" => Some(RuleKind::ViasUnderSMD),
            "MaximumViaCount" => Some(RuleKind::MaximumViaCount),
            "MinimumAnnularRing" => Some(RuleKind::MinimumAnnularRing),
            "PolygonConnect" => Some(RuleKind::PolygonConnect),
            "AcuteAngle" => Some(RuleKind::AcuteAngle),
            "RoomDefinition" => Some(RuleKind::RoomDefinition),
            "SMDToCorner" => Some(RuleKind::SMDToCorner),
            "ComponentClearance" => Some(RuleKind::ComponentClearance),
            "ComponentOrientations" => Some(RuleKind::ComponentOrientations),
            "PermittedLayers" => Some(RuleKind::PermittedLayers),
            "NetsToIgnore" => Some(RuleKind::NetsToIgnore),
            "SignalStimulus" => Some(RuleKind::SignalStimulus),
            "OvershootFalling" => Some(RuleKind::OvershootFalling),
            "OvershootRising" => Some(RuleKind::OvershootRising),
            "UndershootFalling" => Some(RuleKind::UndershootFalling),
            "UndershootRising" => Some(RuleKind::UndershootRising),
            "MaxMinImpedance" => Some(RuleKind::MaxMinImpedance),
            "SignalTopValue" => Some(RuleKind::SignalTopValue),
            "SignalBaseValue" => Some(RuleKind::SignalBaseValue),
            "FlightTimeRising" => Some(RuleKind::FlightTimeRising),
            "FlightTimeFalling" => Some(RuleKind::FlightTimeFalling),
            "LayerStack" => Some(RuleKind::LayerStack),
            "SlopeRising" => Some(RuleKind::SlopeRising),
            "SlopeFalling" => Some(RuleKind::SlopeFalling),
            "SupplyNets" => Some(RuleKind::SupplyNets),
            "HoleSize" => Some(RuleKind::HoleSize),
            "Testpoint" | "TestPointStyle" => Some(RuleKind::TestPointStyle),
            "TestPointUsage" => Some(RuleKind::TestPointUsage),
            "UnConnectedPin" | "UnconnectedPin" => Some(RuleKind::UnconnectedPin),
            "SMDToPlane" => Some(RuleKind::SMDToPlane),
            "SMDNeckDown" => Some(RuleKind::SMDNeckDown),
            "LayerPairs" => Some(RuleKind::LayerPairs),
            "FanoutControl" => Some(RuleKind::FanoutControl),
            "Height" => Some(RuleKind::Height),
            "DiffPairsRouting" => Some(RuleKind::DiffPairsRouting),
            "HoleToHoleClearance" => Some(RuleKind::HoleToHoleClearance),
            "MinimumSolderMaskSliver" => Some(RuleKind::MinimumSolderMaskSliver),
            "SilkToSolderMaskClearance" => Some(RuleKind::SilkToSolderMaskClearance),
            "SilkToSilkClearance" => Some(RuleKind::SilkToSilkClearance),
            "NetAntennae" => Some(RuleKind::NetAntennae),
            "AssemblyTestpoint" => Some(RuleKind::AssemblyTestpoint),
            "AssemblyTestPointUsage" => Some(RuleKind::AssemblyTestPointUsage),
            "SilkToBoardRegionClearance" => Some(RuleKind::SilkToBoardRegionClearance),
            "UnpouredPolygon" => Some(RuleKind::UnpouredPolygon),
            _ => None,
        }
    }
}

impl fmt::Display for RuleKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuleKind::Unknown(id) => write!(f, "Unknown({})", id),
            _ => write!(f, "{}", self.name()),
        }
    }
}

/// Net scope for rule application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetScope {
    #[default]
    AnyNet,
    InNetClass,
    Net,
    FromTo,
    Board,
    DifferentialPair,
}

impl NetScope {
    pub fn parse(s: &str) -> Self {
        match s {
            "AnyNet" => NetScope::AnyNet,
            "InNetClass" => NetScope::InNetClass,
            "Net" => NetScope::Net,
            "FromTo" => NetScope::FromTo,
            "Board" => NetScope::Board,
            "DifferentialPair" => NetScope::DifferentialPair,
            _ => NetScope::AnyNet,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            NetScope::AnyNet => "AnyNet",
            NetScope::InNetClass => "InNetClass",
            NetScope::Net => "Net",
            NetScope::FromTo => "FromTo",
            NetScope::Board => "Board",
            NetScope::DifferentialPair => "DifferentialPair",
        }
    }
}

/// Layer kind for rule application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayerKind {
    #[default]
    SameLayer,
    AnyLayer,
    OnLayer,
    InLayerClass,
}

impl LayerKind {
    pub fn parse(s: &str) -> Self {
        match s {
            "SameLayer" => LayerKind::SameLayer,
            "AnyLayer" => LayerKind::AnyLayer,
            "OnLayer" => LayerKind::OnLayer,
            "InLayerClass" => LayerKind::InLayerClass,
            _ => LayerKind::SameLayer,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            LayerKind::SameLayer => "SameLayer",
            LayerKind::AnyLayer => "AnyLayer",
            LayerKind::OnLayer => "OnLayer",
            LayerKind::InLayerClass => "InLayerClass",
        }
    }
}

/// A PCB design rule.
///
/// Design rules contain common metadata plus rule-specific parameters.
/// The parameters are stored in a ParameterCollection to support
/// all rule types and preserve unknown fields for round-tripping.
///
/// **DEPRECATED**: Use `v2::pcb::PcbRuleV2` instead.
#[deprecated(note = "Use v2::pcb::PcbRuleV2")]
#[derive(Debug, Clone)]
pub struct PcbRule {
    /// The kind of rule (Clearance, Width, etc.).
    pub kind: RuleKind,
    /// Rule name (user-defined or default).
    pub name: String,
    /// Whether the rule is enabled.
    pub enabled: bool,
    /// Rule priority (lower is higher priority).
    pub priority: i32,
    /// Rule comment/description.
    pub comment: String,
    /// Unique ID for the rule.
    pub unique_id: String,
    /// Net scope for rule application.
    pub net_scope: NetScope,
    /// Layer kind for rule application.
    pub layer_kind: LayerKind,
    /// First scope expression (e.g., "All", "IsTrack", net name).
    pub scope1_expression: String,
    /// Second scope expression.
    pub scope2_expression: String,
    /// Whether the rule is defined by a logical document.
    pub defined_by_logical_document: bool,
    /// All parameters (including rule-specific ones).
    /// This preserves any unknown parameters for round-tripping.
    pub params: ParameterCollection,
}

impl Default for PcbRule {
    fn default() -> Self {
        PcbRule {
            kind: RuleKind::Clearance,
            name: String::new(),
            enabled: true,
            priority: 1,
            comment: String::new(),
            unique_id: String::new(),
            net_scope: NetScope::AnyNet,
            layer_kind: LayerKind::SameLayer,
            scope1_expression: "All".to_string(),
            scope2_expression: "All".to_string(),
            defined_by_logical_document: false,
            params: ParameterCollection::new(),
        }
    }
}

impl PcbRule {
    /// Create a new rule with the given kind and name.
    pub fn new(kind: RuleKind, name: &str) -> Self {
        PcbRule {
            kind,
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// Read a design rule from a binary stream.
    ///
    /// Format:
    /// - 2 bytes: rule kind ID (u16 LE)
    /// - 4 bytes: data size (u32 LE)
    /// - data_size bytes: pipe-delimited parameters (null-terminated)
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        // Read rule kind ID
        let kind_id = reader.read_u16::<LittleEndian>()?;
        let kind = RuleKind::from_id(kind_id);

        // Read data size
        let data_size = reader.read_u32::<LittleEndian>()? as usize;

        if data_size == 0 {
            return Ok(PcbRule {
                kind,
                ..Default::default()
            });
        }

        // Read parameter data
        let mut data = vec![0u8; data_size];
        reader.read_exact(&mut data)?;

        // Remove null terminator if present
        let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
        let param_str = decode_windows_1252(&data[..end]);

        // Parse parameters
        let params = ParameterCollection::from_string(&param_str);

        // Extract common fields
        let name = params
            .get("NAME")
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        let enabled = params
            .get("ENABLED")
            .map(|v| v.as_bool_or(true))
            .unwrap_or(true);
        let priority = params.get("PRIORITY").map(|v| v.as_int_or(1)).unwrap_or(1);
        let comment = params
            .get("COMMENT")
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        let unique_id = params
            .get("UNIQUEID")
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        let net_scope = params
            .get("NETSCOPE")
            .map(|v| NetScope::parse(v.as_str()))
            .unwrap_or_default();
        let layer_kind = params
            .get("LAYERKIND")
            .map(|v| LayerKind::parse(v.as_str()))
            .unwrap_or_default();
        let scope1_expression = params
            .get("SCOPE1EXPRESSION")
            .map(|v| v.as_str().to_string())
            .unwrap_or_else(|| "All".to_string());
        let scope2_expression = params
            .get("SCOPE2EXPRESSION")
            .map(|v| v.as_str().to_string())
            .unwrap_or_else(|| "All".to_string());
        let defined_by_logical_document = params
            .get("DEFINEDBYLOGICALDOCUMENT")
            .map(|v| v.as_bool_or(false))
            .unwrap_or(false);

        Ok(PcbRule {
            kind,
            name,
            enabled,
            priority,
            comment,
            unique_id,
            net_scope,
            layer_kind,
            scope1_expression,
            scope2_expression,
            defined_by_logical_document,
            params,
        })
    }

    /// Write a design rule to a binary stream.
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Sync common fields back to params
        let mut params = self.params.clone();
        params.add("RULEKIND", self.kind.name());
        params.add("NAME", &self.name);
        params.add("ENABLED", if self.enabled { "TRUE" } else { "FALSE" });
        params.add("PRIORITY", &self.priority.to_string());
        params.add("COMMENT", &self.comment);
        params.add("UNIQUEID", &self.unique_id);
        params.add("NETSCOPE", self.net_scope.to_str());
        params.add("LAYERKIND", self.layer_kind.to_str());
        params.add("SCOPE1EXPRESSION", &self.scope1_expression);
        params.add("SCOPE2EXPRESSION", &self.scope2_expression);
        params.add(
            "DEFINEDBYLOGICALDOCUMENT",
            if self.defined_by_logical_document {
                "TRUE"
            } else {
                "FALSE"
            },
        );

        // Convert to string and encode
        let param_str = params.to_string();
        let mut data = encode_windows_1252(&param_str);
        data.push(0); // Null terminator

        // Write rule kind ID
        writer.write_u16::<LittleEndian>(self.kind.to_id())?;

        // Write data size
        writer.write_u32::<LittleEndian>(data.len() as u32)?;

        // Write parameter data
        writer.write_all(&data)?;

        Ok(())
    }

    /// Get the binary size of this rule.
    pub fn binary_size(&self) -> usize {
        // 2 bytes kind + 4 bytes size + params + null terminator
        let param_str = self.params.to_string();
        6 + param_str.len() + 1
    }

    // ========== Convenience accessors for common rule-specific parameters ==========

    /// Get a clearance value (for Clearance rules).
    pub fn clearance(&self) -> Option<Coord> {
        self.params.get("GAP").and_then(|v| v.as_coord().ok())
    }

    /// Set a clearance value.
    pub fn set_clearance(&mut self, value: Coord) {
        self.params.add_coord("GAP", value);
    }

    /// Get minimum width (for Width rules).
    pub fn min_width(&self) -> Option<Coord> {
        self.params.get("MINWIDTH").and_then(|v| v.as_coord().ok())
    }

    /// Set minimum width.
    pub fn set_min_width(&mut self, value: Coord) {
        self.params.add_coord("MINWIDTH", value);
    }

    /// Get maximum width (for Width rules).
    pub fn max_width(&self) -> Option<Coord> {
        self.params.get("MAXWIDTH").and_then(|v| v.as_coord().ok())
    }

    /// Set maximum width.
    pub fn set_max_width(&mut self, value: Coord) {
        self.params.add_coord("MAXWIDTH", value);
    }

    /// Get preferred width (for Width rules).
    pub fn preferred_width(&self) -> Option<Coord> {
        self.params.get("PREFWIDTH").and_then(|v| v.as_coord().ok())
    }

    /// Set preferred width.
    pub fn set_preferred_width(&mut self, value: Coord) {
        self.params.add_coord("PREFWIDTH", value);
    }

    /// Get minimum hole size (for HoleSize rules).
    pub fn min_hole_size(&self) -> Option<Coord> {
        self.params
            .get("MINHOLESIZE")
            .and_then(|v| v.as_coord().ok())
    }

    /// Set minimum hole size.
    pub fn set_min_hole_size(&mut self, value: Coord) {
        self.params.add_coord("MINHOLESIZE", value);
    }

    /// Get maximum hole size (for HoleSize rules).
    pub fn max_hole_size(&self) -> Option<Coord> {
        self.params
            .get("MAXHOLESIZE")
            .and_then(|v| v.as_coord().ok())
    }

    /// Set maximum hole size.
    pub fn set_max_hole_size(&mut self, value: Coord) {
        self.params.add_coord("MAXHOLESIZE", value);
    }

    /// Get solder mask expansion (for SolderMaskExpansion rules).
    pub fn solder_mask_expansion(&self) -> Option<Coord> {
        self.params.get("EXPANSION").and_then(|v| v.as_coord().ok())
    }

    /// Set solder mask expansion.
    pub fn set_solder_mask_expansion(&mut self, value: Coord) {
        self.params.add_coord("EXPANSION", value);
    }

    /// Get paste mask expansion (for PasteMaskExpansion rules).
    pub fn paste_mask_expansion(&self) -> Option<Coord> {
        self.params.get("EXPANSION").and_then(|v| v.as_coord().ok())
    }

    /// Set paste mask expansion.
    pub fn set_paste_mask_expansion(&mut self, value: Coord) {
        self.params.add_coord("EXPANSION", value);
    }

    // ========== Differential Pair Routing Parameters ==========

    /// Get minimum gap for differential pairs.
    pub fn diff_pair_min_gap(&self) -> Option<Coord> {
        self.params.get("MINLIMIT").and_then(|v| v.as_coord().ok())
    }

    /// Set minimum gap for differential pairs.
    pub fn set_diff_pair_min_gap(&mut self, value: Coord) {
        self.params.add_coord("MINLIMIT", value);
    }

    /// Get maximum gap for differential pairs.
    pub fn diff_pair_max_gap(&self) -> Option<Coord> {
        self.params.get("MAXLIMIT").and_then(|v| v.as_coord().ok())
    }

    /// Set maximum gap for differential pairs.
    pub fn set_diff_pair_max_gap(&mut self, value: Coord) {
        self.params.add_coord("MAXLIMIT", value);
    }

    /// Get preferred/most frequent gap for differential pairs.
    pub fn diff_pair_preferred_gap(&self) -> Option<Coord> {
        self.params
            .get("MOSTFREQGAP")
            .and_then(|v| v.as_coord().ok())
    }

    /// Set preferred gap for differential pairs.
    pub fn set_diff_pair_preferred_gap(&mut self, value: Coord) {
        self.params.add_coord("MOSTFREQGAP", value);
    }

    /// Get maximum uncoupled length for differential pairs.
    pub fn diff_pair_max_uncoupled_length(&self) -> Option<Coord> {
        self.params
            .get("MAXUNCOUPLEDLENGTH")
            .and_then(|v| v.as_coord().ok())
    }

    /// Set maximum uncoupled length for differential pairs.
    pub fn set_diff_pair_max_uncoupled_length(&mut self, value: Coord) {
        self.params.add_coord("MAXUNCOUPLEDLENGTH", value);
    }

    // ========== Via Parameters ==========

    /// Get via minimum diameter (RoutingVias rule).
    pub fn via_min_diameter(&self) -> Option<Coord> {
        self.params.get("MINWIDTH").and_then(|v| v.as_coord().ok())
    }

    /// Set via minimum diameter.
    pub fn set_via_min_diameter(&mut self, value: Coord) {
        self.params.add_coord("MINWIDTH", value);
    }

    /// Get via maximum diameter (RoutingVias rule).
    pub fn via_max_diameter(&self) -> Option<Coord> {
        self.params.get("MAXWIDTH").and_then(|v| v.as_coord().ok())
    }

    /// Set via maximum diameter.
    pub fn set_via_max_diameter(&mut self, value: Coord) {
        self.params.add_coord("MAXWIDTH", value);
    }

    /// Get via preferred diameter (RoutingVias rule).
    pub fn via_preferred_diameter(&self) -> Option<Coord> {
        self.params.get("PREFWIDTH").and_then(|v| v.as_coord().ok())
    }

    /// Set via preferred diameter.
    pub fn set_via_preferred_diameter(&mut self, value: Coord) {
        self.params.add_coord("PREFWIDTH", value);
    }

    // ========== Annular Ring Parameters ==========

    /// Get minimum annular ring (MinimumAnnularRing rule).
    pub fn annular_ring(&self) -> Option<Coord> {
        self.params
            .get("ANNULARRING")
            .and_then(|v| v.as_coord().ok())
    }

    /// Set minimum annular ring.
    pub fn set_annular_ring(&mut self, value: Coord) {
        self.params.add_coord("ANNULARRING", value);
    }

    // ========== Hole Clearance Parameters ==========

    /// Get hole-to-hole clearance (HoleToHoleClearance rule).
    pub fn hole_to_hole_clearance(&self) -> Option<Coord> {
        self.params.get("GAP").and_then(|v| v.as_coord().ok())
    }

    /// Set hole-to-hole clearance.
    pub fn set_hole_to_hole_clearance(&mut self, value: Coord) {
        self.params.add_coord("GAP", value);
    }

    // ========== Silkscreen Parameters ==========

    /// Get minimum solder mask sliver.
    pub fn min_solder_mask_sliver(&self) -> Option<Coord> {
        self.params.get("MINLIMIT").and_then(|v| v.as_coord().ok())
    }

    /// Set minimum solder mask sliver.
    pub fn set_min_solder_mask_sliver(&mut self, value: Coord) {
        self.params.add_coord("MINLIMIT", value);
    }

    // ========== Plane Connection Parameters ==========

    /// Get plane connect style (Relief, Direct, NoConnect).
    pub fn plane_connect_style(&self) -> Option<String> {
        self.params
            .get("CONNECTSTYLE")
            .map(|v| v.as_str().to_string())
    }

    /// Set plane connect style.
    pub fn set_plane_connect_style(&mut self, style: &str) {
        self.params.add("CONNECTSTYLE", style);
    }

    /// Get thermal relief air gap width.
    pub fn thermal_relief_air_gap(&self) -> Option<Coord> {
        self.params
            .get("AIRGAPWIDTH")
            .and_then(|v| v.as_coord().ok())
    }

    /// Set thermal relief air gap width.
    pub fn set_thermal_relief_air_gap(&mut self, value: Coord) {
        self.params.add_coord("AIRGAPWIDTH", value);
    }

    /// Get thermal relief conductor width.
    pub fn thermal_relief_conductor_width(&self) -> Option<Coord> {
        self.params
            .get("CONDUCTORWIDTH")
            .and_then(|v| v.as_coord().ok())
    }

    /// Set thermal relief conductor width.
    pub fn set_thermal_relief_conductor_width(&mut self, value: Coord) {
        self.params.add_coord("CONDUCTORWIDTH", value);
    }

    /// Get thermal relief conductor count.
    pub fn thermal_relief_conductors(&self) -> Option<i32> {
        self.params.get("CONDUCTORS").map(|v| v.as_int_or(4))
    }

    /// Set thermal relief conductor count.
    pub fn set_thermal_relief_conductors(&mut self, count: i32) {
        self.params.add("CONDUCTORS", &count.to_string());
    }

    // ========== Routing Corner Parameters ==========

    /// Get routing corner style (45, 90, Any).
    pub fn routing_corner_style(&self) -> Option<String> {
        self.params.get("STYLE").map(|v| v.as_str().to_string())
    }

    /// Set routing corner style.
    pub fn set_routing_corner_style(&mut self, style: &str) {
        self.params.add("STYLE", style);
    }

    // ========== Routing Topology Parameters ==========

    /// Get routing topology (Shortest, DaisyChain, Star, Starburst).
    pub fn routing_topology(&self) -> Option<String> {
        self.params.get("TOPOLOGY").map(|v| v.as_str().to_string())
    }

    /// Set routing topology.
    pub fn set_routing_topology(&mut self, topology: &str) {
        self.params.add("TOPOLOGY", topology);
    }

    // ========== Component Parameters ==========

    /// Get component clearance.
    pub fn component_clearance(&self) -> Option<Coord> {
        self.params.get("GAP").and_then(|v| v.as_coord().ok())
    }

    /// Set component clearance.
    pub fn set_component_clearance(&mut self, value: Coord) {
        self.params.add_coord("GAP", value);
    }

    /// Get maximum component height.
    pub fn max_height(&self) -> Option<Coord> {
        self.params.get("MAXLIMIT").and_then(|v| v.as_coord().ok())
    }

    /// Set maximum component height.
    pub fn set_max_height(&mut self, value: Coord) {
        self.params.add_coord("MAXLIMIT", value);
    }

    // ========== Length Parameters ==========

    /// Get minimum length (Length rule).
    pub fn min_length(&self) -> Option<Coord> {
        self.params.get("MINLIMIT").and_then(|v| v.as_coord().ok())
    }

    /// Set minimum length.
    pub fn set_min_length(&mut self, value: Coord) {
        self.params.add_coord("MINLIMIT", value);
    }

    /// Get maximum length (Length rule).
    pub fn max_length(&self) -> Option<Coord> {
        self.params.get("MAXLIMIT").and_then(|v| v.as_coord().ok())
    }

    /// Set maximum length.
    pub fn set_max_length(&mut self, value: Coord) {
        self.params.add_coord("MAXLIMIT", value);
    }

    /// Get matched length tolerance.
    pub fn matched_length_tolerance(&self) -> Option<Coord> {
        self.params.get("TOLERANCE").and_then(|v| v.as_coord().ok())
    }

    /// Set matched length tolerance.
    pub fn set_matched_length_tolerance(&mut self, value: Coord) {
        self.params.add_coord("TOLERANCE", value);
    }

    /// Get maximum stub length.
    pub fn max_stub_length(&self) -> Option<Coord> {
        self.params.get("MAXLIMIT").and_then(|v| v.as_coord().ok())
    }

    /// Set maximum stub length.
    pub fn set_max_stub_length(&mut self, value: Coord) {
        self.params.add_coord("MAXLIMIT", value);
    }

    // ========== Signal Integrity Parameters ==========

    /// Get minimum impedance.
    pub fn min_impedance(&self) -> Option<f64> {
        self.params.get("MINLIMIT").map(|v| v.as_double_or(0.0))
    }

    /// Set minimum impedance.
    pub fn set_min_impedance(&mut self, value: f64) {
        self.params.add("MINLIMIT", &value.to_string());
    }

    /// Get maximum impedance.
    pub fn max_impedance(&self) -> Option<f64> {
        self.params.get("MAXLIMIT").map(|v| v.as_double_or(0.0))
    }

    /// Set maximum impedance.
    pub fn set_max_impedance(&mut self, value: f64) {
        self.params.add("MAXLIMIT", &value.to_string());
    }

    // ========== Via Under SMD Parameters ==========

    /// Check if vias under SMD are allowed.
    pub fn vias_under_smd_allowed(&self) -> bool {
        self.params
            .get("ALLOWVIAS")
            .map(|v| v.as_bool_or(false))
            .unwrap_or(false)
    }

    /// Set whether vias under SMD are allowed.
    pub fn set_vias_under_smd_allowed(&mut self, allowed: bool) {
        self.params
            .add("ALLOWVIAS", if allowed { "TRUE" } else { "FALSE" });
    }

    // ========== Maximum Via Count Parameters ==========

    /// Get maximum via count.
    pub fn max_via_count(&self) -> Option<i32> {
        self.params.get("MAXVIACOUNT").map(|v| v.as_int_or(0))
    }

    /// Set maximum via count.
    pub fn set_max_via_count(&mut self, count: i32) {
        self.params.add("MAXVIACOUNT", &count.to_string());
    }

    // ========== Fanout Parameters ==========

    /// Get fanout direction.
    pub fn fanout_direction(&self) -> Option<String> {
        self.params.get("DIRECTION").map(|v| v.as_str().to_string())
    }

    /// Set fanout direction.
    pub fn set_fanout_direction(&mut self, direction: &str) {
        self.params.add("DIRECTION", direction);
    }

    /// Get fanout style.
    pub fn fanout_style(&self) -> Option<String> {
        self.params.get("STYLE").map(|v| v.as_str().to_string())
    }

    /// Set fanout style.
    pub fn set_fanout_style(&mut self, style: &str) {
        self.params.add("STYLE", style);
    }

    // ========== Generic Limit Parameters ==========

    /// Get generic minimum limit (works for various rule types).
    pub fn generic_min_limit(&self) -> Option<Coord> {
        self.params.get("MINLIMIT").and_then(|v| v.as_coord().ok())
    }

    /// Get generic maximum limit (works for various rule types).
    pub fn generic_max_limit(&self) -> Option<Coord> {
        self.params.get("MAXLIMIT").and_then(|v| v.as_coord().ok())
    }

    /// Get generic gap parameter (works for clearance-type rules).
    pub fn generic_gap(&self) -> Option<Coord> {
        self.params.get("GAP").and_then(|v| v.as_coord().ok())
    }
}

/// Read all design rules from a binary stream.
pub fn read_rules<R: Read>(reader: &mut R, data_len: usize) -> Result<Vec<PcbRule>> {
    let mut rules = Vec::new();
    let mut bytes_read = 0;

    while bytes_read < data_len {
        let start_pos = bytes_read;

        // Check if we have at least 6 bytes for header
        if bytes_read + 6 > data_len {
            break;
        }

        match PcbRule::read_from(reader) {
            Ok(rule) => {
                // Calculate bytes consumed
                let rule_size = rule.binary_size();
                bytes_read += rule_size;
                rules.push(rule);
            }
            Err(e) => {
                // If we can't read a complete rule, stop
                if start_pos == bytes_read {
                    return Err(e);
                }
                break;
            }
        }
    }

    Ok(rules)
}

/// Write all design rules to a binary stream.
pub fn write_rules<W: Write>(writer: &mut W, rules: &[PcbRule]) -> Result<()> {
    for rule in rules {
        rule.write_to(writer)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_rule_kind_roundtrip() {
        let kinds = [
            RuleKind::Clearance,
            RuleKind::Width,
            RuleKind::RoutingLayers,
            RuleKind::DiffPairsRouting,
        ];

        for kind in &kinds {
            let id = kind.to_id();
            let parsed = RuleKind::from_id(id);
            assert_eq!(*kind, parsed);
        }
    }

    #[test]
    fn test_rule_kind_name() {
        assert_eq!(RuleKind::Clearance.name(), "Clearance");
        assert_eq!(RuleKind::Width.name(), "Width");
        assert_eq!(RuleKind::DiffPairsRouting.name(), "DiffPairsRouting");
    }

    #[test]
    fn test_rule_serialization() {
        let mut rule = PcbRule::new(RuleKind::Clearance, "TestClearance");
        rule.enabled = true;
        rule.priority = 1;
        rule.scope1_expression = "All".to_string();
        rule.scope2_expression = "All".to_string();
        rule.params.add("GAP", "10mil");

        // Write to buffer
        let mut buffer = Vec::new();
        rule.write_to(&mut buffer).unwrap();

        // Read back
        let mut cursor = Cursor::new(&buffer);
        let parsed = PcbRule::read_from(&mut cursor).unwrap();

        assert_eq!(parsed.kind, RuleKind::Clearance);
        assert_eq!(parsed.name, "TestClearance");
        assert!(parsed.enabled);
        assert_eq!(parsed.priority, 1);
    }
}

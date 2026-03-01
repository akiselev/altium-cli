//! Typed DRC (Design Rule Check) parsing for PcbDoc files.
//!
//! Replaces opaque `ParamSectionData` storage for Rules6, violation sections,
//! WaivedViolations, and DesignRuleCheckerOptions6 with fully typed Rust structs
//! parsed at load time.

use altium_format_derive::{FromParams, ToParams};
use altium_format_types::{
    BgaFanoutDirection, BgaFanoutViaMode, CornerStyle, FanoutDirection, FanoutStyle, NetScope,
    NetTopology, PlaneConnectionStyle, PolygonReliefAngle, RouteVia, RuleKind, RuleLayerKind,
};

use crate::param_collection::ParameterCollection;
use crate::param_value::{ClearanceMatrix, MilCoord, ToParamValue};
use crate::pcbdoc::records::ParamSectionKind;
use crate::{AltiumFormatError, Result};

// ── Rule types ──────────────────────────────────────────────────────────────

/// Common fields shared by ALL rule records.
/// Parsed from the |KEY=VALUE| param record in Rules6/Data.
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct PcbRuleBase {
    #[param(key = "SELECTION", default = false)]
    pub selection: bool,
    #[param(key = "LAYER", default = String::new())]
    pub layer: String,
    #[param(key = "LOCKED", default = false)]
    pub locked: bool,
    #[param(key = "POLYGONOUTLINE", default = false)]
    pub polygon_outline: bool,
    #[param(key = "USERROUTED", default = true)]
    pub user_routed: bool,
    #[param(key = "KEEPOUT", default = false)]
    pub keepout: bool,
    #[param(key = "UNIONINDEX", default = 0u32)]
    pub union_index: u32,
    #[param(key = "RULEKIND")]
    pub rule_kind: RuleKind,
    #[param(key = "NETSCOPE")]
    pub net_scope: NetScope,
    #[param(key = "LAYERKIND", default = RuleLayerKind::SameLayer)]
    pub layer_kind: RuleLayerKind,
    #[param(key = "NAME")]
    pub name: String,
    #[param(key = "COMMENT", default = String::new())]
    pub comment: String,
    #[param(key = "UNIQUEID", default = String::new())]
    pub unique_id: String,
    #[param(key = "SCOPE1EXPRESSION", default = String::new())]
    pub scope1_expression: String,
    #[param(key = "SCOPE2EXPRESSION", default = String::new())]
    pub scope2_expression: String,
    #[param(key = "ENABLED", default = true)]
    pub enabled: bool,
    #[param(key = "PRIORITY", default = 1u16)]
    pub priority: u16,
    #[param(key = "DEFINEDBYLOGICALDOCUMENT", default = false)]
    pub defined_by_logical_document: bool,
}

/// A fully typed rule record: prefix + base + kind-specific data.
#[derive(Debug)]
pub(crate) struct PcbRule {
    pub prefix: u16,
    pub base: PcbRuleBase,
    pub kind_data: PcbRuleKindData,
}

/// Kind-specific rule data. Each variant holds the extra parameters
/// for that rule type. The base params have already been consumed.
#[derive(Debug)]
pub(crate) enum PcbRuleKindData {
    Clearance(ClearanceRuleData),
    ParallelSegment(ParallelSegmentRuleData),
    Width(WidthRuleData),
    Length(LengthRuleData),
    MatchedLengths(MatchedLengthsRuleData),
    DaisyChainStubLength(DaisyChainStubLengthRuleData),
    PowerPlaneConnectStyle(PowerPlaneConnectStyleRuleData),
    RoutingTopology(RoutingTopologyRuleData),
    RoutingPriority(RoutingPriorityRuleData),
    RoutingLayers(RoutingLayersRuleData),
    RoutingCornerStyle(RoutingCornerStyleRuleData),
    RoutingViaStyle(RoutingViaStyleRuleData),
    PowerPlaneClearance(PowerPlaneClearanceRuleData),
    SolderMaskExpansion(SolderMaskExpansionRuleData),
    PasteMaskExpansion(PasteMaskExpansionRuleData),
    ShortCircuit(ShortCircuitRuleData),
    BrokenNets(BrokenNetsRuleData),
    ViasUnderSmd(ViasUnderSmdRuleData),
    MaximumViaCount(MaximumViaCountRuleData),
    MinimumAnnularRing(MinimumAnnularRingRuleData),
    PolygonConnectStyle(PolygonConnectStyleRuleData),
    AcuteAngle(AcuteAngleRuleData),
    ConfinementConstraint(ConfinementConstraintRuleData),
    SmdToCorner(SmdToCornerRuleData),
    ComponentClearance(ComponentClearanceRuleData),
    ComponentRotations(EmptyRuleData),
    PermittedLayers(EmptyRuleData),
    NetsToIgnore(EmptyRuleData),
    SignalStimulus(SignalStimulusRuleData),
    OvershootFallingEdge(OvershootUndershootRuleData),
    OvershootRisingEdge(OvershootUndershootRuleData),
    UndershootFallingEdge(OvershootUndershootRuleData),
    UndershootRisingEdge(OvershootUndershootRuleData),
    MaxMinImpedance(MaxMinImpedanceRuleData),
    SignalTopValue(SignalValueRuleData),
    SignalBaseValue(SignalValueRuleData),
    FlightTimeRisingEdge(FlightTimeRuleData),
    FlightTimeFallingEdge(FlightTimeRuleData),
    LayerStack(EmptyRuleData),
    MaxSlopeRisingEdge(SlopeRuleData),
    MaxSlopeFallingEdge(SlopeRuleData),
    SupplyNets(SupplyNetsRuleData),
    MaxMinHoleSize(MaxMinHoleSizeRuleData),
    FabricationTestpointStyle(TestpointStyleRuleData),
    FabricationTestpointUsage(TestpointUsageRuleData),
    UnconnectedPin(EmptyRuleData),
    SmdToPlane(SmdToPlaneRuleData),
    SmdNeckDown(SmdNeckDownRuleData),
    LayerPair(LayerPairRuleData),
    FanoutControl(FanoutControlRuleData),
    MaxMinHeight(MaxMinHeightRuleData),
    DifferentialPairsRouting(DiffPairsRoutingRuleData),
    HoleToHoleClearance(HoleToHoleClearanceRuleData),
    MinimumSolderMaskSliver(MinimumSolderMaskSliverRuleData),
    SilkToSolderMaskClearance(SilkToSolderMaskClearanceRuleData),
    SilkToSilkClearance(SilkToSilkClearanceRuleData),
    NetAntennae(NetAntennaeRuleData),
    AssyTestPointStyle(TestpointStyleRuleData),
    AssyTestPointUsage(TestpointUsageRuleData),
    SilkToBoardRegionClearance(EmptyRuleData),
    SmdEntry(SmdEntryRuleData),
    None(EmptyRuleData),
    UnpouredPolygon(UnpouredPolygonRuleData),
    BoardOutlineClearance(BoardOutlineClearanceRuleData),
    BackDrilling(BackDrillingRuleData),
    Creepage(CreepageRuleData),
    ReturnPath(ReturnPathRuleData),
    RoutingNeckDown(RoutingNeckDownRuleData),
    WireBonding(WireBondingRuleData),
    ZAxisClearance(ZAxisClearanceRuleData),
}

// ── Concrete rule data structs ──────────────────────────────────────────────

/// Rules with no kind-specific parameters beyond the base.
#[derive(Debug)]
pub(crate) struct EmptyRuleData;

impl EmptyRuleData {
    pub(crate) fn from_params(_params: &mut ParameterCollection) -> Result<Self> {
        Ok(Self)
    }
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct BrokenNetsRuleData {
    #[param(key = "CHECKBADCONNECTIONS", default = false)]
    pub check_bad_connections: bool,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SmdEntryRuleData {
    #[param(key = "SIDE", default = false)]
    pub side: bool,
    #[param(key = "CORNER", default = false)]
    pub corner: bool,
    #[param(key = "ANYANGLE", default = false)]
    pub any_angle: bool,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct UnpouredPolygonRuleData {
    #[param(key = "ALLOWUNPOURED", default = false)]
    pub allow_unpoured: bool,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SmdToPlaneRuleData {
    #[param(key = "DISTANCE", default = MilCoord::default())]
    pub distance: MilCoord,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SmdNeckDownRuleData {
    #[param(key = "PERCENT", default = 0f64)]
    pub percent: f64,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct ReturnPathRuleData {
    #[param(key = "IMPEDANCEPROFILEID", default = String::new())]
    pub impedance_profile_id: String,
    #[param(key = "_GAPLIMIT", default = MilCoord::default())]
    pub gap_limit: MilCoord,
    #[param(key = "_USEANTIPADS", default = false)]
    pub use_antipads: bool,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct ClearanceRuleData {
    #[param(key = "GAP", default = MilCoord::default())]
    pub gap: MilCoord,
    #[param(key = "GENERICCLEARANCE", default = MilCoord::default())]
    pub generic_clearance: MilCoord,
    #[param(key = "IGNOREPADTOPADCLEARANCEINFOOTPRINT", default = false)]
    pub ignore_pad_to_pad: bool,
    #[param(key = "OBJECTCLEARANCES", default = ClearanceMatrix::default())]
    pub object_clearances: ClearanceMatrix,
    #[param(key = "CHECKNETSINDIFFPAIR", default = false)]
    pub check_nets_in_diff_pair: bool,
    #[param(key = "CHECKDIFFPAIRVSDIFFPAIR", default = false)]
    pub check_diff_pair_vs_diff_pair: bool,
    #[param(key = "CHECKXSIGNALS", default = false)]
    pub check_x_signals: bool,
    #[param(key = "CHECKOTHERS", default = false)]
    pub check_others: bool,
    #[param(key = "CHECKCONNECTEDCOPPER", default = false)]
    pub check_connected_copper: bool,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct ParallelSegmentRuleData {
    #[param(key = "GAP", default = MilCoord::default())]
    pub gap: MilCoord,
    #[param(key = "LIMIT", default = MilCoord::default())]
    pub limit: MilCoord,
    #[param(key = "PARALLELLENGTH", default = MilCoord::default())]
    pub parallel_length: MilCoord,
    #[param(key = "CHECKPARALLEL", default = true)]
    pub check_parallel: bool,
    #[param(key = "CHECKADJACENTLAYERS", default = false)]
    pub check_adjacent_layers: bool,
}

#[derive(Debug)]
pub(crate) struct WidthLayerOverride {
    pub prefix: String,
    pub min_width: Option<MilCoord>,
    pub max_width: Option<MilCoord>,
    pub pref_width: Option<MilCoord>,
}

#[derive(Debug)]
pub(crate) struct WidthSubstackOverride {
    pub substack_id: String,
    pub layer_overrides: Vec<(String, Option<MilCoord>, Option<MilCoord>, Option<MilCoord>)>,
}

#[derive(Debug)]
pub(crate) struct WidthRuleData {
    pub min_limit: MilCoord,
    pub max_limit: MilCoord,
    pub preferred_width: MilCoord,
    pub check_connected_copper: bool,
    pub per_layer: Vec<WidthLayerOverride>,
    pub impedance_driven: Option<bool>,
    pub min_imp: Option<f64>,
    pub max_imp: Option<f64>,
    pub fav_imp: Option<f64>,
    pub impedance_profile_driven: Option<bool>,
    pub impedance_profile_id: Option<String>,
    pub impedance_profile_value: Option<f64>,
    pub substack_overrides: Vec<WidthSubstackOverride>,
}

impl WidthRuleData {
    fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        let min_limit: MilCoord = params.remove_required("MINLIMIT")?;
        let max_limit: MilCoord = params.remove_required("MAXLIMIT")?;
        let preferred_width: MilCoord = params.remove_required("PREFEREDWIDTH")?;

        // Per-layer overrides.
        let mut per_layer = Vec::new();
        for prefix in &signal_layer_prefixes() {
            let min_w: Option<MilCoord> = params.remove_optional(&format!("{prefix}_MINWIDTH"))?;
            let max_w: Option<MilCoord> = params.remove_optional(&format!("{prefix}_MAXWIDTH"))?;
            let pref_w: Option<MilCoord> = params.remove_optional(&format!("{prefix}_PREFWIDTH"))?;
            if min_w.is_some() || max_w.is_some() || pref_w.is_some() {
                per_layer.push(WidthLayerOverride {
                    prefix: prefix.to_string(),
                    min_width: min_w,
                    max_width: max_w,
                    pref_width: pref_w,
                });
            }
        }

        let check_connected_copper: bool = params.remove_with_default("CHECKCONNECTEDCOPPER", false)?;

        // Impedance fields.
        let impedance_driven: Option<bool> = params.remove_optional("IMPEDANCEDRIVEN")?;
        let min_imp: Option<f64> = params.remove_optional("MINIMP")?;
        let max_imp: Option<f64> = params.remove_optional("MAXIMP")?;
        let fav_imp: Option<f64> = params.remove_optional("FAVIMP")?;
        let impedance_profile_driven: Option<bool> = params.remove_optional("IMPEDANCEPROFILEDRIVEN")?;
        let impedance_profile_id: Option<String> = params.remove_optional("IMPEDANCEPROFILEID")?;
        let impedance_profile_value: Option<f64> = params.remove_optional("IMPEDANCEPROFILEVALUE")?;

        // Per-substack overrides: SUBSTACK{n} holds a GUID; consume GUID-keyed layer overrides.
        let mut substack_overrides = Vec::new();
        let mut n = 1usize;
        loop {
            let substack_key = format!("SUBSTACK{n}");
            let substack_id: Option<String> = params.remove_optional(&substack_key)?;
            let Some(substack_id) = substack_id else { break };

            // Consume all {LAYER}_{GUID}_MINWIDTH/MAXWIDTH/PREFWIDTH for this GUID.
            let guid_upper = substack_id.to_ascii_uppercase();
            let mut layer_overrides = Vec::new();
            for prefix in &signal_layer_prefixes() {
                let min_key = format!("{prefix}_{guid_upper}_MINWIDTH");
                let max_key = format!("{prefix}_{guid_upper}_MAXWIDTH");
                let pref_key = format!("{prefix}_{guid_upper}_PREFWIDTH");
                let min_w: Option<MilCoord> = params.remove_optional(&min_key)?;
                let max_w: Option<MilCoord> = params.remove_optional(&max_key)?;
                let pref_w: Option<MilCoord> = params.remove_optional(&pref_key)?;
                if min_w.is_some() || max_w.is_some() || pref_w.is_some() {
                    layer_overrides.push((prefix.to_string(), min_w, max_w, pref_w));
                }
            }

            substack_overrides.push(WidthSubstackOverride {
                substack_id,
                layer_overrides,
            });
            n += 1;
        }

        Ok(Self {
            min_limit,
            max_limit,
            preferred_width,
            check_connected_copper,
            per_layer,
            impedance_driven,
            min_imp,
            max_imp,
            fav_imp,
            impedance_profile_driven,
            impedance_profile_id,
            impedance_profile_value,
            substack_overrides,
        })
    }
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct LengthRuleData {
    #[param(key = "MINLIMIT", default = MilCoord::default())]
    pub min_limit: MilCoord,
    #[param(key = "MAXLIMIT", default = MilCoord::default())]
    pub max_limit: MilCoord,
    #[param(key = "USEDELAYUNITS", default = false)]
    pub use_delay_units: bool,
    #[param(key = "MINDELAY", default = 0f64)]
    pub min_delay: f64,
    #[param(key = "MAXDELAY", default = 0f64)]
    pub max_delay: f64,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct MatchedLengthsRuleData {
    #[param(key = "TOLERANCE", default = MilCoord::default())]
    pub tolerance: MilCoord,
    #[param(key = "CHECKNETSINDIFFPAIR", default = false)]
    pub check_nets_in_diff_pair: bool,
    #[param(key = "CHECKDIFFPAIRVSDIFFPAIR", default = false)]
    pub check_diff_pair_vs_diff_pair: bool,
    #[param(key = "CHECKXSIGNALS", default = false)]
    pub check_x_signals: bool,
    #[param(key = "CHECKOTHERS", default = false)]
    pub check_others: bool,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct DaisyChainStubLengthRuleData {
    #[param(key = "MAXLIMIT", default = MilCoord::default())]
    pub max_limit: MilCoord,
    #[param(key = "LIMIT", default = MilCoord::default())]
    pub limit: MilCoord,
}

#[derive(Debug)]
pub(crate) struct PlaneConnectTypeOverride {
    pub prefix: String,
    pub connect_style: PlaneConnectionStyle,
    pub relief_expansion: MilCoord,
    pub relief_entries: i32,
    pub relief_conductor_width: MilCoord,
    pub relief_air_gap: MilCoord,
}

#[derive(Debug)]
pub(crate) struct PowerPlaneConnectStyleRuleData {
    pub connect_style: Option<PlaneConnectionStyle>,
    pub relief_expansion: Option<MilCoord>,
    pub relief_entries: Option<i32>,
    pub relief_conductor_width: Option<MilCoord>,
    pub relief_air_gap: Option<MilCoord>,
    pub type_overrides: Vec<PlaneConnectTypeOverride>,
}

impl PowerPlaneConnectStyleRuleData {
    fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        // Simple top-level fields (may be absent when per-type overrides are used).
        let connect_style: Option<PlaneConnectionStyle> = params.remove_optional("PLANECONNECTSTYLE")?;
        let relief_expansion: Option<MilCoord> = params.remove_optional("RELIEFEXPANSION")?;
        let relief_entries: Option<i32> = params.remove_optional("RELIEFENTRIES")?;
        let relief_conductor_width: Option<MilCoord> = params.remove_optional("RELIEFCONDUCTORWIDTH")?;
        let relief_air_gap: Option<MilCoord> = params.remove_optional("RELIEFAIRGAP")?;

        // Per-type overrides: PAD.* and VIA.*
        let mut type_overrides = Vec::new();
        for prefix in &["PAD", "VIA"] {
            let cs_key = format!("{prefix}.PLANECONNECTSTYLE");
            if let Some(cs) = params.remove_optional::<PlaneConnectionStyle>(&cs_key)? {
                let exp_key = format!("{prefix}.RELIEFEXPANSION");
                let ent_key = format!("{prefix}.RELIEFENTRIES");
                let cw_key = format!("{prefix}.RELIEFCONDUCTORWIDTH");
                let ag_key = format!("{prefix}.RELIEFAIRGAP");
                type_overrides.push(PlaneConnectTypeOverride {
                    prefix: prefix.to_string(),
                    connect_style: cs,
                    relief_expansion: params.remove_with_default(&exp_key, MilCoord::default())?,
                    relief_entries: params.remove_with_default(&ent_key, 4)?,
                    relief_conductor_width: params.remove_with_default(&cw_key, MilCoord::default())?,
                    relief_air_gap: params.remove_with_default(&ag_key, MilCoord::default())?,
                });
            }
        }

        Ok(Self {
            connect_style,
            relief_expansion,
            relief_entries,
            relief_conductor_width,
            relief_air_gap,
            type_overrides,
        })
    }
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct RoutingTopologyRuleData {
    #[param(key = "TOPOLOGY")]
    pub topology: NetTopology,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct RoutingPriorityRuleData {
    #[param(key = "ROUTINGPRIORITY", default = 0i32)]
    pub routing_priority: i32,
}

/// Routing layers: per-signal-layer boolean flags.
/// Uses "TOP LAYER_V5", "MID LAYER 1_V5", ..., "BOTTOM LAYER_V5" key convention.
#[derive(Debug)]
pub(crate) struct RoutingLayersRuleData {
    pub layer_flags: Vec<(String, bool)>,
}

impl RoutingLayersRuleData {
    fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        let mut layer_flags = Vec::new();
        let layer_prefixes = routing_layer_prefixes();
        for prefix in &layer_prefixes {
            let key = format!("{prefix}_V5");
            if let Some(val) = params.remove_optional::<bool>(&key)? {
                layer_flags.push((prefix.to_string(), val));
            }
        }
        Ok(Self { layer_flags })
    }
}

fn routing_layer_prefixes() -> Vec<String> {
    let mut prefixes = vec!["TOP LAYER".to_string()];
    for i in 1..=30 {
        prefixes.push(format!("MID LAYER {i}"));
    }
    prefixes.push("BOTTOM LAYER".to_string());
    prefixes
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct RoutingCornerStyleRuleData {
    #[param(key = "CORNERSTYLE")]
    pub corner_style: CornerStyle,
    #[param(key = "MINSETBACK", default = MilCoord::default())]
    pub min_setback: MilCoord,
    #[param(key = "MAXSETBACK", default = MilCoord::default())]
    pub max_setback: MilCoord,
}

#[derive(Debug)]
pub(crate) struct ViaTemplate {
    pub guid: String,
    pub name: String,
}

#[derive(Debug)]
pub(crate) struct RoutingViaStyleRuleData {
    pub min_hole_width: MilCoord,
    pub max_hole_width: MilCoord,
    pub preferred_hole_width: MilCoord,
    pub min_width: MilCoord,
    pub max_width: MilCoord,
    pub preferred_width: MilCoord,
    pub via_style: RouteVia,
    pub use_via_templates: bool,
    pub via_templates: Vec<ViaTemplate>,
}

impl RoutingViaStyleRuleData {
    fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        let min_hole_width: MilCoord = params.remove_required("MINHOLEWIDTH")?;
        let max_hole_width: MilCoord = params.remove_required("MAXHOLEWIDTH")?;
        let preferred_hole_width: MilCoord = params.remove_required("HOLEWIDTH")?;
        let min_width: MilCoord = params.remove_required("MINWIDTH")?;
        let max_width: MilCoord = params.remove_required("MAXWIDTH")?;
        let preferred_width: MilCoord = params.remove_required("WIDTH")?;
        let via_style: RouteVia = params.remove_required("VIASTYLE")?;
        let use_via_templates: bool = params.remove_with_default("USEVIATEMPLATES", false)?;

        // Consume indexed VIATEMPLATEGUID#N / VIATEMPLATENAME#N pairs (1-based).
        let mut via_templates = Vec::new();
        let mut n = 1usize;
        loop {
            let guid_key = format!("VIATEMPLATEGUID#{n}");
            let guid: Option<String> = params.remove_optional(&guid_key)?;
            let Some(guid) = guid else { break };
            let name_key = format!("VIATEMPLATENAME#{n}");
            let name: String = params.remove_with_default(&name_key, String::new())?;
            via_templates.push(ViaTemplate { guid, name });
            n += 1;
        }

        Ok(Self {
            min_hole_width,
            max_hole_width,
            preferred_hole_width,
            min_width,
            max_width,
            preferred_width,
            via_style,
            use_via_templates,
            via_templates,
        })
    }
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct PowerPlaneClearanceRuleData {
    #[param(key = "CLEARANCE", default = MilCoord::default())]
    pub clearance: MilCoord,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SolderMaskExpansionRuleData {
    #[param(key = "EXPANSION", default = MilCoord::default())]
    pub expansion: MilCoord,
    #[param(key = "ISTENTINGTOP", default = false)]
    pub is_tenting_top: bool,
    #[param(key = "ISTENTINGBOTTOM", default = false)]
    pub is_tenting_bottom: bool,
    #[param(key = "SOLDERMASKFROMHOLE", default = false)]
    pub soldermask_from_hole: bool,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct PasteMaskExpansionRuleData {
    #[param(key = "EXPANSION", default = MilCoord::default())]
    pub expansion: MilCoord,
    #[param(key = "PERCENT", default = 0f64)]
    pub percent: f64,
    #[param(key = "THPADUSETOPPASTE", default = false)]
    pub thpad_use_top_paste: bool,
    #[param(key = "THPADUSEBOTTOMPASTE", default = false)]
    pub thpad_use_bottom_paste: bool,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct ShortCircuitRuleData {
    #[param(key = "ALLOWED", default = false)]
    pub allowed: bool,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct ViasUnderSmdRuleData {
    #[param(key = "ALLOWED", default = true)]
    pub allowed: bool,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct MaximumViaCountRuleData {
    #[param(key = "MAXVIACOUNT", default = 10u32)]
    pub max_via_count: u32,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct MinimumAnnularRingRuleData {
    #[param(key = "MINIMUMRING", default = MilCoord::default())]
    pub min_limit: MilCoord,
}

/// Polygon connect style rule. Has two patterns:
/// - Simple: CONNECTSTYLE, RELIEFCONDUCTORWIDTH, etc. at top level
/// - Per-pad-type: THPAD.CONNECTSTYLE, SMDPAD.CONNECTSTYLE, VIA.CONNECTSTYLE
///
/// The per-pad-type pattern is used when the rule's priority > some threshold.
/// We parse both patterns into a unified structure.
#[derive(Debug)]
pub(crate) struct PolygonConnectStyleRuleData {
    /// Simple top-level fields (when no per-pad-type prefix).
    pub connect_style: Option<PlaneConnectionStyle>,
    pub relief_conductor_width: Option<MilCoord>,
    pub relief_entries: Option<i32>,
    pub polygon_relief_angle: Option<PolygonReliefAngle>,
    pub air_gap_width: Option<MilCoord>,
    /// Per-pad-type overrides (THPAD.*, SMDPAD.*, VIA.*).
    pub pad_type_overrides: Vec<PadTypePolygonConnect>,
}

#[derive(Debug)]
pub(crate) struct PadTypePolygonConnect {
    pub prefix: String,
    pub connect_style: PlaneConnectionStyle,
    pub relief_conductor_width: MilCoord,
    pub relief_entries: i32,
    pub polygon_relief_angle: PolygonReliefAngle,
    pub air_gap_width: MilCoord,
}

impl PolygonConnectStyleRuleData {
    fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        // Try simple top-level fields first.
        let connect_style: Option<PlaneConnectionStyle> =
            params.remove_optional("CONNECTSTYLE")?;
        let relief_conductor_width: Option<MilCoord> =
            params.remove_optional("RELIEFCONDUCTORWIDTH")?;
        let relief_entries: Option<i32> = params.remove_optional("RELIEFENTRIES")?;
        let polygon_relief_angle: Option<PolygonReliefAngle> =
            params.remove_optional("POLYGONRELIEFANGLE")?;
        let air_gap_width: Option<MilCoord> = params.remove_optional("AIRGAPWIDTH")?;

        // Try per-pad-type prefixed fields (THPAD., SMDPAD., VIA.).
        let mut pad_type_overrides = Vec::new();
        for prefix in &["THPAD", "SMDPAD", "VIA"] {
            let key = format!("{prefix}.CONNECTSTYLE");
            if let Some(cs) = params.remove_optional::<PlaneConnectionStyle>(&key)? {
                let rw_key = format!("{prefix}.RELIEFCONDUCTORWIDTH");
                let re_key = format!("{prefix}.RELIEFENTRIES");
                let ra_key = format!("{prefix}.POLYGONRELIEFANGLE");
                let ag_key = format!("{prefix}.AIRGAPWIDTH");
                pad_type_overrides.push(PadTypePolygonConnect {
                    prefix: prefix.to_string(),
                    connect_style: cs,
                    relief_conductor_width: params
                        .remove_with_default(&rw_key, MilCoord::default())?,
                    relief_entries: params.remove_with_default(&re_key, 4)?,
                    polygon_relief_angle: params
                        .remove_with_default(&ra_key, PolygonReliefAngle::Angle90)?,
                    air_gap_width: params
                        .remove_with_default(&ag_key, MilCoord::default())?,
                });
            }
        }

        Ok(Self {
            connect_style,
            relief_conductor_width,
            relief_entries,
            polygon_relief_angle,
            air_gap_width,
            pad_type_overrides,
        })
    }
}

#[derive(Debug)]
pub(crate) struct AcuteAngleRuleData {
    pub minimum: f64,
    pub check_tracks_only: bool,
}

impl AcuteAngleRuleData {
    fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        let minimum: f64 = params.remove_with_default("MINIMUM", 0f64)?;
        let check_tracks_only: bool = params.remove_with_default("CHECKTRACKSONLY", false)?;
        Ok(Self {
            minimum,
            check_tracks_only,
        })
    }
}

#[derive(Debug)]
pub(crate) struct ConfinementVertex {
    pub kind: u32,
    pub vx: MilCoord,
    pub vy: MilCoord,
    pub cx: MilCoord,
    pub cy: MilCoord,
    pub sa: String,
    pub ea: String,
    pub r: MilCoord,
}

#[derive(Debug)]
pub(crate) struct ConfinementConstraintRuleData {
    pub confinement_style: altium_format_types::ConfinementStyle,
    pub lock_components: bool,
    pub format_copy: bool,
    pub vertices: Vec<ConfinementVertex>,
}

impl ConfinementConstraintRuleData {
    fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        let confinement_style: altium_format_types::ConfinementStyle =
            params.remove_with_default("CONFINEMENTSTYLE", altium_format_types::ConfinementStyle::ConfineIn)?;
        let lock_components: bool = params.remove_with_default("LOCKCOMPONENTS", false)?;
        let format_copy: bool = params.remove_with_default("FORMATCOPY", false)?;

        let mut vertices = Vec::new();
        let mut n = 0usize;
        loop {
            let kind_key = format!("KIND{n}");
            let kind: Option<u32> = params.remove_optional(&kind_key)?;
            let Some(kind) = kind else { break };

            let vx: MilCoord = params.remove_with_default(&format!("VX{n}"), MilCoord::default())?;
            let vy: MilCoord = params.remove_with_default(&format!("VY{n}"), MilCoord::default())?;
            let cx: MilCoord = params.remove_with_default(&format!("CX{n}"), MilCoord::default())?;
            let cy: MilCoord = params.remove_with_default(&format!("CY{n}"), MilCoord::default())?;
            let sa: String = params.remove_with_default(&format!("SA{n}"), String::new())?;
            let ea: String = params.remove_with_default(&format!("EA{n}"), String::new())?;
            let r: MilCoord = params.remove_with_default(&format!("R{n}"), MilCoord::default())?;

            vertices.push(ConfinementVertex { kind, vx, vy, cx, cy, sa, ea, r });
            n += 1;
        }

        Ok(Self {
            confinement_style,
            lock_components,
            format_copy,
            vertices,
        })
    }
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SmdToCornerRuleData {
    #[param(key = "DISTANCE", default = MilCoord::default())]
    pub distance: MilCoord,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct ComponentClearanceRuleData {
    #[param(key = "GAP", default = MilCoord::default())]
    pub gap: MilCoord,
    #[param(key = "COLLISIONCHECKMODE", default = altium_format_types::ComponentCollisionCheckMode::ComponentBodyCheck)]
    pub collision_check_mode: altium_format_types::ComponentCollisionCheckMode,
    #[param(key = "VERTICALGAP", default = MilCoord::default())]
    pub vertical_gap: MilCoord,
    #[param(key = "SHOWDISTANCES", default = false)]
    pub show_distances: bool,
    #[param(key = "DONOTCHECKWITHOUT3DBODY", default = false)]
    pub do_not_check_without_3d_body: bool,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SignalStimulusRuleData {
    #[param(key = "STIMULUSTYPE", default = 0u32)]
    pub stimulus_type: u32,
    #[param(key = "SIGNALLEVEL", default = 0u32)]
    pub signal_level: u32,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct OvershootUndershootRuleData {
    #[param(key = "MAXOVERSHOOT", default = 0f64)]
    pub max_overshoot: f64,
    #[param(key = "MAXUNDERSHOOT", default = 0f64)]
    pub max_undershoot: f64,
    #[param(key = "MAXIMUM", default = 0f64)]
    pub maximum: f64,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct MaxMinImpedanceRuleData {
    #[param(key = "MINIMUM", default = 0f64)]
    pub min_impedance: f64,
    #[param(key = "MAXIMUM", default = 0f64)]
    pub max_impedance: f64,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SignalValueRuleData {
    #[param(key = "MINVALUE", default = 0f64)]
    pub min_value: f64,
    #[param(key = "MAXVALUE", default = 0f64)]
    pub max_value: f64,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct FlightTimeRuleData {
    #[param(key = "MINVALUE", default = 0f64)]
    pub min_value: f64,
    #[param(key = "MAXVALUE", default = 0f64)]
    pub max_value: f64,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SlopeRuleData {
    #[param(key = "MAXSLOPE", default = 0f64)]
    pub max_slope: f64,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct MaxMinHoleSizeRuleData {
    #[param(key = "ABSOLUTEVALUES", default = true)]
    pub absolute_values: bool,
    #[param(key = "MINLIMIT", default = MilCoord::default())]
    pub min_limit: MilCoord,
    #[param(key = "MAXLIMIT", default = MilCoord::default())]
    pub max_limit: MilCoord,
    #[param(key = "MINPERCENT", default = 0f64)]
    pub min_percent: f64,
    #[param(key = "MAXPERCENT", default = 0f64)]
    pub max_percent: f64,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct TestpointStyleRuleData {
    #[param(key = "SIDE", default = 0u32)]
    pub side: u32,
    #[param(key = "TESTPOINTUNDERCOMPONENT", default = true)]
    pub testpoint_under_component: bool,
    #[param(key = "MINSIZE", default = MilCoord::default())]
    pub min_size: MilCoord,
    #[param(key = "MAXSIZE", default = MilCoord::default())]
    pub max_size: MilCoord,
    #[param(key = "PREFEREDSIZE", default = MilCoord::default())]
    pub preferred_size: MilCoord,
    #[param(key = "MINHOLESIZE", default = MilCoord::default())]
    pub min_hole_size: MilCoord,
    #[param(key = "MAXHOLESIZE", default = MilCoord::default())]
    pub max_hole_size: MilCoord,
    #[param(key = "PREFEREDHOLESIZE", default = MilCoord::default())]
    pub preferred_hole_size: MilCoord,
    #[param(key = "TESTPOINTGRID", default = MilCoord::default())]
    pub testpoint_grid: MilCoord,
    #[param(key = "USEGRID", default = true)]
    pub use_grid: bool,
    #[param(key = "GRIDTOLERANCE", default = MilCoord::default())]
    pub grid_tolerance: MilCoord,
    #[param(key = "ALLOWSIDETOP", default = true)]
    pub allow_side_top: bool,
    #[param(key = "ALLOWSIDEBOTTOM", default = true)]
    pub allow_side_bottom: bool,
    #[param(key = "MINSPACING", default = MilCoord::default())]
    pub min_spacing: MilCoord,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct TestpointUsageRuleData {
    #[param(key = "VALID", default = 0u32)]
    pub valid: u32,
    #[param(key = "ALLOWMULTIPLE", default = false)]
    pub allow_multiple: bool,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct LayerPairRuleData {
    #[param(key = "ENFORCE", default = true)]
    pub enforce: bool,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct FanoutControlRuleData {
    #[param(key = "BGADIR", default = BgaFanoutDirection::Out)]
    pub bga_dir: BgaFanoutDirection,
    #[param(key = "BGAVIAMODE", default = BgaFanoutViaMode::Centered)]
    pub bga_via_mode: BgaFanoutViaMode,
    #[param(key = "FANOUTSTYLE", default = FanoutStyle::Auto)]
    pub fanout_style: FanoutStyle,
    #[param(key = "FANOUTDIRECTION", default = FanoutDirection::Alternating)]
    pub fanout_direction: FanoutDirection,
    #[param(key = "VIAGRID", default = MilCoord::default())]
    pub via_grid: MilCoord,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct MaxMinHeightRuleData {
    #[param(key = "MINHEIGHT", default = MilCoord::default())]
    pub min_height: MilCoord,
    #[param(key = "MAXHEIGHT", default = MilCoord::default())]
    pub max_height: MilCoord,
    #[param(key = "PREFHEIGHT", default = MilCoord::default())]
    pub pref_height: MilCoord,
}

/// Differential pairs routing rule. Has global limits + per-layer overrides.
/// Per-layer params use TOPLAYER_, MIDLAYER1_, ..., BOTTOMLAYER_ prefixes.
#[derive(Debug)]
pub(crate) struct DiffPairsRoutingRuleData {
    pub min_limit: MilCoord,
    pub max_limit: MilCoord,
    pub most_freq_gap: MilCoord,
    pub max_uncoupled_length: MilCoord,
    /// Per-layer overrides: (prefix, min_width, max_width, pref_width,
    ///                        min_gap, max_gap, pref_gap).
    pub per_layer: Vec<DiffPairsLayerOverride>,
    pub impedance_profile_driven: Option<bool>,
    pub impedance_profile_id: Option<String>,
    pub impedance_profile_value: Option<f64>,
    pub substack_overrides: Vec<DiffPairsSubstackOverride>,
}

#[derive(Debug)]
pub(crate) struct DiffPairsSubstackOverride {
    pub substack_id: String,
    pub layer_overrides: Vec<(String, Option<MilCoord>, Option<MilCoord>, Option<MilCoord>, Option<MilCoord>, Option<MilCoord>, Option<MilCoord>)>,
}

#[derive(Debug)]
pub(crate) struct DiffPairsLayerOverride {
    pub prefix: String,
    pub min_width: Option<MilCoord>,
    pub max_width: Option<MilCoord>,
    pub pref_width: Option<MilCoord>,
    pub min_gap: Option<MilCoord>,
    pub max_gap: Option<MilCoord>,
    pub pref_gap: Option<MilCoord>,
}

impl DiffPairsRoutingRuleData {
    fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        let min_limit: MilCoord = params.remove_required("MINLIMIT")?;
        let max_limit: MilCoord = params.remove_required("MAXLIMIT")?;
        let most_freq_gap: MilCoord = params.remove_with_default("MOSTFREQGAP", MilCoord::default())?;
        let max_uncoupled_length: MilCoord =
            params.remove_with_default("MAXUNCOUPLEDLENGTH", MilCoord::default())?;

        let mut per_layer = Vec::new();
        let layer_prefixes = signal_layer_prefixes();
        for prefix in &layer_prefixes {
            // Check if any per-layer param exists for this prefix.
            let min_w_key = format!("{prefix}_MINWIDTH");
            let max_w_key = format!("{prefix}_MAXWIDTH");
            let pref_w_key = format!("{prefix}_PREFWIDTH");
            let min_g_key = format!("{prefix}_MINGAP");
            let max_g_key = format!("{prefix}_MAXGAP");
            let pref_g_key = format!("{prefix}_PREFGAP");

            let min_w: Option<MilCoord> = params.remove_optional(&min_w_key)?;
            let max_w: Option<MilCoord> = params.remove_optional(&max_w_key)?;
            let pref_w: Option<MilCoord> = params.remove_optional(&pref_w_key)?;
            let min_g: Option<MilCoord> = params.remove_optional(&min_g_key)?;
            let max_g: Option<MilCoord> = params.remove_optional(&max_g_key)?;
            let pref_g: Option<MilCoord> = params.remove_optional(&pref_g_key)?;

            if min_w.is_some()
                || max_w.is_some()
                || pref_w.is_some()
                || min_g.is_some()
                || max_g.is_some()
                || pref_g.is_some()
            {
                per_layer.push(DiffPairsLayerOverride {
                    prefix: prefix.to_string(),
                    min_width: min_w,
                    max_width: max_w,
                    pref_width: pref_w,
                    min_gap: min_g,
                    max_gap: max_g,
                    pref_gap: pref_g,
                });
            }
        }

        let impedance_profile_driven: Option<bool> = params.remove_optional("IMPEDANCEPROFILEDRIVEN")?;
        let impedance_profile_id: Option<String> = params.remove_optional("IMPEDANCEPROFILEID")?;
        let impedance_profile_value: Option<f64> = params.remove_optional("IMPEDANCEPROFILEVALUE")?;

        // Per-substack overrides for diff-pairs routing.
        let mut substack_overrides = Vec::new();
        let mut n = 1usize;
        loop {
            let substack_key = format!("SUBSTACK{n}");
            let substack_id: Option<String> = params.remove_optional(&substack_key)?;
            let Some(substack_id) = substack_id else { break };

            let guid_upper = substack_id.to_ascii_uppercase();
            let mut layer_overrides = Vec::new();
            for prefix in &signal_layer_prefixes() {
                let min_w_key = format!("{prefix}_{guid_upper}_MINWIDTH");
                let max_w_key = format!("{prefix}_{guid_upper}_MAXWIDTH");
                let pref_w_key = format!("{prefix}_{guid_upper}_PREFWIDTH");
                let min_g_key = format!("{prefix}_{guid_upper}_MINGAP");
                let max_g_key = format!("{prefix}_{guid_upper}_MAXGAP");
                let pref_g_key = format!("{prefix}_{guid_upper}_PREFGAP");
                let min_w: Option<MilCoord> = params.remove_optional(&min_w_key)?;
                let max_w: Option<MilCoord> = params.remove_optional(&max_w_key)?;
                let pref_w: Option<MilCoord> = params.remove_optional(&pref_w_key)?;
                let min_g: Option<MilCoord> = params.remove_optional(&min_g_key)?;
                let max_g: Option<MilCoord> = params.remove_optional(&max_g_key)?;
                let pref_g: Option<MilCoord> = params.remove_optional(&pref_g_key)?;
                if min_w.is_some() || max_w.is_some() || pref_w.is_some()
                    || min_g.is_some() || max_g.is_some() || pref_g.is_some()
                {
                    layer_overrides.push((prefix.to_string(), min_w, max_w, pref_w, min_g, max_g, pref_g));
                }
            }

            substack_overrides.push(DiffPairsSubstackOverride { substack_id, layer_overrides });
            n += 1;
        }

        Ok(Self {
            min_limit,
            max_limit,
            most_freq_gap,
            max_uncoupled_length,
            per_layer,
            impedance_profile_driven,
            impedance_profile_id,
            impedance_profile_value,
            substack_overrides,
        })
    }
}

fn signal_layer_prefixes() -> Vec<String> {
    let mut prefixes = vec!["TOPLAYER".to_string()];
    for i in 1..=30 {
        prefixes.push(format!("MIDLAYER{i}"));
    }
    prefixes.push("BOTTOMLAYER".to_string());
    prefixes
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct HoleToHoleClearanceRuleData {
    #[param(key = "GAP", default = MilCoord::default())]
    pub gap: MilCoord,
    #[param(key = "ALLOWSTACKEDMICROVIAS", default = false)]
    pub allow_stacked_microvias: bool,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct MinimumSolderMaskSliverRuleData {
    #[param(key = "MINSOLDERMASKWIDTH", default = MilCoord::default())]
    pub min_solder_mask_width: MilCoord,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SilkToSolderMaskClearanceRuleData {
    #[param(key = "MINSILKSCREENTOMASKGAP", default = MilCoord::default())]
    pub min_silkscreen_to_mask_gap: MilCoord,
    #[param(key = "CLEARANCETOEXPOSEDCOPPER", default = false)]
    pub clearance_to_exposed_copper: bool,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SilkToSilkClearanceRuleData {
    #[param(key = "SILKTOSILKCLEARANCE", default = MilCoord::default())]
    pub silk_to_silk_clearance: MilCoord,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct NetAntennaeRuleData {
    #[param(key = "NETANTENNAETOLERANCE", default = MilCoord::default())]
    pub net_antennae_tolerance: MilCoord,
}

/// BoardOutlineClearance reuses the same structure as Clearance.
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct BoardOutlineClearanceRuleData {
    #[param(key = "GAP", default = MilCoord::default())]
    pub gap: MilCoord,
    #[param(key = "GENERICCLEARANCE", default = MilCoord::default())]
    pub generic_clearance: MilCoord,
    #[param(key = "IGNOREPADTOPADCLEARANCEINFOOTPRINT", default = false)]
    pub ignore_pad_to_pad: bool,
    #[param(key = "OBJECTCLEARANCES", default = ClearanceMatrix::default())]
    pub object_clearances: ClearanceMatrix,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct BackDrillingRuleData {
    #[param(key = "BACKDRILLINGDEPTH", default = MilCoord::default())]
    pub backdrill_depth: MilCoord,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct CreepageRuleData {
    #[param(key = "GAP", default = MilCoord::default())]
    pub gap: MilCoord,
    #[param(key = "CHECKDISTANCE", default = MilCoord::default())]
    pub check_distance: MilCoord,
    #[param(key = "APPLYTOPOLYGONPOUR", default = false)]
    pub apply_to_polygon_pour: bool,
    #[param(key = "VOLTAGE", default = MilCoord::default())]
    pub voltage: MilCoord,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct RoutingNeckDownRuleData {
    #[param(key = "NECKDOWNPERCENTAGE", default = 0f64)]
    pub neck_down_percentage: f64,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct WireBondingRuleData {
    #[param(key = "MINLIMIT", default = MilCoord::default())]
    pub min_limit: MilCoord,
    #[param(key = "MAXLIMIT", default = MilCoord::default())]
    pub max_limit: MilCoord,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SupplyNetsRuleData {
    #[param(key = "VOLTAGE", default = 0f64)]
    pub voltage: f64,
}

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct ZAxisClearanceRuleData {
    #[param(key = "GAP", default = MilCoord::default())]
    pub gap: MilCoord,
}

// ── Rule parsing dispatch ───────────────────────────────────────────────────

pub(crate) fn parse_rule(prefix: u16, params: &mut ParameterCollection) -> Result<PcbRule> {
    let base = PcbRuleBase::from_params(params)?;
    let kind_data = match base.rule_kind {
        RuleKind::Clearance => PcbRuleKindData::Clearance(ClearanceRuleData::from_params(params)?),
        RuleKind::ParallelSegment => PcbRuleKindData::ParallelSegment(ParallelSegmentRuleData::from_params(params)?),
        RuleKind::Width => PcbRuleKindData::Width(WidthRuleData::from_params(params)?),
        RuleKind::Length => PcbRuleKindData::Length(LengthRuleData::from_params(params)?),
        RuleKind::MatchedLengths => PcbRuleKindData::MatchedLengths(MatchedLengthsRuleData::from_params(params)?),
        RuleKind::DaisyChainStubLength => PcbRuleKindData::DaisyChainStubLength(DaisyChainStubLengthRuleData::from_params(params)?),
        RuleKind::PowerPlaneConnectStyle => PcbRuleKindData::PowerPlaneConnectStyle(PowerPlaneConnectStyleRuleData::from_params(params)?),
        RuleKind::RoutingTopology => PcbRuleKindData::RoutingTopology(RoutingTopologyRuleData::from_params(params)?),
        RuleKind::RoutingPriority => PcbRuleKindData::RoutingPriority(RoutingPriorityRuleData::from_params(params)?),
        RuleKind::RoutingLayers => PcbRuleKindData::RoutingLayers(RoutingLayersRuleData::from_params(params)?),
        RuleKind::RoutingCornerStyle => PcbRuleKindData::RoutingCornerStyle(RoutingCornerStyleRuleData::from_params(params)?),
        RuleKind::RoutingViaStyle => PcbRuleKindData::RoutingViaStyle(RoutingViaStyleRuleData::from_params(params)?),
        RuleKind::PowerPlaneClearance => PcbRuleKindData::PowerPlaneClearance(PowerPlaneClearanceRuleData::from_params(params)?),
        RuleKind::SolderMaskExpansion => PcbRuleKindData::SolderMaskExpansion(SolderMaskExpansionRuleData::from_params(params)?),
        RuleKind::PasteMaskExpansion => PcbRuleKindData::PasteMaskExpansion(PasteMaskExpansionRuleData::from_params(params)?),
        RuleKind::ShortCircuit => PcbRuleKindData::ShortCircuit(ShortCircuitRuleData::from_params(params)?),
        RuleKind::BrokenNets => PcbRuleKindData::BrokenNets(BrokenNetsRuleData::from_params(params)?),
        RuleKind::ViasUnderSmd => PcbRuleKindData::ViasUnderSmd(ViasUnderSmdRuleData::from_params(params)?),
        RuleKind::MaximumViaCount => PcbRuleKindData::MaximumViaCount(MaximumViaCountRuleData::from_params(params)?),
        RuleKind::MinimumAnnularRing => PcbRuleKindData::MinimumAnnularRing(MinimumAnnularRingRuleData::from_params(params)?),
        RuleKind::PolygonConnectStyle => PcbRuleKindData::PolygonConnectStyle(PolygonConnectStyleRuleData::from_params(params)?),
        RuleKind::AcuteAngle => PcbRuleKindData::AcuteAngle(AcuteAngleRuleData::from_params(params)?),
        RuleKind::ConfinementConstraint => PcbRuleKindData::ConfinementConstraint(ConfinementConstraintRuleData::from_params(params)?),
        RuleKind::SmdToCorner => PcbRuleKindData::SmdToCorner(SmdToCornerRuleData::from_params(params)?),
        RuleKind::ComponentClearance => PcbRuleKindData::ComponentClearance(ComponentClearanceRuleData::from_params(params)?),
        RuleKind::ComponentRotations => PcbRuleKindData::ComponentRotations(EmptyRuleData::from_params(params)?),
        RuleKind::PermittedLayers => PcbRuleKindData::PermittedLayers(EmptyRuleData::from_params(params)?),
        RuleKind::NetsToIgnore => PcbRuleKindData::NetsToIgnore(EmptyRuleData::from_params(params)?),
        RuleKind::SignalStimulus => PcbRuleKindData::SignalStimulus(SignalStimulusRuleData::from_params(params)?),
        RuleKind::OvershootFallingEdge => PcbRuleKindData::OvershootFallingEdge(OvershootUndershootRuleData::from_params(params)?),
        RuleKind::OvershootRisingEdge => PcbRuleKindData::OvershootRisingEdge(OvershootUndershootRuleData::from_params(params)?),
        RuleKind::UndershootFallingEdge => PcbRuleKindData::UndershootFallingEdge(OvershootUndershootRuleData::from_params(params)?),
        RuleKind::UndershootRisingEdge => PcbRuleKindData::UndershootRisingEdge(OvershootUndershootRuleData::from_params(params)?),
        RuleKind::MaxMinImpedance => PcbRuleKindData::MaxMinImpedance(MaxMinImpedanceRuleData::from_params(params)?),
        RuleKind::SignalTopValue => PcbRuleKindData::SignalTopValue(SignalValueRuleData::from_params(params)?),
        RuleKind::SignalBaseValue => PcbRuleKindData::SignalBaseValue(SignalValueRuleData::from_params(params)?),
        RuleKind::FlightTimeRisingEdge => PcbRuleKindData::FlightTimeRisingEdge(FlightTimeRuleData::from_params(params)?),
        RuleKind::FlightTimeFallingEdge => PcbRuleKindData::FlightTimeFallingEdge(FlightTimeRuleData::from_params(params)?),
        RuleKind::LayerStack => PcbRuleKindData::LayerStack(EmptyRuleData::from_params(params)?),
        RuleKind::MaxSlopeRisingEdge => PcbRuleKindData::MaxSlopeRisingEdge(SlopeRuleData::from_params(params)?),
        RuleKind::MaxSlopeFallingEdge => PcbRuleKindData::MaxSlopeFallingEdge(SlopeRuleData::from_params(params)?),
        RuleKind::SupplyNets => PcbRuleKindData::SupplyNets(SupplyNetsRuleData::from_params(params)?),
        RuleKind::MaxMinHoleSize => PcbRuleKindData::MaxMinHoleSize(MaxMinHoleSizeRuleData::from_params(params)?),
        RuleKind::FabricationTestpointStyle => PcbRuleKindData::FabricationTestpointStyle(TestpointStyleRuleData::from_params(params)?),
        RuleKind::FabricationTestpointUsage => PcbRuleKindData::FabricationTestpointUsage(TestpointUsageRuleData::from_params(params)?),
        RuleKind::UnconnectedPin => PcbRuleKindData::UnconnectedPin(EmptyRuleData::from_params(params)?),
        RuleKind::SmdToPlane => PcbRuleKindData::SmdToPlane(SmdToPlaneRuleData::from_params(params)?),
        RuleKind::SmdNeckDown => PcbRuleKindData::SmdNeckDown(SmdNeckDownRuleData::from_params(params)?),
        RuleKind::LayerPair => PcbRuleKindData::LayerPair(LayerPairRuleData::from_params(params)?),
        RuleKind::FanoutControl => PcbRuleKindData::FanoutControl(FanoutControlRuleData::from_params(params)?),
        RuleKind::MaxMinHeight => PcbRuleKindData::MaxMinHeight(MaxMinHeightRuleData::from_params(params)?),
        RuleKind::DifferentialPairsRouting => PcbRuleKindData::DifferentialPairsRouting(DiffPairsRoutingRuleData::from_params(params)?),
        RuleKind::HoleToHoleClearance => PcbRuleKindData::HoleToHoleClearance(HoleToHoleClearanceRuleData::from_params(params)?),
        RuleKind::MinimumSolderMaskSliver => PcbRuleKindData::MinimumSolderMaskSliver(MinimumSolderMaskSliverRuleData::from_params(params)?),
        RuleKind::SilkToSolderMaskClearance => PcbRuleKindData::SilkToSolderMaskClearance(SilkToSolderMaskClearanceRuleData::from_params(params)?),
        RuleKind::SilkToSilkClearance => PcbRuleKindData::SilkToSilkClearance(SilkToSilkClearanceRuleData::from_params(params)?),
        RuleKind::NetAntennae => PcbRuleKindData::NetAntennae(NetAntennaeRuleData::from_params(params)?),
        RuleKind::AssyTestPointStyle => PcbRuleKindData::AssyTestPointStyle(TestpointStyleRuleData::from_params(params)?),
        RuleKind::AssyTestPointUsage => PcbRuleKindData::AssyTestPointUsage(TestpointUsageRuleData::from_params(params)?),
        RuleKind::SilkToBoardRegionClearance => PcbRuleKindData::SilkToBoardRegionClearance(EmptyRuleData::from_params(params)?),
        RuleKind::SmdEntry => PcbRuleKindData::SmdEntry(SmdEntryRuleData::from_params(params)?),
        RuleKind::None => PcbRuleKindData::None(EmptyRuleData::from_params(params)?),
        RuleKind::UnpouredPolygon => PcbRuleKindData::UnpouredPolygon(UnpouredPolygonRuleData::from_params(params)?),
        RuleKind::BoardOutlineClearance => PcbRuleKindData::BoardOutlineClearance(BoardOutlineClearanceRuleData::from_params(params)?),
        RuleKind::BackDrilling => PcbRuleKindData::BackDrilling(BackDrillingRuleData::from_params(params)?),
        RuleKind::Creepage => PcbRuleKindData::Creepage(CreepageRuleData::from_params(params)?),
        RuleKind::ReturnPath => PcbRuleKindData::ReturnPath(ReturnPathRuleData::from_params(params)?),
        RuleKind::RoutingNeckDown => PcbRuleKindData::RoutingNeckDown(RoutingNeckDownRuleData::from_params(params)?),
        RuleKind::WireBonding => PcbRuleKindData::WireBonding(WireBondingRuleData::from_params(params)?),
        RuleKind::ZAxisClearance => PcbRuleKindData::ZAxisClearance(ZAxisClearanceRuleData::from_params(params)?),
        // #[non_exhaustive] requires wildcard
        _ => {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "RULEKIND".to_owned(),
                detail: format!("unimplemented rule kind: {:?}", base.rule_kind),
            });
        }
    };
    params.assert_exhausted()?;
    Ok(PcbRule {
        prefix,
        base,
        kind_data,
    })
}

// ── Violation types ─────────────────────────────────────────────────────────

/// Common fields shared by ALL violation records.
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct PcbViolationBase {
    #[param(key = "SELECTION", default = false)]
    pub selection: bool,
    #[param(key = "LAYER", default = String::new())]
    pub layer: String,
    #[param(key = "LOCKED", default = false)]
    pub locked: bool,
    #[param(key = "POLYGONOUTLINE", default = false)]
    pub polygon_outline: bool,
    #[param(key = "USERROUTED", default = true)]
    pub user_routed: bool,
    #[param(key = "KEEPOUT", default = false)]
    pub keepout: bool,
    #[param(key = "UNIONINDEX", default = 0u32)]
    pub union_index: u32,
    #[param(key = "RULEINDEX")]
    pub rule_index: u32,
    #[param(key = "PRIM1ID")]
    pub prim1_id: String,
    #[param(key = "PRIM1INDEX")]
    pub prim1_index: u32,
    #[param(key = "DESCRIPTION", default = String::new())]
    pub description: String,
    #[param(key = "INVOLVEDPRIMCOUNT", default = 0u32)]
    pub involved_prim_count: u32,
}

/// A typed violation record. The variant is determined by the CFB storage name.
#[derive(Debug)]
pub(crate) enum PcbViolation {
    /// Generic violation: base fields + any remaining params stored as raw strings.
    /// Used as initial implementation for all violation types until they can be
    /// individually verified against fixture data.
    Generic(GenericViolationData),
}

#[derive(Debug)]
pub(crate) struct GenericViolationData {
    pub base: PcbViolationBase,
    pub extra_params: Vec<(String, String)>,
}

pub(crate) fn parse_violation(
    _kind: ParamSectionKind,
    params: &mut ParameterCollection,
) -> Result<PcbViolation> {
    let base = PcbViolationBase::from_params(params)?;
    // Collect any remaining params into a flat list.
    // Phase 2 will replace this with per-violation-type typed structs.
    let extra_params = params.drain_remaining();
    Ok(PcbViolation::Generic(GenericViolationData {
        base,
        extra_params,
    }))
}

// ── Waived violations ───────────────────────────────────────────────────────

#[derive(FromParams, ToParams, Debug)]
pub(crate) struct WaivedViolation {
    #[param(key = "UNICODE", default = String::new())]
    pub unicode: String,
    #[param(key = "RULEINDEX")]
    pub rule_index: u32,
    #[param(key = "PRIM1KIND")]
    pub prim1_kind: String,
    #[param(key = "PRIM1INDEX")]
    pub prim1_index: u32,
    #[param(key = "PRIM2KIND", optional)]
    pub prim2_kind: Option<String>,
    #[param(key = "PRIM2INDEX", optional)]
    pub prim2_index: Option<u32>,
    #[param(key = "CREATEDAT", default = String::new())]
    pub created_at: String,
    #[param(key = "AUTHORID", default = String::new())]
    pub author_id: String,
    #[param(key = "AUTHORTITLE", default = String::new())]
    pub author_title: String,
    #[param(key = "SOURCE", default = String::new())]
    pub source: String,
    #[param(key = "COMMENT", default = String::new())]
    pub comment: String,
}

// ── DRC Options ─────────────────────────────────────────────────────────────

/// Design Rule Checker Options (single record from DesignRuleCheckerOptions6/Data).
/// This struct captures only the known fields; remaining params are drained.
#[derive(Debug)]
pub(crate) struct DrcOptions {
    pub record: String,
    pub extra_params: Vec<(String, String)>,
}

impl DrcOptions {
    pub(crate) fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        let record: String = params.remove_with_default(
            "RECORD",
            "DesignRuleCheckerOptions".to_string(),
        )?;
        // DRC options has many boolean flags and list params.
        // Drain remaining for now — Phase 3 will fully type them.
        let extra_params = params.drain_remaining();
        Ok(Self {
            record,
            extra_params,
        })
    }
}

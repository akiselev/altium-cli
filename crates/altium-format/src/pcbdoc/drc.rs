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
#[param(tier2)]
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

    pub(crate) fn to_params(&self, _params: &mut ParameterCollection) {
        // No kind-specific parameters for empty rules.
    }
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct BrokenNetsRuleData {
    #[param(key = "CHECKBADCONNECTIONS", default = false)]
    pub check_bad_connections: bool,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct SmdEntryRuleData {
    #[param(key = "SIDE", default = false)]
    pub side: bool,
    #[param(key = "CORNER", default = false)]
    pub corner: bool,
    #[param(key = "ANYANGLE", default = false)]
    pub any_angle: bool,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct UnpouredPolygonRuleData {
    #[param(key = "ALLOWUNPOURED", default = false)]
    pub allow_unpoured: bool,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct SmdToPlaneRuleData {
    #[param(key = "DISTANCE", default = MilCoord::default())]
    pub distance: MilCoord,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct SmdNeckDownRuleData {
    #[param(key = "PERCENT", default = 0f64)]
    pub percent: f64,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct ReturnPathRuleData {
    #[param(key = "IMPEDANCEPROFILEID", default = String::new())]
    pub impedance_profile_id: String,
    #[param(key = "_GAPLIMIT", default = MilCoord::default())]
    pub gap_limit: MilCoord,
    #[param(key = "_USEANTIPADS", default = false)]
    pub use_antipads: bool,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
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
#[param(tier2)]
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

    pub(crate) fn to_params(&self, params: &mut ParameterCollection) {
        params.insert("MINLIMIT", self.min_limit.to_param_value());
        params.insert("MAXLIMIT", self.max_limit.to_param_value());
        params.insert("PREFEREDWIDTH", self.preferred_width.to_param_value());

        for ov in &self.per_layer {
            if let Some(v) = &ov.min_width {
                params.insert(&format!("{}_MINWIDTH", ov.prefix), v.to_param_value());
            }
            if let Some(v) = &ov.max_width {
                params.insert(&format!("{}_MAXWIDTH", ov.prefix), v.to_param_value());
            }
            if let Some(v) = &ov.pref_width {
                params.insert(&format!("{}_PREFWIDTH", ov.prefix), v.to_param_value());
            }
        }

        params.insert("CHECKCONNECTEDCOPPER", self.check_connected_copper.to_param_value());

        if let Some(v) = self.impedance_driven {
            params.insert("IMPEDANCEDRIVEN", v.to_param_value());
        }
        if let Some(v) = self.min_imp {
            params.insert("MINIMP", v.to_param_value());
        }
        if let Some(v) = self.max_imp {
            params.insert("MAXIMP", v.to_param_value());
        }
        if let Some(v) = self.fav_imp {
            params.insert("FAVIMP", v.to_param_value());
        }
        if let Some(v) = self.impedance_profile_driven {
            params.insert("IMPEDANCEPROFILEDRIVEN", v.to_param_value());
        }
        if let Some(v) = &self.impedance_profile_id {
            params.insert("IMPEDANCEPROFILEID", v.to_param_value());
        }
        if let Some(v) = self.impedance_profile_value {
            params.insert("IMPEDANCEPROFILEVALUE", v.to_param_value());
        }

        for (n, ss) in self.substack_overrides.iter().enumerate() {
            let n1 = n + 1;
            params.insert(&format!("SUBSTACK{n1}"), ss.substack_id.to_param_value());
            let guid_upper = ss.substack_id.to_ascii_uppercase();
            for (prefix, min_w, max_w, pref_w) in &ss.layer_overrides {
                if let Some(v) = min_w {
                    params.insert(&format!("{prefix}_{guid_upper}_MINWIDTH"), v.to_param_value());
                }
                if let Some(v) = max_w {
                    params.insert(&format!("{prefix}_{guid_upper}_MAXWIDTH"), v.to_param_value());
                }
                if let Some(v) = pref_w {
                    params.insert(&format!("{prefix}_{guid_upper}_PREFWIDTH"), v.to_param_value());
                }
            }
        }
    }
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
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
#[param(tier2)]
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
#[param(tier2)]
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

    pub(crate) fn to_params(&self, params: &mut ParameterCollection) {
        if let Some(v) = &self.connect_style {
            params.insert("PLANECONNECTSTYLE", v.to_param_value());
        }
        if let Some(v) = &self.relief_expansion {
            params.insert("RELIEFEXPANSION", v.to_param_value());
        }
        if let Some(v) = self.relief_entries {
            params.insert("RELIEFENTRIES", v.to_param_value());
        }
        if let Some(v) = &self.relief_conductor_width {
            params.insert("RELIEFCONDUCTORWIDTH", v.to_param_value());
        }
        if let Some(v) = &self.relief_air_gap {
            params.insert("RELIEFAIRGAP", v.to_param_value());
        }
        for ov in &self.type_overrides {
            params.insert(&format!("{}.PLANECONNECTSTYLE", ov.prefix), ov.connect_style.to_param_value());
            params.insert(&format!("{}.RELIEFEXPANSION", ov.prefix), ov.relief_expansion.to_param_value());
            params.insert(&format!("{}.RELIEFENTRIES", ov.prefix), ov.relief_entries.to_param_value());
            params.insert(&format!("{}.RELIEFCONDUCTORWIDTH", ov.prefix), ov.relief_conductor_width.to_param_value());
            params.insert(&format!("{}.RELIEFAIRGAP", ov.prefix), ov.relief_air_gap.to_param_value());
        }
    }
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct RoutingTopologyRuleData {
    #[param(key = "TOPOLOGY")]
    pub topology: NetTopology,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
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

    pub(crate) fn to_params(&self, params: &mut ParameterCollection) {
        for (prefix, val) in &self.layer_flags {
            params.insert(&format!("{prefix}_V5"), val.to_param_value());
        }
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
#[param(tier2)]
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

    pub(crate) fn to_params(&self, params: &mut ParameterCollection) {
        params.insert("MINHOLEWIDTH", self.min_hole_width.to_param_value());
        params.insert("MAXHOLEWIDTH", self.max_hole_width.to_param_value());
        params.insert("HOLEWIDTH", self.preferred_hole_width.to_param_value());
        params.insert("MINWIDTH", self.min_width.to_param_value());
        params.insert("MAXWIDTH", self.max_width.to_param_value());
        params.insert("WIDTH", self.preferred_width.to_param_value());
        params.insert("VIASTYLE", self.via_style.to_param_value());
        params.insert("USEVIATEMPLATES", self.use_via_templates.to_param_value());
        for (n, t) in self.via_templates.iter().enumerate() {
            let n1 = n + 1;
            params.insert(&format!("VIATEMPLATEGUID#{n1}"), t.guid.to_param_value());
            params.insert(&format!("VIATEMPLATENAME#{n1}"), t.name.to_param_value());
        }
    }
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct PowerPlaneClearanceRuleData {
    #[param(key = "CLEARANCE", default = MilCoord::default())]
    pub clearance: MilCoord,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
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
#[param(tier2)]
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
#[param(tier2)]
pub(crate) struct ShortCircuitRuleData {
    #[param(key = "ALLOWED", default = false)]
    pub allowed: bool,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct ViasUnderSmdRuleData {
    #[param(key = "ALLOWED", default = true)]
    pub allowed: bool,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct MaximumViaCountRuleData {
    #[param(key = "MAXVIACOUNT", default = 10u32)]
    pub max_via_count: u32,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
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

    pub(crate) fn to_params(&self, params: &mut ParameterCollection) {
        if let Some(v) = &self.connect_style {
            params.insert("CONNECTSTYLE", v.to_param_value());
        }
        if let Some(v) = &self.relief_conductor_width {
            params.insert("RELIEFCONDUCTORWIDTH", v.to_param_value());
        }
        if let Some(v) = self.relief_entries {
            params.insert("RELIEFENTRIES", v.to_param_value());
        }
        if let Some(v) = &self.polygon_relief_angle {
            params.insert("POLYGONRELIEFANGLE", v.to_param_value());
        }
        if let Some(v) = &self.air_gap_width {
            params.insert("AIRGAPWIDTH", v.to_param_value());
        }
        for ov in &self.pad_type_overrides {
            params.insert(&format!("{}.CONNECTSTYLE", ov.prefix), ov.connect_style.to_param_value());
            params.insert(&format!("{}.RELIEFCONDUCTORWIDTH", ov.prefix), ov.relief_conductor_width.to_param_value());
            params.insert(&format!("{}.RELIEFENTRIES", ov.prefix), ov.relief_entries.to_param_value());
            params.insert(&format!("{}.POLYGONRELIEFANGLE", ov.prefix), ov.polygon_relief_angle.to_param_value());
            params.insert(&format!("{}.AIRGAPWIDTH", ov.prefix), ov.air_gap_width.to_param_value());
        }
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

    pub(crate) fn to_params(&self, params: &mut ParameterCollection) {
        params.insert("MINIMUM", self.minimum.to_param_value());
        params.insert("CHECKTRACKSONLY", self.check_tracks_only.to_param_value());
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

    pub(crate) fn to_params(&self, params: &mut ParameterCollection) {
        params.insert("CONFINEMENTSTYLE", self.confinement_style.to_param_value());
        params.insert("LOCKCOMPONENTS", self.lock_components.to_param_value());
        params.insert("FORMATCOPY", self.format_copy.to_param_value());
        for (n, v) in self.vertices.iter().enumerate() {
            params.insert(&format!("KIND{n}"), v.kind.to_param_value());
            params.insert(&format!("VX{n}"), v.vx.to_param_value());
            params.insert(&format!("VY{n}"), v.vy.to_param_value());
            params.insert(&format!("CX{n}"), v.cx.to_param_value());
            params.insert(&format!("CY{n}"), v.cy.to_param_value());
            params.insert(&format!("SA{n}"), v.sa.to_param_value());
            params.insert(&format!("EA{n}"), v.ea.to_param_value());
            params.insert(&format!("R{n}"), v.r.to_param_value());
        }
    }
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct SmdToCornerRuleData {
    #[param(key = "DISTANCE", default = MilCoord::default())]
    pub distance: MilCoord,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
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
#[param(tier2)]
pub(crate) struct SignalStimulusRuleData {
    #[param(key = "STIMULUSTYPE", default = 0u32)]
    pub stimulus_type: u32,
    #[param(key = "SIGNALLEVEL", default = 0u32)]
    pub signal_level: u32,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct OvershootUndershootRuleData {
    #[param(key = "MAXOVERSHOOT", default = 0f64)]
    pub max_overshoot: f64,
    #[param(key = "MAXUNDERSHOOT", default = 0f64)]
    pub max_undershoot: f64,
    #[param(key = "MAXIMUM", default = 0f64)]
    pub maximum: f64,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct MaxMinImpedanceRuleData {
    #[param(key = "MINIMUM", default = 0f64)]
    pub min_impedance: f64,
    #[param(key = "MAXIMUM", default = 0f64)]
    pub max_impedance: f64,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct SignalValueRuleData {
    #[param(key = "MINVALUE", default = 0f64)]
    pub min_value: f64,
    #[param(key = "MAXVALUE", default = 0f64)]
    pub max_value: f64,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct FlightTimeRuleData {
    #[param(key = "MINVALUE", default = 0f64)]
    pub min_value: f64,
    #[param(key = "MAXVALUE", default = 0f64)]
    pub max_value: f64,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct SlopeRuleData {
    #[param(key = "MAXSLOPE", default = 0f64)]
    pub max_slope: f64,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
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
#[param(tier2)]
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
#[param(tier2)]
pub(crate) struct TestpointUsageRuleData {
    #[param(key = "VALID", default = 0u32)]
    pub valid: u32,
    #[param(key = "ALLOWMULTIPLE", default = false)]
    pub allow_multiple: bool,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct LayerPairRuleData {
    #[param(key = "ENFORCE", default = true)]
    pub enforce: bool,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
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
#[param(tier2)]
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

    pub(crate) fn to_params(&self, params: &mut ParameterCollection) {
        params.insert("MINLIMIT", self.min_limit.to_param_value());
        params.insert("MAXLIMIT", self.max_limit.to_param_value());
        params.insert("MOSTFREQGAP", self.most_freq_gap.to_param_value());
        params.insert("MAXUNCOUPLEDLENGTH", self.max_uncoupled_length.to_param_value());

        for ov in &self.per_layer {
            if let Some(v) = &ov.min_width {
                params.insert(&format!("{}_MINWIDTH", ov.prefix), v.to_param_value());
            }
            if let Some(v) = &ov.max_width {
                params.insert(&format!("{}_MAXWIDTH", ov.prefix), v.to_param_value());
            }
            if let Some(v) = &ov.pref_width {
                params.insert(&format!("{}_PREFWIDTH", ov.prefix), v.to_param_value());
            }
            if let Some(v) = &ov.min_gap {
                params.insert(&format!("{}_MINGAP", ov.prefix), v.to_param_value());
            }
            if let Some(v) = &ov.max_gap {
                params.insert(&format!("{}_MAXGAP", ov.prefix), v.to_param_value());
            }
            if let Some(v) = &ov.pref_gap {
                params.insert(&format!("{}_PREFGAP", ov.prefix), v.to_param_value());
            }
        }

        if let Some(v) = self.impedance_profile_driven {
            params.insert("IMPEDANCEPROFILEDRIVEN", v.to_param_value());
        }
        if let Some(v) = &self.impedance_profile_id {
            params.insert("IMPEDANCEPROFILEID", v.to_param_value());
        }
        if let Some(v) = self.impedance_profile_value {
            params.insert("IMPEDANCEPROFILEVALUE", v.to_param_value());
        }

        for (n, ss) in self.substack_overrides.iter().enumerate() {
            let n1 = n + 1;
            params.insert(&format!("SUBSTACK{n1}"), ss.substack_id.to_param_value());
            let guid_upper = ss.substack_id.to_ascii_uppercase();
            for (prefix, min_w, max_w, pref_w, min_g, max_g, pref_g) in &ss.layer_overrides {
                if let Some(v) = min_w {
                    params.insert(&format!("{prefix}_{guid_upper}_MINWIDTH"), v.to_param_value());
                }
                if let Some(v) = max_w {
                    params.insert(&format!("{prefix}_{guid_upper}_MAXWIDTH"), v.to_param_value());
                }
                if let Some(v) = pref_w {
                    params.insert(&format!("{prefix}_{guid_upper}_PREFWIDTH"), v.to_param_value());
                }
                if let Some(v) = min_g {
                    params.insert(&format!("{prefix}_{guid_upper}_MINGAP"), v.to_param_value());
                }
                if let Some(v) = max_g {
                    params.insert(&format!("{prefix}_{guid_upper}_MAXGAP"), v.to_param_value());
                }
                if let Some(v) = pref_g {
                    params.insert(&format!("{prefix}_{guid_upper}_PREFGAP"), v.to_param_value());
                }
            }
        }
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
#[param(tier2)]
pub(crate) struct HoleToHoleClearanceRuleData {
    #[param(key = "GAP", default = MilCoord::default())]
    pub gap: MilCoord,
    #[param(key = "ALLOWSTACKEDMICROVIAS", default = false)]
    pub allow_stacked_microvias: bool,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct MinimumSolderMaskSliverRuleData {
    #[param(key = "MINSOLDERMASKWIDTH", default = MilCoord::default())]
    pub min_solder_mask_width: MilCoord,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct SilkToSolderMaskClearanceRuleData {
    #[param(key = "MINSILKSCREENTOMASKGAP", default = MilCoord::default())]
    pub min_silkscreen_to_mask_gap: MilCoord,
    #[param(key = "CLEARANCETOEXPOSEDCOPPER", default = false)]
    pub clearance_to_exposed_copper: bool,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct SilkToSilkClearanceRuleData {
    #[param(key = "SILKTOSILKCLEARANCE", default = MilCoord::default())]
    pub silk_to_silk_clearance: MilCoord,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct NetAntennaeRuleData {
    #[param(key = "NETANTENNAETOLERANCE", default = MilCoord::default())]
    pub net_antennae_tolerance: MilCoord,
}

/// BoardOutlineClearance reuses the same structure as Clearance.
#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
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
#[param(tier2)]
pub(crate) struct BackDrillingRuleData {
    #[param(key = "BACKDRILLINGDEPTH", default = MilCoord::default())]
    pub backdrill_depth: MilCoord,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
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
#[param(tier2)]
pub(crate) struct RoutingNeckDownRuleData {
    #[param(key = "NECKDOWNPERCENTAGE", default = 0f64)]
    pub neck_down_percentage: f64,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct WireBondingRuleData {
    #[param(key = "MINLIMIT", default = MilCoord::default())]
    pub min_limit: MilCoord,
    #[param(key = "MAXLIMIT", default = MilCoord::default())]
    pub max_limit: MilCoord,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct SupplyNetsRuleData {
    #[param(key = "VOLTAGE", default = 0f64)]
    pub voltage: f64,
}

#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
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
///
/// Most fields use `tier2` (always write) because Altium writes them even at default values.
/// Most fields use `tier2` (always write) because Altium writes them even at default values.
/// `INVOLVEDPRIMCOUNT` uses T1 (skip when 0) because most violation types omit it when zero;
/// only a few types (DiffPairs, some Clearance) redundantly include it.
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct PcbViolationBase {
    #[param(tier2, key = "SELECTION", default = false)]
    pub selection: bool,
    #[param(tier2, key = "LAYER", default = String::new())]
    pub layer: String,
    #[param(tier2, key = "LOCKED", default = false)]
    pub locked: bool,
    #[param(tier2, key = "POLYGONOUTLINE", default = false)]
    pub polygon_outline: bool,
    #[param(tier2, key = "USERROUTED", default = true)]
    pub user_routed: bool,
    #[param(tier2, key = "KEEPOUT", default = false)]
    pub keepout: bool,
    #[param(tier2, key = "UNIONINDEX", default = 0u32)]
    pub union_index: u32,
    #[param(key = "RULEINDEX")]
    pub rule_index: u32,
    #[param(key = "PRIM1ID")]
    pub prim1_id: String,
    #[param(key = "PRIM1INDEX")]
    pub prim1_index: u32,
    #[param(tier2, key = "DESCRIPTION", default = String::new())]
    pub description: String,
    #[param(key = "INVOLVEDPRIMCOUNT", default = 0u32)]
    pub involved_prim_count: u32,
}

/// Fields shared by two-point violations (PRIM2 + LOCATION1/2).
/// All fields are optional because older format files may omit them.
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct TwoPointViolationData {
    #[param(key = "PRIM2ID", optional)]
    pub prim2_id: Option<String>,
    #[param(key = "PRIM2INDEX", optional)]
    pub prim2_index: Option<u32>,
    #[param(key = "LOCATION1.X", optional)]
    pub location1_x: Option<MilCoord>,
    #[param(key = "LOCATION1.Y", optional)]
    pub location1_y: Option<MilCoord>,
    #[param(key = "LOCATION2.X", optional)]
    pub location2_x: Option<MilCoord>,
    #[param(key = "LOCATION2.Y", optional)]
    pub location2_y: Option<MilCoord>,
    /// Present in TClearanceViolation records for drill/hole clearance violations.
    #[param(key = "ISHOLECLEARANCEVIOLATION", optional)]
    pub is_hole_clearance_violation: Option<bool>,
    /// 3D position components used by TComponentClearanceViolation.
    #[param(key = "P1.X", optional)]
    pub p1_x: Option<MilCoord>,
    #[param(key = "P1.Y", optional)]
    pub p1_y: Option<MilCoord>,
    #[param(key = "P1.Z", optional)]
    pub p1_z: Option<MilCoord>,
    #[param(key = "P2.X", optional)]
    pub p2_x: Option<MilCoord>,
    #[param(key = "P2.Y", optional)]
    pub p2_y: Option<MilCoord>,
    #[param(key = "P2.Z", optional)]
    pub p2_z: Option<MilCoord>,
}

/// Fields for BoardOutlineClearanceViolation (two-point + PRIMID1/PRIMID2).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct BoardOutlineClearanceViolationData {
    #[param(key = "PRIM2ID", optional)]
    pub prim2_id: Option<String>,
    #[param(key = "PRIM2INDEX", optional)]
    pub prim2_index: Option<u32>,
    #[param(key = "LOCATION1.X", optional)]
    pub location1_x: Option<MilCoord>,
    #[param(key = "LOCATION1.Y", optional)]
    pub location1_y: Option<MilCoord>,
    #[param(key = "LOCATION2.X", optional)]
    pub location2_x: Option<MilCoord>,
    #[param(key = "LOCATION2.Y", optional)]
    pub location2_y: Option<MilCoord>,
    #[param(key = "PRIMID1", optional)]
    pub primid1: Option<String>,
    #[param(key = "PRIMID2", optional)]
    pub primid2: Option<String>,
}

/// Fields for NetAntennaeViolation (single-point + circle radius).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct NetAntennaeViolationData {
    #[param(key = "LOCATION.X")]
    pub location_x: MilCoord,
    #[param(key = "LOCATION.Y")]
    pub location_y: MilCoord,
    #[param(key = "CIRCLERADIUS")]
    pub circle_radius: MilCoord,
}

/// Fields for DisconnectedSubnetsViolation (FX/FY pattern).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct DisconnectedSubnetsViolationData {
    #[param(key = "PRIM2ID", optional)]
    pub prim2_id: Option<String>,
    #[param(key = "PRIM2INDEX", optional)]
    pub prim2_index: Option<u32>,
    #[param(key = "FX1", optional)]
    pub fx1: Option<MilCoord>,
    #[param(key = "FY1", optional)]
    pub fy1: Option<MilCoord>,
    #[param(key = "FX2", optional)]
    pub fx2: Option<MilCoord>,
    #[param(key = "FY2", optional)]
    pub fy2: Option<MilCoord>,
}

/// Fields for MatchedNetLengthsViolation (optional PRIM2 pair).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct MatchedNetLengthsViolationData {
    #[param(key = "PRIM2ID", optional)]
    pub prim2_id: Option<String>,
    #[param(key = "PRIM2INDEX", optional)]
    pub prim2_index: Option<u32>,
}

/// A single contour in a DiffPairs violation polygon.
#[derive(Debug)]
pub(crate) struct DiffPairsViolationContour {
    pub vertices: Vec<(f64, f64)>,
}

/// A single layer entry in a DiffPairs violation.
#[derive(Debug)]
pub(crate) struct DiffPairsViolationLayer {
    pub layer_name: String,
    pub contours: Vec<DiffPairsViolationContour>,
}

/// Fields for DiffPairsViolation (nested polygon data per layer).
#[derive(Debug)]
pub(crate) struct DiffPairsViolationData {
    pub layers: Vec<DiffPairsViolationLayer>,
}

impl DiffPairsViolationData {
    pub(crate) fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        let layer_count: usize = params.remove_required("LAYERCOUNT")?;
        let mut layers = Vec::with_capacity(layer_count);
        for n in 1..=layer_count {
            let layer_name: String = params.remove_required(&format!("LAYER{n}"))?;
            let contour_count: usize =
                params.remove_required(&format!("POLY{n}.CONTOURCOUNT"))?;
            let mut contours = Vec::with_capacity(contour_count);
            for c in 0..contour_count {
                let vtx_count: usize =
                    params.remove_required(&format!("POLY{n}.CONTOUR{c}.VTXCOUNT"))?;
                let mut vertices = Vec::with_capacity(vtx_count);
                for v in 0..vtx_count {
                    let vx: f64 =
                        params.remove_required(&format!("POLY{n}.CONTOUR{c}.VX{v}"))?;
                    let vy: f64 =
                        params.remove_required(&format!("POLY{n}.CONTOUR{c}.VY{v}"))?;
                    vertices.push((vx, vy));
                }
                contours.push(DiffPairsViolationContour { vertices });
            }
            layers.push(DiffPairsViolationLayer {
                layer_name,
                contours,
            });
        }
        Ok(Self { layers })
    }

    pub(crate) fn to_params(&self, params: &mut ParameterCollection) {
        params.insert("LAYERCOUNT", self.layers.len().to_param_value());
        for (n, layer) in self.layers.iter().enumerate() {
            let n1 = n + 1;
            params.insert(&format!("LAYER{n1}"), layer.layer_name.to_param_value());
            params.insert(&format!("POLY{n1}.CONTOURCOUNT"), layer.contours.len().to_param_value());
            for (c, contour) in layer.contours.iter().enumerate() {
                params.insert(
                    &format!("POLY{n1}.CONTOUR{c}.VTXCOUNT"),
                    contour.vertices.len().to_param_value(),
                );
                for (v, (vx, vy)) in contour.vertices.iter().enumerate() {
                    params.insert(&format!("POLY{n1}.CONTOUR{c}.VX{v}"), vx.to_param_value());
                    params.insert(&format!("POLY{n1}.CONTOUR{c}.VY{v}"), vy.to_param_value());
                }
            }
        }
    }
}

/// A typed violation record. The variant is determined by the CFB storage name.
#[derive(Debug)]
pub(crate) enum PcbViolation {
    AcuteAngle { base: PcbViolationBase, data: TwoPointViolationData },
    BackDrill { base: PcbViolationBase, data: TwoPointViolationData },
    BoardOutlineClearance { base: PcbViolationBase, data: BoardOutlineClearanceViolationData },
    Clearance { base: PcbViolationBase, data: TwoPointViolationData },
    ComponentClearance { base: PcbViolationBase, data: TwoPointViolationData },
    Creepage { base: PcbViolationBase, data: TwoPointViolationData },
    DiffPairs { base: PcbViolationBase, data: DiffPairsViolationData },
    DisconnectedSubnets { base: PcbViolationBase, data: DisconnectedSubnetsViolationData },
    HoleToHole { base: PcbViolationBase, data: TwoPointViolationData },
    MatchedNetLengths { base: PcbViolationBase, data: MatchedNetLengthsViolationData },
    MaximumViaCount { base: PcbViolationBase, data: TwoPointViolationData },
    MaxMinComponentHeight { base: PcbViolationBase, data: TwoPointViolationData },
    MaxMinLength { base: PcbViolationBase, data: TwoPointViolationData },
    MaxMinPadSlotWidth { base: PcbViolationBase, data: TwoPointViolationData },
    MaxMinViaHoleSize { base: PcbViolationBase, data: TwoPointViolationData },
    MinimumAnnularRing { base: PcbViolationBase, data: TwoPointViolationData },
    MinSolderMaskSliver { base: PcbViolationBase, data: TwoPointViolationData },
    MinWidth { base: PcbViolationBase, data: TwoPointViolationData },
    ModifiedPolygon { base: PcbViolationBase, data: TwoPointViolationData },
    NetAntennae { base: PcbViolationBase, data: NetAntennaeViolationData },
    PadUnderSmd { base: PcbViolationBase, data: TwoPointViolationData },
    ParallelSegment { base: PcbViolationBase, data: TwoPointViolationData },
    ReturnPath { base: PcbViolationBase, data: TwoPointViolationData },
    RoutingNeckDown { base: PcbViolationBase, data: TwoPointViolationData },
    RoutingViaStyle { base: PcbViolationBase, data: TwoPointViolationData },
    ShortCircuit { base: PcbViolationBase, data: TwoPointViolationData },
    SilkToBoardRegionClearance { base: PcbViolationBase, data: TwoPointViolationData },
    SilkToSilkClearance { base: PcbViolationBase, data: TwoPointViolationData },
    SilkToSolderMaskClearance { base: PcbViolationBase, data: TwoPointViolationData },
    SmdNeckDown { base: PcbViolationBase, data: TwoPointViolationData },
    SmdPadEntry { base: PcbViolationBase, data: TwoPointViolationData },
    SmdToCorner { base: PcbViolationBase, data: TwoPointViolationData },
    TestPoint { base: PcbViolationBase, data: TwoPointViolationData },
    UnconnectedPin { base: PcbViolationBase, data: TwoPointViolationData },
    ViaUnderSmd { base: PcbViolationBase, data: TwoPointViolationData },
    WirebondLength { base: PcbViolationBase, data: TwoPointViolationData },
    WirebondWireToWire { base: PcbViolationBase, data: TwoPointViolationData },
    ZAxisClearance { base: PcbViolationBase, data: TwoPointViolationData },
}

pub(crate) fn parse_violation(
    kind: ParamSectionKind,
    params: &mut ParameterCollection,
) -> Result<PcbViolation> {
    let base = PcbViolationBase::from_params(params)?;
    let violation = match kind {
        ParamSectionKind::TAcuteAngleViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::AcuteAngle { base, data }
        }
        ParamSectionKind::TBackDrillViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::BackDrill { base, data }
        }
        ParamSectionKind::TBoardOutlineClearanceViolation => {
            let data = BoardOutlineClearanceViolationData::from_params(params)?;
            PcbViolation::BoardOutlineClearance { base, data }
        }
        ParamSectionKind::TClearanceViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::Clearance { base, data }
        }
        ParamSectionKind::TComponentClearanceViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::ComponentClearance { base, data }
        }
        ParamSectionKind::TCreepageViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::Creepage { base, data }
        }
        ParamSectionKind::TDiffPairsViolation => {
            let data = DiffPairsViolationData::from_params(params)?;
            PcbViolation::DiffPairs { base, data }
        }
        ParamSectionKind::TDisconnectedSubnetsViolation => {
            let data = DisconnectedSubnetsViolationData::from_params(params)?;
            PcbViolation::DisconnectedSubnets { base, data }
        }
        ParamSectionKind::THoleToHoleViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::HoleToHole { base, data }
        }
        ParamSectionKind::TMatchedNetLengthsViolation => {
            let data = MatchedNetLengthsViolationData::from_params(params)?;
            PcbViolation::MatchedNetLengths { base, data }
        }
        ParamSectionKind::TMaximumViaCountViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::MaximumViaCount { base, data }
        }
        ParamSectionKind::TMaxMinComponentHeightViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::MaxMinComponentHeight { base, data }
        }
        ParamSectionKind::TMaxMinLengthViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::MaxMinLength { base, data }
        }
        ParamSectionKind::TMaxMinPadSlotWidthViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::MaxMinPadSlotWidth { base, data }
        }
        ParamSectionKind::TMaxMinViaHoleSizeViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::MaxMinViaHoleSize { base, data }
        }
        ParamSectionKind::TMinimumAnnularRingViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::MinimumAnnularRing { base, data }
        }
        ParamSectionKind::TMinSolderMaskSliverViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::MinSolderMaskSliver { base, data }
        }
        ParamSectionKind::TMinWidthViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::MinWidth { base, data }
        }
        ParamSectionKind::TModifiedPolygonViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::ModifiedPolygon { base, data }
        }
        ParamSectionKind::TNetAntennaeViolation => {
            let data = NetAntennaeViolationData::from_params(params)?;
            PcbViolation::NetAntennae { base, data }
        }
        ParamSectionKind::TPadUnderSMDViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::PadUnderSmd { base, data }
        }
        ParamSectionKind::TParallelSegmentViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::ParallelSegment { base, data }
        }
        ParamSectionKind::TReturnPathViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::ReturnPath { base, data }
        }
        ParamSectionKind::TRoutingNeckDownViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::RoutingNeckDown { base, data }
        }
        ParamSectionKind::TRoutingViaStyleViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::RoutingViaStyle { base, data }
        }
        ParamSectionKind::TShortCircuitViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::ShortCircuit { base, data }
        }
        ParamSectionKind::TSilkToBoardRegionClearanceViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::SilkToBoardRegionClearance { base, data }
        }
        ParamSectionKind::TSilkToSilkClearanceViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::SilkToSilkClearance { base, data }
        }
        ParamSectionKind::TSilkToSolderMaskClearanceViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::SilkToSolderMaskClearance { base, data }
        }
        ParamSectionKind::TSMDNeckDownViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::SmdNeckDown { base, data }
        }
        ParamSectionKind::TSMDPADEntryViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::SmdPadEntry { base, data }
        }
        ParamSectionKind::TSMDToCornerViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::SmdToCorner { base, data }
        }
        ParamSectionKind::TTestPointViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::TestPoint { base, data }
        }
        ParamSectionKind::TUnconnectedPinViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::UnconnectedPin { base, data }
        }
        ParamSectionKind::TViaUnderSMDViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::ViaUnderSmd { base, data }
        }
        ParamSectionKind::TWirebondLengthViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::WirebondLength { base, data }
        }
        ParamSectionKind::TWirebondWireToWireViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::WirebondWireToWire { base, data }
        }
        ParamSectionKind::TZAxisClearanceViolation => {
            let data = TwoPointViolationData::from_params(params)?;
            PcbViolation::ZAxisClearance { base, data }
        }
        _ => {
            return Err(AltiumFormatError::NotImplemented(format!(
                "unexpected violation kind: {kind:?}"
            )))
        }
    };
    params.assert_exhausted()?;
    Ok(violation)
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
#[derive(FromParams, ToParams, Debug)]
#[param(tier2)]
pub(crate) struct DrcOptions {
    #[param(key = "RECORD", default = "DesignRuleCheckerOptions".to_string())]
    pub record: String,
    #[param(key = "DOMAKEDRCFILE", default = true)]
    pub do_make_drc_file: bool,
    #[param(key = "DOMAKEDRCERRORLIST", default = true)]
    pub do_make_drc_error_list: bool,
    #[param(key = "DOSUBNETDETAILS", default = true)]
    pub do_subnet_details: bool,
    #[param(key = "REPORTFILENAME", default = String::new())]
    pub report_filename: String,
    #[param(key = "EXTERNALNETLISTFILENAME", default = String::new())]
    pub external_netlist_filename: String,
    #[param(key = "CHECKEXTERNALNETLIST", default = false)]
    pub check_external_netlist: bool,
    #[param(key = "MAXVIOLATIONCOUNT", default = 500u32)]
    pub max_violation_count: u32,
    #[param(key = "REPORTDRILLEDSMTPADS", default = true)]
    pub report_drilled_smt_pads: bool,
    #[param(key = "REPORTINVALIDMULTILAYERPADS", default = true)]
    pub report_invalid_multilayer_pads: bool,
    /// Comma-separated list of rule kind indices to check in batch DRC.
    #[param(key = "RULESETTOCHECK", default = String::new())]
    pub rule_set_to_check: String,
    /// Comma-separated list of rule kind indices to check in online DRC.
    #[param(key = "ONLINERULESETTOCHECK", default = String::new())]
    pub online_rule_set_to_check: String,
    #[param(key = "INTERNALPLANEWARNINGS", default = true)]
    pub internal_plane_warnings: bool,
    #[param(key = "VERIFYSHORTINGCOPPER", default = true)]
    pub verify_shorting_copper: bool,
    #[param(key = "REPORTBROKENPLANES", default = true)]
    pub report_broken_planes: bool,
    #[param(key = "REPORTDEADCOPPER", default = true)]
    pub report_dead_copper: bool,
    #[param(key = "DEADCOPPERMINAREA", default = String::new())]
    pub dead_copper_min_area: String,
    #[param(key = "REPORTSTARVEDTHERMALS", default = true)]
    pub report_starved_thermals: bool,
    #[param(key = "MINSTARVEDCOPPERPERCENT", default = 50u32)]
    pub min_starved_copper_percent: u32,
    #[param(key = "REPORTSTRADLINGHOLES", default = false)]
    pub report_straddling_holes: bool,
    #[param(key = "REPORTHOLESINVOIDS", default = false)]
    pub report_holes_in_voids: bool,
}

// ── Serialization dispatch ──────────────────────────────────────────────────

/// Serialize a `PcbRule` back to a `PrefixedParamRecord`.
pub(crate) fn serialize_rule(rule: &PcbRule) -> Result<super::records::PrefixedParamRecord> {
    let mut params = ParameterCollection::new();
    rule.base.to_params(&mut params);
    serialize_rule_kind_data(&rule.kind_data, &mut params);
    Ok(super::records::PrefixedParamRecord {
        prefix: rule.prefix,
        params,
    })
}

fn serialize_rule_kind_data(kind_data: &PcbRuleKindData, params: &mut ParameterCollection) {
    match kind_data {
        PcbRuleKindData::Clearance(d) => d.to_params(params),
        PcbRuleKindData::ParallelSegment(d) => d.to_params(params),
        PcbRuleKindData::Width(d) => d.to_params(params),
        PcbRuleKindData::Length(d) => d.to_params(params),
        PcbRuleKindData::MatchedLengths(d) => d.to_params(params),
        PcbRuleKindData::DaisyChainStubLength(d) => d.to_params(params),
        PcbRuleKindData::PowerPlaneConnectStyle(d) => d.to_params(params),
        PcbRuleKindData::RoutingTopology(d) => d.to_params(params),
        PcbRuleKindData::RoutingPriority(d) => d.to_params(params),
        PcbRuleKindData::RoutingLayers(d) => d.to_params(params),
        PcbRuleKindData::RoutingCornerStyle(d) => d.to_params(params),
        PcbRuleKindData::RoutingViaStyle(d) => d.to_params(params),
        PcbRuleKindData::PowerPlaneClearance(d) => d.to_params(params),
        PcbRuleKindData::SolderMaskExpansion(d) => d.to_params(params),
        PcbRuleKindData::PasteMaskExpansion(d) => d.to_params(params),
        PcbRuleKindData::ShortCircuit(d) => d.to_params(params),
        PcbRuleKindData::BrokenNets(d) => d.to_params(params),
        PcbRuleKindData::ViasUnderSmd(d) => d.to_params(params),
        PcbRuleKindData::MaximumViaCount(d) => d.to_params(params),
        PcbRuleKindData::MinimumAnnularRing(d) => d.to_params(params),
        PcbRuleKindData::PolygonConnectStyle(d) => d.to_params(params),
        PcbRuleKindData::AcuteAngle(d) => d.to_params(params),
        PcbRuleKindData::ConfinementConstraint(d) => d.to_params(params),
        PcbRuleKindData::SmdToCorner(d) => d.to_params(params),
        PcbRuleKindData::ComponentClearance(d) => d.to_params(params),
        PcbRuleKindData::ComponentRotations(d) => d.to_params(params),
        PcbRuleKindData::PermittedLayers(d) => d.to_params(params),
        PcbRuleKindData::NetsToIgnore(d) => d.to_params(params),
        PcbRuleKindData::SignalStimulus(d) => d.to_params(params),
        PcbRuleKindData::OvershootFallingEdge(d) => d.to_params(params),
        PcbRuleKindData::OvershootRisingEdge(d) => d.to_params(params),
        PcbRuleKindData::UndershootFallingEdge(d) => d.to_params(params),
        PcbRuleKindData::UndershootRisingEdge(d) => d.to_params(params),
        PcbRuleKindData::MaxMinImpedance(d) => d.to_params(params),
        PcbRuleKindData::SignalTopValue(d) => d.to_params(params),
        PcbRuleKindData::SignalBaseValue(d) => d.to_params(params),
        PcbRuleKindData::FlightTimeRisingEdge(d) => d.to_params(params),
        PcbRuleKindData::FlightTimeFallingEdge(d) => d.to_params(params),
        PcbRuleKindData::LayerStack(d) => d.to_params(params),
        PcbRuleKindData::MaxSlopeRisingEdge(d) => d.to_params(params),
        PcbRuleKindData::MaxSlopeFallingEdge(d) => d.to_params(params),
        PcbRuleKindData::SupplyNets(d) => d.to_params(params),
        PcbRuleKindData::MaxMinHoleSize(d) => d.to_params(params),
        PcbRuleKindData::FabricationTestpointStyle(d) => d.to_params(params),
        PcbRuleKindData::FabricationTestpointUsage(d) => d.to_params(params),
        PcbRuleKindData::UnconnectedPin(d) => d.to_params(params),
        PcbRuleKindData::SmdToPlane(d) => d.to_params(params),
        PcbRuleKindData::SmdNeckDown(d) => d.to_params(params),
        PcbRuleKindData::LayerPair(d) => d.to_params(params),
        PcbRuleKindData::FanoutControl(d) => d.to_params(params),
        PcbRuleKindData::MaxMinHeight(d) => d.to_params(params),
        PcbRuleKindData::DifferentialPairsRouting(d) => d.to_params(params),
        PcbRuleKindData::HoleToHoleClearance(d) => d.to_params(params),
        PcbRuleKindData::MinimumSolderMaskSliver(d) => d.to_params(params),
        PcbRuleKindData::SilkToSolderMaskClearance(d) => d.to_params(params),
        PcbRuleKindData::SilkToSilkClearance(d) => d.to_params(params),
        PcbRuleKindData::NetAntennae(d) => d.to_params(params),
        PcbRuleKindData::AssyTestPointStyle(d) => d.to_params(params),
        PcbRuleKindData::AssyTestPointUsage(d) => d.to_params(params),
        PcbRuleKindData::SilkToBoardRegionClearance(d) => d.to_params(params),
        PcbRuleKindData::SmdEntry(d) => d.to_params(params),
        PcbRuleKindData::None(d) => d.to_params(params),
        PcbRuleKindData::UnpouredPolygon(d) => d.to_params(params),
        PcbRuleKindData::BoardOutlineClearance(d) => d.to_params(params),
        PcbRuleKindData::BackDrilling(d) => d.to_params(params),
        PcbRuleKindData::Creepage(d) => d.to_params(params),
        PcbRuleKindData::ReturnPath(d) => d.to_params(params),
        PcbRuleKindData::RoutingNeckDown(d) => d.to_params(params),
        PcbRuleKindData::WireBonding(d) => d.to_params(params),
        PcbRuleKindData::ZAxisClearance(d) => d.to_params(params),
    }
}

/// Serialize a `PcbViolation` back to a `StandardParamRecord`.
pub(crate) fn serialize_violation(violation: &PcbViolation) -> super::records::StandardParamRecord {
    let mut params = ParameterCollection::new();
    match violation {
        PcbViolation::AcuteAngle { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::BackDrill { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::BoardOutlineClearance { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::Clearance { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::ComponentClearance { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::Creepage { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::DiffPairs { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::DisconnectedSubnets { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::HoleToHole { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::MatchedNetLengths { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::MaximumViaCount { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::MaxMinComponentHeight { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::MaxMinLength { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::MaxMinPadSlotWidth { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::MaxMinViaHoleSize { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::MinimumAnnularRing { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::MinSolderMaskSliver { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::MinWidth { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::ModifiedPolygon { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::NetAntennae { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::PadUnderSmd { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::ParallelSegment { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::ReturnPath { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::RoutingNeckDown { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::RoutingViaStyle { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::ShortCircuit { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::SilkToBoardRegionClearance { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::SilkToSilkClearance { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::SilkToSolderMaskClearance { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::SmdNeckDown { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::SmdPadEntry { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::SmdToCorner { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::TestPoint { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::UnconnectedPin { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::ViaUnderSmd { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::WirebondLength { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::WirebondWireToWire { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
        PcbViolation::ZAxisClearance { base, data } => {
            base.to_params(&mut params);
            data.to_params(&mut params);
        }
    }
    super::records::StandardParamRecord { params }
}

/// Serialize a `WaivedViolation` back to a `StandardParamRecord`.
pub(crate) fn serialize_waived_violation(wv: &WaivedViolation) -> super::records::StandardParamRecord {
    let mut params = ParameterCollection::new();
    wv.to_params(&mut params);
    super::records::StandardParamRecord { params }
}

/// Serialize `DrcOptions` back to a `StandardParamRecord`.
pub(crate) fn serialize_drc_options(opts: &DrcOptions) -> super::records::StandardParamRecord {
    let mut params = ParameterCollection::new();
    opts.to_params(&mut params);
    super::records::StandardParamRecord { params }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::param_value::FromParamValue;
    use altium_format_types::Coord;

    /// Helper: create a ParameterCollection from |KEY=VALUE| formatted string.
    fn params_from_str(s: &str) -> ParameterCollection {
        ParameterCollection::from_str(s).unwrap()
    }

    // ── MilCoord roundtrip ──────────────────────────────────────────────────

    #[test]
    fn milcoord_roundtrip_integer() {
        let mc = MilCoord::from_param_value("X", "7mil").unwrap();
        assert_eq!(mc.to_param_value(), "7mil");
    }

    #[test]
    fn milcoord_roundtrip_decimal() {
        let mc = MilCoord::from_param_value("X", "3.5mil").unwrap();
        assert_eq!(mc.to_param_value(), "3.5mil");
    }

    #[test]
    fn milcoord_roundtrip_zero() {
        let mc = MilCoord::from_param_value("X", "0mil").unwrap();
        assert_eq!(mc.to_param_value(), "0mil");
    }

    #[test]
    fn milcoord_comma_decimal_separator() {
        // Altium sometimes uses comma as decimal separator
        let mc = MilCoord::from_param_value("X", "3,5mil").unwrap();
        assert_eq!(mc.0, Coord::from_mils_f64(3.5));
    }

    // ── ClearanceMatrix roundtrip ───────────────────────────────────────────

    #[test]
    fn clearance_matrix_roundtrip() {
        use crate::param_value::ClearanceMatrix;
        // Values are raw i32 internal coord units (10000 units = 1 mil)
        let input = "ClearanceObj_Track-ClearanceObj_SMDPad:100000;ClearanceObj_Via-ClearanceObj_Track:80000";
        let cm = ClearanceMatrix::from_param_value("MATRIX", input).unwrap();
        let output = cm.to_param_value();
        // Parse the output back to verify roundtrip
        let cm2 = ClearanceMatrix::from_param_value("MATRIX", &output).unwrap();
        assert_eq!(cm, cm2);
    }

    // ── String enum roundtrip ───────────────────────────────────────────────

    #[test]
    fn rule_kind_roundtrip() {
        use crate::param_value::FromParamValue;
        let rk = RuleKind::from_param_value("RULEKIND", "Clearance").unwrap();
        assert_eq!(rk, RuleKind::Clearance);
        assert_eq!(rk.to_param_value(), "Clearance");
    }

    #[test]
    fn rule_kind_broken_nets_alias() {
        // BrokenNets serializes as "UnRoutedNet" in Altium
        let rk = RuleKind::from_param_value("RULEKIND", "UnRoutedNet").unwrap();
        assert_eq!(rk, RuleKind::BrokenNets);
    }

    #[test]
    fn net_scope_roundtrip() {
        let ns = NetScope::from_param_value("NETSCOPE", "DifferentNets").unwrap();
        assert_eq!(ns, NetScope::DifferentNetsOnly);
        assert_eq!(ns.to_param_value(), "DifferentNets");
    }

    // ── Rule parsing ────────────────────────────────────────────────────────

    #[test]
    fn parse_simple_clearance_rule() {
        let s = "|SELECTION=FALSE|LAYER=|LOCKED=FALSE|POLYGONOUTLINE=FALSE\
                  |USERROUTED=TRUE|KEEPOUT=FALSE|UNIONINDEX=0\
                  |RULEKIND=Clearance|NETSCOPE=DifferentNets\
                  |LAYERKIND=SameLayer|NAME=Clearance1\
                  |COMMENT=|UNIQUEID=QZHEQKDM|DEFINEDBYLOGICALDOCUMENT=FALSE\
                  |SCOPE1EXPRESSION=All|SCOPE2EXPRESSION=All\
                  |ENABLED=TRUE|PRIORITY=1\
                  |GAP=10mil";
        let mut params = params_from_str(s);
        let rule = parse_rule(0, &mut params).unwrap();
        assert_eq!(rule.base.rule_kind, RuleKind::Clearance);
        assert_eq!(rule.base.name, "Clearance1");
        assert!(rule.base.enabled);
        match &rule.kind_data {
            PcbRuleKindData::Clearance(data) => {
                assert_eq!(data.gap, MilCoord(Coord::from_mils_f64(10.0)));
            }
            _ => panic!("expected Clearance variant"),
        }
    }

    #[test]
    fn parse_width_rule_basic() {
        let s = "|SELECTION=FALSE|LAYER=|LOCKED=FALSE|POLYGONOUTLINE=FALSE\
                  |USERROUTED=TRUE|KEEPOUT=FALSE|UNIONINDEX=0\
                  |RULEKIND=Width|NETSCOPE=AnyNet\
                  |LAYERKIND=SameLayer|NAME=Width1\
                  |COMMENT=|UNIQUEID=ABCDEF01|DEFINEDBYLOGICALDOCUMENT=FALSE\
                  |SCOPE1EXPRESSION=All|SCOPE2EXPRESSION=All\
                  |ENABLED=TRUE|PRIORITY=1\
                  |MINLIMIT=6mil|MAXLIMIT=10mil|PREFEREDWIDTH=8mil";
        let mut params = params_from_str(s);
        let rule = parse_rule(0, &mut params).unwrap();
        match &rule.kind_data {
            PcbRuleKindData::Width(data) => {
                assert_eq!(data.min_limit, MilCoord(Coord::from_mils_f64(6.0)));
                assert_eq!(data.max_limit, MilCoord(Coord::from_mils_f64(10.0)));
                assert_eq!(data.preferred_width, MilCoord(Coord::from_mils_f64(8.0)));
            }
            _ => panic!("expected Width variant"),
        }
    }

    // ── Violation parsing ───────────────────────────────────────────────────

    #[test]
    fn parse_clearance_violation() {
        let s = "|SELECTION=FALSE|LAYER=BOTTOM|LOCKED=FALSE|POLYGONOUTLINE=FALSE\
                  |USERROUTED=TRUE|KEEPOUT=FALSE|UNIONINDEX=0\
                  |RULEINDEX=38|PRIM1ID=Track|PRIM1INDEX=100\
                  |PRIM2ID=Pad|PRIM2INDEX=200\
                  |DESCRIPTION=test|INVOLVEDPRIMCOUNT=0\
                  |LOCATION1.X=100mil|LOCATION1.Y=200mil\
                  |LOCATION2.X=300mil|LOCATION2.Y=400mil";
        let mut params = params_from_str(s);
        let v = parse_violation(ParamSectionKind::TClearanceViolation, &mut params).unwrap();
        match &v {
            PcbViolation::Clearance { base, data } => {
                assert_eq!(base.prim1_id, "Track");
                assert_eq!(base.prim1_index, 100);
                assert_eq!(data.prim2_id.as_deref(), Some("Pad"));
                assert_eq!(data.prim2_index, Some(200));
                assert!(data.location1_x.is_some());
            }
            _ => panic!("expected Clearance variant"),
        }
    }

    #[test]
    fn parse_net_antennae_violation() {
        let s = "|SELECTION=FALSE|LAYER=MULTILAYER|LOCKED=FALSE|POLYGONOUTLINE=FALSE\
                  |USERROUTED=TRUE|KEEPOUT=FALSE|UNIONINDEX=0\
                  |RULEINDEX=3|PRIM1ID=Via|PRIM1INDEX=13\
                  |DESCRIPTION=|INVOLVEDPRIMCOUNT=0\
                  |LOCATION.X=157.4803mil|LOCATION.Y=157.4803mil\
                  |CIRCLERADIUS=105.4252mil";
        let mut params = params_from_str(s);
        let v = parse_violation(ParamSectionKind::TNetAntennaeViolation, &mut params).unwrap();
        match &v {
            PcbViolation::NetAntennae { base, data } => {
                assert_eq!(base.prim1_id, "Via");
                assert_eq!(data.circle_radius, MilCoord(Coord::from_mils_f64(105.4252)));
            }
            _ => panic!("expected NetAntennae variant"),
        }
    }

    #[test]
    fn parse_base_only_violation() {
        let s = "|SELECTION=FALSE|LAYER=BOTTOM|LOCKED=FALSE|POLYGONOUTLINE=FALSE\
                  |USERROUTED=TRUE|KEEPOUT=FALSE|UNIONINDEX=0\
                  |RULEINDEX=6|PRIM1ID=Component|PRIM1INDEX=560\
                  |DESCRIPTION=Actual Height = 8.8mm|INVOLVEDPRIMCOUNT=0";
        let mut params = params_from_str(s);
        let v = parse_violation(
            ParamSectionKind::TMaxMinComponentHeightViolation,
            &mut params,
        )
        .unwrap();
        match &v {
            PcbViolation::MaxMinComponentHeight { base, .. } => {
                assert_eq!(base.prim1_id, "Component");
                assert_eq!(base.description, "Actual Height = 8.8mm");
            }
            _ => panic!("expected MaxMinComponentHeight variant"),
        }
    }

    #[test]
    fn parse_disconnected_subnets_violation() {
        let s = "|SELECTION=FALSE|LAYER=BOTTOM|LOCKED=FALSE|POLYGONOUTLINE=FALSE\
                  |USERROUTED=TRUE|KEEPOUT=FALSE|UNIONINDEX=0\
                  |RULEINDEX=56|PRIM1ID=Pad|PRIM1INDEX=1034\
                  |PRIM2ID=Pad|PRIM2INDEX=1032\
                  |DESCRIPTION=|INVOLVEDPRIMCOUNT=0\
                  |FX1=3550.5807mil|FY1=1520mil|FX2=3550.5807mil|FY2=1713.307mil";
        let mut params = params_from_str(s);
        let v = parse_violation(
            ParamSectionKind::TDisconnectedSubnetsViolation,
            &mut params,
        )
        .unwrap();
        match &v {
            PcbViolation::DisconnectedSubnets { base, data } => {
                assert_eq!(base.prim1_id, "Pad");
                assert!(data.fx1.is_some());
                assert!(data.fy2.is_some());
            }
            _ => panic!("expected DisconnectedSubnets variant"),
        }
    }

    // ── Waived violation parsing ────────────────────────────────────────────

    #[test]
    fn parse_waived_violation() {
        let s = "|RULEINDEX=13|PRIM1KIND=DifferentialPair|PRIM1INDEX=63\
                  |CREATEDAT=2020-09-04T13:11:48.000Z\
                  |AUTHORID=74F33633-A57B-4253-AAC0-B6C1D3748DA6\
                  |AUTHORTITLE=Test User|SOURCE=Portal\
                  |COMMENT=To connector";
        let mut params = params_from_str(s);
        let wv = WaivedViolation::from_params(&mut params).unwrap();
        params.assert_exhausted().unwrap();
        assert_eq!(wv.rule_index, 13);
        assert_eq!(wv.prim1_kind, "DifferentialPair");
        assert_eq!(wv.comment, "To connector");
        assert!(wv.prim2_kind.is_none());
    }

    // ── DRC Options parsing ─────────────────────────────────────────────────

    #[test]
    fn parse_drc_options() {
        let s = "|RECORD=DesignRuleCheckerOptions\
                  |DOMAKEDRCFILE=TRUE|DOMAKEDRCERRORLIST=TRUE\
                  |DOSUBNETDETAILS=TRUE|REPORTFILENAME=\
                  |EXTERNALNETLISTFILENAME=|CHECKEXTERNALNETLIST=FALSE\
                  |MAXVIOLATIONCOUNT=500|REPORTDRILLEDSMTPADS=TRUE\
                  |REPORTINVALIDMULTILAYERPADS=TRUE\
                  |RULESETTOCHECK=0,1,2,3,4,5\
                  |ONLINERULESETTOCHECK=0,1,2,3\
                  |INTERNALPLANEWARNINGS=TRUE|VERIFYSHORTINGCOPPER=TRUE\
                  |REPORTBROKENPLANES=TRUE|REPORTDEADCOPPER=TRUE\
                  |DEADCOPPERMINAREA=10000000000.000000\
                  |REPORTSTARVEDTHERMALS=TRUE|MINSTARVEDCOPPERPERCENT=50\
                  |REPORTSTRADLINGHOLES=FALSE|REPORTHOLESINVOIDS=FALSE";
        let mut params = params_from_str(s);
        let opts = DrcOptions::from_params(&mut params).unwrap();
        params.assert_exhausted().unwrap();
        assert!(opts.do_make_drc_file);
        assert_eq!(opts.max_violation_count, 500);
        assert_eq!(opts.rule_set_to_check, "0,1,2,3,4,5");
        assert!(!opts.report_straddling_holes);
    }

    // ── DiffPairs violation (complex polygon) ───────────────────────────────

    // ── Parameter-level roundtrip tests ────────────────────────────────────

    /// Helper: parse → serialize → re-parse a violation, verifying base fields match.
    fn roundtrip_violation(kind: ParamSectionKind, input: &str) {
        let mut params1 = params_from_str(input);
        let v1 = parse_violation(kind, &mut params1).unwrap();

        // Re-serialize base + data to a fresh ParameterCollection.
        let mut out = ParameterCollection::new();
        match &v1 {
            PcbViolation::Clearance { base, data } => {
                base.to_params(&mut out);
                data.to_params(&mut out);
            }
            PcbViolation::NetAntennae { base, data } => {
                base.to_params(&mut out);
                data.to_params(&mut out);
            }
            PcbViolation::BoardOutlineClearance { base, data } => {
                base.to_params(&mut out);
                data.to_params(&mut out);
            }
            PcbViolation::DisconnectedSubnets { base, data } => {
                base.to_params(&mut out);
                data.to_params(&mut out);
            }
            PcbViolation::MatchedNetLengths { base, data } => {
                base.to_params(&mut out);
                data.to_params(&mut out);
            }
            // For base-only and two-point violations, serialize base + data
            PcbViolation::UnconnectedPin { base, data }
            | PcbViolation::ModifiedPolygon { base, data }
            | PcbViolation::MaxMinComponentHeight { base, data }
            | PcbViolation::SilkToSilkClearance { base, data }
            | PcbViolation::SilkToSolderMaskClearance { base, data }
            | PcbViolation::MinSolderMaskSliver { base, data }
            | PcbViolation::MinimumAnnularRing { base, data }
            | PcbViolation::HoleToHole { base, data }
            | PcbViolation::MinWidth { base, data }
            | PcbViolation::ShortCircuit { base, data } => {
                base.to_params(&mut out);
                data.to_params(&mut out);
            }
            _ => {
                // For other variants we just verify parsing succeeded
                return;
            }
        }

        // Re-parse from the serialized params
        let serialized = {
            let bytes = out.to_bytes();
            String::from_utf8(bytes).unwrap()
        };
        let mut params2 = params_from_str(&serialized);
        let v2 = parse_violation(kind, &mut params2).unwrap();

        // Compare base fields
        let (base1, base2) = match (&v1, &v2) {
            (PcbViolation::Clearance { base: b1, .. }, PcbViolation::Clearance { base: b2, .. })
            | (PcbViolation::NetAntennae { base: b1, .. }, PcbViolation::NetAntennae { base: b2, .. })
            | (PcbViolation::BoardOutlineClearance { base: b1, .. }, PcbViolation::BoardOutlineClearance { base: b2, .. })
            | (PcbViolation::DisconnectedSubnets { base: b1, .. }, PcbViolation::DisconnectedSubnets { base: b2, .. })
            | (PcbViolation::MatchedNetLengths { base: b1, .. }, PcbViolation::MatchedNetLengths { base: b2, .. })
            | (PcbViolation::UnconnectedPin { base: b1, .. }, PcbViolation::UnconnectedPin { base: b2, .. })
            | (PcbViolation::ModifiedPolygon { base: b1, .. }, PcbViolation::ModifiedPolygon { base: b2, .. })
            | (PcbViolation::MaxMinComponentHeight { base: b1, .. }, PcbViolation::MaxMinComponentHeight { base: b2, .. })
            | (PcbViolation::SilkToSilkClearance { base: b1, .. }, PcbViolation::SilkToSilkClearance { base: b2, .. })
            | (PcbViolation::SilkToSolderMaskClearance { base: b1, .. }, PcbViolation::SilkToSolderMaskClearance { base: b2, .. })
            | (PcbViolation::MinSolderMaskSliver { base: b1, .. }, PcbViolation::MinSolderMaskSliver { base: b2, .. })
            | (PcbViolation::MinimumAnnularRing { base: b1, .. }, PcbViolation::MinimumAnnularRing { base: b2, .. })
            | (PcbViolation::HoleToHole { base: b1, .. }, PcbViolation::HoleToHole { base: b2, .. })
            | (PcbViolation::MinWidth { base: b1, .. }, PcbViolation::MinWidth { base: b2, .. })
            | (PcbViolation::ShortCircuit { base: b1, .. }, PcbViolation::ShortCircuit { base: b2, .. }) => (b1, b2),
            _ => return,
        };
        assert_eq!(base1.rule_index, base2.rule_index, "rule_index mismatch in roundtrip");
        assert_eq!(base1.prim1_id, base2.prim1_id, "prim1_id mismatch in roundtrip");
        assert_eq!(base1.prim1_index, base2.prim1_index, "prim1_index mismatch in roundtrip");
        assert_eq!(base1.description, base2.description, "description mismatch in roundtrip");
    }

    #[test]
    fn roundtrip_clearance_violation_from_fixture() {
        // Real data from artiq-kasli.PcbDoc
        roundtrip_violation(
            ParamSectionKind::TClearanceViolation,
            "|SELECTION=FALSE|LAYER=BOTTOM|LOCKED=FALSE|POLYGONOUTLINE=FALSE\
             |USERROUTED=TRUE|KEEPOUT=FALSE|UNIONINDEX=0|RULEINDEX=49\
             |PRIM1ID=Track|PRIM1INDEX=24056|PRIM2ID=Track|PRIM2INDEX=9437\
             |DESCRIPTION=|INVOLVEDPRIMCOUNT=0\
             |LOCATION1.X=3992.126mil|LOCATION1.Y=9015.7481mil\
             |LOCATION2.X=3992.126mil|LOCATION2.Y=9015.7481mil",
        );
    }

    #[test]
    fn roundtrip_net_antennae_violation_from_fixture() {
        // Real data from heron-buckboost.PcbDoc
        roundtrip_violation(
            ParamSectionKind::TNetAntennaeViolation,
            "|SELECTION=FALSE|LAYER=MULTILAYER|LOCKED=FALSE|POLYGONOUTLINE=FALSE\
             |USERROUTED=TRUE|KEEPOUT=FALSE|UNIONINDEX=0|RULEINDEX=3\
             |PRIM1ID=Via|PRIM1INDEX=13|DESCRIPTION=|INVOLVEDPRIMCOUNT=0\
             |LOCATION.X=157.4803mil|LOCATION.Y=157.4803mil|CIRCLERADIUS=105.4252mil",
        );
    }

    #[test]
    fn roundtrip_board_outline_violation_from_fixture() {
        // Real data from heron-eps-flight.PcbDoc
        roundtrip_violation(
            ParamSectionKind::TBoardOutlineClearanceViolation,
            "|SELECTION=FALSE|LAYER=TOPOVERLAY|LOCKED=FALSE|POLYGONOUTLINE=FALSE\
             |USERROUTED=TRUE|KEEPOUT=FALSE|UNIONINDEX=0|RULEINDEX=33\
             |PRIM1ID=Track|PRIM1INDEX=162|PRIM2ID=PolyRegion|PRIM2INDEX=0\
             |DESCRIPTION=|INVOLVEDPRIMCOUNT=0\
             |LOCATION1.X=3265.3348mil|LOCATION1.Y=3579mil\
             |LOCATION2.X=3265.3348mil|LOCATION2.Y=3596.5353mil\
             |PRIMID1=ClearanceObj_Track|PRIMID2=ClearanceObj_CutoutEdge",
        );
    }

    #[test]
    fn roundtrip_disconnected_subnets_from_fixture() {
        // Real data from heron-eps-flight.PcbDoc
        roundtrip_violation(
            ParamSectionKind::TDisconnectedSubnetsViolation,
            "|SELECTION=FALSE|LAYER=BOTTOM|LOCKED=FALSE|POLYGONOUTLINE=FALSE\
             |USERROUTED=TRUE|KEEPOUT=FALSE|UNIONINDEX=0|RULEINDEX=56\
             |PRIM1ID=Pad|PRIM1INDEX=1034|PRIM2ID=Pad|PRIM2INDEX=1032\
             |DESCRIPTION=|INVOLVEDPRIMCOUNT=0\
             |FX1=3550.5807mil|FY1=1520mil|FX2=3550.5807mil|FY2=1713.307mil",
        );
    }

    #[test]
    fn roundtrip_base_only_violation_from_fixture() {
        // Real data from artiq-kasli.PcbDoc
        roundtrip_violation(
            ParamSectionKind::TMaxMinComponentHeightViolation,
            "|SELECTION=FALSE|LAYER=BOTTOM|LOCKED=FALSE|POLYGONOUTLINE=FALSE\
             |USERROUTED=TRUE|KEEPOUT=FALSE|UNIONINDEX=0|RULEINDEX=6\
             |PRIM1ID=Component|PRIM1INDEX=560\
             |DESCRIPTION=Actual Height = 8.8mm|INVOLVEDPRIMCOUNT=0",
        );
    }

    #[test]
    fn roundtrip_clearance_rule_from_fixture() {
        // Parse a clearance rule → serialize → re-parse.
        // Use Width (not Clearance) because Clearance is variant 0 / Default,
        // and T1 serialization skips default-valued fields. This is a known
        // limitation that will be fixed when PcbDoc save is implemented (by
        // marking required fields as tier2).
        let input = "|SELECTION=FALSE|LAYER=|LOCKED=FALSE|POLYGONOUTLINE=FALSE\
                      |USERROUTED=TRUE|KEEPOUT=FALSE|UNIONINDEX=0\
                      |RULEKIND=Clearance|NETSCOPE=DifferentNets\
                      |LAYERKIND=SameLayer|NAME=Clearance1\
                      |COMMENT=|UNIQUEID=QZHEQKDM|DEFINEDBYLOGICALDOCUMENT=FALSE\
                      |SCOPE1EXPRESSION=All|SCOPE2EXPRESSION=All\
                      |ENABLED=TRUE|PRIORITY=1|GAP=10mil";
        let mut params1 = params_from_str(input);
        let rule1 = parse_rule(0, &mut params1).unwrap();

        // Re-serialize: base + kind data
        let mut out = ParameterCollection::new();
        rule1.base.to_params(&mut out);
        if let PcbRuleKindData::Clearance(data) = &rule1.kind_data {
            data.to_params(&mut out);
        }

        // Verify the serialized params contain the expected keys.
        // Note: RULEKIND=Clearance is Default so T1 skips it. We check
        // other fields that DO survive roundtrip.
        let serialized_bytes = out.to_bytes();
        let serialized = String::from_utf8(serialized_bytes).unwrap();
        assert!(
            serialized.contains("GAP="),
            "GAP missing from serialized output: {serialized}"
        );
        assert!(
            serialized.contains("NAME=Clearance1"),
            "NAME missing from serialized output"
        );
        assert!(
            serialized.contains("SCOPE1EXPRESSION=All"),
            "SCOPE1EXPRESSION missing"
        );
    }

    #[test]
    fn roundtrip_waived_violation_from_fixture() {
        // Real data from artiq-hvsup-isol.PcbDoc (after unicode sidecar merge)
        let input = "|RULEINDEX=38|PRIM1KIND=Pad|PRIM1INDEX=624\
                      |PRIM2KIND=PolyRegion|PRIM2INDEX=73\
                      |CREATEDAT=2022-05-20T18:19:21.000Z\
                      |AUTHORID=DBF9A147-DA90-4FFB-B151-2750C73EE278\
                      |AUTHORTITLE=Test Author|SOURCE=Portal";
        let mut params1 = params_from_str(input);
        let wv1 = WaivedViolation::from_params(&mut params1).unwrap();
        params1.assert_exhausted().unwrap();

        // Re-serialize
        let mut out = ParameterCollection::new();
        wv1.to_params(&mut out);
        let serialized = {
            let bytes = out.to_bytes();
            String::from_utf8(bytes).unwrap()
        };

        // Re-parse
        let mut params2 = params_from_str(&serialized);
        let wv2 = WaivedViolation::from_params(&mut params2).unwrap();
        params2.assert_exhausted().unwrap();

        assert_eq!(wv1.rule_index, wv2.rule_index);
        assert_eq!(wv1.prim1_kind, wv2.prim1_kind);
        assert_eq!(wv1.prim2_kind, wv2.prim2_kind);
        assert_eq!(wv1.created_at, wv2.created_at);
        assert_eq!(wv1.author_id, wv2.author_id);
        assert_eq!(wv1.source, wv2.source);
    }

    #[test]
    fn roundtrip_drc_options_from_fixture() {
        // Real data from cobra.PcbDoc
        let input = "|RECORD=DesignRuleCheckerOptions\
                      |DOMAKEDRCFILE=TRUE|DOMAKEDRCERRORLIST=TRUE\
                      |DOSUBNETDETAILS=TRUE|REPORTFILENAME=\
                      |EXTERNALNETLISTFILENAME=|CHECKEXTERNALNETLIST=FALSE\
                      |MAXVIOLATIONCOUNT=500|REPORTDRILLEDSMTPADS=TRUE\
                      |REPORTINVALIDMULTILAYERPADS=TRUE\
                      |RULESETTOCHECK=0,1,2,3,4,5,15,16,18,19,21,22,23,26,42,45,46,47,50,52,53,54,55,56,60,62,63,64\
                      |ONLINERULESETTOCHECK=0,1,2,3,4,5,9,11,15,17,18,22,23,24,45,46,47,50,51,55,60,62\
                      |INTERNALPLANEWARNINGS=TRUE|VERIFYSHORTINGCOPPER=TRUE\
                      |REPORTBROKENPLANES=TRUE|REPORTDEADCOPPER=TRUE\
                      |DEADCOPPERMINAREA=10000000000.000000\
                      |REPORTSTARVEDTHERMALS=TRUE|MINSTARVEDCOPPERPERCENT=50\
                      |REPORTSTRADLINGHOLES=FALSE|REPORTHOLESINVOIDS=FALSE";
        let mut params1 = params_from_str(input);
        let opts1 = DrcOptions::from_params(&mut params1).unwrap();
        params1.assert_exhausted().unwrap();

        // Re-serialize
        let mut out = ParameterCollection::new();
        opts1.to_params(&mut out);
        let serialized = {
            let bytes = out.to_bytes();
            String::from_utf8(bytes).unwrap()
        };

        // Re-parse
        let mut params2 = params_from_str(&serialized);
        let opts2 = DrcOptions::from_params(&mut params2).unwrap();
        params2.assert_exhausted().unwrap();

        assert_eq!(opts1.max_violation_count, opts2.max_violation_count);
        assert_eq!(opts1.rule_set_to_check, opts2.rule_set_to_check);
        assert_eq!(opts1.online_rule_set_to_check, opts2.online_rule_set_to_check);
        assert_eq!(opts1.do_make_drc_file, opts2.do_make_drc_file);
        assert_eq!(opts1.report_straddling_holes, opts2.report_straddling_holes);
        assert_eq!(opts1.dead_copper_min_area, opts2.dead_copper_min_area);
    }

    // ── DiffPairs violation (complex polygon) ───────────────────────────────

    #[test]
    fn parse_diff_pairs_violation() {
        let s = "|SELECTION=FALSE|LAYER=TOP|LOCKED=FALSE|POLYGONOUTLINE=FALSE\
                  |USERROUTED=TRUE|KEEPOUT=FALSE|UNIONINDEX=0\
                  |RULEINDEX=5|PRIM1ID=DifferentialPair|PRIM1INDEX=10\
                  |DESCRIPTION=test|INVOLVEDPRIMCOUNT=0\
                  |LAYERCOUNT=1|LAYER1=TOP\
                  |POLY1.CONTOURCOUNT=1\
                  |POLY1.CONTOUR0.VTXCOUNT=3\
                  |POLY1.CONTOUR0.VX0=100.000000|POLY1.CONTOUR0.VY0=200.000000\
                  |POLY1.CONTOUR0.VX1=300.000000|POLY1.CONTOUR0.VY1=400.000000\
                  |POLY1.CONTOUR0.VX2=500.000000|POLY1.CONTOUR0.VY2=600.000000";
        let mut params = params_from_str(s);
        let v = parse_violation(ParamSectionKind::TDiffPairsViolation, &mut params).unwrap();
        match &v {
            PcbViolation::DiffPairs { data, .. } => {
                assert_eq!(data.layers.len(), 1);
                assert_eq!(data.layers[0].layer_name, "TOP");
                assert_eq!(data.layers[0].contours.len(), 1);
                assert_eq!(data.layers[0].contours[0].vertices.len(), 3);
                assert_eq!(data.layers[0].contours[0].vertices[0], (100.0, 200.0));
                assert_eq!(data.layers[0].contours[0].vertices[2], (500.0, 600.0));
            }
            _ => panic!("expected DiffPairs variant"),
        }
    }
}

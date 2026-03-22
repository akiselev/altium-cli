//! Direct spec-to-IR compiler.
//!
//! Converts a [`PcbDocSpec`] into a [`PcbIr`] without touching `altium_format`.
//! Only `altium_format_spec` and `altium_format_types` are used.

use std::collections::BTreeMap;

use altium_format_spec::model::{BoardLayerSpec, BoardSpec, PadGeometrySpec, PcbDocSpec};
use indexmap::IndexMap;
use altium_format_types::PadShape;
use altium_format_types::pcb::{CornerStyle, NetTopology, RuleKind};

use crate::board::{IrBoardGeometry, IrKeepoutZone};
use crate::compile_error::IrCompileError;
use crate::component::{IrComponent, IrComponentPad, PadShapeInfo, PadShapeKind};
use crate::copper::FreeCopperGeometry;
use crate::extract::PcbIr;
use crate::geometry::{compute_component_bounds, local_to_world, world_to_local};
use crate::handles::{ComponentId, IdMap, LayerId, NetId, PadId, RuleId};
use crate::layer_stack::{IrCopperLayer, IrLayerStack};
use crate::net::{IrNet, IrNetPin};
use crate::rule::{IrDesignRule, IrRuleParams, IrRuleScope, IrRuleScopePair};
use crate::types::{BoardSide, BoundingBoxMm, PointMm};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Compile a [`PcbDocSpec`] directly to a [`PcbIr`].
///
/// Returns [`IrCompileError::NoBoardsDefined`] when the spec contains no boards.
pub fn spec_to_ir(spec: &PcbDocSpec) -> Result<PcbIr, IrCompileError> {
    let board = spec.boards.first().ok_or(IrCompileError::NoBoardsDefined)?;

    let (layer_stack, layer_lookup) = compile_layer_stack(board);
    let (nets, net_lookup) = compile_nets(board);
    let mut components = compile_components(board, &net_lookup, &layer_lookup, &layer_stack)?;
    let nets = backfill_net_pins(nets, &components);
    compute_component_bounds(&mut components);
    let rules = compile_rules(board, &layer_lookup)?;
    let board_geometry = compile_board_geometry(board)?;

    Ok(PcbIr {
        board: board_geometry,
        layer_stack,
        components,
        nets,
        rules,
        free_copper: FreeCopperGeometry::default(),
        polygons: IdMap::new(),
        texts: IdMap::new(),
        regions: IdMap::new(),
        component_bodies: IdMap::new(),
    })
}

// ---------------------------------------------------------------------------
// Layer stack
// ---------------------------------------------------------------------------

fn compile_layer_stack(board: &BoardSpec) -> (IrLayerStack, BTreeMap<String, LayerId>) {
    let mut layers: IdMap<LayerId, IrCopperLayer> = IdMap::new();
    let mut lookup: BTreeMap<String, LayerId> = BTreeMap::new();

    if board.layers.is_empty() {
        // Default 2-layer stack when none is specified.
        let top_id = layers.push(IrCopperLayer {
            id: LayerId::from(0),
            name: "TopLayer".to_string(),
            is_top: true,
            is_bottom: false,
            preferred_direction: None,
        });
        layers[top_id].id = top_id;
        lookup.insert("TopLayer".to_string(), top_id);

        let bot_id = layers.push(IrCopperLayer {
            id: LayerId::from(0),
            name: "BottomLayer".to_string(),
            is_top: false,
            is_bottom: true,
            preferred_direction: None,
        });
        layers[bot_id].id = bot_id;
        lookup.insert("BottomLayer".to_string(), bot_id);
    } else {
        let copper: Vec<&BoardLayerSpec> =
            board.layers.iter().filter(|l| l.is_copper).collect();
        let copper_count = copper.len();
        for (i, layer_spec) in copper.iter().enumerate() {
            let id = layers.push(IrCopperLayer {
                id: LayerId::from(0),
                name: layer_spec.name.clone(),
                is_top: i == 0,
                is_bottom: i == copper_count - 1,
                preferred_direction: None,
            });
            layers[id].id = id;
            lookup.insert(layer_spec.name.clone(), id);
        }

    }

    let copper_layer_count = layers.len();
    let copper_layers: Vec<IrCopperLayer> = layers.iter().map(|(_, l)| l.clone()).collect();
    (
        IrLayerStack {
            copper_layers,
            copper_layer_count,
        },
        lookup,
    )
}

// ---------------------------------------------------------------------------
// Nets
// ---------------------------------------------------------------------------

fn compile_nets(
    board: &BoardSpec,
) -> (IdMap<NetId, IrNet>, BTreeMap<String, NetId>) {
    let mut nets: IdMap<NetId, IrNet> = IdMap::new();
    let mut lookup: BTreeMap<String, NetId> = BTreeMap::new();

    for net_spec in &board.nets {
        let id = nets.push(IrNet {
            id: NetId::from(0),
            name: net_spec.name.clone(),
            pins: Vec::new(),
            component_count: 0,
            net_class: None,
            diff_pair_partner: None,
        });
        nets[id].id = id;
        lookup.insert(net_spec.name.clone(), id);
    }

    // Populate net_class from spec classes (net-class kind only).
    for class in &board.classes {
        let is_net_class = class
            .kind
            .as_deref()
            .map(|k| k.eq_ignore_ascii_case("net"))
            .unwrap_or(true);
        if !is_net_class {
            continue;
        }
        for member_net_name in &class.members {
            if let Some(&net_id) = lookup.get(member_net_name.as_str()) {
                nets[net_id].net_class = Some(class.name.clone());
            }
        }
    }

    // Populate diff_pair_partner.
    for dp in &board.differential_pairs {
        if let (Some(pos_name), Some(neg_name)) = (&dp.positive_net, &dp.negative_net) {
            if let (Some(&pos_id), Some(&neg_id)) =
                (lookup.get(pos_name.as_str()), lookup.get(neg_name.as_str()))
            {
                nets[pos_id].diff_pair_partner = Some(neg_id);
                nets[neg_id].diff_pair_partner = Some(pos_id);
            }
        }
    }

    (nets, lookup)
}

// ---------------------------------------------------------------------------
// Components + pads
// ---------------------------------------------------------------------------

fn compile_components(
    board: &BoardSpec,
    net_lookup: &BTreeMap<String, NetId>,
    layer_lookup: &BTreeMap<String, LayerId>,
    layer_stack: &IrLayerStack,
) -> Result<IdMap<ComponentId, IrComponent>, IrCompileError> {
    let mut components: IdMap<ComponentId, IrComponent> =
        IdMap::with_capacity(board.components.len());
    let mut seen_designators: BTreeMap<String, ()> = BTreeMap::new();
    let mut next_pad_id: u32 = 0;

    let all_copper_layers: Vec<LayerId> = layer_stack.copper_layers.iter().map(|l| l.id).collect();

    for comp_spec in &board.components {
        if seen_designators.contains_key(&comp_spec.designator) {
            return Err(IrCompileError::DuplicateDesignator(
                comp_spec.designator.clone(),
            ));
        }
        seen_designators.insert(comp_spec.designator.clone(), ());

        let position = comp_spec
            .location
            .as_ref()
            .map(|cp| PointMm::new(cp.x.to_mms(), cp.y.to_mms()))
            .unwrap_or(PointMm::new(0.0, 0.0));

        let rotation = comp_spec.rotation.unwrap_or(0.0);

        // Determine side from layer spec name.
        let side = match &comp_spec.layer {
            Some(altium_format_spec::model::LayerSpec::NamedLayer(name))
                if name.contains("Bottom") =>
            {
                BoardSide::Bottom
            }
            _ => BoardSide::Top,
        };

        let ir_pads = compile_pads(
            &comp_spec.pads,
            position,
            rotation,
            net_lookup,
            layer_lookup,
            &all_copper_layers,
            &mut next_pad_id,
        )?;

        let zero_bb = BoundingBoxMm::new(PointMm::new(0.0, 0.0), PointMm::new(0.0, 0.0));
        let comp_id = components.push(IrComponent {
            id: ComponentId::from(0),
            designator: comp_spec.designator.clone(),
            pattern: comp_spec.pattern.clone().unwrap_or_default(),
            value: comp_spec.comment.clone().unwrap_or_default(),
            position,
            rotation,
            side,
            local_bounds: zero_bb,
            world_bounds: zero_bb,
            pads: ir_pads,
        });
        components[comp_id].id = comp_id;
    }

    Ok(components)
}

fn compile_pads(
    pads: &[PadGeometrySpec],
    comp_pos: PointMm,
    rotation: f64,
    net_lookup: &BTreeMap<String, NetId>,
    layer_lookup: &BTreeMap<String, LayerId>,
    all_copper_layers: &[LayerId],
    next_pad_id: &mut u32,
) -> Result<Vec<IrComponentPad>, IrCompileError> {
    let mut ir_pads = Vec::new();

    for pad_spec in pads {
        // After merge_pcbdoc_spec(), pad positions from PcbDoc import are absolute
        // world coordinates.  When no PcbDoc target is used (pure spec-only board),
        // positions come from PcbLib and are footprint-relative.  The merge path is
        // the dominant case; treat positions as world coords and derive local from
        // world - comp_pos.
        let world_pos = PointMm::new(pad_spec.position.x.to_mms(), pad_spec.position.y.to_mms());
        let local_pos = world_to_local(world_pos, comp_pos, rotation);
        let net_id = match &pad_spec.net {
            None => None,
            Some(name) => {
                let id = net_lookup
                    .get(name.as_str())
                    .copied()
                    .ok_or_else(|| IrCompileError::UnknownNet(name.clone()))?;
                Some(id)
            }
        };

        let shape_kind = match pad_spec.shape {
            PadShape::Round | PadShape::Circle => PadShapeKind::Round,
            PadShape::Rectangular | PadShape::RotatedRect => PadShapeKind::Rectangular,
            PadShape::RoundRect | PadShape::RoundedRectangular => PadShapeKind::RoundRect,
            PadShape::Octagonal => PadShapeKind::Octagonal,
            _ => PadShapeKind::Other,
        };

        let hole_size_mm = pad_spec
            .hole_size
            .map(|h| h.to_mms())
            .unwrap_or(0.0);
        let is_through_hole = hole_size_mm > 0.0;

        let layer_set = if is_through_hole {
            all_copper_layers.to_vec()
        } else {
            let layer_name = match &pad_spec.layer {
                altium_format_spec::model::LayerSpec::NamedLayer(n) => n.clone(),
                altium_format_spec::model::LayerSpec::Resolved(r) => {
                    format!("{r:?}")
                }
                altium_format_spec::model::LayerSpec::CopperPosition(_) => String::new(),
            };
            layer_lookup
                .get(&layer_name)
                .copied()
                .into_iter()
                .collect()
        };

        let pad_id = PadId::from(*next_pad_id);
        *next_pad_id += 1;
        ir_pads.push(IrComponentPad {
            id: pad_id,
            name: pad_spec.designator.clone(),
            local_position: local_pos,
            world_position: world_pos,
            net: net_id,
            shape: PadShapeInfo {
                kind: shape_kind,
                size_x: pad_spec.size_x.to_mms(),
                size_y: pad_spec.size_y.to_mms(),
                rotation: pad_spec.rotation,
            },
            is_through_hole,
            hole_size_mm,
            swap_id_pin: None,
            swap_id_part: None,
            layer_set,
        });
    }

    Ok(ir_pads)
}

// ---------------------------------------------------------------------------
// Net pin backfill
// ---------------------------------------------------------------------------

fn backfill_net_pins(
    mut nets: IdMap<NetId, IrNet>,
    components: &IdMap<ComponentId, IrComponent>,
) -> IdMap<NetId, IrNet> {
    for (comp_id, comp) in components.iter() {
        for pad in &comp.pads {
            if let Some(net_id) = pad.net {
                if let Some(net) = nets.get_mut(net_id) {
                    net.pins.push(IrNetPin {
                        pad: pad.id,
                        component: comp_id,
                        position: pad.world_position,
                    });
                }
            }
        }
    }

    for (_id, net) in nets.iter_mut() {
        let mut seen = std::collections::HashSet::new();
        for pin in &net.pins {
            seen.insert(pin.component.raw());
        }
        net.component_count = seen.len();
    }

    nets
}

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

fn compile_rules(
    board: &BoardSpec,
    layer_lookup: &BTreeMap<String, LayerId>,
) -> Result<IdMap<RuleId, IrDesignRule>, IrCompileError> {
    let mut rules: IdMap<RuleId, IrDesignRule> = IdMap::new();

    for rule_spec in &board.rules {
        let kind = parse_rule_kind(rule_spec.kind.as_deref().unwrap_or("Clearance"))?;

        let scope1 = resolve_scope(
            rule_spec.scope.as_deref().unwrap_or("All"),
            layer_lookup,
        )?;
        let scope2 = resolve_scope(
            rule_spec.scope2.as_deref().unwrap_or("All"),
            layer_lookup,
        )?;

        let params = compile_rule_params(kind, &rule_spec.properties, layer_lookup)?;

        let id = rules.push(IrDesignRule {
            id: RuleId::from(0),
            name: rule_spec.name.clone(),
            kind,
            priority: rule_spec.priority.unwrap_or(1),
            enabled: rule_spec.enabled.unwrap_or(true),
            scope: IrRuleScopePair { scope1, scope2 },
            params,
        });
        rules[id].id = id;
    }

    Ok(rules)
}

/// Parse a rule kind string to a [`RuleKind`].
fn parse_rule_kind(s: &str) -> Result<RuleKind, IrCompileError> {
    match s {
        "Clearance" => Ok(RuleKind::Clearance),
        "ParallelSegment" => Ok(RuleKind::ParallelSegment),
        "Width" => Ok(RuleKind::Width),
        "Length" => Ok(RuleKind::Length),
        "MatchedLengths" => Ok(RuleKind::MatchedLengths),
        "DaisyChainStubLength" => Ok(RuleKind::DaisyChainStubLength),
        "PowerPlaneConnectStyle" => Ok(RuleKind::PowerPlaneConnectStyle),
        "RoutingTopology" => Ok(RuleKind::RoutingTopology),
        "RoutingPriority" => Ok(RuleKind::RoutingPriority),
        "RoutingLayers" => Ok(RuleKind::RoutingLayers),
        "RoutingCornerStyle" => Ok(RuleKind::RoutingCornerStyle),
        "RoutingViaStyle" => Ok(RuleKind::RoutingViaStyle),
        "PowerPlaneClearance" => Ok(RuleKind::PowerPlaneClearance),
        "SolderMaskExpansion" => Ok(RuleKind::SolderMaskExpansion),
        "PasteMaskExpansion" => Ok(RuleKind::PasteMaskExpansion),
        "ShortCircuit" => Ok(RuleKind::ShortCircuit),
        "BrokenNets" => Ok(RuleKind::BrokenNets),
        "ViasUnderSmd" => Ok(RuleKind::ViasUnderSmd),
        "MaximumViaCount" => Ok(RuleKind::MaximumViaCount),
        "MinimumAnnularRing" => Ok(RuleKind::MinimumAnnularRing),
        "PolygonConnectStyle" => Ok(RuleKind::PolygonConnectStyle),
        "AcuteAngle" => Ok(RuleKind::AcuteAngle),
        "ConfinementConstraint" => Ok(RuleKind::ConfinementConstraint),
        "SmdToCorner" => Ok(RuleKind::SmdToCorner),
        "ComponentClearance" => Ok(RuleKind::ComponentClearance),
        "ComponentRotations" => Ok(RuleKind::ComponentRotations),
        "PermittedLayers" => Ok(RuleKind::PermittedLayers),
        "NetsToIgnore" => Ok(RuleKind::NetsToIgnore),
        "MaxMinHoleSize" => Ok(RuleKind::MaxMinHoleSize),
        "MaxMinHeight" => Ok(RuleKind::MaxMinHeight),
        "DifferentialPairsRouting" => Ok(RuleKind::DifferentialPairsRouting),
        "HoleToHoleClearance" => Ok(RuleKind::HoleToHoleClearance),
        "MinimumSolderMaskSliver" => Ok(RuleKind::MinimumSolderMaskSliver),
        "SilkToSolderMaskClearance" => Ok(RuleKind::SilkToSolderMaskClearance),
        "SilkToSilkClearance" => Ok(RuleKind::SilkToSilkClearance),
        "NetAntennae" => Ok(RuleKind::NetAntennae),
        "SilkToBoardRegionClearance" => Ok(RuleKind::SilkToBoardRegionClearance),
        "SmdEntry" => Ok(RuleKind::SmdEntry),
        "BoardOutlineClearance" => Ok(RuleKind::BoardOutlineClearance),
        "Creepage" => Ok(RuleKind::Creepage),
        "ZAxisClearance" => Ok(RuleKind::ZAxisClearance),
        "SmdNeckDown" => Ok(RuleKind::SmdNeckDown),
        "SmdToPlane" => Ok(RuleKind::SmdToPlane),
        "LayerPair" => Ok(RuleKind::LayerPair),
        "FanoutControl" => Ok(RuleKind::FanoutControl),
        "RoutingNeckDown" => Ok(RuleKind::RoutingNeckDown),
        "BackDrilling" => Ok(RuleKind::BackDrilling),
        "ReturnPath" => Ok(RuleKind::ReturnPath),
        "WireBonding" => Ok(RuleKind::WireBonding),
        "UnpouredPolygon" => Ok(RuleKind::UnpouredPolygon),
        "UnconnectedPin" => Ok(RuleKind::UnconnectedPin),
        "SignalStimulus" => Ok(RuleKind::SignalStimulus),
        "OvershootFallingEdge" => Ok(RuleKind::OvershootFallingEdge),
        "OvershootRisingEdge" => Ok(RuleKind::OvershootRisingEdge),
        "UndershootFallingEdge" => Ok(RuleKind::UndershootFallingEdge),
        "UndershootRisingEdge" => Ok(RuleKind::UndershootRisingEdge),
        "MaxMinImpedance" => Ok(RuleKind::MaxMinImpedance),
        "SignalTopValue" => Ok(RuleKind::SignalTopValue),
        "SignalBaseValue" => Ok(RuleKind::SignalBaseValue),
        "FlightTimeRisingEdge" => Ok(RuleKind::FlightTimeRisingEdge),
        "FlightTimeFallingEdge" => Ok(RuleKind::FlightTimeFallingEdge),
        "LayerStack" => Ok(RuleKind::LayerStack),
        "MaxSlopeRisingEdge" => Ok(RuleKind::MaxSlopeRisingEdge),
        "MaxSlopeFallingEdge" => Ok(RuleKind::MaxSlopeFallingEdge),
        "SupplyNets" => Ok(RuleKind::SupplyNets),
        "FabricationTestpointStyle" => Ok(RuleKind::FabricationTestpointStyle),
        "FabricationTestpointUsage" => Ok(RuleKind::FabricationTestpointUsage),
        "AssyTestPointStyle" => Ok(RuleKind::AssyTestPointStyle),
        "AssyTestPointUsage" => Ok(RuleKind::AssyTestPointUsage),
        "None" => Ok(RuleKind::None),
        other => Err(IrCompileError::UnknownRuleKind(other.to_string())),
    }
}

/// Convert a mm-valued string (e.g. `"0.25"`, `"0.25mm"`) to `f64`.
///
/// Returns `Err` containing the unparseable string on parse failure.
fn parse_mm_value(s: &str) -> Result<f64, String> {
    let trimmed = s
        .trim()
        .trim_end_matches("mm")
        .trim_end_matches("mil")
        .trim();
    // If "mil" suffix was present, convert mils to mm.
    let is_mil = s.trim().ends_with("mil");
    let v: f64 = trimmed.parse().map_err(|_| s.to_string())?;
    if is_mil {
        Ok(v * 0.0254)
    } else {
        Ok(v)
    }
}

/// Return `Ok(0.0)` when `key` is absent; `Err(InvalidPropertyValue)` when
/// the key is present but its value cannot be parsed as a mm quantity.
fn get_mm(
    props: &IndexMap<String, String>,
    key: &str,
) -> Result<f64, IrCompileError> {
    match props.get(key) {
        None => Ok(0.0),
        Some(s) => parse_mm_value(s)
            .map_err(|bad| IrCompileError::InvalidPropertyValue(key.to_string(), bad)),
    }
}

/// Return `Ok(0.0)` when `key` is absent; `Err(InvalidPropertyValue)` when
/// the key is present but its value cannot be parsed as `f64`.
fn get_f64(
    props: &IndexMap<String, String>,
    key: &str,
) -> Result<f64, IrCompileError> {
    match props.get(key) {
        None => Ok(0.0),
        Some(s) => s
            .trim()
            .parse::<f64>()
            .map_err(|_| IrCompileError::InvalidPropertyValue(key.to_string(), s.clone())),
    }
}

/// Return `Ok(0)` when `key` is absent; `Err(InvalidPropertyValue)` when
/// the key is present but its value cannot be parsed as `i32`.
fn get_i32(
    props: &IndexMap<String, String>,
    key: &str,
) -> Result<i32, IrCompileError> {
    match props.get(key) {
        None => Ok(0),
        Some(s) => s
            .trim()
            .parse::<i32>()
            .map_err(|_| IrCompileError::InvalidPropertyValue(key.to_string(), s.clone())),
    }
}

/// Return `Ok(0)` when `key` is absent; `Err(InvalidPropertyValue)` when
/// the key is present but its value cannot be parsed as `u32`.
fn get_u32(
    props: &IndexMap<String, String>,
    key: &str,
) -> Result<u32, IrCompileError> {
    match props.get(key) {
        None => Ok(0),
        Some(s) => s
            .trim()
            .parse::<u32>()
            .map_err(|_| IrCompileError::InvalidPropertyValue(key.to_string(), s.clone())),
    }
}

fn compile_rule_params(
    kind: RuleKind,
    props: &IndexMap<String, String>,
    layer_lookup: &BTreeMap<String, LayerId>,
) -> Result<IrRuleParams, IrCompileError> {
    let params = match kind {
        RuleKind::Clearance => IrRuleParams::Clearance {
            gap_mm: get_mm(props, "gap")?.max(get_mm(props, "min_gap")?),
        },
        RuleKind::Width => IrRuleParams::Width {
            min_mm: get_mm(props, "min")?,
            max_mm: get_mm(props, "max")?,
            preferred_mm: get_mm(props, "preferred")?,
        },
        RuleKind::ComponentClearance => IrRuleParams::ComponentClearance {
            gap_mm: get_mm(props, "gap")?,
        },
        RuleKind::BoardOutlineClearance => IrRuleParams::BoardOutlineClearance {
            gap_mm: get_mm(props, "gap")?,
        },
        RuleKind::HoleToHoleClearance => IrRuleParams::HoleToHoleClearance {
            gap_mm: get_mm(props, "gap")?,
        },
        RuleKind::MinimumAnnularRing => IrRuleParams::MinimumAnnularRing {
            min_mm: get_mm(props, "min")?,
        },
        RuleKind::SolderMaskExpansion => IrRuleParams::SolderMaskExpansion {
            expansion_mm: get_mm(props, "expansion")?,
        },
        RuleKind::PasteMaskExpansion => IrRuleParams::PasteMaskExpansion {
            expansion_mm: get_mm(props, "expansion")?,
        },
        RuleKind::RoutingPriority => IrRuleParams::RoutingPriority {
            priority: get_i32(props, "priority")?,
        },
        RuleKind::RoutingViaStyle => IrRuleParams::RoutingViaStyle {
            width_min_mm: get_mm(props, "min_width")?,
            width_max_mm: get_mm(props, "max_width")?,
            hole_min_mm: get_mm(props, "min_hole_width")?,
            hole_max_mm: get_mm(props, "max_hole_width")?,
        },
        RuleKind::MatchedLengths => IrRuleParams::MatchedLengths {
            tolerance_mm: get_mm(props, "tolerance")?,
        },
        RuleKind::ShortCircuit => IrRuleParams::ShortCircuit,
        RuleKind::BrokenNets => IrRuleParams::BrokenNets,
        RuleKind::NetAntennae => IrRuleParams::NetAntennae,
        RuleKind::ViasUnderSmd => IrRuleParams::ViasUnderSmd,
        RuleKind::AcuteAngle => IrRuleParams::AcuteAngle {
            min_angle_deg: get_f64(props, "min_angle")?,
        },
        RuleKind::SmdToCorner => IrRuleParams::SmdToCorner {
            clearance_mm: get_mm(props, "clearance")?,
        },
        RuleKind::MaximumViaCount => IrRuleParams::MaximumViaCount {
            max: get_u32(props, "max")?,
        },
        RuleKind::MaxMinHoleSize => IrRuleParams::MaxMinHoleSize {
            min_mm: get_mm(props, "min")?,
            max_mm: get_mm(props, "max")?,
        },
        RuleKind::Length => IrRuleParams::Length {
            min_mm: get_mm(props, "min")?,
            max_mm: get_mm(props, "max")?,
        },
        RuleKind::DaisyChainStubLength => IrRuleParams::DaisyChainStubLength {
            max_mm: get_mm(props, "max")?,
        },
        RuleKind::SmdNeckDown => IrRuleParams::SmdNeckDown,
        RuleKind::SmdEntry => IrRuleParams::SmdEntry,
        RuleKind::ParallelSegment => IrRuleParams::ParallelSegment {
            max_run_mm: get_mm(props, "max_run")?,
            check_gap_mm: get_mm(props, "check_gap")?,
        },
        RuleKind::MinimumSolderMaskSliver => IrRuleParams::MinimumSolderMaskSliver {
            min_mm: get_mm(props, "min")?,
        },
        RuleKind::SilkToSolderMaskClearance => IrRuleParams::SilkToSolderMaskClearance {
            clearance_mm: get_mm(props, "clearance")?,
        },
        RuleKind::SilkToSilkClearance => IrRuleParams::SilkToSilkClearance {
            clearance_mm: get_mm(props, "clearance")?,
        },
        RuleKind::SilkToBoardRegionClearance => IrRuleParams::SilkToBoardRegionClearance {
            clearance_mm: get_mm(props, "clearance")?,
        },
        RuleKind::PowerPlaneClearance => IrRuleParams::PowerPlaneClearance {
            gap_mm: get_mm(props, "gap")?,
        },
        RuleKind::PolygonConnectStyle => IrRuleParams::PolygonConnectStyle,
        RuleKind::Creepage => IrRuleParams::Creepage {
            min_mm: get_mm(props, "min")?,
        },
        RuleKind::MaxMinHeight => IrRuleParams::MaxMinHeight {
            min_mm: get_mm(props, "min")?,
            max_mm: get_mm(props, "max")?,
        },
        RuleKind::ZAxisClearance => IrRuleParams::ZAxisClearance {
            min_mm: get_mm(props, "min")?,
        },
        RuleKind::RoutingTopology => {
            let topology = props
                .get("topology")
                .and_then(|s| match s.trim() {
                    "Shortest" | "shortest" => Some(NetTopology::Shortest),
                    "Horizontal" | "horizontal" => Some(NetTopology::Horizontal),
                    "Vertical" | "vertical" => Some(NetTopology::Vertical),
                    "DaisyChainSimple" | "daisy_chain_simple" => Some(NetTopology::DaisyChainSimple),
                    "DaisyChainMidDriven" | "daisy_chain_mid_driven" => Some(NetTopology::DaisyChainMidDriven),
                    "DaisyChainBalanced" | "daisy_chain_balanced" => Some(NetTopology::DaisyChainBalanced),
                    "Starburst" | "starburst" => Some(NetTopology::Starburst),
                    _ => None,
                })
                .unwrap_or(NetTopology::Shortest);
            IrRuleParams::RoutingTopology { topology }
        }
        RuleKind::RoutingLayers => {
            let allowed = props
                .iter()
                .filter(|(_, v)| v.trim().eq_ignore_ascii_case("true") || v.trim() == "1")
                .filter_map(|(k, _)| layer_lookup.get(k.as_str()).copied())
                .collect();
            IrRuleParams::RoutingLayers { allowed }
        }
        RuleKind::RoutingCornerStyle => {
            let style = props
                .get("style")
                .and_then(|s| match s.trim() {
                    "45" | "Degree45" => Some(CornerStyle::Degree45),
                    "90" | "Degree90" => Some(CornerStyle::Degree90),
                    "Round" | "round" => Some(CornerStyle::Round),
                    _ => None,
                })
                .unwrap_or(CornerStyle::Degree45);
            IrRuleParams::RoutingCornerStyle { style }
        }
        RuleKind::DifferentialPairsRouting => IrRuleParams::DiffPairsRouting {
            gap_mm: get_mm(props, "gap")?,
            max_gap_mm: get_mm(props, "max_gap")?,
            max_uncoupled_length_mm: get_mm(props, "max_uncoupled_length")?,
        },
        _ => IrRuleParams::Other { kind },
    };
    Ok(params)
}

// ---------------------------------------------------------------------------
// Scope resolution
// ---------------------------------------------------------------------------

/// Parse a scope expression string into an [`IrRuleScope`].
///
/// Supported forms:
/// - `""`, `"All"` → `IrRuleScope::All`
/// - `"InNetClass(<name>)"` → `IrRuleScope::NetClass(name)`
/// - `"OnLayer(<name>)"` → `IrRuleScope::Layer(layer_id)`
/// - `"InNetClass(<name>) And OnLayer(<name>)"` → `IrRuleScope::NetClassAndLayer`
pub(crate) fn resolve_scope(
    scope_str: &str,
    layer_lookup: &BTreeMap<String, LayerId>,
) -> Result<IrRuleScope, IrCompileError> {
    let s = scope_str.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("all") {
        return Ok(IrRuleScope::All);
    }

    // Check for combined "InNetClass(...) And OnLayer(...)"
    let lower = s.to_ascii_lowercase();
    if let Some(and_pos) = lower.find(" and ") {
        let left = s[..and_pos].trim();
        let right = s[and_pos + 5..].trim();
        let net_class = extract_parens(left, "InNetClass")
            .ok_or_else(|| IrCompileError::InvalidScope(s.to_string()))?;
        let layer_name = extract_parens(right, "OnLayer")
            .ok_or_else(|| IrCompileError::InvalidScope(s.to_string()))?;
        let layer_id = layer_lookup
            .get(layer_name)
            .copied()
            .ok_or_else(|| IrCompileError::UnknownLayer(layer_name.to_string()))?;
        return Ok(IrRuleScope::NetClassAndLayer(net_class.to_string(), layer_id));
    }

    // Single-clause forms
    if let Some(class_name) = extract_parens(s, "InNetClass") {
        return Ok(IrRuleScope::NetClass(class_name.to_string()));
    }
    if let Some(layer_name) = extract_parens(s, "OnLayer") {
        let layer_id = layer_lookup
            .get(layer_name)
            .copied()
            .ok_or_else(|| IrCompileError::UnknownLayer(layer_name.to_string()))?;
        return Ok(IrRuleScope::Layer(layer_id));
    }

    // Unrecognized scope expressions (e.g., InPolygon, InNet, IsVia, etc.)
    // are treated as All (global) — the rule applies everywhere. This is safe
    // because narrowing scope to All is conservative (may produce false DRC
    // violations but never silently hides real ones).
    Ok(IrRuleScope::All)
}

/// Extract the content inside `prefix(...)`, case-insensitively.
/// Returns `None` if the string does not match the pattern.
fn extract_parens<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let lower_s = s.to_ascii_lowercase();
    let lower_prefix = prefix.to_ascii_lowercase();
    if !lower_s.starts_with(lower_prefix.as_str()) {
        return None;
    }
    let after = &s[prefix.len()..].trim_start();
    if after.starts_with('(') && after.ends_with(')') {
        Some(after[1..after.len() - 1].trim())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Board geometry
// ---------------------------------------------------------------------------

fn compile_board_geometry(board: &BoardSpec) -> Result<IrBoardGeometry, IrCompileError> {
    let outline_pts = board
        .outline
        .as_ref()
        .ok_or(IrCompileError::MissingBoardOutline)?;

    let outline: Vec<PointMm> = outline_pts
        .iter()
        .map(|cp| PointMm::new(cp.x.to_mms(), cp.y.to_mms()))
        .collect();

    let bounds = BoundingBoxMm::from_points(&outline)
        .unwrap_or(BoundingBoxMm::new(PointMm::new(0.0, 0.0), PointMm::new(0.0, 0.0)));

    let keepouts: Vec<IrKeepoutZone> = board
        .keepouts
        .iter()
        .map(|kz| IrKeepoutZone {
            outline: kz
                .vertices
                .iter()
                .map(|cp| PointMm::new(cp.x.to_mms(), cp.y.to_mms()))
                .collect(),
            layer_name: kz.layer.as_ref().map(|l| match l {
                altium_format_spec::model::LayerSpec::NamedLayer(n) => n.clone(),
                altium_format_spec::model::LayerSpec::Resolved(r) => format!("{r:?}"),
                altium_format_spec::model::LayerSpec::CopperPosition(n) => {
                    format!("copper({n})")
                }
            }),
        })
        .collect();

    Ok(IrBoardGeometry {
        outline,
        cutouts: Vec::new(),
        bounds,
        keepouts,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use altium_format_spec::model::{
        BoardLayerSpec, BoardSpec, PcbDocComponentSpec, PcbDocNetSpec, PcbDocRuleSpec, PcbDocSpec,
    };
    use altium_format_types::{Coord, CoordPoint};

    fn minimal_spec() -> PcbDocSpec {
        PcbDocSpec {
            boards: vec![minimal_board()],
            placement: None,
            placement_rules: vec![],
            routing: None,
        }
    }

    fn cp(x_mils: f64, y_mils: f64) -> CoordPoint {
        CoordPoint {
            x: Coord::from_mils_f64(x_mils),
            y: Coord::from_mils_f64(y_mils),
        }
    }

    fn minimal_board() -> BoardSpec {
        BoardSpec {
            annotation: None,
            name: "TestBoard".to_string(),
            signal_layer_count: None,
            snap_grid_size: None,
            visible_grid_size: None,
            display_unit: None,
            outline: Some(vec![
                cp(0.0, 0.0),
                cp(4000.0, 0.0),
                cp(4000.0, 3000.0),
                cp(0.0, 3000.0),
            ]),
            keepouts: vec![],
            layers: vec![
                BoardLayerSpec {
                    name: "TopLayer".to_string(),
                    is_copper: true,
                    copper_index: Some(1),
                },
                BoardLayerSpec {
                    name: "BottomLayer".to_string(),
                    is_copper: true,
                    copper_index: Some(2),
                },
            ],
            nets: vec![
                PcbDocNetSpec {
                    annotation: None,
                    name: "GND".to_string(),
                    color: None,
                    visible: None,
                },
                PcbDocNetSpec {
                    annotation: None,
                    name: "VCC".to_string(),
                    color: None,
                    visible: None,
                },
            ],
            components: vec![
                PcbDocComponentSpec {
                    annotation: None,
                    designator: "U1".to_string(),
                    pattern: Some("SOT23".to_string()),
                    comment: Some("100nF".to_string()),
                    location: Some(cp(1000.0, 1000.0)),
                    rotation: Some(0.0),
                    layer: None,
                    source_library: None,
                    parameters: IndexMap::new(),
                    pads: vec![],
                },
            ],
            tracks: vec![],
            arcs: vec![],
            vias: vec![],
            pads: vec![],
            fills: vec![],
            texts: vec![],
            regions: vec![],
            component_bodies: vec![],
            dimensions: vec![],
            polygons: vec![],
            rules: vec![
                PcbDocRuleSpec {
                    annotation: None,
                    name: "ClearanceRule".to_string(),
                    kind: Some("Clearance".to_string()),
                    enabled: Some(true),
                    priority: Some(1),
                    properties: {
                        let mut m = IndexMap::new();
                        m.insert("gap".to_string(), "0.25".to_string());
                        m
                    },
                    scope: None,
                    scope2: None,
                },
            ],
            classes: vec![],
            differential_pairs: vec![],
        }
    }

    #[test]
    fn minimal_spec_compiles_to_valid_ir() {
        let spec = minimal_spec();
        let ir = spec_to_ir(&spec).expect("spec_to_ir should succeed");

        assert_eq!(ir.components.len(), 1);
        assert_eq!(ir.nets.len(), 2);
        assert_eq!(ir.rules.len(), 1);
        assert_eq!(ir.layer_stack.copper_layer_count, 2);
        assert!(!ir.board.outline.is_empty());
    }

    #[test]
    fn no_boards_returns_error() {
        let spec = PcbDocSpec {
            boards: vec![],
            placement: None,
            placement_rules: vec![],
            routing: None,
        };
        assert!(matches!(
            spec_to_ir(&spec),
            Err(IrCompileError::NoBoardsDefined)
        ));
    }

    // ── Shape function → outline pipeline tests ─────────────────────────────

    fn parse_and_compile_pcbdoc(source: &str) -> PcbDocSpec {
        use altium_format_spec::compiler::compile_spec;
        use altium_format_spec::model::SpecDomain;
        use altium_format_spec::parser::parse_spec;

        let file = parse_spec(source).expect("parse_spec should succeed");
        match compile_spec(&file, SpecDomain::PcbDoc).expect("compile_spec should succeed") {
            altium_format_spec::model::SpecModel::PcbDoc(spec) => spec,
            _ => panic!("expected PcbDoc model"),
        }
    }

    #[test]
    fn rect_outline_produces_4_vertices_in_ir() {
        let source = r#"
board "shape-test" {
    signal_layer_count: 2
    outline: rect(50mm, 30mm)
}
net VCC {}
net GND {}
"#;
        let spec = parse_and_compile_pcbdoc(source);
        let board = spec.boards.first().expect("should have a board");
        let outline = board.outline.as_ref().expect("outline should be Some");
        assert_eq!(outline.len(), 4, "rect() should produce 4 vertices");

        let ir = spec_to_ir(&spec).expect("spec_to_ir should succeed");
        assert_eq!(ir.board.outline.len(), 4, "IR outline should have 4 points");

        let b = &ir.board.bounds;
        assert!((b.width() - 50.0).abs() < 0.01, "board width should be ~50mm, got {}", b.width());
        assert!((b.height() - 30.0).abs() < 0.01, "board height should be ~30mm, got {}", b.height());
    }

    #[test]
    fn rounded_rect_outline_in_ir() {
        let source = r#"
board "shape-test" {
    signal_layer_count: 2
    outline: rounded_rect(50mm, 30mm, 3mm)
}
net VCC {}
net GND {}
"#;
        let spec = parse_and_compile_pcbdoc(source);
        let ir = spec_to_ir(&spec).expect("spec_to_ir should succeed");
        assert!(ir.board.outline.len() > 4, "rounded_rect should produce more than 4 vertices");
        let b = &ir.board.bounds;
        assert!((b.width() - 50.0).abs() < 0.01, "board width should be ~50mm");
        assert!((b.height() - 30.0).abs() < 0.01, "board height should be ~30mm");
    }

    #[test]
    fn rect_from_to_outline_in_ir() {
        let source = r#"
board "shape-test" {
    signal_layer_count: 2
    outline: rect(from: (0mm, 0mm), to: (50mm, 30mm))
}
net VCC {}
net GND {}
"#;
        let spec = parse_and_compile_pcbdoc(source);
        let ir = spec_to_ir(&spec).expect("spec_to_ir should succeed");
        assert_eq!(ir.board.outline.len(), 4, "rect(from:,to:) should produce 4 vertices");
        let b = &ir.board.bounds;
        assert!((b.width() - 50.0).abs() < 0.01, "board width should be ~50mm");
        assert!((b.height() - 30.0).abs() < 0.01, "board height should be ~30mm");
    }

    #[test]
    fn let_binding_shape_outline_in_ir() {
        let source = r#"
board "shape-test" {
    signal_layer_count: 2
    let board_shape = rect(50mm, 30mm)
    outline: board_shape
}
net VCC {}
net GND {}
"#;
        let spec = parse_and_compile_pcbdoc(source);
        let ir = spec_to_ir(&spec).expect("spec_to_ir should succeed");
        assert_eq!(ir.board.outline.len(), 4, "let binding shape should produce 4 vertices");
        let b = &ir.board.bounds;
        assert!((b.width() - 50.0).abs() < 0.01, "board width should be ~50mm");
        assert!((b.height() - 30.0).abs() < 0.01, "board height should be ~30mm");
    }

    #[test]
    fn circle_outline_produces_72_vertices_in_ir() {
        let source = r#"
board "shape-test" {
    signal_layer_count: 2
    outline: circle(25mm)
}
net VCC {}
net GND {}
"#;
        let spec = parse_and_compile_pcbdoc(source);
        let ir = spec_to_ir(&spec).expect("spec_to_ir should succeed");
        assert_eq!(ir.board.outline.len(), 72, "circle() should produce 72-vertex polygon approximation");
        let b = &ir.board.bounds;
        assert!((b.width() - 50.0).abs() < 0.1, "circle diameter (width) should be ~50mm, got {}", b.width());
        assert!((b.height() - 50.0).abs() < 0.1, "circle diameter (height) should be ~50mm, got {}", b.height());
    }

    #[test]
    fn width_rule_with_net_class_scope() {
        let spec = PcbDocSpec {
            boards: vec![{
                let mut b = minimal_board();
                b.rules = vec![PcbDocRuleSpec {
                    annotation: None,
                    name: "PowerWidth".to_string(),
                    kind: Some("Width".to_string()),
                    enabled: Some(true),
                    priority: Some(2),
                    properties: {
                        let mut m = IndexMap::new();
                        m.insert("min".to_string(), "0.5".to_string());
                        m.insert("max".to_string(), "2.0".to_string());
                        m.insert("preferred".to_string(), "1.0".to_string());
                        m
                    },
                    scope: Some("InNetClass(Power)".to_string()),
                    scope2: None,
                }];
                b
            }],
            placement: None,
            placement_rules: vec![],
            routing: None,
        };

        let ir = spec_to_ir(&spec).expect("spec_to_ir should succeed");
        assert_eq!(ir.rules.len(), 1);
        let rule = &ir.rules[RuleId::from(0)];
        assert_eq!(
            rule.scope.scope1,
            IrRuleScope::NetClass("Power".to_string())
        );
        assert_eq!(rule.scope.scope2, IrRuleScope::All);
        match &rule.params {
            IrRuleParams::Width {
                min_mm,
                max_mm,
                preferred_mm,
            } => {
                assert!((min_mm - 0.5).abs() < 1e-9);
                assert!((max_mm - 2.0).abs() < 1e-9);
                assert!((preferred_mm - 1.0).abs() < 1e-9);
            }
            other => panic!("expected Width params, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_designator_returns_error() {
        let spec = PcbDocSpec {
            boards: vec![{
                let mut b = minimal_board();
                b.components.push(PcbDocComponentSpec {
                    annotation: None,
                    designator: "U1".to_string(),
                    pattern: None,
                    comment: None,
                    location: None,
                    rotation: None,
                    layer: None,
                    source_library: None,
                    parameters: IndexMap::new(),
                    pads: vec![],
                });
                b
            }],
            placement: None,
            placement_rules: vec![],
            routing: None,
        };
        assert!(matches!(
            spec_to_ir(&spec),
            Err(IrCompileError::DuplicateDesignator(_))
        ));
    }

    #[test]
    fn resolve_scope_all_variants() {
        let mut layer_lookup = BTreeMap::new();
        layer_lookup.insert("TopLayer".to_string(), LayerId::from(0));
        layer_lookup.insert("BottomLayer".to_string(), LayerId::from(1));

        assert_eq!(
            resolve_scope("All", &layer_lookup).unwrap(),
            IrRuleScope::All
        );
        assert_eq!(
            resolve_scope("", &layer_lookup).unwrap(),
            IrRuleScope::All
        );
        assert_eq!(
            resolve_scope("InNetClass(Power)", &layer_lookup).unwrap(),
            IrRuleScope::NetClass("Power".to_string())
        );
        assert_eq!(
            resolve_scope("OnLayer(TopLayer)", &layer_lookup).unwrap(),
            IrRuleScope::Layer(LayerId::from(0))
        );
        assert_eq!(
            resolve_scope(
                "InNetClass(Power) And OnLayer(TopLayer)",
                &layer_lookup
            )
            .unwrap(),
            IrRuleScope::NetClassAndLayer("Power".to_string(), LayerId::from(0))
        );
    }
}

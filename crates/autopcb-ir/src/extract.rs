//! Extraction of `PcbIr` from an `altium_format::PcbDocBoard`.

use std::collections::HashMap;

use altium_format::api::{
    BoardContour, ContourSegment, PcbDocBoard, RuleParams,
};
use altium_format_types::pcb::{RuleKind, V6Layer};

use crate::board::{IrBoardGeometry, IrKeepoutZone};
use crate::component::{IrComponent, IrComponentPad, PadShapeInfo, PadShapeKind};
use crate::copper::{FreeCopperGeometry, IrFill, IrTrack, IrVia};
use crate::handles::{ComponentId, IdMap, LayerId, NetId, PadId, PolygonId, RuleId};
use crate::layer_stack::{IrCopperLayer, IrLayerStack};
use crate::net::{IrNet, IrNetPin};
use crate::polygon::IrPolygon;
use crate::rule::{IrDesignRule, IrRuleParams};
use crate::types::{BoardSide, BoundingBoxMm, PointMm};
use crate::{IrError, Result};

/// The complete intermediate representation of a PcbDoc board.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PcbIr {
    pub board: IrBoardGeometry,
    pub layer_stack: IrLayerStack,
    pub components: IdMap<ComponentId, IrComponent>,
    pub nets: IdMap<NetId, IrNet>,
    pub rules: IdMap<RuleId, IrDesignRule>,
    pub free_copper: FreeCopperGeometry,
    pub polygons: IdMap<PolygonId, IrPolygon>,
}

impl PcbIr {
    /// Extract an IR from a parsed PcbDoc board.
    pub fn extract(board: &PcbDocBoard) -> Result<Self> {
        let ir_board = extract_board_geometry(board)?;
        let layer_stack = extract_layer_stack(board);
        // Build a name → LayerId lookup shared across extraction steps.
        let layer_lookup: HashMap<String, LayerId> = layer_stack
            .copper_layers
            .iter()
            .map(|l| (l.name.clone(), l.id))
            .collect();
        let (net_lookup, mut nets) = extract_nets(board);
        let mut components = extract_components(board, &net_lookup, &layer_stack)?;
        backfill_net_pins(&mut nets, &components);
        compute_component_bounds(&mut components);
        let rules = extract_rules(board, &layer_lookup);
        let free_copper = extract_free_copper(board, &net_lookup, &layer_lookup)?;
        let polygons = extract_polygons(board, &net_lookup);

        Ok(PcbIr {
            board: ir_board,
            layer_stack,
            components,
            nets,
            rules,
            free_copper,
            polygons,
        })
    }
}

// ---------------------------------------------------------------------------
// Board geometry
// ---------------------------------------------------------------------------

fn extract_board_geometry(board: &PcbDocBoard) -> Result<IrBoardGeometry> {
    let outline_contour = board
        .settings
        .geometry
        .outline
        .as_ref()
        .ok_or(IrError::NoBoardOutline)?;

    let outline = tessellate_contour(outline_contour);
    let bounds = BoundingBoxMm::from_points(&outline)
        .ok_or_else(|| IrError::ExtractionError("empty board outline".into()))?;

    let cutouts = board
        .settings
        .geometry
        .cutouts
        .iter()
        .map(tessellate_contour)
        .collect();

    let keepouts = board
        .settings
        .geometry
        .keepouts
        .iter()
        .map(|kz| IrKeepoutZone {
            outline: tessellate_contour(&kz.outline),
            layer_name: kz.layer.display_name().map(|s| s.to_string()),
        })
        .collect();

    Ok(IrBoardGeometry {
        outline,
        cutouts,
        bounds,
        keepouts,
    })
}

/// Convert a `PcbContour` (which may contain arcs) into a `Vec<PointMm>`.
/// Line segments pass through directly; arcs are sampled at ~1° intervals.
fn tessellate_contour(contour: &BoardContour) -> Vec<PointMm> {
    let mut points = Vec::new();
    for seg in &contour.segments {
        match seg {
            ContourSegment::Line { endpoint } => {
                points.push(PointMm::from_coord_point(endpoint));
            }
            ContourSegment::Arc {
                endpoint,
                center,
                radius,
                start_angle,
                end_angle,
            } => {
                let cx = center.x.to_mms();
                let cy = center.y.to_mms();
                let r = radius.to_mms();

                // Determine arc sweep direction
                let mut sweep = end_angle - start_angle;
                if sweep <= 0.0 {
                    sweep += 360.0;
                }
                let steps = (sweep.abs() as usize).max(1);
                let step_deg = sweep / steps as f64;

                for i in 1..=steps {
                    let angle_deg = start_angle + step_deg * i as f64;
                    let angle_rad = angle_deg.to_radians();
                    points.push(PointMm::new(
                        cx + r * angle_rad.cos(),
                        cy + r * angle_rad.sin(),
                    ));
                }
                // Ensure we land exactly on the endpoint
                if let Some(last) = points.last_mut() {
                    *last = PointMm::from_coord_point(endpoint);
                }
            }
        }
    }
    points
}

// ---------------------------------------------------------------------------
// Layer stack
// ---------------------------------------------------------------------------

fn extract_layer_stack(board: &PcbDocBoard) -> IrLayerStack {
    let stack = &board.settings.layer_stack;
    let layer_count = stack.layers.len();
    let mut layers = IdMap::<LayerId, IrCopperLayer>::new();
    for (i, sl) in stack.layers.iter().enumerate() {
        let id = layers.push(IrCopperLayer {
            id: LayerId::from(0),
            name: sl.name.clone(),
            is_top: i == 0,
            is_bottom: i == layer_count - 1,
            preferred_direction: None,
        });
        layers[id].id = id;
    }
    IrLayerStack {
        copper_layers: layers.iter().map(|(_, l)| l.clone()).collect(),
        copper_layer_count: stack.copper_layer_count,
    }
}

// ---------------------------------------------------------------------------
// Nets
// ---------------------------------------------------------------------------

fn extract_nets(board: &PcbDocBoard) -> (HashMap<String, NetId>, IdMap<NetId, IrNet>) {
    let mut lookup = HashMap::new();
    let mut nets = IdMap::new();
    for n in &board.nets {
        let id = nets.push(IrNet {
            id: NetId::from(0),
            name: n.name.clone(),
            pins: Vec::new(),
            component_count: 0,
            net_class: None,
            diff_pair_partner: None,
        });
        nets[id].id = id;
        lookup.insert(n.name.clone(), id);
    }

    // Populate net_class: for each net-class whose members include a net name,
    // assign that class name (last write wins if a net belongs to multiple classes).
    for class in &board.classes {
        for member_name in &class.members {
            if let Some(&net_id) = lookup.get(member_name.as_str()) {
                nets[net_id].net_class = Some(class.name.clone());
            }
        }
    }

    // Populate diff_pair_partner from differential pair definitions.
    for dp in &board.differential_pairs {
        if let (Some(&pos_id), Some(&neg_id)) = (
            lookup.get(dp.positive_net.as_str()),
            lookup.get(dp.negative_net.as_str()),
        ) {
            nets[pos_id].diff_pair_partner = Some(neg_id);
            nets[neg_id].diff_pair_partner = Some(pos_id);
        }
    }

    (lookup, nets)
}

// ---------------------------------------------------------------------------
// Components + pads
// ---------------------------------------------------------------------------

fn extract_components(
    board: &PcbDocBoard,
    net_lookup: &HashMap<String, NetId>,
    layer_stack: &IrLayerStack,
) -> Result<IdMap<ComponentId, IrComponent>> {
    let mut components = IdMap::with_capacity(board.components.len());
    let mut next_pad_id: u32 = 0;

    // Build name → LayerId lookup for resolving pad layer_set entries.
    let layer_name_to_id: HashMap<String, LayerId> = layer_stack
        .copper_layers
        .iter()
        .map(|l| (l.name.clone(), l.id))
        .collect();

    // Collect all copper LayerIds for through-hole pads.
    let all_copper_layers: Vec<LayerId> = layer_stack
        .copper_layers
        .iter()
        .map(|l| l.id)
        .collect();

    for comp in &board.components {
        let comp_pos = PointMm::from_coord_point(&comp.location);
        let rotation = comp.rotation;
        // Determine side: check display name first (V7-safe), fall back to V6
        let side = if comp.layer.display_name().is_some_and(|n| n.contains("Top"))
            || comp.layer.to_v6().is_some_and(|v6| v6 == V6Layer::TopLayer)
        {
            BoardSide::Top
        } else {
            BoardSide::Bottom
        };

        let pads_iter = board.pads_for_component(&comp.designator);
        let mut ir_pads = Vec::new();

        for pad in pads_iter {
            let world_pos = PointMm::from_coord_point(&pad.location);
            let local_pos = world_to_local(world_pos, comp_pos, rotation);
            let net_id = pad.net.as_ref().and_then(|n| net_lookup.get(n)).copied();

            let shape_kind = match pad.shape {
                altium_format_types::pcb::PadShape::Round => PadShapeKind::Round,
                altium_format_types::pcb::PadShape::Rectangular => PadShapeKind::Rectangular,
                altium_format_types::pcb::PadShape::RoundRect
                | altium_format_types::pcb::PadShape::RoundedRectangular => PadShapeKind::RoundRect,
                altium_format_types::pcb::PadShape::Octagonal => PadShapeKind::Octagonal,
                _ => PadShapeKind::Other,
            };

            let is_through_hole = pad.hole_size.to_mms() > 0.0;

            // Build layer_set: through-hole pads span all copper layers;
            // SMD pads exist only on the layer reported by the pad record.
            let layer_set = if is_through_hole {
                all_copper_layers.clone()
            } else {
                let pad_layer_name = pad
                    .layer
                    .display_name()
                    .unwrap_or("Unknown")
                    .to_string();
                layer_name_to_id
                    .get(&pad_layer_name)
                    .copied()
                    .into_iter()
                    .collect()
            };

            let pad_id = PadId::from(next_pad_id);
            next_pad_id += 1;
            ir_pads.push(IrComponentPad {
                id: pad_id,
                name: pad.pad_name.clone(),
                local_position: local_pos,
                world_position: world_pos,
                net: net_id,
                shape: PadShapeInfo {
                    kind: shape_kind,
                    size_x: pad.x_size.to_mms(),
                    size_y: pad.y_size.to_mms(),
                    rotation: pad.rotation,
                },
                is_through_hole,
                hole_size_mm: pad.hole_size.to_mms(),
                swap_id_pin: pad.swap_id_pin.clone(),
                swap_id_part: pad.swap_id_part.clone(),
                layer_set,
            });

            // pad_id already assigned above
        }

        // Placeholder bounds — computed in a second pass
        let zero_bb = BoundingBoxMm::new(PointMm::new(0.0, 0.0), PointMm::new(0.0, 0.0));
        let comp_id = components.push(IrComponent {
            id: ComponentId::from(0),
            designator: comp.designator.clone(),
            pattern: comp.pattern.clone(),
            value: comp.comment.clone(),
            position: comp_pos,
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

/// Convert a world position back to component-local coordinates.
fn world_to_local(world: PointMm, comp_pos: PointMm, rotation_deg: f64) -> PointMm {
    let dx = world.x - comp_pos.x;
    let dy = world.y - comp_pos.y;
    if rotation_deg.abs() < 1e-6 {
        return PointMm::new(dx, dy);
    }
    let angle = -rotation_deg.to_radians();
    PointMm::new(
        dx * angle.cos() - dy * angle.sin(),
        dx * angle.sin() + dy * angle.cos(),
    )
}

/// Compute bounding boxes from pad extents.
fn compute_component_bounds(components: &mut IdMap<ComponentId, IrComponent>) {
    for (_id, comp) in components.iter_mut() {
        if comp.pads.is_empty() {
            // Fallback: 1mm box around component position
            comp.world_bounds = BoundingBoxMm::new(comp.position, comp.position).expand(0.5);
            comp.local_bounds = BoundingBoxMm::new(PointMm::new(0.0, 0.0), PointMm::new(0.0, 0.0)).expand(0.5);
            continue;
        }

        // World bounds from pad world positions + pad sizes
        let world_points: Vec<PointMm> = comp
            .pads
            .iter()
            .flat_map(|p| {
                let half_x = p.shape.size_x / 2.0;
                let half_y = p.shape.size_y / 2.0;
                [
                    PointMm::new(p.world_position.x - half_x, p.world_position.y - half_y),
                    PointMm::new(p.world_position.x + half_x, p.world_position.y + half_y),
                ]
            })
            .collect();
        comp.world_bounds = BoundingBoxMm::from_points(&world_points)
            .unwrap_or_else(|| BoundingBoxMm::new(comp.position, comp.position));

        // Local bounds from pad local positions + pad sizes
        let local_points: Vec<PointMm> = comp
            .pads
            .iter()
            .flat_map(|p| {
                let half_x = p.shape.size_x / 2.0;
                let half_y = p.shape.size_y / 2.0;
                [
                    PointMm::new(p.local_position.x - half_x, p.local_position.y - half_y),
                    PointMm::new(p.local_position.x + half_x, p.local_position.y + half_y),
                ]
            })
            .collect();
        comp.local_bounds = BoundingBoxMm::from_points(&local_points)
            .unwrap_or_else(|| BoundingBoxMm::new(PointMm::new(0.0, 0.0), PointMm::new(0.0, 0.0)));
    }
}

// ---------------------------------------------------------------------------
// Net pin backfill
// ---------------------------------------------------------------------------

fn backfill_net_pins(
    nets: &mut IdMap<NetId, IrNet>,
    components: &IdMap<ComponentId, IrComponent>,
) {
    // Build pin lists from component pads
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

    // Count distinct components per net
    for (_id, net) in nets.iter_mut() {
        let mut seen = std::collections::HashSet::new();
        for pin in &net.pins {
            seen.insert(pin.component.raw());
        }
        net.component_count = seen.len();
    }
}

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

fn extract_rules(
    board: &PcbDocBoard,
    layer_lookup: &HashMap<String, LayerId>,
) -> IdMap<RuleId, IrDesignRule> {
    let mut rules = IdMap::new();
    for r in &board.rules {
        let params = match &r.params {
            RuleParams::Clearance { gap, .. } => IrRuleParams::Clearance {
                gap_mm: gap.to_mms(),
            },
            RuleParams::Width {
                min,
                max,
                preferred,
            } => IrRuleParams::Width {
                min_mm: min.to_mms(),
                max_mm: max.to_mms(),
                preferred_mm: preferred.to_mms(),
            },
            RuleParams::ComponentClearance { gap, .. } => IrRuleParams::ComponentClearance {
                gap_mm: gap.to_mms(),
            },
            RuleParams::BoardOutlineClearance { gap } => IrRuleParams::BoardOutlineClearance {
                gap_mm: gap.to_mms(),
            },
            RuleParams::HoleToHoleClearance { gap } => IrRuleParams::HoleToHoleClearance {
                gap_mm: gap.to_mms(),
            },
            RuleParams::MinimumAnnularRing { min } => IrRuleParams::MinimumAnnularRing {
                min_mm: min.to_mms(),
            },
            RuleParams::SolderMaskExpansion { expansion, .. } => {
                IrRuleParams::SolderMaskExpansion {
                    expansion_mm: expansion.to_mms(),
                }
            }
            RuleParams::PasteMaskExpansion { expansion, .. } => {
                IrRuleParams::PasteMaskExpansion {
                    expansion_mm: expansion.to_mms(),
                }
            }
            RuleParams::RoutingTopology { topology } => IrRuleParams::RoutingTopology {
                topology: *topology,
            },
            RuleParams::RoutingPriority { priority } => IrRuleParams::RoutingPriority {
                priority: *priority,
            },
            RuleParams::RoutingLayers { layer_flags } => {
                let allowed = layer_flags
                    .iter()
                    .filter(|(_, enabled)| *enabled)
                    .filter_map(|(name, _)| layer_lookup.get(name).copied())
                    .collect();
                IrRuleParams::RoutingLayers { allowed }
            }
            RuleParams::RoutingViaStyle {
                min_width,
                max_width,
                min_hole_width,
                max_hole_width,
                ..
            } => IrRuleParams::RoutingViaStyle {
                width_min_mm: min_width.to_mms(),
                width_max_mm: max_width.to_mms(),
                hole_min_mm: min_hole_width.to_mms(),
                hole_max_mm: max_hole_width.to_mms(),
            },
            RuleParams::RoutingCornerStyle { corner_style, .. } => {
                IrRuleParams::RoutingCornerStyle {
                    style: *corner_style,
                }
            }
            RuleParams::DiffPairsRouting {
                min_gap,
                max_gap,
                max_uncoupled_length,
                ..
            } => IrRuleParams::DiffPairsRouting {
                gap_mm: min_gap.to_mms(),
                max_gap_mm: max_gap.to_mms(),
                max_uncoupled_length_mm: max_uncoupled_length.to_mms(),
            },
            RuleParams::MatchedLengths { tolerance } => IrRuleParams::MatchedLengths {
                tolerance_mm: tolerance.to_mms(),
            },
            RuleParams::ShortCircuit { .. } => IrRuleParams::ShortCircuit,
            RuleParams::BrokenNets { .. } => IrRuleParams::BrokenNets,
            RuleParams::NetAntennae { .. } => IrRuleParams::NetAntennae,
            RuleParams::ViasUnderSmd { .. } => IrRuleParams::ViasUnderSmd,
            RuleParams::AcuteAngle { minimum } => IrRuleParams::AcuteAngle {
                min_angle_deg: *minimum,
            },
            RuleParams::SmdToCorner { distance } => IrRuleParams::SmdToCorner {
                clearance_mm: distance.to_mms(),
            },
            RuleParams::MaximumViaCount { max_via_count } => IrRuleParams::MaximumViaCount {
                max: *max_via_count,
            },
            RuleParams::MaxMinHoleSize { min, max } => IrRuleParams::MaxMinHoleSize {
                min_mm: min.to_mms(),
                max_mm: max.to_mms(),
            },
            RuleParams::Length { min, max } => IrRuleParams::Length {
                min_mm: min.to_mms(),
                max_mm: max.to_mms(),
            },
            RuleParams::DaisyChainStubLength { max_limit } => IrRuleParams::DaisyChainStubLength {
                max_mm: max_limit.to_mms(),
            },
            RuleParams::SmdNeckDown { .. } => IrRuleParams::SmdNeckDown,
            RuleParams::SmdEntry { .. } => IrRuleParams::SmdEntry,
            RuleParams::ParallelSegment {
                gap,
                parallel_length,
                ..
            } => IrRuleParams::ParallelSegment {
                max_run_mm: parallel_length.to_mms(),
                check_gap_mm: gap.to_mms(),
            },
            RuleParams::MinimumSolderMaskSliver { min_width } => {
                IrRuleParams::MinimumSolderMaskSliver {
                    min_mm: min_width.to_mms(),
                }
            }
            RuleParams::SilkToSolderMaskClearance { gap } => {
                IrRuleParams::SilkToSolderMaskClearance {
                    clearance_mm: gap.to_mms(),
                }
            }
            RuleParams::SilkToSilkClearance { gap } => IrRuleParams::SilkToSilkClearance {
                clearance_mm: gap.to_mms(),
            },
            _ => match r.kind {
                RuleKind::SilkToBoardRegionClearance => {
                    // altium-format parses SilkToBoardRegionClearance as EmptyRuleData because
                    // no dedicated IPCB_SilkToBoardRegionRule interface was found in the C# SDK
                    // and the gap parameter has not been verified via Ghidra or test fixtures.
                    // Until altium-format exposes RuleParams::SilkToBoardRegionClearance { gap },
                    // the clearance value cannot be extracted here and falls back to 0.0.
                    // See: docs/routing/rules6-audit.md § SilkToBoardRegionClearance
                    IrRuleParams::SilkToBoardRegionClearance { clearance_mm: 0.0 }
                }
                _ => IrRuleParams::Other { kind: r.kind },
            },
        };

        let id = rules.push(IrDesignRule {
            id: RuleId::from(0),
            name: r.name.clone(),
            kind: r.kind,
            priority: r.priority,
            enabled: r.enabled,
            params,
        });
        rules[id].id = id;
    }
    rules
}

// ---------------------------------------------------------------------------
// Free copper
// ---------------------------------------------------------------------------

fn extract_free_copper(
    board: &PcbDocBoard,
    net_lookup: &HashMap<String, NetId>,
    layer_lookup: &HashMap<String, LayerId>,
) -> Result<FreeCopperGeometry> {
    let mut tracks = Vec::new();
    for t in board.tracks.iter().filter(|t| t.component.is_none()) {
        let layer_name = t.layer.display_name().unwrap_or("Unknown").to_string();
        let layer = layer_lookup.get(&layer_name).copied().ok_or_else(|| {
            IrError::ExtractionError(format!(
                "track on unknown copper layer '{layer_name}'"
            ))
        })?;
        tracks.push(IrTrack {
            start: PointMm::from_coord_point(&t.start),
            end: PointMm::from_coord_point(&t.end),
            width_mm: t.width.to_mms(),
            layer_name,
            layer,
            net: t.net.as_ref().and_then(|n| net_lookup.get(n)).copied(),
            locked: false,
            pre_routed: false,
        });
    }

    let mut vias = Vec::new();
    for v in board.vias.iter().filter(|v| v.component.is_none()) {
        // Resolve from_layer / to_layer by display name, falling back to the
        // top and bottom copper layers when the layer name is not in the stack
        // (e.g. older files that record "MultiLayer" for through-hole vias).
        let from_layer_name = v.from_layer.display_name().unwrap_or("Unknown").to_string();
        let to_layer_name = v.to_layer.display_name().unwrap_or("Unknown").to_string();
        // TODO: older PcbDoc files may record via span layers that do not
        // appear in the copper layer stack (e.g. "Multi-Layer"). When that
        // happens we fall back to the first/last copper layer. Proper blind/buried
        // via layer mapping should be revisited once test fixtures are available.
        let from_layer = layer_lookup
            .get(&from_layer_name)
            .or_else(|| layer_lookup.values().next())
            .copied()
            .ok_or_else(|| {
                IrError::ExtractionError("board has no copper layers".into())
            })?;
        let to_layer = layer_lookup
            .get(&to_layer_name)
            .or_else(|| layer_lookup.values().last())
            .copied()
            .ok_or_else(|| {
                IrError::ExtractionError("board has no copper layers".into())
            })?;
        vias.push(IrVia {
            position: PointMm::from_coord_point(&v.location),
            diameter_mm: v.diameter.to_mms(),
            hole_size_mm: v.hole_size.to_mms(),
            net: v.net.as_ref().and_then(|n| net_lookup.get(n)).copied(),
            from_layer,
            to_layer,
            locked: false,
            pre_routed: false,
        });
    }

    let fills = board
        .fills
        .iter()
        .filter(|f| f.component.is_none())
        .map(|f| {
            let layer_name = f.layer.display_name().unwrap_or("Unknown").to_string();
            IrFill {
                corner1: PointMm::from_coord_point(&f.corner1),
                corner2: PointMm::from_coord_point(&f.corner2),
                layer_name,
                net: f.net.as_ref().and_then(|n| net_lookup.get(n)).copied(),
            }
        })
        .collect();

    Ok(FreeCopperGeometry {
        tracks,
        vias,
        fills,
    })
}

// ---------------------------------------------------------------------------
// Polygons
// ---------------------------------------------------------------------------

fn extract_polygons(
    board: &PcbDocBoard,
    net_lookup: &HashMap<String, NetId>,
) -> IdMap<PolygonId, IrPolygon> {
    let mut polygons = IdMap::new();
    for p in &board.polygons {
        let id = polygons.push(IrPolygon {
            id: PolygonId::from(0),
            name: p.name.clone(),
            net: p.net.as_ref().and_then(|n| net_lookup.get(n)).copied(),
            layer_name: p
                .layer
                .display_name()
                .unwrap_or("Unknown")
                .to_string(),
            vertices: p
                .vertices
                .iter()
                .map(|v| PointMm::from_coord_point(v))
                .collect(),
        });
        polygons[id].id = id;
    }
    polygons
}

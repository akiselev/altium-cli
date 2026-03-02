//! Extraction of `PcbIr` from an `altium_format::PcbDocBoard`.

use std::collections::HashMap;

use altium_format::api::{
    BoardContour, ContourSegment, PcbDocBoard, RuleParams,
};
use altium_format_types::pcb::V6Layer;

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
        let (net_lookup, mut nets) = extract_nets(board);
        let mut components = extract_components(board, &net_lookup)?;
        backfill_net_pins(&mut nets, &components);
        compute_component_bounds(&mut components);
        let rules = extract_rules(board);
        let free_copper = extract_free_copper(board, &net_lookup);
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
        });
        nets[id].id = id;
        lookup.insert(n.name.clone(), id);
    }
    (lookup, nets)
}

// ---------------------------------------------------------------------------
// Components + pads
// ---------------------------------------------------------------------------

fn extract_components(
    board: &PcbDocBoard,
    net_lookup: &HashMap<String, NetId>,
) -> Result<IdMap<ComponentId, IrComponent>> {
    let mut components = IdMap::with_capacity(board.components.len());
    let mut next_pad_id: u32 = 0;

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

fn extract_rules(board: &PcbDocBoard) -> IdMap<RuleId, IrDesignRule> {
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
            _ => IrRuleParams::Other { kind: r.kind },
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
) -> FreeCopperGeometry {
    let tracks = board
        .tracks
        .iter()
        .filter(|t| t.component.is_none())
        .map(|t| {
            let layer_name = t
                .layer
                .display_name()
                .unwrap_or("Unknown")
                .to_string();
            IrTrack {
                start: PointMm::from_coord_point(&t.start),
                end: PointMm::from_coord_point(&t.end),
                width_mm: t.width.to_mms(),
                layer_name,
                net: t.net.as_ref().and_then(|n| net_lookup.get(n)).copied(),
            }
        })
        .collect();

    let vias = board
        .vias
        .iter()
        .filter(|v| v.component.is_none())
        .map(|v| IrVia {
            position: PointMm::from_coord_point(&v.location),
            diameter_mm: v.diameter.to_mms(),
            hole_size_mm: v.hole_size.to_mms(),
            net: v.net.as_ref().and_then(|n| net_lookup.get(n)).copied(),
        })
        .collect();

    let fills = board
        .fills
        .iter()
        .filter(|f| f.component.is_none())
        .map(|f| {
            let layer_name = f
                .layer
                .display_name()
                .unwrap_or("Unknown")
                .to_string();
            IrFill {
                corner1: PointMm::from_coord_point(&f.corner1),
                corner2: PointMm::from_coord_point(&f.corner2),
                layer_name,
                net: f.net.as_ref().and_then(|n| net_lookup.get(n)).copied(),
            }
        })
        .collect();

    FreeCopperGeometry {
        tracks,
        vias,
        fills,
    }
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

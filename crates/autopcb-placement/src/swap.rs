//! Pin and part swap optimization for PCB placement.
//!
//! After the analytical solver and optional SA refinement have produced a placement,
//! this module applies greedy swap passes that improve HPWL by reassigning nets to
//! electrically equivalent pads (pin swap) or exchanging positions of functionally
//! identical components (part swap).
//!
//! # Data flow
//!
//! 1. `build_swap_model(ir)` — groups swappable pads and components using
//!    `swap_id_pin` / `swap_id_part` fields on `IrComponentPad`.
//! 2. `greedy_part_swap_pass` — run after Phase 2 (legalization).
//! 3. `greedy_pin_swap_sweep` — run after Phase 4 (final refinement).
//! 4. `verify_swap_integrity` — assert that net count and per-net pin counts
//!    are unchanged after all swaps.
//! 5. `write_swap_overlay` — emit a `.schdoc-spec` overlay listing the accepted swaps.

use std::collections::HashMap;

use autopcb_ir::{NetId, PcbIr};

use crate::PlacementResult;

// ---------------------------------------------------------------------------
// Error type

#[derive(Debug, thiserror::Error)]
pub enum SwapError {
    #[error("net count changed after swaps: before={before}, after={after}")]
    NetCountChanged { before: usize, after: usize },
    #[error("pin count for net '{net}' changed: before={before}, after={after}")]
    PinCountChanged {
        net: String,
        before: usize,
        after: usize,
    },
}

// ---------------------------------------------------------------------------
// Swap model

/// Groups of swappable pads and components derived from `IrComponentPad` swap ID fields.
pub struct SwapModel {
    /// `(component_idx, group_id)` → indices into `IrComponent::pads` that can swap.
    pub pin_swap_groups: HashMap<(usize, String), Vec<usize>>,
    /// `group_id` → component indices (into `ir.components` iteration order) with identical pinouts.
    pub part_swap_groups: HashMap<String, Vec<usize>>,
}

/// Build the swap model from the IR.  Groups with only one member are omitted because
/// a single-member group offers no swap opportunity.
pub fn build_swap_model(ir: &PcbIr) -> SwapModel {
    let mut pin_swap_groups: HashMap<(usize, String), Vec<usize>> = HashMap::new();
    let mut part_swap_groups: HashMap<String, Vec<usize>> = HashMap::new();

    for (comp_idx, (_comp_id, comp)) in ir.components.iter().enumerate() {
        // Pin swap groups: group pads within this component by swap_id_pin.
        for (pad_idx, pad) in comp.pads.iter().enumerate() {
            if let Some(group_id) = &pad.swap_id_pin {
                pin_swap_groups
                    .entry((comp_idx, group_id.clone()))
                    .or_default()
                    .push(pad_idx);
            }
        }

        // Part swap groups: group components by swap_id_part.
        // All pads on a component must agree on the same swap_id_part for this to be valid.
        let part_group = comp.pads.first().and_then(|p| p.swap_id_part.as_ref());
        if let Some(group_id) = part_group {
            let all_same = comp
                .pads
                .iter()
                .all(|p| p.swap_id_part.as_deref() == Some(group_id.as_str()));
            if all_same {
                part_swap_groups
                    .entry(group_id.clone())
                    .or_default()
                    .push(comp_idx);
            }
        }
    }

    // Remove groups with fewer than 2 members (no swap possible).
    pin_swap_groups.retain(|_, v| v.len() >= 2);
    part_swap_groups.retain(|_, v| v.len() >= 2);

    SwapModel {
        pin_swap_groups,
        part_swap_groups,
    }
}

// ---------------------------------------------------------------------------
// Changelog types

/// Records a single accepted pin swap.
#[derive(Debug, Clone)]
pub struct PinSwapEntry {
    /// Designator of the component whose pins were swapped.
    pub component: String,
    /// Name of the first pad.
    pub pin_a: String,
    /// Name of the second pad.
    pub pin_b: String,
    /// HPWL reduction (positive means improvement).
    pub hpwl_improvement: f64,
}

/// Records a single accepted part (component position) swap.
#[derive(Debug, Clone)]
pub struct PartSwapEntry {
    /// Designator of the first component.
    pub comp_a: String,
    /// Designator of the second component.
    pub comp_b: String,
    /// HPWL reduction (positive means improvement).
    pub hpwl_improvement: f64,
}

/// Summary of all accepted swaps from a placement run.
#[derive(Debug, Clone, Default)]
pub struct SwapChangelog {
    pub pin_swaps: Vec<PinSwapEntry>,
    pub part_swaps: Vec<PartSwapEntry>,
    pub total_hpwl_improvement: f64,
}

// ---------------------------------------------------------------------------
// HPWL computation

/// Net assignment overlay: maps (component_idx_in_ir, pad_idx) → NetId.
/// When `None`, the pad's net comes from the original IR.
type NetOverlay = HashMap<(usize, usize), Option<NetId>>;

/// Compute exact HPWL for all nets using component positions from `placement`,
/// pad positions from `ir`, and net assignments from `overlay` (fallback: IR).
pub fn compute_hpwl_with_overlay(
    placement: &PlacementResult,
    ir: &PcbIr,
    overlay: &NetOverlay,
) -> f64 {
    let pos_map: HashMap<&str, (f64, f64, f64)> = placement
        .components
        .iter()
        .map(|c| (c.designator.as_str(), (c.x_mm, c.y_mm, c.rotation_deg)))
        .collect();

    // Build net → Vec<world_point> mapping.
    let n_nets = ir.nets.len();
    let mut net_min_x: Vec<f64> = vec![f64::INFINITY; n_nets];
    let mut net_max_x: Vec<f64> = vec![f64::NEG_INFINITY; n_nets];
    let mut net_min_y: Vec<f64> = vec![f64::INFINITY; n_nets];
    let mut net_max_y: Vec<f64> = vec![f64::NEG_INFINITY; n_nets];
    let mut net_pin_count: Vec<usize> = vec![0; n_nets];

    for (comp_ir_idx, (_comp_id, comp)) in ir.components.iter().enumerate() {
        let (cx, cy, rot_deg) = match pos_map.get(comp.designator.as_str()) {
            Some(&p) => p,
            None => continue,
        };
        let (sin_t, cos_t) = rot_deg.to_radians().sin_cos();

        for (pad_idx, pad) in comp.pads.iter().enumerate() {
            // Resolve net: overlay takes priority over IR.
            let net_id: Option<NetId> = overlay
                .get(&(comp_ir_idx, pad_idx))
                .copied()
                .unwrap_or(pad.net);

            let net_idx = match net_id {
                Some(nid) => nid.raw() as usize,
                None => continue,
            };
            if net_idx >= n_nets {
                continue;
            }

            let lx = pad.local_position.x;
            let ly = pad.local_position.y;
            let wx = cx + lx * cos_t - ly * sin_t;
            let wy = cy + lx * sin_t + ly * cos_t;

            net_min_x[net_idx] = net_min_x[net_idx].min(wx);
            net_max_x[net_idx] = net_max_x[net_idx].max(wx);
            net_min_y[net_idx] = net_min_y[net_idx].min(wy);
            net_max_y[net_idx] = net_max_y[net_idx].max(wy);
            net_pin_count[net_idx] += 1;
        }
    }

    let mut total = 0.0;
    for ni in 0..n_nets {
        if net_pin_count[ni] >= 2 {
            total += (net_max_x[ni] - net_min_x[ni]) + (net_max_y[ni] - net_min_y[ni]);
        }
    }
    total
}

/// Compute HPWL with no overlay (uses IR net assignments directly).
pub fn compute_hpwl(placement: &PlacementResult, ir: &PcbIr) -> f64 {
    compute_hpwl_with_overlay(placement, ir, &HashMap::new())
}

// ---------------------------------------------------------------------------
// Pin swap

/// Try all pairwise pin swaps within each swap group.  Accept a swap if it
/// strictly reduces HPWL.  Repeat until no more improvements are possible.
///
/// Pin swapping exchanges the net assignments of two electrically equivalent pads.
/// The net assignments are tracked in an overlay (not modifying the IR directly),
/// so the IR remains unchanged.  The overlay is returned as part of the changelog
/// (via the accepted swap list) for external application.
pub fn greedy_pin_swap_sweep(
    placement: &mut PlacementResult,
    ir: &PcbIr,
    model: &SwapModel,
) -> SwapChangelog {
    let mut changelog = SwapChangelog::default();

    // Build a mutable net assignment overlay starting from IR defaults.
    let mut overlay: NetOverlay = HashMap::new();

    // Map designator → IR iteration index.
    let desig_to_ir_idx: HashMap<&str, usize> = ir
        .components
        .iter()
        .enumerate()
        .map(|(i, (_, c))| (c.designator.as_str(), i))
        .collect();

    let mut improved = true;
    while improved {
        improved = false;

        for ((comp_ir_idx, group_id), pad_indices) in &model.pin_swap_groups {
            let n = pad_indices.len();
            for i in 0..n {
                for j in (i + 1)..n {
                    let pi = pad_indices[i];
                    let pj = pad_indices[j];

                    // Resolve current net assignments via overlay.
                    let comp = match ir.components.iter().nth(*comp_ir_idx) {
                        Some((_, c)) => c,
                        None => continue,
                    };
                    let net_i = overlay
                        .get(&(*comp_ir_idx, pi))
                        .copied()
                        .unwrap_or(comp.pads[pi].net);
                    let net_j = overlay
                        .get(&(*comp_ir_idx, pj))
                        .copied()
                        .unwrap_or(comp.pads[pj].net);

                    if net_i == net_j {
                        continue;
                    }

                    let before_hpwl = compute_hpwl_with_overlay(placement, ir, &overlay);

                    // Apply swap in overlay.
                    overlay.insert((*comp_ir_idx, pi), net_j);
                    overlay.insert((*comp_ir_idx, pj), net_i);

                    let after_hpwl = compute_hpwl_with_overlay(placement, ir, &overlay);

                    if after_hpwl < before_hpwl - 1e-9 {
                        let improvement = before_hpwl - after_hpwl;
                        changelog.total_hpwl_improvement += improvement;
                        changelog.pin_swaps.push(PinSwapEntry {
                            component: comp.designator.clone(),
                            pin_a: comp.pads[pi].name.clone(),
                            pin_b: comp.pads[pj].name.clone(),
                            hpwl_improvement: improvement,
                        });
                        improved = true;
                    } else {
                        // Revert overlay.
                        overlay.insert((*comp_ir_idx, pi), net_i);
                        overlay.insert((*comp_ir_idx, pj), net_j);
                    }
                }
            }

            let _ = (group_id, desig_to_ir_idx.len()); // suppress unused warnings
        }
    }

    // Update the HPWL estimate in the placement result.
    if changelog.total_hpwl_improvement > 0.0 {
        placement.hpwl_estimate_mm = compute_hpwl_with_overlay(placement, ir, &overlay);
    }

    changelog
}

// ---------------------------------------------------------------------------
// Part swap

/// Try all pairwise component position swaps within each part swap group.  Accept a
/// swap if it strictly reduces HPWL.
pub fn greedy_part_swap_pass(
    placement: &mut PlacementResult,
    ir: &PcbIr,
    model: &SwapModel,
) -> SwapChangelog {
    let mut changelog = SwapChangelog::default();

    let comp_designators: Vec<String> = ir
        .components
        .iter()
        .map(|(_, c)| c.designator.clone())
        .collect();

    let mut improved = true;
    while improved {
        improved = false;

        for (_group_id, comp_indices) in &model.part_swap_groups {
            let n = comp_indices.len();
            for i in 0..n {
                for j in (i + 1)..n {
                    let idx_a = comp_indices[i];
                    let idx_b = comp_indices[j];

                    let desig_a = match comp_designators.get(idx_a) {
                        Some(d) => d.clone(),
                        None => continue,
                    };
                    let desig_b = match comp_designators.get(idx_b) {
                        Some(d) => d.clone(),
                        None => continue,
                    };

                    let before_hpwl = compute_hpwl(placement, ir);

                    let pos_a = match placement
                        .components
                        .iter()
                        .find(|c| c.designator == desig_a)
                    {
                        Some(c) => (c.x_mm, c.y_mm, c.rotation_deg),
                        None => continue,
                    };
                    let pos_b = match placement
                        .components
                        .iter()
                        .find(|c| c.designator == desig_b)
                    {
                        Some(c) => (c.x_mm, c.y_mm, c.rotation_deg),
                        None => continue,
                    };

                    apply_part_swap(placement, &desig_a, &desig_b, pos_a, pos_b);

                    let after_hpwl = compute_hpwl(placement, ir);

                    if after_hpwl < before_hpwl - 1e-9 {
                        let improvement = before_hpwl - after_hpwl;
                        changelog.total_hpwl_improvement += improvement;
                        changelog.part_swaps.push(PartSwapEntry {
                            comp_a: desig_a,
                            comp_b: desig_b,
                            hpwl_improvement: improvement,
                        });
                        improved = true;
                    } else {
                        // Revert: swap back (original pos_a goes back to desig_a).
                        apply_part_swap(placement, &desig_a, &desig_b, pos_b, pos_a);
                    }
                }
            }
        }
    }

    changelog
}

/// Exchange the positions of two components in the placement result.
fn apply_part_swap(
    placement: &mut PlacementResult,
    desig_a: &str,
    desig_b: &str,
    new_pos_a: (f64, f64, f64),
    new_pos_b: (f64, f64, f64),
) {
    for c in &mut placement.components {
        if c.designator == desig_a {
            c.x_mm = new_pos_a.0;
            c.y_mm = new_pos_a.1;
            c.rotation_deg = new_pos_a.2;
        } else if c.designator == desig_b {
            c.x_mm = new_pos_b.0;
            c.y_mm = new_pos_b.1;
            c.rotation_deg = new_pos_b.2;
        }
    }
}

// ---------------------------------------------------------------------------
// Integrity verification

/// Verify that swap operations have preserved net topology.
///
/// `overlay` is the final `NetOverlay` produced by `greedy_pin_swap_sweep`.
/// Pass `&HashMap::new()` when no pin swaps were performed.
pub fn verify_swap_integrity(
    ir: &PcbIr,
    overlay: &NetOverlay,
    before_net_pin_counts: &HashMap<String, usize>,
) -> Result<(), SwapError> {
    let after_net_pin_counts = collect_net_pin_counts_with_overlay(ir, overlay);

    if before_net_pin_counts.len() != after_net_pin_counts.len() {
        return Err(SwapError::NetCountChanged {
            before: before_net_pin_counts.len(),
            after: after_net_pin_counts.len(),
        });
    }

    for (net_name, &before_count) in before_net_pin_counts {
        let after_count = after_net_pin_counts.get(net_name).copied().unwrap_or(0);
        if before_count != after_count {
            return Err(SwapError::PinCountChanged {
                net: net_name.clone(),
                before: before_count,
                after: after_count,
            });
        }
    }

    Ok(())
}

/// Collect a snapshot of (net_name → pin_count) using overlay-resolved net assignments.
///
/// For each pad, the overlay takes priority over the pad's original net from the IR.
/// Pads with no assigned net (neither in overlay nor in IR) are excluded from counts.
pub fn collect_net_pin_counts_with_overlay(
    ir: &PcbIr,
    overlay: &NetOverlay,
) -> HashMap<String, usize> {
    let net_names: HashMap<usize, &str> = ir
        .nets
        .iter()
        .map(|(_, net)| (net.id.raw() as usize, net.name.as_str()))
        .collect();

    let mut counts: HashMap<String, usize> = ir
        .nets
        .iter()
        .map(|(_, net)| (net.name.clone(), 0))
        .collect();

    for (comp_ir_idx, (_comp_id, comp)) in ir.components.iter().enumerate() {
        for (pad_idx, pad) in comp.pads.iter().enumerate() {
            let net_id: Option<autopcb_ir::NetId> = overlay
                .get(&(comp_ir_idx, pad_idx))
                .copied()
                .unwrap_or(pad.net);

            if let Some(nid) = net_id {
                if let Some(&name) = net_names.get(&(nid.raw() as usize)) {
                    *counts.entry(name.to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    counts
}

/// Collect a snapshot of (net_name → pin_count) from the current IR state.
pub fn collect_net_pin_counts(ir: &PcbIr) -> HashMap<String, usize> {
    ir.nets
        .iter()
        .map(|(_, net)| (net.name.clone(), net.pins.len()))
        .collect()
}

// ---------------------------------------------------------------------------
// Swap overlay generation

/// Generate a `.schdoc-spec` overlay text that documents all accepted swaps.
pub fn write_swap_overlay(changelog: &SwapChangelog) -> String {
    let mut out = String::new();

    out.push_str("// AutoPCB pin/part swap overlay\n");
    out.push_str("// Generated automatically — delete this file to undo all swaps.\n");
    out.push_str("//\n");
    out.push_str("// Pin swaps: electrically equivalent pads reordered to reduce wire length.\n");
    out.push_str("// Part swaps: identical components exchanged to reduce wire length.\n");
    out.push('\n');

    if changelog.pin_swaps.is_empty() && changelog.part_swaps.is_empty() {
        out.push_str("// No swaps were accepted.\n");
        return out;
    }

    if !changelog.pin_swaps.is_empty() {
        out.push_str("// Pin swaps\n");
        out.push_str("pin_swaps {\n");
        for entry in &changelog.pin_swaps {
            out.push_str(&format!(
                "    // {}: swap pin {} <-> {} (HPWL improvement: {:.4}mm)\n",
                entry.component, entry.pin_a, entry.pin_b, entry.hpwl_improvement
            ));
            out.push_str(&format!(
                "    swap_pins {{ component: {}, pin_a: {}, pin_b: {} }}\n",
                entry.component, entry.pin_a, entry.pin_b
            ));
        }
        out.push_str("}\n\n");
    }

    if !changelog.part_swaps.is_empty() {
        out.push_str("// Part swaps\n");
        out.push_str("part_swaps {\n");
        for entry in &changelog.part_swaps {
            out.push_str(&format!(
                "    // swap {} <-> {} (HPWL improvement: {:.4}mm)\n",
                entry.comp_a, entry.comp_b, entry.hpwl_improvement
            ));
            out.push_str(&format!(
                "    swap_parts {{ comp_a: {}, comp_b: {} }}\n",
                entry.comp_a, entry.comp_b
            ));
        }
        out.push_str("}\n\n");
    }

    out.push_str(&format!(
        "// Total HPWL improvement: {:.4}mm\n",
        changelog.total_hpwl_improvement
    ));

    out
}

// ---------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PlacementComponentState, PlacementResult};
    use autopcb_ir::component::{PadShapeInfo, PadShapeKind};
    use autopcb_ir::types::{BoardSide, BoundingBoxMm, PointMm};
    use autopcb_ir::{
        FreeCopperGeometry, IrBoardGeometry, IrComponent, IrComponentPad, IrLayerStack, IrNet,
        IrNetPin,
        extract::PcbIr,
        handles::{ComponentId, IdMap, NetId, PadId},
    };

    fn make_point(x: f64, y: f64) -> PointMm {
        PointMm::new(x, y)
    }

    fn make_bb(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> BoundingBoxMm {
        BoundingBoxMm::new(make_point(min_x, min_y), make_point(max_x, max_y))
    }

    fn default_shape() -> PadShapeInfo {
        PadShapeInfo {
            kind: PadShapeKind::Round,
            size_x: 1.0,
            size_y: 1.0,
            rotation: 0.0,
        }
    }

    fn make_pad(
        id: u32,
        name: &str,
        lx: f64,
        ly: f64,
        net: Option<NetId>,
        swap_pin: Option<&str>,
        swap_part: Option<&str>,
    ) -> IrComponentPad {
        IrComponentPad {
            id: PadId::from(id),
            name: name.to_string(),
            local_position: make_point(lx, ly),
            world_position: make_point(lx, ly),
            net,
            shape: default_shape(),
            is_through_hole: false,
            hole_size_mm: 0.0,
            swap_id_pin: swap_pin.map(|s| s.to_string()),
            swap_id_part: swap_part.map(|s| s.to_string()),
            layer_set: Vec::new(),
        }
    }

    fn make_placement_result(comps: Vec<(&str, f64, f64)>) -> PlacementResult {
        PlacementResult {
            status: "Solved".to_string(),
            total_iterations: 0,
            duration_ms: 0,
            components: comps
                .into_iter()
                .map(|(d, x, y)| PlacementComponentState {
                    designator: d.to_string(),
                    x_mm: x,
                    y_mm: y,
                    rotation_deg: 0.0,
                })
                .collect(),
            snapshots: Vec::new(),
            hpwl_estimate_mm: 0.0,
            overlap_violations: 0,
        }
    }

    fn build_minimal_ir() -> PcbIr {
        let mut nets: IdMap<NetId, IrNet> = IdMap::new();
        let net_a = nets.push(IrNet {
            id: NetId::from(0),
            name: "NET_A".to_string(),
            pins: Vec::new(),
            component_count: 0,
            net_class: None,
            diff_pair_partner: None,
        });
        nets[net_a].id = net_a;

        let net_b = nets.push(IrNet {
            id: NetId::from(0),
            name: "NET_B".to_string(),
            pins: Vec::new(),
            component_count: 0,
            net_class: None,
            diff_pair_partner: None,
        });
        nets[net_b].id = net_b;

        let mut components: IdMap<ComponentId, IrComponent> = IdMap::new();

        let r1_id = components.push(IrComponent {
            id: ComponentId::from(0),
            designator: "R1".to_string(),
            pattern: "0402".to_string(),
            value: "10k".to_string(),
            position: make_point(10.0, 10.0),
            rotation: 0.0,
            side: BoardSide::Top,
            local_bounds: make_bb(-1.5, -0.5, 1.5, 0.5),
            world_bounds: make_bb(8.5, 9.5, 11.5, 10.5),
            pads: vec![
                make_pad(0, "1", -1.0, 0.0, Some(net_a), Some("G1"), Some("RG")),
                make_pad(1, "2", 1.0, 0.0, Some(net_b), Some("G1"), Some("RG")),
            ],
        });
        components[r1_id].id = r1_id;

        let r2_id = components.push(IrComponent {
            id: ComponentId::from(0),
            designator: "R2".to_string(),
            pattern: "0402".to_string(),
            value: "10k".to_string(),
            position: make_point(10.0, 50.0),
            rotation: 0.0,
            side: BoardSide::Top,
            local_bounds: make_bb(-1.5, -0.5, 1.5, 0.5),
            world_bounds: make_bb(8.5, 49.5, 11.5, 50.5),
            pads: vec![
                make_pad(2, "1", -1.0, 0.0, Some(net_a), Some("G1"), Some("RG")),
                make_pad(3, "2", 1.0, 0.0, Some(net_b), Some("G1"), Some("RG")),
            ],
        });
        components[r2_id].id = r2_id;

        nets[net_a].pins = vec![
            IrNetPin {
                pad: PadId::from(0),
                component: r1_id,
                position: make_point(9.0, 10.0),
            },
            IrNetPin {
                pad: PadId::from(2),
                component: r2_id,
                position: make_point(9.0, 50.0),
            },
        ];
        nets[net_b].pins = vec![
            IrNetPin {
                pad: PadId::from(1),
                component: r1_id,
                position: make_point(11.0, 10.0),
            },
            IrNetPin {
                pad: PadId::from(3),
                component: r2_id,
                position: make_point(11.0, 50.0),
            },
        ];
        nets[net_a].component_count = 2;
        nets[net_b].component_count = 2;

        PcbIr {
            board: IrBoardGeometry {
                outline: vec![
                    make_point(0.0, 0.0),
                    make_point(100.0, 0.0),
                    make_point(100.0, 100.0),
                    make_point(0.0, 100.0),
                ],
                cutouts: Vec::new(),
                bounds: make_bb(0.0, 0.0, 100.0, 100.0),
                keepouts: Vec::new(),
            },
            layer_stack: IrLayerStack {
                copper_layers: Vec::new(),
                copper_layer_count: 2,
            },
            components,
            nets,
            rules: IdMap::new(),
            free_copper: FreeCopperGeometry {
                tracks: Vec::new(),
                arcs: Vec::new(),
                vias: Vec::new(),
                fills: Vec::new(),
            },
            polygons: IdMap::new(),
            texts: IdMap::new(),
            regions: IdMap::new(),
            component_bodies: IdMap::new(),
        }
    }

    #[test]
    fn test_build_swap_model_with_known_groups() {
        let ir = build_minimal_ir();
        let model = build_swap_model(&ir);

        // Each component has one pin swap group "G1" with 2 pads.
        assert_eq!(
            model.pin_swap_groups.len(),
            2,
            "R1 and R2 each have one group"
        );

        // Both components are in part swap group "RG".
        assert_eq!(model.part_swap_groups.len(), 1);
        let rg = model.part_swap_groups.get("RG").expect("RG part group");
        assert_eq!(rg.len(), 2);
    }

    #[test]
    fn test_single_pad_group_excluded() {
        let mut nets: IdMap<NetId, IrNet> = IdMap::new();
        let net_a = nets.push(IrNet {
            id: NetId::from(0),
            name: "A".to_string(),
            pins: Vec::new(),
            component_count: 0,
            net_class: None,
            diff_pair_partner: None,
        });
        nets[net_a].id = net_a;

        let mut components: IdMap<ComponentId, IrComponent> = IdMap::new();
        let c_id = components.push(IrComponent {
            id: ComponentId::from(0),
            designator: "U1".to_string(),
            pattern: "SOT23".to_string(),
            value: "".to_string(),
            position: make_point(0.0, 0.0),
            rotation: 0.0,
            side: BoardSide::Top,
            local_bounds: make_bb(-1.0, -1.0, 1.0, 1.0),
            world_bounds: make_bb(-1.0, -1.0, 1.0, 1.0),
            pads: vec![make_pad(0, "1", 0.0, 0.0, Some(net_a), Some("SOLO"), None)],
        });
        components[c_id].id = c_id;

        let ir = PcbIr {
            board: IrBoardGeometry {
                outline: vec![
                    make_point(0.0, 0.0),
                    make_point(10.0, 0.0),
                    make_point(10.0, 10.0),
                    make_point(0.0, 10.0),
                ],
                cutouts: Vec::new(),
                bounds: make_bb(0.0, 0.0, 10.0, 10.0),
                keepouts: Vec::new(),
            },
            layer_stack: IrLayerStack {
                copper_layers: Vec::new(),
                copper_layer_count: 2,
            },
            components,
            nets,
            rules: IdMap::new(),
            free_copper: FreeCopperGeometry {
                tracks: Vec::new(),
                arcs: Vec::new(),
                vias: Vec::new(),
                fills: Vec::new(),
            },
            polygons: IdMap::new(),
            texts: IdMap::new(),
            regions: IdMap::new(),
            component_bodies: IdMap::new(),
        };

        let model = build_swap_model(&ir);
        assert!(
            model.pin_swap_groups.is_empty(),
            "single-pad group must be excluded"
        );
        assert!(
            model.part_swap_groups.is_empty(),
            "single-component part group must be excluded"
        );
    }

    #[test]
    fn test_part_swap_runs_without_panic() {
        let ir = build_minimal_ir();
        let mut placement = make_placement_result(vec![("R1", 10.0, 10.0), ("R2", 10.0, 50.0)]);
        let model = build_swap_model(&ir);

        let before = compute_hpwl(&placement, &ir);
        let changelog = greedy_part_swap_pass(&mut placement, &ir, &model);

        if !changelog.part_swaps.is_empty() {
            let after = compute_hpwl(&placement, &ir);
            assert!(after <= before, "part swap must not increase HPWL");
        }
    }

    #[test]
    fn test_pin_swap_runs_without_panic() {
        let ir = build_minimal_ir();
        let mut placement = make_placement_result(vec![("R1", 10.0, 10.0), ("R2", 10.0, 50.0)]);
        let model = build_swap_model(&ir);

        let before = compute_hpwl(&placement, &ir);
        let changelog = greedy_pin_swap_sweep(&mut placement, &ir, &model);

        if !changelog.pin_swaps.is_empty() {
            assert!(
                placement.hpwl_estimate_mm <= before,
                "pin swap must not increase HPWL"
            );
        }
    }

    #[test]
    fn test_verify_swap_integrity_passes_after_no_swaps() {
        let ir = build_minimal_ir();
        let before = collect_net_pin_counts(&ir);
        verify_swap_integrity(&ir, &HashMap::new(), &before)
            .expect("integrity check must pass when nothing changed");
    }

    #[test]
    fn test_verify_swap_integrity_detects_pin_count_change() {
        let ir = build_minimal_ir();
        let mut before = collect_net_pin_counts(&ir);
        before.insert("NET_A".to_string(), 3);
        let err = verify_swap_integrity(&ir, &HashMap::new(), &before)
            .expect_err("should detect pin count change");
        assert!(matches!(err, SwapError::PinCountChanged { .. }));
    }

    #[test]
    fn test_write_swap_overlay_empty() {
        let changelog = SwapChangelog::default();
        let overlay = write_swap_overlay(&changelog);
        assert!(overlay.contains("No swaps were accepted"));
    }

    #[test]
    fn test_write_swap_overlay_with_entries() {
        let changelog = SwapChangelog {
            pin_swaps: vec![PinSwapEntry {
                component: "R1".to_string(),
                pin_a: "1".to_string(),
                pin_b: "2".to_string(),
                hpwl_improvement: 3.5,
            }],
            part_swaps: vec![PartSwapEntry {
                comp_a: "R1".to_string(),
                comp_b: "R2".to_string(),
                hpwl_improvement: 7.2,
            }],
            total_hpwl_improvement: 10.7,
        };
        let overlay = write_swap_overlay(&changelog);
        assert!(overlay.contains("swap_pins"));
        assert!(overlay.contains("swap_parts"));
        assert!(overlay.contains("10.7000"));
    }

    #[test]
    fn test_collect_net_pin_counts() {
        let ir = build_minimal_ir();
        let counts = collect_net_pin_counts(&ir);
        assert_eq!(counts.get("NET_A"), Some(&2));
        assert_eq!(counts.get("NET_B"), Some(&2));
    }
}

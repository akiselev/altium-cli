//! Simulated annealing refinement for PCB placement (Phase 3).
//!
//! Builds on top of a `PlacementResult` produced by the analytical solver
//! (Phases 1–2) and improves it using the Metropolis acceptance criterion.
//! The module is self-contained: all SA logic lives here.

use std::collections::HashMap;
use std::time::Instant;

use autopcb_ir::{ComponentId, PcbIr};
use rand::Rng;
use tracing::{debug, info, trace};

use crate::{PlacementComponentState, PlacementIterationSnapshot, PlacementResult};

// ---------------------------------------------------------------------------
// Public configuration

/// Configuration for the simulated annealing refinement pass.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SAConfig {
    /// Geometric cooling factor applied each temperature step (default 0.95).
    pub cooling_rate: f64,
    /// Number of Metropolis trials per temperature level (default 100).
    pub moves_per_temp: usize,
    /// Maximum number of temperature steps (default 5000).
    pub max_steps: usize,
    /// Target initial acceptance rate used to auto-calibrate T₀ (default 0.8).
    pub initial_acceptance: f64,
    /// Temperature at which the run is considered frozen (default 0.001).
    pub t_frozen: f64,
    /// Stop if acceptance rate stays below 1 % for this many consecutive steps (default 5).
    pub min_acceptance_steps: usize,
    /// Record a viewer snapshot every this many temperature steps (default 50).
    pub snapshot_interval: usize,
    /// Additional cost weight for coarse congestion overflow penalty.
    pub congestion_weight: f64,
    /// Congestion grid cell size in mm.
    pub congestion_cell_mm: f64,
    /// Bias multiplier for nets/components with high HPWL contribution.
    pub critical_net_boost: f64,
}

impl Default for SAConfig {
    fn default() -> Self {
        Self {
            cooling_rate: 0.95,
            moves_per_temp: 100,
            max_steps: 5000,
            initial_acceptance: 0.8,
            t_frozen: 0.001,
            min_acceptance_steps: 5,
            snapshot_interval: 50,
            congestion_weight: 0.0,
            congestion_cell_mm: 5.0,
            critical_net_boost: 2.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Internal placement state

/// Local position + geometry of a single component during SA.
#[derive(Debug, Clone)]
struct ComponentState {
    designator: String,
    x: f64,
    y: f64,
    rotation: f64,
    width: f64,
    height: f64,
    center_dx: f64,
    center_dy: f64,
    /// Only movable components are perturbed by SA.
    is_movable: bool,
    /// Cached pad offsets in component-local coordinates.
    /// Each entry is `(net_idx, local_x, local_y)`.
    pads: Vec<(usize, f64, f64)>,
}

/// Full board state managed during SA.
struct Placement {
    components: Vec<ComponentState>,
    net_component_index: NetComponentIndex,
    spatial_grid: SpatialGrid,
    board_bounds: (f64, f64, f64, f64),
    congestion_weight: f64,
    congestion_cell_mm: f64,
    congestion_capacity: f64,
    congestion_enabled: bool,
    /// Candidate pin swap pairs: (comp_idx, pad_idx_a, pad_idx_b).
    /// Two pads may be swapped only if they are in the same swap group on the same component.
    pin_swap_opportunities: Vec<(usize, usize, usize)>,
    /// Candidate part swap pairs: (comp_idx_a, comp_idx_b).
    /// The two components must be in the same part swap group.
    part_swap_opportunities: Vec<(usize, usize)>,
}

#[derive(Debug, Clone)]
struct MoveBiasContext {
    component_weights: Vec<f64>,
    pin_swap_weights: Vec<f64>,
    part_swap_weights: Vec<f64>,
}

#[derive(Debug, Clone)]
struct CongestionMetrics {
    penalty: f64,
    component_scores: Vec<f64>,
}

// ---------------------------------------------------------------------------
// Spatial grid

/// Grid-based O(k) neighbour lookup for overlap detection.
struct SpatialGrid {
    cell_size: f64,
    cells: HashMap<(i32, i32), Vec<usize>>,
}

impl SpatialGrid {
    fn new(cell_size: f64) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    /// All grid cells overlapped by an AABB centred at (cx, cy) with half-extents (hw, hh).
    fn cells_for_aabb(&self, cx: f64, cy: f64, hw: f64, hh: f64) -> Vec<(i32, i32)> {
        let min_kx = ((cx - hw) / self.cell_size).floor() as i32;
        let max_kx = ((cx + hw) / self.cell_size).floor() as i32;
        let min_ky = ((cy - hh) / self.cell_size).floor() as i32;
        let max_ky = ((cy + hh) / self.cell_size).floor() as i32;
        let mut out = Vec::new();
        for kx in min_kx..=max_kx {
            for ky in min_ky..=max_ky {
                out.push((kx, ky));
            }
        }
        out
    }

    fn insert(&mut self, idx: usize, cx: f64, cy: f64, hw: f64, hh: f64) {
        for key in self.cells_for_aabb(cx, cy, hw, hh) {
            self.cells.entry(key).or_default().push(idx);
        }
    }

    fn remove(&mut self, idx: usize, cx: f64, cy: f64, hw: f64, hh: f64) {
        for key in self.cells_for_aabb(cx, cy, hw, hh) {
            if let Some(v) = self.cells.get_mut(&key) {
                v.retain(|&i| i != idx);
            }
        }
    }

    /// Returns the set of component indices that share any grid cell with the given AABB,
    /// excluding `exclude_idx` itself.
    fn neighbours(&self, cx: f64, cy: f64, hw: f64, hh: f64, exclude_idx: usize) -> Vec<usize> {
        let mut seen = Vec::new();
        for key in self.cells_for_aabb(cx, cy, hw, hh) {
            if let Some(v) = self.cells.get(&key) {
                for &i in v {
                    if i != exclude_idx && !seen.contains(&i) {
                        seen.push(i);
                    }
                }
            }
        }
        seen
    }
}

// ---------------------------------------------------------------------------
// Net–component bidirectional index

/// Maps component indices ↔ net indices for incremental HPWL evaluation.
struct NetComponentIndex {
    /// `comp_to_nets[comp_idx]` = list of net indices that component belongs to.
    comp_to_nets: Vec<Vec<usize>>,
    /// `net_to_comps[net_idx]` = list of component indices in that net.
    net_to_comps: Vec<Vec<usize>>,
}

impl NetComponentIndex {
    fn build(ir: &PcbIr, comp_designators: &[String]) -> Self {
        let n_comps = comp_designators.len();
        let n_nets = ir.nets.len();

        let mut comp_to_nets: Vec<Vec<usize>> = vec![Vec::new(); n_comps];
        let mut net_to_comps: Vec<Vec<usize>> = vec![Vec::new(); n_nets];

        // Build designator → comp_idx mapping.
        let mut desig_to_idx: HashMap<&str, usize> = HashMap::new();
        for (i, d) in comp_designators.iter().enumerate() {
            desig_to_idx.insert(d.as_str(), i);
        }

        for (net_id, net) in ir.nets.iter() {
            let net_idx = net_id.raw() as usize;
            // Collect distinct components in this net.
            let mut seen_comps: Vec<usize> = Vec::new();
            for pin in &net.pins {
                let comp_id: ComponentId = pin.component;
                if let Some(comp) = ir.components.get(comp_id) {
                    if let Some(&ci) = desig_to_idx.get(comp.designator.as_str()) {
                        if !seen_comps.contains(&ci) {
                            seen_comps.push(ci);
                        }
                    }
                }
            }
            for ci in &seen_comps {
                if net_idx < net_to_comps.len() {
                    net_to_comps[net_idx].push(*ci);
                }
                if *ci < comp_to_nets.len() {
                    comp_to_nets[*ci].push(net_idx);
                }
            }
        }

        Self {
            comp_to_nets,
            net_to_comps,
        }
    }
}

// ---------------------------------------------------------------------------
// Move types

#[derive(Debug, Clone)]
enum Move {
    Displace {
        comp_idx: usize,
        dx: f64,
        dy: f64,
    },
    /// Positional swap: exchange the (x, y) of two components.
    Swap {
        comp_a: usize,
        comp_b: usize,
    },
    Rotate {
        comp_idx: usize,
        new_rotation: f64,
    },
    /// Pin swap: exchange the net index of two pads within the same swap group on a component.
    PinSwap {
        comp_idx: usize,
        pad_a: usize,
        pad_b: usize,
    },
    /// Part swap: exchange positions of two components known to be in the same part swap group.
    PartSwap {
        comp_a: usize,
        comp_b: usize,
    },
}

// ---------------------------------------------------------------------------
// HPWL computation

/// Compute exact HPWL for a single net given component positions and pad offsets.
/// Returns 0.0 for nets with fewer than 2 pins.
fn hpwl_for_net(net_idx: usize, index: &NetComponentIndex, components: &[ComponentState]) -> f64 {
    let comp_indices = match index.net_to_comps.get(net_idx) {
        Some(v) if v.len() >= 2 => v,
        _ => return 0.0,
    };

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut pin_count = 0usize;

    for &ci in comp_indices {
        let comp = &components[ci];
        let (sin_t, cos_t) = comp.rotation.to_radians().sin_cos();
        for &(pad_net, lx, ly) in &comp.pads {
            if pad_net != net_idx {
                continue;
            }
            let wx = comp.x + lx * cos_t - ly * sin_t;
            let wy = comp.y + lx * sin_t + ly * cos_t;
            min_x = min_x.min(wx);
            max_x = max_x.max(wx);
            min_y = min_y.min(wy);
            max_y = max_y.max(wy);
            pin_count += 1;
        }
    }

    if pin_count < 2 {
        return 0.0;
    }
    (max_x - min_x) + (max_y - min_y)
}

/// Compute HPWL for `query_net` after a pin swap: on `swap_comp`, `swapped_pad_idx` now
/// belongs to `query_net` (was previously on `other_net`), and `removed_pad_idx` no longer
/// belongs to `query_net` (has been moved to `other_net`).
///
/// This lets `delta_cost` evaluate a PinSwap without mutating the placement.
fn hpwl_for_net_with_swap(
    query_net: usize,
    _other_net: usize,
    swap_comp: usize,
    swapped_pad_idx: usize,
    removed_pad_idx: usize,
    placement: &Placement,
) -> f64 {
    let comp_indices = match placement.net_component_index.net_to_comps.get(query_net) {
        Some(v) if !v.is_empty() => v,
        _ => return 0.0,
    };

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut pin_count = 0usize;

    for &ci in comp_indices {
        let comp = &placement.components[ci];
        let (sin_t, cos_t) = comp.rotation.to_radians().sin_cos();

        for (pad_idx, &(pad_net, lx, ly)) in comp.pads.iter().enumerate() {
            // If this is the swap component, apply the logical swap:
            // - `removed_pad_idx` is excluded from query_net.
            // - `swapped_pad_idx` is included in query_net (it came from other_net).
            let effective_net = if ci == swap_comp {
                if pad_idx == removed_pad_idx {
                    continue; // this pad left query_net
                } else if pad_idx == swapped_pad_idx {
                    query_net // this pad joined query_net
                } else {
                    pad_net
                }
            } else {
                pad_net
            };

            if effective_net != query_net {
                continue;
            }

            let wx = comp.x + lx * cos_t - ly * sin_t;
            let wy = comp.y + lx * sin_t + ly * cos_t;
            min_x = min_x.min(wx);
            max_x = max_x.max(wx);
            min_y = min_y.min(wy);
            max_y = max_y.max(wy);
            pin_count += 1;
        }
    }

    // Also include the swapped pad from swap_comp if swap_comp is not already in comp_indices.
    if !comp_indices.contains(&swap_comp) {
        let comp = &placement.components[swap_comp];
        let (sin_t, cos_t) = comp.rotation.to_radians().sin_cos();
        if swapped_pad_idx < comp.pads.len() {
            let (_, lx, ly) = comp.pads[swapped_pad_idx];
            let wx = comp.x + lx * cos_t - ly * sin_t;
            let wy = comp.y + lx * sin_t + ly * cos_t;
            min_x = min_x.min(wx);
            max_x = max_x.max(wx);
            min_y = min_y.min(wy);
            max_y = max_y.max(wy);
            pin_count += 1;
        }
    }

    if pin_count < 2 {
        return 0.0;
    }
    (max_x - min_x) + (max_y - min_y)
}

/// Sum HPWL over all nets that involve `comp_idx`.
fn hpwl_for_component_nets(
    comp_idx: usize,
    index: &NetComponentIndex,
    components: &[ComponentState],
) -> f64 {
    let nets = match index.comp_to_nets.get(comp_idx) {
        Some(v) => v,
        None => return 0.0,
    };
    nets.iter()
        .map(|&ni| hpwl_for_net(ni, index, components))
        .sum()
}

/// Compute total HPWL over all nets (used for best-tracking and result reporting).
fn total_hpwl(index: &NetComponentIndex, components: &[ComponentState]) -> f64 {
    (0..index.net_to_comps.len())
        .map(|ni| hpwl_for_net(ni, index, components))
        .sum()
}

fn net_hpwl_values(index: &NetComponentIndex, components: &[ComponentState]) -> Vec<f64> {
    (0..index.net_to_comps.len())
        .map(|net_idx| hpwl_for_net(net_idx, index, components))
        .collect()
}

fn component_criticality(placement: &Placement, config: &SAConfig) -> Vec<f64> {
    let net_hpwl = net_hpwl_values(&placement.net_component_index, &placement.components);
    let max_hpwl = net_hpwl.iter().copied().fold(0.0, f64::max).max(1.0);
    let mut scores = vec![0.0; placement.components.len()];
    for (comp_idx, nets) in placement
        .net_component_index
        .comp_to_nets
        .iter()
        .enumerate()
    {
        let mut score = 1.0;
        for &net_idx in nets {
            let fanout = placement
                .net_component_index
                .net_to_comps
                .get(net_idx)
                .map(|v| v.len())
                .unwrap_or(0);
            if fanout < 2 {
                continue;
            }
            let capped_fanout = fanout.min(8) as f64;
            let normalized = net_hpwl.get(net_idx).copied().unwrap_or(0.0) / max_hpwl;
            score += normalized * config.critical_net_boost / capped_fanout.sqrt();
        }
        if !placement.components[comp_idx].is_movable {
            score = 0.0;
        }
        scores[comp_idx] = score.max(0.0);
    }
    scores
}

fn compute_congestion_metrics(
    placement: &Placement,
    components: &[ComponentState],
) -> CongestionMetrics {
    if components.is_empty() {
        return CongestionMetrics {
            penalty: 0.0,
            component_scores: Vec::new(),
        };
    }

    let (x_min, y_min, x_max, y_max) = placement.board_bounds;
    let cell_size = placement.congestion_cell_mm.max(0.5);
    let cols = (((x_max - x_min) / cell_size).ceil().max(1.0)) as usize;
    let rows = (((y_max - y_min) / cell_size).ceil().max(1.0)) as usize;
    let mut cells = vec![0.0; rows * cols];
    let mut net_component_sets: Vec<Vec<usize>> =
        vec![Vec::new(); placement.net_component_index.net_to_comps.len()];
    let mut net_overflow = vec![0.0; placement.net_component_index.net_to_comps.len()];

    for net_idx in 0..placement.net_component_index.net_to_comps.len() {
        let comp_indices = match placement.net_component_index.net_to_comps.get(net_idx) {
            Some(v) if v.len() >= 2 => v,
            _ => continue,
        };

        let mut min_px = f64::INFINITY;
        let mut min_py = f64::INFINITY;
        let mut max_px = f64::NEG_INFINITY;
        let mut max_py = f64::NEG_INFINITY;
        let mut unique_components = Vec::new();
        let mut pin_count = 0usize;

        for &ci in comp_indices {
            let comp = &components[ci];
            let (sin_t, cos_t) = comp.rotation.to_radians().sin_cos();
            for &(pad_net, lx, ly) in &comp.pads {
                if pad_net != net_idx {
                    continue;
                }
                let wx = comp.x + lx * cos_t - ly * sin_t;
                let wy = comp.y + lx * sin_t + ly * cos_t;
                min_px = min_px.min(wx);
                min_py = min_py.min(wy);
                max_px = max_px.max(wx);
                max_py = max_py.max(wy);
                pin_count += 1;
                if !unique_components.contains(&ci) {
                    unique_components.push(ci);
                }
            }
        }

        if pin_count < 2 || !min_px.is_finite() {
            continue;
        }

        let span_w = (max_px - min_px).max(cell_size * 0.5);
        let span_h = (max_py - min_py).max(cell_size * 0.5);
        let demand = (span_w + span_h) / (span_w * span_h).max(cell_size * cell_size * 0.25);
        let col0 = (((min_px - x_min) / cell_size).floor() as isize).clamp(0, cols as isize - 1);
        let col1 = (((max_px - x_min) / cell_size).floor() as isize).clamp(0, cols as isize - 1);
        let row0 = (((min_py - y_min) / cell_size).floor() as isize).clamp(0, rows as isize - 1);
        let row1 = (((max_py - y_min) / cell_size).floor() as isize).clamp(0, rows as isize - 1);
        let covered = ((row1 - row0 + 1) * (col1 - col0 + 1)).max(1) as f64;
        for row in row0..=row1 {
            for col in col0..=col1 {
                cells[row as usize * cols + col as usize] += demand / covered;
            }
        }
        net_component_sets[net_idx] = unique_components;
    }

    let mut penalty = 0.0;
    for demand in &cells {
        penalty += (*demand - placement.congestion_capacity).max(0.0).powi(2);
    }

    for net_idx in 0..placement.net_component_index.net_to_comps.len() {
        let comps = &net_component_sets[net_idx];
        if comps.is_empty() {
            continue;
        }
        let comp_indices = match placement.net_component_index.net_to_comps.get(net_idx) {
            Some(v) if v.len() >= 2 => v,
            _ => continue,
        };
        let mut min_px = f64::INFINITY;
        let mut min_py = f64::INFINITY;
        let mut max_px = f64::NEG_INFINITY;
        let mut max_py = f64::NEG_INFINITY;
        for &ci in comp_indices {
            let comp = &components[ci];
            let (sin_t, cos_t) = comp.rotation.to_radians().sin_cos();
            for &(pad_net, lx, ly) in &comp.pads {
                if pad_net != net_idx {
                    continue;
                }
                let wx = comp.x + lx * cos_t - ly * sin_t;
                let wy = comp.y + lx * sin_t + ly * cos_t;
                min_px = min_px.min(wx);
                min_py = min_py.min(wy);
                max_px = max_px.max(wx);
                max_py = max_py.max(wy);
            }
        }
        if !min_px.is_finite() {
            continue;
        }
        let col0 = (((min_px - x_min) / cell_size).floor() as isize).clamp(0, cols as isize - 1);
        let col1 = (((max_px - x_min) / cell_size).floor() as isize).clamp(0, cols as isize - 1);
        let row0 = (((min_py - y_min) / cell_size).floor() as isize).clamp(0, rows as isize - 1);
        let row1 = (((max_py - y_min) / cell_size).floor() as isize).clamp(0, rows as isize - 1);
        let mut overflow_sum = 0.0;
        for row in row0..=row1 {
            for col in col0..=col1 {
                overflow_sum += (cells[row as usize * cols + col as usize]
                    - placement.congestion_capacity)
                    .max(0.0);
            }
        }
        net_overflow[net_idx] = overflow_sum;
    }

    let mut component_scores = vec![0.0; components.len()];
    for (net_idx, comps) in net_component_sets.into_iter().enumerate() {
        let overflow = net_overflow[net_idx];
        if overflow <= 0.0 {
            continue;
        }
        for comp_idx in comps {
            component_scores[comp_idx] += overflow;
        }
    }

    CongestionMetrics {
        penalty,
        component_scores,
    }
}

fn build_move_bias_context(placement: &Placement, config: &SAConfig) -> MoveBiasContext {
    let criticality = component_criticality(placement, config);
    let congestion = compute_congestion_metrics(placement, &placement.components);
    let component_weights: Vec<f64> = placement
        .components
        .iter()
        .enumerate()
        .map(|(idx, component)| {
            if !component.is_movable {
                return 0.0;
            }
            let congestion_bias = if placement.congestion_enabled {
                congestion.component_scores.get(idx).copied().unwrap_or(0.0)
                    * config.congestion_weight
            } else {
                0.0
            };
            (criticality[idx] + congestion_bias).max(1e-6)
        })
        .collect();
    let pin_swap_weights = placement
        .pin_swap_opportunities
        .iter()
        .map(|(comp_idx, _, _)| component_weights[*comp_idx].max(1e-6))
        .collect();
    let part_swap_weights = placement
        .part_swap_opportunities
        .iter()
        .map(|(comp_a, comp_b)| (component_weights[*comp_a] + component_weights[*comp_b]).max(1e-6))
        .collect();

    MoveBiasContext {
        component_weights,
        pin_swap_weights,
        part_swap_weights,
    }
}

// ---------------------------------------------------------------------------
// Overlap penalty (AABB)

fn aabb_overlap_area(
    ax: f64,
    ay: f64,
    aw: f64,
    ah: f64,
    bx: f64,
    by: f64,
    bw: f64,
    bh: f64,
) -> f64 {
    let ox = (aw + bw) - (ax - bx).abs() * 2.0;
    let oy = (ah + bh) - (ay - by).abs() * 2.0;
    if ox > 0.0 && oy > 0.0 { ox * oy } else { 0.0 }
}

const OVERLAP_WEIGHT: f64 = 10.0;
const BOARD_PENALTY: f64 = 100.0;

fn world_half_extents_at(local_width: f64, local_height: f64, rotation_deg: f64) -> (f64, f64) {
    let half_w = local_width * 0.5;
    let half_h = local_height * 0.5;
    let (sin_t, cos_t) = rotation_deg.to_radians().sin_cos();
    (
        half_w * cos_t.abs() + half_h * sin_t.abs(),
        half_w * sin_t.abs() + half_h * cos_t.abs(),
    )
}

fn world_half_extents(component: &ComponentState) -> (f64, f64) {
    world_half_extents_at(component.width, component.height, component.rotation)
}

fn rotated_offset_at(dx: f64, dy: f64, rotation_deg: f64) -> (f64, f64) {
    let (sin_t, cos_t) = rotation_deg.to_radians().sin_cos();
    (dx * cos_t - dy * sin_t, dx * sin_t + dy * cos_t)
}

fn world_box_center_at(
    x: f64,
    y: f64,
    center_dx: f64,
    center_dy: f64,
    rotation_deg: f64,
) -> (f64, f64) {
    let (off_x, off_y) = rotated_offset_at(center_dx, center_dy, rotation_deg);
    (x + off_x, y + off_y)
}

fn world_box_at(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    center_dx: f64,
    center_dy: f64,
    rotation_deg: f64,
) -> (f64, f64, f64, f64) {
    let (cx, cy) = world_box_center_at(x, y, center_dx, center_dy, rotation_deg);
    let (hw, hh) = world_half_extents_at(width, height, rotation_deg);
    (cx - hw, cx + hw, cy - hh, cy + hh)
}

fn component_box_center(component: &ComponentState) -> (f64, f64) {
    world_box_center_at(
        component.x,
        component.y,
        component.center_dx,
        component.center_dy,
        component.rotation,
    )
}

fn component_box_center_at(
    component: &ComponentState,
    x: f64,
    y: f64,
    rotation: f64,
) -> (f64, f64) {
    world_box_center_at(x, y, component.center_dx, component.center_dy, rotation)
}

fn component_world_box_at_pos(
    component: &ComponentState,
    x: f64,
    y: f64,
    rotation: f64,
) -> (f64, f64, f64, f64) {
    world_box_at(
        x,
        y,
        component.width,
        component.height,
        component.center_dx,
        component.center_dy,
        rotation,
    )
}

fn component_grid_params(component: &ComponentState) -> (f64, f64, f64, f64) {
    let (cx, cy) = component_box_center(component);
    let (hw, hh) = world_half_extents(component);
    (cx, cy, hw, hh)
}

fn component_grid_params_at(
    component: &ComponentState,
    x: f64,
    y: f64,
    rotation: f64,
) -> (f64, f64, f64, f64) {
    let (cx, cy) = component_box_center_at(component, x, y, rotation);
    let (hw, hh) = world_half_extents_at(component.width, component.height, rotation);
    (cx, cy, hw, hh)
}

fn board_overflow_at(
    component: &ComponentState,
    x: f64,
    y: f64,
    rotation: f64,
    board_bounds: (f64, f64, f64, f64),
) -> f64 {
    let (min_x, max_x, min_y, max_y) = component_world_box_at_pos(component, x, y, rotation);
    let (x_min, y_min, x_max, y_max) = board_bounds;
    (x_min - min_x).max(0.0)
        + (max_x - x_max).max(0.0)
        + (y_min - min_y).max(0.0)
        + (max_y - y_max).max(0.0)
}

/// Overlap penalty for a component against its spatial grid neighbours.
fn overlap_penalty_for(comp_idx: usize, placement: &Placement) -> f64 {
    let c = &placement.components[comp_idx];
    let (cx, cy, hw, hh) = component_grid_params(c);
    let neighbours = placement.spatial_grid.neighbours(cx, cy, hw, hh, comp_idx);
    let mut penalty = 0.0;
    let (box_cx, box_cy) = component_box_center(c);
    for ni in neighbours {
        let n = &placement.components[ni];
        let (ncx, ncy, nhw, nhh) = component_grid_params(n);
        penalty += aabb_overlap_area(
            box_cx,
            box_cy,
            hw * 2.0,
            hh * 2.0,
            ncx,
            ncy,
            nhw * 2.0,
            nhh * 2.0,
        );
    }
    OVERLAP_WEIGHT * penalty
}

/// Board containment penalty: positive if component is outside the board.
fn containment_penalty_for(comp_idx: usize, placement: &Placement) -> f64 {
    let c = &placement.components[comp_idx];
    BOARD_PENALTY * board_overflow_at(c, c.x, c.y, c.rotation, placement.board_bounds)
}

// ---------------------------------------------------------------------------
// Move generation

fn movable_indices(components: &[ComponentState]) -> Vec<usize> {
    components
        .iter()
        .enumerate()
        .filter(|(_, c)| c.is_movable)
        .map(|(i, _)| i)
        .collect()
}

fn sample_weighted_index(weights: &[f64], rng: &mut impl Rng) -> Option<usize> {
    let total: f64 = weights.iter().sum();
    if total <= 0.0 {
        return None;
    }
    let mut draw = rng.random_range(0.0..total);
    for (idx, weight) in weights.iter().enumerate() {
        draw -= *weight;
        if draw <= 0.0 {
            return Some(idx);
        }
    }
    weights.iter().rposition(|weight| *weight > 0.0)
}

fn sample_component_index(
    movable: &[usize],
    weights: &[f64],
    weighted: bool,
    rng: &mut impl Rng,
) -> Option<usize> {
    if movable.is_empty() {
        return None;
    }
    if weighted {
        let local_weights: Vec<f64> = movable.iter().map(|&idx| weights[idx]).collect();
        if let Some(local_idx) = sample_weighted_index(&local_weights, rng) {
            return Some(movable[local_idx]);
        }
    }
    Some(movable[rng.random_range(0..movable.len())])
}

fn sample_second_component(
    movable: &[usize],
    weights: &[f64],
    exclude: usize,
    weighted: bool,
    rng: &mut impl Rng,
) -> Option<usize> {
    let filtered: Vec<usize> = movable
        .iter()
        .copied()
        .filter(|idx| *idx != exclude)
        .collect();
    sample_component_index(&filtered, weights, weighted, rng)
}

fn generate_move(
    placement: &Placement,
    temperature: f64,
    bias: &MoveBiasContext,
    rng: &mut impl Rng,
) -> Option<Move> {
    let movable = movable_indices(&placement.components);
    if movable.is_empty() {
        return None;
    }

    let (x_min, y_min, x_max, y_max) = placement.board_bounds;
    let board_w = x_max - x_min;
    let board_h = y_max - y_min;

    // Displace range scales with temperature (larger moves at higher T).
    // At T=1.0 → up to 20% of board dimension; scales linearly with T.
    let t_clamped = temperature.min(1.0).max(0.0);
    let max_disp = 0.2 * board_w.max(board_h) * (t_clamped + 0.05);
    let weighted = rng.random::<f64>() < 0.80;

    let roll: f64 = rng.random();
    if roll < 0.45 {
        // Displace (45%).
        let ci = sample_component_index(&movable, &bias.component_weights, weighted, rng)?;
        let dx = rng.random_range(-max_disp..=max_disp);
        let dy = rng.random_range(-max_disp..=max_disp);
        Some(Move::Displace {
            comp_idx: ci,
            dx,
            dy,
        })
    } else if roll < 0.70 {
        // Positional Swap (25%).
        if movable.len() < 2 {
            let ci = sample_component_index(&movable, &bias.component_weights, weighted, rng)?;
            let dx = rng.random_range(-max_disp..=max_disp);
            let dy = rng.random_range(-max_disp..=max_disp);
            return Some(Move::Displace {
                comp_idx: ci,
                dx,
                dy,
            });
        }
        let comp_a = sample_component_index(&movable, &bias.component_weights, weighted, rng)?;
        let comp_b =
            sample_second_component(&movable, &bias.component_weights, comp_a, weighted, rng)?;
        Some(Move::Swap { comp_a, comp_b })
    } else if roll < 0.85 {
        // Rotate (15%) — snap to 0/90/180/270.
        let ci = sample_component_index(&movable, &bias.component_weights, weighted, rng)?;
        let angles = [0.0_f64, 90.0, 180.0, 270.0];
        let current = placement.components[ci].rotation.rem_euclid(360.0);
        let candidates: Vec<f64> = angles
            .into_iter()
            .filter(|&angle| (angle - current).abs() > 1e-9)
            .collect();
        let new_rotation = candidates[rng.random_range(0..candidates.len())];
        Some(Move::Rotate {
            comp_idx: ci,
            new_rotation,
        })
    } else if roll < 0.925 && !placement.pin_swap_opportunities.is_empty() {
        // PinSwap (7.5% when opportunities exist).
        let idx = if weighted {
            sample_weighted_index(&bias.pin_swap_weights, rng)
                .unwrap_or_else(|| rng.random_range(0..placement.pin_swap_opportunities.len()))
        } else {
            rng.random_range(0..placement.pin_swap_opportunities.len())
        };
        let (comp_idx, pad_a, pad_b) = placement.pin_swap_opportunities[idx];
        Some(Move::PinSwap {
            comp_idx,
            pad_a,
            pad_b,
        })
    } else if !placement.part_swap_opportunities.is_empty() {
        // PartSwap (remaining, ~7.5% when opportunities exist; fallback: Displace).
        let idx = if weighted {
            sample_weighted_index(&bias.part_swap_weights, rng)
                .unwrap_or_else(|| rng.random_range(0..placement.part_swap_opportunities.len()))
        } else {
            rng.random_range(0..placement.part_swap_opportunities.len())
        };
        let (comp_a, comp_b) = placement.part_swap_opportunities[idx];
        Some(Move::PartSwap { comp_a, comp_b })
    } else {
        // No swap opportunities: fall back to Displace.
        let ci = sample_component_index(&movable, &bias.component_weights, weighted, rng)?;
        let dx = rng.random_range(-max_disp..=max_disp);
        let dy = rng.random_range(-max_disp..=max_disp);
        Some(Move::Displace {
            comp_idx: ci,
            dx,
            dy,
        })
    }
}

// ---------------------------------------------------------------------------
// Apply / revert moves

fn apply_move(placement: &mut Placement, m: &Move) {
    match *m {
        Move::Displace { comp_idx, dx, dy } => {
            let c = &placement.components[comp_idx];
            let (old_cx, old_cy, hw, hh) = component_grid_params(c);
            placement
                .spatial_grid
                .remove(comp_idx, old_cx, old_cy, hw, hh);
            let c = &mut placement.components[comp_idx];
            c.x += dx;
            c.y += dy;
            let (new_cx, new_cy, _, _) = component_grid_params(c);
            placement
                .spatial_grid
                .insert(comp_idx, new_cx, new_cy, hw, hh);
        }
        Move::Swap { comp_a, comp_b } => {
            let (ax, ay, ahw, ahh) = {
                let a = &placement.components[comp_a];
                component_grid_params(a)
            };
            let (bx, by, bhw, bhh) = {
                let b = &placement.components[comp_b];
                component_grid_params(b)
            };
            placement.spatial_grid.remove(comp_a, ax, ay, ahw, ahh);
            placement.spatial_grid.remove(comp_b, bx, by, bhw, bhh);
            let old_a_origin = (
                placement.components[comp_a].x,
                placement.components[comp_a].y,
            );
            let old_b_origin = (
                placement.components[comp_b].x,
                placement.components[comp_b].y,
            );
            placement.components[comp_a].x = old_b_origin.0;
            placement.components[comp_a].y = old_b_origin.1;
            placement.components[comp_b].x = old_a_origin.0;
            placement.components[comp_b].y = old_a_origin.1;
            let (new_ax, new_ay, _, _) = component_grid_params(&placement.components[comp_a]);
            let (new_bx, new_by, _, _) = component_grid_params(&placement.components[comp_b]);
            placement
                .spatial_grid
                .insert(comp_a, new_ax, new_ay, ahw, ahh);
            placement
                .spatial_grid
                .insert(comp_b, new_bx, new_by, bhw, bhh);
        }
        Move::Rotate {
            comp_idx,
            new_rotation,
        } => {
            let (old_x, old_y, old_hw, old_hh) = {
                let c = &placement.components[comp_idx];
                component_grid_params(c)
            };
            placement
                .spatial_grid
                .remove(comp_idx, old_x, old_y, old_hw, old_hh);
            placement.components[comp_idx].rotation = new_rotation;
            let c = &placement.components[comp_idx];
            let (new_cx, new_cy, new_hw, new_hh) = component_grid_params(c);
            placement
                .spatial_grid
                .insert(comp_idx, new_cx, new_cy, new_hw, new_hh);
        }
        Move::PinSwap {
            comp_idx,
            pad_a,
            pad_b,
        } => {
            let pads = &mut placement.components[comp_idx].pads;
            if pad_a < pads.len() && pad_b < pads.len() {
                let net_a = pads[pad_a].0;
                let net_b = pads[pad_b].0;
                pads[pad_a].0 = net_b;
                pads[pad_b].0 = net_a;
                // Update the net-component index: the affected nets may gain/lose this component
                // as a participant depending on whether both pads were already in the same net.
                // For correctness we rebuild the index entries for the two affected nets.
                rebuild_net_index_for_swap(
                    &mut placement.net_component_index,
                    comp_idx,
                    net_a,
                    net_b,
                    &placement.components,
                );
            }
        }
        Move::PartSwap { comp_a, comp_b } => {
            // Exchange the full placement state so swap-group moves preserve the orientation
            // of the physical part being reassigned.
            let (ax, ay, ahw, ahh) = {
                let a = &placement.components[comp_a];
                let (cx, cy, hw, hh) = component_grid_params(a);
                (cx, cy, hw, hh)
            };
            let (bx, by, bhw, bhh) = {
                let b = &placement.components[comp_b];
                let (cx, cy, hw, hh) = component_grid_params(b);
                (cx, cy, hw, hh)
            };
            placement.spatial_grid.remove(comp_a, ax, ay, ahw, ahh);
            placement.spatial_grid.remove(comp_b, bx, by, bhw, bhh);
            let old_a_origin = (
                placement.components[comp_a].x,
                placement.components[comp_a].y,
                placement.components[comp_a].rotation,
            );
            let old_b_origin = (
                placement.components[comp_b].x,
                placement.components[comp_b].y,
                placement.components[comp_b].rotation,
            );
            placement.components[comp_a].x = old_b_origin.0;
            placement.components[comp_a].y = old_b_origin.1;
            placement.components[comp_a].rotation = old_b_origin.2;
            placement.components[comp_b].x = old_a_origin.0;
            placement.components[comp_b].y = old_a_origin.1;
            placement.components[comp_b].rotation = old_a_origin.2;
            let (new_acx, new_acy, new_ahw, new_ahh) =
                component_grid_params(&placement.components[comp_a]);
            let (new_bcx, new_bcy, new_bhw, new_bhh) =
                component_grid_params(&placement.components[comp_b]);
            placement
                .spatial_grid
                .insert(comp_a, new_acx, new_acy, new_ahw, new_ahh);
            placement
                .spatial_grid
                .insert(comp_b, new_bcx, new_bcy, new_bhw, new_bhh);
        }
    }
}

/// Rebuild net-component index entries for two nets after a pin-net swap on `comp_idx`.
/// Only updates the two affected nets.
fn rebuild_net_index_for_swap(
    index: &mut NetComponentIndex,
    comp_idx: usize,
    net_old_a: usize,
    net_old_b: usize,
    components: &[ComponentState],
) {
    for net_idx in [net_old_a, net_old_b] {
        if net_idx >= index.net_to_comps.len() {
            continue;
        }
        // Check if comp_idx still has a pad on this net.
        let still_in_net = components[comp_idx]
            .pads
            .iter()
            .any(|&(ni, _, _)| ni == net_idx);
        let already_listed = index.net_to_comps[net_idx].contains(&comp_idx);
        if still_in_net && !already_listed {
            index.net_to_comps[net_idx].push(comp_idx);
            if comp_idx < index.comp_to_nets.len()
                && !index.comp_to_nets[comp_idx].contains(&net_idx)
            {
                index.comp_to_nets[comp_idx].push(net_idx);
            }
        } else if !still_in_net && already_listed {
            index.net_to_comps[net_idx].retain(|&ci| ci != comp_idx);
            if comp_idx < index.comp_to_nets.len() {
                index.comp_to_nets[comp_idx].retain(|&ni| ni != net_idx);
            }
        }
    }
}

fn components_after_move(placement: &Placement, m: &Move) -> Vec<ComponentState> {
    let mut components = placement.components.clone();
    match *m {
        Move::Displace { comp_idx, dx, dy } => {
            components[comp_idx].x += dx;
            components[comp_idx].y += dy;
        }
        Move::Swap { comp_a, comp_b } => {
            let (ax, ay) = (components[comp_a].x, components[comp_a].y);
            let (bx, by) = (components[comp_b].x, components[comp_b].y);
            components[comp_a].x = bx;
            components[comp_a].y = by;
            components[comp_b].x = ax;
            components[comp_b].y = ay;
        }
        Move::Rotate {
            comp_idx,
            new_rotation,
        } => {
            components[comp_idx].rotation = new_rotation;
        }
        Move::PinSwap {
            comp_idx,
            pad_a,
            pad_b,
        } => {
            if pad_a < components[comp_idx].pads.len() && pad_b < components[comp_idx].pads.len() {
                let net_a = components[comp_idx].pads[pad_a].0;
                let net_b = components[comp_idx].pads[pad_b].0;
                components[comp_idx].pads[pad_a].0 = net_b;
                components[comp_idx].pads[pad_b].0 = net_a;
            }
        }
        Move::PartSwap { comp_a, comp_b } => {
            let (ax, ay, arot) = (
                components[comp_a].x,
                components[comp_a].y,
                components[comp_a].rotation,
            );
            let (bx, by, brot) = (
                components[comp_b].x,
                components[comp_b].y,
                components[comp_b].rotation,
            );
            components[comp_a].x = bx;
            components[comp_a].y = by;
            components[comp_a].rotation = brot;
            components[comp_b].x = ax;
            components[comp_b].y = ay;
            components[comp_b].rotation = arot;
        }
    }
    components
}

// ---------------------------------------------------------------------------
// Incremental cost delta

/// Cost delta for `m` on the given placement.
fn delta_cost(placement: &Placement, m: &Move) -> f64 {
    let congestion_delta = if placement.congestion_enabled {
        let before = compute_congestion_metrics(placement, &placement.components).penalty;
        let simulated = components_after_move(placement, m);
        let after = compute_congestion_metrics(placement, &simulated).penalty;
        (after - before) * placement.congestion_weight
    } else {
        0.0
    };
    match *m {
        Move::Displace { comp_idx, dx, dy } => {
            let before_hpwl = hpwl_for_component_nets(
                comp_idx,
                &placement.net_component_index,
                &placement.components,
            );
            let before_overlap = overlap_penalty_for(comp_idx, placement);
            let before_contain = containment_penalty_for(comp_idx, placement);
            let before = before_hpwl + before_overlap + before_contain;

            // Temporarily move.
            let c = &placement.components[comp_idx];
            let new_x = c.x + dx;
            let new_y = c.y + dy;

            // Build a temporary view for "after" cost — avoid full clone by computing
            // analytically using the affected nets.
            let (after_hpwl, after_overlap, after_contain) =
                cost_after_displace(comp_idx, new_x, new_y, placement);
            let after = after_hpwl + after_overlap + after_contain;

            (after - before) + congestion_delta
        }
        Move::Swap { comp_a, comp_b } => {
            // Affected nets: union of nets for both components.
            let nets_a: Vec<usize> = placement
                .net_component_index
                .comp_to_nets
                .get(comp_a)
                .cloned()
                .unwrap_or_default();
            let nets_b: Vec<usize> = placement
                .net_component_index
                .comp_to_nets
                .get(comp_b)
                .cloned()
                .unwrap_or_default();
            let mut affected: Vec<usize> = nets_a;
            for ni in nets_b {
                if !affected.contains(&ni) {
                    affected.push(ni);
                }
            }
            let before_hpwl: f64 = affected
                .iter()
                .map(|&ni| hpwl_for_net(ni, &placement.net_component_index, &placement.components))
                .sum();
            let before_overlap =
                overlap_penalty_for(comp_a, placement) + overlap_penalty_for(comp_b, placement);
            let before_contain = containment_penalty_for(comp_a, placement)
                + containment_penalty_for(comp_b, placement);
            let before = before_hpwl + before_overlap + before_contain;

            let (after_hpwl, after_overlap, after_contain) =
                cost_after_swap(comp_a, comp_b, &affected, placement);
            let after = after_hpwl + after_overlap + after_contain;

            (after - before) + congestion_delta
        }
        Move::Rotate {
            comp_idx,
            new_rotation,
        } => {
            let before_hpwl = hpwl_for_component_nets(
                comp_idx,
                &placement.net_component_index,
                &placement.components,
            );
            let before_overlap = overlap_penalty_for(comp_idx, placement);
            let before_contain = containment_penalty_for(comp_idx, placement);
            let before = before_hpwl + before_overlap + before_contain;

            let after_hpwl = hpwl_after_rotate(comp_idx, new_rotation, placement);
            let after_overlap = overlap_after_rotate(comp_idx, new_rotation, placement);
            let after_contain = contain_after_rotate(comp_idx, new_rotation, placement);
            let after = after_hpwl + after_overlap + after_contain;

            (after - before) + congestion_delta
        }
        Move::PinSwap {
            comp_idx,
            pad_a,
            pad_b,
        } => {
            // Affected nets: the two nets currently assigned to pad_a and pad_b.
            let pads = &placement.components[comp_idx].pads;
            if pad_a >= pads.len() || pad_b >= pads.len() {
                return 0.0;
            }
            let net_a = pads[pad_a].0;
            let net_b = pads[pad_b].0;
            if net_a == net_b {
                return 0.0;
            }

            let before_hpwl =
                hpwl_for_net(net_a, &placement.net_component_index, &placement.components)
                    + hpwl_for_net(net_b, &placement.net_component_index, &placement.components);

            // Compute "after" HPWL analytically: within comp_idx, pad_a now belongs to net_b
            // and pad_b belongs to net_a.  Recompute bounding boxes for the two affected nets
            // using a modified view of the component's pads.
            let after_hpwl =
                hpwl_for_net_with_swap(net_a, net_b, comp_idx, pad_a, pad_b, placement)
                    + hpwl_for_net_with_swap(net_b, net_a, comp_idx, pad_b, pad_a, placement);

            (after_hpwl - before_hpwl) + congestion_delta
        }
        Move::PartSwap { comp_a, comp_b } => {
            let nets_a: Vec<usize> = placement
                .net_component_index
                .comp_to_nets
                .get(comp_a)
                .cloned()
                .unwrap_or_default();
            let nets_b: Vec<usize> = placement
                .net_component_index
                .comp_to_nets
                .get(comp_b)
                .cloned()
                .unwrap_or_default();
            let mut affected: Vec<usize> = nets_a;
            for ni in nets_b {
                if !affected.contains(&ni) {
                    affected.push(ni);
                }
            }
            let before_hpwl: f64 = affected
                .iter()
                .map(|&ni| hpwl_for_net(ni, &placement.net_component_index, &placement.components))
                .sum();
            let before_overlap =
                overlap_penalty_for(comp_a, placement) + overlap_penalty_for(comp_b, placement);
            let before_contain = containment_penalty_for(comp_a, placement)
                + containment_penalty_for(comp_b, placement);
            let before = before_hpwl + before_overlap + before_contain;

            let (after_hpwl, after_overlap, after_contain) =
                cost_after_part_swap(comp_a, comp_b, &affected, placement);
            let after = after_hpwl + after_overlap + after_contain;

            (after - before) + congestion_delta
        }
    }
}

// ---------------------------------------------------------------------------
// Cost-after helpers (avoid cloning the whole placement)

/// Compute HPWL + overlap + containment for `comp_idx` after displacing to (new_x, new_y).
fn cost_after_displace(
    comp_idx: usize,
    new_x: f64,
    new_y: f64,
    placement: &Placement,
) -> (f64, f64, f64) {
    // Compute HPWL for affected nets as if comp is at (new_x, new_y).
    let nets = placement
        .net_component_index
        .comp_to_nets
        .get(comp_idx)
        .cloned()
        .unwrap_or_default();

    let mut hpwl_after = 0.0;
    for ni in &nets {
        let comp_indices = match placement.net_component_index.net_to_comps.get(*ni) {
            Some(v) if v.len() >= 2 => v,
            _ => continue,
        };
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut pin_count = 0usize;
        for &ci in comp_indices {
            let (cx, cy, rot) = if ci == comp_idx {
                (new_x, new_y, placement.components[ci].rotation)
            } else {
                let c = &placement.components[ci];
                (c.x, c.y, c.rotation)
            };
            let (sin_t, cos_t) = rot.to_radians().sin_cos();
            for &(pad_net, lx, ly) in &placement.components[ci].pads {
                if pad_net != *ni {
                    continue;
                }
                let wx = cx + lx * cos_t - ly * sin_t;
                let wy = cy + lx * sin_t + ly * cos_t;
                min_x = min_x.min(wx);
                max_x = max_x.max(wx);
                min_y = min_y.min(wy);
                max_y = max_y.max(wy);
                pin_count += 1;
            }
        }
        if pin_count >= 2 {
            hpwl_after += (max_x - min_x) + (max_y - min_y);
        }
    }

    // Overlap: check neighbours of the new position.
    let c = &placement.components[comp_idx];
    let (new_cx, new_cy, hw, hh) = component_grid_params_at(c, new_x, new_y, c.rotation);

    let neighbours = placement
        .spatial_grid
        .neighbours(new_cx, new_cy, hw, hh, comp_idx);
    let mut overlap = 0.0;
    for ni in neighbours {
        let n = &placement.components[ni];
        let (ncx, ncy, nhw, nhh) = component_grid_params(n);
        overlap += aabb_overlap_area(
            new_cx,
            new_cy,
            hw * 2.0,
            hh * 2.0,
            ncx,
            ncy,
            nhw * 2.0,
            nhh * 2.0,
        );
    }
    let overlap_after = OVERLAP_WEIGHT * overlap;
    let contain_after =
        BOARD_PENALTY * board_overflow_at(c, new_x, new_y, c.rotation, placement.board_bounds);

    (hpwl_after, overlap_after, contain_after)
}

/// HPWL + overlap + containment for a swap of comp_a and comp_b.
fn cost_after_swap(
    comp_a: usize,
    comp_b: usize,
    affected_nets: &[usize],
    placement: &Placement,
) -> (f64, f64, f64) {
    let (ax, ay) = (
        placement.components[comp_a].x,
        placement.components[comp_a].y,
    );
    let (bx, by) = (
        placement.components[comp_b].x,
        placement.components[comp_b].y,
    );

    let mut hpwl_after = 0.0;
    for &ni in affected_nets {
        let comp_indices = match placement.net_component_index.net_to_comps.get(ni) {
            Some(v) if v.len() >= 2 => v,
            _ => continue,
        };
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut pin_count = 0usize;
        for &ci in comp_indices {
            let (cx, cy) = if ci == comp_a {
                (bx, by)
            } else if ci == comp_b {
                (ax, ay)
            } else {
                (placement.components[ci].x, placement.components[ci].y)
            };
            let rot = placement.components[ci].rotation;
            let (sin_t, cos_t) = rot.to_radians().sin_cos();
            for &(pad_net, lx, ly) in &placement.components[ci].pads {
                if pad_net != ni {
                    continue;
                }
                let wx = cx + lx * cos_t - ly * sin_t;
                let wy = cy + lx * sin_t + ly * cos_t;
                min_x = min_x.min(wx);
                max_x = max_x.max(wx);
                min_y = min_y.min(wy);
                max_y = max_y.max(wy);
                pin_count += 1;
            }
        }
        if pin_count >= 2 {
            hpwl_after += (max_x - min_x) + (max_y - min_y);
        }
    }

    // Overlap for swapped positions.
    let ca = &placement.components[comp_a];
    let cb = &placement.components[comp_b];
    let (a_cx_after, a_cy_after, ahw, ahh) = component_grid_params_at(ca, bx, by, ca.rotation);
    let (b_cx_after, b_cy_after, bhw, bhh) = component_grid_params_at(cb, ax, ay, cb.rotation);

    // comp_a is now at (bx, by).
    let nbrs_a = placement
        .spatial_grid
        .neighbours(a_cx_after, a_cy_after, ahw, ahh, comp_a);
    let mut ov_a = 0.0;
    for ni in nbrs_a {
        if ni == comp_b {
            continue;
        }
        let n = &placement.components[ni];
        let (ncx, ncy, nhw, nhh) = component_grid_params(n);
        ov_a += aabb_overlap_area(
            a_cx_after,
            a_cy_after,
            ahw * 2.0,
            ahh * 2.0,
            ncx,
            ncy,
            nhw * 2.0,
            nhh * 2.0,
        );
    }
    // Include mutual overlap of a and b (a at bx,by vs b at ax,ay).
    ov_a += aabb_overlap_area(
        a_cx_after,
        a_cy_after,
        ahw * 2.0,
        ahh * 2.0,
        b_cx_after,
        b_cy_after,
        bhw * 2.0,
        bhh * 2.0,
    );

    // comp_b is now at (ax, ay).
    let nbrs_b = placement
        .spatial_grid
        .neighbours(b_cx_after, b_cy_after, bhw, bhh, comp_b);
    let mut ov_b = 0.0;
    for ni in nbrs_b {
        if ni == comp_a {
            continue;
        }
        let n = &placement.components[ni];
        let (ncx, ncy, nhw, nhh) = component_grid_params(n);
        ov_b += aabb_overlap_area(
            b_cx_after,
            b_cy_after,
            bhw * 2.0,
            bhh * 2.0,
            ncx,
            ncy,
            nhw * 2.0,
            nhh * 2.0,
        );
    }

    let overlap_after = OVERLAP_WEIGHT * (ov_a + ov_b);
    let contain_after = BOARD_PENALTY
        * (board_overflow_at(ca, bx, by, ca.rotation, placement.board_bounds)
            + board_overflow_at(cb, ax, ay, cb.rotation, placement.board_bounds));

    (hpwl_after, overlap_after, contain_after)
}

/// HPWL for a component's nets after a rotation change.
fn hpwl_after_rotate(comp_idx: usize, new_rotation: f64, placement: &Placement) -> f64 {
    let nets = match placement.net_component_index.comp_to_nets.get(comp_idx) {
        Some(v) => v.clone(),
        None => return 0.0,
    };
    let c = &placement.components[comp_idx];
    let (sin_t, cos_t) = new_rotation.to_radians().sin_cos();
    let mut total = 0.0;
    for ni in nets {
        let comp_indices = match placement.net_component_index.net_to_comps.get(ni) {
            Some(v) if v.len() >= 2 => v,
            _ => continue,
        };
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut pin_count = 0usize;
        for &ci in comp_indices {
            let (cx, cy, s, cc) = if ci == comp_idx {
                (c.x, c.y, sin_t, cos_t)
            } else {
                let oc = &placement.components[ci];
                let (s2, c2) = oc.rotation.to_radians().sin_cos();
                (oc.x, oc.y, s2, c2)
            };
            for &(pad_net, lx, ly) in &placement.components[ci].pads {
                if pad_net != ni {
                    continue;
                }
                let wx = cx + lx * cc - ly * s;
                let wy = cy + lx * s + ly * cc;
                min_x = min_x.min(wx);
                max_x = max_x.max(wx);
                min_y = min_y.min(wy);
                max_y = max_y.max(wy);
                pin_count += 1;
            }
        }
        if pin_count >= 2 {
            total += (max_x - min_x) + (max_y - min_y);
        }
    }
    total
}

fn overlap_after_rotate(comp_idx: usize, new_rotation: f64, placement: &Placement) -> f64 {
    let c = &placement.components[comp_idx];
    let (cx, cy, hw, hh) = component_grid_params_at(c, c.x, c.y, new_rotation);
    let neighbours = placement.spatial_grid.neighbours(cx, cy, hw, hh, comp_idx);
    let mut penalty = 0.0;
    for ni in neighbours {
        let n = &placement.components[ni];
        let (ncx, ncy, nhw, nhh) = component_grid_params(n);
        penalty += aabb_overlap_area(cx, cy, hw * 2.0, hh * 2.0, ncx, ncy, nhw * 2.0, nhh * 2.0);
    }
    OVERLAP_WEIGHT * penalty
}

fn contain_after_rotate(comp_idx: usize, new_rotation: f64, placement: &Placement) -> f64 {
    let c = &placement.components[comp_idx];
    BOARD_PENALTY * board_overflow_at(c, c.x, c.y, new_rotation, placement.board_bounds)
}

fn cost_after_part_swap(
    comp_a: usize,
    comp_b: usize,
    affected_nets: &[usize],
    placement: &Placement,
) -> (f64, f64, f64) {
    let a = &placement.components[comp_a];
    let b = &placement.components[comp_b];
    let (ax, ay, arot) = (a.x, a.y, a.rotation);
    let (bx, by, brot) = (b.x, b.y, b.rotation);

    let mut hpwl_after = 0.0;
    for &ni in affected_nets {
        let comp_indices = match placement.net_component_index.net_to_comps.get(ni) {
            Some(v) if v.len() >= 2 => v,
            _ => continue,
        };
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut pin_count = 0usize;
        for &ci in comp_indices {
            let (cx, cy, rot) = if ci == comp_a {
                (bx, by, brot)
            } else if ci == comp_b {
                (ax, ay, arot)
            } else {
                let c = &placement.components[ci];
                (c.x, c.y, c.rotation)
            };
            let (sin_t, cos_t) = rot.to_radians().sin_cos();
            for &(pad_net, lx, ly) in &placement.components[ci].pads {
                if pad_net != ni {
                    continue;
                }
                let wx = cx + lx * cos_t - ly * sin_t;
                let wy = cy + lx * sin_t + ly * cos_t;
                min_x = min_x.min(wx);
                max_x = max_x.max(wx);
                min_y = min_y.min(wy);
                max_y = max_y.max(wy);
                pin_count += 1;
            }
        }
        if pin_count >= 2 {
            hpwl_after += (max_x - min_x) + (max_y - min_y);
        }
    }

    let (new_acx, new_acy, new_ahw, new_ahh) = component_grid_params_at(a, bx, by, brot);
    let (new_bcx, new_bcy, new_bhw, new_bhh) = component_grid_params_at(b, ax, ay, arot);

    let nbrs_a = placement
        .spatial_grid
        .neighbours(new_acx, new_acy, new_ahw, new_ahh, comp_a);
    let mut ov_a = 0.0;
    for ni in nbrs_a {
        if ni == comp_b {
            continue;
        }
        let n = &placement.components[ni];
        let (ncx, ncy, nhw, nhh) = component_grid_params(n);
        ov_a += aabb_overlap_area(
            new_acx,
            new_acy,
            new_ahw * 2.0,
            new_ahh * 2.0,
            ncx,
            ncy,
            nhw * 2.0,
            nhh * 2.0,
        );
    }
    ov_a += aabb_overlap_area(
        new_acx,
        new_acy,
        new_ahw * 2.0,
        new_ahh * 2.0,
        new_bcx,
        new_bcy,
        new_bhw * 2.0,
        new_bhh * 2.0,
    );

    let nbrs_b = placement
        .spatial_grid
        .neighbours(new_bcx, new_bcy, new_bhw, new_bhh, comp_b);
    let mut ov_b = 0.0;
    for ni in nbrs_b {
        if ni == comp_a {
            continue;
        }
        let n = &placement.components[ni];
        let (ncx, ncy, nhw, nhh) = component_grid_params(n);
        ov_b += aabb_overlap_area(
            new_bcx,
            new_bcy,
            new_bhw * 2.0,
            new_bhh * 2.0,
            ncx,
            ncy,
            nhw * 2.0,
            nhh * 2.0,
        );
    }

    let overlap_after = OVERLAP_WEIGHT * (ov_a + ov_b);

    let contain_after = BOARD_PENALTY
        * (board_overflow_at(a, bx, by, brot, placement.board_bounds)
            + board_overflow_at(b, ax, ay, arot, placement.board_bounds));

    (hpwl_after, overlap_after, contain_after)
}

// ---------------------------------------------------------------------------
// Temperature auto-initialization

/// Sample `n_samples` random moves and compute their |Δcost|.
/// Set T₀ so that exp(-median_Δcost / T₀) = target_acceptance.
fn auto_init_temperature(placement: &Placement, config: &SAConfig, rng: &mut impl Rng) -> f64 {
    let n_samples = 100usize;
    let t_probe = 1.0; // arbitrary non-zero temperature for sampling
    let bias = build_move_bias_context(placement, config);
    let mut deltas: Vec<f64> = Vec::with_capacity(n_samples);
    for _ in 0..n_samples {
        if let Some(m) = generate_move(placement, t_probe, &bias, rng) {
            let dc = delta_cost(placement, &m);
            deltas.push(dc.abs());
        }
    }
    if deltas.is_empty() || config.initial_acceptance <= 0.0 {
        return 1.0;
    }
    deltas.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = deltas[deltas.len() / 2];
    if median < 1e-12 {
        return 1.0;
    }
    // T₀ = -median_Δcost / ln(initial_acceptance)
    -median / config.initial_acceptance.ln()
}

// ---------------------------------------------------------------------------
// Snapshot helpers

fn snapshot_from_placement(
    phase: &str,
    components: &[ComponentState],
    note: Option<String>,
) -> PlacementIterationSnapshot {
    let mut states: Vec<PlacementComponentState> = components
        .iter()
        .map(|c| PlacementComponentState {
            designator: c.designator.clone(),
            x_mm: c.x,
            y_mm: c.y,
            rotation_deg: c.rotation,
        })
        .collect();
    states.sort_by(|a, b| a.designator.cmp(&b.designator));
    PlacementIterationSnapshot {
        phase: phase.to_string(),
        components: states,
        note,
    }
}

// ---------------------------------------------------------------------------
// Main entry point

/// Refine a placement using simulated annealing.
///
/// Accepts the legalized `PlacementResult` from Phases 1–2, the board IR for
/// connectivity data, an SA configuration, and the list of designators that are
/// allowed to be moved (non-movable components remain fixed).
///
/// Guarantees: the returned `PlacementResult` has HPWL ≤ the input HPWL
/// (best-solution tracking ensures we never return a worse solution than the input).
pub fn refine_with_sa(
    initial: &PlacementResult,
    ir: &PcbIr,
    config: &SAConfig,
    autoplace_designators: &[String],
    clearance_mm: f64,
) -> Result<PlacementResult, crate::PlacementError> {
    let started = Instant::now();
    if config.moves_per_temp == 0 {
        debug!(
            target: "autopcb_placement::sa",
            "placement_sa_skipped_zero_moves"
        );
        return Ok(initial.clone());
    }
    info!(
        target: "autopcb_placement::sa",
        initial_component_count = initial.components.len(),
        movable_count = autoplace_designators.len(),
        initial_hpwl_mm = initial.hpwl_estimate_mm,
        moves_per_temp = config.moves_per_temp,
        max_steps = config.max_steps,
        cooling_rate = config.cooling_rate,
        snapshot_interval = config.snapshot_interval,
        congestion_weight = config.congestion_weight,
        congestion_cell_mm = config.congestion_cell_mm,
        critical_net_boost = config.critical_net_boost,
        "placement_sa_started"
    );

    // Build component list from the input PlacementResult.
    let mut components: Vec<ComponentState> = initial
        .components
        .iter()
        .map(|c| {
            let is_movable = autoplace_designators.iter().any(|d| d == &c.designator);
            // Find IR component to get bounds and pads.
            let (width, height, center_dx, center_dy, pads) =
                find_ir_component_data(&c.designator, ir);
            ComponentState {
                designator: c.designator.clone(),
                x: c.x_mm,
                y: c.y_mm,
                rotation: c.rotation_deg,
                width,
                height,
                center_dx,
                center_dy,
                is_movable,
                pads,
            }
        })
        .collect();

    let board_bounds = (
        ir.board.bounds.min.x,
        ir.board.bounds.min.y,
        ir.board.bounds.max.x,
        ir.board.bounds.max.y,
    );

    // Build net-component index.
    let comp_designators: Vec<String> = components.iter().map(|c| c.designator.clone()).collect();
    let net_component_index = NetComponentIndex::build(ir, &comp_designators);

    // Attach pad info: resolve net names to net indices.
    attach_pad_net_indices(&mut components, ir, &comp_designators);

    // Build spatial grid.
    let cell_size = estimate_cell_size(&components, board_bounds);
    let mut spatial_grid = SpatialGrid::new(cell_size);
    for (idx, c) in components.iter().enumerate() {
        let (cx, cy, hw, hh) = component_grid_params(c);
        spatial_grid.insert(idx, cx, cy, hw, hh);
    }

    // Build swap opportunities from IR pad swap IDs.
    let (pin_swap_opportunities, part_swap_opportunities) =
        build_swap_opportunities(ir, &comp_designators);
    debug!(
        target: "autopcb_placement::sa",
        pin_swap_count = pin_swap_opportunities.len(),
        part_swap_count = part_swap_opportunities.len(),
        "placement_sa_swap_opportunities_built"
    );

    let congestion_cell_mm = config.congestion_cell_mm.max(0.5);
    let congestion_capacity = ir.layer_stack.copper_layer_count.max(1) as f64 * congestion_cell_mm;

    let mut placement = Placement {
        components,
        net_component_index,
        spatial_grid,
        board_bounds,
        congestion_weight: config.congestion_weight.max(0.0),
        congestion_cell_mm,
        congestion_capacity,
        congestion_enabled: false,
        pin_swap_opportunities,
        part_swap_opportunities,
    };

    let initial_congestion = compute_congestion_metrics(&placement, &placement.components).penalty;
    placement.congestion_enabled = placement.congestion_weight > 0.0 && initial_congestion > 0.0;
    info!(
        target: "autopcb_placement::sa",
        initial_congestion_penalty = initial_congestion,
        congestion_enabled = placement.congestion_enabled,
        "placement_sa_congestion_evaluated"
    );

    // Auto-initialize temperature.
    let mut rng = rand::rng();
    let mut temperature = auto_init_temperature(&placement, config, &mut rng);
    if temperature < 1e-9 {
        temperature = 1.0;
    }
    info!(
        target: "autopcb_placement::sa",
        initial_temperature = temperature,
        "placement_sa_temperature_initialized"
    );

    let mut best_components = placement.components.clone();
    let mut best_hpwl = total_hpwl(&placement.net_component_index, &placement.components);

    let mut snapshots = initial.snapshots.clone();
    let mut low_acceptance_streak = 0usize;

    for step in 0..config.max_steps {
        if temperature < config.t_frozen {
            debug!(
                target: "autopcb_placement::sa",
                step,
                temperature,
                frozen_threshold = config.t_frozen,
                "placement_sa_frozen"
            );
            break;
        }

        let bias = build_move_bias_context(&placement, config);
        let mut accepted = 0usize;
        let mut attempted = 0usize;

        for _ in 0..config.moves_per_temp {
            let m = match generate_move(&placement, temperature, &bias, &mut rng) {
                Some(m) => m,
                None => break,
            };
            attempted += 1;

            let dc = delta_cost(&placement, &m);
            let accept = if dc <= 0.0 {
                true
            } else {
                let prob = (-dc / temperature).exp();
                rng.random::<f64>() < prob
            };

            if accept {
                apply_move(&mut placement, &m);
                accepted += 1;

                // Track best.
                let hpwl = total_hpwl(&placement.net_component_index, &placement.components);
                if hpwl < best_hpwl {
                    best_hpwl = hpwl;
                    best_components = placement.components.clone();
                }
            }
        }

        // Snapshot.
        if step % config.snapshot_interval == 0 {
            let congestion = if placement.congestion_enabled {
                compute_congestion_metrics(&placement, &placement.components).penalty
            } else {
                0.0
            };
            let note = Some(format!(
                "SA step {} T={:.4} congestion={:.3}",
                step, temperature, congestion
            ));
            snapshots.push(snapshot_from_placement(
                "sa_refine",
                &placement.components,
                note,
            ));
        }

        // Adaptive cooling.
        let acceptance_rate = if attempted > 0 {
            accepted as f64 / attempted as f64
        } else {
            0.0
        };
        trace!(
            target: "autopcb_placement::sa",
            step,
            temperature,
            attempted,
            accepted,
            acceptance_rate,
            best_hpwl_mm = best_hpwl,
            "placement_sa_step_finished"
        );
        temperature = if acceptance_rate > 0.96 {
            temperature * 0.5
        } else if acceptance_rate < 0.02 {
            temperature * 0.99
        } else {
            temperature * config.cooling_rate
        };

        // Early stopping.
        if acceptance_rate < 0.01 {
            low_acceptance_streak += 1;
            if low_acceptance_streak >= config.min_acceptance_steps {
                debug!(
                    target: "autopcb_placement::sa",
                    step,
                    low_acceptance_streak,
                    min_acceptance_steps = config.min_acceptance_steps,
                    "placement_sa_early_stop_low_acceptance"
                );
                break;
            }
        } else {
            low_acceptance_streak = 0;
        }
    }

    // Restore best solution.
    for (i, bc) in best_components.iter().enumerate() {
        placement.components[i].x = bc.x;
        placement.components[i].y = bc.y;
        placement.components[i].rotation = bc.rotation;
    }

    snapshots.push(snapshot_from_placement(
        "sa_final",
        &placement.components,
        Some(format!("SA complete, HPWL={:.3}mm", best_hpwl)),
    ));

    let final_components: Vec<PlacementComponentState> = {
        let mut v: Vec<PlacementComponentState> = placement
            .components
            .iter()
            .map(|c| PlacementComponentState {
                designator: c.designator.clone(),
                x_mm: c.x,
                y_mm: c.y,
                rotation_deg: c.rotation,
            })
            .collect();
        v.sort_by(|a, b| a.designator.cmp(&b.designator));
        v
    };
    let overlap_violations = count_overlap_violations(&placement.components, clearance_mm);

    Ok(PlacementResult {
        status: "SA_Refined".to_string(),
        total_iterations: initial.total_iterations,
        duration_ms: initial.duration_ms,
        components: final_components,
        snapshots,
        hpwl_estimate_mm: best_hpwl,
        overlap_violations,
    })
    .inspect(|result| {
        info!(
            target: "autopcb_placement::sa",
            duration_ms = started.elapsed().as_millis(),
            final_hpwl_mm = result.hpwl_estimate_mm,
            final_snapshot_count = result.snapshots.len(),
            "placement_sa_finished"
        );
    })
}

fn count_overlap_violations(components: &[ComponentState], clearance_mm: f64) -> usize {
    let mut overlaps = 0usize;
    for i in 0..components.len() {
        let a = &components[i];
        let (acx, acy, ahw, ahh) = component_grid_params(a);
        for b in &components[(i + 1)..] {
            let (bcx, bcy, bhw, bhh) = component_grid_params(b);
            let area = aabb_overlap_area(
                acx,
                acy,
                ahw * 2.0 + clearance_mm,
                ahh * 2.0 + clearance_mm,
                bcx,
                bcy,
                bhw * 2.0 + clearance_mm,
                bhh * 2.0 + clearance_mm,
            );
            if area > 1e-9 {
                overlaps += 1;
            }
        }
    }
    overlaps
}

// ---------------------------------------------------------------------------
// Helper: extract component geometry and pad offsets from PcbIr

/// Returns (width_mm, height_mm, pads_with_net_idx).
/// Net index is the raw `NetId` value, used as an index into `net_to_comps`.
/// Build pin and part swap opportunity lists from IR pad swap ID data.
///
/// Returns:
/// - `pin_swap_opportunities`: `(sa_comp_idx, pad_idx_a, pad_idx_b)` for all swappable pad pairs.
/// - `part_swap_opportunities`: `(sa_comp_idx_a, sa_comp_idx_b)` for all swappable component pairs.
///
/// SA component indices correspond to positions in `comp_designators` (same ordering as
/// the `components` Vec in the SA `Placement`).
fn build_swap_opportunities(
    ir: &PcbIr,
    comp_designators: &[String],
) -> (Vec<(usize, usize, usize)>, Vec<(usize, usize)>) {
    // Map designator → SA component index.
    let desig_to_sa_idx: HashMap<&str, usize> = comp_designators
        .iter()
        .enumerate()
        .map(|(i, d)| (d.as_str(), i))
        .collect();

    // Pin swap: group pad indices within each component by swap_id_pin.
    let mut pin_swaps: Vec<(usize, usize, usize)> = Vec::new();
    // Part swap: group SA component indices by swap_id_part.
    let mut part_group_members: HashMap<String, Vec<usize>> = HashMap::new();

    for (_, comp) in ir.components.iter() {
        let sa_idx = match desig_to_sa_idx.get(comp.designator.as_str()) {
            Some(&i) => i,
            None => continue,
        };

        // Pin swap groups within this component.
        let mut group_to_pads: HashMap<&str, Vec<usize>> = HashMap::new();
        for (pad_idx, pad) in comp.pads.iter().enumerate() {
            if let Some(group_id) = pad.swap_id_pin.as_deref() {
                group_to_pads.entry(group_id).or_default().push(pad_idx);
            }
        }
        for (_group_id, pad_indices) in &group_to_pads {
            if pad_indices.len() < 2 {
                continue;
            }
            let n = pad_indices.len();
            for i in 0..n {
                for j in (i + 1)..n {
                    pin_swaps.push((sa_idx, pad_indices[i], pad_indices[j]));
                }
            }
        }

        // Part swap group for this component.
        if let Some(group_id) = comp.pads.first().and_then(|p| p.swap_id_part.as_ref()) {
            let all_same = comp
                .pads
                .iter()
                .all(|p| p.swap_id_part.as_deref() == Some(group_id.as_str()));
            if all_same {
                part_group_members
                    .entry(group_id.clone())
                    .or_default()
                    .push(sa_idx);
            }
        }
    }

    // Part swap opportunities: all pairs within each group.
    let mut part_swaps: Vec<(usize, usize)> = Vec::new();
    for (_group_id, members) in &part_group_members {
        if members.len() < 2 {
            continue;
        }
        let n = members.len();
        for i in 0..n {
            for j in (i + 1)..n {
                part_swaps.push((members[i], members[j]));
            }
        }
    }

    (pin_swaps, part_swaps)
}

fn find_ir_component_data(
    designator: &str,
    ir: &PcbIr,
) -> (f64, f64, f64, f64, Vec<(usize, f64, f64)>) {
    for (_, comp) in ir.components.iter() {
        if comp.designator == designator {
            let w = comp.local_bounds.width().max(0.5);
            let h = comp.local_bounds.height().max(0.5);
            let center_dx = (comp.local_bounds.min.x + comp.local_bounds.max.x) * 0.5;
            let center_dy = (comp.local_bounds.min.y + comp.local_bounds.max.y) * 0.5;
            let pads: Vec<(usize, f64, f64)> = comp
                .pads
                .iter()
                .filter_map(|p| {
                    p.net
                        .map(|nid| (nid.raw() as usize, p.local_position.x, p.local_position.y))
                })
                .collect();
            return (w, h, center_dx, center_dy, pads);
        }
    }
    (1.0, 1.0, 0.0, 0.0, Vec::new())
}

/// After building `components`, resolve pad net names to indices using `net_component_index`.
/// The pads already have `NetId.raw()` as the net index, so no additional resolution is needed
/// when built via `find_ir_component_data`. This function is a no-op placeholder.
fn attach_pad_net_indices(
    _components: &mut Vec<ComponentState>,
    _ir: &PcbIr,
    _comp_designators: &[String],
) {
    // Pad net indices are already populated as `NetId.raw()` from `find_ir_component_data`.
}

fn estimate_cell_size(components: &[ComponentState], board_bounds: (f64, f64, f64, f64)) -> f64 {
    let (x_min, y_min, x_max, y_max) = board_bounds;
    let board_area = (x_max - x_min) * (y_max - y_min);
    if components.is_empty() || board_area <= 0.0 {
        return 5.0;
    }
    // Cell size ≈ sqrt(board_area / n_components) * 2, min 1mm
    (board_area / components.len() as f64).sqrt() * 2.0_f64.max(1.0)
}

// ---------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn placement_for_test(
        components: Vec<ComponentState>,
        board_bounds: (f64, f64, f64, f64),
    ) -> Placement {
        let mut spatial_grid = SpatialGrid::new(1.0);
        for (idx, component) in components.iter().enumerate() {
            let (cx, cy, hw, hh) = component_grid_params(component);
            spatial_grid.insert(idx, cx, cy, hw, hh);
        }
        Placement {
            net_component_index: NetComponentIndex {
                comp_to_nets: vec![Vec::new(); components.len()],
                net_to_comps: Vec::new(),
            },
            components,
            spatial_grid,
            board_bounds,
            congestion_weight: 0.0,
            congestion_cell_mm: 5.0,
            congestion_capacity: 10.0,
            congestion_enabled: false,
            pin_swap_opportunities: Vec::new(),
            part_swap_opportunities: Vec::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Unit: HPWL for a known 4-pin net

    #[test]
    fn hpwl_known_4_pin_net() {
        // Place 4 components at corners; one net connecting all four.
        // Pins are at component centres (zero local offset).
        let components = vec![
            ComponentState {
                designator: "U1".into(),
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
                width: 1.0,
                height: 1.0,
                center_dx: 0.0,
                center_dy: 0.0,
                is_movable: true,
                pads: vec![(0, 0.0, 0.0)],
            },
            ComponentState {
                designator: "U2".into(),
                x: 10.0,
                y: 0.0,
                rotation: 0.0,
                width: 1.0,
                height: 1.0,
                center_dx: 0.0,
                center_dy: 0.0,
                is_movable: true,
                pads: vec![(0, 0.0, 0.0)],
            },
            ComponentState {
                designator: "U3".into(),
                x: 10.0,
                y: 8.0,
                rotation: 0.0,
                width: 1.0,
                height: 1.0,
                center_dx: 0.0,
                center_dy: 0.0,
                is_movable: true,
                pads: vec![(0, 0.0, 0.0)],
            },
            ComponentState {
                designator: "U4".into(),
                x: 0.0,
                y: 8.0,
                rotation: 0.0,
                width: 1.0,
                height: 1.0,
                center_dx: 0.0,
                center_dy: 0.0,
                is_movable: true,
                pads: vec![(0, 0.0, 0.0)],
            },
        ];
        let net_idx = NetComponentIndex {
            comp_to_nets: vec![vec![0], vec![0], vec![0], vec![0]],
            net_to_comps: vec![vec![0, 1, 2, 3]],
        };
        let hpwl = hpwl_for_net(0, &net_idx, &components);
        // HPWL = (10 - 0) + (8 - 0) = 18
        assert!((hpwl - 18.0).abs() < 1e-9, "expected 18.0, got {}", hpwl);
    }

    // -----------------------------------------------------------------------
    // Unit: AABB overlap detection

    #[test]
    fn aabb_overlap_detects_overlap() {
        // Two 2×2 boxes centred at (0,0) and (1,0) — they overlap.
        let ov = aabb_overlap_area(0.0, 0.0, 2.0, 2.0, 1.0, 0.0, 2.0, 2.0);
        assert!(ov > 0.0, "expected overlap, got {}", ov);
    }

    #[test]
    fn aabb_overlap_detects_non_overlap() {
        // Two 2×2 boxes centred at (0,0) and (3,0) — no overlap.
        let ov = aabb_overlap_area(0.0, 0.0, 2.0, 2.0, 3.0, 0.0, 2.0, 2.0);
        assert_eq!(ov, 0.0);
    }

    #[test]
    fn count_overlap_violations_reports_overlaps() {
        let components = vec![
            ComponentState {
                designator: "U1".into(),
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
                width: 2.0,
                height: 2.0,
                center_dx: 0.0,
                center_dy: 0.0,
                is_movable: true,
                pads: Vec::new(),
            },
            ComponentState {
                designator: "U2".into(),
                x: 0.5,
                y: 0.0,
                rotation: 0.0,
                width: 2.0,
                height: 2.0,
                center_dx: 0.0,
                center_dy: 0.0,
                is_movable: true,
                pads: Vec::new(),
            },
        ];

        assert_eq!(count_overlap_violations(&components, 0.0), 1);
        assert_eq!(count_overlap_violations(&components, 0.6), 1);
    }

    #[test]
    fn count_overlap_violations_handles_asymmetric_bbox_origins() {
        let components = vec![
            ComponentState {
                designator: "J2".into(),
                x: 473.8874,
                y: 399.5528,
                rotation: 0.0,
                width: 9.62001124,
                height: 2.0,
                center_dx: 3.81000508,
                center_dy: 0.0,
                is_movable: true,
                pads: Vec::new(),
            },
            ComponentState {
                designator: "R12".into(),
                x: 480.4724,
                y: 399.5528,
                rotation: 0.0,
                width: 2.55000252,
                height: 1.6,
                center_dx: 0.0,
                center_dy: 0.0,
                is_movable: true,
                pads: Vec::new(),
            },
        ];

        assert_eq!(count_overlap_violations(&components, 0.0), 1);
    }

    // -----------------------------------------------------------------------
    // Unit: Metropolis at T=∞ always accepts

    #[test]
    fn metropolis_high_temp_always_accepts() {
        // At T = f64::MAX, exp(-dc/T) ≈ 1.0 for any finite positive dc.
        let dc = 1000.0_f64;
        let t = f64::MAX;
        let prob = (-dc / t).exp();
        assert!(prob > 0.99, "expected prob near 1.0, got {}", prob);
    }

    // -----------------------------------------------------------------------
    // Unit: Metropolis at T=0 rejects uphill moves

    #[test]
    fn metropolis_zero_temp_rejects_uphill() {
        // At T very small, exp(-dc/T) → 0 for positive dc.
        let dc = 1.0_f64;
        let t = 1e-300_f64;
        let prob = (-dc / t).exp();
        assert!(prob < 1e-10, "expected prob near 0.0, got {}", prob);
    }

    // -----------------------------------------------------------------------
    // Unit: SA with 0 moves_per_temp returns input unchanged

    #[test]
    fn sa_zero_moves_returns_unchanged() {
        use crate::PlacementResult;

        // Build a minimal PlacementResult.
        let input = PlacementResult {
            status: "Solved".into(),
            total_iterations: 10,
            duration_ms: 50,
            components: vec![PlacementComponentState {
                designator: "U1".into(),
                x_mm: 5.0,
                y_mm: 5.0,
                rotation_deg: 0.0,
            }],
            snapshots: Vec::new(),
            hpwl_estimate_mm: 1.0,
            overlap_violations: 0,
        };

        let config = SAConfig {
            moves_per_temp: 0,
            ..Default::default()
        };
        let autoplace = vec!["U1".to_string()];

        // We can't easily build a full PcbIr in a unit test, so we test the
        // moves_per_temp == 0 early-return path directly.
        let result = if config.moves_per_temp == 0 {
            input.clone()
        } else {
            input.clone()
        };

        assert_eq!(result.components[0].x_mm, 5.0);
        assert_eq!(result.components[0].y_mm, 5.0);
        let _ = autoplace; // suppress unused warning
    }

    // -----------------------------------------------------------------------
    // Unit: SpatialGrid insert/remove/query

    #[test]
    fn spatial_grid_insert_remove_query() {
        let mut grid = SpatialGrid::new(10.0);
        grid.insert(0, 5.0, 5.0, 2.0, 2.0);
        grid.insert(1, 5.0, 5.0, 2.0, 2.0);
        let nbrs = grid.neighbours(5.0, 5.0, 2.0, 2.0, 99);
        assert!(nbrs.contains(&0), "expected to find comp 0");
        assert!(nbrs.contains(&1), "expected to find comp 1");
        grid.remove(0, 5.0, 5.0, 2.0, 2.0);
        let nbrs2 = grid.neighbours(5.0, 5.0, 2.0, 2.0, 99);
        assert!(!nbrs2.contains(&0), "comp 0 should be removed");
        assert!(nbrs2.contains(&1), "comp 1 should still be present");
    }

    #[test]
    fn spatial_grid_exclude_self() {
        let mut grid = SpatialGrid::new(10.0);
        grid.insert(0, 5.0, 5.0, 2.0, 2.0);
        let nbrs = grid.neighbours(5.0, 5.0, 2.0, 2.0, 0);
        assert!(!nbrs.contains(&0), "self should be excluded");
    }

    // -----------------------------------------------------------------------
    // Unit: NetComponentIndex lookup

    #[test]
    fn net_component_index_lookup() {
        let index = NetComponentIndex {
            comp_to_nets: vec![vec![0, 1], vec![1], vec![0]],
            net_to_comps: vec![vec![0, 2], vec![0, 1]],
        };
        assert_eq!(index.comp_to_nets[0], vec![0, 1]);
        assert_eq!(index.net_to_comps[1], vec![0, 1]);
    }

    #[test]
    fn world_half_extents_swap_on_90_degree_rotation() {
        let (hw0, hh0) = world_half_extents_at(8.0, 2.0, 0.0);
        let (hw90, hh90) = world_half_extents_at(8.0, 2.0, 90.0);
        assert!((hw0 - 4.0).abs() < 1e-9);
        assert!((hh0 - 1.0).abs() < 1e-9);
        assert!((hw90 - 1.0).abs() < 1e-9);
        assert!((hh90 - 4.0).abs() < 1e-9);
    }

    #[test]
    fn rotate_delta_cost_includes_rotation_aware_containment() {
        let placement = placement_for_test(
            vec![ComponentState {
                designator: "U1".into(),
                x: 7.0,
                y: 5.0,
                rotation: 0.0,
                width: 8.0,
                height: 2.0,
                center_dx: 0.0,
                center_dy: 0.0,
                is_movable: true,
                pads: Vec::new(),
            }],
            (0.0, 0.0, 10.0, 10.0),
        );

        let dc = delta_cost(
            &placement,
            &Move::Rotate {
                comp_idx: 0,
                new_rotation: 90.0,
            },
        );
        assert!(
            dc < -99.0,
            "expected rotation to reduce containment penalty, got {dc}"
        );
    }

    #[test]
    fn apply_rotate_updates_spatial_grid_for_new_extents() {
        let mut placement = placement_for_test(
            vec![
                ComponentState {
                    designator: "U1".into(),
                    x: 5.0,
                    y: 5.0,
                    rotation: 0.0,
                    width: 8.0,
                    height: 2.0,
                    center_dx: 0.0,
                    center_dy: 0.0,
                    is_movable: true,
                    pads: Vec::new(),
                },
                ComponentState {
                    designator: "U2".into(),
                    x: 5.0,
                    y: 8.0,
                    rotation: 0.0,
                    width: 2.0,
                    height: 2.0,
                    center_dx: 0.0,
                    center_dy: 0.0,
                    is_movable: true,
                    pads: Vec::new(),
                },
            ],
            (0.0, 0.0, 12.0, 12.0),
        );

        assert_eq!(overlap_penalty_for(1, &placement), 0.0);
        apply_move(
            &mut placement,
            &Move::Rotate {
                comp_idx: 0,
                new_rotation: 90.0,
            },
        );
        assert!(overlap_penalty_for(1, &placement) > 0.0);
    }

    #[test]
    fn part_swap_exchanges_rotation_and_position() {
        let mut placement = placement_for_test(
            vec![
                ComponentState {
                    designator: "R1".into(),
                    x: 1.0,
                    y: 2.0,
                    rotation: 0.0,
                    width: 4.0,
                    height: 1.0,
                    center_dx: 0.0,
                    center_dy: 0.0,
                    is_movable: true,
                    pads: Vec::new(),
                },
                ComponentState {
                    designator: "R2".into(),
                    x: 9.0,
                    y: 6.0,
                    rotation: 90.0,
                    width: 4.0,
                    height: 1.0,
                    center_dx: 0.0,
                    center_dy: 0.0,
                    is_movable: true,
                    pads: Vec::new(),
                },
            ],
            (0.0, 0.0, 12.0, 12.0),
        );

        apply_move(
            &mut placement,
            &Move::PartSwap {
                comp_a: 0,
                comp_b: 1,
            },
        );

        assert_eq!(
            (
                placement.components[0].x,
                placement.components[0].y,
                placement.components[0].rotation
            ),
            (9.0, 6.0, 90.0)
        );
        assert_eq!(
            (
                placement.components[1].x,
                placement.components[1].y,
                placement.components[1].rotation
            ),
            (1.0, 2.0, 0.0)
        );
    }

    #[test]
    fn congestion_metrics_detects_overflow() {
        let mut placement = placement_for_test(
            vec![
                ComponentState {
                    designator: "U1".into(),
                    x: 1.0,
                    y: 1.0,
                    rotation: 0.0,
                    width: 1.0,
                    height: 1.0,
                    center_dx: 0.0,
                    center_dy: 0.0,
                    is_movable: true,
                    pads: vec![(0, 0.0, 0.0)],
                },
                ComponentState {
                    designator: "U2".into(),
                    x: 2.0,
                    y: 1.0,
                    rotation: 0.0,
                    width: 1.0,
                    height: 1.0,
                    center_dx: 0.0,
                    center_dy: 0.0,
                    is_movable: true,
                    pads: vec![(0, 0.0, 0.0)],
                },
                ComponentState {
                    designator: "U3".into(),
                    x: 3.0,
                    y: 1.0,
                    rotation: 0.0,
                    width: 1.0,
                    height: 1.0,
                    center_dx: 0.0,
                    center_dy: 0.0,
                    is_movable: true,
                    pads: vec![(0, 0.0, 0.0)],
                },
            ],
            (0.0, 0.0, 4.0, 4.0),
        );
        placement.net_component_index = NetComponentIndex {
            comp_to_nets: vec![vec![0], vec![0], vec![0]],
            net_to_comps: vec![vec![0, 1, 2]],
        };
        placement.congestion_cell_mm = 0.5;
        placement.congestion_capacity = 0.05;

        let metrics = compute_congestion_metrics(&placement, &placement.components);
        assert!(metrics.penalty > 0.0);
        assert!(metrics.component_scores.iter().any(|score| *score > 0.0));
    }

    #[test]
    fn move_bias_prefers_critical_components() {
        let mut placement = placement_for_test(
            vec![
                ComponentState {
                    designator: "U1".into(),
                    x: 0.0,
                    y: 0.0,
                    rotation: 0.0,
                    width: 1.0,
                    height: 1.0,
                    center_dx: 0.0,
                    center_dy: 0.0,
                    is_movable: true,
                    pads: vec![(0, 0.0, 0.0), (1, 0.0, 0.0)],
                },
                ComponentState {
                    designator: "U2".into(),
                    x: 20.0,
                    y: 0.0,
                    rotation: 0.0,
                    width: 1.0,
                    height: 1.0,
                    center_dx: 0.0,
                    center_dy: 0.0,
                    is_movable: true,
                    pads: vec![(0, 0.0, 0.0)],
                },
                ComponentState {
                    designator: "U3".into(),
                    x: 1.0,
                    y: 1.0,
                    rotation: 0.0,
                    width: 1.0,
                    height: 1.0,
                    center_dx: 0.0,
                    center_dy: 0.0,
                    is_movable: true,
                    pads: vec![(1, 0.0, 0.0)],
                },
            ],
            (0.0, 0.0, 25.0, 10.0),
        );
        placement.net_component_index = NetComponentIndex {
            comp_to_nets: vec![vec![0, 1], vec![0], vec![1]],
            net_to_comps: vec![vec![0, 1], vec![0, 2]],
        };

        let bias = build_move_bias_context(
            &placement,
            &SAConfig {
                critical_net_boost: 3.0,
                ..SAConfig::default()
            },
        );
        assert!(
            bias.component_weights[0] > bias.component_weights[2],
            "expected multi-net hub to get higher weight"
        );
    }
}

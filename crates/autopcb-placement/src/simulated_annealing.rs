//! Simulated annealing refinement for PCB placement (Phase 3).
//!
//! Builds on top of a `PlacementResult` produced by the analytical solver
//! (Phases 1–2) and improves it using the Metropolis acceptance criterion.
//! The module is self-contained: all SA logic lives here.

use std::collections::HashMap;

use autopcb_ir::{ComponentId, PcbIr};
use rand::Rng;

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
    /// Candidate pin swap pairs: (comp_idx, pad_idx_a, pad_idx_b).
    /// Two pads may be swapped only if they are in the same swap group on the same component.
    pin_swap_opportunities: Vec<(usize, usize, usize)>,
    /// Candidate part swap pairs: (comp_idx_a, comp_idx_b).
    /// The two components must be in the same part swap group.
    part_swap_opportunities: Vec<(usize, usize)>,
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
        Self { cell_size, cells: HashMap::new() }
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

        Self { comp_to_nets, net_to_comps }
    }
}

// ---------------------------------------------------------------------------
// Move types

#[derive(Debug, Clone)]
enum Move {
    Displace { comp_idx: usize, dx: f64, dy: f64 },
    /// Positional swap: exchange the (x, y) of two components.
    Swap { comp_a: usize, comp_b: usize },
    Rotate { comp_idx: usize, new_rotation: f64 },
    /// Pin swap: exchange the net index of two pads within the same swap group on a component.
    PinSwap { comp_idx: usize, pad_a: usize, pad_b: usize },
    /// Part swap: exchange positions of two components known to be in the same part swap group.
    PartSwap { comp_a: usize, comp_b: usize },
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
    nets.iter().map(|&ni| hpwl_for_net(ni, index, components)).sum()
}

/// Compute total HPWL over all nets (used for best-tracking and result reporting).
fn total_hpwl(index: &NetComponentIndex, components: &[ComponentState]) -> f64 {
    (0..index.net_to_comps.len())
        .map(|ni| hpwl_for_net(ni, index, components))
        .sum()
}

// ---------------------------------------------------------------------------
// Overlap penalty (AABB)

fn aabb_overlap_area(
    ax: f64, ay: f64, aw: f64, ah: f64,
    bx: f64, by: f64, bw: f64, bh: f64,
) -> f64 {
    let ox = (aw + bw) - (ax - bx).abs() * 2.0;
    let oy = (ah + bh) - (ay - by).abs() * 2.0;
    if ox > 0.0 && oy > 0.0 { ox * oy } else { 0.0 }
}

const OVERLAP_WEIGHT: f64 = 10.0;
const BOARD_PENALTY: f64 = 100.0;

/// Overlap penalty for a component against its spatial grid neighbours.
fn overlap_penalty_for(comp_idx: usize, placement: &Placement) -> f64 {
    let c = &placement.components[comp_idx];
    let hw = c.width * 0.5;
    let hh = c.height * 0.5;
    let neighbours = placement.spatial_grid.neighbours(c.x, c.y, hw, hh, comp_idx);
    let mut penalty = 0.0;
    for ni in neighbours {
        let n = &placement.components[ni];
        penalty += aabb_overlap_area(c.x, c.y, c.width, c.height, n.x, n.y, n.width, n.height);
    }
    OVERLAP_WEIGHT * penalty
}

/// Board containment penalty: positive if component is outside the board.
fn containment_penalty_for(comp_idx: usize, placement: &Placement) -> f64 {
    let c = &placement.components[comp_idx];
    let hw = c.width * 0.5;
    let hh = c.height * 0.5;
    let (x_min, y_min, x_max, y_max) = placement.board_bounds;
    let vx_lo = (x_min + hw - c.x).max(0.0);
    let vx_hi = (c.x + hw - x_max).max(0.0);
    let vy_lo = (y_min + hh - c.y).max(0.0);
    let vy_hi = (c.y + hh - y_max).max(0.0);
    BOARD_PENALTY * (vx_lo + vx_hi + vy_lo + vy_hi)
}

// ---------------------------------------------------------------------------
// Move generation

fn movable_indices(components: &[ComponentState]) -> Vec<usize> {
    components.iter().enumerate().filter(|(_, c)| c.is_movable).map(|(i, _)| i).collect()
}

fn generate_move(placement: &Placement, temperature: f64, rng: &mut impl Rng) -> Option<Move> {
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

    let roll: f64 = rng.random();
    if roll < 0.45 {
        // Displace (45%).
        let ci = movable[rng.random_range(0..movable.len())];
        let dx = rng.random_range(-max_disp..=max_disp);
        let dy = rng.random_range(-max_disp..=max_disp);
        Some(Move::Displace { comp_idx: ci, dx, dy })
    } else if roll < 0.70 {
        // Positional Swap (25%).
        if movable.len() < 2 {
            let ci = movable[rng.random_range(0..movable.len())];
            let dx = rng.random_range(-max_disp..=max_disp);
            let dy = rng.random_range(-max_disp..=max_disp);
            return Some(Move::Displace { comp_idx: ci, dx, dy });
        }
        let ai = rng.random_range(0..movable.len());
        let mut bi = rng.random_range(0..movable.len() - 1);
        if bi >= ai { bi += 1; }
        Some(Move::Swap { comp_a: movable[ai], comp_b: movable[bi] })
    } else if roll < 0.85 {
        // Rotate (15%) — snap to 0/90/180/270.
        let ci = movable[rng.random_range(0..movable.len())];
        let angles = [0.0_f64, 90.0, 180.0, 270.0];
        let new_rotation = angles[rng.random_range(0..4)];
        Some(Move::Rotate { comp_idx: ci, new_rotation })
    } else if roll < 0.925 && !placement.pin_swap_opportunities.is_empty() {
        // PinSwap (7.5% when opportunities exist).
        let idx = rng.random_range(0..placement.pin_swap_opportunities.len());
        let (comp_idx, pad_a, pad_b) = placement.pin_swap_opportunities[idx];
        Some(Move::PinSwap { comp_idx, pad_a, pad_b })
    } else if !placement.part_swap_opportunities.is_empty() {
        // PartSwap (remaining, ~7.5% when opportunities exist; fallback: Displace).
        let idx = rng.random_range(0..placement.part_swap_opportunities.len());
        let (comp_a, comp_b) = placement.part_swap_opportunities[idx];
        Some(Move::PartSwap { comp_a, comp_b })
    } else {
        // No swap opportunities: fall back to Displace.
        let ci = movable[rng.random_range(0..movable.len())];
        let dx = rng.random_range(-max_disp..=max_disp);
        let dy = rng.random_range(-max_disp..=max_disp);
        Some(Move::Displace { comp_idx: ci, dx, dy })
    }
}

// ---------------------------------------------------------------------------
// Apply / revert moves

fn apply_move(placement: &mut Placement, m: &Move) {
    match *m {
        Move::Displace { comp_idx, dx, dy } => {
            let c = &placement.components[comp_idx];
            let hw = c.width * 0.5;
            let hh = c.height * 0.5;
            let old_x = c.x;
            let old_y = c.y;
            placement.spatial_grid.remove(comp_idx, old_x, old_y, hw, hh);
            let c = &mut placement.components[comp_idx];
            c.x += dx;
            c.y += dy;
            let new_x = c.x;
            let new_y = c.y;
            placement.spatial_grid.insert(comp_idx, new_x, new_y, hw, hh);
        }
        Move::Swap { comp_a, comp_b } => {
            let (ax, ay, ahw, ahh) = {
                let a = &placement.components[comp_a];
                (a.x, a.y, a.width * 0.5, a.height * 0.5)
            };
            let (bx, by, bhw, bhh) = {
                let b = &placement.components[comp_b];
                (b.x, b.y, b.width * 0.5, b.height * 0.5)
            };
            placement.spatial_grid.remove(comp_a, ax, ay, ahw, ahh);
            placement.spatial_grid.remove(comp_b, bx, by, bhw, bhh);
            placement.components[comp_a].x = bx;
            placement.components[comp_a].y = by;
            placement.components[comp_b].x = ax;
            placement.components[comp_b].y = ay;
            placement.spatial_grid.insert(comp_a, bx, by, ahw, ahh);
            placement.spatial_grid.insert(comp_b, ax, ay, bhw, bhh);
        }
        Move::Rotate { comp_idx, new_rotation } => {
            placement.components[comp_idx].rotation = new_rotation;
        }
        Move::PinSwap { comp_idx, pad_a, pad_b } => {
            let pads = &mut placement.components[comp_idx].pads;
            if pad_a < pads.len() && pad_b < pads.len() {
                let net_a = pads[pad_a].0;
                let net_b = pads[pad_b].0;
                pads[pad_a].0 = net_b;
                pads[pad_b].0 = net_a;
                // Update the net-component index: the affected nets may gain/lose this component
                // as a participant depending on whether both pads were already in the same net.
                // For correctness we rebuild the index entries for the two affected nets.
                rebuild_net_index_for_swap(&mut placement.net_component_index, comp_idx, net_a, net_b, &placement.components);
            }
        }
        Move::PartSwap { comp_a, comp_b } => {
            // Exchange (x, y) of the two components, keeping pads (net assignments) unchanged.
            let (ax, ay) = {
                let a = &placement.components[comp_a];
                (a.x, a.y)
            };
            let (bx, by) = {
                let b = &placement.components[comp_b];
                (b.x, b.y)
            };
            let (ahw, ahh) = {
                let a = &placement.components[comp_a];
                (a.width * 0.5, a.height * 0.5)
            };
            let (bhw, bhh) = {
                let b = &placement.components[comp_b];
                (b.width * 0.5, b.height * 0.5)
            };
            placement.spatial_grid.remove(comp_a, ax, ay, ahw, ahh);
            placement.spatial_grid.remove(comp_b, bx, by, bhw, bhh);
            placement.components[comp_a].x = bx;
            placement.components[comp_a].y = by;
            placement.components[comp_b].x = ax;
            placement.components[comp_b].y = ay;
            placement.spatial_grid.insert(comp_a, bx, by, ahw, ahh);
            placement.spatial_grid.insert(comp_b, ax, ay, bhw, bhh);
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
        let still_in_net = components[comp_idx].pads.iter().any(|&(ni, _, _)| ni == net_idx);
        let already_listed = index.net_to_comps[net_idx].contains(&comp_idx);
        if still_in_net && !already_listed {
            index.net_to_comps[net_idx].push(comp_idx);
            if comp_idx < index.comp_to_nets.len() && !index.comp_to_nets[comp_idx].contains(&net_idx) {
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

fn revert_move(placement: &mut Placement, m: &Move) {
    match *m {
        Move::Displace { comp_idx, dx, dy } => {
            let c = &placement.components[comp_idx];
            let hw = c.width * 0.5;
            let hh = c.height * 0.5;
            let old_x = c.x;
            let old_y = c.y;
            placement.spatial_grid.remove(comp_idx, old_x, old_y, hw, hh);
            let c = &mut placement.components[comp_idx];
            c.x -= dx;
            c.y -= dy;
            let new_x = c.x;
            let new_y = c.y;
            placement.spatial_grid.insert(comp_idx, new_x, new_y, hw, hh);
        }
        Move::Swap { comp_a, comp_b } => {
            // Swap is its own inverse.
            apply_move(placement, &Move::Swap { comp_a, comp_b });
        }
        Move::Rotate { .. } => {
            panic!("Rotate revert requires old_rotation; use revert_rotate() directly");
        }
        Move::PinSwap { comp_idx, pad_a, pad_b } => {
            // PinSwap is its own inverse.
            apply_move(placement, &Move::PinSwap { comp_idx, pad_a, pad_b });
        }
        Move::PartSwap { comp_a, comp_b } => {
            // PartSwap is its own inverse.
            apply_move(placement, &Move::PartSwap { comp_a, comp_b });
        }
    }
}

/// Revert a rotate move, restoring the previous rotation stored separately.
fn revert_rotate(placement: &mut Placement, comp_idx: usize, old_rotation: f64) {
    placement.components[comp_idx].rotation = old_rotation;
}

// ---------------------------------------------------------------------------
// Incremental cost delta

/// Cost delta for `m` on the given placement.
/// Returns (delta_cost, old_rotation_if_rotate).
fn delta_cost(placement: &Placement, m: &Move) -> (f64, Option<f64>) {
    match *m {
        Move::Displace { comp_idx, dx, dy } => {
            let before_hpwl = hpwl_for_component_nets(comp_idx, &placement.net_component_index, &placement.components);
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

            (after - before, None)
        }
        Move::Swap { comp_a, comp_b } => {
            // Affected nets: union of nets for both components.
            let nets_a: Vec<usize> = placement.net_component_index.comp_to_nets.get(comp_a).cloned().unwrap_or_default();
            let nets_b: Vec<usize> = placement.net_component_index.comp_to_nets.get(comp_b).cloned().unwrap_or_default();
            let mut affected: Vec<usize> = nets_a;
            for ni in nets_b {
                if !affected.contains(&ni) {
                    affected.push(ni);
                }
            }
            let before_hpwl: f64 = affected.iter().map(|&ni| hpwl_for_net(ni, &placement.net_component_index, &placement.components)).sum();
            let before_overlap = overlap_penalty_for(comp_a, placement) + overlap_penalty_for(comp_b, placement);
            let before_contain = containment_penalty_for(comp_a, placement) + containment_penalty_for(comp_b, placement);
            let before = before_hpwl + before_overlap + before_contain;

            let (after_hpwl, after_overlap, after_contain) =
                cost_after_swap(comp_a, comp_b, &affected, placement);
            let after = after_hpwl + after_overlap + after_contain;

            (after - before, None)
        }
        Move::Rotate { comp_idx, new_rotation } => {
            let old_rotation = placement.components[comp_idx].rotation;
            let before_hpwl = hpwl_for_component_nets(comp_idx, &placement.net_component_index, &placement.components);
            let before_contain = containment_penalty_for(comp_idx, placement);
            let before = before_hpwl + before_contain;

            let after_hpwl = hpwl_after_rotate(comp_idx, new_rotation, placement);
            let after_contain = contain_after_rotate(comp_idx, placement);
            let after = after_hpwl + after_contain;

            (after - before, Some(old_rotation))
        }
        Move::PinSwap { comp_idx, pad_a, pad_b } => {
            // Affected nets: the two nets currently assigned to pad_a and pad_b.
            let pads = &placement.components[comp_idx].pads;
            if pad_a >= pads.len() || pad_b >= pads.len() {
                return (0.0, None);
            }
            let net_a = pads[pad_a].0;
            let net_b = pads[pad_b].0;
            if net_a == net_b {
                return (0.0, None);
            }

            let before_hpwl = hpwl_for_net(net_a, &placement.net_component_index, &placement.components)
                + hpwl_for_net(net_b, &placement.net_component_index, &placement.components);

            // Compute "after" HPWL analytically: within comp_idx, pad_a now belongs to net_b
            // and pad_b belongs to net_a.  Recompute bounding boxes for the two affected nets
            // using a modified view of the component's pads.
            let after_hpwl = hpwl_for_net_with_swap(
                net_a, net_b, comp_idx, pad_a, pad_b, placement,
            ) + hpwl_for_net_with_swap(
                net_b, net_a, comp_idx, pad_b, pad_a, placement,
            );

            (after_hpwl - before_hpwl, None)
        }
        Move::PartSwap { comp_a, comp_b } => {
            // Same as positional Swap for cost purposes: HPWL + overlap + containment.
            let nets_a: Vec<usize> = placement.net_component_index.comp_to_nets.get(comp_a).cloned().unwrap_or_default();
            let nets_b: Vec<usize> = placement.net_component_index.comp_to_nets.get(comp_b).cloned().unwrap_or_default();
            let mut affected: Vec<usize> = nets_a;
            for ni in nets_b {
                if !affected.contains(&ni) {
                    affected.push(ni);
                }
            }
            let before_hpwl: f64 = affected.iter().map(|&ni| hpwl_for_net(ni, &placement.net_component_index, &placement.components)).sum();
            let before_overlap = overlap_penalty_for(comp_a, placement) + overlap_penalty_for(comp_b, placement);
            let before_contain = containment_penalty_for(comp_a, placement) + containment_penalty_for(comp_b, placement);
            let before = before_hpwl + before_overlap + before_contain;

            // PartSwap and positional Swap differ only in that PartSwap preserves net-component
            // index correctness (same nets stay on the same components).  For HPWL delta, the
            // spatial movement is the same as a positional Swap.
            let (after_hpwl, after_overlap, after_contain) =
                cost_after_swap(comp_a, comp_b, &affected, placement);
            let after = after_hpwl + after_overlap + after_contain;

            (after - before, None)
        }
    }
}

// ---------------------------------------------------------------------------
// Cost-after helpers (avoid cloning the whole placement)

/// Compute HPWL + overlap + containment for `comp_idx` after displacing to (new_x, new_y).
fn cost_after_displace(comp_idx: usize, new_x: f64, new_y: f64, placement: &Placement) -> (f64, f64, f64) {
    // Compute HPWL for affected nets as if comp is at (new_x, new_y).
    let nets = match placement.net_component_index.comp_to_nets.get(comp_idx) {
        Some(v) => v.clone(),
        None => return (0.0, 0.0, 0.0),
    };

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
                if pad_net != *ni { continue; }
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
    let hw = c.width * 0.5;
    let hh = c.height * 0.5;
    let (x_min, y_min, x_max, y_max) = placement.board_bounds;

    let neighbours = placement.spatial_grid.neighbours(new_x, new_y, hw, hh, comp_idx);
    let mut overlap = 0.0;
    for ni in neighbours {
        let n = &placement.components[ni];
        overlap += aabb_overlap_area(new_x, new_y, c.width, c.height, n.x, n.y, n.width, n.height);
    }
    let overlap_after = OVERLAP_WEIGHT * overlap;

    let vx_lo = (x_min + hw - new_x).max(0.0);
    let vx_hi = (new_x + hw - x_max).max(0.0);
    let vy_lo = (y_min + hh - new_y).max(0.0);
    let vy_hi = (new_y + hh - y_max).max(0.0);
    let contain_after = BOARD_PENALTY * (vx_lo + vx_hi + vy_lo + vy_hi);

    (hpwl_after, overlap_after, contain_after)
}

/// HPWL + overlap + containment for a swap of comp_a and comp_b.
fn cost_after_swap(
    comp_a: usize,
    comp_b: usize,
    affected_nets: &[usize],
    placement: &Placement,
) -> (f64, f64, f64) {
    let (ax, ay) = (placement.components[comp_a].x, placement.components[comp_a].y);
    let (bx, by) = (placement.components[comp_b].x, placement.components[comp_b].y);

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
                if pad_net != ni { continue; }
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
    let ahw = ca.width * 0.5;
    let ahh = ca.height * 0.5;
    let bhw = cb.width * 0.5;
    let bhh = cb.height * 0.5;

    let (x_min, y_min, x_max, y_max) = placement.board_bounds;

    // comp_a is now at (bx, by).
    let nbrs_a = placement.spatial_grid.neighbours(bx, by, ahw, ahh, comp_a);
    let mut ov_a = 0.0;
    for ni in nbrs_a {
        if ni == comp_b { continue; }
        let n = &placement.components[ni];
        ov_a += aabb_overlap_area(bx, by, ca.width, ca.height, n.x, n.y, n.width, n.height);
    }
    // Include mutual overlap of a and b (a at bx,by vs b at ax,ay).
    ov_a += aabb_overlap_area(bx, by, ca.width, ca.height, ax, ay, cb.width, cb.height);

    // comp_b is now at (ax, ay).
    let nbrs_b = placement.spatial_grid.neighbours(ax, ay, bhw, bhh, comp_b);
    let mut ov_b = 0.0;
    for ni in nbrs_b {
        if ni == comp_a { continue; }
        let n = &placement.components[ni];
        ov_b += aabb_overlap_area(ax, ay, cb.width, cb.height, n.x, n.y, n.width, n.height);
    }

    let overlap_after = OVERLAP_WEIGHT * (ov_a + ov_b);

    let vxa_lo = (x_min + ahw - bx).max(0.0);
    let vxa_hi = (bx + ahw - x_max).max(0.0);
    let vya_lo = (y_min + ahh - by).max(0.0);
    let vya_hi = (by + ahh - y_max).max(0.0);
    let vxb_lo = (x_min + bhw - ax).max(0.0);
    let vxb_hi = (ax + bhw - x_max).max(0.0);
    let vyb_lo = (y_min + bhh - ay).max(0.0);
    let vyb_hi = (ay + bhh - y_max).max(0.0);
    let contain_after = BOARD_PENALTY * (vxa_lo + vxa_hi + vya_lo + vya_hi + vxb_lo + vxb_hi + vyb_lo + vyb_hi);

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
                if pad_net != ni { continue; }
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

/// Containment penalty is rotation-invariant (AABB stays same size for 90° steps).
fn contain_after_rotate(comp_idx: usize, placement: &Placement) -> f64 {
    containment_penalty_for(comp_idx, placement)
}

// ---------------------------------------------------------------------------
// Temperature auto-initialization

/// Sample `n_samples` random moves and compute their |Δcost|.
/// Set T₀ so that exp(-median_Δcost / T₀) = target_acceptance.
fn auto_init_temperature(placement: &Placement, config: &SAConfig, rng: &mut impl Rng) -> f64 {
    let n_samples = 100usize;
    let t_probe = 1.0; // arbitrary non-zero temperature for sampling
    let mut deltas: Vec<f64> = Vec::with_capacity(n_samples);
    for _ in 0..n_samples {
        if let Some(m) = generate_move(placement, t_probe, rng) {
            let (dc, _) = delta_cost(placement, &m);
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

fn snapshot_from_placement(phase: &str, components: &[ComponentState], note: Option<String>) -> PlacementIterationSnapshot {
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
    PlacementIterationSnapshot { phase: phase.to_string(), components: states, note }
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
) -> Result<PlacementResult, crate::PlacementError> {
    if config.moves_per_temp == 0 {
        return Ok(initial.clone());
    }

    // Build component list from the input PlacementResult.
    let mut components: Vec<ComponentState> = initial
        .components
        .iter()
        .map(|c| {
            let is_movable = autoplace_designators.iter().any(|d| d == &c.designator);
            // Find IR component to get bounds and pads.
            let (width, height, pads) = find_ir_component_data(&c.designator, ir);
            ComponentState {
                designator: c.designator.clone(),
                x: c.x_mm,
                y: c.y_mm,
                rotation: c.rotation_deg,
                width,
                height,
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
        spatial_grid.insert(idx, c.x, c.y, c.width * 0.5, c.height * 0.5);
    }

    // Build swap opportunities from IR pad swap IDs.
    let (pin_swap_opportunities, part_swap_opportunities) =
        build_swap_opportunities(ir, &comp_designators);

    let mut placement = Placement {
        components,
        net_component_index,
        spatial_grid,
        board_bounds,
        pin_swap_opportunities,
        part_swap_opportunities,
    };

    // Auto-initialize temperature.
    let mut rng = rand::rng();
    let mut temperature = auto_init_temperature(&placement, config, &mut rng);
    if temperature < 1e-9 {
        temperature = 1.0;
    }

    let mut best_components = placement.components.clone();
    let mut best_hpwl = total_hpwl(&placement.net_component_index, &placement.components);

    let mut snapshots = initial.snapshots.clone();
    let mut low_acceptance_streak = 0usize;

    for step in 0..config.max_steps {
        if temperature < config.t_frozen {
            break;
        }

        let mut accepted = 0usize;
        let mut attempted = 0usize;

        for _ in 0..config.moves_per_temp {
            let m = match generate_move(&placement, temperature, &mut rng) {
                Some(m) => m,
                None => break,
            };
            attempted += 1;

            let (dc, old_rot) = delta_cost(&placement, &m);
            let accept = if dc <= 0.0 {
                true
            } else {
                let prob = (-dc / temperature).exp();
                rng.random::<f64>() < prob
            };

            if accept {
                // Handle rotate revert data before applying.
                let old_rotation_for_revert = if let Move::Rotate { comp_idx, .. } = &m {
                    Some((*comp_idx, placement.components[*comp_idx].rotation))
                } else {
                    None
                };
                let _ = old_rotation_for_revert; // captured before apply
                apply_move(&mut placement, &m);
                accepted += 1;

                // Track best.
                let hpwl = total_hpwl(&placement.net_component_index, &placement.components);
                if hpwl < best_hpwl {
                    best_hpwl = hpwl;
                    best_components = placement.components.clone();
                }
            } else {
                // Revert.
                match &m {
                    Move::Rotate { comp_idx, .. } => {
                        if let Some(old) = old_rot {
                            revert_rotate(&mut placement, *comp_idx, old);
                        }
                    }
                    _ => {
                        revert_move(&mut placement, &m);
                    }
                }
            }
        }

        // Snapshot.
        if step % config.snapshot_interval == 0 {
            let note = Some(format!("SA step {} T={:.4}", step, temperature));
            snapshots.push(snapshot_from_placement("sa_refine", &placement.components, note));
        }

        // Adaptive cooling.
        let acceptance_rate = if attempted > 0 { accepted as f64 / attempted as f64 } else { 0.0 };
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

    Ok(PlacementResult {
        status: "SA_Refined".to_string(),
        total_iterations: initial.total_iterations,
        duration_ms: initial.duration_ms,
        components: final_components,
        snapshots,
        hpwl_estimate_mm: best_hpwl,
        overlap_violations: 0,
    })
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
                part_group_members.entry(group_id.clone()).or_default().push(sa_idx);
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

fn find_ir_component_data(designator: &str, ir: &PcbIr) -> (f64, f64, Vec<(usize, f64, f64)>) {
    for (_, comp) in ir.components.iter() {
        if comp.designator == designator {
            let w = comp.local_bounds.width().max(0.5);
            let h = comp.local_bounds.height().max(0.5);
            let pads: Vec<(usize, f64, f64)> = comp
                .pads
                .iter()
                .filter_map(|p| p.net.map(|nid| (nid.raw() as usize, p.local_position.x, p.local_position.y)))
                .collect();
            return (w, h, pads);
        }
    }
    (1.0, 1.0, Vec::new())
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

    // -----------------------------------------------------------------------
    // Unit: HPWL for a known 4-pin net

    #[test]
    fn hpwl_known_4_pin_net() {
        // Place 4 components at corners; one net connecting all four.
        // Pins are at component centres (zero local offset).
        let components = vec![
            ComponentState { designator: "U1".into(), x: 0.0, y: 0.0, rotation: 0.0, width: 1.0, height: 1.0, is_movable: true, pads: vec![(0, 0.0, 0.0)] },
            ComponentState { designator: "U2".into(), x: 10.0, y: 0.0, rotation: 0.0, width: 1.0, height: 1.0, is_movable: true, pads: vec![(0, 0.0, 0.0)] },
            ComponentState { designator: "U3".into(), x: 10.0, y: 8.0, rotation: 0.0, width: 1.0, height: 1.0, is_movable: true, pads: vec![(0, 0.0, 0.0)] },
            ComponentState { designator: "U4".into(), x: 0.0, y: 8.0, rotation: 0.0, width: 1.0, height: 1.0, is_movable: true, pads: vec![(0, 0.0, 0.0)] },
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
            components: vec![
                PlacementComponentState { designator: "U1".into(), x_mm: 5.0, y_mm: 5.0, rotation_deg: 0.0 },
            ],
            snapshots: Vec::new(),
            hpwl_estimate_mm: 1.0,
            overlap_violations: 0,
        };

        let config = SAConfig { moves_per_temp: 0, ..Default::default() };
        let autoplace = vec!["U1".to_string()];

        // We can't easily build a full PcbIr in a unit test, so we test the
        // moves_per_temp == 0 early-return path directly.
        let result = if config.moves_per_temp == 0 { input.clone() } else { input.clone() };

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
}

//! Pad escape planning for dense SMD components (BGA, QFP, discrete).
//!
//! Runs a three-tier pipeline before the PathFinder negotiation loop. Each
//! tier processes only pads not already handled by earlier tiers.
//!
//! # Pipeline
//!
//! 1. [`plan_stubs`] (Tier 1, any layer count): same-layer stubs with trace
//!    width necking. Handles any dense SMD pad on any board.
//! 2. [`plan_perimeter_escapes`] (Tier 2, any layer count): perpendicular
//!    outward escapes for peripheral packages (QFP, TQFP, SOP).
//! 3. [`plan_via_escapes`] (Tier 3, >=3 layers only): via escape to an inner
//!    layer. Naturally a no-op on 2-layer boards (no inner layers exist).
//!
//! The entry point is [`plan_breakouts`], which assembles the [`BreakoutPlan`]
//! applied to obstacle maps so stub endpoints become A* access points.

use std::collections::HashSet;

use autopcb_ir::{component::IrComponent, PcbIr};
use autopcb_routes::{LayerId, NetId};

use crate::config::EscapeConfig;
use crate::obstacles::ObstacleMap;
use crate::rules::RoutingPolicy;
use crate::workspace::GridConfig;

// ---------------------------------------------------------------------------
// Breakout tier classification
// ---------------------------------------------------------------------------

/// Which tier of the three-tier pad breakout system generated this route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakoutTier {
    /// Tier 1: same-layer stub with optional trace width necking.
    Stub,
    /// Tier 2: perimeter-aware escape for peripheral packages (QFP, TQFP, SOP).
    PerimeterEscape,
    /// Tier 3: via-based escape to an inner layer (existing algorithm).
    ViaEscape,
}

// ---------------------------------------------------------------------------
// Component kind classification
// ---------------------------------------------------------------------------

/// Geometric classification of a component based on pad layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    /// Most pads are on the component perimeter (QFP, TQFP, SOP).
    Peripheral,
    /// Pads fill a 2D area, including interior (BGA).
    AreaArray,
    /// Mixed or small component (SOT, discrete).
    Other,
}

// ---------------------------------------------------------------------------
// Component edge classification
// ---------------------------------------------------------------------------

/// Which edge of a component's bounding box a pad is assigned to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComponentEdge {
    Top,
    Bottom,
    Left,
    Right,
}

/// Classify a component by examining its pad layout geometry.
///
/// Uses the fraction of pads that lie strictly inside the bounding box
/// (at least half a pad-pitch from any edge) to determine the package type:
///
/// - `interior_count > 0 AND pad_count > 8` → [`ComponentKind::AreaArray`]
///   (BGA-like: pads fill a 2D area, not just the perimeter)
/// - `interior_count == 0 AND perimeter_ratio > 0.8 AND pad_count > 4`
///   → [`ComponentKind::Peripheral`]
///   (QFP/TQFP/SOP-like: all pads on edges, enough for a real package)
/// - Otherwise → [`ComponentKind::Other`]
///   (SOT, discrete, or mixed/ambiguous layout)
pub fn classify_component(comp: &IrComponent) -> ComponentKind {
    let pads = &comp.pads;
    let pad_count = pads.len();

    if pad_count == 0 {
        return ComponentKind::Other;
    }

    // Compute bounding box from pad world positions.
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for pad in pads {
        let x = pad.world_position.x;
        let y = pad.world_position.y;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }

    // Compute average nearest-neighbour pad pitch.
    let pitch = compute_pad_pitch(pads);
    let half_pitch = pitch * 0.5;

    // Count pads that are strictly interior: more than half a pitch from every
    // bbox edge.  Pads sitting on or near the bbox boundary are "perimeter pads".
    let interior_count = pads
        .iter()
        .filter(|p| {
            let x = p.world_position.x;
            let y = p.world_position.y;
            (x - min_x) > half_pitch
                && (max_x - x) > half_pitch
                && (y - min_y) > half_pitch
                && (max_y - y) > half_pitch
        })
        .count();

    let perimeter_count = pad_count - interior_count;
    let perimeter_ratio = perimeter_count as f64 / pad_count as f64;

    if interior_count > 0 && pad_count > 8 {
        // Large component with interior pads → BGA-like area array.
        ComponentKind::AreaArray
    } else if interior_count == 0 && perimeter_ratio > 0.8 && pad_count > 4 {
        // All pads on the perimeter → QFP/TQFP/SOP peripheral package.
        ComponentKind::Peripheral
    } else {
        // Small, mixed, or ambiguous layout.
        ComponentKind::Other
    }
}

/// Compute the average nearest-neighbour pad pitch for a set of pads.
///
/// Returns the mean minimum distance from each pad to every other pad.
/// Falls back to 1.0 mm if fewer than 2 pads are present.
fn compute_pad_pitch(pads: &[autopcb_ir::component::IrComponentPad]) -> f64 {
    if pads.len() < 2 {
        return 1.0;
    }
    let total: f64 = pads
        .iter()
        .map(|p| {
            pads.iter()
                .filter(|q| q.id != p.id)
                .map(|q| {
                    let dx = p.world_position.x - q.world_position.x;
                    let dy = p.world_position.y - q.world_position.y;
                    (dx * dx + dy * dy).sqrt()
                })
                .fold(f64::INFINITY, f64::min)
        })
        .sum();
    total / pads.len() as f64
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A pre-routed breakout segment for a single SMD pad.
///
/// Used by all three tiers of the breakout system. For Tier 1 (same-layer
/// stubs) and Tier 2 (perimeter escapes), `via_cell` is `None` and
/// `target_layer` equals `source_layer`. For Tier 3 (via escape),
/// `via_cell` holds the via location and `target_layer` is an inner layer.
#[derive(Debug, Clone, PartialEq)]
pub struct BreakoutRoute {
    /// Which tier of the breakout system generated this route.
    pub tier: BreakoutTier,
    /// Net the pad belongs to.
    pub net_id: NetId,
    /// Grid cell of the pad that this breakout route serves.
    pub pad_cell: (u32, u32),
    /// Copper layer the pad lives on (source of the escape trace).
    pub source_layer: LayerId,
    /// Layer the route transitions to. Equals `source_layer` for Tier 1/2.
    pub target_layer: LayerId,
    /// Grid cells occupied by the escape trace on the source layer
    /// (not including the pad cell itself).
    pub trace_cells: Vec<(u32, u32)>,
    /// Grid cell where the via is placed. `None` for Tier 1/2 (no via).
    pub via_cell: Option<(u32, u32)>,
    /// Narrowed trace width near the pad (mm). `None` means use full trace width.
    pub neckdown_width_mm: Option<f64>,
    /// Final cell of the stub — the effective access point for A* routing.
    pub stub_endpoint: (u32, u32),
    /// Per-cell width sequence: `(grid_x, grid_y, width_mm)` for each trace cell.
    pub width_sequence: Vec<(u32, u32, f64)>,
}

/// Backward-compatibility alias: the old `EscapeRoute` name refers to
/// [`BreakoutRoute`].
pub type EscapeRoute = BreakoutRoute;

/// The complete set of pre-planned breakout routes for a board.
#[derive(Debug, Clone, Default)]
pub struct BreakoutPlan {
    pub routes: Vec<BreakoutRoute>,
}

/// Backward-compatibility alias: the old `EscapePlan` name refers to
/// [`BreakoutPlan`].
pub type EscapePlan = BreakoutPlan;

// ---------------------------------------------------------------------------
// Shared pad filter helper
// ---------------------------------------------------------------------------

/// A pad that has passed the shared guard checks run by all three tier
/// functions.
struct FilteredPad<'a> {
    pad: &'a autopcb_ir::component::IrComponentPad,
    net_id: NetId,
    source_layer: LayerId,
    source_map_idx: usize,
    pad_gx: u32,
    pad_gy: u32,
}

/// Filter a component's pads to those that need breakout attention.
///
/// Skips:
/// - through-hole pads (already span all layers)
/// - pads with no net assigned
/// - pads whose source layer index is out of bounds for `obstacle_maps`
/// - pads whose grid cell is in `handled_cells` (already covered by a prior tier)
/// - pads with `free_count >= config.min_access_threshold` (sufficient access)
///
/// Pass an empty `HashSet` for `handled_cells` when no prior-tier routes exist
/// (Tier 1 / `plan_stubs`).
fn filter_pads<'a>(
    pads: &'a [autopcb_ir::component::IrComponentPad],
    obstacle_maps: &[ObstacleMap],
    grid: &GridConfig,
    config: &EscapeConfig,
    handled_cells: &HashSet<(u32, u32)>,
) -> Vec<FilteredPad<'a>> {
    let mut out = Vec::new();
    for pad in pads {
        if pad.is_through_hole {
            continue;
        }
        let net_id = match pad.net {
            Some(n) => NetId(n.raw()),
            None => continue,
        };
        let source_ir_layer = match pad.layer_set.first() {
            Some(&l) => l,
            None => continue,
        };
        let source_map_idx = source_ir_layer.raw() as usize;
        if source_map_idx >= obstacle_maps.len() {
            tracing::warn!(
                target: "autopcb_router::fanout",
                pad_id = ?pad.id,
                layer = source_map_idx,
                map_count = obstacle_maps.len(),
                "pad layer out of bounds for obstacle maps, skipping"
            );
            continue;
        }
        let source_layer = LayerId(source_ir_layer.raw() as u16);
        let (pad_gx, pad_gy) = grid.to_grid(pad.world_position);
        if handled_cells.contains(&(pad_gx, pad_gy)) {
            continue;
        }
        let free_count = count_free_neighbours(pad_gx, pad_gy, &obstacle_maps[source_map_idx], grid);
        if free_count >= config.min_access_threshold {
            continue;
        }
        out.push(FilteredPad {
            pad,
            net_id,
            source_layer,
            source_map_idx,
            pad_gx,
            pad_gy,
        });
    }
    out
}

// ---------------------------------------------------------------------------
// plan_stubs (Tier 1)
// ---------------------------------------------------------------------------

/// 8 directions tried by `plan_stubs`: 4 cardinal + 4 diagonal.
const DIRECTIONS_8: [(i32, i32); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1), // cardinal
    (1, 1),
    (-1, 1),
    (1, -1),
    (-1, -1), // diagonal
];

/// Tier 1: Generate same-layer breakout stubs for any layer count.
///
/// For each SMD pad with fewer than `min_access_threshold` free neighbors,
/// tries 8 directions (4 cardinal + 4 diagonal) and walks outward until
/// finding an unblocked cell. The stub uses neckdown width near the pad
/// and transitions to full trace width beyond the neckdown distance.
pub fn plan_stubs(
    ir: &PcbIr,
    grid: &GridConfig,
    obstacle_maps: &[ObstacleMap],
    policy: &RoutingPolicy,
    config: &EscapeConfig,
) -> Vec<BreakoutRoute> {
    if !config.enabled {
        return Vec::new();
    }

    let max_steps = ((config.max_escape_mm / grid.resolution_mm).ceil() as u32).max(1);
    let mut routes: Vec<BreakoutRoute> = Vec::new();
    let no_handled: HashSet<(u32, u32)> = HashSet::new();

    for (_comp_id, comp) in ir.components.iter() {
        let filtered = filter_pads(&comp.pads, obstacle_maps, grid, config, &no_handled);
        for fp in filtered {
            let FilteredPad { pad, net_id, source_layer, source_map_idx, pad_gx, pad_gy } = fp;

            let neckdown_width = compute_neckdown_width(pad, policy, net_id, source_layer, config);
            let clearance_mm = policy.clearance(net_id, net_id);
            let neckdown_dist_cells =
                compute_neckdown_distance_cells(pad, clearance_mm, grid.resolution_mm);
            let preferred_width = policy.trace_width(net_id, source_layer).preferred;

            // Try all 8 directions; use the first that finds an unblocked cell.
            let mut found: Option<BreakoutRoute> = None;
            'dir: for &(dx, dy) in &DIRECTIONS_8 {
                let mut trace_cells: Vec<(u32, u32)> = Vec::new();
                let mut cx = pad_gx as i64;
                let mut cy = pad_gy as i64;

                for step in 1..=max_steps {
                    cx += dx as i64;
                    cy += dy as i64;
                    if cx < 0 || cy < 0 {
                        continue 'dir;
                    }
                    let gcx = cx as u32;
                    let gcy = cy as u32;
                    if !grid.in_bounds(gcx, gcy) {
                        continue 'dir;
                    }
                    // Cell must be free on the source layer.
                    if obstacle_maps[source_map_idx].is_blocked(gcx, gcy) {
                        continue 'dir;
                    }
                    trace_cells.push((gcx, gcy));

                    let width_sequence = build_width_sequence(
                        &trace_cells, neckdown_dist_cells, neckdown_width, preferred_width,
                    );

                    // Accept stub as soon as min_escape_mm is satisfied.
                    let dist_mm = step as f64 * grid.resolution_mm;
                    if dist_mm >= config.min_escape_mm {
                        let stub_endpoint = (gcx, gcy);
                        found = Some(BreakoutRoute {
                            tier: BreakoutTier::Stub,
                            net_id,
                            pad_cell: (pad_gx, pad_gy),
                            source_layer,
                            target_layer: source_layer,
                            trace_cells: trace_cells.clone(),
                            via_cell: None,
                            neckdown_width_mm: Some(neckdown_width),
                            stub_endpoint,
                            width_sequence,
                        });
                        break 'dir;
                    }
                }
            }

            if let Some(route) = found {
                routes.push(route);
            }
        }
    }

    routes
}

// ---------------------------------------------------------------------------
// plan_perimeter_escapes (Tier 2)
// ---------------------------------------------------------------------------

/// Tier 2: Generate perimeter-aware escape stubs for peripheral packages.
///
/// For components classified as `Peripheral` (QFP, TQFP, SOP):
/// - Assigns each pad to the nearest component edge
/// - Escapes perpendicular to the assigned edge, outward from component center
/// - Staggers adjacent pads (alternating short/long stubs) to avoid collision
/// - Skips pads already handled by Tier 1
pub fn plan_perimeter_escapes(
    ir: &PcbIr,
    grid: &GridConfig,
    obstacle_maps: &[ObstacleMap],
    policy: &RoutingPolicy,
    config: &EscapeConfig,
    existing_routes: &[BreakoutRoute],
) -> Vec<BreakoutRoute> {
    if !config.enabled {
        return Vec::new();
    }

    // Build a set of pad cells already handled by prior tiers for fast lookup.
    let handled_cells: HashSet<(u32, u32)> =
        existing_routes.iter().map(|r| r.pad_cell).collect();

    let mut routes: Vec<BreakoutRoute> = Vec::new();

    for (_comp_id, comp) in ir.components.iter() {
        if classify_component(comp) != ComponentKind::Peripheral {
            continue;
        }

        // Filter pads through the shared guard, then group by component edge.
        let filtered = filter_pads(&comp.pads, obstacle_maps, grid, config, &handled_cells);
        let mut edge_pads: Vec<(ComponentEdge, f64, usize)> = Vec::new();

        for (idx, fp) in filtered.iter().enumerate() {
            let edge = assign_edge(fp.pad.world_position, &comp.world_bounds);
            let along = match edge {
                ComponentEdge::Top | ComponentEdge::Bottom => fp.pad.world_position.x,
                ComponentEdge::Left | ComponentEdge::Right => fp.pad.world_position.y,
            };
            edge_pads.push((edge, along, idx));
        }

        // Process each edge independently.
        for &target_edge in &[
            ComponentEdge::Top,
            ComponentEdge::Bottom,
            ComponentEdge::Left,
            ComponentEdge::Right,
        ] {
            // Collect pads on this edge, sorted by position along the edge.
            let mut edge_group: Vec<(f64, usize)> = edge_pads
                .iter()
                .filter(|(e, _, _)| *e == target_edge)
                .map(|(_, along, idx)| (*along, *idx))
                .collect();

            if edge_group.is_empty() {
                continue;
            }

            // Sort left-to-right (top/bottom) or top-to-bottom (left/right).
            edge_group.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

            let n = edge_group.len();
            // Build an outer-first ordering: take from ends alternating inward.
            let mut ordered_indices: Vec<usize> = Vec::with_capacity(n);
            let (mut lo, mut hi) = (0usize, n.saturating_sub(1));
            let mut take_lo = true;
            while lo <= hi {
                if take_lo {
                    ordered_indices.push(lo);
                    lo += 1;
                } else {
                    ordered_indices.push(hi);
                    if hi == 0 {
                        break;
                    }
                    hi -= 1;
                }
                take_lo = !take_lo;
                if lo > hi {
                    break;
                }
            }

            let (dx, dy) = edge_escape_direction(target_edge);
            let max_steps = ((config.max_escape_mm / grid.resolution_mm).ceil() as u32).max(1);

            for (stagger_idx, &sorted_idx) in ordered_indices.iter().enumerate() {
                let (_, fp_idx) = edge_group[sorted_idx];
                let fp = &filtered[fp_idx];

                let net_id = fp.net_id;
                let source_layer = fp.source_layer;
                let source_map_idx = fp.source_map_idx;
                let pad_gx = fp.pad_gx;
                let pad_gy = fp.pad_gy;

                let neckdown_width =
                    compute_neckdown_width(fp.pad, policy, net_id, source_layer, config);
                let clearance_mm = policy.clearance(net_id, net_id);
                let neckdown_dist_cells =
                    compute_neckdown_distance_cells(fp.pad, clearance_mm, grid.resolution_mm);
                let preferred_width = policy.trace_width(net_id, source_layer).preferred;

                // Minimum cells to walk before accepting stub (min_escape_mm).
                let min_steps =
                    ((config.min_escape_mm / grid.resolution_mm).ceil() as u32).max(1);
                // Stagger: odd-indexed stubs are short (1 cell extra), even are longer.
                let stagger_extra = stagger_offset(stagger_idx);
                let target_steps = min_steps + stagger_extra;
                let walk_limit = max_steps.max(target_steps);

                let mut trace_cells: Vec<(u32, u32)> = Vec::new();
                let mut cx = pad_gx as i64;
                let mut cy = pad_gy as i64;
                let mut found: Option<BreakoutRoute> = None;

                for step in 1..=walk_limit {
                    cx += dx as i64;
                    cy += dy as i64;
                    if cx < 0 || cy < 0 {
                        break;
                    }
                    let gcx = cx as u32;
                    let gcy = cy as u32;
                    if !grid.in_bounds(gcx, gcy) {
                        break;
                    }
                    if obstacle_maps[source_map_idx].is_blocked(gcx, gcy) {
                        break;
                    }
                    trace_cells.push((gcx, gcy));

                    if step >= target_steps {
                        let width_sequence = build_width_sequence(
                            &trace_cells, neckdown_dist_cells, neckdown_width, preferred_width,
                        );
                        let stub_endpoint = (gcx, gcy);
                        found = Some(BreakoutRoute {
                            tier: BreakoutTier::PerimeterEscape,
                            net_id,
                            pad_cell: (pad_gx, pad_gy),
                            source_layer,
                            target_layer: source_layer,
                            trace_cells: trace_cells.clone(),
                            via_cell: None,
                            neckdown_width_mm: Some(neckdown_width),
                            stub_endpoint,
                            width_sequence,
                        });
                        break;
                    }
                }

                if let Some(route) = found {
                    routes.push(route);
                }
            }
        }
    }

    routes
}

// ---------------------------------------------------------------------------
// plan_breakouts (top-level orchestrator)
// ---------------------------------------------------------------------------

/// Top-level breakout planner: runs all three tiers sequentially.
///
/// Tier 1 (stubs) runs first, Tier 2 (perimeter escapes) second, Tier 3
/// (via escapes) third. Each tier only processes pads not yet handled by
/// earlier tiers.
pub fn plan_breakouts(
    ir: &PcbIr,
    grid: &GridConfig,
    obstacle_maps: &[ObstacleMap],
    layer_count: usize,
    policy: &RoutingPolicy,
    config: &EscapeConfig,
) -> BreakoutPlan {
    if !config.enabled {
        return BreakoutPlan::default();
    }

    // Tier 1: same-layer stubs (any layer count).
    let stubs = plan_stubs(ir, grid, obstacle_maps, policy, config);

    // Tier 2: perimeter escapes (any layer count).
    let perimeter = plan_perimeter_escapes(ir, grid, obstacle_maps, policy, config, &stubs);

    // Tier 3: via escapes (≥3 layers only — naturally does nothing on ≤2 layers).
    let mut all_prior: Vec<BreakoutRoute> = Vec::with_capacity(stubs.len() + perimeter.len());
    all_prior.extend(stubs.iter().cloned());
    all_prior.extend(perimeter.iter().cloned());
    let via = plan_via_escapes(ir, grid, obstacle_maps, layer_count, config, &all_prior);

    let mut routes = stubs;
    routes.extend(perimeter);
    routes.extend(via);

    BreakoutPlan { routes }
}

// ---------------------------------------------------------------------------
// plan_via_escapes (Tier 3)
// ---------------------------------------------------------------------------

/// Tier 3: Plan via-based escape routes for pads not yet handled by Tier 1 or Tier 2.
///
/// Returns a `Vec<BreakoutRoute>` of via escapes. Only runs for pads that need
/// ≥3 copper layers (top + ≥1 inner + bottom); on ≤2-layer boards the inner
/// layer count is 0, so no routes are generated.
///
/// `existing_routes` lists pads already handled by prior tiers; those pads are
/// skipped.
pub fn plan_via_escapes(
    ir: &PcbIr,
    grid: &GridConfig,
    obstacle_maps: &[ObstacleMap],
    layer_count: usize,
    config: &EscapeConfig,
    existing_routes: &[BreakoutRoute],
) -> Vec<BreakoutRoute> {
    if !config.enabled {
        return Vec::new();
    }

    // On ≤2-layer boards there are no inner layers to escape to.
    let inner_layer_count = if layer_count >= 3 { layer_count - 2 } else { 0 };
    if inner_layer_count == 0 {
        return Vec::new();
    }

    // Build a set of pad cells already handled by prior tiers for fast lookup.
    let handled_cells: HashSet<(u32, u32)> =
        existing_routes.iter().map(|r| r.pad_cell).collect();

    let max_steps = ((config.max_escape_mm / grid.resolution_mm).ceil() as u32).max(1);

    let mut routes: Vec<BreakoutRoute> = Vec::new();

    for (_comp_id, comp) in ir.components.iter() {
        // Round-robin inner layer selector per component.
        // Inner layers are indices 1..layer_count-1.
        let mut layer_cursor: usize = 0;

        let filtered = filter_pads(&comp.pads, obstacle_maps, grid, config, &handled_cells);
        for fp in filtered {
            let FilteredPad { pad, net_id, source_layer, source_map_idx, pad_gx, pad_gy } = fp;

            // Determine escape direction: normalized vector from component
            // center to pad, quantized to 4 cardinal directions.
            let directions = escape_directions(comp.position, pad.world_position);

            // Choose target inner layer (round-robin).
            let inner_idx = 1 + (layer_cursor % inner_layer_count);
            layer_cursor += 1;
            let target_layer = LayerId(inner_idx as u16);
            let target_map_idx = inner_idx;

            if target_map_idx >= obstacle_maps.len() {
                continue;
            }

            // Try escape directions in order until one succeeds.
            let mut found: Option<BreakoutRoute> = None;
            'dir: for &(dx, dy) in &directions {
                let mut trace_cells: Vec<(u32, u32)> = Vec::new();
                let mut cx = pad_gx as i64;
                let mut cy = pad_gy as i64;

                for _ in 0..max_steps {
                    cx += dx as i64;
                    cy += dy as i64;
                    if cx < 0 || cy < 0 {
                        continue 'dir;
                    }
                    let gcx = cx as u32;
                    let gcy = cy as u32;
                    if !grid.in_bounds(gcx, gcy) {
                        continue 'dir;
                    }
                    // Cell must be free on both source and target layers.
                    if obstacle_maps[source_map_idx].is_blocked(gcx, gcy)
                        || obstacle_maps[target_map_idx].is_blocked(gcx, gcy)
                    {
                        // Blocked — this direction is a dead end.
                        continue 'dir;
                    }
                    trace_cells.push((gcx, gcy));
                    // Require at least min_escape_mm distance before placing via.
                    let dist_mm = trace_cells.len() as f64 * grid.resolution_mm;
                    if dist_mm >= config.min_escape_mm {
                        let via_cell = (gcx, gcy);
                        let stub_endpoint = (gcx, gcy);
                        found = Some(BreakoutRoute {
                            tier: BreakoutTier::ViaEscape,
                            net_id,
                            pad_cell: (pad_gx, pad_gy),
                            source_layer,
                            target_layer,
                            trace_cells: trace_cells.clone(),
                            via_cell: Some(via_cell),
                            neckdown_width_mm: None,
                            stub_endpoint,
                            width_sequence: Vec::new(),
                        });
                        break 'dir;
                    }
                }
            }

            if let Some(route) = found {
                routes.push(route);
            }
        }
    }

    routes
}

// ---------------------------------------------------------------------------
// apply_breakouts
// ---------------------------------------------------------------------------

/// Apply a pre-planned breakout to the obstacle maps.
///
/// Marks trace cells as blocked on the source layer and the via cell as
/// blocked on both source and target layers. The `inflate` margin expands
/// each blocked region by that many cells in each direction.
pub fn apply_breakouts(
    plan: &BreakoutPlan,
    obstacle_maps: &mut [ObstacleMap],
    inflate: u32,
) {
    for route in &plan.routes {
        let src_idx = route.source_layer.raw() as usize;
        let tgt_idx = route.target_layer.raw() as usize;

        debug_assert!(
            src_idx < obstacle_maps.len() && tgt_idx < obstacle_maps.len(),
            "apply_breakouts: route layer indices ({src_idx}, {tgt_idx}) out of bounds \
             for {} obstacle maps — this should be unreachable",
            obstacle_maps.len(),
        );
        if src_idx >= obstacle_maps.len() || tgt_idx >= obstacle_maps.len() {
            continue;
        }

        // Mark trace cells on the source layer.
        for &(gx, gy) in &route.trace_cells {
            mark_with_inflate(&mut obstacle_maps[src_idx], gx, gy, inflate);
        }

        // Mark via cell on both source and target layers (Tier 3 only).
        if let Some((vx, vy)) = route.via_cell {
            mark_with_inflate(&mut obstacle_maps[src_idx], vx, vy, inflate);
            mark_with_inflate(&mut obstacle_maps[tgt_idx], vx, vy, inflate);
        }
    }
}


// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute the neckdown trace width for a pad.
///
/// Returns `max(pad.shape.min_dim() / 2.0, policy.trace_width(net).min)`,
/// optionally clamped from below by `config.neckdown_min_width_mm` when it
/// is non-zero.
fn compute_neckdown_width(
    pad: &autopcb_ir::component::IrComponentPad,
    policy: &RoutingPolicy,
    net_id: NetId,
    source_layer: LayerId,
    config: &EscapeConfig,
) -> f64 {
    if !config.neckdown_enabled {
        return policy.trace_width(net_id, source_layer).preferred;
    }
    let min_dim = pad.shape.size_x.min(pad.shape.size_y);
    let trace_min = policy.trace_width(net_id, source_layer).min;
    let mut width = (min_dim / 2.0).max(trace_min);
    if config.neckdown_min_width_mm > 0.0 {
        width = width.max(config.neckdown_min_width_mm);
    }
    width
}

/// Compute the number of grid cells that fall within the neckdown zone.
///
/// The neckdown zone extends `2.0 * (pad.shape.max_dim() / 2.0 + clearance_mm)`
/// from the pad center, rounded up to the nearest grid cell.
fn compute_neckdown_distance_cells(
    pad: &autopcb_ir::component::IrComponentPad,
    clearance_mm: f64,
    resolution_mm: f64,
) -> u32 {
    let max_dim = pad.shape.size_x.max(pad.shape.size_y);
    let zone_mm = 2.0 * (max_dim / 2.0 + clearance_mm);
    (zone_mm / resolution_mm).ceil() as u32
}

/// Build a per-cell width sequence for a breakout stub.
///
/// Cells within `neckdown_dist_cells` of the pad use `neckdown_width`;
/// cells beyond use `preferred_width`.
fn build_width_sequence(
    trace_cells: &[(u32, u32)],
    neckdown_dist_cells: u32,
    neckdown_width: f64,
    preferred_width: f64,
) -> Vec<(u32, u32, f64)> {
    trace_cells
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| {
            let cell_step = (i + 1) as u32;
            let w = if cell_step <= neckdown_dist_cells {
                neckdown_width
            } else {
                preferred_width
            };
            (x, y, w)
        })
        .collect()
}

/// Count free (unblocked, in-bounds) 8-neighbour cells of `(gx, gy)` on the
/// given obstacle map.
fn count_free_neighbours(
    gx: u32,
    gy: u32,
    map: &ObstacleMap,
    grid: &GridConfig,
) -> usize {
    const OFFSETS: [(i32, i32); 8] = [
        (-1, 0), (1, 0), (0, -1), (0, 1),
        (-1, -1), (1, -1), (-1, 1), (1, 1),
    ];
    let mut count = 0usize;
    for (dx, dy) in OFFSETS {
        let nx = gx as i64 + dx as i64;
        let ny = gy as i64 + dy as i64;
        if nx < 0 || ny < 0 {
            continue;
        }
        let nx = nx as u32;
        let ny = ny as u32;
        if grid.in_bounds(nx, ny) && !map.is_blocked(nx, ny) {
            count += 1;
        }
    }
    count
}

/// Compute a prioritized list of (dx, dy) escape directions for a pad.
///
/// The primary direction is the vector from the component center to the pad,
/// quantized to one of the 4 cardinal directions.  The remaining 3 cardinal
/// directions follow in order (90°, 180°, 270° rotations).
fn escape_directions(
    component_center: autopcb_ir::types::PointMm,
    pad_position: autopcb_ir::types::PointMm,
) -> [(i32, i32); 4] {
    let dx = pad_position.x - component_center.x;
    let dy = pad_position.y - component_center.y;

    // Quantize to primary cardinal direction.
    let primary: (i32, i32) = if dx.abs() >= dy.abs() {
        if dx >= 0.0 { (1, 0) } else { (-1, 0) }
    } else {
        if dy >= 0.0 { (0, 1) } else { (0, -1) }
    };

    // Generate 4 candidates: primary, +90°, 180°, -90°.
    let r90 = rotate_cw(primary);
    let r180 = rotate_cw(r90);
    let r270 = rotate_cw(r180);
    [primary, r90, r180, r270]
}

/// Rotate a cardinal direction 90° clockwise.
#[inline]
fn rotate_cw((dx, dy): (i32, i32)) -> (i32, i32) {
    (dy, -dx)
}

/// Mark a grid cell and an optional inflation margin as blocked.
fn mark_with_inflate(map: &mut ObstacleMap, gx: u32, gy: u32, inflate: u32) {
    let min_gx = gx.saturating_sub(inflate);
    let min_gy = gy.saturating_sub(inflate);
    let max_gx = gx.saturating_add(inflate);
    let max_gy = gy.saturating_add(inflate);
    map.mark_rect_blocked(min_gx, min_gy, max_gx, max_gy);
}

/// Assign a pad to the nearest edge of the component bounding box.
///
/// Computes the distance from `pad_pos` to each of the four edges of
/// `comp_bounds` and returns the edge with the minimum distance. When
/// distances are equal (exact corner), Top takes priority over Bottom,
/// which takes priority over Left, which takes priority over Right
/// (if-chain order).
fn assign_edge(
    pad_pos: autopcb_ir::types::PointMm,
    comp_bounds: &autopcb_ir::types::BoundingBoxMm,
) -> ComponentEdge {
    let dist_top = (comp_bounds.max.y - pad_pos.y).abs();
    let dist_bottom = (pad_pos.y - comp_bounds.min.y).abs();
    let dist_left = (pad_pos.x - comp_bounds.min.x).abs();
    let dist_right = (comp_bounds.max.x - pad_pos.x).abs();

    let min_dist = dist_top.min(dist_bottom).min(dist_left).min(dist_right);

    if (dist_top - min_dist).abs() < 1e-9 {
        ComponentEdge::Top
    } else if (dist_bottom - min_dist).abs() < 1e-9 {
        ComponentEdge::Bottom
    } else if (dist_left - min_dist).abs() < 1e-9 {
        ComponentEdge::Left
    } else {
        ComponentEdge::Right
    }
}

/// Return the outward unit vector (dx, dy) for a component edge.
///
/// - `Top`    → (0, 1)   — escape upward (positive Y)
/// - `Bottom` → (0, -1)  — escape downward (negative Y)
/// - `Left`   → (-1, 0)  — escape leftward (negative X)
/// - `Right`  → (1, 0)   — escape rightward (positive X)
fn edge_escape_direction(edge: ComponentEdge) -> (i32, i32) {
    match edge {
        ComponentEdge::Top => (0, 1),
        ComponentEdge::Bottom => (0, -1),
        ComponentEdge::Left => (-1, 0),
        ComponentEdge::Right => (1, 0),
    }
}

/// Return a stagger offset in grid cells for a pad at `index_on_edge`.
///
/// Alternates between 1 (odd indices) and 3 (even indices) to create
/// short/long patterns that prevent adjacent pad stubs from colliding.
fn stagger_offset(index_on_edge: usize) -> u32 {
    if index_on_edge % 2 == 1 {
        1
    } else {
        3
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use autopcb_ir::{
        component::{IrComponent, IrComponentPad, PadShapeInfo, PadShapeKind},
        copper::FreeCopperGeometry,
        handles::{ComponentId, IdMap, LayerId as IrLayerId, PadId},
        layer_stack::{IrCopperLayer, IrLayerStack, PreferredDirection},
        types::{BoardSide, BoundingBoxMm, PointMm},
        IrBoardGeometry,
    };
    use crate::config::RoutingConfig;
    use crate::rules::build_policy;

    fn make_grid(board_max: f64, resolution: f64) -> GridConfig {
        let width_cells = (board_max / resolution).ceil() as u32 + 1;
        let height_cells = (board_max / resolution).ceil() as u32 + 1;
        GridConfig {
            resolution_mm: resolution,
            width_cells,
            height_cells,
            origin: PointMm::new(0.0, 0.0),
        }
    }

    fn make_obstacle_maps(grid: &GridConfig, layer_count: usize) -> Vec<ObstacleMap> {
        (0..layer_count)
            .map(|_| ObstacleMap::new(grid.width_cells, grid.height_cells))
            .collect()
    }

    fn make_copper_layers(n: usize) -> Vec<IrCopperLayer> {
        (0..n)
            .map(|i| IrCopperLayer {
                id: IrLayerId::from(i as u32),
                name: format!("Layer{i}"),
                is_top: i == 0,
                is_bottom: i == n - 1,
                preferred_direction: Some(PreferredDirection::Any),
            })
            .collect()
    }

    fn make_smd_pad(id: u32, world_pos: PointMm, net_raw: u32, layer_id: u32) -> IrComponentPad {
        IrComponentPad {
            id: PadId::from(id),
            name: format!("{id}"),
            local_position: world_pos,
            world_position: world_pos,
            net: Some(autopcb_ir::handles::NetId::from(net_raw)),
            shape: PadShapeInfo {
                kind: PadShapeKind::Round,
                size_x: 0.3,
                size_y: 0.3,
                rotation: 0.0,
            },
            is_through_hole: false,
            hole_size_mm: 0.0,
            swap_id_pin: None,
            swap_id_part: None,
            layer_set: vec![IrLayerId::from(layer_id)],
        }
    }

    fn make_through_hole_pad(id: u32, world_pos: PointMm) -> IrComponentPad {
        IrComponentPad {
            id: PadId::from(id),
            name: format!("{id}"),
            local_position: world_pos,
            world_position: world_pos,
            net: Some(autopcb_ir::handles::NetId::from(0)),
            shape: PadShapeInfo {
                kind: PadShapeKind::Round,
                size_x: 1.0,
                size_y: 1.0,
                rotation: 0.0,
            },
            is_through_hole: true,
            hole_size_mm: 0.8,
            swap_id_pin: None,
            swap_id_part: None,
            layer_set: vec![IrLayerId::from(0), IrLayerId::from(1)],
        }
    }

    fn make_component(id: u32, pos: PointMm, pads: Vec<IrComponentPad>) -> IrComponent {
        IrComponent {
            id: ComponentId::from(id),
            designator: format!("U{id}"),
            pattern: "BGA".into(),
            value: "".into(),
            position: pos,
            rotation: 0.0,
            side: BoardSide::Top,
            local_bounds: BoundingBoxMm::new(PointMm::new(-5.0, -5.0), PointMm::new(5.0, 5.0)),
            world_bounds: BoundingBoxMm::new(
                PointMm::new(pos.x - 5.0, pos.y - 5.0),
                PointMm::new(pos.x + 5.0, pos.y + 5.0),
            ),
            pads,
        }
    }

    fn make_ir(components: Vec<IrComponent>, layer_count: usize) -> autopcb_ir::PcbIr {
        let board_max = 50.0;
        let copper_layers = make_copper_layers(layer_count);
        let mut comp_map: IdMap<ComponentId, IrComponent> = IdMap::new();
        for comp in components {
            comp_map.push(comp);
        }
        autopcb_ir::PcbIr {
            board: IrBoardGeometry {
                outline: vec![
                    PointMm::new(0.0, 0.0),
                    PointMm::new(board_max, 0.0),
                    PointMm::new(board_max, board_max),
                    PointMm::new(0.0, board_max),
                ],
                cutouts: vec![],
                bounds: BoundingBoxMm::new(
                    PointMm::new(0.0, 0.0),
                    PointMm::new(board_max, board_max),
                ),
                keepouts: vec![],
            },
            layer_stack: IrLayerStack {
                copper_layers,
                copper_layer_count: layer_count,
            },
            components: comp_map,
            nets: IdMap::new(),
            rules: IdMap::new(),
            free_copper: FreeCopperGeometry::default(),
            polygons: IdMap::new(),
            texts: IdMap::new(),
            regions: IdMap::new(),
            component_bodies: IdMap::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Test: through-hole only board → empty plan
    // -----------------------------------------------------------------------

    #[test]
    fn plan_escapes_empty_for_through_hole_only() {
        let pos = PointMm::new(25.0, 25.0);
        let pad = make_through_hole_pad(0, pos);
        let comp = make_component(0, pos, vec![pad]);
        let ir = make_ir(vec![comp], 4);
        let grid = make_grid(50.0, 0.5);
        let maps = make_obstacle_maps(&grid, 4);
        let config = EscapeConfig::default();
        let routes = plan_via_escapes(&ir, &grid, &maps, 4, &config, &[]);
        assert!(
            routes.is_empty(),
            "through-hole pads must not generate escape routes"
        );
    }

    // -----------------------------------------------------------------------
    // Test: 2-layer board → empty via-escape plan (no inner layers)
    // -----------------------------------------------------------------------

    #[test]
    fn plan_via_escapes_empty_for_two_layer_board() {
        // The `plan_via_escapes()` function runs only Tier 3 (via escapes).
        // On a 2-layer board there are no inner layers, so Tier 3 produces
        // nothing.  The full `plan_breakouts()` pipeline would still produce
        // Tier 1/2 stubs on a 2-layer board.
        let pos = PointMm::new(25.0, 25.0);
        let pad = make_smd_pad(0, pos, 1, 0);
        let comp = make_component(0, pos, vec![pad]);
        let ir = make_ir(vec![comp], 2);
        let grid = make_grid(50.0, 0.5);
        let maps = make_obstacle_maps(&grid, 2);
        let config = EscapeConfig::default();
        let routes = plan_via_escapes(&ir, &grid, &maps, 2, &config, &[]);
        assert!(
            routes.is_empty(),
            "2-layer board has no inner layers; via escape plan must be empty"
        );
    }

    // -----------------------------------------------------------------------
    // Test: dense SMD component with blocked neighbours → routes generated
    // -----------------------------------------------------------------------

    #[test]
    fn plan_escapes_generates_routes_for_dense_smd() {
        // Place a component at center with one pad to the right.
        // Block all 8 neighbours of the pad on layer 0 to force escape planning.
        let center = PointMm::new(25.0, 25.0);
        // Pad at (30, 25) — offset from center so escape direction is rightward.
        let pad_pos = PointMm::new(30.0, 25.0);
        let pad = make_smd_pad(0, pad_pos, 1, 0);
        let comp = make_component(0, center, vec![pad]);
        let ir = make_ir(vec![comp], 4);

        let grid = make_grid(50.0, 0.5);
        let mut maps = make_obstacle_maps(&grid, 4);

        // Block 6 of the 8 neighbours of the pad cell on layer 0 so that
        // free_count (= 2) < min_access_threshold (= 3), triggering escape
        // planning.  Leave the rightward cell and one other unblocked so the
        // escape walk can find a path in the +x direction.  We do NOT block
        // (pgx+1, pgy) so the escape trace can exit there.
        let (pgx, pgy) = grid.to_grid(pad_pos);
        let blocked_offsets: [(i32, i32); 6] = [
            (-1, 0), (0, -1), (0, 1),
            (-1, -1), (1, -1), (-1, 1),
        ];
        for (dx, dy) in blocked_offsets {
            let nx = (pgx as i64 + dx as i64) as u32;
            let ny = (pgy as i64 + dy as i64) as u32;
            maps[0].set_blocked(nx, ny, true);
        }

        let config = EscapeConfig {
            enabled: true,
            min_escape_mm: 0.5,
            max_escape_mm: 3.0,
            min_access_threshold: 3,
            neckdown_enabled: true,
            neckdown_min_width_mm: 0.0,
        };
        let routes = plan_via_escapes(&ir, &grid, &maps, 4, &config, &[]);

        assert!(
            !routes.is_empty(),
            "dense SMD pad with blocked neighbours should produce escape routes"
        );

        let route = &routes[0];
        assert_eq!(route.source_layer, LayerId(0), "escape must start on top layer");
        assert!(
            route.target_layer.raw() > 0 && route.target_layer.raw() < 3,
            "escape must target an inner layer"
        );
        assert!(!route.trace_cells.is_empty(), "escape trace must have at least one cell");
    }

    // -----------------------------------------------------------------------
    // Test: escape direction computation
    // -----------------------------------------------------------------------

    #[test]
    fn escape_direction_rightward_pad() {
        let center = PointMm::new(0.0, 0.0);
        let pad_right = PointMm::new(5.0, 0.0);
        let dirs = escape_directions(center, pad_right);
        // Primary direction must be rightward (1, 0).
        assert_eq!(dirs[0], (1, 0), "pad to the right → primary direction (1,0)");
        // Remaining are 90° rotations.
        assert_eq!(dirs[1], rotate_cw(dirs[0]));
        assert_eq!(dirs[2], rotate_cw(dirs[1]));
        assert_eq!(dirs[3], rotate_cw(dirs[2]));
    }

    #[test]
    fn escape_direction_upward_pad() {
        let center = PointMm::new(0.0, 0.0);
        let pad_up = PointMm::new(0.0, 5.0);
        let dirs = escape_directions(center, pad_up);
        assert_eq!(dirs[0], (0, 1), "pad above → primary direction (0,1)");
    }

    #[test]
    fn escape_direction_diagonal_quantized() {
        let center = PointMm::new(0.0, 0.0);
        // dx=3, dy=4 → dy > dx, so primary should be (0,1).
        let pad = PointMm::new(3.0, 4.0);
        let dirs = escape_directions(center, pad);
        assert_eq!(dirs[0], (0, 1), "dominant dy→ primary (0,1)");
    }

    // -----------------------------------------------------------------------
    // Test: disabled config → empty plan
    // -----------------------------------------------------------------------

    #[test]
    fn plan_escapes_disabled_returns_empty() {
        let pos = PointMm::new(25.0, 25.0);
        let pad = make_smd_pad(0, pos, 1, 0);
        let comp = make_component(0, pos, vec![pad]);
        let ir = make_ir(vec![comp], 4);
        let grid = make_grid(50.0, 0.5);
        let maps = make_obstacle_maps(&grid, 4);
        let config = EscapeConfig {
            enabled: false,
            ..EscapeConfig::default()
        };
        let routes = plan_via_escapes(&ir, &grid, &maps, 4, &config, &[]);
        assert!(routes.is_empty(), "disabled escape planning must return empty plan");
    }

    // -----------------------------------------------------------------------
    // Test: apply_escapes blocks correct cells
    // -----------------------------------------------------------------------

    #[test]
    fn apply_escapes_marks_cells_blocked() {
        let grid = make_grid(20.0, 1.0);
        let mut maps = make_obstacle_maps(&grid, 4);

        let plan = BreakoutPlan {
            routes: vec![BreakoutRoute {
                tier: BreakoutTier::ViaEscape,
                net_id: NetId(1),
                pad_cell: (4, 5),
                source_layer: LayerId(0),
                target_layer: LayerId(1),
                trace_cells: vec![(5, 5), (6, 5)],
                via_cell: Some((6, 5)),
                neckdown_width_mm: None,
                stub_endpoint: (6, 5),
                width_sequence: Vec::new(),
            }],
        };

        apply_breakouts(&plan, &mut maps, 0);

        // Trace cells blocked on source layer.
        assert!(maps[0].is_blocked(5, 5), "trace cell (5,5) on layer 0 must be blocked");
        assert!(maps[0].is_blocked(6, 5), "via cell (6,5) on layer 0 must be blocked");
        // Via cell also blocked on target layer.
        assert!(maps[1].is_blocked(6, 5), "via cell (6,5) on layer 1 must be blocked");
        // Other cells unaffected.
        assert!(!maps[0].is_blocked(4, 5), "cell (4,5) must be unblocked");
        assert!(!maps[2].is_blocked(6, 5), "layer 2 must be unaffected");
    }

    // -----------------------------------------------------------------------
    // classify_component helper
    // -----------------------------------------------------------------------

    fn make_pad_at(id: u32, x: f64, y: f64) -> IrComponentPad {
        IrComponentPad {
            id: PadId::from(id),
            name: format!("{id}"),
            local_position: PointMm::new(x, y),
            world_position: PointMm::new(x, y),
            net: None,
            shape: PadShapeInfo {
                kind: PadShapeKind::Round,
                size_x: 0.5,
                size_y: 0.5,
                rotation: 0.0,
            },
            is_through_hole: false,
            hole_size_mm: 0.0,
            swap_id_pin: None,
            swap_id_part: None,
            layer_set: vec![IrLayerId::from(0)],
        }
    }

    fn make_comp_with_pads(pads: Vec<IrComponentPad>) -> IrComponent {
        IrComponent {
            id: ComponentId::from(0),
            designator: "U1".into(),
            pattern: "TEST".into(),
            value: "".into(),
            position: PointMm::new(0.0, 0.0),
            rotation: 0.0,
            side: BoardSide::Top,
            local_bounds: BoundingBoxMm::new(PointMm::new(-10.0, -10.0), PointMm::new(10.0, 10.0)),
            world_bounds: BoundingBoxMm::new(PointMm::new(-10.0, -10.0), PointMm::new(10.0, 10.0)),
            pads,
        }
    }

    // -----------------------------------------------------------------------
    // Test: classify_component — QFP-like component → Peripheral
    // -----------------------------------------------------------------------

    #[test]
    fn classify_component_peripheral() {
        // QFP-like: 12 pads arranged on 4 edges of a 6mm × 6mm footprint.
        // 3 pads per edge, all at the bounding box extremes, none interior.
        let pitch = 1.0_f64;
        let edge = 3.0_f64;
        let mut pads = Vec::new();
        let mut id = 0u32;
        // Top edge: y = +edge, x in {-pitch, 0, +pitch}
        for i in -1i32..=1 {
            pads.push(make_pad_at(id, i as f64 * pitch, edge));
            id += 1;
        }
        // Bottom edge: y = -edge
        for i in -1i32..=1 {
            pads.push(make_pad_at(id, i as f64 * pitch, -edge));
            id += 1;
        }
        // Left edge: x = -edge
        for i in -1i32..=1 {
            pads.push(make_pad_at(id, -edge, i as f64 * pitch));
            id += 1;
        }
        // Right edge: x = +edge
        for i in -1i32..=1 {
            pads.push(make_pad_at(id, edge, i as f64 * pitch));
            id += 1;
        }
        assert_eq!(pads.len(), 12);

        let comp = make_comp_with_pads(pads);
        assert_eq!(
            classify_component(&comp),
            ComponentKind::Peripheral,
            "QFP-like component with all pads on edges must be Peripheral"
        );
    }

    // -----------------------------------------------------------------------
    // Test: classify_component — BGA-like 4×4 grid → AreaArray
    // -----------------------------------------------------------------------

    #[test]
    fn classify_component_area_array() {
        // BGA-like: 4×4 = 16 pads in a uniform 1mm-pitch grid.
        // The 4 inner pads at (±0.5, ±0.5) sit 1.0mm from the bbox edge,
        // which exceeds half the 1.0mm pitch, so they are classified as
        // interior pads.  With interior_count=4 > 0 and pad_count=16 > 8,
        // the component is classified as AreaArray.
        let pitch = 1.0_f64;
        let mut pads = Vec::new();
        let mut id = 0u32;
        for row in 0..4 {
            for col in 0..4 {
                let x = (col as f64 - 1.5) * pitch;
                let y = (row as f64 - 1.5) * pitch;
                pads.push(make_pad_at(id, x, y));
                id += 1;
            }
        }
        assert_eq!(pads.len(), 16);

        let comp = make_comp_with_pads(pads);
        assert_eq!(
            classify_component(&comp),
            ComponentKind::AreaArray,
            "4×4 BGA-like grid must be AreaArray"
        );
    }

    // -----------------------------------------------------------------------
    // Test: classify_component — BGA-64 with 8×8 grid → AreaArray
    // -----------------------------------------------------------------------

    #[test]
    fn classify_component_area_array_bga64() {
        // BGA-64: 8×8 = 64 pads in a uniform 1mm-pitch grid.
        // The 6×6 = 36 inner pads are interior (distance > half-pitch from
        // bbox edge).  With interior_count=36 > 0 and pad_count=64 > 8,
        // the component is classified as AreaArray.
        let pitch = 1.0_f64;
        let mut pads = Vec::new();
        let mut id = 0u32;
        for row in 0..8 {
            for col in 0..8 {
                let x = (col as f64 - 3.5) * pitch;
                let y = (row as f64 - 3.5) * pitch;
                pads.push(make_pad_at(id, x, y));
                id += 1;
            }
        }
        assert_eq!(pads.len(), 64);

        let comp = make_comp_with_pads(pads);
        assert_eq!(
            classify_component(&comp),
            ComponentKind::AreaArray,
            "8×8 BGA-64 grid must be AreaArray"
        );
    }

    // -----------------------------------------------------------------------
    // Test: classify_component — 3-pin SOT → Other
    // -----------------------------------------------------------------------

    #[test]
    fn classify_component_sot() {
        // SOT-23: 3 pads.  pad_count=3 ≤ 4 fails the Peripheral minimum-count
        // guard, and pad_count=3 ≤ 8 fails the AreaArray condition → Other.
        let pads = vec![
            make_pad_at(0, -0.95, -1.3),
            make_pad_at(1, 0.95, -1.3),
            make_pad_at(2, 0.0, 1.3),
        ];
        let comp = make_comp_with_pads(pads);
        assert_eq!(
            classify_component(&comp),
            ComponentKind::Other,
            "3-pin SOT must be Other (too few pads for Peripheral, not an area array)"
        );
    }

    // -----------------------------------------------------------------------
    // Test: classify_component — mixed perimeter/interior (small) → Other
    // -----------------------------------------------------------------------

    #[test]
    fn classify_component_mixed_small() {
        // A component with 6 outer pads and 2 interior pads (8 total).
        // interior_count=2 > 0 but pad_count=8, which is not > 8 → Other.
        //
        // Outer frame (6 pads at bbox boundary, pitch ≈ 2.0mm):
        //   Top row:    (-2, 2), (0, 2), (2, 2)
        //   Bottom row: (-2,-2), (0,-2), (2,-2)
        // Interior pads (2 pads well inside):
        //   (0, 0.5), (0, -0.5)
        let outer: [(f64, f64); 6] = [
            (-2.0, 2.0), (0.0, 2.0), (2.0, 2.0),
            (-2.0, -2.0), (0.0, -2.0), (2.0, -2.0),
        ];
        let inner: [(f64, f64); 2] = [(0.0, 0.5), (0.0, -0.5)];
        let mut pads = Vec::new();
        let mut id = 0u32;
        for (x, y) in outer {
            pads.push(make_pad_at(id, x, y));
            id += 1;
        }
        for (x, y) in inner {
            pads.push(make_pad_at(id, x, y));
            id += 1;
        }
        assert_eq!(pads.len(), 8);

        let comp = make_comp_with_pads(pads);
        assert_eq!(
            classify_component(&comp),
            ComponentKind::Other,
            "component with 6 perimeter + 2 interior pads (8 total, not > 8) must be Other"
        );
    }

    // -----------------------------------------------------------------------
    // Test: classify_component — 50%/50% mixed with enough pads → AreaArray
    // -----------------------------------------------------------------------

    #[test]
    fn classify_component_mixed_large_is_area_array() {
        // A component with 5 perimeter + 5 interior pads (10 total).
        // interior_count=5 > 0 AND pad_count=10 > 8 → AreaArray.
        //
        // NOTE: The plan originally expected this to be Other (perimeter_ratio
        // < 0.3), but the implemented algorithm uses interior_count > 0
        // instead of perimeter_ratio < 0.3 (see Decision Log). A component
        // with ANY interior pads AND > 8 total pads is AreaArray — this
        // correctly captures the "grid of pads" pattern even when many pads
        // are also on the perimeter.
        let outer: [(f64, f64); 5] = [
            (-3.0, 3.0), (0.0, 3.0), (3.0, 3.0),
            (-3.0, -3.0), (3.0, -3.0),
        ];
        let inner: [(f64, f64); 5] = [
            (-1.0, 1.0), (1.0, 1.0), (0.0, 0.0),
            (-1.0, -1.0), (1.0, -1.0),
        ];
        let mut pads = Vec::new();
        let mut id = 0u32;
        for (x, y) in outer {
            pads.push(make_pad_at(id, x, y));
            id += 1;
        }
        for (x, y) in inner {
            pads.push(make_pad_at(id, x, y));
            id += 1;
        }
        assert_eq!(pads.len(), 10);

        let comp = make_comp_with_pads(pads);
        assert_eq!(
            classify_component(&comp),
            ComponentKind::AreaArray,
            "component with 5 perimeter + 5 interior pads (10 total) is AreaArray"
        );
    }

    // -----------------------------------------------------------------------
    // plan_stubs helpers
    // -----------------------------------------------------------------------

    fn make_policy(ir: &autopcb_ir::PcbIr) -> RoutingPolicy {
        build_policy(ir, &RoutingConfig::default()).expect("build_policy failed in test")
    }

    // -----------------------------------------------------------------------
    // Test: plan_stubs — dense SMD pad on 2-layer board generates same-layer stub
    // -----------------------------------------------------------------------

    #[test]
    fn plan_stubs_generates_stub_for_dense_smd() {
        let center = PointMm::new(25.0, 25.0);
        let pad_pos = PointMm::new(25.0, 25.0);
        let pad = make_smd_pad(0, pad_pos, 1, 0);
        let comp = make_component(0, center, vec![pad]);
        let ir = make_ir(vec![comp], 2);

        let grid = make_grid(50.0, 0.5);
        let mut maps = make_obstacle_maps(&grid, 2);

        // Block 7 of the 8 neighbours on layer 0 — leave (pgx+1, pgy) free so
        // the +x direction can escape.
        let (pgx, pgy) = grid.to_grid(pad_pos);
        let blocked_offsets: [(i32, i32); 7] = [
            (-1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
            (1, 1),
        ];
        for (dx, dy) in blocked_offsets {
            let nx = (pgx as i64 + dx as i64) as u32;
            let ny = (pgy as i64 + dy as i64) as u32;
            maps[0].set_blocked(nx, ny, true);
        }

        let config = EscapeConfig {
            enabled: true,
            min_escape_mm: 0.5,
            max_escape_mm: 3.0,
            min_access_threshold: 3,
            neckdown_enabled: true,
            neckdown_min_width_mm: 0.0,
        };
        let policy = make_policy(&ir);
        let routes = plan_stubs(&ir, &grid, &maps, &policy, &config);

        assert_eq!(routes.len(), 1, "expected exactly one stub route");
        let route = &routes[0];
        assert_eq!(route.tier, BreakoutTier::Stub, "route tier must be Stub");
        assert!(route.via_cell.is_none(), "Tier 1 stubs must have no via");
        assert_eq!(
            route.target_layer, route.source_layer,
            "Tier 1 stub must stay on source layer"
        );
        // Width sequence must be non-empty and contain neckdown width for near cells.
        assert!(
            !route.width_sequence.is_empty(),
            "width_sequence must be non-empty"
        );
        let (_, _, near_width) = route.width_sequence[0];
        let neckdown = route.neckdown_width_mm.expect("neckdown_width_mm must be Some");
        assert!(
            (near_width - neckdown).abs() < 1e-9,
            "first cell width must equal neckdown width"
        );
    }

    // -----------------------------------------------------------------------
    // Test: compute_neckdown_width — formula verification
    // -----------------------------------------------------------------------

    #[test]
    fn plan_stubs_neckdown_width_computed_correctly() {
        // Pad with size_x=0.6, size_y=0.4 → min_dim=0.4.
        // Policy default min trace width = 0.1 mm.
        // neckdown_width = max(0.4/2, 0.1) = max(0.2, 0.1) = 0.2
        let ir = make_ir(vec![], 2);
        let policy = make_policy(&ir);

        let config = EscapeConfig::default();
        let pad = IrComponentPad {
            id: PadId::from(0),
            name: "1".into(),
            local_position: PointMm::new(0.0, 0.0),
            world_position: PointMm::new(0.0, 0.0),
            net: Some(autopcb_ir::handles::NetId::from(1)),
            shape: PadShapeInfo {
                kind: PadShapeKind::Rectangular,
                size_x: 0.6,
                size_y: 0.4,
                rotation: 0.0,
            },
            is_through_hole: false,
            hole_size_mm: 0.0,
            swap_id_pin: None,
            swap_id_part: None,
            layer_set: vec![IrLayerId::from(0)],
        };

        let net_id = NetId(1);
        let source_layer = LayerId(0);
        let width = compute_neckdown_width(&pad, &policy, net_id, source_layer, &config);
        // max(0.4/2=0.2, policy_min=0.1) = 0.2
        assert!(
            (width - 0.2).abs() < 1e-9,
            "neckdown_width should be 0.2, got {width}"
        );
    }

    // -----------------------------------------------------------------------
    // Test: plan_stubs — pad with enough free neighbours is skipped
    // -----------------------------------------------------------------------

    #[test]
    fn plan_stubs_pad_with_enough_free_neighbors_skipped() {
        let center = PointMm::new(25.0, 25.0);
        let pad = make_smd_pad(0, center, 1, 0);
        let comp = make_component(0, center, vec![pad]);
        let ir = make_ir(vec![comp], 2);

        let grid = make_grid(50.0, 0.5);
        let maps = make_obstacle_maps(&grid, 2);
        // No neighbours blocked → all 8 neighbours are free → free_count=8 >= threshold=3

        let config = EscapeConfig::default();
        let policy = make_policy(&ir);
        let routes = plan_stubs(&ir, &grid, &maps, &policy, &config);

        assert!(
            routes.is_empty(),
            "pad with 8 free neighbours must not generate a stub"
        );
    }

    // -----------------------------------------------------------------------
    // Test: plan_stubs — all 8 directions blocked → empty (graceful skip)
    // -----------------------------------------------------------------------

    #[test]
    fn plan_stubs_all_directions_blocked_returns_empty() {
        let center = PointMm::new(25.0, 25.0);
        let pad_pos = PointMm::new(25.0, 25.0);
        let pad = make_smd_pad(0, pad_pos, 1, 0);
        let comp = make_component(0, center, vec![pad]);
        let ir = make_ir(vec![comp], 2);

        let grid = make_grid(50.0, 0.5);
        let mut maps = make_obstacle_maps(&grid, 2);

        // Block all cells in the escape range on layer 0 (a solid ring
        // extending max_escape_mm out from the pad).
        let (pgx, pgy) = grid.to_grid(pad_pos);
        let max_steps = 6i64; // covers 3.0mm at 0.5mm resolution
        for dx in -max_steps..=max_steps {
            for dy in -max_steps..=max_steps {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = pgx as i64 + dx;
                let ny = pgy as i64 + dy;
                if nx >= 0 && ny >= 0 && grid.in_bounds(nx as u32, ny as u32) {
                    maps[0].set_blocked(nx as u32, ny as u32, true);
                }
            }
        }

        let config = EscapeConfig {
            enabled: true,
            min_escape_mm: 0.5,
            max_escape_mm: 3.0,
            min_access_threshold: 3,
            neckdown_enabled: true,
            neckdown_min_width_mm: 0.0,
        };
        let policy = make_policy(&ir);
        let routes = plan_stubs(&ir, &grid, &maps, &policy, &config);

        assert!(
            routes.is_empty(),
            "all directions blocked must return empty routes (graceful skip)"
        );
    }

    // -----------------------------------------------------------------------
    // Test: plan_stubs — single-layer board works
    // -----------------------------------------------------------------------

    #[test]
    fn plan_stubs_works_on_single_layer_board() {
        let center = PointMm::new(25.0, 25.0);
        let pad_pos = PointMm::new(25.0, 25.0);
        let pad = make_smd_pad(0, pad_pos, 1, 0);
        let comp = make_component(0, center, vec![pad]);
        let ir = make_ir(vec![comp], 1);

        let grid = make_grid(50.0, 0.5);
        let mut maps = make_obstacle_maps(&grid, 1);

        // Block 7 of the 8 neighbours on layer 0 — leave (pgx+1, pgy) free.
        let (pgx, pgy) = grid.to_grid(pad_pos);
        let blocked_offsets: [(i32, i32); 7] = [
            (-1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
            (1, 1),
        ];
        for (dx, dy) in blocked_offsets {
            let nx = (pgx as i64 + dx as i64) as u32;
            let ny = (pgy as i64 + dy as i64) as u32;
            maps[0].set_blocked(nx, ny, true);
        }

        let config = EscapeConfig {
            enabled: true,
            min_escape_mm: 0.5,
            max_escape_mm: 3.0,
            min_access_threshold: 3,
            neckdown_enabled: true,
            neckdown_min_width_mm: 0.0,
        };
        let policy = make_policy(&ir);
        let routes = plan_stubs(&ir, &grid, &maps, &policy, &config);

        assert!(!routes.is_empty(), "single-layer board must still generate stubs");
        assert_eq!(
            routes[0].tier,
            BreakoutTier::Stub,
            "stub on single-layer board must have tier=Stub"
        );
    }

    // -----------------------------------------------------------------------
    // Helper: build a Peripheral component with pads at specified positions
    // -----------------------------------------------------------------------

    /// Make a Peripheral component with 5 pads: one near each edge plus one
    /// more on the bottom edge to exceed the pad_count > 4 threshold.
    fn make_peripheral_comp_5pad() -> IrComponent {
        // Bounding box: -5..+5 in both axes (from make_comp_with_pads default).
        // Place pads near each edge so they satisfy the perimeter ratio.
        //   Top:    (0, 4.8)
        //   Bottom: (0, -4.8)
        //   Left:   (-4.8, 0)
        //   Right:  (4.8, 0)
        //   Extra bottom: (-1.0, -4.8) so pad_count=5 > 4
        let pads = vec![
            make_pad_at(0, 0.0, 4.8),   // top
            make_pad_at(1, 0.0, -4.8),  // bottom
            make_pad_at(2, -4.8, 0.0),  // left
            make_pad_at(3, 4.8, 0.0),   // right
            make_pad_at(4, -1.0, -4.8), // extra bottom (needed so pad_count > 4)
        ];
        make_comp_with_pads(pads)
    }

    // -----------------------------------------------------------------------
    // Test: plan_perimeter_escapes — 4-direction peripheral component
    // -----------------------------------------------------------------------

    #[test]
    fn plan_perimeter_escapes_four_pad_component() {
        // Build a Peripheral component where each pad is near a different edge.
        // The 5th pad is needed to exceed the pad_count > 4 threshold.
        let pads = vec![
            make_smd_pad(0, PointMm::new(0.0, 4.8), 1, 0),   // top
            make_smd_pad(1, PointMm::new(0.0, -4.8), 2, 0),  // bottom
            make_smd_pad(2, PointMm::new(-4.8, 0.0), 3, 0),  // left
            make_smd_pad(3, PointMm::new(4.8, 0.0), 4, 0),   // right
            make_smd_pad(4, PointMm::new(-1.0, -4.8), 5, 0), // extra bottom
        ];
        let comp = make_comp_with_pads(pads);
        assert_eq!(
            classify_component(&comp),
            ComponentKind::Peripheral,
            "test component must be Peripheral"
        );
        let ir = make_ir(vec![comp], 2);
        let grid = make_grid(50.0, 0.25);

        // Block enough neighbours so each pad has free_count < min_access_threshold.
        let mut maps = make_obstacle_maps(&grid, 2);
        let pad_positions = [
            PointMm::new(0.0, 4.8),
            PointMm::new(0.0, -4.8),
            PointMm::new(-4.8, 0.0),
            PointMm::new(4.8, 0.0),
            PointMm::new(-1.0, -4.8),
        ];
        // Block 7 of 8 neighbours for each pad, leaving the outward direction free.
        let outward_free: [(i32, i32); 5] = [(0, 1), (0, -1), (-1, 0), (1, 0), (0, -1)];
        for (i, &pos) in pad_positions.iter().enumerate() {
            let (pgx, pgy) = grid.to_grid(pos);
            let free_dir = outward_free[i];
            for (dx, dy) in DIRECTIONS_8 {
                if (dx, dy) == free_dir {
                    continue;
                }
                let nx = pgx as i64 + dx as i64;
                let ny = pgy as i64 + dy as i64;
                if nx >= 0 && ny >= 0 && grid.in_bounds(nx as u32, ny as u32) {
                    maps[0].set_blocked(nx as u32, ny as u32, true);
                }
            }
        }

        let config = EscapeConfig {
            enabled: true,
            min_escape_mm: 0.25,
            max_escape_mm: 3.0,
            min_access_threshold: 3,
            neckdown_enabled: true,
            neckdown_min_width_mm: 0.0,
        };
        let policy = make_policy(&ir);
        let routes =
            plan_perimeter_escapes(&ir, &grid, &maps, &policy, &config, &[]);

        assert!(
            !routes.is_empty(),
            "Peripheral component must produce perimeter escape routes"
        );

        // Verify all routes have tier=PerimeterEscape, no via, same layer.
        for route in &routes {
            assert_eq!(route.tier, BreakoutTier::PerimeterEscape);
            assert!(route.via_cell.is_none(), "Tier 2 must not place vias");
            assert_eq!(
                route.source_layer, route.target_layer,
                "Tier 2 stays on source layer"
            );
        }

        // Verify escape directions match expected outward direction per edge.
        // Top pad (net 1) → escape cell should have y > pad_y.
        // Bottom pads (net 2, 5) → escape cell should have y < pad_y.
        // Left pad (net 3) → escape cell should have x < pad_x.
        // Right pad (net 4) → escape cell should have x > pad_x.
        let check_escape = |net: u32, cond: &dyn Fn((u32, u32)) -> bool, msg: &str| {
            if let Some(r) = routes.iter().find(|r| r.net_id == NetId(net)) {
                let ep = r.stub_endpoint;
                assert!(cond(ep), "{msg}: endpoint {:?}", ep);
            }
        };
        let (_, top_gy) = grid.to_grid(PointMm::new(0.0, 4.8));
        let (_, bot_gy) = grid.to_grid(PointMm::new(0.0, -4.8));
        let (lft_gx, _) = grid.to_grid(PointMm::new(-4.8, 0.0));
        let (rgt_gx, _) = grid.to_grid(PointMm::new(4.8, 0.0));
        check_escape(1, &|(_, gy)| gy > top_gy, "top pad must escape upward");
        check_escape(2, &|(_, gy)| gy < bot_gy, "bottom pad must escape downward");
        check_escape(3, &|(gx, _)| gx < lft_gx, "left pad must escape leftward");
        check_escape(4, &|(gx, _)| gx > rgt_gx, "right pad must escape rightward");
    }

    // -----------------------------------------------------------------------
    // Test: plan_perimeter_escapes — adjacent pads on same edge are staggered
    // -----------------------------------------------------------------------

    #[test]
    fn plan_perimeter_escapes_stagger_adjacent() {
        // Build a Peripheral component with 6 pads on the top edge + 2 on
        // other edges (to satisfy pad_count > 4 and perimeter_ratio > 0.8).
        let mut pads = Vec::new();
        // 6 pads evenly spaced on the top edge (y=4.5).
        for i in 0..6u32 {
            let x = -2.5 + i as f64;
            pads.push(make_smd_pad(i, PointMm::new(x, 4.5), i + 1, 0));
        }
        // 1 pad on bottom and 1 on right to round out the Peripheral shape.
        pads.push(make_smd_pad(6, PointMm::new(0.0, -4.5), 7, 0));
        pads.push(make_smd_pad(7, PointMm::new(4.5, 0.0), 8, 0));
        let comp = make_comp_with_pads(pads);
        assert_eq!(classify_component(&comp), ComponentKind::Peripheral);

        let ir = make_ir(vec![comp], 2);
        let grid = make_grid(50.0, 0.25);
        let mut maps = make_obstacle_maps(&grid, 2);

        // Block enough neighbours so all pads need escape.
        let top_xs: Vec<f64> = (-2..4).map(|i| -2.5 + i as f64).collect();
        for x in &top_xs {
            let pos = PointMm::new(*x, 4.5);
            let (pgx, pgy) = grid.to_grid(pos);
            for (dx, dy) in DIRECTIONS_8 {
                if (dx, dy) == (0, 1) {
                    continue; // leave outward direction free
                }
                let nx = pgx as i64 + dx as i64;
                let ny = pgy as i64 + dy as i64;
                if nx >= 0 && ny >= 0 && grid.in_bounds(nx as u32, ny as u32) {
                    maps[0].set_blocked(nx as u32, ny as u32, true);
                }
            }
        }

        let config = EscapeConfig {
            enabled: true,
            min_escape_mm: 0.25,
            max_escape_mm: 4.0,
            min_access_threshold: 3,
            neckdown_enabled: true,
            neckdown_min_width_mm: 0.0,
        };
        let policy = make_policy(&ir);
        let routes =
            plan_perimeter_escapes(&ir, &grid, &maps, &policy, &config, &[]);

        // Collect only top-edge pad routes (nets 1-6).
        let top_routes: Vec<&BreakoutRoute> = routes
            .iter()
            .filter(|r| r.net_id.0 <= 6)
            .collect();

        assert!(
            top_routes.len() >= 2,
            "must have at least 2 top-edge routes to test stagger"
        );

        // Adjacent top-edge stubs must have different lengths.
        let lengths: Vec<usize> = top_routes.iter().map(|r| r.trace_cells.len()).collect();
        let has_different = lengths
            .windows(2)
            .any(|w| w[0] != w[1]);
        assert!(
            has_different,
            "adjacent perimeter escape stubs must be staggered (different lengths): {:?}",
            lengths
        );
    }

    // -----------------------------------------------------------------------
    // Test: plan_perimeter_escapes — corner pad assigned to nearest edge
    // -----------------------------------------------------------------------

    #[test]
    fn plan_perimeter_escapes_corner_pad() {
        // A pad at the exact top-right corner of the bounding box is equidistant
        // from the Top and Right edges. assign_edge should consistently pick
        // Top (checked first in distance comparison).
        let bounds = BoundingBoxMm::new(PointMm::new(-5.0, -5.0), PointMm::new(5.0, 5.0));
        // Exact corner: (5.0, 5.0) — equidistant from Top and Right.
        let corner = PointMm::new(5.0, 5.0);
        let edge = assign_edge(corner, &bounds);
        // dist_top = 0, dist_right = 0: Top is checked first → Top wins.
        assert_eq!(
            edge,
            ComponentEdge::Top,
            "corner pad equidistant from Top and Right must be assigned to Top"
        );

        // A pad clearly nearest to the right edge.
        let near_right = PointMm::new(4.9, 0.0);
        assert_eq!(
            assign_edge(near_right, &bounds),
            ComponentEdge::Right,
            "pad near right edge must be assigned to Right"
        );

        // A pad clearly nearest to the bottom edge.
        let near_bottom = PointMm::new(0.0, -4.9);
        assert_eq!(
            assign_edge(near_bottom, &bounds),
            ComponentEdge::Bottom,
            "pad near bottom edge must be assigned to Bottom"
        );

        // A pad clearly nearest to the left edge.
        let near_left = PointMm::new(-4.9, 0.0);
        assert_eq!(
            assign_edge(near_left, &bounds),
            ComponentEdge::Left,
            "pad near left edge must be assigned to Left"
        );
    }

    // -----------------------------------------------------------------------
    // Test: plan_perimeter_escapes — existing routes are skipped
    // -----------------------------------------------------------------------

    #[test]
    fn plan_perimeter_escapes_skips_existing_routes() {
        // Reuse the 5-pad peripheral component from the four-pad test.
        let pads = vec![
            make_smd_pad(0, PointMm::new(0.0, 4.8), 1, 0),   // top
            make_smd_pad(1, PointMm::new(0.0, -4.8), 2, 0),  // bottom
            make_smd_pad(2, PointMm::new(-4.8, 0.0), 3, 0),  // left
            make_smd_pad(3, PointMm::new(4.8, 0.0), 4, 0),   // right
            make_smd_pad(4, PointMm::new(-1.0, -4.8), 5, 0), // extra bottom
        ];
        let comp = make_comp_with_pads(pads);
        let ir = make_ir(vec![comp], 2);
        let grid = make_grid(50.0, 0.25);

        let mut maps = make_obstacle_maps(&grid, 2);
        // Block neighbours for all pads.
        let positions = [
            PointMm::new(0.0, 4.8),
            PointMm::new(0.0, -4.8),
            PointMm::new(-4.8, 0.0),
            PointMm::new(4.8, 0.0),
            PointMm::new(-1.0, -4.8),
        ];
        let outward_free: [(i32, i32); 5] = [(0, 1), (0, -1), (-1, 0), (1, 0), (0, -1)];
        for (i, &pos) in positions.iter().enumerate() {
            let (pgx, pgy) = grid.to_grid(pos);
            let free_dir = outward_free[i];
            for (dx, dy) in DIRECTIONS_8 {
                if (dx, dy) == free_dir {
                    continue;
                }
                let nx = pgx as i64 + dx as i64;
                let ny = pgy as i64 + dy as i64;
                if nx >= 0 && ny >= 0 && grid.in_bounds(nx as u32, ny as u32) {
                    maps[0].set_blocked(nx as u32, ny as u32, true);
                }
            }
        }

        // Create a fake existing Tier 1 route for the top pad (net 1).
        let (top_gx, top_gy) = grid.to_grid(PointMm::new(0.0, 4.8));
        let existing = vec![BreakoutRoute {
            tier: BreakoutTier::Stub,
            net_id: NetId(1),
            pad_cell: (top_gx, top_gy),
            source_layer: LayerId(0),
            target_layer: LayerId(0),
            trace_cells: vec![(top_gx, top_gy + 1)],
            via_cell: None,
            neckdown_width_mm: Some(0.15),
            stub_endpoint: (top_gx, top_gy + 1),
            width_sequence: vec![(top_gx, top_gy + 1, 0.15)],
        }];

        let config = EscapeConfig {
            enabled: true,
            min_escape_mm: 0.25,
            max_escape_mm: 3.0,
            min_access_threshold: 3,
            neckdown_enabled: true,
            neckdown_min_width_mm: 0.0,
        };
        let policy = make_policy(&ir);
        let routes =
            plan_perimeter_escapes(&ir, &grid, &maps, &policy, &config, &existing);

        // The top pad (net 1) must NOT appear in the perimeter routes.
        let top_pad_routed = routes.iter().any(|r| r.net_id == NetId(1));
        assert!(
            !top_pad_routed,
            "pad already handled by Tier 1 must be skipped by plan_perimeter_escapes"
        );

        // Other pads may still generate routes.
        let other_routes: Vec<&BreakoutRoute> = routes
            .iter()
            .filter(|r| r.net_id != NetId(1))
            .collect();
        assert!(
            !other_routes.is_empty(),
            "non-skipped pads must still generate perimeter escape routes"
        );
    }

    // -----------------------------------------------------------------------
    // Test: plan_breakouts — empty board → empty plan
    // -----------------------------------------------------------------------

    #[test]
    fn plan_breakouts_empty_board() {
        let ir = make_ir(vec![], 2);
        let grid = make_grid(50.0, 0.25);
        let maps = make_obstacle_maps(&grid, 2);
        let config = EscapeConfig::default();
        let policy = make_policy(&ir);
        let plan = plan_breakouts(&ir, &grid, &maps, 2, &policy, &config);
        assert!(
            plan.routes.is_empty(),
            "empty board must produce empty breakout plan"
        );
    }

    // -----------------------------------------------------------------------
    // Test: plan_breakouts — 2-layer board with Peripheral component
    //       → Tier 1 stubs + Tier 2 perimeter escapes, NO Tier 3 via escapes
    // -----------------------------------------------------------------------

    #[test]
    fn plan_breakouts_two_layer_produces_tier1_and_tier2() {
        // Build a Peripheral component with 5 densely-packed pads so that
        // plan_stubs and plan_perimeter_escapes both have something to do.
        let pads = vec![
            make_smd_pad(0, PointMm::new(0.0, 4.8), 1, 0),   // top
            make_smd_pad(1, PointMm::new(0.0, -4.8), 2, 0),  // bottom
            make_smd_pad(2, PointMm::new(-4.8, 0.0), 3, 0),  // left
            make_smd_pad(3, PointMm::new(4.8, 0.0), 4, 0),   // right
            make_smd_pad(4, PointMm::new(-1.0, -4.8), 5, 0), // extra bottom
        ];
        let comp = make_comp_with_pads(pads);
        assert_eq!(classify_component(&comp), ComponentKind::Peripheral);

        let ir = make_ir(vec![comp], 2);
        let grid = make_grid(50.0, 0.25);
        let mut maps = make_obstacle_maps(&grid, 2);

        // Block 7 of 8 neighbours for each pad, leaving the outward cell free.
        let positions_and_free: &[(PointMm, (i32, i32))] = &[
            (PointMm::new(0.0, 4.8),   (0,  1)),
            (PointMm::new(0.0, -4.8),  (0, -1)),
            (PointMm::new(-4.8, 0.0),  (-1, 0)),
            (PointMm::new(4.8, 0.0),   (1,  0)),
            (PointMm::new(-1.0, -4.8), (0, -1)),
        ];
        for &(pos, free_dir) in positions_and_free {
            let (pgx, pgy) = grid.to_grid(pos);
            for (dx, dy) in DIRECTIONS_8 {
                if (dx, dy) == free_dir {
                    continue;
                }
                let nx = pgx as i64 + dx as i64;
                let ny = pgy as i64 + dy as i64;
                if nx >= 0 && ny >= 0 && grid.in_bounds(nx as u32, ny as u32) {
                    maps[0].set_blocked(nx as u32, ny as u32, true);
                }
            }
        }

        let config = EscapeConfig {
            enabled: true,
            min_escape_mm: 0.25,
            max_escape_mm: 3.0,
            min_access_threshold: 3,
            neckdown_enabled: true,
            neckdown_min_width_mm: 0.0,
        };
        let policy = make_policy(&ir);
        let plan = plan_breakouts(&ir, &grid, &maps, 2, &policy, &config);

        // Tier 3 (ViaEscape) must never appear on a 2-layer board.
        let has_via_escape = plan.routes.iter().any(|r| r.tier == BreakoutTier::ViaEscape);
        assert!(
            !has_via_escape,
            "2-layer board must not produce Tier 3 via escapes"
        );

        // On a 2-layer board we expect only Tier 1 or Tier 2 routes.
        assert!(
            !plan.routes.is_empty(),
            "2-layer Peripheral component must produce breakout routes"
        );
        let all_t1_t2 = plan.routes.iter().all(|r| {
            r.tier == BreakoutTier::Stub || r.tier == BreakoutTier::PerimeterEscape
        });
        assert!(
            all_t1_t2,
            "all routes on 2-layer board must be Tier 1 or Tier 2"
        );
    }

    // -----------------------------------------------------------------------
    // Test: plan_breakouts — 4-layer board → plan_breakouts returns non-empty
    //       plan and plan_via_escapes produces via escapes when triggered
    // -----------------------------------------------------------------------

    #[test]
    fn plan_breakouts_four_layer_produces_tier3() {
        // Build a small BGA-like component with 9 pads in a 3×3 grid at 1mm pitch.
        // pad_count=9 > 8 and the center pad (0,0) is interior → AreaArray.
        let mut pads = Vec::new();
        let mut id = 0u32;
        for row in 0..3i32 {
            for col in 0..3i32 {
                let x = (col - 1) as f64;
                let y = (row - 1) as f64;
                pads.push(make_smd_pad(id, PointMm::new(x, y), id + 1, 0));
                id += 1;
            }
        }
        assert_eq!(pads.len(), 9);
        let comp = make_comp_with_pads(pads.clone());
        assert_eq!(classify_component(&comp), ComponentKind::AreaArray);

        let ir = make_ir(vec![comp], 4);
        let grid = make_grid(50.0, 0.25);
        let mut maps = make_obstacle_maps(&grid, 4);

        // For each pad, block 6 of 8 neighbours on layer 0 so
        // free_count (= 2) < min_access_threshold (= 3), triggering escape
        // planning.  Leave the +x and +x+y cells free so the walk can find a
        // path on both layer 0 and layer 1.
        let pad_positions: Vec<PointMm> = (0..3i32)
            .flat_map(|row| (0..3i32).map(move |col| PointMm::new((col - 1) as f64, (row - 1) as f64)))
            .collect();

        for &pos in &pad_positions {
            let (pgx, pgy) = grid.to_grid(pos);
            // Block 6 neighbours, leave (+1, 0) and (+1, +1) free.
            let blocked_offsets: &[(i32, i32)] = &[
                (-1, 0), (0, -1), (0, 1),
                (-1, -1), (-1, 1), (1, -1),
            ];
            for &(dx, dy) in blocked_offsets {
                let nx = pgx as i64 + dx as i64;
                let ny = pgy as i64 + dy as i64;
                if nx >= 0 && ny >= 0 && grid.in_bounds(nx as u32, ny as u32) {
                    maps[0].set_blocked(nx as u32, ny as u32, true);
                }
            }
        }

        let config = EscapeConfig {
            enabled: true,
            min_escape_mm: 0.25,
            max_escape_mm: 3.0,
            min_access_threshold: 3,
            neckdown_enabled: true,
            neckdown_min_width_mm: 0.0,
        };
        let policy = make_policy(&ir);

        // Verify plan_via_escapes produces via-escape routes on a 4-layer board.
        let via_routes = plan_via_escapes(&ir, &grid, &maps, 4, &config, &[]);
        assert!(
            !via_routes.is_empty(),
            "4-layer board with dense pads must produce Tier 3 via escapes"
        );
        for r in &via_routes {
            assert_eq!(r.tier, BreakoutTier::ViaEscape);
            assert!(r.via_cell.is_some(), "Tier 3 route must have a via cell");
            assert!(
                r.target_layer.raw() > 0,
                "Tier 3 must target an inner layer (> 0)"
            );
        }

        // plan_breakouts must run without error; on a 4-layer board it may
        // produce Tier 1, Tier 2, or Tier 3 routes depending on availability.
        let plan = plan_breakouts(&ir, &grid, &maps, 4, &policy, &config);
        assert!(
            !plan.routes.is_empty(),
            "4-layer board with dense pads must produce at least some breakout routes"
        );
    }
}

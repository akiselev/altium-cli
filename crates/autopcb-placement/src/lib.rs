pub mod simulated_annealing;
pub mod swap;

use std::collections::{BTreeMap, HashMap, HashSet};

use autopcb_ir::{PcbIr, PointMm};
use serde::{Deserialize, Serialize};
use simulated_annealing::SAConfig;
use solverang::constraint::Constraint;
use solverang::entity::Entity;
use solverang::id::{ConstraintId, EntityId, ParamId};
use solverang::param::ParamStore;
use solverang::solver::LMConfig;
use solverang::system::{ConstraintSystem, SystemConfig, SystemStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementConfig {
    pub gamma_start: f64,
    pub gamma_end: f64,
    pub max_iters: usize,
    pub ratsnest_weight: f64,
    pub default_clearance_mm: f64,
    pub board_edge_clearance_mm: f64,
    pub grid_snap_mm: Option<f64>,
    /// When set, run SA refinement (Phase 3) after legalization.
    #[serde(default)]
    pub sa_config: Option<SAConfig>,
    /// Run greedy part swap pass (Phase 2.5) after legalization if swap data is available.
    #[serde(default)]
    pub allow_part_swap: bool,
    /// Run greedy pin swap sweep (Phase 4.5) after final refinement if swap data is available.
    #[serde(default)]
    pub allow_pin_swap: bool,
}

impl Default for PlacementConfig {
    fn default() -> Self {
        Self {
            gamma_start: 2.0,
            gamma_end: 10.0,
            max_iters: 250,
            ratsnest_weight: 0.01,
            default_clearance_mm: 0.5,
            board_edge_clearance_mm: 0.0,
            grid_snap_mm: None,
            sa_config: None,
            allow_part_swap: false,
            allow_pin_swap: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlacementEdge {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Direction {
    LeftOf,
    RightOf,
    Above,
    Below,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RectRegion {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserConstraint {
    EdgePlacement {
        designator: String,
        edge: PlacementEdge,
        inset_mm: f64,
    },
    Directional {
        a: String,
        b: String,
        direction: Direction,
        gap_mm: f64,
    },
    Near {
        a: String,
        b: String,
        max_distance_mm: f64,
    },
    RegionContainment {
        designator: String,
        region: RectRegion,
    },
    FixedPosition {
        designator: String,
        x_mm: f64,
        y_mm: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementIterationSnapshot {
    pub phase: String,
    pub components: Vec<PlacementComponentState>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementComponentState {
    pub designator: String,
    pub x_mm: f64,
    pub y_mm: f64,
    pub rotation_deg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementResult {
    pub status: String,
    pub total_iterations: usize,
    pub duration_ms: u128,
    pub components: Vec<PlacementComponentState>,
    pub snapshots: Vec<PlacementIterationSnapshot>,
    pub hpwl_estimate_mm: f64,
    pub overlap_violations: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum PlacementError {
    #[error("constraint references unknown component '{0}'")]
    UnknownComponent(String),
    #[error("empty board outline bounds")]
    InvalidBoardBounds,
    #[error("no movable components in IR")]
    NoComponents,
}

#[derive(Debug, Clone)]
struct PcbComponentEntity {
    id: EntityId,
    x: ParamId,
    y: ParamId,
    theta: ParamId,
    params: [ParamId; 3],
    designator: String,
    half_w: f64,
    half_h: f64,
}

impl Entity for PcbComponentEntity {
    fn id(&self) -> EntityId {
        self.id
    }
    fn params(&self) -> &[ParamId] {
        &self.params
    }
    fn name(&self) -> &str {
        &self.designator
    }
}

#[derive(Debug, Clone)]
struct PcbBoardEntity {
    id: EntityId,
    params: [ParamId; 0],
}

impl Entity for PcbBoardEntity {
    fn id(&self) -> EntityId {
        self.id
    }
    fn params(&self) -> &[ParamId] {
        &self.params
    }
    fn name(&self) -> &str {
        "PcbBoardOutline"
    }
}

#[derive(Debug, Clone)]
struct BoardContainment {
    id: ConstraintId,
    entity: EntityId,
    x: ParamId,
    y: ParamId,
    s: [ParamId; 4],
    half_w: f64,
    half_h: f64,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    params: [ParamId; 6],
}

impl Constraint for BoardContainment {
    fn id(&self) -> ConstraintId {
        self.id
    }
    fn name(&self) -> &str {
        "BoardContainment"
    }
    fn entity_ids(&self) -> &[EntityId] {
        std::slice::from_ref(&self.entity)
    }
    fn param_ids(&self) -> &[ParamId] {
        &self.params
    }
    fn equation_count(&self) -> usize {
        4
    }

    fn residuals(&self, store: &ParamStore) -> Vec<f64> {
        let x = store.get(self.x);
        let y = store.get(self.y);
        let g0 = (x - self.half_w) - self.min_x;
        let g1 = self.max_x - (x + self.half_w);
        let g2 = (y - self.half_h) - self.min_y;
        let g3 = self.max_y - (y + self.half_h);
        vec![
            g0 - store.get(self.s[0]).powi(2),
            g1 - store.get(self.s[1]).powi(2),
            g2 - store.get(self.s[2]).powi(2),
            g3 - store.get(self.s[3]).powi(2),
        ]
    }

    fn jacobian(&self, store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        vec![
            (0, self.x, 1.0),
            (0, self.s[0], -2.0 * store.get(self.s[0])),
            (1, self.x, -1.0),
            (1, self.s[1], -2.0 * store.get(self.s[1])),
            (2, self.y, 1.0),
            (2, self.s[2], -2.0 * store.get(self.s[2])),
            (3, self.y, -1.0),
            (3, self.s[3], -2.0 * store.get(self.s[3])),
        ]
    }
}

#[derive(Debug, Clone)]
struct ComponentClearance {
    id: ConstraintId,
    entities: [EntityId; 2],
    x1: ParamId,
    y1: ParamId,
    x2: ParamId,
    y2: ParamId,
    s: ParamId,
    combined_hw: f64,
    combined_hh: f64,
    params: [ParamId; 5],
}

impl Constraint for ComponentClearance {
    fn id(&self) -> ConstraintId {
        self.id
    }
    fn name(&self) -> &str {
        "ComponentClearance"
    }
    fn entity_ids(&self) -> &[EntityId] {
        &self.entities
    }
    fn param_ids(&self) -> &[ParamId] {
        &self.params
    }
    fn equation_count(&self) -> usize {
        1
    }

    fn residuals(&self, store: &ParamStore) -> Vec<f64> {
        let dx = store.get(self.x2) - store.get(self.x1);
        let dy = store.get(self.y2) - store.get(self.y1);
        let g = (dx / self.combined_hw).powi(2) + (dy / self.combined_hh).powi(2) - 1.0;
        vec![g - store.get(self.s).powi(2)]
    }

    fn jacobian(&self, store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        let dx = store.get(self.x2) - store.get(self.x1);
        let dy = store.get(self.y2) - store.get(self.y1);
        let hw2 = self.combined_hw * self.combined_hw;
        let hh2 = self.combined_hh * self.combined_hh;
        vec![
            (0, self.x1, -2.0 * dx / hw2),
            (0, self.y1, -2.0 * dy / hh2),
            (0, self.x2, 2.0 * dx / hw2),
            (0, self.y2, 2.0 * dy / hh2),
            (0, self.s, -2.0 * store.get(self.s)),
        ]
    }
}

#[derive(Debug, Clone)]
struct EdgePlacementConstraint {
    id: ConstraintId,
    entity: EntityId,
    x: ParamId,
    y: ParamId,
    edge: PlacementEdge,
    inset: f64,
    half_w: f64,
    half_h: f64,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    params: [ParamId; 2],
}

impl Constraint for EdgePlacementConstraint {
    fn id(&self) -> ConstraintId {
        self.id
    }
    fn name(&self) -> &str {
        "EdgePlacement"
    }
    fn entity_ids(&self) -> &[EntityId] {
        std::slice::from_ref(&self.entity)
    }
    fn param_ids(&self) -> &[ParamId] {
        &self.params
    }
    fn equation_count(&self) -> usize {
        1
    }

    fn residuals(&self, store: &ParamStore) -> Vec<f64> {
        let x = store.get(self.x);
        let y = store.get(self.y);
        let r = match self.edge {
            PlacementEdge::Top => y + self.half_h - (self.max_y - self.inset),
            PlacementEdge::Bottom => y - self.half_h - (self.min_y + self.inset),
            PlacementEdge::Left => x - self.half_w - (self.min_x + self.inset),
            PlacementEdge::Right => x + self.half_w - (self.max_x - self.inset),
        };
        vec![r]
    }

    fn jacobian(&self, _store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        match self.edge {
            PlacementEdge::Top | PlacementEdge::Bottom => vec![(0, self.y, 1.0)],
            PlacementEdge::Left | PlacementEdge::Right => vec![(0, self.x, 1.0)],
        }
    }
}

#[derive(Debug, Clone)]
struct DirectionalOrderingConstraint {
    id: ConstraintId,
    entities: [EntityId; 2],
    a_x: ParamId,
    a_y: ParamId,
    b_x: ParamId,
    b_y: ParamId,
    slack: ParamId,
    direction: Direction,
    gap: f64,
    a_half_w: f64,
    a_half_h: f64,
    b_half_w: f64,
    b_half_h: f64,
    params: [ParamId; 5],
}

impl Constraint for DirectionalOrderingConstraint {
    fn id(&self) -> ConstraintId {
        self.id
    }
    fn name(&self) -> &str {
        "DirectionalOrdering"
    }
    fn entity_ids(&self) -> &[EntityId] {
        &self.entities
    }
    fn param_ids(&self) -> &[ParamId] {
        &self.params
    }
    fn equation_count(&self) -> usize {
        1
    }

    fn residuals(&self, store: &ParamStore) -> Vec<f64> {
        let ax = store.get(self.a_x);
        let ay = store.get(self.a_y);
        let bx = store.get(self.b_x);
        let by = store.get(self.b_y);
        let g = match self.direction {
            Direction::LeftOf => (bx - self.b_half_w) - (ax + self.a_half_w) - self.gap,
            Direction::RightOf => (ax - self.a_half_w) - (bx + self.b_half_w) - self.gap,
            Direction::Above => (ay - self.a_half_h) - (by + self.b_half_h) - self.gap,
            Direction::Below => (by - self.b_half_h) - (ay + self.a_half_h) - self.gap,
        };
        vec![g - store.get(self.slack).powi(2)]
    }

    fn jacobian(&self, store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        let mut j = vec![(0, self.slack, -2.0 * store.get(self.slack))];
        match self.direction {
            Direction::LeftOf => {
                j.push((0, self.a_x, -1.0));
                j.push((0, self.b_x, 1.0));
            }
            Direction::RightOf => {
                j.push((0, self.a_x, 1.0));
                j.push((0, self.b_x, -1.0));
            }
            Direction::Above => {
                j.push((0, self.a_y, 1.0));
                j.push((0, self.b_y, -1.0));
            }
            Direction::Below => {
                j.push((0, self.a_y, -1.0));
                j.push((0, self.b_y, 1.0));
            }
        }
        j
    }
}

#[derive(Debug, Clone)]
struct NearConstraint {
    id: ConstraintId,
    entities: [EntityId; 2],
    a_x: ParamId,
    a_y: ParamId,
    b_x: ParamId,
    b_y: ParamId,
    slack: ParamId,
    max_dist_sq: f64,
    params: [ParamId; 5],
}

impl Constraint for NearConstraint {
    fn id(&self) -> ConstraintId {
        self.id
    }
    fn name(&self) -> &str {
        "NearConstraint"
    }
    fn entity_ids(&self) -> &[EntityId] {
        &self.entities
    }
    fn param_ids(&self) -> &[ParamId] {
        &self.params
    }
    fn equation_count(&self) -> usize {
        1
    }

    fn residuals(&self, store: &ParamStore) -> Vec<f64> {
        let dx = store.get(self.b_x) - store.get(self.a_x);
        let dy = store.get(self.b_y) - store.get(self.a_y);
        let g = self.max_dist_sq - (dx * dx + dy * dy);
        vec![g - store.get(self.slack).powi(2)]
    }

    fn jacobian(&self, store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        let dx = store.get(self.b_x) - store.get(self.a_x);
        let dy = store.get(self.b_y) - store.get(self.a_y);
        vec![
            (0, self.a_x, 2.0 * dx),
            (0, self.a_y, 2.0 * dy),
            (0, self.b_x, -2.0 * dx),
            (0, self.b_y, -2.0 * dy),
            (0, self.slack, -2.0 * store.get(self.slack)),
        ]
    }
}

#[derive(Debug, Clone)]
struct RegionContainmentConstraint {
    id: ConstraintId,
    entity: EntityId,
    x: ParamId,
    y: ParamId,
    s: [ParamId; 4],
    region: RectRegion,
    params: [ParamId; 6],
}

impl Constraint for RegionContainmentConstraint {
    fn id(&self) -> ConstraintId {
        self.id
    }
    fn name(&self) -> &str {
        "RegionContainment"
    }
    fn entity_ids(&self) -> &[EntityId] {
        std::slice::from_ref(&self.entity)
    }
    fn param_ids(&self) -> &[ParamId] {
        &self.params
    }
    fn equation_count(&self) -> usize {
        4
    }

    fn residuals(&self, store: &ParamStore) -> Vec<f64> {
        let x = store.get(self.x);
        let y = store.get(self.y);
        vec![
            (x - self.region.min_x) - store.get(self.s[0]).powi(2),
            (self.region.max_x - x) - store.get(self.s[1]).powi(2),
            (y - self.region.min_y) - store.get(self.s[2]).powi(2),
            (self.region.max_y - y) - store.get(self.s[3]).powi(2),
        ]
    }

    fn jacobian(&self, store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        vec![
            (0, self.x, 1.0),
            (0, self.s[0], -2.0 * store.get(self.s[0])),
            (1, self.x, -1.0),
            (1, self.s[1], -2.0 * store.get(self.s[1])),
            (2, self.y, 1.0),
            (2, self.s[2], -2.0 * store.get(self.s[2])),
            (3, self.y, -1.0),
            (3, self.s[3], -2.0 * store.get(self.s[3])),
        ]
    }
}

#[derive(Debug, Clone)]
struct FixedPositionConstraint {
    id: ConstraintId,
    entity: EntityId,
    x: ParamId,
    y: ParamId,
    tx: f64,
    ty: f64,
    params: [ParamId; 2],
}

impl Constraint for FixedPositionConstraint {
    fn id(&self) -> ConstraintId {
        self.id
    }
    fn name(&self) -> &str {
        "FixedPosition"
    }
    fn entity_ids(&self) -> &[EntityId] {
        std::slice::from_ref(&self.entity)
    }
    fn param_ids(&self) -> &[ParamId] {
        &self.params
    }
    fn equation_count(&self) -> usize {
        2
    }

    fn residuals(&self, store: &ParamStore) -> Vec<f64> {
        vec![store.get(self.x) - self.tx, store.get(self.y) - self.ty]
    }

    fn jacobian(&self, _store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        vec![(0, self.x, 1.0), (1, self.y, 1.0)]
    }
}

#[derive(Debug, Clone)]
struct RotationDiscretizeConstraint {
    id: ConstraintId,
    entity: EntityId,
    theta: ParamId,
    params: [ParamId; 1],
}

impl Constraint for RotationDiscretizeConstraint {
    fn id(&self) -> ConstraintId {
        self.id
    }
    fn name(&self) -> &str {
        "RotationDiscretize"
    }
    fn entity_ids(&self) -> &[EntityId] {
        std::slice::from_ref(&self.entity)
    }
    fn param_ids(&self) -> &[ParamId] {
        &self.params
    }
    fn equation_count(&self) -> usize {
        1
    }

    fn residuals(&self, store: &ParamStore) -> Vec<f64> {
        let theta = store.get(self.theta);
        vec![(2.0 * theta).sin()]
    }

    fn jacobian(&self, store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        let theta = store.get(self.theta);
        vec![(0, self.theta, 2.0 * (2.0 * theta).cos())]
    }
}

#[derive(Debug, Clone)]
struct HpwlPin {
    comp_x: ParamId,
    comp_y: ParamId,
    comp_theta: ParamId,
    local_x: f64,
    local_y: f64,
}

#[derive(Debug, Clone)]
struct SmoothHpwlConstraint {
    id: ConstraintId,
    entity_ids: Vec<EntityId>,
    param_ids: Vec<ParamId>,
    pins: Vec<HpwlPin>,
    gamma: f64,
    weight: f64,
}

impl SmoothHpwlConstraint {
    fn compute_world_positions(&self, store: &ParamStore) -> (Vec<f64>, Vec<f64>) {
        let mut xs = Vec::with_capacity(self.pins.len());
        let mut ys = Vec::with_capacity(self.pins.len());
        for pin in &self.pins {
            let cx = store.get(pin.comp_x);
            let cy = store.get(pin.comp_y);
            let theta = store.get(pin.comp_theta);
            let (sin_t, cos_t) = theta.sin_cos();
            xs.push(cx + pin.local_x * cos_t - pin.local_y * sin_t);
            ys.push(cy + pin.local_x * sin_t + pin.local_y * cos_t);
        }
        (xs, ys)
    }
}

impl Constraint for SmoothHpwlConstraint {
    fn id(&self) -> ConstraintId {
        self.id
    }
    fn name(&self) -> &str {
        "SmoothHPWL"
    }
    fn entity_ids(&self) -> &[EntityId] {
        &self.entity_ids
    }
    fn param_ids(&self) -> &[ParamId] {
        &self.param_ids
    }
    fn equation_count(&self) -> usize {
        2
    }

    fn residuals(&self, store: &ParamStore) -> Vec<f64> {
        let (xs, ys) = self.compute_world_positions(store);
        let x_span = lse_max(&xs, self.gamma) - lse_min(&xs, self.gamma);
        let y_span = lse_max(&ys, self.gamma) - lse_min(&ys, self.gamma);
        vec![self.weight * x_span, self.weight * y_span]
    }

    fn jacobian(&self, store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        let (xs, ys) = self.compute_world_positions(store);
        let smx_max = softmax(&xs, self.gamma);
        let smx_min = softmax_neg(&xs, self.gamma);
        let smy_max = softmax(&ys, self.gamma);
        let smy_min = softmax_neg(&ys, self.gamma);

        let mut out = Vec::with_capacity(self.pins.len() * 6);
        for (i, pin) in self.pins.iter().enumerate() {
            let theta = store.get(pin.comp_theta);
            let (sin_t, cos_t) = theta.sin_cos();

            let d_hx_d_wx = smx_max[i] - smx_min[i];
            let d_hy_d_wy = smy_max[i] - smy_min[i];

            out.push((0, pin.comp_x, self.weight * d_hx_d_wx));
            out.push((1, pin.comp_y, self.weight * d_hy_d_wy));

            let d_wx_d_theta = -pin.local_x * sin_t - pin.local_y * cos_t;
            let d_wy_d_theta = pin.local_x * cos_t - pin.local_y * sin_t;
            let d0 = self.weight * d_hx_d_wx * d_wx_d_theta;
            let d1 = self.weight * d_hy_d_wy * d_wy_d_theta;
            if d0.abs() > 1e-15 {
                out.push((0, pin.comp_theta, d0));
            }
            if d1.abs() > 1e-15 {
                out.push((1, pin.comp_theta, d1));
            }
        }
        out
    }

    fn is_soft(&self) -> bool {
        true
    }
    fn weight(&self) -> f64 {
        self.weight
    }
}

fn lse_max(values: &[f64], gamma: f64) -> f64 {
    let max_v = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let sum: f64 = values.iter().map(|v| ((v - max_v) * gamma).exp()).sum();
    max_v + sum.ln() / gamma
}

fn lse_min(values: &[f64], gamma: f64) -> f64 {
    let neg: Vec<f64> = values.iter().map(|v| -v).collect();
    -lse_max(&neg, gamma)
}

fn softmax(values: &[f64], gamma: f64) -> Vec<f64> {
    let max_v = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = values.iter().map(|v| ((v - max_v) * gamma).exp()).collect();
    let sum: f64 = exps.iter().sum();
    exps.into_iter().map(|v| v / sum).collect()
}

fn softmax_neg(values: &[f64], gamma: f64) -> Vec<f64> {
    let min_v = values.iter().copied().fold(f64::INFINITY, f64::min);
    let exps: Vec<f64> = values
        .iter()
        .map(|v| (-(v - min_v) * gamma).exp())
        .collect();
    let sum: f64 = exps.iter().sum();
    exps.into_iter().map(|v| v / sum).collect()
}

#[derive(Debug, Clone)]
struct ComponentRuntime {
    entity: PcbComponentEntity,
    pads: Vec<(String, PointMm)>,
}

fn named_region(
    region: &str,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
) -> Option<RectRegion> {
    let mid_x = (min_x + max_x) / 2.0;
    let mid_y = (min_y + max_y) / 2.0;
    Some(match region {
        "center" => {
            let w = (max_x - min_x) * 0.25;
            let h = (max_y - min_y) * 0.25;
            RectRegion {
                min_x: mid_x - w,
                max_x: mid_x + w,
                min_y: mid_y - h,
                max_y: mid_y + h,
            }
        }
        "top_half" => RectRegion {
            min_x,
            max_x,
            min_y: mid_y,
            max_y,
        },
        "bottom_half" => RectRegion {
            min_x,
            max_x,
            min_y,
            max_y: mid_y,
        },
        "left_half" => RectRegion {
            min_x,
            max_x: mid_x,
            min_y,
            max_y,
        },
        "right_half" => RectRegion {
            min_x: mid_x,
            max_x,
            min_y,
            max_y,
        },
        "quadrant_tl" => RectRegion {
            min_x,
            max_x: mid_x,
            min_y: mid_y,
            max_y,
        },
        "quadrant_tr" => RectRegion {
            min_x: mid_x,
            max_x,
            min_y: mid_y,
            max_y,
        },
        "quadrant_bl" => RectRegion {
            min_x,
            max_x: mid_x,
            min_y,
            max_y: mid_y,
        },
        "quadrant_br" => RectRegion {
            min_x: mid_x,
            max_x,
            min_y,
            max_y: mid_y,
        },
        _ => return None,
    })
}

pub fn solve_placement(
    ir: &PcbIr,
    user_constraints: &[UserConstraint],
    config: &PlacementConfig,
) -> Result<PlacementResult, PlacementError> {
    if ir.components.is_empty() {
        return Err(PlacementError::NoComponents);
    }

    let bounds = ir.board.bounds;
    if !(bounds.max.x > bounds.min.x && bounds.max.y > bounds.min.y) {
        return Err(PlacementError::InvalidBoardBounds);
    }

    let mut snapshots = Vec::new();

    let mut system = ConstraintSystem::with_config(SystemConfig {
        lm_config: LMConfig::robust().with_patience(config.max_iters.max(10)),
        solver_config: Default::default(),
    });

    let board_eid = system.alloc_entity_id();
    system.add_entity(Box::new(PcbBoardEntity {
        id: board_eid,
        params: [],
    }));

    let mut runtimes = Vec::<ComponentRuntime>::with_capacity(ir.components.len());
    let mut designator_to_idx = BTreeMap::<String, usize>::new();

    // Pre-scatter: if components are outside the board or all co-located at the same
    // point, distribute them on a grid inside the board. This prevents a zero-gradient
    // Jacobian that causes the LM solver to stall.
    let margin = config.board_edge_clearance_mm.max(1.0);
    let scatter_positions = {
        let n = ir.components.len();
        let all_same_pos = {
            let mut iter = ir.components.values();
            let first = iter.next().map(|c| (c.position.x, c.position.y));
            first.is_some()
                && iter.all(|c| {
                    (c.position.x - first.unwrap().0).abs() < 0.01
                        && (c.position.y - first.unwrap().1).abs() < 0.01
                })
        };
        let any_outside = ir.components.values().any(|c| {
            c.position.x < bounds.min.x
                || c.position.x > bounds.max.x
                || c.position.y < bounds.min.y
                || c.position.y > bounds.max.y
        });
        if all_same_pos || any_outside {
            let cols = (n as f64).sqrt().ceil() as usize;
            let rows = (n + cols - 1) / cols;
            let w = (bounds.max.x - bounds.min.x) - 2.0 * margin;
            let h = (bounds.max.y - bounds.min.y) - 2.0 * margin;
            let dx = w / cols.max(1) as f64;
            let dy = h / rows.max(1) as f64;
            let mut positions = Vec::with_capacity(n);
            for i in 0..n {
                let col = i % cols;
                let row = i / cols;
                positions.push((
                    bounds.min.x + margin + dx * (col as f64 + 0.5),
                    bounds.min.y + margin + dy * (row as f64 + 0.5),
                ));
            }
            Some(positions)
        } else {
            None
        }
    };

    for (comp_idx, (_id, comp)) in ir.components.iter().enumerate() {
        let eid = system.alloc_entity_id();
        let (init_x, init_y) = if let Some(ref positions) = scatter_positions {
            positions[comp_idx]
        } else {
            (comp.position.x, comp.position.y)
        };
        let x = system.alloc_param(init_x, eid);
        let y = system.alloc_param(init_y, eid);
        let theta = system.alloc_param(comp.rotation.to_radians(), eid);
        let half_w = comp.local_bounds.width() * 0.5;
        let half_h = comp.local_bounds.height() * 0.5;
        let entity = PcbComponentEntity {
            id: eid,
            x,
            y,
            theta,
            params: [x, y, theta],
            designator: comp.designator.clone(),
            half_w,
            half_h,
        };
        system.add_entity(Box::new(entity.clone()));

        let pads = comp
            .pads
            .iter()
            .filter_map(|pad| {
                pad.net
                    .map(|nid| (ir.nets[nid].name.clone(), pad.local_position))
            })
            .collect();

        designator_to_idx.insert(comp.designator.clone(), runtimes.len());
        runtimes.push(ComponentRuntime { entity, pads });
    }

    // Base hard constraints.
    for comp in &runtimes {
        let cid = system.alloc_constraint_id();
        let s0 = system.alloc_param(0.01, comp.entity.id);
        let s1 = system.alloc_param(0.01, comp.entity.id);
        let s2 = system.alloc_param(0.01, comp.entity.id);
        let s3 = system.alloc_param(0.01, comp.entity.id);
        system.add_constraint(Box::new(BoardContainment {
            id: cid,
            entity: comp.entity.id,
            x: comp.entity.x,
            y: comp.entity.y,
            s: [s0, s1, s2, s3],
            half_w: comp.entity.half_w + config.board_edge_clearance_mm,
            half_h: comp.entity.half_h + config.board_edge_clearance_mm,
            min_x: bounds.min.x,
            min_y: bounds.min.y,
            max_x: bounds.max.x,
            max_y: bounds.max.y,
            params: [comp.entity.x, comp.entity.y, s0, s1, s2, s3],
        }));

        let rcid = system.alloc_constraint_id();
        system.add_constraint(Box::new(RotationDiscretizeConstraint {
            id: rcid,
            entity: comp.entity.id,
            theta: comp.entity.theta,
            params: [comp.entity.theta],
        }));
    }

    for i in 0..runtimes.len() {
        for j in (i + 1)..runtimes.len() {
            let a = &runtimes[i].entity;
            let b = &runtimes[j].entity;
            let slack = system.alloc_param(0.01, a.id);
            let cid = system.alloc_constraint_id();
            system.add_constraint(Box::new(ComponentClearance {
                id: cid,
                entities: [a.id, b.id],
                x1: a.x,
                y1: a.y,
                x2: b.x,
                y2: b.y,
                s: slack,
                combined_hw: a.half_w + b.half_w + config.default_clearance_mm,
                combined_hh: a.half_h + b.half_h + config.default_clearance_mm,
                params: [a.x, a.y, b.x, b.y, slack],
            }));
        }
    }

    // User constraints.
    for uc in user_constraints {
        match uc {
            UserConstraint::EdgePlacement {
                designator,
                edge,
                inset_mm,
            } => {
                let idx = *designator_to_idx
                    .get(designator)
                    .ok_or_else(|| PlacementError::UnknownComponent(designator.clone()))?;
                let comp = &runtimes[idx].entity;
                let cid = system.alloc_constraint_id();
                system.add_constraint(Box::new(EdgePlacementConstraint {
                    id: cid,
                    entity: comp.id,
                    x: comp.x,
                    y: comp.y,
                    edge: edge.clone(),
                    inset: *inset_mm,
                    half_w: comp.half_w,
                    half_h: comp.half_h,
                    min_x: bounds.min.x,
                    min_y: bounds.min.y,
                    max_x: bounds.max.x,
                    max_y: bounds.max.y,
                    params: [comp.x, comp.y],
                }));
            }
            UserConstraint::Directional {
                a,
                b,
                direction,
                gap_mm,
            } => {
                let ia = *designator_to_idx
                    .get(a)
                    .ok_or_else(|| PlacementError::UnknownComponent(a.clone()))?;
                let ib = *designator_to_idx
                    .get(b)
                    .ok_or_else(|| PlacementError::UnknownComponent(b.clone()))?;
                let ca = &runtimes[ia].entity;
                let cb = &runtimes[ib].entity;
                let slack = system.alloc_param(0.01, ca.id);
                let cid = system.alloc_constraint_id();
                system.add_constraint(Box::new(DirectionalOrderingConstraint {
                    id: cid,
                    entities: [ca.id, cb.id],
                    a_x: ca.x,
                    a_y: ca.y,
                    b_x: cb.x,
                    b_y: cb.y,
                    slack,
                    direction: direction.clone(),
                    gap: *gap_mm,
                    a_half_w: ca.half_w,
                    a_half_h: ca.half_h,
                    b_half_w: cb.half_w,
                    b_half_h: cb.half_h,
                    params: [ca.x, ca.y, cb.x, cb.y, slack],
                }));
            }
            UserConstraint::Near {
                a,
                b,
                max_distance_mm,
            } => {
                let ia = *designator_to_idx
                    .get(a)
                    .ok_or_else(|| PlacementError::UnknownComponent(a.clone()))?;
                let ib = *designator_to_idx
                    .get(b)
                    .ok_or_else(|| PlacementError::UnknownComponent(b.clone()))?;
                let ca = &runtimes[ia].entity;
                let cb = &runtimes[ib].entity;
                let slack = system.alloc_param(0.01, ca.id);
                let cid = system.alloc_constraint_id();
                system.add_constraint(Box::new(NearConstraint {
                    id: cid,
                    entities: [ca.id, cb.id],
                    a_x: ca.x,
                    a_y: ca.y,
                    b_x: cb.x,
                    b_y: cb.y,
                    slack,
                    max_dist_sq: max_distance_mm * max_distance_mm,
                    params: [ca.x, ca.y, cb.x, cb.y, slack],
                }));
            }
            UserConstraint::RegionContainment { designator, region } => {
                let idx = *designator_to_idx
                    .get(designator)
                    .ok_or_else(|| PlacementError::UnknownComponent(designator.clone()))?;
                let comp = &runtimes[idx].entity;
                let s0 = system.alloc_param(0.01, comp.id);
                let s1 = system.alloc_param(0.01, comp.id);
                let s2 = system.alloc_param(0.01, comp.id);
                let s3 = system.alloc_param(0.01, comp.id);
                let cid = system.alloc_constraint_id();
                system.add_constraint(Box::new(RegionContainmentConstraint {
                    id: cid,
                    entity: comp.id,
                    x: comp.x,
                    y: comp.y,
                    s: [s0, s1, s2, s3],
                    region: region.clone(),
                    params: [comp.x, comp.y, s0, s1, s2, s3],
                }));
            }
            UserConstraint::FixedPosition {
                designator,
                x_mm,
                y_mm,
            } => {
                let idx = *designator_to_idx
                    .get(designator)
                    .ok_or_else(|| PlacementError::UnknownComponent(designator.clone()))?;
                let comp = &runtimes[idx].entity;
                let cid = system.alloc_constraint_id();
                system.add_constraint(Box::new(FixedPositionConstraint {
                    id: cid,
                    entity: comp.id,
                    x: comp.x,
                    y: comp.y,
                    tx: *x_mm,
                    ty: *y_mm,
                    params: [comp.x, comp.y],
                }));
            }
        }
    }

    // HPWL constraints by net.
    let mut net_to_pins = BTreeMap::<String, Vec<HpwlPin>>::new();
    for comp in &runtimes {
        for (net, local) in &comp.pads {
            net_to_pins.entry(net.clone()).or_default().push(HpwlPin {
                comp_x: comp.entity.x,
                comp_y: comp.entity.y,
                comp_theta: comp.entity.theta,
                local_x: local.x,
                local_y: local.y,
            });
        }
    }

    for pins in net_to_pins.values() {
        if pins.len() < 2 {
            continue;
        }
        let mut entities = HashSet::<EntityId>::new();
        let mut params = HashSet::<ParamId>::new();
        for p in pins {
            for comp in &runtimes {
                if comp.entity.x == p.comp_x {
                    entities.insert(comp.entity.id);
                }
            }
            params.insert(p.comp_x);
            params.insert(p.comp_y);
            params.insert(p.comp_theta);
        }
        let cid = system.alloc_constraint_id();
        system.add_constraint(Box::new(SmoothHpwlConstraint {
            id: cid,
            entity_ids: entities.into_iter().collect(),
            param_ids: params.into_iter().collect(),
            pins: pins.clone(),
            gamma: config.gamma_start,
            weight: config.ratsnest_weight,
        }));
    }

    snapshots.push(snapshot_from_system("initial", &system, &runtimes, None));

    let first = system.solve();
    snapshots.push(snapshot_from_system(
        "continuous",
        &system,
        &runtimes,
        Some(status_str(&first.status).to_string()),
    ));

    // Snap rotations to nearest 90.
    for comp in &runtimes {
        let theta = system.get_param(comp.entity.theta);
        let deg = theta.to_degrees();
        let snapped = ((deg / 90.0).round() as i32).rem_euclid(4) as f64 * 90.0;
        system.set_param(comp.entity.theta, snapped.to_radians());
        system.fix_param(comp.entity.theta);
    }

    snapshots.push(snapshot_from_system("snapped", &system, &runtimes, None));

    // Re-solve with higher gamma by adding additional constraints to sharpen HPWL.
    if config.gamma_end > config.gamma_start {
        for pins in net_to_pins.values() {
            if pins.len() < 2 {
                continue;
            }
            let mut entities = HashSet::<EntityId>::new();
            let mut params = HashSet::<ParamId>::new();
            for p in pins {
                for comp in &runtimes {
                    if comp.entity.x == p.comp_x {
                        entities.insert(comp.entity.id);
                    }
                }
                params.insert(p.comp_x);
                params.insert(p.comp_y);
                params.insert(p.comp_theta);
            }
            let cid = system.alloc_constraint_id();
            system.add_constraint(Box::new(SmoothHpwlConstraint {
                id: cid,
                entity_ids: entities.into_iter().collect(),
                param_ids: params.into_iter().collect(),
                pins: pins.clone(),
                gamma: config.gamma_end,
                weight: config.ratsnest_weight,
            }));
        }
    }

    let second = system.solve();

    if let Some(grid) = config.grid_snap_mm {
        for comp in &runtimes {
            let x = system.get_param(comp.entity.x);
            let y = system.get_param(comp.entity.y);
            system.set_param(comp.entity.x, (x / grid).round() * grid);
            system.set_param(comp.entity.y, (y / grid).round() * grid);
        }
    }

    let overlaps = greedy_legalize_overlaps(&mut system, &runtimes, config.default_clearance_mm);
    snapshots.push(snapshot_from_system(
        "legalized",
        &system,
        &runtimes,
        Some(format!("shifted {} overlaps", overlaps)),
    ));

    let mut components = Vec::with_capacity(runtimes.len());
    for comp in &runtimes {
        components.push(PlacementComponentState {
            designator: comp.entity.designator.clone(),
            x_mm: system.get_param(comp.entity.x),
            y_mm: system.get_param(comp.entity.y),
            rotation_deg: system.get_param(comp.entity.theta).to_degrees(),
        });
    }
    components.sort_by(|a, b| a.designator.cmp(&b.designator));

    let hpwl = estimate_hpwl(&components, &runtimes, &net_to_pins);
    let overlap_count = count_overlaps(&system, &runtimes, config.default_clearance_mm);

    let mut phase2_result = PlacementResult {
        status: status_str(&second.status).to_string(),
        total_iterations: first.total_iterations + second.total_iterations,
        duration_ms: first.duration.as_millis() + second.duration.as_millis(),
        components,
        snapshots,
        hpwl_estimate_mm: hpwl,
        overlap_violations: overlap_count,
    };

    // Phase 2.5: optional greedy part swap pass.
    if config.allow_part_swap {
        let swap_model = swap::build_swap_model(ir);
        if !swap_model.part_swap_groups.is_empty() {
            let _changelog = swap::greedy_part_swap_pass(&mut phase2_result, ir, &swap_model);
            phase2_result.hpwl_estimate_mm = swap::compute_hpwl(&phase2_result, ir);
        }
    }

    // Phase 3: optional simulated annealing refinement.
    let mut post_sa_result = if let Some(sa_cfg) = &config.sa_config {
        // All components from the analytical solver are considered movable.
        let autoplace_designators: Vec<String> = phase2_result
            .components
            .iter()
            .map(|c| c.designator.clone())
            .collect();
        simulated_annealing::refine_with_sa(&phase2_result, ir, sa_cfg, &autoplace_designators)?
    } else {
        phase2_result
    };

    // Phase 4.5: optional greedy pin swap sweep.
    if config.allow_pin_swap {
        let swap_model = swap::build_swap_model(ir);
        if !swap_model.pin_swap_groups.is_empty() {
            let _changelog = swap::greedy_pin_swap_sweep(&mut post_sa_result, ir, &swap_model);
        }
    }

    Ok(post_sa_result)
}

fn status_str(status: &SystemStatus) -> &'static str {
    match status {
        SystemStatus::Solved => "Solved",
        SystemStatus::PartiallySolved => "PartiallySolved",
        SystemStatus::DiagnosticFailure(_) => "DiagnosticFailure",
    }
}

fn snapshot_from_system(
    phase: &str,
    system: &ConstraintSystem,
    runtimes: &[ComponentRuntime],
    note: Option<String>,
) -> PlacementIterationSnapshot {
    let mut components: Vec<PlacementComponentState> = runtimes
        .iter()
        .map(|r| PlacementComponentState {
            designator: r.entity.designator.clone(),
            x_mm: system.get_param(r.entity.x),
            y_mm: system.get_param(r.entity.y),
            rotation_deg: system.get_param(r.entity.theta).to_degrees(),
        })
        .collect();
    components.sort_by(|a, b| a.designator.cmp(&b.designator));
    PlacementIterationSnapshot {
        phase: phase.to_string(),
        components,
        note,
    }
}

fn greedy_legalize_overlaps(
    system: &mut ConstraintSystem,
    runtimes: &[ComponentRuntime],
    clearance: f64,
) -> usize {
    let mut moved = 0usize;
    for i in 0..runtimes.len() {
        for j in (i + 1)..runtimes.len() {
            let a = &runtimes[i].entity;
            let b = &runtimes[j].entity;
            let ax = system.get_param(a.x);
            let ay = system.get_param(a.y);
            let bx = system.get_param(b.x);
            let by = system.get_param(b.y);
            let dx = bx - ax;
            let dy = by - ay;
            let need_x = a.half_w + b.half_w + clearance;
            let need_y = a.half_h + b.half_h + clearance;
            let overlap_x = dx.abs() < need_x;
            let overlap_y = dy.abs() < need_y;
            if overlap_x && overlap_y {
                let shift = (need_x - dx.abs()).max(need_y - dy.abs()) + 0.05;
                let sign = if dx >= 0.0 { 1.0 } else { -1.0 };
                system.set_param(b.x, bx + sign * shift);
                moved += 1;
            }
        }
    }
    moved
}

fn count_overlaps(
    system: &ConstraintSystem,
    runtimes: &[ComponentRuntime],
    clearance: f64,
) -> usize {
    let mut count = 0usize;
    for i in 0..runtimes.len() {
        for j in (i + 1)..runtimes.len() {
            let a = &runtimes[i].entity;
            let b = &runtimes[j].entity;
            let ax = system.get_param(a.x);
            let ay = system.get_param(a.y);
            let bx = system.get_param(b.x);
            let by = system.get_param(b.y);
            let overlap_x = (bx - ax).abs() < (a.half_w + b.half_w + clearance);
            let overlap_y = (by - ay).abs() < (a.half_h + b.half_h + clearance);
            if overlap_x && overlap_y {
                count += 1;
            }
        }
    }
    count
}

fn estimate_hpwl(
    final_components: &[PlacementComponentState],
    runtimes: &[ComponentRuntime],
    nets: &BTreeMap<String, Vec<HpwlPin>>,
) -> f64 {
    let mut param_to_pos = HashMap::<ParamId, (f64, f64, f64)>::new();
    for state in final_components {
        if let Some(rt) = runtimes
            .iter()
            .find(|r| r.entity.designator == state.designator)
        {
            param_to_pos.insert(
                rt.entity.x,
                (state.x_mm, state.y_mm, state.rotation_deg.to_radians()),
            );
        }
    }

    let mut total = 0.0;
    for pins in nets.values() {
        if pins.len() < 2 {
            continue;
        }
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for pin in pins {
            if let Some((cx, cy, theta)) = param_to_pos.get(&pin.comp_x).copied() {
                let (sin_t, cos_t) = theta.sin_cos();
                let wx = cx + pin.local_x * cos_t - pin.local_y * sin_t;
                let wy = cy + pin.local_x * sin_t + pin.local_y * cos_t;
                min_x = min_x.min(wx);
                max_x = max_x.max(wx);
                min_y = min_y.min(wy);
                max_y = max_y.max(wy);
            }
        }
        if min_x.is_finite() {
            total += (max_x - min_x) + (max_y - min_y);
        }
    }
    total
}

pub fn named_region_from_board(ir: &PcbIr, name: &str) -> Option<RectRegion> {
    named_region(
        name,
        ir.board.bounds.min.x,
        ir.board.bounds.min.y,
        ir.board.bounds.max.x,
        ir.board.bounds.max.y,
    )
}

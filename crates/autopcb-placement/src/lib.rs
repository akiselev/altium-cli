pub mod clustering;
pub mod simulated_annealing;
pub mod swap;

use std::collections::{BTreeMap, HashMap, HashSet};

use autopcb_ir::{
    ComponentId, FreeCopperGeometry, IdMap, IrNet, IrNetPin, NetId, PadId, PcbIr, PointMm,
};
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
    pub auto_cluster: bool,
    pub cluster_target_size: usize,
    pub cluster_max_depth: usize,
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
            auto_cluster: false,
            cluster_target_size: 12,
            cluster_max_depth: 3,
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

fn world_half_extents_at(half_w: f64, half_h: f64, theta: f64) -> (f64, f64) {
    let (sin_t, cos_t) = theta.sin_cos();
    (
        half_w * cos_t.abs() + half_h * sin_t.abs(),
        half_w * sin_t.abs() + half_h * cos_t.abs(),
    )
}

fn world_half_extents_derivative_at(half_w: f64, half_h: f64, theta: f64) -> (f64, f64) {
    let (sin_t, cos_t) = theta.sin_cos();
    let d_abs_cos = -sin_t * cos_t.signum();
    let d_abs_sin = cos_t * sin_t.signum();
    (
        half_w * d_abs_cos + half_h * d_abs_sin,
        half_w * d_abs_sin + half_h * d_abs_cos,
    )
}

#[derive(Debug, Clone)]
struct BoardContainment {
    id: ConstraintId,
    entity: EntityId,
    x: ParamId,
    y: ParamId,
    theta: ParamId,
    s: [ParamId; 4],
    half_w: f64,
    half_h: f64,
    clearance: f64,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    params: [ParamId; 7],
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
        let (world_hw, world_hh) =
            world_half_extents_at(self.half_w, self.half_h, store.get(self.theta));
        let world_hw = world_hw + self.clearance;
        let world_hh = world_hh + self.clearance;
        let g0 = (x - world_hw) - self.min_x;
        let g1 = self.max_x - (x + world_hw);
        let g2 = (y - world_hh) - self.min_y;
        let g3 = self.max_y - (y + world_hh);
        vec![
            g0 - store.get(self.s[0]).powi(2),
            g1 - store.get(self.s[1]).powi(2),
            g2 - store.get(self.s[2]).powi(2),
            g3 - store.get(self.s[3]).powi(2),
        ]
    }

    fn jacobian(&self, store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        let (d_hw_d_theta, d_hh_d_theta) =
            world_half_extents_derivative_at(self.half_w, self.half_h, store.get(self.theta));
        vec![
            (0, self.x, 1.0),
            (0, self.theta, -d_hw_d_theta),
            (0, self.s[0], -2.0 * store.get(self.s[0])),
            (1, self.x, -1.0),
            (1, self.theta, -d_hw_d_theta),
            (1, self.s[1], -2.0 * store.get(self.s[1])),
            (2, self.y, 1.0),
            (2, self.theta, -d_hh_d_theta),
            (2, self.s[2], -2.0 * store.get(self.s[2])),
            (3, self.y, -1.0),
            (3, self.theta, -d_hh_d_theta),
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
    theta1: ParamId,
    x2: ParamId,
    y2: ParamId,
    theta2: ParamId,
    s: ParamId,
    half_w1: f64,
    half_h1: f64,
    half_w2: f64,
    half_h2: f64,
    clearance: f64,
    params: [ParamId; 7],
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
        let (hw1, hh1) = world_half_extents_at(self.half_w1, self.half_h1, store.get(self.theta1));
        let (hw2, hh2) = world_half_extents_at(self.half_w2, self.half_h2, store.get(self.theta2));
        let combined_hw = hw1 + hw2 + self.clearance;
        let combined_hh = hh1 + hh2 + self.clearance;
        let g = (dx / combined_hw).powi(2) + (dy / combined_hh).powi(2) - 1.0;
        vec![g - store.get(self.s).powi(2)]
    }

    fn jacobian(&self, store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        let dx = store.get(self.x2) - store.get(self.x1);
        let dy = store.get(self.y2) - store.get(self.y1);
        let (hw1, hh1) = world_half_extents_at(self.half_w1, self.half_h1, store.get(self.theta1));
        let (hw2, hh2) = world_half_extents_at(self.half_w2, self.half_h2, store.get(self.theta2));
        let (d_hw1_d_theta, d_hh1_d_theta) =
            world_half_extents_derivative_at(self.half_w1, self.half_h1, store.get(self.theta1));
        let (d_hw2_d_theta, d_hh2_d_theta) =
            world_half_extents_derivative_at(self.half_w2, self.half_h2, store.get(self.theta2));
        let combined_hw = hw1 + hw2 + self.clearance;
        let combined_hh = hh1 + hh2 + self.clearance;
        let combined_hw_sq = combined_hw * combined_hw;
        let combined_hh_sq = combined_hh * combined_hh;
        let d_g_d_theta1 = -2.0 * dx * dx * d_hw1_d_theta / combined_hw.powi(3)
            - 2.0 * dy * dy * d_hh1_d_theta / combined_hh.powi(3);
        let d_g_d_theta2 = -2.0 * dx * dx * d_hw2_d_theta / combined_hw.powi(3)
            - 2.0 * dy * dy * d_hh2_d_theta / combined_hh.powi(3);
        vec![
            (0, self.x1, -2.0 * dx / combined_hw_sq),
            (0, self.y1, -2.0 * dy / combined_hh_sq),
            (0, self.theta1, d_g_d_theta1),
            (0, self.x2, 2.0 * dx / combined_hw_sq),
            (0, self.y2, 2.0 * dy / combined_hh_sq),
            (0, self.theta2, d_g_d_theta2),
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
    theta: ParamId,
    edge: PlacementEdge,
    inset: f64,
    half_w: f64,
    half_h: f64,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    params: [ParamId; 3],
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
        let (world_hw, world_hh) =
            world_half_extents_at(self.half_w, self.half_h, store.get(self.theta));
        let r = match self.edge {
            PlacementEdge::Top => y + world_hh - (self.max_y - self.inset),
            PlacementEdge::Bottom => y - world_hh - (self.min_y + self.inset),
            PlacementEdge::Left => x - world_hw - (self.min_x + self.inset),
            PlacementEdge::Right => x + world_hw - (self.max_x - self.inset),
        };
        vec![r]
    }

    fn jacobian(&self, store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        let (d_hw_d_theta, d_hh_d_theta) =
            world_half_extents_derivative_at(self.half_w, self.half_h, store.get(self.theta));
        match self.edge {
            PlacementEdge::Top => vec![(0, self.y, 1.0), (0, self.theta, d_hh_d_theta)],
            PlacementEdge::Bottom => vec![(0, self.y, 1.0), (0, self.theta, -d_hh_d_theta)],
            PlacementEdge::Left => vec![(0, self.x, 1.0), (0, self.theta, -d_hw_d_theta)],
            PlacementEdge::Right => vec![(0, self.x, 1.0), (0, self.theta, d_hw_d_theta)],
        }
    }
}

#[derive(Debug, Clone)]
struct DirectionalOrderingConstraint {
    id: ConstraintId,
    entities: [EntityId; 2],
    a_x: ParamId,
    a_y: ParamId,
    a_theta: ParamId,
    b_x: ParamId,
    b_y: ParamId,
    b_theta: ParamId,
    slack: ParamId,
    direction: Direction,
    gap: f64,
    a_half_w: f64,
    a_half_h: f64,
    b_half_w: f64,
    b_half_h: f64,
    params: [ParamId; 7],
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
        let (a_hw, a_hh) =
            world_half_extents_at(self.a_half_w, self.a_half_h, store.get(self.a_theta));
        let (b_hw, b_hh) =
            world_half_extents_at(self.b_half_w, self.b_half_h, store.get(self.b_theta));
        let g = match self.direction {
            Direction::LeftOf => (bx - b_hw) - (ax + a_hw) - self.gap,
            Direction::RightOf => (ax - a_hw) - (bx + b_hw) - self.gap,
            Direction::Above => (ay - a_hh) - (by + b_hh) - self.gap,
            Direction::Below => (by - b_hh) - (ay + a_hh) - self.gap,
        };
        vec![g - store.get(self.slack).powi(2)]
    }

    fn jacobian(&self, store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        let mut j = vec![(0, self.slack, -2.0 * store.get(self.slack))];
        let (d_a_hw_d_theta, d_a_hh_d_theta) =
            world_half_extents_derivative_at(self.a_half_w, self.a_half_h, store.get(self.a_theta));
        let (d_b_hw_d_theta, d_b_hh_d_theta) =
            world_half_extents_derivative_at(self.b_half_w, self.b_half_h, store.get(self.b_theta));
        match self.direction {
            Direction::LeftOf => {
                j.push((0, self.a_x, -1.0));
                j.push((0, self.b_x, 1.0));
                j.push((0, self.a_theta, -d_a_hw_d_theta));
                j.push((0, self.b_theta, -d_b_hw_d_theta));
            }
            Direction::RightOf => {
                j.push((0, self.a_x, 1.0));
                j.push((0, self.b_x, -1.0));
                j.push((0, self.a_theta, -d_a_hw_d_theta));
                j.push((0, self.b_theta, -d_b_hw_d_theta));
            }
            Direction::Above => {
                j.push((0, self.a_y, 1.0));
                j.push((0, self.b_y, -1.0));
                j.push((0, self.a_theta, -d_a_hh_d_theta));
                j.push((0, self.b_theta, -d_b_hh_d_theta));
            }
            Direction::Below => {
                j.push((0, self.a_y, -1.0));
                j.push((0, self.b_y, 1.0));
                j.push((0, self.a_theta, -d_a_hh_d_theta));
                j.push((0, self.b_theta, -d_b_hh_d_theta));
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

#[derive(Debug, Clone, Copy)]
struct PlacementSeed {
    x_mm: f64,
    y_mm: f64,
    rotation_deg: f64,
}

pub fn solve_placement(
    ir: &PcbIr,
    user_constraints: &[UserConstraint],
    config: &PlacementConfig,
    placement_groups: &[Vec<String>],
) -> Result<PlacementResult, PlacementError> {
    if config.auto_cluster && ir.components.len() > config.cluster_target_size.max(2) {
        if let Some(plan) =
            clustering::build_cluster_plan(ir, user_constraints, placement_groups, config)?
        {
            let anchored = explicit_anchor_designators(user_constraints);
            let mut seeds = original_seed_positions(ir);
            let mut inherited_constraints = user_constraints.to_vec();

            for leaf in &plan.leaves {
                let leaf_set: HashSet<String> = leaf.members.iter().cloned().collect();
                let leaf_constraints =
                    filter_constraints_for_designators(user_constraints, &leaf_set);
                let has_anchor = leaf
                    .members
                    .iter()
                    .any(|designator| anchored.contains(designator));

                if !has_anchor && leaf.members.len() > 1 {
                    let leaf_ir = build_subset_ir(ir, &leaf_set, leaf.region.clone());
                    let mut leaf_config = config.clone();
                    leaf_config.auto_cluster = false;
                    if let Ok(leaf_result) =
                        solve_flat_placement(&leaf_ir, &leaf_constraints, &leaf_config, None)
                    {
                        for component in leaf_result.components {
                            seeds.insert(
                                component.designator,
                                PlacementSeed {
                                    x_mm: component.x_mm,
                                    y_mm: component.y_mm,
                                    rotation_deg: component.rotation_deg,
                                },
                            );
                        }
                    }
                }

                for designator in &leaf.members {
                    if anchored.contains(designator) {
                        continue;
                    }
                    inherited_constraints.push(UserConstraint::RegionContainment {
                        designator: designator.clone(),
                        region: leaf.region.clone(),
                    });
                }
            }

            let mut flat_config = config.clone();
            flat_config.auto_cluster = false;
            return solve_flat_placement(ir, &inherited_constraints, &flat_config, Some(&seeds));
        }
    }

    solve_flat_placement(ir, user_constraints, config, None)
}

fn solve_flat_placement(
    ir: &PcbIr,
    user_constraints: &[UserConstraint],
    config: &PlacementConfig,
    initial_seeds: Option<&HashMap<String, PlacementSeed>>,
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
        let (init_x, init_y, init_theta) = if let Some(seed) = initial_seeds
            .and_then(|seeds| seeds.get(&comp.designator))
            .copied()
        {
            (seed.x_mm, seed.y_mm, seed.rotation_deg.to_radians())
        } else if let Some(ref positions) = scatter_positions {
            let (x, y) = positions[comp_idx];
            (x, y, comp.rotation.to_radians())
        } else {
            (comp.position.x, comp.position.y, comp.rotation.to_radians())
        };
        let x = system.alloc_param(init_x, eid);
        let y = system.alloc_param(init_y, eid);
        let theta = system.alloc_param(init_theta, eid);
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
            theta: comp.entity.theta,
            s: [s0, s1, s2, s3],
            half_w: comp.entity.half_w,
            half_h: comp.entity.half_h,
            clearance: config.board_edge_clearance_mm,
            min_x: bounds.min.x,
            min_y: bounds.min.y,
            max_x: bounds.max.x,
            max_y: bounds.max.y,
            params: [
                comp.entity.x,
                comp.entity.y,
                comp.entity.theta,
                s0,
                s1,
                s2,
                s3,
            ],
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
                theta1: a.theta,
                x2: b.x,
                y2: b.y,
                theta2: b.theta,
                s: slack,
                half_w1: a.half_w,
                half_h1: a.half_h,
                half_w2: b.half_w,
                half_h2: b.half_h,
                clearance: config.default_clearance_mm,
                params: [a.x, a.y, a.theta, b.x, b.y, b.theta, slack],
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
                    theta: comp.theta,
                    edge: edge.clone(),
                    inset: *inset_mm,
                    half_w: comp.half_w,
                    half_h: comp.half_h,
                    min_x: bounds.min.x,
                    min_y: bounds.min.y,
                    max_x: bounds.max.x,
                    max_y: bounds.max.y,
                    params: [comp.x, comp.y, comp.theta],
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
                    a_theta: ca.theta,
                    b_x: cb.x,
                    b_y: cb.y,
                    b_theta: cb.theta,
                    slack,
                    direction: direction.clone(),
                    gap: *gap_mm,
                    a_half_w: ca.half_w,
                    a_half_h: ca.half_h,
                    b_half_w: cb.half_w,
                    b_half_h: cb.half_h,
                    params: [ca.x, ca.y, ca.theta, cb.x, cb.y, cb.theta, slack],
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

    let overlaps = structured_legalize(
        &mut system,
        &runtimes,
        bounds.min.x,
        bounds.min.y,
        bounds.max.x,
        bounds.max.y,
        config.default_clearance_mm,
        config.board_edge_clearance_mm,
        config.grid_snap_mm,
        user_constraints,
        &net_to_pins,
    );
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

    let fixed_designators = fixed_position_designators(user_constraints);

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
        let autoplace_designators: Vec<String> = phase2_result
            .components
            .iter()
            .filter(|component| !fixed_designators.contains(&component.designator))
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

fn original_seed_positions(ir: &PcbIr) -> HashMap<String, PlacementSeed> {
    ir.components
        .iter()
        .map(|(_, comp)| {
            (
                comp.designator.clone(),
                PlacementSeed {
                    x_mm: comp.position.x,
                    y_mm: comp.position.y,
                    rotation_deg: comp.rotation,
                },
            )
        })
        .collect()
}

fn fixed_position_designators(user_constraints: &[UserConstraint]) -> HashSet<String> {
    user_constraints
        .iter()
        .filter_map(|constraint| match constraint {
            UserConstraint::FixedPosition { designator, .. } => Some(designator.clone()),
            _ => None,
        })
        .collect()
}

fn explicit_anchor_designators(user_constraints: &[UserConstraint]) -> HashSet<String> {
    user_constraints
        .iter()
        .filter_map(|constraint| match constraint {
            UserConstraint::EdgePlacement { designator, .. }
            | UserConstraint::RegionContainment { designator, .. }
            | UserConstraint::FixedPosition { designator, .. } => Some(designator.clone()),
            UserConstraint::Directional { .. } | UserConstraint::Near { .. } => None,
        })
        .collect()
}

fn filter_constraints_for_designators(
    user_constraints: &[UserConstraint],
    designators: &HashSet<String>,
) -> Vec<UserConstraint> {
    user_constraints
        .iter()
        .filter(|constraint| match constraint {
            UserConstraint::EdgePlacement { designator, .. }
            | UserConstraint::RegionContainment { designator, .. }
            | UserConstraint::FixedPosition { designator, .. } => designators.contains(designator),
            UserConstraint::Directional { a, b, .. } | UserConstraint::Near { a, b, .. } => {
                designators.contains(a) && designators.contains(b)
            }
        })
        .cloned()
        .collect()
}

fn build_subset_ir(ir: &PcbIr, designators: &HashSet<String>, region: RectRegion) -> PcbIr {
    let mut board = ir.board.clone();
    board.bounds.min.x = region.min_x;
    board.bounds.min.y = region.min_y;
    board.bounds.max.x = region.max_x;
    board.bounds.max.y = region.max_y;

    let mut components = IdMap::<ComponentId, _>::with_capacity(designators.len());
    let mut comp_map = HashMap::<u32, ComponentId>::new();
    let mut pad_map = HashMap::<u32, PadId>::new();
    let mut next_pad_id = 0u32;
    let region_center = PointMm::new(
        (region.min_x + region.max_x) * 0.5,
        (region.min_y + region.max_y) * 0.5,
    );

    for (old_comp_id, comp) in ir.components.iter() {
        if !designators.contains(&comp.designator) {
            continue;
        }
        let mut cloned = comp.clone();
        cloned.position = PointMm::new(
            comp.position
                .x
                .clamp(region.min_x, region.max_x)
                .max(region.min_x),
            comp.position
                .y
                .clamp(region.min_y, region.max_y)
                .max(region.min_y),
        );
        if !region_contains_center(&region, cloned.position.x, cloned.position.y) {
            cloned.position = region_center;
        }
        for pad in &mut cloned.pads {
            let new_pad = PadId::from(next_pad_id);
            next_pad_id += 1;
            pad_map.insert(pad.id.raw(), new_pad);
            pad.id = new_pad;
        }
        let new_comp_id = components.push(cloned);
        components[new_comp_id].id = new_comp_id;
        comp_map.insert(old_comp_id.raw(), new_comp_id);
    }

    let mut nets = IdMap::<NetId, IrNet>::new();
    let mut net_map = HashMap::<u32, NetId>::new();
    for (old_net_id, net) in ir.nets.iter() {
        let mut new_pins = Vec::<IrNetPin>::new();
        let mut seen_components = HashSet::new();
        for pin in &net.pins {
            let Some(&new_comp) = comp_map.get(&pin.component.raw()) else {
                continue;
            };
            let Some(&new_pad) = pad_map.get(&pin.pad.raw()) else {
                continue;
            };
            new_pins.push(IrNetPin {
                pad: new_pad,
                component: new_comp,
                position: pin.position,
            });
            seen_components.insert(new_comp.raw());
        }
        if new_pins.is_empty() {
            continue;
        }
        let new_id = nets.push(IrNet {
            id: NetId::from(0),
            name: net.name.clone(),
            pins: new_pins,
            component_count: seen_components.len(),
        });
        nets[new_id].id = new_id;
        net_map.insert(old_net_id.raw(), new_id);
    }

    for (_, comp) in components.iter_mut() {
        for pad in &mut comp.pads {
            pad.net = pad
                .net
                .and_then(|old_net| net_map.get(&old_net.raw()).copied());
        }
    }

    PcbIr {
        board,
        layer_stack: ir.layer_stack.clone(),
        components,
        nets,
        rules: IdMap::new(),
        free_copper: FreeCopperGeometry::default(),
        polygons: IdMap::new(),
    }
}

#[derive(Debug, Clone, Copy)]
struct LegalizerPose {
    x: f64,
    y: f64,
    theta: f64,
}

#[derive(Debug, Clone, Copy)]
struct LegalizerEvaluation {
    hard_violations: usize,
    overlap_area: f64,
    overflow: f64,
    hpwl: f64,
    congestion: f64,
}

#[derive(Debug, Clone, Copy)]
struct LegalizerScore {
    hard_violations: usize,
    overlap_area: f64,
    overflow: f64,
    displacement: f64,
    hpwl_delta: f64,
    congestion: f64,
}

fn structured_legalize(
    system: &mut ConstraintSystem,
    runtimes: &[ComponentRuntime],
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    clearance: f64,
    board_clearance: f64,
    grid_snap: Option<f64>,
    user_constraints: &[UserConstraint],
    net_to_pins: &BTreeMap<String, Vec<HpwlPin>>,
) -> usize {
    let mut moved = 0usize;
    let fixed = fixed_position_designators(user_constraints);
    let mut current_eval = evaluate_legalizer_state(
        system,
        runtimes,
        min_x,
        min_y,
        max_x,
        max_y,
        clearance,
        board_clearance,
        net_to_pins,
        grid_snap.unwrap_or(5.0).max(1.0),
    );
    let pass_budget = runtimes.len().max(1) * 16;

    for _ in 0..pass_budget {
        let violating = violating_components(
            system,
            runtimes,
            &fixed,
            min_x,
            min_y,
            max_x,
            max_y,
            clearance,
            board_clearance,
        );
        if violating.is_empty() {
            break;
        }

        let mut improved = false;
        for comp_idx in violating {
            let best = best_legalizer_candidate(
                system,
                runtimes,
                comp_idx,
                min_x,
                min_y,
                max_x,
                max_y,
                clearance,
                board_clearance,
                grid_snap,
                net_to_pins,
                current_eval,
            );
            if let Some((pose, score, eval)) = best {
                let current_score = LegalizerScore {
                    hard_violations: current_eval.hard_violations,
                    overlap_area: current_eval.overlap_area,
                    overflow: current_eval.overflow,
                    displacement: 0.0,
                    hpwl_delta: 0.0,
                    congestion: current_eval.congestion,
                };
                if legalizer_score_better(score, current_score) {
                    let comp = &runtimes[comp_idx].entity;
                    system.set_param(comp.x, pose.x);
                    system.set_param(comp.y, pose.y);
                    system.set_param(comp.theta, pose.theta);
                    current_eval = eval;
                    moved += 1;
                    improved = true;
                    break;
                }
            }
        }
        if !improved {
            break;
        }
    }
    moved
}

#[allow(clippy::too_many_arguments)]
fn violating_components(
    system: &ConstraintSystem,
    runtimes: &[ComponentRuntime],
    fixed: &HashSet<String>,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    clearance: f64,
    board_clearance: f64,
) -> Vec<usize> {
    let mut violating = Vec::new();
    for (idx, runtime) in runtimes.iter().enumerate() {
        if fixed.contains(&runtime.entity.designator) {
            continue;
        }
        let overflow = component_board_overflow(
            system,
            &runtime.entity,
            min_x,
            min_y,
            max_x,
            max_y,
            board_clearance,
        );
        let overlap = runtimes
            .iter()
            .enumerate()
            .filter(|(other_idx, _)| *other_idx != idx)
            .map(|(_, other)| {
                component_overlap_area(system, &runtime.entity, &other.entity, clearance)
            })
            .sum::<f64>();
        if overflow > 1e-9 || overlap > 1e-9 {
            violating.push(idx);
        }
    }
    violating
}

#[allow(clippy::too_many_arguments)]
fn best_legalizer_candidate(
    system: &mut ConstraintSystem,
    runtimes: &[ComponentRuntime],
    comp_idx: usize,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    clearance: f64,
    board_clearance: f64,
    grid_snap: Option<f64>,
    net_to_pins: &BTreeMap<String, Vec<HpwlPin>>,
    current_eval: LegalizerEvaluation,
) -> Option<(LegalizerPose, LegalizerScore, LegalizerEvaluation)> {
    let comp = &runtimes[comp_idx].entity;
    let old_pose = LegalizerPose {
        x: system.get_param(comp.x),
        y: system.get_param(comp.y),
        theta: system.get_param(comp.theta),
    };
    let current_rotations = [
        old_pose.theta,
        0.0,
        90.0_f64.to_radians(),
        180.0_f64.to_radians(),
        270.0_f64.to_radians(),
    ];
    let mut candidates = Vec::<LegalizerPose>::new();

    for theta in current_rotations {
        push_candidate(
            &mut candidates,
            clamp_pose_to_board(
                old_pose.x,
                old_pose.y,
                theta,
                comp,
                min_x,
                min_y,
                max_x,
                max_y,
                board_clearance,
            ),
        );
        let (hw, hh) = world_half_extents_at(comp.half_w, comp.half_h, theta);
        let left_x = min_x + board_clearance + hw;
        let right_x = max_x - board_clearance - hw;
        let bottom_y = min_y + board_clearance + hh;
        let top_y = max_y - board_clearance - hh;
        push_candidate(
            &mut candidates,
            clamp_pose_to_board(
                left_x,
                old_pose.y,
                theta,
                comp,
                min_x,
                min_y,
                max_x,
                max_y,
                board_clearance,
            ),
        );
        push_candidate(
            &mut candidates,
            clamp_pose_to_board(
                right_x,
                old_pose.y,
                theta,
                comp,
                min_x,
                min_y,
                max_x,
                max_y,
                board_clearance,
            ),
        );
        push_candidate(
            &mut candidates,
            clamp_pose_to_board(
                old_pose.x,
                bottom_y,
                theta,
                comp,
                min_x,
                min_y,
                max_x,
                max_y,
                board_clearance,
            ),
        );
        push_candidate(
            &mut candidates,
            clamp_pose_to_board(
                old_pose.x,
                top_y,
                theta,
                comp,
                min_x,
                min_y,
                max_x,
                max_y,
                board_clearance,
            ),
        );

        for (other_idx, other_runtime) in runtimes.iter().enumerate() {
            if other_idx == comp_idx {
                continue;
            }
            let other = &other_runtime.entity;
            let ox = system.get_param(other.x);
            let oy = system.get_param(other.y);
            let other_theta = system.get_param(other.theta);
            let (other_hw, other_hh) =
                world_half_extents_at(other.half_w, other.half_h, other_theta);
            let left = ox - (other_hw + hw + clearance);
            let right = ox + (other_hw + hw + clearance);
            let below = oy - (other_hh + hh + clearance);
            let above = oy + (other_hh + hh + clearance);
            push_candidate(
                &mut candidates,
                clamp_pose_to_board(
                    left,
                    oy,
                    theta,
                    comp,
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    board_clearance,
                ),
            );
            push_candidate(
                &mut candidates,
                clamp_pose_to_board(
                    right,
                    oy,
                    theta,
                    comp,
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    board_clearance,
                ),
            );
            push_candidate(
                &mut candidates,
                clamp_pose_to_board(
                    ox,
                    below,
                    theta,
                    comp,
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    board_clearance,
                ),
            );
            push_candidate(
                &mut candidates,
                clamp_pose_to_board(
                    ox,
                    above,
                    theta,
                    comp,
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    board_clearance,
                ),
            );
        }
    }

    if let Some(grid) = grid_snap {
        let existing = candidates.clone();
        for candidate in existing {
            push_candidate(
                &mut candidates,
                clamp_pose_to_board(
                    (candidate.x / grid).round() * grid,
                    (candidate.y / grid).round() * grid,
                    candidate.theta,
                    comp,
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    board_clearance,
                ),
            );
        }
    }

    let mut best = None;
    for candidate in candidates {
        system.set_param(comp.x, candidate.x);
        system.set_param(comp.y, candidate.y);
        system.set_param(comp.theta, candidate.theta);
        let eval = evaluate_legalizer_state(
            system,
            runtimes,
            min_x,
            min_y,
            max_x,
            max_y,
            clearance,
            board_clearance,
            net_to_pins,
            grid_snap.unwrap_or(5.0).max(1.0),
        );
        let score = LegalizerScore {
            hard_violations: eval.hard_violations,
            overlap_area: eval.overlap_area,
            overflow: eval.overflow,
            displacement: (candidate.x - old_pose.x).abs()
                + (candidate.y - old_pose.y).abs()
                + (candidate.theta - old_pose.theta).abs().to_degrees() * 0.01,
            hpwl_delta: eval.hpwl - current_eval.hpwl,
            congestion: eval.congestion,
        };
        match best {
            Some((_, best_score, _)) if !legalizer_score_better(score, best_score) => {}
            _ => best = Some((candidate, score, eval)),
        }
    }

    system.set_param(comp.x, old_pose.x);
    system.set_param(comp.y, old_pose.y);
    system.set_param(comp.theta, old_pose.theta);
    best
}

fn push_candidate(candidates: &mut Vec<LegalizerPose>, candidate: LegalizerPose) {
    let duplicate = candidates.iter().any(|existing| {
        (existing.x - candidate.x).abs() < 1e-6
            && (existing.y - candidate.y).abs() < 1e-6
            && (existing.theta - candidate.theta).abs() < 1e-9
    });
    if !duplicate {
        candidates.push(candidate);
    }
}

#[allow(clippy::too_many_arguments)]
fn clamp_pose_to_board(
    x: f64,
    y: f64,
    theta: f64,
    comp: &PcbComponentEntity,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    board_clearance: f64,
) -> LegalizerPose {
    let (hw, hh) = world_half_extents_at(comp.half_w, comp.half_h, theta);
    let min_cx = min_x + board_clearance + hw;
    let max_cx = max_x - board_clearance - hw;
    let min_cy = min_y + board_clearance + hh;
    let max_cy = max_y - board_clearance - hh;
    let clamped_x = if min_cx <= max_cx {
        x.clamp(min_cx, max_cx)
    } else {
        (min_x + max_x) * 0.5
    };
    let clamped_y = if min_cy <= max_cy {
        y.clamp(min_cy, max_cy)
    } else {
        (min_y + max_y) * 0.5
    };
    LegalizerPose {
        x: clamped_x,
        y: clamped_y,
        theta,
    }
}

fn legalizer_score_better(candidate: LegalizerScore, current: LegalizerScore) -> bool {
    if candidate.hard_violations != current.hard_violations {
        return candidate.hard_violations < current.hard_violations;
    }
    if (candidate.overlap_area - current.overlap_area).abs() > 1e-6 {
        return candidate.overlap_area < current.overlap_area;
    }
    if (candidate.overflow - current.overflow).abs() > 1e-6 {
        return candidate.overflow < current.overflow;
    }
    if (candidate.displacement - current.displacement).abs() > 1e-6 {
        return candidate.displacement < current.displacement;
    }
    if (candidate.hpwl_delta - current.hpwl_delta).abs() > 1e-6 {
        return candidate.hpwl_delta < current.hpwl_delta;
    }
    candidate.congestion + 1e-6 < current.congestion
}

#[allow(clippy::too_many_arguments)]
fn evaluate_legalizer_state(
    system: &ConstraintSystem,
    runtimes: &[ComponentRuntime],
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    clearance: f64,
    board_clearance: f64,
    net_to_pins: &BTreeMap<String, Vec<HpwlPin>>,
    congestion_cell: f64,
) -> LegalizerEvaluation {
    let components = system_component_states(system, runtimes);
    let mut hard_violations = 0usize;
    let mut overlap_area = 0.0;
    let mut overflow = 0.0;

    for i in 0..runtimes.len() {
        let component_overflow = component_board_overflow(
            system,
            &runtimes[i].entity,
            min_x,
            min_y,
            max_x,
            max_y,
            board_clearance,
        );
        overflow += component_overflow;
        if component_overflow > 1e-9 {
            hard_violations += 1;
        }
        for j in (i + 1)..runtimes.len() {
            let area =
                component_overlap_area(system, &runtimes[i].entity, &runtimes[j].entity, clearance);
            overlap_area += area;
            if area > 1e-9 {
                hard_violations += 1;
            }
        }
    }

    LegalizerEvaluation {
        hard_violations,
        overlap_area,
        overflow,
        hpwl: estimate_hpwl(&components, runtimes, net_to_pins),
        congestion: congestion_penalty_for_components(
            &components,
            runtimes,
            net_to_pins,
            min_x,
            min_y,
            max_x,
            max_y,
            congestion_cell,
            2.0 * congestion_cell,
        ),
    }
}

fn system_component_states(
    system: &ConstraintSystem,
    runtimes: &[ComponentRuntime],
) -> Vec<PlacementComponentState> {
    let mut components: Vec<PlacementComponentState> = runtimes
        .iter()
        .map(|runtime| PlacementComponentState {
            designator: runtime.entity.designator.clone(),
            x_mm: system.get_param(runtime.entity.x),
            y_mm: system.get_param(runtime.entity.y),
            rotation_deg: system.get_param(runtime.entity.theta).to_degrees(),
        })
        .collect();
    components.sort_by(|a, b| a.designator.cmp(&b.designator));
    components
}

fn component_overlap_area(
    system: &ConstraintSystem,
    a: &PcbComponentEntity,
    b: &PcbComponentEntity,
    clearance: f64,
) -> f64 {
    let ax = system.get_param(a.x);
    let ay = system.get_param(a.y);
    let bx = system.get_param(b.x);
    let by = system.get_param(b.y);
    let (a_hw, a_hh) = world_half_extents_at(a.half_w, a.half_h, system.get_param(a.theta));
    let (b_hw, b_hh) = world_half_extents_at(b.half_w, b.half_h, system.get_param(b.theta));
    let dx = (ax - bx).abs();
    let dy = (ay - by).abs();
    let ox = (a_hw + b_hw + clearance) - dx;
    let oy = (a_hh + b_hh + clearance) - dy;
    if ox > 0.0 && oy > 0.0 { ox * oy } else { 0.0 }
}

fn component_board_overflow(
    system: &ConstraintSystem,
    comp: &PcbComponentEntity,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    board_clearance: f64,
) -> f64 {
    let x = system.get_param(comp.x);
    let y = system.get_param(comp.y);
    let (hw, hh) = world_half_extents_at(comp.half_w, comp.half_h, system.get_param(comp.theta));
    let lo_x = (min_x + board_clearance + hw - x).max(0.0);
    let hi_x = (x + hw + board_clearance - max_x).max(0.0);
    let lo_y = (min_y + board_clearance + hh - y).max(0.0);
    let hi_y = (y + hh + board_clearance - max_y).max(0.0);
    lo_x + hi_x + lo_y + hi_y
}

fn count_overlaps(
    system: &ConstraintSystem,
    runtimes: &[ComponentRuntime],
    clearance: f64,
) -> usize {
    let mut count = 0usize;
    for i in 0..runtimes.len() {
        for j in (i + 1)..runtimes.len() {
            if component_overlap_area(system, &runtimes[i].entity, &runtimes[j].entity, clearance)
                > 1e-9
            {
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

#[allow(clippy::too_many_arguments)]
fn congestion_penalty_for_components(
    final_components: &[PlacementComponentState],
    runtimes: &[ComponentRuntime],
    nets: &BTreeMap<String, Vec<HpwlPin>>,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    cell_size: f64,
    capacity: f64,
) -> f64 {
    let width = (max_x - min_x).max(cell_size);
    let height = (max_y - min_y).max(cell_size);
    let cols = (width / cell_size).ceil().max(1.0) as usize;
    let rows = (height / cell_size).ceil().max(1.0) as usize;
    let mut cells = vec![0.0; rows * cols];
    let mut param_to_pos = HashMap::<ParamId, (f64, f64, f64)>::new();
    for state in final_components {
        if let Some(rt) = runtimes
            .iter()
            .find(|runtime| runtime.entity.designator == state.designator)
        {
            param_to_pos.insert(
                rt.entity.x,
                (state.x_mm, state.y_mm, state.rotation_deg.to_radians()),
            );
        }
    }

    for pins in nets.values() {
        if pins.len() < 2 {
            continue;
        }
        let mut min_px = f64::INFINITY;
        let mut min_py = f64::INFINITY;
        let mut max_px = f64::NEG_INFINITY;
        let mut max_py = f64::NEG_INFINITY;
        for pin in pins {
            if let Some((cx, cy, theta)) = param_to_pos.get(&pin.comp_x).copied() {
                let (sin_t, cos_t) = theta.sin_cos();
                let wx = cx + pin.local_x * cos_t - pin.local_y * sin_t;
                let wy = cy + pin.local_x * sin_t + pin.local_y * cos_t;
                min_px = min_px.min(wx);
                min_py = min_py.min(wy);
                max_px = max_px.max(wx);
                max_py = max_py.max(wy);
            }
        }
        if !min_px.is_finite() {
            continue;
        }
        let span_w = (max_px - min_px).max(cell_size * 0.5);
        let span_h = (max_py - min_py).max(cell_size * 0.5);
        let demand = (span_w + span_h) / (span_w * span_h).max(cell_size * cell_size * 0.25);
        let col0 = (((min_px - min_x) / cell_size).floor() as isize).clamp(0, cols as isize - 1);
        let col1 = (((max_px - min_x) / cell_size).floor() as isize).clamp(0, cols as isize - 1);
        let row0 = (((min_py - min_y) / cell_size).floor() as isize).clamp(0, rows as isize - 1);
        let row1 = (((max_py - min_y) / cell_size).floor() as isize).clamp(0, rows as isize - 1);
        let covered = ((row1 - row0 + 1) * (col1 - col0 + 1)).max(1) as f64;
        for row in row0..=row1 {
            for col in col0..=col1 {
                cells[row as usize * cols + col as usize] += demand / covered;
            }
        }
    }

    cells
        .into_iter()
        .map(|demand| (demand - capacity).max(0.0).powi(2))
        .sum()
}

fn region_contains_center(region: &RectRegion, x: f64, y: f64) -> bool {
    x >= region.min_x && x <= region.max_x && y >= region.min_y && y <= region.max_y
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

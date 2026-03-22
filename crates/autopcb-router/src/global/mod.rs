//! Global routing stage: net decomposition, coarse congestion grid, layer
//! assignment, and net ordering heuristic.
//!
//! Produces per-subnet region guidance consumed by the detailed router.
//!
//! # Pipeline
//!
//! 1. Decompose each net into 2-pin subnets via MST (`steiner::MstDecomposer`).
//! 2. Build a coarse congestion grid from the workspace obstacle map.
//! 3. Assign each subnet to a routing layer (heuristic: preferred direction).
//! 4. Compute the net routing order (critical first, short first, RNG tiebreak).
//!
//! The resulting `GlobalRoutePlan` carries all information the detailed router
//! needs to begin A* pathfinding per subnet.

pub mod congestion;
pub mod layer_assignment;
pub mod ordering;
pub mod steiner;

use autopcb_ir::PcbIr;
use autopcb_routes::NetId;

use crate::workspace::RoutingWorkspace;
use crate::RoutingError;

use congestion::GlobalRoutingGrid;
use layer_assignment::{assign_layers, LayerAssignment};
use ordering::{order_nets, NetOrderingInfo};
use steiner::{MstDecomposer, NetDecomposer, Subnet};

/// Default coarse-cell multiplier: 7× the fine grid resolution.
const DEFAULT_CELL_MULTIPLIER: u32 = 7;

// ---------------------------------------------------------------------------
// GlobalRoutePlan
// ---------------------------------------------------------------------------

/// The output of the global routing stage.
///
/// Consumed by the detailed router (Milestone 6) and the PathFinder negotiation
/// loop (Milestone 7).
#[derive(Debug)]
pub struct GlobalRoutePlan {
    /// All 2-pin subnets produced by net decomposition.
    ///
    /// `Subnet::region_path` carries the coarse-grid cell sequence suggested
    /// for each subnet.  It may be empty if global routing was skipped or
    /// the net has only one pin.
    pub subnets: Vec<Subnet>,

    /// Per-subnet layer assignment (index corresponds to `subnets`).
    pub layer_assignments: Vec<LayerAssignment>,

    /// Net routing order: `NetId`s sorted by the global ordering heuristic.
    pub net_order: Vec<NetId>,
}

// ---------------------------------------------------------------------------
// global_route
// ---------------------------------------------------------------------------

/// Produce a [`GlobalRoutePlan`] from a [`RoutingWorkspace`] and board IR.
///
/// Steps:
/// 1. Decompose every net in `ir.nets` into 2-pin subnets via MST.
/// 2. Build a coarse congestion grid from the workspace obstacle maps.
/// 3. Assign layers to subnets using the heuristic in `layer_assignment`.
/// 4. Compute net routing order using the heuristic in `ordering`.
///
/// Currently the coarse A* region path (`Subnet::region_path`) is left empty;
/// it will be populated in Milestone 7 when the PathFinder negotiation loop
/// performs global routing on the congestion grid.
pub fn global_route(
    workspace: &RoutingWorkspace,
    ir: &PcbIr,
) -> Result<GlobalRoutePlan, RoutingError> {
    let decomposer = MstDecomposer;

    // ------------------------------------------------------------------
    // 1. Decompose all nets into 2-pin subnets
    // ------------------------------------------------------------------
    let mut all_subnets: Vec<Subnet> = Vec::new();
    let mut ordering_input: Vec<(NetId, NetOrderingInfo)> = Vec::new();

    for (_ir_net_id, ir_net) in ir.nets.iter() {
        let net_id = NetId(ir_net.id.raw());

        // Collect pin positions from the net's pin list.
        let pins: Vec<autopcb_ir::types::PointMm> =
            ir_net.pins.iter().map(|p| p.position).collect();

        // MST decomposition: 0- and 1-pin nets produce no subnets.
        let subnets = decomposer.decompose(&pins, net_id);

        // Estimate routing length as sum of MST edge lengths (HPWL proxy).
        let estimated_length_mm: f64 = subnets
            .iter()
            .map(|s| s.source.distance_to(&s.target))
            .sum();

        ordering_input.push((
            net_id,
            NetOrderingInfo {
                pin_count: pins.len(),
                estimated_length_mm,
                priority: 0, // default priority; overridden by per-net rules in M3+
            },
        ));

        all_subnets.extend(subnets);
    }

    // ------------------------------------------------------------------
    // 2. Build coarse congestion grid
    // ------------------------------------------------------------------
    let _grid = GlobalRoutingGrid::from_workspace(workspace, DEFAULT_CELL_MULTIPLIER);

    // ------------------------------------------------------------------
    // 3. Layer assignment
    // ------------------------------------------------------------------
    let layer_assignments = assign_layers(&all_subnets, workspace);

    // Apply layer assignments back to subnets so the detailed router can use
    // the preferred start/goal layers directly from the subnet fields.
    for assignment in &layer_assignments {
        if let Some(subnet) = all_subnets.get_mut(assignment.subnet_index) {
            subnet.source_layer = Some(assignment.layer);
            subnet.target_layer = Some(assignment.layer);
        }
    }

    // ------------------------------------------------------------------
    // 4. Net ordering
    // ------------------------------------------------------------------
    let seed = workspace.policy.seed();
    let net_order = order_nets(&ordering_input, seed);

    let plan = GlobalRoutePlan {
        subnets: all_subnets,
        layer_assignments,
        net_order,
    };

    tracing::info!(
        target: "autopcb_router::global",
        subnet_count = plan.subnets.len(),
        net_order_count = plan.net_order.len(),
        "global_route_finished"
    );

    Ok(plan)
}

// ---------------------------------------------------------------------------
// RoutingPolicy::seed accessor
// ---------------------------------------------------------------------------

// The `RoutingPolicy` type does not currently expose the seed.  We add a
// minimal accessor via an inherent impl here, sourced from the config stored
// during `build_policy`.

use crate::rules::RoutingPolicy;

impl RoutingPolicy {
    /// Return the RNG seed from the config used to build this policy.
    ///
    /// Routing ordering uses this seed for deterministic ChaCha8Rng tiebreaks.
    pub(crate) fn seed(&self) -> u64 {
        self.config_seed
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use autopcb_ir::{
        copper::FreeCopperGeometry,
        handles::{IdMap, LayerId as IrLayerId, NetId as IrNetId},
        layer_stack::{IrCopperLayer, IrLayerStack, PreferredDirection},
        net::{IrNet, IrNetPin},
        types::{BoundingBoxMm, PointMm},
        IrBoardGeometry, PcbIr,
    };
    use autopcb_ir::handles::{ComponentId, PadId};
    use crate::config::RoutingConfig;
    use crate::workspace::build_workspace;

    fn two_layer_ir_with_nets(nets: Vec<IrNet>) -> PcbIr {
        let mut nets_map: IdMap<IrNetId, IrNet> = IdMap::new();
        for net in nets {
            nets_map.push(net);
        }
        PcbIr {
            board: IrBoardGeometry {
                outline: vec![],
                cutouts: vec![],
                bounds: BoundingBoxMm::new(
                    PointMm::new(0.0, 0.0),
                    PointMm::new(100.0, 100.0),
                ),
                keepouts: vec![],
            },
            layer_stack: IrLayerStack {
                copper_layers: vec![
                    IrCopperLayer {
                        id: IrLayerId::from(0),
                        name: "Top".into(),
                        is_top: true,
                        is_bottom: false,
                        preferred_direction: Some(PreferredDirection::Any),
                    },
                    IrCopperLayer {
                        id: IrLayerId::from(1),
                        name: "Bottom".into(),
                        is_top: false,
                        is_bottom: true,
                        preferred_direction: Some(PreferredDirection::Any),
                    },
                ],
                copper_layer_count: 2,
            },
            components: IdMap::new(),
            nets: nets_map,
            rules: IdMap::new(),
            free_copper: FreeCopperGeometry::default(),
            polygons: IdMap::new(),
            texts: IdMap::new(),
            regions: IdMap::new(),
            component_bodies: IdMap::new(),
        }
    }

    fn make_net(id: u32, pins: Vec<PointMm>) -> IrNet {
        IrNet {
            id: IrNetId::from(id),
            name: format!("NET{id}"),
            pins: pins
                .into_iter()
                .enumerate()
                .map(|(i, pos)| IrNetPin {
                    pad: PadId::from(i as u32),
                    component: ComponentId::from(0),
                    position: pos,
                })
                .collect(),
            component_count: 1,
            net_class: None,
            diff_pair_partner: None,
        }
    }

    #[test]
    fn global_route_empty_board_succeeds() {
        let ir = two_layer_ir_with_nets(vec![]);
        let config = RoutingConfig::default();
        let ws = build_workspace(&ir, &config).unwrap();
        let plan = global_route(&ws, &ir).unwrap();
        assert!(plan.subnets.is_empty());
        assert!(plan.layer_assignments.is_empty());
        assert!(plan.net_order.is_empty());
    }

    #[test]
    fn global_route_two_pin_net_produces_one_subnet() {
        let net = make_net(0, vec![PointMm::new(10.0, 10.0), PointMm::new(20.0, 10.0)]);
        let ir = two_layer_ir_with_nets(vec![net]);
        let config = RoutingConfig::default();
        let ws = build_workspace(&ir, &config).unwrap();
        let plan = global_route(&ws, &ir).unwrap();
        assert_eq!(plan.subnets.len(), 1, "2-pin net → 1 subnet");
        assert_eq!(plan.layer_assignments.len(), 1);
        assert_eq!(plan.net_order.len(), 1);
    }

    #[test]
    fn global_route_four_pin_net_produces_three_subnets() {
        let net = make_net(
            0,
            vec![
                PointMm::new(0.0, 0.0),
                PointMm::new(10.0, 0.0),
                PointMm::new(10.0, 10.0),
                PointMm::new(0.0, 10.0),
            ],
        );
        let ir = two_layer_ir_with_nets(vec![net]);
        let config = RoutingConfig::default();
        let ws = build_workspace(&ir, &config).unwrap();
        let plan = global_route(&ws, &ir).unwrap();
        assert_eq!(plan.subnets.len(), 3, "4-pin net → 3 subnets");
    }

    #[test]
    fn global_route_one_pin_net_produces_no_subnets() {
        let net = make_net(0, vec![PointMm::new(5.0, 5.0)]);
        let ir = two_layer_ir_with_nets(vec![net]);
        let config = RoutingConfig::default();
        let ws = build_workspace(&ir, &config).unwrap();
        let plan = global_route(&ws, &ir).unwrap();
        assert_eq!(plan.subnets.len(), 0, "1-pin net → 0 subnets");
    }

    #[test]
    fn global_route_net_order_contains_all_nets() {
        let nets = vec![
            make_net(0, vec![PointMm::new(0.0, 0.0), PointMm::new(5.0, 0.0)]),
            make_net(1, vec![PointMm::new(10.0, 0.0), PointMm::new(20.0, 0.0)]),
        ];
        let ir = two_layer_ir_with_nets(nets);
        let config = RoutingConfig::default();
        let ws = build_workspace(&ir, &config).unwrap();
        let plan = global_route(&ws, &ir).unwrap();
        assert_eq!(plan.net_order.len(), 2, "should have an entry per net");
        let mut sorted = plan.net_order.clone();
        sorted.sort_by_key(|id| id.raw());
        assert_eq!(sorted[0], NetId(0));
        assert_eq!(sorted[1], NetId(1));
    }
}

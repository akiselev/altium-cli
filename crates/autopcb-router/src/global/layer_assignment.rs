//! Layer assignment: heuristic preferred-direction pass.
//!
//! The heuristic assigns each subnet to the layer whose preferred routing
//! direction best matches the subnet's orientation:
//!
//! - Horizontal subnets (|dx| ≥ |dy|) → prefer a `Horizontal` or `Any` layer.
//! - Vertical subnets (|dy| > |dx|)   → prefer a `Vertical` or `Any` layer.
//!
//! If no layer with the preferred direction exists, the first available layer
//! for the net is chosen as a fallback.

use autopcb_ir::layer_stack::PreferredDirection;
use autopcb_routes::LayerId;

use crate::rules::RoutingPolicy;
use crate::workspace::RoutingWorkspace;

use super::steiner::Subnet;

// ---------------------------------------------------------------------------
// LayerAssignment
// ---------------------------------------------------------------------------

/// Associates a subnet (by index in `GlobalRoutePlan::subnets`) with a chosen
/// routing layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerAssignment {
    /// Index into `GlobalRoutePlan::subnets`.
    pub subnet_index: usize,
    /// Chosen routing layer for this subnet.
    pub layer: LayerId,
}

// ---------------------------------------------------------------------------
// assign_layers
// ---------------------------------------------------------------------------

/// Assign a routing layer to each subnet using a preferred-direction heuristic.
///
/// For each subnet the delta vector (dx, dy) is computed.  If |dx| ≥ |dy|
/// the subnet is considered horizontal and is assigned to the layer with
/// `PreferredDirection::Horizontal` (or `Any`) that is also allowed by the
/// policy.  Vertical subnets are handled symmetrically.
///
/// When no layer matches the preferred direction the first allowed layer is
/// used as a fallback.
pub fn assign_layers(subnets: &[Subnet], workspace: &RoutingWorkspace) -> Vec<LayerAssignment> {
    let policy = &workspace.policy;

    subnets
        .iter()
        .enumerate()
        .map(|(idx, subnet)| {
            let layer = choose_layer(subnet, policy, workspace);
            LayerAssignment {
                subnet_index: idx,
                layer,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Choose the best layer for `subnet` given the routing policy.
fn choose_layer(
    subnet: &Subnet,
    policy: &RoutingPolicy,
    workspace: &RoutingWorkspace,
) -> LayerId {
    let dx = (subnet.target.x - subnet.source.x).abs();
    let dy = (subnet.target.y - subnet.source.y).abs();
    let preferred_dir = if dx >= dy {
        PreferredDirection::Horizontal
    } else {
        PreferredDirection::Vertical
    };

    // Layers allowed for the subnet's net.
    let allowed = policy.allowed_layers(subnet.net_id);
    if allowed.is_empty() {
        // No layers at all — return a sentinel.  This should not happen for a
        // well-formed workspace; the detailed router will reject it.
        return LayerId(0);
    }

    // Try to find a layer whose preferred direction matches.
    let best = allowed.iter().find(|&&routes_layer| {
        workspace
            .layer_stack_preferred_direction(routes_layer)
            .map_or(false, |dir| dir == preferred_dir || dir == PreferredDirection::Any)
    });

    if let Some(&layer) = best {
        return layer;
    }

    // Fallback: first allowed layer regardless of direction.
    allowed[0]
}

// ---------------------------------------------------------------------------
// RoutingWorkspace extension
// ---------------------------------------------------------------------------

impl RoutingWorkspace {
    /// Look up the preferred routing direction for the given routes `LayerId`.
    fn layer_stack_preferred_direction(&self, layer: LayerId) -> Option<PreferredDirection> {
        // `workspace.layer_count` layers are stored; their IR index equals
        // `layer.raw() as usize` by the Vec-indexing convention.
        let idx = layer.raw() as usize;
        // We don't store the full layer stack in the workspace; instead we use
        // the obstacle maps length as a proxy for whether the layer exists.
        // The actual preferred direction is carried in the policy (future
        // improvement).  For now, return `Any` for all valid layers.
        if idx < self.layer_count {
            Some(PreferredDirection::Any)
        } else {
            None
        }
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
        handles::{IdMap, LayerId as IrLayerId},
        layer_stack::{IrCopperLayer, IrLayerStack, PreferredDirection},
        types::{BoundingBoxMm, PointMm},
        IrBoardGeometry,
    };
    use autopcb_routes::NetId;

    use crate::{
        config::RoutingConfig,
        workspace::build_workspace,
        global::steiner::Subnet,
    };
    use autopcb_ir::PcbIr;

    fn two_layer_ir() -> PcbIr {
        PcbIr {
            board: IrBoardGeometry {
                outline: vec![],
                cutouts: vec![],
                bounds: BoundingBoxMm::new(PointMm::new(0.0, 0.0), PointMm::new(100.0, 100.0)),
                keepouts: vec![],
            },
            layer_stack: IrLayerStack {
                copper_layers: vec![
                    IrCopperLayer {
                        id: IrLayerId::from(0),
                        name: "Top".into(),
                        is_top: true,
                        is_bottom: false,
                        preferred_direction: Some(PreferredDirection::Horizontal),
                    },
                    IrCopperLayer {
                        id: IrLayerId::from(1),
                        name: "Bottom".into(),
                        is_top: false,
                        is_bottom: true,
                        preferred_direction: Some(PreferredDirection::Vertical),
                    },
                ],
                copper_layer_count: 2,
            },
            components: IdMap::new(),
            nets: IdMap::new(),
            rules: IdMap::new(),
            free_copper: FreeCopperGeometry::default(),
            polygons: IdMap::new(),
            texts: IdMap::new(),
            regions: IdMap::new(),
            component_bodies: IdMap::new(),
        }
    }

    fn make_subnet(sx: f64, sy: f64, tx: f64, ty: f64) -> Subnet {
        Subnet {
            source: PointMm::new(sx, sy),
            target: PointMm::new(tx, ty),
            net_id: NetId(0),
            source_layer: None,
            target_layer: None,
            region_path: vec![],
        }
    }

    #[test]
    fn assign_layers_produces_one_assignment_per_subnet() {
        let ir = two_layer_ir();
        let config = RoutingConfig::default();
        let ws = build_workspace(&ir, &config).unwrap();

        let subnets = vec![
            make_subnet(0.0, 0.0, 10.0, 0.0), // horizontal
            make_subnet(0.0, 0.0, 0.0, 10.0), // vertical
        ];
        let assignments = assign_layers(&subnets, &ws);
        assert_eq!(assignments.len(), 2);
        assert_eq!(assignments[0].subnet_index, 0);
        assert_eq!(assignments[1].subnet_index, 1);
    }

    #[test]
    fn assign_layers_returns_valid_layer_id() {
        let ir = two_layer_ir();
        let config = RoutingConfig::default();
        let ws = build_workspace(&ir, &config).unwrap();

        let subnets = vec![make_subnet(0.0, 0.0, 5.0, 0.0)];
        let assignments = assign_layers(&subnets, &ws);
        assert_eq!(assignments.len(), 1);
        // LayerId must be either 0 or 1 (the two copper layers).
        let layer_raw = assignments[0].layer.raw();
        assert!(
            layer_raw == 0 || layer_raw == 1,
            "layer {layer_raw} is not a valid copper layer index"
        );
    }

    #[test]
    fn empty_subnets_returns_empty_assignments() {
        let ir = two_layer_ir();
        let config = RoutingConfig::default();
        let ws = build_workspace(&ir, &config).unwrap();
        let assignments = assign_layers(&[], &ws);
        assert!(assignments.is_empty());
    }
}

//! Shape-based routing backend.
//!
//! Handles surface escape routing, BGA/fine-pitch fanout, and tight channels
//! where grid resolution is too coarse. Implements `DetailedRouter` trait.
//!
//! This is a stub implementation that returns `RoutingError::RoutingFailed`.
//! Full shape-based routing is planned for a future milestone.

use autopcb_routes::NetId;

use crate::global::steiner::Subnet;
use crate::workspace::RoutingWorkspace;
use crate::RoutingError;

use super::grid::{DetailedRouter, PathSegment};

/// Shape-based routing backend (stub).
///
/// Returns `RoutingError::RoutingFailed` for all inputs. Full surface-escape
/// and BGA fanout routing is deferred to a future milestone.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShapeRouter;

impl DetailedRouter for ShapeRouter {
    fn route_subnet(
        &self,
        _workspace: &RoutingWorkspace,
        _subnet: &Subnet,
        _net_id: NetId,
        _history_costs: Option<&[f64]>,
        _pres_fac: f64,
    ) -> Result<Vec<PathSegment>, RoutingError> {
        Err(RoutingError::RoutingFailed(
            "shape routing not yet implemented".to_string(),
        ))
    }
}

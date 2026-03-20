//! Detailed routing stage: 3D A* pathfinding on `(x, y, layer)` node space.
//!
//! Implements grid-based routing with configurable movement style, via cost
//! model, and shape-based routing backend for fanout/escape cases.
//!
//! # Architecture
//!
//! The `DetailedRouter` trait abstracts over the grid-based (`GridRouter`) and
//! shape-based (`ShapeRouter`) backends. Both return `Vec<PathSegment>` which
//! is then converted to mm-space `TraceSegment`s and `RoutedVia`s by
//! `route_subnet_to_traces`.
//!
//! # PathFinder integration
//!
//! `route_subnet` accepts an optional `history_costs` slice (M7 hook). When
//! `Some`, the linearized per-cell history cost is added to each A* neighbour
//! cost to steer the router away from congested cells during rip-up/reroute.

pub mod astar;
pub mod fanout;
pub mod grid;
pub mod shape;
pub mod via_cost;

pub use grid::{DetailedRouter, GridNode, GridRouter, PathSegment, route_subnet_to_traces};
pub use shape::ShapeRouter;
pub use via_cost::ViaCostModel;

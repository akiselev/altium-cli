//! Fanout routing hooks.
//!
//! Generates initial escape segments from pad locations to the first routable
//! grid point, driven by fanout rules in `RoutingPolicy`.
//!
//! # Future work
//!
//! This module is a placeholder for BGA/fine-pitch fanout support. Full fanout
//! routing generates short escape vias or traces from dense pad arrays (e.g.,
//! 0.4mm-pitch BGA) to the nearest unobstructed grid cell on an inner layer,
//! creating routable access points for the main detailed router.
//!
//! The fanout stage runs before the main A* loop and writes pre-routed
//! segments and vias into the workspace obstacle maps so that subsequent
//! detailed routing treats them as fixed copper.

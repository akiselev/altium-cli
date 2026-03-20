//! Bus routing: parallel group routing with member ordering and spacing.
//!
//! A bus is a group of nets that should be routed in parallel through a
//! constrained channel while preserving their relative ordering (to minimise
//! crossings) and maintaining equal spacing between members.
//!
//! ## Planned features
//!
//! - **Member ordering**: sort bus nets by pin position to minimise crossing
//!   count before routing.
//! - **Channel routing**: route the ordered group as parallel traces through a
//!   constrained area using a channel-routing algorithm.
//! - **Spacing preservation**: enforce equal centre-to-centre spacing between
//!   adjacent bus members throughout the channel.
//!
//! This module is a placeholder. Bus routing will be implemented in a future
//! milestone once the detailed router supports bus-mode routing.

/// Placeholder struct for the future bus routing implementation.
///
/// Bus routing is not yet implemented. This type is reserved for the API that
/// will order bus members, route them as a parallel group, and enforce spacing.
pub struct BusRouter;

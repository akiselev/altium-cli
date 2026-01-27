//! Editing engine for programmatic modification of Altium schematics and PCBs.
//!
//! This module provides a comprehensive editing system including:
//! - Layout engine for component placement and collision detection
//! - Routing engine for automatic wire routing between pins
//! - Session management for tracking changes and validation
//! - SchLib integration for instantiating components from libraries
//! - PCB placement engine for component placement in PCB documents
//! - PCB editing session for comprehensive PCB modifications

pub mod layout;
pub mod library;
pub mod netlist;
pub mod pcb_placement;
pub mod pcb_session;
pub mod routing;
pub mod session;
pub mod types;

pub use layout::LayoutEngine;
pub use library::LibraryManager;
pub use netlist::NetlistBuilder;
pub use pcb_placement::{
    BoardEdge, ComponentPosition, ConnectedRoutes, PcbPlacementEngine, PlacementAnchor,
};
pub use pcb_session::{PcbEditOperation, PcbEditSession, Position, PrimitiveCount, TrackPath};
pub use routing::RoutingEngine;
pub use session::EditSession;
pub use types::*;

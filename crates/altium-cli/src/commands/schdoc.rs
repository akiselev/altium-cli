//! Schematic document (SchDoc) commands.
//!
//! High-level operations for exploring and analyzing Altium schematic documents.

use clap::Subcommand;
use serde::Serialize;
use std::path::PathBuf;

use crate::output::{self, TextFormat};
use altium_format::ops::{schdoc, schdoc_edit};

#[derive(Subcommand)]
pub enum SchDocCommands {
    /// Complete design overview with component categories, power architecture, and interfaces
    Overview {
        /// Path to SchDoc file
        path: PathBuf,
    },

    /// Generate bill of materials grouped by component type
    Bom {
        /// Path to SchDoc file
        path: PathBuf,
    },

    /// Extract net connectivity map
    Netlist {
        /// Path to SchDoc file
        path: PathBuf,

        /// Filter by net name (supports wildcards)
        #[arg(short, long)]
        filter: Option<String>,

        /// Minimum connections to include (default: 1)
        #[arg(short, long, default_value = "1")]
        min_connections: usize,
    },

    /// Power distribution analysis showing power rails and consumers
    PowerMap {
        /// Path to SchDoc file
        path: PathBuf,
    },

    /// Block diagram showing major ICs as functional blocks
    Blocks {
        /// Path to SchDoc file
        path: PathBuf,

        /// Include passive components (capacitors, resistors, etc.)
        #[arg(long)]
        all: bool,
    },

    /// Signal flow tracing from inputs to outputs
    SignalFlow {
        /// Path to SchDoc file
        path: PathBuf,

        /// Signal name to trace
        signal: String,
    },

    /// Multi-file hierarchical design analysis
    Project {
        /// Paths to SchDoc files
        paths: Vec<PathBuf>,
    },

    /// Document info and sheet metadata
    Info {
        /// Path to SchDoc file
        path: PathBuf,
    },

    /// Detailed record statistics
    Stats {
        /// Path to SchDoc file
        path: PathBuf,
    },

    /// List all components
    Components {
        /// Path to SchDoc file
        path: PathBuf,

        /// Show child primitive counts
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show detailed component information
    Component {
        /// Path to SchDoc file
        path: PathBuf,

        /// Component designator (e.g., U1) or index
        designator: String,

        /// Show all child primitives
        #[arg(long)]
        children: bool,
    },

    /// List all wires
    Wires {
        /// Path to SchDoc file
        path: PathBuf,

        /// Limit number of wires shown
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// List all net labels
    Nets {
        /// Path to SchDoc file
        path: PathBuf,

        /// Group by net name
        #[arg(short, long)]
        group: bool,
    },

    /// List all ports
    Ports {
        /// Path to SchDoc file
        path: PathBuf,
    },

    /// List all power objects
    Power {
        /// Path to SchDoc file
        path: PathBuf,

        /// Group by net name
        #[arg(short, long)]
        group: bool,
    },

    /// List pins (optionally filtered by component)
    Pins {
        /// Path to SchDoc file
        path: PathBuf,

        /// Filter by component designator
        #[arg(short, long)]
        component: Option<String>,
    },

    /// List all junctions
    Junctions {
        /// Path to SchDoc file
        path: PathBuf,
    },

    /// Show record hierarchy tree
    Hierarchy {
        /// Path to SchDoc file
        path: PathBuf,

        /// Maximum depth to display
        #[arg(short, long)]
        depth: Option<usize>,

        /// Start from specific component designator
        #[arg(short, long)]
        from: Option<String>,
    },

    /// Export as JSON for LLM processing
    Json {
        /// Path to SchDoc file
        path: PathBuf,

        /// Include full component details (pins, parameters)
        #[arg(long)]
        full: bool,

        /// Pretty-print JSON output
        #[arg(long)]
        pretty: bool,
    },

    /// Create new schematic document
    Create {
        /// Path to new SchDoc file
        path: PathBuf,

        /// Optional template file
        #[arg(long)]
        template: Option<PathBuf>,
    },

    /// Add component from library
    AddComponent {
        /// Path to SchDoc file
        path: PathBuf,

        /// Library path
        #[arg(short, long)]
        library: PathBuf,

        /// Component name
        #[arg(short, long)]
        component: String,

        /// X position
        #[arg(short, long)]
        x: String,

        /// Y position
        #[arg(short, long)]
        y: String,

        /// Designator
        #[arg(short, long)]
        designator: Option<String>,
    },

    /// Move component to new location
    MoveComponent {
        /// Path to SchDoc file
        path: PathBuf,

        /// Component designator
        designator: String,

        /// X position
        x: String,

        /// Y position
        y: String,
    },

    /// Delete component by designator
    DeleteComponent {
        /// Path to SchDoc file
        path: PathBuf,

        /// Component designator
        designator: String,
    },

    /// Add wire path
    AddWire {
        /// Path to SchDoc file
        path: PathBuf,

        /// Vertices as comma-separated values
        vertices: String,
    },

    /// Delete wire by index
    DeleteWire {
        /// Path to SchDoc file
        path: PathBuf,

        /// Wire index
        index: usize,
    },

    /// Add net label
    AddNetLabel {
        /// Path to SchDoc file
        path: PathBuf,

        /// Net label text
        name: String,

        /// X position
        x: String,

        /// Y position
        y: String,
    },

    /// Add power port
    AddPower {
        /// Path to SchDoc file
        path: PathBuf,

        /// Power net name
        name: String,

        /// X position
        x: String,

        /// Y position
        y: String,

        /// Style (bar, arrow, wave, ground, etc.)
        style: String,

        /// Orientation (up, down, left, right)
        orientation: String,
    },

    /// Add junction at location
    AddJunction {
        /// Path to SchDoc file
        path: PathBuf,

        /// X position
        x: String,

        /// Y position
        y: String,
    },

    /// Auto-add junctions where wires cross
    AddMissingJunctions {
        /// Path to SchDoc file
        path: PathBuf,
    },

    /// Add port
    AddPort {
        /// Path to SchDoc file
        path: PathBuf,

        /// Port name
        name: String,

        /// X position
        x: String,

        /// Y position
        y: String,

        /// I/O type (input, output, bidirectional, unspecified)
        io_type: String,
    },

    /// Route wire between two points
    RouteWire {
        /// Path to SchDoc file
        path: PathBuf,

        /// From point or pin
        from: String,

        /// To point or pin
        to: String,
    },

    /// Connect two component pins with wire
    ConnectPins {
        /// Path to SchDoc file
        path: PathBuf,

        /// From component designator
        from_comp: String,

        /// From pin name
        from_pin: String,

        /// To component designator
        to_comp: String,

        /// To pin name
        to_pin: String,
    },

    /// Validate schematic connectivity
    Validate {
        /// Path to SchDoc file
        path: PathBuf,
    },

    /// Suggest placement location for component
    SuggestPlacement {
        /// Path to SchDoc file
        path: PathBuf,

        /// Library path
        #[arg(short, long)]
        library: PathBuf,

        /// Component name
        #[arg(short, long)]
        component: String,
    },

    /// Find unconnected pins
    FindUnconnected {
        /// Path to SchDoc file
        path: PathBuf,
    },

    /// Find missing junctions where wires cross
    FindMissingJunctions {
        /// Path to SchDoc file
        path: PathBuf,
    },

    /// Show netlist with connectivity details
    ShowNetlist {
        /// Path to SchDoc file
        path: PathBuf,

        /// Filter by net name
        #[arg(short, long)]
        filter: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Search component library
    SearchLibrary {
        /// Library path
        library: PathBuf,

        /// Search pattern
        pattern: String,
    },
}

pub fn run(cmd: &SchDocCommands, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        SchDocCommands::Overview { path } => {
            let result = schdoc::cmd_overview(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Bom { path } => {
            let result = schdoc::cmd_bom(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Netlist {
            path,
            filter,
            min_connections,
        } => {
            let result = schdoc::cmd_netlist(path, filter.clone(), *min_connections)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::PowerMap { path } => {
            let result = schdoc::cmd_power_map(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Blocks { path, all } => {
            let result = schdoc::cmd_blocks(path, *all)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::SignalFlow { path, signal } => {
            let result = schdoc::cmd_signal_flow(path, signal)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Project { paths } => {
            let result = schdoc::cmd_project(paths)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Info { path } => {
            let result = schdoc::cmd_info(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Stats { path } => {
            let result = schdoc::cmd_stats(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Components { path, verbose } => {
            let result = schdoc::cmd_components(path, *verbose)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Component {
            path,
            designator,
            children,
        } => {
            let result = schdoc::cmd_component(path, designator, *children)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Wires { path, limit } => {
            let result = schdoc::cmd_wires(path, *limit)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Nets { path, group } => {
            let result = schdoc::cmd_nets(path, *group)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Ports { path } => {
            let result = schdoc::cmd_ports(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Power { path, group } => {
            let result = schdoc::cmd_power(path, *group)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Pins { path, component } => {
            let result = schdoc::cmd_pins(path, component.clone(), false)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Junctions { path } => {
            let result = schdoc::cmd_junctions(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Hierarchy { path, depth, from } => {
            let result = schdoc::cmd_hierarchy(path, *depth, from.clone())?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Json { path, full, pretty } => {
            // cmd_json prints directly
            schdoc::cmd_json(path, *full, *pretty).map_err(|e| e.to_string())?;
        }
        SchDocCommands::Create { path, template } => {
            schdoc::cmd_create(path, template.clone())?;
        }
        SchDocCommands::AddComponent {
            path,
            library,
            component,
            x,
            y,
            designator,
        } => {
            schdoc_edit::cmd_add_component(path, library, component, x, y, designator.as_deref(), 0, None)?;
        }
        SchDocCommands::MoveComponent { path, designator, x, y } => {
            schdoc_edit::cmd_move_component(path, designator, x, y, None)?;
        }
        SchDocCommands::DeleteComponent { path, designator } => {
            schdoc_edit::cmd_delete_component(path, designator, None)?;
        }
        SchDocCommands::AddWire { path, vertices } => {
            schdoc_edit::cmd_add_wire(path, vertices, None)?;
        }
        SchDocCommands::DeleteWire { path, index } => {
            schdoc_edit::cmd_delete_wire(path, *index, None)?;
        }
        SchDocCommands::AddNetLabel { path, name, x, y } => {
            schdoc_edit::cmd_add_net_label(path, name, x, y, None)?;
        }
        SchDocCommands::AddPower {
            path,
            name,
            x,
            y,
            style,
            orientation,
        } => {
            schdoc_edit::cmd_add_power(path, name, x, y, style, orientation, None)?;
        }
        SchDocCommands::AddJunction { path, x, y } => {
            schdoc_edit::cmd_add_junction(path, x, y, None)?;
        }
        SchDocCommands::AddMissingJunctions { path } => {
            schdoc_edit::cmd_add_missing_junctions(path, None)?;
        }
        SchDocCommands::AddPort {
            path,
            name,
            x,
            y,
            io_type,
        } => {
            schdoc_edit::cmd_add_port(path, name, x, y, io_type, None)?;
        }
        SchDocCommands::RouteWire { path, from, to } => {
            schdoc_edit::cmd_route_wire(path, from, to, None)?;
        }
        SchDocCommands::ConnectPins {
            path,
            from_comp,
            from_pin,
            to_comp,
            to_pin,
        } => {
            schdoc_edit::cmd_connect_pins(path, from_comp, from_pin, to_comp, to_pin, None)?;
        }
        SchDocCommands::Validate { path } => {
            let result = schdoc_edit::cmd_validate(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::SuggestPlacement {
            path,
            library,
            component,
        } => {
            schdoc_edit::cmd_suggest_placement(path, library, component, None, format == "json")?;
        }
        SchDocCommands::FindUnconnected { path } => {
            let result = schdoc_edit::cmd_find_unconnected(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::FindMissingJunctions { path } => {
            let result = schdoc_edit::cmd_find_missing_junctions(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::ShowNetlist { path, filter, json } => {
            schdoc_edit::cmd_show_netlist(path, filter.as_deref(), *json).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        }
        SchDocCommands::SearchLibrary {
            library,
            pattern,
        } => {
            let result = schdoc_edit::cmd_search_library(library, pattern)?;
            output::print(&TextWrapper(result), format)?;
        }
    }
    Ok(())
}

// Wrapper to add TextFormat impl for library types
#[derive(Serialize)]
#[serde(transparent)]
struct TextWrapper<T>(T);

impl<T: Serialize> TextFormat for TextWrapper<T> {
    fn format_text(&self) -> String {
        // Use serde_json to get a Value, then format it nicely
        if let Ok(value) = serde_json::to_value(&self.0) {
            format_value(&value, 0)
        } else {
            "Error formatting output".to_string()
        }
    }
}

fn format_value(value: &serde_json::Value, indent: usize) -> String {
    let prefix = "  ".repeat(indent);
    match value {
        serde_json::Value::Object(map) => {
            let mut out = String::new();
            for (key, val) in map {
                match val {
                    serde_json::Value::String(s) => {
                        out.push_str(&format!("{}{}: {}\n", prefix, key, s));
                    }
                    serde_json::Value::Number(n) => {
                        out.push_str(&format!("{}{}: {}\n", prefix, key, n));
                    }
                    serde_json::Value::Bool(b) => {
                        out.push_str(&format!("{}{}: {}\n", prefix, key, b));
                    }
                    serde_json::Value::Null => {
                        out.push_str(&format!("{}{}: null\n", prefix, key));
                    }
                    serde_json::Value::Array(arr) => {
                        if arr.is_empty() {
                            out.push_str(&format!("{}{}: []\n", prefix, key));
                        } else {
                            out.push_str(&format!("{}{}:\n", prefix, key));
                            for item in arr {
                                out.push_str(&format_value(item, indent + 1));
                                out.push('\n');
                            }
                        }
                    }
                    serde_json::Value::Object(_) => {
                        out.push_str(&format!("{}{}:\n", prefix, key));
                        out.push_str(&format_value(val, indent + 1));
                    }
                }
            }
            out
        }
        serde_json::Value::Array(arr) => {
            let mut out = String::new();
            for (i, item) in arr.iter().enumerate() {
                out.push_str(&format!("{}[{}]\n", prefix, i));
                out.push_str(&format_value(item, indent + 1));
            }
            out
        }
        serde_json::Value::String(s) => format!("{}{}\n", prefix, s),
        serde_json::Value::Number(n) => format!("{}{}\n", prefix, n),
        serde_json::Value::Bool(b) => format!("{}{}\n", prefix, b),
        serde_json::Value::Null => format!("{}null\n", prefix),
    }
}

//! Schematic document (SchDoc) commands.
//!
//! High-level operations for exploring and analyzing Altium schematic documents.

use clap::Subcommand;
use serde::Serialize;
use std::path::PathBuf;

use crate::output::{self, TextFormat};
use altium_format::ops::{schdoc, schdoc_edit, schdoc_patterns};

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

    /// Auto-wire a pin with a net label or power port
    SmartWire {
        /// Path to SchDoc file
        path: PathBuf,

        /// Component designator (e.g., U1)
        component: String,

        /// Pin designator or name
        pin: String,

        /// Net name for the label or power port
        net: String,

        /// Create a power port instead of a net label
        #[arg(long)]
        power: Option<String>,

        /// Wire stub length in mils (default: 200)
        #[arg(long, default_value = "200")]
        wire_length: f64,
    },

    /// Batch auto-wire pins from a mapping string
    SmartWireBatch {
        /// Path to SchDoc file
        path: PathBuf,

        /// Pin mappings: "COMP.PIN=NET,COMP.PIN=NET:power_style,..."
        mappings: String,

        /// Wire stub length in mils (default: 200)
        #[arg(long, default_value = "200")]
        wire_length: f64,
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

    // ═══════════════════════════════════════════════════════════════════════
    // PATTERN COMMANDS
    // ═══════════════════════════════════════════════════════════════════════

    /// List all available schematic patterns
    PatternList,

    /// Add bypass/decoupling capacitor on a power pin
    PatternBypassCap {
        /// Path to SchDoc file
        path: PathBuf,
        /// Component designator (e.g., U1)
        component: String,
        /// Pin name or designator (e.g., VCC)
        pin: String,
        /// Capacitor value (documentation only, e.g., "100nF")
        #[arg(long, default_value = "100nF")]
        value: String,
        /// Ground net name
        #[arg(long, default_value = "GND")]
        gnd: String,
    },

    /// Add pull-up resistor to power rail
    PatternPullUp {
        /// Path to SchDoc file
        path: PathBuf,
        /// Component designator
        component: String,
        /// Pin name or designator
        pin: String,
        /// Resistor value (e.g., "10K")
        #[arg(long, default_value = "10K")]
        value: String,
        /// Power net name
        #[arg(long, default_value = "VCC")]
        power: String,
    },

    /// Add pull-down resistor to ground
    PatternPullDown {
        /// Path to SchDoc file
        path: PathBuf,
        /// Component designator
        component: String,
        /// Pin name or designator
        pin: String,
        /// Resistor value (e.g., "10K")
        #[arg(long, default_value = "10K")]
        value: String,
        /// Ground net name
        #[arg(long, default_value = "GND")]
        gnd: String,
    },

    /// Add test point with net label stub
    PatternTestPoint {
        /// Path to SchDoc file
        path: PathBuf,
        /// Component designator
        component: String,
        /// Pin name or designator
        pin: String,
        /// Test point label
        label: String,
    },

    /// Add series resistor between two pins
    PatternSeriesResistor {
        /// Path to SchDoc file
        path: PathBuf,
        /// Source component.pin (e.g., U1.OUT)
        from: String,
        /// Destination component.pin (e.g., U2.IN)
        to: String,
        /// Resistor value (e.g., "33R")
        #[arg(long, default_value = "33R")]
        value: String,
    },

    /// Add voltage divider
    PatternVoltageDivider {
        /// Path to SchDoc file
        path: PathBuf,
        /// High-side net name
        high_net: String,
        /// Low-side net name (ground)
        low_net: String,
        /// Top resistor value
        r_top: String,
        /// Bottom resistor value
        r_bottom: String,
        /// Output net name
        output_net: String,
        /// X position
        x: String,
        /// Y position
        y: String,
    },

    /// Add ferrite bead filter with bypass caps
    PatternFerriteFilter {
        /// Path to SchDoc file
        path: PathBuf,
        /// Input power net
        input_net: String,
        /// Output power net
        output_net: String,
        /// Ground net
        #[arg(long, default_value = "GND")]
        gnd: String,
        /// X position
        x: String,
        /// Y position
        y: String,
    },

    /// Add bulk decoupling capacitor
    PatternBulkDecoupling {
        /// Path to SchDoc file
        path: PathBuf,
        /// Power net name
        power_net: String,
        /// Ground net name
        #[arg(long, default_value = "GND")]
        gnd: String,
        /// X position
        x: String,
        /// Y position
        y: String,
    },

    /// Add series termination resistor (high-speed digital)
    PatternSeriesTermination {
        /// Path to SchDoc file
        path: PathBuf,
        /// Driver component designator
        component: String,
        /// Driver output pin
        pin: String,
        /// Net name for terminated signal
        net: String,
        /// Resistor value (e.g., "33R")
        #[arg(long, default_value = "33R")]
        value: String,
    },

    /// Add AC coupling capacitor between pins
    PatternAcCoupling {
        /// Path to SchDoc file
        path: PathBuf,
        /// Source component.pin
        from: String,
        /// Destination component.pin
        to: String,
        /// Capacitor value (e.g., "100nF")
        #[arg(long, default_value = "100nF")]
        value: String,
    },

    /// Add differential pair termination resistor
    PatternDiffPairTerm {
        /// Path to SchDoc file
        path: PathBuf,
        /// Component designator
        component: String,
        /// Positive pin
        pin_p: String,
        /// Negative pin
        pin_n: String,
        /// Resistor value (e.g., "100R")
        #[arg(long, default_value = "100R")]
        value: String,
    },

    /// Add RC low-pass filter
    PatternRcLowpass {
        /// Path to SchDoc file
        path: PathBuf,
        /// Input component designator
        component: String,
        /// Input pin
        pin: String,
        /// Output net name
        output_net: String,
        /// Resistor value
        r_value: String,
        /// Capacitor value
        c_value: String,
        /// Ground net
        #[arg(long, default_value = "GND")]
        gnd: String,
    },

    /// Add feedback voltage divider (for regulators)
    PatternFeedbackDivider {
        /// Path to SchDoc file
        path: PathBuf,
        /// Output net name (regulator output)
        output_net: String,
        /// Feedback component designator
        fb_component: String,
        /// Feedback pin
        fb_pin: String,
        /// Top resistor value
        r_top: String,
        /// Bottom resistor value
        r_bottom: String,
        /// Ground net
        #[arg(long, default_value = "GND")]
        gnd: String,
    },

    /// Add RC snubber across pins
    PatternSnubber {
        /// Path to SchDoc file
        path: PathBuf,
        /// Component designator
        component: String,
        /// First pin
        pin_a: String,
        /// Second pin
        pin_b: String,
        /// Resistor value
        r_value: String,
        /// Capacitor value
        c_value: String,
    },

    /// Add DC blocking capacitor (RF)
    PatternDcBlock {
        /// Path to SchDoc file
        path: PathBuf,
        /// Source component designator
        component: String,
        /// Source pin
        pin: String,
        /// Output net name
        to_net: String,
        /// Capacitor value
        #[arg(long, default_value = "100pF")]
        value: String,
    },

    /// Add pi attenuator network (RF)
    PatternPiAttenuator {
        /// Path to SchDoc file
        path: PathBuf,
        /// Input net name
        input_net: String,
        /// Output net name
        output_net: String,
        /// Series resistor value
        r_series: String,
        /// Shunt resistor value
        r_shunt: String,
        /// Ground net
        #[arg(long, default_value = "GND")]
        gnd: String,
        /// X position
        x: String,
        /// Y position
        y: String,
    },

    /// Add ESD protection diode pair
    PatternEsdClamp {
        /// Path to SchDoc file
        path: PathBuf,
        /// Signal component designator
        component: String,
        /// Signal pin
        pin: String,
        /// VCC net name
        #[arg(long, default_value = "VCC")]
        vcc: String,
        /// GND net name
        #[arg(long, default_value = "GND")]
        gnd: String,
    },

    /// Add TVS diode on power rail
    PatternTvsDiode {
        /// Path to SchDoc file
        path: PathBuf,
        /// Power net name
        power_net: String,
        /// Ground net name
        #[arg(long, default_value = "GND")]
        gnd: String,
        /// X position
        x: String,
        /// Y position
        y: String,
    },

    /// Add I2C pull-up resistors
    PatternI2cPullups {
        /// Path to SchDoc file
        path: PathBuf,
        /// SDA component.pin (e.g., U1.SDA)
        sda: String,
        /// SCL component.pin (e.g., U1.SCL)
        scl: String,
        /// VCC net name
        #[arg(long, default_value = "VCC")]
        vcc: String,
        /// Resistor value
        #[arg(long, default_value = "4.7K")]
        value: String,
    },

    /// Add crystal oscillator load capacitors
    PatternCrystalLoadCaps {
        /// Path to SchDoc file
        path: PathBuf,
        /// Component designator
        component: String,
        /// XTAL_IN pin
        xtal_in: String,
        /// XTAL_OUT pin
        xtal_out: String,
        /// Capacitor value
        #[arg(long, default_value = "22pF")]
        value: String,
        /// Ground net
        #[arg(long, default_value = "GND")]
        gnd: String,
    },

    /// Add RC reset circuit with pull-up
    PatternResetCircuit {
        /// Path to SchDoc file
        path: PathBuf,
        /// Component designator
        component: String,
        /// Reset pin
        pin: String,
        /// VCC net name
        #[arg(long, default_value = "VCC")]
        vcc: String,
        /// GND net name
        #[arg(long, default_value = "GND")]
        gnd: String,
        /// Resistor value
        #[arg(long, default_value = "10K")]
        r_value: String,
        /// Capacitor value
        #[arg(long, default_value = "100nF")]
        c_value: String,
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
        SchDocCommands::SmartWire {
            path,
            component,
            pin,
            net,
            power,
            wire_length,
        } => {
            schdoc_edit::cmd_smart_wire(
                path,
                component,
                pin,
                net,
                power.as_deref(),
                *wire_length,
                None,
            )?;
        }
        SchDocCommands::SmartWireBatch {
            path,
            mappings,
            wire_length,
        } => {
            schdoc_edit::cmd_smart_wire_batch(path, mappings, *wire_length, None)?;
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

        // Pattern commands
        SchDocCommands::PatternList => {
            schdoc_patterns::cmd_list_patterns()?;
        }
        SchDocCommands::PatternBypassCap { path, component, pin, value, gnd } => {
            schdoc_patterns::cmd_bypass_cap(path, component, pin, value, gnd, None)?;
        }
        SchDocCommands::PatternPullUp { path, component, pin, value, power } => {
            schdoc_patterns::cmd_pull_up(path, component, pin, value, power, None)?;
        }
        SchDocCommands::PatternPullDown { path, component, pin, value, gnd } => {
            schdoc_patterns::cmd_pull_down(path, component, pin, value, gnd, None)?;
        }
        SchDocCommands::PatternTestPoint { path, component, pin, label } => {
            schdoc_patterns::cmd_test_point(path, component, pin, label, None)?;
        }
        SchDocCommands::PatternSeriesResistor { path, from, to, value } => {
            let (from_comp, from_pin) = from.split_once('.')
                .ok_or_else(|| "Expected 'from' as Component.Pin (e.g., U1.OUT)".to_string())?;
            let (to_comp, to_pin) = to.split_once('.')
                .ok_or_else(|| "Expected 'to' as Component.Pin (e.g., U2.IN)".to_string())?;
            schdoc_patterns::cmd_series_resistor(path, from_comp, from_pin, to_comp, to_pin, value, None)?;
        }
        SchDocCommands::PatternVoltageDivider { path, high_net, low_net, r_top, r_bottom, output_net, x, y } => {
            schdoc_patterns::cmd_voltage_divider(path, high_net, low_net, r_top, r_bottom, output_net, x, y, None)?;
        }
        SchDocCommands::PatternFerriteFilter { path, input_net, output_net, gnd, x, y } => {
            schdoc_patterns::cmd_ferrite_filter(path, input_net, output_net, gnd, x, y, None)?;
        }
        SchDocCommands::PatternBulkDecoupling { path, power_net, gnd, x, y } => {
            schdoc_patterns::cmd_bulk_decoupling(path, power_net, gnd, x, y, None)?;
        }
        SchDocCommands::PatternSeriesTermination { path, component, pin, net, value } => {
            schdoc_patterns::cmd_series_termination(path, component, pin, value, net, None)?;
        }
        SchDocCommands::PatternAcCoupling { path, from, to, value } => {
            let (from_comp, from_pin) = from.split_once('.')
                .ok_or_else(|| "Expected 'from' as Component.Pin".to_string())?;
            let (to_comp, to_pin) = to.split_once('.')
                .ok_or_else(|| "Expected 'to' as Component.Pin".to_string())?;
            schdoc_patterns::cmd_ac_coupling(path, from_comp, from_pin, to_comp, to_pin, value, None)?;
        }
        SchDocCommands::PatternDiffPairTerm { path, component, pin_p, pin_n, value } => {
            schdoc_patterns::cmd_diff_pair_termination(path, component, pin_p, pin_n, value, None)?;
        }
        SchDocCommands::PatternRcLowpass { path, component, pin, output_net, r_value, c_value, gnd } => {
            schdoc_patterns::cmd_rc_lowpass(path, component, pin, output_net, r_value, c_value, gnd, None)?;
        }
        SchDocCommands::PatternFeedbackDivider { path, output_net, fb_component, fb_pin, r_top, r_bottom, gnd } => {
            schdoc_patterns::cmd_feedback_divider(path, output_net, fb_component, fb_pin, r_top, r_bottom, gnd, None)?;
        }
        SchDocCommands::PatternSnubber { path, component, pin_a, pin_b, r_value, c_value } => {
            schdoc_patterns::cmd_snubber(path, component, pin_a, pin_b, r_value, c_value, None)?;
        }
        SchDocCommands::PatternDcBlock { path, component, pin, to_net, value } => {
            schdoc_patterns::cmd_dc_block(path, component, pin, to_net, value, None)?;
        }
        SchDocCommands::PatternPiAttenuator { path, input_net, output_net, r_series, r_shunt, gnd, x, y } => {
            schdoc_patterns::cmd_pi_attenuator(path, input_net, output_net, r_series, r_shunt, gnd, x, y, None)?;
        }
        SchDocCommands::PatternEsdClamp { path, component, pin, vcc, gnd } => {
            schdoc_patterns::cmd_esd_clamp(path, component, pin, vcc, gnd, None)?;
        }
        SchDocCommands::PatternTvsDiode { path, power_net, gnd, x, y } => {
            schdoc_patterns::cmd_tvs_diode(path, power_net, gnd, x, y, None)?;
        }
        SchDocCommands::PatternI2cPullups { path, sda, scl, vcc, value } => {
            let (sda_comp, sda_pin) = sda.split_once('.')
                .ok_or_else(|| "Expected 'sda' as Component.Pin (e.g., U1.SDA)".to_string())?;
            let (scl_comp, scl_pin) = scl.split_once('.')
                .ok_or_else(|| "Expected 'scl' as Component.Pin (e.g., U1.SCL)".to_string())?;
            schdoc_patterns::cmd_i2c_pullups(path, sda_comp, sda_pin, scl_comp, scl_pin, vcc, value, None)?;
        }
        SchDocCommands::PatternCrystalLoadCaps { path, component, xtal_in, xtal_out, value, gnd } => {
            schdoc_patterns::cmd_crystal_load_caps(path, component, xtal_in, xtal_out, value, gnd, None)?;
        }
        SchDocCommands::PatternResetCircuit { path, component, pin, vcc, gnd, r_value, c_value } => {
            schdoc_patterns::cmd_reset_circuit(path, component, pin, vcc, gnd, r_value, c_value, None)?;
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

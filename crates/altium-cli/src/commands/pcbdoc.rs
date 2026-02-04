//! PCB document (PcbDoc) commands.
//!
//! High-level operations for exploring and analyzing Altium PCB documents.

use clap::Subcommand;
use serde::Serialize;
use std::path::PathBuf;

use crate::output::{self, TextFormat};
use altium_format::ops::pcbdoc;

#[derive(Subcommand)]
pub enum PcbDocCommands {
    /// Complete document overview with components, nets, and rules
    Overview {
        /// Path to PcbDoc file
        path: PathBuf,
    },

    /// Document info and statistics
    Info {
        /// Path to PcbDoc file
        path: PathBuf,
    },

    /// List all design rules
    Rules {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Filter by rule kind (e.g., "clearance", "width")
        #[arg(short, long)]
        kind: Option<String>,

        /// Show rule parameters
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show details for a specific rule
    Rule {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Rule name
        name: String,
    },

    /// List all components
    Components {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Show additional details
        #[arg(short, long)]
        verbose: bool,

        /// Filter by layer (e.g., "top", "bottom")
        #[arg(short, long)]
        layer: Option<String>,
    },

    /// Show component details
    Component {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Component designator (e.g., U1)
        designator: String,
    },

    /// List all nets
    Nets {
        /// Path to PcbDoc file
        path: PathBuf,
    },

    /// Export as JSON for LLM processing
    Json {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Include full details (rules, components)
        #[arg(long)]
        full: bool,

        /// Pretty-print JSON output
        #[arg(long)]
        pretty: bool,
    },

    /// Create new PCB document
    Create {
        /// Path to new PcbDoc file
        path: PathBuf,

        /// Optional template file
        #[arg(long)]
        template: Option<PathBuf>,
    },

    /// Show board outline
    Outline {
        /// Path to PcbDoc file
        path: PathBuf,
    },

    /// Set rectangular board outline
    SetOutlineRect {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Width (e.g., "100mm", "4000mil")
        width: String,

        /// Height (e.g., "80mm", "3200mil")
        height: String,

        /// Origin X coordinate (e.g., "0mm")
        #[arg(long, default_value = "0mm")]
        origin_x: String,

        /// Origin Y coordinate (e.g., "0mm")
        #[arg(long, default_value = "0mm")]
        origin_y: String,
    },

    /// Set board outline from vertices
    SetOutline {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Vertices as "x1,y1 x2,y2 x3,y3 ..."
        vertices: String,
    },

    /// Show board settings
    Settings {
        /// Path to PcbDoc file
        path: PathBuf,
    },

    /// Update board settings
    SetSettings {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Set display unit to metric
        #[arg(long)]
        metric: bool,

        /// Set display unit to imperial
        #[arg(long)]
        imperial: bool,

        /// Snap grid size
        #[arg(long)]
        snap_grid: Option<String>,

        /// Visible grid size
        #[arg(long)]
        visible_grid: Option<String>,

        /// Component grid size
        #[arg(long)]
        component_grid: Option<String>,

        /// Track grid size
        #[arg(long)]
        track_grid: Option<String>,

        /// Via grid size
        #[arg(long)]
        via_grid: Option<String>,

        /// Default track width
        #[arg(long)]
        track_width: Option<String>,

        /// Origin X coordinate
        #[arg(long)]
        origin_x: Option<String>,

        /// Origin Y coordinate
        #[arg(long)]
        origin_y: Option<String>,
    },

    /// Show layer stack
    Layers {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Show all layers (not just used ones)
        #[arg(long)]
        all: bool,
    },

    /// List keepout regions
    Keepouts {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Filter by layer
        #[arg(short, long)]
        layer: Option<String>,
    },

    /// Add keepout region
    AddKeepout {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Layer
        layer: String,

        /// X1 coordinate
        x1: String,

        /// Y1 coordinate
        y1: String,

        /// X2 coordinate
        x2: String,

        /// Y2 coordinate
        y2: String,
    },

    /// List board cutouts
    Cutouts {
        /// Path to PcbDoc file
        path: PathBuf,
    },

    /// Add board cutout
    AddCutout {
        /// Path to PcbDoc file
        path: PathBuf,

        /// X1 coordinate
        x1: String,

        /// Y1 coordinate
        y1: String,

        /// X2 coordinate
        x2: String,

        /// Y2 coordinate
        y2: String,
    },

    /// List copper pours
    Polygons {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Filter by layer
        #[arg(short, long)]
        layer: Option<String>,

        /// Filter by net
        #[arg(short, long)]
        net: Option<String>,
    },

    /// Show polygon details
    Polygon {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Polygon index
        index: usize,
    },

    /// Add copper pour
    AddPolygon {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Layer
        layer: String,

        /// Net name
        net: String,

        /// Vertices as "x1,y1 x2,y2 x3,y3 ..."
        vertices: String,

        /// Pour over all objects
        #[arg(long)]
        pour_over: bool,

        /// Remove dead copper
        #[arg(long)]
        remove_dead: bool,

        /// Hatch style (solid, 45deg, 90deg, horizontal, vertical, none)
        #[arg(long, default_value = "solid")]
        hatch_style: String,
    },

    /// List tracks
    Tracks {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Filter by layer
        #[arg(short, long)]
        layer: Option<String>,
    },

    /// Add track segment
    AddTrack {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Start position "x,y"
        #[arg(long)]
        start: Option<String>,

        /// End position "x,y"
        #[arg(long)]
        end: Option<String>,

        /// Start pad reference "U1.1"
        #[arg(long)]
        start_pad: Option<String>,

        /// End pad reference "U1.2"
        #[arg(long)]
        end_pad: Option<String>,

        /// Track width
        #[arg(short, long)]
        width: Option<String>,

        /// Layer
        #[arg(short, long)]
        layer: String,

        /// Net name
        #[arg(short, long)]
        net: Option<String>,
    },

    /// Add track path
    AddTrackPath {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Vertices as "x1,y1 x2,y2 x3,y3 ..."
        vertices: String,

        /// Track width
        #[arg(short, long)]
        width: Option<String>,

        /// Layer
        #[arg(short, long)]
        layer: String,

        /// Net name
        #[arg(short, long)]
        net: Option<String>,
    },

    /// List vias
    Vias {
        /// Path to PcbDoc file
        path: PathBuf,
    },

    /// Add via
    AddVia {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Position "x,y"
        #[arg(long)]
        at: Option<String>,

        /// Pad reference "U1.1"
        #[arg(long)]
        at_pad: Option<String>,

        /// Via diameter
        #[arg(short, long)]
        diameter: Option<String>,

        /// Hole size
        #[arg(long)]
        hole: Option<String>,

        /// From layer
        #[arg(long)]
        from_layer: String,

        /// To layer
        #[arg(long)]
        to_layer: String,

        /// Net name
        #[arg(short, long)]
        net: Option<String>,
    },

    /// List arcs
    Arcs {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Filter by layer
        #[arg(short, long)]
        layer: Option<String>,
    },

    /// Add arc
    AddArc {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Center position "x,y"
        center: String,

        /// Radius
        radius: String,

        /// Start angle (degrees)
        start_angle: f64,

        /// End angle (degrees)
        end_angle: f64,

        /// Arc width
        #[arg(short, long)]
        width: Option<String>,

        /// Layer
        #[arg(short, long)]
        layer: String,

        /// Net name
        #[arg(short, long)]
        net: Option<String>,
    },

    /// List fills
    Fills {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Filter by layer
        #[arg(short, long)]
        layer: Option<String>,
    },

    /// Add fill
    AddFill {
        /// Path to PcbDoc file
        path: PathBuf,

        /// First corner "x1,y1"
        x1y1: String,

        /// Second corner "x2,y2"
        x2y2: String,

        /// Layer
        #[arg(short, long)]
        layer: String,

        /// Rotation (degrees)
        #[arg(short, long, default_value = "0")]
        rotation: f64,

        /// Net name
        #[arg(short, long)]
        net: Option<String>,
    },

    /// List texts
    Texts {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Filter by layer
        #[arg(short, long)]
        layer: Option<String>,
    },

    /// Add text
    AddText {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Text string
        text: String,

        /// Position "x,y"
        at: String,

        /// Text height
        #[arg(long)]
        height: Option<String>,

        /// Layer
        #[arg(short, long)]
        layer: String,

        /// Rotation (degrees)
        #[arg(short, long, default_value = "0")]
        rotation: f64,
    },

    /// List regions
    Regions {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Filter by layer
        #[arg(short, long)]
        layer: Option<String>,
    },

    /// Add region
    AddRegion {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Vertices as "x1,y1 x2,y2 x3,y3 ..."
        vertices: String,

        /// Layer
        #[arg(short, long)]
        layer: String,

        /// Mark as keepout region
        #[arg(long)]
        keepout: bool,

        /// Net name
        #[arg(short, long)]
        net: Option<String>,
    },

    /// Place component
    PlaceComponent {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Component designator
        designator: String,

        /// Position "x,y"
        #[arg(long)]
        at: Option<String>,

        /// Position near component
        #[arg(long)]
        near: Option<String>,

        /// Align X to coordinate
        #[arg(long)]
        align_x: Option<String>,

        /// Align Y to coordinate
        #[arg(long)]
        align_y: Option<String>,

        /// Position at board edge (top, bottom, left, right)
        #[arg(long)]
        edge: Option<String>,

        /// Offset from edge
        #[arg(long)]
        offset: Option<String>,

        /// Rotation (degrees)
        #[arg(short, long)]
        rotation: Option<f64>,

        /// Layer (top, bottom)
        #[arg(short, long)]
        layer: Option<String>,

        /// Grid size for snapping
        #[arg(long)]
        grid: Option<String>,

        /// Force placement even if constraints violated
        #[arg(long)]
        force: bool,
    },

    /// Add component from schematic
    AddComponent {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Path to schematic file
        schematic: PathBuf,

        /// Component designator
        designator: String,

        /// Footprint library path
        #[arg(long)]
        footprint_lib: Option<PathBuf>,

        /// Override footprint name
        #[arg(long)]
        footprint: Option<String>,

        /// Position "x,y"
        #[arg(long)]
        at: Option<String>,

        /// Layer (top, bottom)
        #[arg(short, long, default_value = "top")]
        layer: String,
    },

    /// Add net
    AddNet {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Net name
        name: String,
    },

    /// Add design rule
    AddRule {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Rule kind (e.g., "clearance", "width")
        kind: String,

        /// Rule name
        name: String,

        /// Priority
        #[arg(short, long, default_value = "1")]
        priority: i32,

        /// Scope 1 expression
        #[arg(long, default_value = "All")]
        scope1: String,

        /// Scope 2 expression
        #[arg(long, default_value = "All")]
        scope2: String,

        /// Gap/clearance value
        #[arg(long)]
        gap: Option<String>,

        /// Minimum width
        #[arg(long)]
        min_width: Option<String>,

        /// Maximum width
        #[arg(long)]
        max_width: Option<String>,

        /// Preferred width
        #[arg(long)]
        pref_width: Option<String>,

        /// Comment
        #[arg(long)]
        comment: Option<String>,

        /// Disable rule
        #[arg(long)]
        disabled: bool,
    },

    /// Modify design rule
    ModifyRule {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Rule name
        name: String,

        /// Priority
        #[arg(short, long)]
        priority: Option<i32>,

        /// Gap/clearance value
        #[arg(long)]
        gap: Option<String>,

        /// Minimum width
        #[arg(long)]
        min_width: Option<String>,

        /// Maximum width
        #[arg(long)]
        max_width: Option<String>,

        /// Preferred width
        #[arg(long)]
        pref_width: Option<String>,

        /// Comment
        #[arg(long)]
        comment: Option<String>,

        /// Enable rule
        #[arg(long)]
        enable: bool,

        /// Disable rule
        #[arg(long)]
        disable: bool,
    },

    /// Delete design rule
    DeleteRule {
        /// Path to PcbDoc file
        path: PathBuf,

        /// Rule name
        name: String,
    },
}

pub fn run(cmd: &PcbDocCommands, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        PcbDocCommands::Overview { path } => {
            let result = pcbdoc::cmd_overview(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::Info { path } => {
            let result = pcbdoc::cmd_info(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::Rules {
            path,
            kind,
            verbose,
        } => {
            let result = pcbdoc::cmd_rules(path, kind.clone(), *verbose)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::Rule { path, name } => {
            let result = pcbdoc::cmd_rule(path, name, true)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::Components {
            path,
            verbose,
            layer,
        } => {
            let result = pcbdoc::cmd_components(path, *verbose, layer.clone())?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::Component { path, designator } => {
            let result = pcbdoc::cmd_component(path, designator, true)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::Nets { path } => {
            let result = pcbdoc::cmd_nets(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::Json { path, full, pretty } => {
            let result = pcbdoc::cmd_json(path, *full, *pretty)?;
            let json_str = if *pretty {
                serde_json::to_string_pretty(&result)?
            } else {
                serde_json::to_string(&result)?
            };
            println!("{}", json_str);
        }
        PcbDocCommands::Create { path, template } => {
            pcbdoc::cmd_create(path, template.clone())?;
        }
        PcbDocCommands::Outline { path } => {
            let result = pcbdoc::cmd_outline(path, false)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::SetOutlineRect {
            path,
            width,
            height,
            origin_x,
            origin_y,
        } => {
            pcbdoc::cmd_set_outline_rect(path, width, height, origin_x, origin_y)?;
        }
        PcbDocCommands::SetOutline { path, vertices } => {
            pcbdoc::cmd_set_outline(path, vertices)?;
        }
        PcbDocCommands::Settings { path } => {
            let result = pcbdoc::cmd_settings(path, false)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::SetSettings {
            path,
            metric,
            imperial,
            snap_grid,
            visible_grid,
            component_grid,
            track_grid,
            via_grid,
            track_width,
            origin_x,
            origin_y,
        } => {
            pcbdoc::cmd_set_settings(
                path,
                *metric,
                *imperial,
                snap_grid.clone(),
                visible_grid.clone(),
                component_grid.clone(),
                track_grid.clone(),
                via_grid.clone(),
                track_width.clone(),
                origin_x.clone(),
                origin_y.clone(),
            )?;
        }
        PcbDocCommands::Layers { path, all } => {
            let result = pcbdoc::cmd_layers(path, *all)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::Keepouts { path, layer } => {
            let result = pcbdoc::cmd_keepouts(path, layer.clone())?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::AddKeepout {
            path,
            layer,
            x1,
            y1,
            x2,
            y2,
        } => {
            pcbdoc::cmd_add_keepout(path, layer, x1, y1, x2, y2)?;
        }
        PcbDocCommands::Cutouts { path } => {
            let result = pcbdoc::cmd_cutouts(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::AddCutout {
            path,
            x1,
            y1,
            x2,
            y2,
        } => {
            pcbdoc::cmd_add_cutout(path, x1, y1, x2, y2)?;
        }
        PcbDocCommands::Polygons { path, layer, net } => {
            let result = pcbdoc::cmd_polygons(path, layer.clone(), net.clone())?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::Polygon { path, index } => {
            let result = pcbdoc::cmd_polygon(path, *index)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::AddPolygon {
            path,
            layer,
            net,
            vertices,
            pour_over,
            remove_dead,
            hatch_style,
        } => {
            pcbdoc::cmd_add_polygon(
                path,
                layer,
                net,
                vertices,
                *pour_over,
                *remove_dead,
                hatch_style,
            )?;
        }
        PcbDocCommands::Tracks { path, layer } => {
            let result = pcbdoc::cmd_tracks(path, layer.clone())?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::AddTrack {
            path,
            start,
            end,
            start_pad,
            end_pad,
            width,
            layer,
            net,
        } => {
            pcbdoc::cmd_add_track(
                path,
                start.clone(),
                end.clone(),
                start_pad.clone(),
                end_pad.clone(),
                width.clone(),
                layer,
                net.clone(),
            )?;
        }
        PcbDocCommands::AddTrackPath {
            path,
            vertices,
            width,
            layer,
            net,
        } => {
            pcbdoc::cmd_add_track_path(path, vertices, width.clone(), layer, net.clone())?;
        }
        PcbDocCommands::Vias { path } => {
            let result = pcbdoc::cmd_vias(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::AddVia {
            path,
            at,
            at_pad,
            diameter,
            hole,
            from_layer,
            to_layer,
            net,
        } => {
            pcbdoc::cmd_add_via(
                path,
                at.clone(),
                at_pad.clone(),
                diameter.clone(),
                hole.clone(),
                from_layer,
                to_layer,
                net.clone(),
            )?;
        }
        PcbDocCommands::Arcs { path, layer } => {
            let result = pcbdoc::cmd_arcs(path, layer.clone())?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::AddArc {
            path,
            center,
            radius,
            start_angle,
            end_angle,
            width,
            layer,
            net,
        } => {
            pcbdoc::cmd_add_arc(
                path,
                center,
                radius,
                *start_angle,
                *end_angle,
                width.clone(),
                layer,
                net.clone(),
            )?;
        }
        PcbDocCommands::Fills { path, layer } => {
            let result = pcbdoc::cmd_fills(path, layer.clone())?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::AddFill {
            path,
            x1y1,
            x2y2,
            layer,
            rotation,
            net,
        } => {
            // Parse x1y1 and x2y2 as "x,y" into separate coordinates
            let (x1, y1) = parse_coordinate_pair(x1y1)?;
            let (x2, y2) = parse_coordinate_pair(x2y2)?;
            pcbdoc::cmd_add_fill(path, layer, net.as_deref(), &x1, &y1, &x2, &y2, *rotation)?;
        }
        PcbDocCommands::Texts { path, layer } => {
            let result = pcbdoc::cmd_texts(path, layer.clone())?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::AddText {
            path,
            text,
            at,
            height,
            layer,
            rotation,
        } => {
            // Parse "at" as "x,y" into separate coordinates
            let (x, y) = parse_coordinate_pair(at)?;
            pcbdoc::cmd_add_text(path, layer, text, &x, &y, height.clone(), *rotation, false)?;
        }
        PcbDocCommands::Regions { path, layer } => {
            let result = pcbdoc::cmd_regions(path, layer.clone())?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbDocCommands::AddRegion {
            path,
            vertices,
            layer,
            keepout,
            net,
        } => {
            let kind = if *keepout { Some("keepout") } else { None };
            pcbdoc::cmd_add_region(path, layer, net.as_deref(), vertices, kind)?;
        }
        PcbDocCommands::PlaceComponent {
            path,
            designator,
            at,
            near,
            align_x,
            align_y,
            edge,
            offset,
            rotation,
            layer,
            grid,
            force,
        } => {
            pcbdoc::cmd_place_component(
                path,
                designator,
                at.clone(),
                near.clone(),
                align_x.clone(),
                align_y.clone(),
                edge.clone(),
                offset.clone(),
                *rotation,
                layer.clone(),
                grid.clone(),
                *force,
            )?;
        }
        PcbDocCommands::AddComponent {
            path,
            schematic,
            designator,
            footprint_lib,
            footprint,
            at,
            layer,
        } => {
            pcbdoc::cmd_add_component(
                path,
                schematic,
                designator,
                footprint_lib.clone(),
                footprint.clone(),
                at.clone(),
                layer,
            )?;
        }
        PcbDocCommands::AddNet { path, name } => {
            pcbdoc::cmd_add_net(path, name)?;
        }
        PcbDocCommands::AddRule {
            path,
            kind,
            name,
            priority,
            scope1,
            scope2,
            gap,
            min_width,
            max_width,
            pref_width,
            comment,
            disabled,
        } => {
            pcbdoc::cmd_add_rule(
                path,
                kind,
                name,
                *priority,
                scope1,
                scope2,
                gap.clone(),
                min_width.clone(),
                max_width.clone(),
                pref_width.clone(),
                comment.clone(),
                *disabled,
            )?;
        }
        PcbDocCommands::ModifyRule {
            path,
            name,
            priority,
            gap,
            min_width,
            max_width,
            pref_width,
            comment,
            enable,
            disable,
        } => {
            pcbdoc::cmd_modify_rule(
                path,
                name,
                *priority,
                gap.clone(),
                min_width.clone(),
                max_width.clone(),
                pref_width.clone(),
                comment.clone(),
                *enable,
                *disable,
            )?;
        }
        PcbDocCommands::DeleteRule { path, name } => {
            pcbdoc::cmd_delete_rule(path, name)?;
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

/// Parse a coordinate pair string "x,y" into separate x and y strings.
fn parse_coordinate_pair(s: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid coordinate pair '{}', expected format 'x,y'", s).into());
    }
    Ok((parts[0].trim().to_string(), parts[1].trim().to_string()))
}

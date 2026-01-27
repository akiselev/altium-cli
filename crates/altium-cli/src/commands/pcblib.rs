//! PCB footprint library (PcbLib) commands.
//!
//! High-level operations for exploring and managing Altium PCB footprint libraries.

use clap::Subcommand;
use serde::Serialize;
use std::path::PathBuf;

use crate::output::{self, TextFormat};
use altium_format::ops::pcblib;

#[derive(Subcommand)]
pub enum PcbLibCommands {
    /// Complete library overview with footprint categories and statistics
    Overview {
        /// Path to PcbLib file
        path: PathBuf,
    },

    /// List all footprints in the library
    List {
        /// Path to PcbLib file
        path: PathBuf,
    },

    /// Search for footprints by name or description
    Search {
        /// Path to PcbLib file
        path: PathBuf,

        /// Search query
        query: String,

        /// Maximum results to return
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Library info and statistics
    Info {
        /// Path to PcbLib file
        path: PathBuf,
    },

    /// Show detailed footprint information
    Footprint {
        /// Path to PcbLib file
        path: PathBuf,

        /// Footprint name or index
        name: String,

        /// Show primitive details
        #[arg(long)]
        primitives: bool,
    },

    /// List pads (all or for specific footprint)
    Pads {
        /// Path to PcbLib file
        path: PathBuf,

        /// Filter by footprint name
        #[arg(short, long)]
        footprint: Option<String>,
    },

    /// List primitives for a footprint
    Primitives {
        /// Path to PcbLib file
        path: PathBuf,

        /// Footprint name
        footprint: String,
    },

    /// Analyze hole sizes across the library
    Holes {
        /// Path to PcbLib file
        path: PathBuf,
    },

    /// Measure footprint dimensions and clearances
    Measure {
        /// Path to PcbLib file
        path: PathBuf,

        /// Footprint name
        footprint: String,
    },

    /// Export as JSON for LLM processing
    Json {
        /// Path to PcbLib file
        path: PathBuf,

        /// Include full pad details
        #[arg(long)]
        full: bool,
    },

    /// Render footprint as ASCII art
    RenderAscii {
        /// Path to PcbLib file
        path: PathBuf,

        /// Footprint name
        footprint: String,
    },

    /// Create new empty library
    Create {
        /// Path to new PcbLib file
        path: PathBuf,
    },

    /// Add footprint pattern to library
    AddFootprint {
        /// Path to PcbLib file
        path: PathBuf,

        /// Footprint name
        name: String,

        /// Footprint description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Add pad to footprint
    AddPad {
        /// Path to PcbLib file
        path: PathBuf,

        /// Footprint name
        #[arg(short, long)]
        footprint: String,

        /// Pad designator
        #[arg(short, long)]
        designator: String,

        /// X position in mm
        #[arg(short, long)]
        x: f64,

        /// Y position in mm
        #[arg(short, long)]
        y: f64,

        /// Pad width in mm
        #[arg(short, long)]
        width: f64,

        /// Pad height in mm
        #[arg(long)]
        height: f64,

        /// Pad shape (round, rectangular, rounded_rect, octagonal)
        #[arg(short, long, default_value = "rectangular")]
        shape: String,

        /// Hole diameter in mm (0 for SMD)
        #[arg(long, default_value = "0")]
        hole: f64,
    },

    /// Add silkscreen line to footprint
    AddSilkscreen {
        /// Path to PcbLib file
        path: PathBuf,

        /// Footprint name
        #[arg(short, long)]
        footprint: String,

        /// Start X in mm
        #[arg(long)]
        x1: f64,

        /// Start Y in mm
        #[arg(long)]
        y1: f64,

        /// End X in mm
        #[arg(long)]
        x2: f64,

        /// End Y in mm
        #[arg(long)]
        y2: f64,

        /// Line width in mm
        #[arg(short, long, default_value = "0.15")]
        width: f64,
    },

    /// Add silkscreen arc to footprint
    AddArc {
        /// Path to PcbLib file
        path: PathBuf,

        /// Footprint name
        #[arg(short, long)]
        footprint: String,

        /// Center X in mm
        #[arg(short, long)]
        x: f64,

        /// Center Y in mm
        #[arg(short, long)]
        y: f64,

        /// Radius in mm
        #[arg(short, long)]
        radius: f64,

        /// Start angle in degrees
        #[arg(long)]
        start_angle: f64,

        /// End angle in degrees
        #[arg(long)]
        end_angle: f64,

        /// Line width in mm
        #[arg(short, long, default_value = "0.15")]
        width: f64,
    },

    /// Generate chip footprint by size
    GenChip {
        /// Path to PcbLib file
        path: PathBuf,

        /// Chip size (0201, 0402, 0603, 0805, 1206)
        size: String,

        /// Pad density (most, nominal, least)
        #[arg(short, long, default_value = "nominal")]
        density: String,
    },

    /// Render footprint as SVG
    RenderSvg {
        /// Path to PcbLib file
        path: PathBuf,

        /// Footprint name
        footprint: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Scale in pixels per mil
        #[arg(short, long, default_value = "0.5")]
        scale: f64,

        /// Use light theme
        #[arg(long)]
        light: bool,

        /// Hide grid
        #[arg(long)]
        no_grid: bool,

        /// Hide pad designators
        #[arg(long)]
        no_designators: bool,
    },

    /// Render footprint as PNG
    RenderPng {
        /// Path to PcbLib file
        path: PathBuf,

        /// Footprint name
        footprint: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Scale in pixels per mil
        #[arg(short, long, default_value = "0.5")]
        scale: f64,

        /// Target width in pixels
        #[arg(short, long)]
        width: Option<u32>,
    },

    /// Batch import from JSON
    AddJson {
        /// Path to PcbLib file
        path: PathBuf,

        /// JSON file path (use "-" for stdin)
        #[arg(short, long)]
        file: Option<String>,

        /// JSON string
        #[arg(short, long)]
        json: Option<String>,
    },

    /// Add row of pads to footprint
    AddPadRow {
        /// Path to PcbLib file
        path: PathBuf,

        /// Footprint name
        #[arg(short, long)]
        footprint: String,

        /// Number of pads
        #[arg(short, long)]
        count: usize,

        /// Pitch with unit (e.g., "2.54mm", "100mil")
        #[arg(short, long)]
        pitch: String,

        /// Pad width with unit
        #[arg(long)]
        pad_width: String,

        /// Pad height with unit
        #[arg(long)]
        pad_height: String,

        /// Direction (horizontal, vertical, h, v, x, y)
        #[arg(short, long, default_value = "horizontal")]
        direction: String,

        /// Starting pad number
        #[arg(short, long, default_value = "1")]
        start: u32,

        /// Row X offset with unit
        #[arg(short, long, default_value = "0mm")]
        x: String,

        /// Row Y offset with unit
        #[arg(short, long, default_value = "0mm")]
        y: String,

        /// Pad shape
        #[arg(long, default_value = "rectangular")]
        shape: String,

        /// Hole diameter with unit (0mm for SMD)
        #[arg(long, default_value = "0mm")]
        hole: String,

        /// Interpret pitch as spacing (pad edge to edge)
        #[arg(long)]
        use_spacing: bool,
    },

    /// Add dual row of pads (SOIC, DIP style)
    AddDualRow {
        /// Path to PcbLib file
        path: PathBuf,

        /// Footprint name
        #[arg(short, long)]
        footprint: String,

        /// Pads per side
        #[arg(short = 'n', long)]
        pads_per_side: usize,

        /// Pitch with unit
        #[arg(short, long)]
        pitch: String,

        /// Row spacing with unit
        #[arg(short, long)]
        row_spacing: String,

        /// Pad width with unit (for SMD or through-hole)
        #[arg(long)]
        pad_width: Option<String>,

        /// Pad height with unit (for SMD)
        #[arg(long)]
        pad_height: Option<String>,

        /// Pad diameter with unit (for through-hole)
        #[arg(long)]
        pad_diameter: Option<String>,

        /// Hole diameter with unit (makes through-hole)
        #[arg(long)]
        hole: Option<String>,

        /// Pad shape
        #[arg(long, default_value = "rectangular")]
        shape: String,
    },

    /// Add quad pattern pads (QFP style)
    AddQuadPads {
        /// Path to PcbLib file
        path: PathBuf,

        /// Footprint name
        #[arg(short, long)]
        footprint: String,

        /// Pads per side
        #[arg(short = 'n', long)]
        pads_per_side: usize,

        /// Pitch with unit
        #[arg(short, long)]
        pitch: String,

        /// Span with unit (center to center of opposite rows)
        #[arg(short, long)]
        span: String,

        /// Pad width with unit
        #[arg(long)]
        pad_width: String,

        /// Pad height with unit
        #[arg(long)]
        pad_height: String,

        /// Pad shape
        #[arg(long, default_value = "rectangular")]
        shape: String,
    },

    /// Add grid of pads (BGA style)
    AddPadGrid {
        /// Path to PcbLib file
        path: PathBuf,

        /// Footprint name
        #[arg(short, long)]
        footprint: String,

        /// Number of rows
        #[arg(short, long)]
        rows: usize,

        /// Number of columns
        #[arg(short, long)]
        cols: usize,

        /// Pitch with unit
        #[arg(short, long)]
        pitch: String,

        /// Pad diameter with unit
        #[arg(long)]
        pad_diameter: String,

        /// Pad shape
        #[arg(long, default_value = "round")]
        shape: String,

        /// Skip center region diameter with unit
        #[arg(long, default_value = "0mm")]
        skip_center: String,
    },
}

pub fn run(cmd: &PcbLibCommands, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        PcbLibCommands::Overview { path } => {
            let result = pcblib::cmd_overview(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbLibCommands::List { path } => {
            let result = pcblib::cmd_list(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbLibCommands::Search {
            path,
            query,
            limit: _,
        } => {
            let result = pcblib::cmd_search(path, query)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbLibCommands::Info { path } => {
            let result = pcblib::cmd_info(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbLibCommands::Footprint {
            path,
            name,
            primitives,
        } => {
            let result = pcblib::cmd_footprint(path, name, *primitives)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbLibCommands::Pads { path, footprint } => {
            let result = pcblib::cmd_pads(path, footprint.clone(), false)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbLibCommands::Primitives { path, footprint } => {
            let result = pcblib::cmd_primitives(path, footprint)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbLibCommands::Holes { path } => {
            let result = pcblib::cmd_holes(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        PcbLibCommands::Measure { path, footprint } => {
            // Use "summary" type for overview measurement
            pcblib::cmd_measure(path, footprint, "summary", None, None, None, format == "json")?;
        }
        PcbLibCommands::Json { path, full } => {
            let result = pcblib::cmd_json(path, *full)?;
            let json_str = serde_json::to_string_pretty(&result)?;
            println!("{}", json_str);
        }
        PcbLibCommands::RenderAscii { path, footprint } => {
            pcblib::cmd_render_ascii(path, footprint, 80, 40)?;
        }
        PcbLibCommands::Create { path } => {
            pcblib::cmd_create(path)?;
        }
        PcbLibCommands::AddFootprint {
            path,
            name,
            description,
        } => {
            pcblib::cmd_add_footprint(path, name, description.clone())?;
        }
        PcbLibCommands::AddPad {
            path,
            footprint,
            designator,
            x,
            y,
            width,
            height,
            shape,
            hole,
        } => {
            pcblib::cmd_add_pad(
                path,
                footprint,
                designator,
                *x,
                *y,
                *width,
                *height,
                shape,
                *hole,
            )?;
        }
        PcbLibCommands::AddSilkscreen {
            path,
            footprint,
            x1,
            y1,
            x2,
            y2,
            width,
        } => {
            pcblib::cmd_add_silkscreen(path, footprint, *x1, *y1, *x2, *y2, *width)?;
        }
        PcbLibCommands::AddArc {
            path,
            footprint,
            x,
            y,
            radius,
            start_angle,
            end_angle,
            width,
        } => {
            pcblib::cmd_add_arc(
                path,
                footprint,
                *x,
                *y,
                *radius,
                *start_angle,
                *end_angle,
                *width,
            )?;
        }
        PcbLibCommands::GenChip {
            path,
            size,
            density,
        } => {
            pcblib::cmd_gen_chip(path, size, density)?;
        }
        PcbLibCommands::RenderSvg {
            path,
            footprint,
            output,
            scale,
            light,
            no_grid,
            no_designators,
        } => {
            pcblib::cmd_render_svg(
                path,
                footprint,
                output.clone(),
                *scale,
                *light,
                *no_grid,
                *no_designators,
            )?;
        }
        PcbLibCommands::RenderPng {
            path,
            footprint,
            output,
            scale,
            width,
        } => {
            pcblib::cmd_render_png(path, footprint, output.clone(), *scale, *width)?;
        }
        PcbLibCommands::AddJson { path, file, json } => {
            pcblib::cmd_add_json(path, file.clone(), json.clone())?;
        }
        PcbLibCommands::AddPadRow {
            path,
            footprint,
            count,
            pitch,
            pad_width,
            pad_height,
            direction,
            start,
            x,
            y,
            shape,
            hole,
            use_spacing,
        } => {
            pcblib::cmd_add_pad_row(
                path,
                footprint,
                *count,
                pitch,
                pad_width,
                pad_height,
                direction,
                *start,
                x,
                y,
                shape,
                hole,
                *use_spacing,
            )?;
        }
        PcbLibCommands::AddDualRow {
            path,
            footprint,
            pads_per_side,
            pitch,
            row_spacing,
            pad_width,
            pad_height,
            pad_diameter,
            hole,
            shape,
        } => {
            pcblib::cmd_add_dual_row(
                path,
                footprint,
                *pads_per_side,
                pitch,
                row_spacing,
                pad_width.as_deref(),
                pad_height.as_deref(),
                pad_diameter.as_deref(),
                hole.as_deref(),
                shape,
            )?;
        }
        PcbLibCommands::AddQuadPads {
            path,
            footprint,
            pads_per_side,
            pitch,
            span,
            pad_width,
            pad_height,
            shape,
        } => {
            pcblib::cmd_add_quad_pads(
                path,
                footprint,
                *pads_per_side,
                pitch,
                span,
                pad_width,
                pad_height,
                shape,
            )?;
        }
        PcbLibCommands::AddPadGrid {
            path,
            footprint,
            rows,
            cols,
            pitch,
            pad_diameter,
            shape,
            skip_center,
        } => {
            pcblib::cmd_add_pad_grid(
                path,
                footprint,
                *rows,
                *cols,
                pitch,
                pad_diameter,
                shape,
                skip_center,
            )?;
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

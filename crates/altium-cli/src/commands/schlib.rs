//! Schematic library (SchLib) commands.
//!
//! High-level operations for exploring and managing Altium schematic libraries.

use clap::Subcommand;
use serde::Serialize;
use std::path::PathBuf;

use crate::output::{self, TextFormat};
use altium_format::ops::schlib;

#[derive(Subcommand)]
pub enum SchLibCommands {
    /// Complete library overview with component categories and statistics
    Overview {
        /// Path to SchLib file
        path: PathBuf,

        /// Include full component details
        #[arg(long)]
        full: bool,
    },

    /// List all components in the library
    List {
        /// Path to SchLib file
        path: PathBuf,
    },

    /// Search for components by name or description
    Search {
        /// Path to SchLib file
        path: PathBuf,

        /// Search query
        query: String,

        /// Maximum results to return
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Library info and statistics
    Info {
        /// Path to SchLib file
        path: PathBuf,
    },

    /// Show detailed component information
    Component {
        /// Path to SchLib file
        path: PathBuf,

        /// Component name or index
        name: String,

        /// Show primitive details
        #[arg(long)]
        primitives: bool,
    },

    /// List pins (all or for specific component)
    Pins {
        /// Path to SchLib file
        path: PathBuf,

        /// Filter by component name
        #[arg(short, long)]
        component: Option<String>,

        /// Group by electrical type
        #[arg(short, long)]
        group: bool,
    },

    /// List primitives for a component
    Primitives {
        /// Path to SchLib file
        path: PathBuf,

        /// Component name
        component: String,
    },

    /// Export as JSON for LLM processing
    Json {
        /// Path to SchLib file
        path: PathBuf,
    },

    /// Render component as ASCII art
    RenderAscii {
        /// Path to SchLib file
        path: PathBuf,

        /// Component name
        component: String,
    },

    /// Create a new empty SchLib file
    Create {
        /// Path to new SchLib file
        path: PathBuf,
    },

    /// Add a new component to the library
    AddComponent {
        /// Path to SchLib file
        path: PathBuf,

        /// Component name
        name: String,

        /// Component description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Add a pin to a component
    AddPin {
        /// Path to SchLib file
        path: PathBuf,

        /// Component name
        component: String,

        /// Pin designator (e.g., "1", "A1")
        designator: String,

        /// Pin name (e.g., "VCC", "GND")
        name: String,

        /// X position (e.g., "100mil", "2.54mm", "100")
        x: String,

        /// Y position
        y: String,

        /// Pin length (e.g., "200mil")
        #[arg(short, long, default_value = "200mil")]
        length: String,

        /// Electrical type: input, output, io, passive, power, oc, oe, hiz
        #[arg(short, long, default_value = "passive")]
        electrical: String,

        /// Pin orientation: left, right, up, down
        #[arg(short, long, default_value = "right")]
        orientation: String,

        /// Hide the pin
        #[arg(long)]
        hidden: bool,
    },

    /// Add a rectangle to a component
    AddRectangle {
        /// Path to SchLib file
        path: PathBuf,

        /// Component name
        component: String,

        /// Corner 1 X
        x1: String,

        /// Corner 1 Y
        y1: String,

        /// Corner 2 X
        x2: String,

        /// Corner 2 Y
        y2: String,

        /// Fill the rectangle
        #[arg(short, long)]
        filled: bool,

        /// Fill color in hex (RRGGBB)
        #[arg(long, default_value = "FFFFB0")]
        fill_color: String,

        /// Border color in hex (RRGGBB)
        #[arg(long, default_value = "000080")]
        border_color: String,
    },

    /// Add a line to a component
    AddLine {
        /// Path to SchLib file
        path: PathBuf,

        /// Component name
        component: String,

        /// Start X
        x1: String,

        /// Start Y
        y1: String,

        /// End X
        x2: String,

        /// End Y
        y2: String,

        /// Line color in hex (RRGGBB)
        #[arg(short, long, default_value = "000080")]
        color: String,
    },

    /// Add a polygon to a component
    AddPolygon {
        /// Path to SchLib file
        path: PathBuf,

        /// Component name
        component: String,

        /// Vertices as comma-separated values: x1,y1,x2,y2,...
        vertices: String,

        /// Fill the polygon
        #[arg(short, long)]
        filled: bool,

        /// Fill color in hex (RRGGBB)
        #[arg(long, default_value = "FFFFB0")]
        fill_color: String,

        /// Border color in hex (RRGGBB)
        #[arg(long, default_value = "000080")]
        border_color: String,
    },

    /// Generate a standard IC symbol
    GenIc {
        /// Path to SchLib file
        path: PathBuf,

        /// Component name
        name: String,

        /// Pin definitions: designator:name:type[:side],...
        pins: String,

        /// Component description
        #[arg(short, long)]
        description: Option<String>,

        /// Body width (e.g., "800mil")
        #[arg(short, long, default_value = "800mil")]
        width: String,

        /// Pin length (e.g., "200mil")
        #[arg(short = 'l', long, default_value = "200mil")]
        pin_length: String,

        /// Pin spacing (e.g., "100mil")
        #[arg(short = 's', long, default_value = "100mil")]
        pin_spacing: String,
    },

    /// Batch import component from JSON
    AddJson {
        /// Path to SchLib file
        path: PathBuf,

        /// JSON file path (use "-" for stdin)
        #[arg(short, long)]
        file: Option<String>,

        /// JSON string
        #[arg(short, long)]
        json: Option<String>,
    },

    /// Generate a complete SchLib from a YAML/JSON/TOML definition file
    GenerateFrom {
        /// Path to import definition file (.yml, .yaml, .json, .toml)
        input: PathBuf,

        /// Path to output SchLib file
        output: PathBuf,
    },
}

pub fn run(cmd: &SchLibCommands, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        SchLibCommands::Overview { path, full } => {
            let result = schlib::cmd_overview(path, *full)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchLibCommands::List { path } => {
            let result = schlib::cmd_list(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchLibCommands::Search {
            path,
            query,
            limit,
        } => {
            let result = schlib::cmd_search(path, query, *limit)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchLibCommands::Info { path } => {
            let result = schlib::cmd_info(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchLibCommands::Component {
            path,
            name,
            primitives,
        } => {
            let result = schlib::cmd_component(path, name, *primitives)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchLibCommands::Pins {
            path,
            component,
            group,
        } => {
            let result = schlib::cmd_pins(path, component.clone(), *group)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchLibCommands::Primitives { path, component } => {
            let result = schlib::cmd_primitives(path, component)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchLibCommands::Json { path } => {
            let result = schlib::cmd_json(path)?;
            let json_str = serde_json::to_string_pretty(&result)?;
            println!("{}", json_str);
        }
        SchLibCommands::RenderAscii { path, component } => {
            let ascii = schlib::cmd_render_ascii(path, component, 80, 40)?;
            println!("{}", ascii);
        }
        SchLibCommands::Create { path } => {
            let result = schlib::cmd_create(path)?;
            println!("{}", result);
        }
        SchLibCommands::AddComponent {
            path,
            name,
            description,
        } => {
            let result = schlib::cmd_add_component(path, name, description.clone())?;
            println!("{}", result);
        }
        SchLibCommands::AddPin {
            path,
            component,
            designator,
            name,
            x,
            y,
            length,
            electrical,
            orientation,
            hidden,
        } => {
            let result = schlib::cmd_add_pin(
                path,
                component,
                designator,
                name,
                x,
                y,
                length,
                electrical,
                orientation,
                *hidden,
            )?;
            println!("{}", result);
        }
        SchLibCommands::AddRectangle {
            path,
            component,
            x1,
            y1,
            x2,
            y2,
            filled,
            fill_color,
            border_color,
        } => {
            let result = schlib::cmd_add_rectangle(
                path,
                component,
                x1,
                y1,
                x2,
                y2,
                *filled,
                fill_color,
                border_color,
            )?;
            println!("{}", result);
        }
        SchLibCommands::AddLine {
            path,
            component,
            x1,
            y1,
            x2,
            y2,
            color,
        } => {
            let result = schlib::cmd_add_line(path, component, x1, y1, x2, y2, color)?;
            println!("{}", result);
        }
        SchLibCommands::AddPolygon {
            path,
            component,
            vertices,
            filled,
            fill_color,
            border_color,
        } => {
            let result = schlib::cmd_add_polygon(
                path,
                component,
                vertices,
                *filled,
                fill_color,
                border_color,
            )?;
            println!("{}", result);
        }
        SchLibCommands::GenIc {
            path,
            name,
            pins,
            description,
            width,
            pin_length,
            pin_spacing,
        } => {
            let result = schlib::cmd_gen_ic(
                path,
                name,
                pins,
                description.clone(),
                width,
                pin_length,
                pin_spacing,
            )?;
            println!("{}", result);
        }
        SchLibCommands::AddJson { path, file, json } => {
            let result = schlib::cmd_add_json(path, file.clone(), json.clone())?;
            println!("{}", result);
        }
        SchLibCommands::GenerateFrom { input, output } => {
            use altium_format::import::{parse_import_file, ImportFile};
            let import_file = parse_import_file(input)?;
            match import_file {
                ImportFile::SchLib(import) => {
                    let result =
                        altium_format::import::schlib::generate_schlib(output, &import)?;
                    println!("{}", result);
                }
                _ => {
                    return Err(format!(
                        "Import file does not have format: schlib. Check the 'format' field."
                    )
                    .into());
                }
            }
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

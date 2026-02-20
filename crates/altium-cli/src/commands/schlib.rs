//! Schematic library (SchLib) commands.
//!
//! Provides CLI interface for exploring and managing Altium schematic libraries.
//! Acts as a thin wrapper over ops::schlib functions. Contains no business logic.

use clap::Subcommand;
use serde::Serialize;
use std::path::PathBuf;

use crate::output::{self, TextFormat};
use altium_format_ops::schlib;

#[derive(Subcommand)]
pub enum SchLibCommands {
    /// Complete library overview with component categories and statistics
    Overview {
        /// Path to SchLib file
        path: PathBuf,
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

        /// Component name
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
    },

    /// List primitives for a component
    Primitives {
        /// Path to SchLib file
        path: PathBuf,

        /// Component name
        component: String,
    },

    /// Create new empty library
    Create {
        /// Path to new SchLib file
        path: PathBuf,
    },

    /// Add component to library
    AddComponent {
        /// Path to SchLib file
        path: PathBuf,

        /// Component name
        name: String,

        /// Component description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Add pin to component
    AddPin {
        /// Path to SchLib file
        path: PathBuf,

        /// Component name
        #[arg(short, long)]
        component: String,

        /// Pin designator
        #[arg(short, long)]
        designator: String,

        /// Pin name
        #[arg(short, long)]
        name: String,

        /// Electrical type (input, output, bidirectional, passive, power, etc.)
        #[arg(short, long, default_value = "passive")]
        electrical_type: String,
    },

    /// Export as JSON for LLM processing
    Json {
        /// Path to SchLib file
        path: PathBuf,

        /// Include full primitive details
        #[arg(long)]
        full: bool,
    },
}

pub fn run(cmd: &SchLibCommands, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        SchLibCommands::Overview { path } => {
            let result = schlib::cmd_overview(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchLibCommands::List { path } => {
            let result = schlib::cmd_list(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchLibCommands::Search { path, query, limit } => {
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
        SchLibCommands::Pins { path, component } => {
            let result = schlib::cmd_pins(path, component.clone())?;
            output::print(&TextWrapper(result), format)?;
        }
        SchLibCommands::Primitives { path, component } => {
            let result = schlib::cmd_primitives(path, component)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchLibCommands::Create { path } => {
            schlib::cmd_create(path)?;
        }
        SchLibCommands::AddComponent {
            path,
            name,
            description,
        } => {
            schlib::cmd_add_component(path, name, description.clone())?;
        }
        SchLibCommands::AddPin {
            path,
            component,
            designator,
            name,
            electrical_type,
        } => {
            schlib::cmd_add_pin(path, component, designator, name, electrical_type)?;
        }
        SchLibCommands::Json { path, full } => {
            let result = schlib::cmd_json(path, *full)?;
            let json_str = serde_json::to_string_pretty(&result)?;
            println!("{}", json_str);
        }
    }
    Ok(())
}

/// Wrapper that adds TextFormat impl for library types.
/// Enables JSON output types to be formatted as human-readable text.
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

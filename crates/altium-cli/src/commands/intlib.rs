//! Integrated library (IntLib) commands.
//!
//! Provides CLI interface for exploring and extracting content from Altium
//! integrated library files. Acts as a thin wrapper over ops::intlib functions.
//! Contains no business logic.

use clap::Subcommand;
use serde::Serialize;
use std::path::PathBuf;

use crate::output::{self, TextFormat};
use altium_format::ops::intlib;

#[derive(Subcommand)]
pub enum IntLibCommands {
    /// Complete library overview with component counts and statistics
    Overview {
        /// Path to IntLib file
        path: PathBuf,
    },

    /// List all components in the library
    List {
        /// Path to IntLib file
        path: PathBuf,
    },

    /// Search for components by name, description, or footprint
    Search {
        /// Path to IntLib file
        path: PathBuf,

        /// Search query (supports wildcards with *)
        query: String,

        /// Maximum number of results to return
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Get detailed information about a specific component
    Component {
        /// Path to IntLib file
        path: PathBuf,

        /// Component name
        name: String,
    },

    /// Library metadata and statistics
    Info {
        /// Path to IntLib file
        path: PathBuf,
    },

    /// List embedded schematic symbols
    Symbols {
        /// Path to IntLib file
        path: PathBuf,
    },

    /// List embedded PCB footprints
    Footprints {
        /// Path to IntLib file
        path: PathBuf,
    },

    /// List component parameters
    Parameters {
        /// Path to IntLib file
        path: PathBuf,

        /// Filter by component name
        #[arg(short, long)]
        component: Option<String>,
    },

    /// Extract embedded SchLib to standalone file
    ExtractSchlib {
        /// Path to IntLib file
        path: PathBuf,

        /// Output path for extracted SchLib
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Extract embedded PcbLib to standalone file
    ExtractPcblib {
        /// Path to IntLib file
        path: PathBuf,

        /// Output path for extracted PcbLib
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Export as JSON for LLM processing
    Json {
        /// Path to IntLib file
        path: PathBuf,

        /// Include detailed symbol and footprint information
        #[arg(long)]
        full: bool,
    },
}

pub fn run(cmd: &IntLibCommands, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        IntLibCommands::Overview { path } => {
            let result = intlib::cmd_overview(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        IntLibCommands::List { path } => {
            let result = intlib::cmd_list(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        IntLibCommands::Search { path, query, limit } => {
            let result = intlib::cmd_search(path, query, *limit)?;
            output::print(&TextWrapper(result), format)?;
        }
        IntLibCommands::Component { path, name } => {
            let result = intlib::cmd_component(path, name)?;
            output::print(&TextWrapper(result), format)?;
        }
        IntLibCommands::Info { path } => {
            let result = intlib::cmd_info(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        IntLibCommands::Symbols { path } => {
            let result = intlib::cmd_symbols(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        IntLibCommands::Footprints { path } => {
            let result = intlib::cmd_footprints(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        IntLibCommands::Parameters { path, component } => {
            let result = intlib::cmd_parameters(path, component.clone())?;
            output::print(&TextWrapper(result), format)?;
        }
        IntLibCommands::ExtractSchlib { path, output } => {
            intlib::cmd_extract_schlib(path, output)?;
        }
        IntLibCommands::ExtractPcblib { path, output } => {
            intlib::cmd_extract_pcblib(path, output)?;
        }
        IntLibCommands::Json { path, full } => {
            let result = intlib::cmd_json(path, *full)?;
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

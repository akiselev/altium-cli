//! Integrated library (IntLib) commands.
//!
//! High-level operations for exploring Altium integrated libraries.

use clap::Subcommand;
use serde::Serialize;
use std::path::PathBuf;

use crate::output::{self, TextFormat};
use altium_format::ops::intlib;

#[derive(Subcommand)]
pub enum IntLibCommands {
    /// Library overview with component categories and statistics
    Overview {
        /// Path to IntLib file
        path: PathBuf,

        /// Include full component details
        #[arg(long)]
        full: bool,
    },

    /// List all components in the library
    List {
        /// Path to IntLib file
        path: PathBuf,
    },

    /// Search for components by name or description
    Search {
        /// Path to IntLib file
        path: PathBuf,

        /// Search query
        query: String,

        /// Maximum results to return
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Library info and statistics
    Info {
        /// Path to IntLib file
        path: PathBuf,
    },

    /// Show detailed component information
    Component {
        /// Path to IntLib file
        path: PathBuf,

        /// Component name or index
        name: String,

        /// Show parameters
        #[arg(long)]
        params: bool,
    },

    /// Show symbol/footprint cross-references
    Crossrefs {
        /// Path to IntLib file
        path: PathBuf,

        /// Filter by footprint name
        #[arg(short, long)]
        footprint: Option<String>,
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

    /// Show BOM parameters across components
    Parameters {
        /// Path to IntLib file
        path: PathBuf,

        /// Filter by component name
        #[arg(short, long)]
        component: Option<String>,

        /// Filter by parameter keys
        #[arg(short, long)]
        keys: Option<String>,
    },

    /// Extract schematic library
    ExtractSchlib {
        /// Path to IntLib file
        path: PathBuf,

        /// Output SchLib file path
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Extract PCB library
    ExtractPcblib {
        /// Path to IntLib file
        path: PathBuf,

        /// Output PcbLib file path
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Export as JSON for LLM processing
    Json {
        /// Path to IntLib file
        path: PathBuf,
    },
}

pub fn run(cmd: &IntLibCommands, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        IntLibCommands::Overview { path, full } => {
            let result = intlib::cmd_overview(path, *full)?;
            output::print(&TextWrapper(result), format)?;
        }
        IntLibCommands::List { path } => {
            let result = intlib::cmd_list(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        IntLibCommands::Search {
            path,
            query,
            limit,
        } => {
            let result = intlib::cmd_search(path, query, *limit)?;
            output::print(&TextWrapper(result), format)?;
        }
        IntLibCommands::Info { path } => {
            let result = intlib::cmd_info(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        IntLibCommands::Component {
            path,
            name,
            params,
        } => {
            let result = intlib::cmd_component(path, name, *params)?;
            output::print(&TextWrapper(result), format)?;
        }
        IntLibCommands::Crossrefs { path, footprint } => {
            let result = intlib::cmd_crossrefs(path, footprint.as_deref())?;
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
        IntLibCommands::Parameters { path, component, keys } => {
            let result = intlib::cmd_parameters(path, component.as_deref(), keys.as_deref())?;
            output::print(&TextWrapper(result), format)?;
        }
        IntLibCommands::ExtractSchlib { path, output } => {
            let result = intlib::cmd_extract_schlib(path, output)?;
            println!("{}", result);
        }
        IntLibCommands::ExtractPcblib { path, output } => {
            let result = intlib::cmd_extract_pcblib(path, output)?;
            println!("{}", result);
        }
        IntLibCommands::Json { path } => {
            let result = intlib::cmd_overview(path, true)?;
            let json_str = serde_json::to_string_pretty(&result)?;
            println!("{}", json_str);
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

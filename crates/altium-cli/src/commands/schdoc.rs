//! Schematic document (SchDoc) commands.
//!
//! Provides CLI interface for exploring and analyzing Altium schematic documents.
//! Acts as a thin wrapper over ops::schdoc functions. Contains no business logic.

use clap::Subcommand;
use serde::Serialize;
use std::path::PathBuf;

use crate::output::{self, TextFormat};
use altium_format::ops::schdoc;

#[derive(Subcommand)]
pub enum SchDocCommands {
    /// Complete schematic overview with component categories and statistics
    Overview {
        /// Path to SchDoc file
        path: PathBuf,
    },

    /// Detailed sheet metadata and properties
    Info {
        /// Path to SchDoc file
        path: PathBuf,
    },

    /// List all placed components
    Components {
        /// Path to SchDoc file
        path: PathBuf,
    },

    /// Extract net connectivity information
    Netlist {
        /// Path to SchDoc file
        path: PathBuf,

        /// Filter nets by name pattern
        #[arg(short, long)]
        filter: Option<String>,
    },

    /// List wire primitives for routing analysis
    Wires {
        /// Path to SchDoc file
        path: PathBuf,
    },

    /// List port definitions for hierarchical designs
    Ports {
        /// Path to SchDoc file
        path: PathBuf,
    },

    /// Analyze power distribution and connections
    PowerMap {
        /// Path to SchDoc file
        path: PathBuf,
    },

    /// Export as JSON for LLM processing
    Json {
        /// Path to SchDoc file
        path: PathBuf,

        /// Include full primitive details
        #[arg(long)]
        full: bool,
    },
}

pub fn run(cmd: &SchDocCommands, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        SchDocCommands::Overview { path } => {
            let result = schdoc::cmd_overview(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Info { path } => {
            let result = schdoc::cmd_info(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Components { path } => {
            let result = schdoc::cmd_components(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Netlist { path, filter } => {
            let result = schdoc::cmd_netlist(path, filter.clone())?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Wires { path } => {
            let result = schdoc::cmd_wires(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Ports { path } => {
            let result = schdoc::cmd_ports(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::PowerMap { path } => {
            let result = schdoc::cmd_power_map(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        SchDocCommands::Json { path, full } => {
            let result = schdoc::cmd_json(path, *full)?;
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

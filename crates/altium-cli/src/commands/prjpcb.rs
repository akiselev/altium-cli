//! Project file (PrjPcb) commands.
//!
//! Provides CLI interface for exploring and analyzing Altium project files.
//! Acts as a thin wrapper over ops::prjpcb functions. Contains no business logic.

use clap::Subcommand;
use serde::Serialize;
use std::path::PathBuf;

use crate::output::{self, TextFormat};
use altium_format::ops::prjpcb;

#[derive(Subcommand)]
pub enum PrjPcbCommands {
    /// Complete project overview with document counts and statistics
    Overview {
        /// Path to PrjPcb file
        path: PathBuf,
    },

    /// Detailed project metadata and configuration
    Info {
        /// Path to PrjPcb file
        path: PathBuf,
    },

    /// List referenced documents in the project
    Documents {
        /// Path to PrjPcb file
        path: PathBuf,

        /// Filter by document type (Schematic, PCB, SchLib, PcbLib, IntLib, OutJob)
        #[arg(short = 't', long)]
        doc_type: Option<String>,
    },

    /// Aggregate BOM from all schematic sheets
    Bom {
        /// Path to PrjPcb file
        path: PathBuf,

        /// Group components by part number with quantity
        #[arg(short, long)]
        grouped: bool,
    },

    /// Validate project for missing documents and configuration issues
    Validate {
        /// Path to PrjPcb file
        path: PathBuf,
    },

    /// Export as JSON for LLM processing
    Json {
        /// Path to PrjPcb file
        path: PathBuf,
    },
}

pub fn run(cmd: &PrjPcbCommands, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        PrjPcbCommands::Overview { path } => {
            let result = prjpcb::cmd_overview(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        PrjPcbCommands::Info { path } => {
            let result = prjpcb::cmd_info(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        PrjPcbCommands::Documents { path, doc_type } => {
            let result = prjpcb::cmd_documents(path, doc_type.clone())?;
            output::print(&TextWrapper(result), format)?;
        }
        PrjPcbCommands::Bom { path, grouped } => {
            let result = prjpcb::cmd_bom(path, *grouped)?;
            output::print(&TextWrapper(result), format)?;
        }
        PrjPcbCommands::Validate { path } => {
            let result = prjpcb::cmd_validate(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        PrjPcbCommands::Json { path } => {
            let result = prjpcb::cmd_json(path)?;
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

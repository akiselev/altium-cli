//! PCB project (PrjPcb) commands.
//!
//! High-level operations for exploring and managing Altium PCB projects.

use clap::Subcommand;
use serde::Serialize;
use std::path::PathBuf;

use crate::output::{self, TextFormat};
use altium_format::ops::prjpcb;

#[derive(Subcommand)]
pub enum PrjPcbCommands {
    /// Project overview with documents and parameters
    Overview {
        /// Path to PrjPcb file
        path: PathBuf,
    },

    /// Project info and metadata
    Info {
        /// Path to PrjPcb file
        path: PathBuf,
    },

    /// List project documents
    Documents {
        /// Path to PrjPcb file
        path: PathBuf,

        /// Filter by document type
        #[arg(short = 't', long)]
        doc_type: Option<String>,
    },

    /// Create new project
    Create {
        /// Path to new PrjPcb file
        path: PathBuf,

        /// Project name
        #[arg(short, long)]
        name: Option<String>,

        /// Template project path
        #[arg(short = 't', long)]
        template: Option<PathBuf>,
    },

    /// Add document to project
    AddDocument {
        /// Path to PrjPcb file
        path: PathBuf,

        /// Document path to add
        document: String,
    },

    /// Remove document from project
    RemoveDocument {
        /// Path to PrjPcb file
        path: PathBuf,

        /// Document path to remove
        document: String,
    },

    /// Show project parameters
    Parameters {
        /// Path to PrjPcb file
        path: PathBuf,
    },

    /// Set project parameter
    SetParameter {
        /// Path to PrjPcb file
        path: PathBuf,

        /// Parameter name
        name: String,

        /// Parameter value
        value: String,
    },

    /// Remove project parameter
    RemoveParameter {
        /// Path to PrjPcb file
        path: PathBuf,

        /// Parameter name
        name: String,
    },

    /// Show netlist for project
    Netlist {
        /// Path to PrjPcb file
        path: PathBuf,
    },

    /// List all components in project
    Components {
        /// Path to PrjPcb file
        path: PathBuf,
    },

    /// Generate BOM for project
    Bom {
        /// Path to PrjPcb file
        path: PathBuf,

        /// Group by value and footprint
        #[arg(short, long)]
        grouped: bool,
    },

    /// Import design to PCB
    ImportToPcb {
        /// Path to PrjPcb file
        path: PathBuf,

        /// PCB document name (optional)
        #[arg(short, long)]
        pcb: Option<String>,

        /// Dry run without modifying files
        #[arg(long)]
        dry_run: bool,
    },

    /// Sync schematic changes to PCB
    SyncToPcb {
        /// Path to PrjPcb file
        path: PathBuf,

        /// PCB document name (optional)
        #[arg(short, long)]
        pcb: Option<String>,

        /// Dry run without modifying files
        #[arg(long)]
        dry_run: bool,
    },

    /// Show differences between schematic and PCB
    DiffSchPcb {
        /// Path to PrjPcb file
        path: PathBuf,

        /// PCB document name (optional)
        #[arg(short, long)]
        pcb: Option<String>,
    },

    /// Validate project
    Validate {
        /// Path to PrjPcb file
        path: PathBuf,

        /// Check if referenced files exist
        #[arg(long)]
        check_files: bool,
    },

    /// Export as JSON for LLM processing
    Json {
        /// Path to PrjPcb file
        path: PathBuf,

        /// Include full details
        #[arg(long)]
        full: bool,

        /// Pretty-print JSON
        #[arg(long)]
        pretty: bool,
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
        PrjPcbCommands::Create { path, name, template } => {
            let result = prjpcb::cmd_create(path, name.clone(), template.clone())?;
            println!("{}", result);
        }
        PrjPcbCommands::AddDocument { path, document } => {
            let result = prjpcb::cmd_add_document(path, document)?;
            println!("{}", result);
        }
        PrjPcbCommands::RemoveDocument { path, document } => {
            let result = prjpcb::cmd_remove_document(path, document)?;
            println!("{}", result);
        }
        PrjPcbCommands::Parameters { path } => {
            let result = prjpcb::cmd_parameters(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        PrjPcbCommands::SetParameter { path, name, value } => {
            let result = prjpcb::cmd_set_parameter(path, name, value)?;
            println!("{}", result);
        }
        PrjPcbCommands::RemoveParameter { path, name } => {
            let result = prjpcb::cmd_remove_parameter(path, name)?;
            println!("{}", result);
        }
        PrjPcbCommands::Netlist { path } => {
            let result = prjpcb::cmd_netlist(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        PrjPcbCommands::Components { path } => {
            let result = prjpcb::cmd_components(path)?;
            output::print(&TextWrapper(result), format)?;
        }
        PrjPcbCommands::Bom { path, grouped } => {
            let result = prjpcb::cmd_bom(path, *grouped)?;
            output::print(&TextWrapper(result), format)?;
        }
        PrjPcbCommands::ImportToPcb { path, pcb, dry_run } => {
            let result = prjpcb::cmd_import_to_pcb(path, pcb.clone(), *dry_run)?;
            println!("{}", result);
        }
        PrjPcbCommands::SyncToPcb { path, pcb, dry_run } => {
            let result = prjpcb::cmd_sync_to_pcb(path, pcb.clone(), *dry_run)?;
            println!("{}", result);
        }
        PrjPcbCommands::DiffSchPcb { path, pcb } => {
            let result = prjpcb::cmd_diff_sch_pcb(path, pcb.clone())?;
            output::print(&TextWrapper(result), format)?;
        }
        PrjPcbCommands::Validate { path, check_files } => {
            let result = prjpcb::cmd_validate(path, *check_files)?;
            output::print(&TextWrapper(result), format)?;
        }
        PrjPcbCommands::Json { path, full, pretty } => {
            let result = prjpcb::cmd_json(path, *full, *pretty)?;
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

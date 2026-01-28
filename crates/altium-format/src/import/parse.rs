// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! File format detection and parsing for import files.

use std::path::Path;

use super::types::ImportFile;

/// Supported import file formats.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImportFormat {
    Yaml,
    Json,
    Toml,
}

/// Detect format from file extension.
pub fn detect_format(path: &Path) -> Result<ImportFormat, Box<dyn std::error::Error>> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("yml" | "yaml") => Ok(ImportFormat::Yaml),
        Some("json") => Ok(ImportFormat::Json),
        Some("toml") => Ok(ImportFormat::Toml),
        Some(ext) => Err(format!(
            "Unknown import file extension '.{}'. Use .yml, .yaml, .json, or .toml",
            ext
        )
        .into()),
        None => Err("Import file must have an extension (.yml, .yaml, .json, or .toml)".into()),
    }
}

/// Parse an import file from disk, auto-detecting format from extension.
pub fn parse_import_file(path: &Path) -> Result<ImportFile, Box<dyn std::error::Error>> {
    let format = detect_format(path)?;
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Error reading import file '{}': {}", path.display(), e))?;
    parse_import_string(&content, format)
}

/// Parse an import file from a string with explicit format.
pub fn parse_import_string(
    content: &str,
    format: ImportFormat,
) -> Result<ImportFile, Box<dyn std::error::Error>> {
    match format {
        ImportFormat::Yaml => {
            serde_yaml::from_str(content).map_err(|e| format!("YAML parse error: {}", e).into())
        }
        ImportFormat::Json => {
            serde_json::from_str(content).map_err(|e| format!("JSON parse error: {}", e).into())
        }
        ImportFormat::Toml => {
            toml::from_str(content).map_err(|e| format!("TOML parse error: {}", e).into())
        }
    }
}

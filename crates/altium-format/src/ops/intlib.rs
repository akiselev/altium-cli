// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Integrated library operations.
//!
//! Provides high-level operations for exploring and extracting content from Altium
//! integrated library (.IntLib) files. IntLib files are CFB containers that embed
//! both schematic symbols (SchLib) and PCB footprints (PcbLib) along with component
//! cross-reference data.
//!
//! # CFB Storage Hierarchy (IntLib)
//!
//! ```text
//! Root
//! ├── FileHeader                (version, library metadata)
//! ├── ComponentInfoFile         (component cross-references: name, description, footprint)
//! ├── {SchLibStream}            (embedded SchLib data as raw CFB or stream)
//! ├── {PcbLibStream}            (embedded PcbLib data as raw CFB or stream)
//! └── SectionKeys               (name mappings)
//! ```
//!
//! IntLib files bundle a source schematic library with its linked PCB library
//! into a single portable file. The component cross-reference table maps
//! component names to their corresponding symbols and footprints.

// cmd_* functions mix presentation and business logic; separation punted until
// usage patterns clarify abstraction boundaries (premature abstraction risk)

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, Write};
use std::path::Path;

use crate::ops::output::*;
use crate::v2::io::schlib::SchLibV2;
use crate::v2::pcb::io::pcblib::PcbLib;

// ═══════════════════════════════════════════════════════════════════════════
// INTLIB STRUCTURE
// ═══════════════════════════════════════════════════════════════════════════

/// Represents a parsed IntLib file with embedded libraries and cross-references.
#[derive(Debug, Default)]
pub struct IntLib {
    /// IntLib file path.
    pub path: String,
    /// Library version from FileHeader.
    pub version: u32,
    /// Component cross-references (component name -> footprint mapping).
    pub cross_refs: Vec<IntLibCrossRef>,
    /// Embedded schematic library (lazy-loaded).
    pub schlib: Option<SchLibV2>,
    /// Embedded PCB library (lazy-loaded).
    pub pcblib: Option<PcbLib>,
    /// Raw SchLib stream data for extraction.
    pub raw_schlib: Option<Vec<u8>>,
    /// Raw PcbLib stream data for extraction.
    pub raw_pcblib: Option<Vec<u8>>,
    /// Parameter sets indexed by component name.
    pub parameter_sets: HashMap<String, HashMap<String, String>>,
}

/// Component cross-reference entry from IntLib.
#[derive(Debug, Clone, Default)]
pub struct IntLibCrossRef {
    /// Component name (LibRef).
    pub name: String,
    /// Component description.
    pub description: String,
    /// Linked footprint name.
    pub footprint: String,
    /// Path to schematic symbol within embedded SchLib.
    pub schlib_path: String,
    /// Path to footprint within embedded PcbLib.
    pub pcblib_path: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Sorts strings with embedded numbers naturally (e.g., "A2" < "A10").
///
/// TODO: Consolidate with pcblib/schlib/prjpcb::alphanumeric_sort after all 4 ops modules exist
/// to validate the pattern before abstracting.
fn alphanumeric_sort(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(&ac), Some(&bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    // Extract and compare numbers
                    let a_num: String = a_chars
                        .by_ref()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    let b_num: String = b_chars
                        .by_ref()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    let a_val: u64 = a_num.parse().unwrap_or(0);
                    let b_val: u64 = b_num.parse().unwrap_or(0);
                    match a_val.cmp(&b_val) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    a_chars.next();
                    b_chars.next();
                    match ac.cmp(&bc) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                }
            }
        }
    }
}

/// Read a stream from a CFB compound file.
fn read_cfb_stream<R: Read + Seek>(
    cfb: &mut cfb::CompoundFile<R>,
    path: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut stream = cfb
        .open_stream(path)
        .map_err(|e| format!("Failed to open stream '{}': {}", path, e))?;
    let mut data = Vec::new();
    stream.read_to_end(&mut data)?;
    Ok(data)
}

/// Check if a storage or stream exists in the CFB.
fn cfb_entry_exists<R: Read + Seek>(cfb: &mut cfb::CompoundFile<R>, path: &str) -> bool {
    cfb.exists(path)
}

/// Parse a pipe-delimited parameter string into a HashMap.
/// Format: "|KEY1=VALUE1|KEY2=VALUE2|..."
fn parse_parameters(data: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for part in data.split('|') {
        if let Some((key, value)) = part.split_once('=') {
            params.insert(key.to_string(), value.to_string());
        }
    }
    params
}

/// Parse the ComponentInfoFile stream to extract cross-references.
/// The format is ASCII text with pipe-delimited parameters.
fn parse_component_info(data: &[u8]) -> Vec<IntLibCrossRef> {
    let mut cross_refs = Vec::new();

    // Try to parse as UTF-8 text
    let text = match String::from_utf8(data.to_vec()) {
        Ok(s) => s,
        Err(_) => {
            // Try lossy conversion
            String::from_utf8_lossy(data).to_string()
        }
    };

    // Split by record separator (typically newlines or null bytes)
    for line in text.split(&['\n', '\r', '\0'][..]) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let params = parse_parameters(line);

        // Look for component entries with RECORD type
        if let Some(record_type) = params.get("RECORD") {
            // Component records typically have LibRef and Footprint info
            if record_type == "Component" || params.contains_key("LIBREF") {
                let name = params
                    .get("LIBREF")
                    .or_else(|| params.get("NAME"))
                    .cloned()
                    .unwrap_or_default();

                if !name.is_empty() {
                    cross_refs.push(IntLibCrossRef {
                        name,
                        description: params.get("DESCRIPTION").cloned().unwrap_or_default(),
                        footprint: params.get("FOOTPRINT").cloned().unwrap_or_default(),
                        schlib_path: params.get("SCHLIB").cloned().unwrap_or_default(),
                        pcblib_path: params.get("PCBLIB").cloned().unwrap_or_default(),
                    });
                }
            }
        } else if params.contains_key("LIBREF") || params.contains_key("NAME") {
            // Fallback: entry without RECORD type but with component info
            let name = params
                .get("LIBREF")
                .or_else(|| params.get("NAME"))
                .cloned()
                .unwrap_or_default();

            if !name.is_empty() {
                cross_refs.push(IntLibCrossRef {
                    name,
                    description: params.get("DESCRIPTION").cloned().unwrap_or_default(),
                    footprint: params
                        .get("FOOTPRINT")
                        .or_else(|| params.get("CURRENTFOOTPRINT"))
                        .cloned()
                        .unwrap_or_default(),
                    schlib_path: params.get("SCHLIB").cloned().unwrap_or_default(),
                    pcblib_path: params.get("PCBLIB").cloned().unwrap_or_default(),
                });
            }
        }
    }

    cross_refs
}

/// Parse the FileHeader stream to extract library version and metadata.
fn parse_file_header(data: &[u8]) -> (u32, HashMap<String, String>) {
    let mut version = 0u32;
    let mut metadata = HashMap::new();

    // Try text-based parsing first
    if let Ok(text) = String::from_utf8(data.to_vec()) {
        let params = parse_parameters(&text);
        if let Some(v) = params.get("VERSION") {
            version = v.parse().unwrap_or(0);
        }
        metadata = params;
    }
    // If binary, try to extract version from first 4 bytes
    else if data.len() >= 4 {
        version = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    }

    (version, metadata)
}

/// Opens and parses an IntLib file from the given path.
///
/// IntLib files are CFB compound files that contain:
/// - Component cross-reference data (ComponentInfoFile or similar)
/// - Embedded SchLib data (schematic symbols)
/// - Embedded PcbLib data (PCB footprints)
///
/// This function performs a complete parse including embedded libraries.
pub fn open_intlib(path: &Path) -> Result<IntLib, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut cfb = cfb::CompoundFile::open(BufReader::new(file))
        .map_err(|e| format!("Failed to open CFB file: {}", e))?;

    let mut intlib = IntLib {
        path: path.display().to_string(),
        ..Default::default()
    };

    // Explore the CFB structure to understand what's available
    let _entries: Vec<(String, bool)> = cfb
        .walk()
        .map(|e| {
            (
                e.path().to_string_lossy().replace('\\', "/"),
                e.is_stream(),
            )
        })
        .collect();

    // Parse FileHeader if present
    if cfb_entry_exists(&mut cfb, "/FileHeader") {
        if let Ok(data) = read_cfb_stream(&mut cfb, "/FileHeader") {
            let (version, _metadata) = parse_file_header(&data);
            intlib.version = version;
        }
    }

    // Look for component cross-reference data
    // IntLib files typically store this in ComponentInfoFile or similar streams
    let info_stream_candidates = [
        "/ComponentInfoFile",
        "/ComponentInfo",
        "/CrossRef",
        "/Library/ComponentInfo",
        "/Library/Data",
    ];

    for candidate in &info_stream_candidates {
        if cfb_entry_exists(&mut cfb, candidate) {
            if let Ok(data) = read_cfb_stream(&mut cfb, candidate) {
                let refs = parse_component_info(&data);
                if !refs.is_empty() {
                    intlib.cross_refs = refs;
                    break;
                }
            }
        }
    }

    // If no cross-refs found from dedicated streams, try to build from embedded libraries
    if intlib.cross_refs.is_empty() {
        // Will be populated after parsing embedded libraries
    }

    // Look for embedded SchLib data
    let schlib_candidates = [
        "/SchLib",
        "/SchematicLib",
        "/Schematic",
        "/Library/SchLib",
        "/EmbeddedSchLib",
    ];

    for candidate in &schlib_candidates {
        if cfb_entry_exists(&mut cfb, candidate) {
            if let Ok(data) = read_cfb_stream(&mut cfb, candidate) {
                intlib.raw_schlib = Some(data.clone());
                // Try to parse as SchLib
                if let Ok(schlib) = SchLibV2::open(Cursor::new(data)) {
                    intlib.schlib = Some(schlib);
                    break;
                }
            }
        }
    }

    // Look for embedded PcbLib data
    let pcblib_candidates = [
        "/PcbLib",
        "/PCBLib",
        "/FootprintLib",
        "/Library/PcbLib",
        "/EmbeddedPcbLib",
    ];

    for candidate in &pcblib_candidates {
        if cfb_entry_exists(&mut cfb, candidate) {
            if let Ok(data) = read_cfb_stream(&mut cfb, candidate) {
                intlib.raw_pcblib = Some(data.clone());
                // Try to parse as PcbLib
                if let Ok(pcblib) = PcbLib::open(Cursor::new(data)) {
                    intlib.pcblib = Some(pcblib);
                    break;
                }
            }
        }
    }

    // If embedded libraries weren't found as separate streams, check if the IntLib
    // itself contains SchLib-style or PcbLib-style structures directly
    if intlib.schlib.is_none() {
        // Check if the IntLib has SchLib-style FileHeader with component entries
        if cfb_entry_exists(&mut cfb, "/FileHeader") && cfb_entry_exists(&mut cfb, "/SectionKeys")
        {
            // Try to re-open the file as a SchLib (IntLib may BE a SchLib with extra data)
            drop(cfb);
            let file = File::open(path)?;
            if let Ok(schlib) = SchLibV2::open(BufReader::new(file)) {
                // Build cross-refs from SchLib components if we don't have them
                if intlib.cross_refs.is_empty() {
                    for comp in &schlib.components {
                        // Look for CURRENTFOOTPRINT parameter in component
                        let footprint = comp
                            .records
                            .iter()
                            .find_map(|r| {
                                if r.params.contains("CURRENTFOOTPRINT") {
                                    let params = parse_parameters(&r.params);
                                    params.get("CURRENTFOOTPRINT").cloned()
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_default();

                        intlib.cross_refs.push(IntLibCrossRef {
                            name: comp.entry.lib_ref.clone(),
                            description: comp.entry.description.clone(),
                            footprint,
                            schlib_path: String::new(),
                            pcblib_path: String::new(),
                        });
                    }
                }
                intlib.schlib = Some(schlib);
            }
        }
    }

    // Sort cross-refs by name
    intlib
        .cross_refs
        .sort_by(|a, b| alphanumeric_sort(&a.name, &b.name));

    Ok(intlib)
}

/// Extract the embedded SchLib from an IntLib.
///
/// Returns the parsed SchLibV2 if available.
pub fn extract_embedded_schlib(intlib: &IntLib) -> Option<&SchLibV2> {
    intlib.schlib.as_ref()
}

/// Extract the embedded PcbLib from an IntLib.
///
/// Returns the parsed PcbLib if available.
pub fn extract_embedded_pcblib(intlib: &IntLib) -> Option<&PcbLib> {
    intlib.pcblib.as_ref()
}

// ═══════════════════════════════════════════════════════════════════════════
// BROWSE COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Returns library overview with statistics for both embedded libraries.
///
/// Provides a high-level summary including:
/// - Component count and cross-references
/// - Schematic symbol count and details
/// - PCB footprint count and details
/// - Parameter set count
pub fn cmd_overview(path: &Path) -> Result<IntLibOverview, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    // Build component list from cross-refs
    let mut component_list: Vec<ComponentCrossRef> = intlib
        .cross_refs
        .iter()
        .map(|cr| ComponentCrossRef {
            name: cr.name.clone(),
            description: cr.description.clone(),
            footprint: cr.footprint.clone(),
        })
        .collect();

    component_list.sort_by(|a, b| alphanumeric_sort(&a.name, &b.name));

    // Count footprint usage
    let mut footprint_usage: HashMap<String, usize> = HashMap::new();
    for cr in &intlib.cross_refs {
        if !cr.footprint.is_empty() {
            *footprint_usage.entry(cr.footprint.clone()).or_insert(0) += 1;
        }
    }
    let mut footprint_usage_vec: Vec<_> = footprint_usage.into_iter().collect();
    footprint_usage_vec.sort_by(|a, b| b.1.cmp(&a.1));

    // Get schematic symbol count
    let schematic_symbol_count = intlib
        .schlib
        .as_ref()
        .map(|s| s.components.len())
        .unwrap_or(0);

    // Get PCB footprint count
    let pcb_footprint_count = intlib
        .pcblib
        .as_ref()
        .map(|p| p.footprints.len())
        .unwrap_or(0);

    Ok(IntLibOverview {
        path: path.display().to_string(),
        version: intlib.version,
        component_count: intlib.cross_refs.len(),
        schematic_symbol_count,
        pcb_footprint_count,
        parameter_set_count: intlib.parameter_sets.len(),
        footprint_usage: footprint_usage_vec,
        component_list,
        symbols: None,
        footprints: None,
        parameters: None,
    })
}

/// Lists all components in the library.
///
/// Returns the component cross-reference table showing:
/// - Component name
/// - Description
/// - Linked footprint
pub fn cmd_list(path: &Path) -> Result<IntLibComponentList, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    let mut components: Vec<ComponentCrossRef> = intlib
        .cross_refs
        .iter()
        .map(|cr| ComponentCrossRef {
            name: cr.name.clone(),
            description: cr.description.clone(),
            footprint: cr.footprint.clone(),
        })
        .collect();

    components.sort_by(|a, b| alphanumeric_sort(&a.name, &b.name));

    Ok(IntLibComponentList { components })
}

/// Searches for components matching the query in name, description, or footprint.
///
/// Returns results up to the optional limit, sorted by relevance.
pub fn cmd_search(
    path: &Path,
    query: &str,
    limit: Option<usize>,
) -> Result<IntLibSearchResults, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    let query_lower = query.to_lowercase();
    let has_wildcard = query.contains('*');

    let mut matches: Vec<ComponentCrossRef> = intlib
        .cross_refs
        .iter()
        .filter(|cr| {
            let name = cr.name.to_lowercase();
            let desc = cr.description.to_lowercase();
            let footprint = cr.footprint.to_lowercase();

            if has_wildcard {
                let pattern = query_lower.replace('*', "");
                name.contains(&pattern)
                    || desc.contains(&pattern)
                    || footprint.contains(&pattern)
            } else {
                name.contains(&query_lower)
                    || desc.contains(&query_lower)
                    || footprint.contains(&query_lower)
            }
        })
        .map(|cr| ComponentCrossRef {
            name: cr.name.clone(),
            description: cr.description.clone(),
            footprint: cr.footprint.clone(),
        })
        .collect();

    // Sort by relevance (exact name match first)
    matches.sort_by(|a, b| {
        let a_exact = a.name.to_lowercase() == query_lower;
        let b_exact = b.name.to_lowercase() == query_lower;
        match (a_exact, b_exact) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => alphanumeric_sort(&a.name, &b.name),
        }
    });

    let total_matches = matches.len();

    // Apply limit
    if let Some(max) = limit {
        matches.truncate(max);
    }

    Ok(IntLibSearchResults {
        query: query.to_string(),
        total_matches,
        results: matches,
    })
}

/// Returns detailed information about a single component.
///
/// Includes the component's symbol info, footprint info, and parameters
/// from both the schematic and PCB libraries.
pub fn cmd_component(
    path: &Path,
    name: &str,
) -> Result<IntLibComponentDetail, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    let name_lower = name.to_lowercase();

    // Find cross-ref entry
    let cross_ref = intlib
        .cross_refs
        .iter()
        .find(|cr| cr.name.to_lowercase() == name_lower)
        .ok_or_else(|| format!("Component '{}' not found", name))?;

    // Get symbol info from embedded SchLib
    let symbol_info = intlib.schlib.as_ref().and_then(|schlib| {
        schlib
            .components
            .iter()
            .find(|c| c.entry.lib_ref.to_lowercase() == name_lower)
            .map(|c| SymbolInfo {
                pin_count: c.pins().count(),
                primitive_count: c.typed_records.len(),
            })
    });

    // Get footprint info from embedded PcbLib
    let footprint_info = intlib.pcblib.as_ref().and_then(|pcblib| {
        let fp_name_lower = cross_ref.footprint.to_lowercase();
        pcblib
            .footprints
            .iter()
            .find(|f| f.name.to_lowercase() == fp_name_lower)
            .map(|f| {
                let primitive_count = f.tracks.len()
                    + f.arcs.len()
                    + f.fills.len()
                    + f.pads.len()
                    + f.vias.len()
                    + f.texts.len()
                    + f.regions.len()
                    + f.component_bodies.len();
                FootprintInfo {
                    pad_count: f.pads.len(),
                    primitive_count,
                }
            })
    });

    // Get parameters
    let parameters = intlib.parameter_sets.get(&cross_ref.name).cloned();

    Ok(IntLibComponentDetail {
        name: cross_ref.name.clone(),
        description: cross_ref.description.clone(),
        footprint: cross_ref.footprint.clone(),
        schlib_path: cross_ref.schlib_path.clone(),
        pcblib_path: cross_ref.pcblib_path.clone(),
        symbol_info,
        footprint_info,
        parameters,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// EXTRACTION COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Extracts the embedded SchLib to a standalone file.
///
/// If the IntLib contains raw SchLib data, it is written directly.
/// Otherwise, if a parsed SchLib is available, it is serialized.
pub fn cmd_extract_schlib(path: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    // Prefer raw data for lossless extraction
    if let Some(raw_data) = &intlib.raw_schlib {
        let mut file = File::create(output)?;
        file.write_all(raw_data)?;
        println!(
            "Extracted SchLib ({} bytes) to {}",
            raw_data.len(),
            output.display()
        );
        return Ok(());
    }

    // Fall back to serializing parsed SchLib
    if let Some(schlib) = &intlib.schlib {
        let mut buffer = Cursor::new(Vec::new());
        schlib.write(&mut buffer).map_err(|e| e.to_string())?;
        let mut file = File::create(output)?;
        file.write_all(buffer.get_ref())?;
        println!(
            "Extracted SchLib ({} components) to {}",
            schlib.components.len(),
            output.display()
        );
        return Ok(());
    }

    Err("No embedded SchLib found in IntLib".into())
}

/// Extracts the embedded PcbLib to a standalone file.
///
/// If the IntLib contains raw PcbLib data, it is written directly.
/// Otherwise, if a parsed PcbLib is available, it is serialized.
pub fn cmd_extract_pcblib(path: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    // Prefer raw data for lossless extraction
    if let Some(raw_data) = &intlib.raw_pcblib {
        let mut file = File::create(output)?;
        file.write_all(raw_data)?;
        println!(
            "Extracted PcbLib ({} bytes) to {}",
            raw_data.len(),
            output.display()
        );
        return Ok(());
    }

    // Fall back to serializing parsed PcbLib
    if let Some(pcblib) = &intlib.pcblib {
        let mut buffer = Cursor::new(Vec::new());
        pcblib.write(&mut buffer)?;
        let mut file = File::create(output)?;
        file.write_all(buffer.get_ref())?;
        println!(
            "Extracted PcbLib ({} footprints) to {}",
            pcblib.footprints.len(),
            output.display()
        );
        return Ok(());
    }

    Err("No embedded PcbLib found in IntLib".into())
}

// ═══════════════════════════════════════════════════════════════════════════
// EXPORT COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Serializes the IntLib structure to JSON for LLM processing or external analysis.
///
/// When `full` is true, includes detailed symbol and footprint information.
pub fn cmd_json(
    path: &Path,
    full: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    // Build component list
    let components: Vec<serde_json::Value> = intlib
        .cross_refs
        .iter()
        .map(|cr| {
            serde_json::json!({
                "name": cr.name,
                "description": cr.description,
                "footprint": cr.footprint,
            })
        })
        .collect();

    let mut result = serde_json::json!({
        "file": path.display().to_string(),
        "version": intlib.version,
        "component_count": intlib.cross_refs.len(),
        "schematic_symbol_count": intlib.schlib.as_ref().map(|s| s.components.len()).unwrap_or(0),
        "pcb_footprint_count": intlib.pcblib.as_ref().map(|p| p.footprints.len()).unwrap_or(0),
        "components": components,
    });

    if full {
        // Add symbol details
        if let Some(schlib) = &intlib.schlib {
            let symbols: Vec<serde_json::Value> = schlib
                .components
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "name": c.entry.lib_ref,
                        "description": c.entry.description,
                        "pin_count": c.pins().count(),
                        "part_count": c.entry.part_count,
                        "primitive_count": c.typed_records.len(),
                    })
                })
                .collect();
            result["symbols"] = serde_json::json!(symbols);
        }

        // Add footprint details
        if let Some(pcblib) = &intlib.pcblib {
            let footprints: Vec<serde_json::Value> = pcblib
                .footprints
                .iter()
                .map(|f| {
                    serde_json::json!({
                        "name": f.name,
                        "pad_count": f.pads.len(),
                        "primitive_count": f.tracks.len() + f.arcs.len() + f.fills.len()
                            + f.pads.len() + f.vias.len() + f.texts.len()
                            + f.regions.len() + f.component_bodies.len(),
                    })
                })
                .collect();
            result["footprints"] = serde_json::json!(footprints);
        }

        // Add parameters
        if !intlib.parameter_sets.is_empty() {
            result["parameter_sets"] = serde_json::to_value(&intlib.parameter_sets)?;
        }
    }

    Ok(result)
}

/// Returns library info with statistics.
pub fn cmd_info(path: &Path) -> Result<IntLibInfo, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    Ok(IntLibInfo {
        path: path.display().to_string(),
        version: intlib.version,
        cross_ref_count: intlib.cross_refs.len(),
        schematic_symbol_count: intlib
            .schlib
            .as_ref()
            .map(|s| s.components.len())
            .unwrap_or(0),
        pcb_footprint_count: intlib
            .pcblib
            .as_ref()
            .map(|p| p.footprints.len())
            .unwrap_or(0),
        parameter_set_count: intlib.parameter_sets.len(),
    })
}

/// Lists embedded schematic symbols.
pub fn cmd_symbols(path: &Path) -> Result<IntLibSymbolList, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    let symbols = intlib
        .schlib
        .as_ref()
        .map(|schlib| {
            schlib
                .components
                .iter()
                .map(|c| SymbolSummary {
                    name: c.entry.lib_ref.clone(),
                    description: c.entry.description.clone(),
                    pin_count: c.pins().count(),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(IntLibSymbolList { symbols })
}

/// Lists embedded PCB footprints.
pub fn cmd_footprints(path: &Path) -> Result<IntLibFootprintList, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    let footprints = intlib
        .pcblib
        .as_ref()
        .map(|pcblib| {
            pcblib
                .footprints
                .iter()
                .map(|f| FootprintSummary {
                    name: f.name.clone(),
                    description: f
                        .parameters
                        .get("DESCRIPTION")
                        .cloned()
                        .unwrap_or_default(),
                    pad_count: f.pads.len(),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(IntLibFootprintList { footprints })
}

/// Lists component parameters.
pub fn cmd_parameters(
    path: &Path,
    component: Option<String>,
) -> Result<IntLibParameterList, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    let filter_lower = component.as_ref().map(|s| s.to_lowercase());

    // Build parameter list from embedded SchLib if available
    let mut parameters = Vec::new();

    if let Some(schlib) = &intlib.schlib {
        for comp in &schlib.components {
            if let Some(ref filter) = filter_lower {
                if comp.entry.lib_ref.to_lowercase() != *filter {
                    continue;
                }
            }

            let mut params = HashMap::new();

            // Extract parameters from typed records
            for record in &comp.typed_records {
                if let crate::v2::fields::TypedRecord::Parameter(p) = record {
                    params.insert(p.name.clone(), p.text.clone());
                }
            }

            // Also extract from raw records
            for record in &comp.records {
                let record_params = parse_parameters(&record.params);
                if record_params.get("RECORD").map(|r| r == "41").unwrap_or(false)
                    || record_params.contains_key("NAME")
                {
                    if let (Some(name), Some(value)) =
                        (record_params.get("NAME"), record_params.get("TEXT"))
                    {
                        params.insert(name.clone(), value.clone());
                    }
                }
            }

            if !params.is_empty() {
                parameters.push(ComponentParameters {
                    component_name: comp.entry.lib_ref.clone(),
                    params,
                });
            }
        }
    }

    // Also include parameters from parameter_sets
    for (comp_name, params) in &intlib.parameter_sets {
        if let Some(ref filter) = filter_lower {
            if comp_name.to_lowercase() != *filter {
                continue;
            }
        }

        // Check if we already have this component
        if !parameters.iter().any(|p| p.component_name == *comp_name) {
            parameters.push(ComponentParameters {
                component_name: comp_name.clone(),
                params: params.clone(),
            });
        }
    }

    parameters.sort_by(|a, b| alphanumeric_sort(&a.component_name, &b.component_name));

    Ok(IntLibParameterList { parameters })
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alphanumeric_sort() {
        let mut items = vec!["A10", "A2", "A1", "B1"];
        items.sort_by(|a, b| alphanumeric_sort(a, b));
        assert_eq!(items, vec!["A1", "A2", "A10", "B1"]);
    }

    #[test]
    fn test_alphanumeric_sort_mixed() {
        let mut items = vec!["LM358", "LM7805", "LM393", "LM2596"];
        items.sort_by(|a, b| alphanumeric_sort(a, b));
        assert_eq!(items, vec!["LM358", "LM393", "LM2596", "LM7805"]);
    }

    #[test]
    fn test_parse_parameters() {
        let params = parse_parameters("|NAME=Test|VALUE=123|DESCRIPTION=A component|");
        assert_eq!(params.get("NAME"), Some(&"Test".to_string()));
        assert_eq!(params.get("VALUE"), Some(&"123".to_string()));
        assert_eq!(
            params.get("DESCRIPTION"),
            Some(&"A component".to_string())
        );
    }

    #[test]
    fn test_parse_parameters_empty() {
        let params = parse_parameters("");
        assert!(params.is_empty());
    }

    #[test]
    fn test_parse_component_info_basic() {
        let data = b"|RECORD=Component|LIBREF=LM358|DESCRIPTION=Op Amp|FOOTPRINT=SOIC-8|";
        let refs = parse_component_info(data);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].name, "LM358");
        assert_eq!(refs[0].description, "Op Amp");
        assert_eq!(refs[0].footprint, "SOIC-8");
    }

    #[test]
    fn test_parse_component_info_multiple() {
        let data = b"|LIBREF=LM358|FOOTPRINT=SOIC-8|\n|LIBREF=LM7805|FOOTPRINT=TO-220|";
        let refs = parse_component_info(data);
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn test_cross_ref_default() {
        let cr = IntLibCrossRef::default();
        assert!(cr.name.is_empty());
        assert!(cr.footprint.is_empty());
    }
}

// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Integrated library operations.
//!
//! High-level operations for exploring Altium integrated library (.IntLib) files.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::io::IntLib;
use crate::ops::output::*;

fn open_intlib(path: &Path) -> Result<IntLib, Box<dyn std::error::Error>> {
    let file = BufReader::new(File::open(path)?);
    let intlib = IntLib::open(file)?;
    Ok(intlib)
}

pub fn cmd_overview(path: &Path, full: bool) -> Result<IntLibOverview, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    // Footprint summary
    let mut footprint_counts: HashMap<String, usize> = HashMap::new();
    for xref in &intlib.cross_refs {
        *footprint_counts.entry(xref.footprint.clone()).or_insert(0) += 1;
    }

    let mut footprint_usage: Vec<_> = footprint_counts.into_iter().collect();
    footprint_usage.sort_by(|a, b| b.1.cmp(&a.1));

    let component_list: Vec<ComponentCrossRef> = intlib
        .cross_refs
        .iter()
        .map(|xref| ComponentCrossRef {
            name: xref.name.clone(),
            description: xref.description.clone(),
            footprint: xref.footprint.clone(),
        })
        .collect();

    // If full details requested, include symbols, footprints, and parameters
    let (symbols, footprints, parameters) = if full {
        let symbols: Vec<SymbolSummary> = intlib
            .schlib
            .iter()
            .map(|comp| SymbolSummary {
                name: comp.name().to_string(),
                description: comp.description().to_string(),
                pin_count: comp.pin_count(),
            })
            .collect();

        let footprints: Vec<FootprintSummary> = intlib
            .pcblib
            .iter()
            .map(|comp| FootprintSummary {
                name: comp.pattern.clone(),
                description: comp.description.clone(),
                pad_count: comp.pad_count(),
            })
            .collect();

        let parameters: Vec<ComponentParameters> = intlib
            .parameters
            .iter()
            .map(|p| {
                let params = p
                    .params
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.as_str().to_string()))
                    .collect();
                ComponentParameters {
                    component_name: p.name.clone(),
                    params,
                }
            })
            .collect();

        (Some(symbols), Some(footprints), Some(parameters))
    } else {
        (None, None, None)
    };

    Ok(IntLibOverview {
        path: path.display().to_string(),
        version: intlib.version,
        component_count: intlib.cross_refs.len(),
        schematic_symbol_count: intlib.schematic_component_count(),
        pcb_footprint_count: intlib.footprint_count(),
        parameter_set_count: intlib.parameters.len(),
        footprint_usage,
        component_list,
        symbols,
        footprints,
        parameters,
    })
}

pub fn cmd_list(path: &Path) -> Result<IntLibComponentList, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    let components: Vec<ComponentCrossRef> = intlib
        .cross_refs
        .iter()
        .map(|xref| ComponentCrossRef {
            name: xref.name.clone(),
            description: xref.description.clone(),
            footprint: xref.footprint.clone(),
        })
        .collect();

    Ok(IntLibComponentList { components })
}

pub fn cmd_search(
    path: &Path,
    query: &str,
    limit: Option<usize>,
) -> Result<IntLibSearchResults, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;
    let query_lower = query.to_lowercase();

    let matches: Vec<ComponentCrossRef> = intlib
        .cross_refs
        .iter()
        .filter(|xref| {
            xref.name.to_lowercase().contains(&query_lower)
                || xref.description.to_lowercase().contains(&query_lower)
                || xref.footprint.to_lowercase().contains(&query_lower)
        })
        .map(|xref| ComponentCrossRef {
            name: xref.name.clone(),
            description: xref.description.clone(),
            footprint: xref.footprint.clone(),
        })
        .collect();

    let total_matches = matches.len();
    let results = if let Some(limit) = limit {
        matches.into_iter().take(limit).collect()
    } else {
        matches
    };

    Ok(IntLibSearchResults {
        query: query.to_string(),
        total_matches,
        results,
    })
}

pub fn cmd_info(path: &Path) -> Result<IntLibInfo, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    Ok(IntLibInfo {
        path: path.display().to_string(),
        version: intlib.version,
        cross_ref_count: intlib.cross_refs.len(),
        schematic_symbol_count: intlib.schematic_component_count(),
        pcb_footprint_count: intlib.footprint_count(),
        parameter_set_count: intlib.parameters.len(),
    })
}

pub fn cmd_component(
    path: &Path,
    name: &str,
    show_params: bool,
) -> Result<IntLibComponentDetail, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    // Find cross-reference
    let xref = intlib
        .cross_refs
        .iter()
        .find(|x| x.name == name)
        .ok_or_else(|| format!("Component '{}' not found", name))?;

    // Find and show schematic symbol info
    let symbol_info = intlib
        .schlib
        .iter()
        .find(|c| c.name() == name)
        .map(|comp| SymbolInfo {
            pin_count: comp.pin_count(),
            primitive_count: comp.primitive_count(),
        });

    // Find and show footprint info
    let footprint_info = intlib
        .pcblib
        .iter()
        .find(|c| c.pattern == xref.footprint)
        .map(|comp| FootprintInfo {
            pad_count: comp.pad_count(),
            primitive_count: comp.primitives.len(),
        });

    let parameters = if show_params {
        intlib
            .parameters
            .iter()
            .find(|p| p.name == name)
            .map(|params| {
                params
                    .params
                    .iter()
                    .map(|(key, value)| (key.to_string(), value.as_str().to_string()))
                    .collect()
            })
    } else {
        None
    };

    Ok(IntLibComponentDetail {
        name: xref.name.clone(),
        description: xref.description.clone(),
        footprint: xref.footprint.clone(),
        schlib_path: xref.schlib_path.clone(),
        pcblib_path: xref.pcblib_path.clone(),
        symbol_info,
        footprint_info,
        parameters,
    })
}

pub fn cmd_crossrefs(
    path: &Path,
    footprint_filter: Option<&str>,
) -> Result<IntLibComponentList, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    let refs: Vec<ComponentCrossRef> = intlib
        .cross_refs
        .iter()
        .filter(|x| {
            footprint_filter
                .is_none_or(|filter| x.footprint.to_lowercase().contains(&filter.to_lowercase()))
        })
        .map(|xref| ComponentCrossRef {
            name: xref.name.clone(),
            description: xref.description.clone(),
            footprint: xref.footprint.clone(),
        })
        .collect();

    Ok(IntLibComponentList { components: refs })
}

pub fn cmd_symbols(path: &Path) -> Result<IntLibSymbolList, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    let symbols: Vec<SymbolSummary> = intlib
        .schlib
        .iter()
        .map(|comp| SymbolSummary {
            name: comp.name().to_string(),
            description: comp.description().to_string(),
            pin_count: comp.pin_count(),
        })
        .collect();

    Ok(IntLibSymbolList { symbols })
}

pub fn cmd_footprints(path: &Path) -> Result<IntLibFootprintList, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    let footprints: Vec<FootprintSummary> = intlib
        .pcblib
        .iter()
        .map(|comp| FootprintSummary {
            name: comp.pattern.clone(),
            description: comp.description.clone(),
            pad_count: comp.pad_count(),
        })
        .collect();

    Ok(IntLibFootprintList { footprints })
}

pub fn cmd_parameters(
    path: &Path,
    component_filter: Option<&str>,
    keys_filter: Option<&str>,
) -> Result<IntLibParameterList, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    let keys: Option<Vec<String>> =
        keys_filter.map(|k| k.split(',').map(|s| s.trim().to_lowercase()).collect());

    let params: Vec<ComponentParameters> = intlib
        .parameters
        .iter()
        .filter(|p| {
            component_filter
                .is_none_or(|filter| p.name.to_lowercase().contains(&filter.to_lowercase()))
        })
        .map(|p| {
            let filtered_params = p
                .params
                .iter()
                .filter(|(key, _)| {
                    keys.as_ref()
                        .is_none_or(|k_vec| k_vec.iter().any(|k| key.to_lowercase().contains(k)))
                })
                .map(|(k, v)| (k.to_string(), v.as_str().to_string()))
                .collect();
            ComponentParameters {
                component_name: p.name.clone(),
                params: filtered_params,
            }
        })
        .collect();

    Ok(IntLibParameterList { parameters: params })
}

/// Extract the embedded SchLib to a standalone file.
/// Returns success message.
pub fn cmd_extract_schlib(
    path: &Path,
    output: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    intlib.schlib.save_to_file(output)?;

    Ok(format!(
        "Extracted SchLib to: {}\n  Components: {}",
        output.display(),
        intlib.schematic_component_count()
    ))
}

/// Extract the embedded PcbLib to a standalone file.
/// Returns success message.
pub fn cmd_extract_pcblib(
    path: &Path,
    output: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let intlib = open_intlib(path)?;

    intlib.pcblib.save_to_file(output)?;

    Ok(format!(
        "Extracted PcbLib to: {}\n  Footprints: {}",
        output.display(),
        intlib.footprint_count()
    ))
}

// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! PCB document operations.
//!
//! High-level operations for exploring and editing Altium PCB document (.PcbDoc) files.
//!
//! NOTE: This module has been migrated from V1 to V2 API. Write/edit operations
//! are currently stubbed and will be implemented in M7 when V2 write support is complete.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

// V2 PCB document types - correct coordinate scale (10K units/mil)
use crate::v2::pcb::io::pcbdoc::PcbDoc;
use crate::v2::pcb::rule::PcbRule;
use crate::v2::pcb::enums::TLayer;
use crate::v2::pcb::PcbCoord;
use crate::ops::output::*;

/// Open a PcbDoc file using V2 API.
fn open_pcbdoc(path: &Path) -> Result<PcbDoc, String> {
    let file = File::open(path).map_err(|e| format!("Error opening file: {}", e))?;
    PcbDoc::open(BufReader::new(file)).map_err(|e| format!("Error parsing PcbDoc: {:?}", e))
}

/// Parse a coordinate string like "10mil" or "0.254mm".
#[allow(dead_code)]
fn parse_coord(s: &str) -> Result<PcbCoord, String> {
    let s = s.trim().to_lowercase();

    if s.ends_with("mil") {
        let val: f64 = s
            .trim_end_matches("mil")
            .trim()
            .parse()
            .map_err(|_| format!("Invalid coordinate: {}", s))?;
        Ok(PcbCoord::from_mils(val))
    } else if s.ends_with("mm") {
        let val: f64 = s
            .trim_end_matches("mm")
            .trim()
            .parse()
            .map_err(|_| format!("Invalid coordinate: {}", s))?;
        Ok(PcbCoord::from_mms(val))
    } else {
        // Try parsing as mils by default
        let val: f64 = s
            .parse()
            .map_err(|_| format!("Invalid coordinate: {} (use '10mil' or '0.254mm')", s))?;
        Ok(PcbCoord::from_mils(val))
    }
}

/// Get rule kind name for display.
fn rule_kind_display(rule: &PcbRule) -> String {
    rule.rule_kind_str().unwrap_or("Unknown").to_string()
}

// ═══════════════════════════════════════════════════════════════════════════
// HIGH-LEVEL COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Complete document overview.
pub fn cmd_overview(path: &Path) -> Result<PcbDocOverview, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    // Calculate total primitives count from all typed vectors
    let primitives_count = pcb.tracks.len()
        + pcb.arcs.len()
        + pcb.fills.len()
        + pcb.pads.len()
        + pcb.vias.len()
        + pcb.texts.len()
        + pcb.regions.len()
        + pcb.component_bodies.len();

    // Summary statistics
    let summary = PcbDocSummary {
        components: pcb.components.len(),
        nets: pcb.nets.len(),
        rules: pcb.rules.len(),
        primitives: primitives_count,
        tracks: pcb.tracks.len(),
        vias: pcb.vias.len(),
    };

    // Design rules by category
    let mut rules_by_kind: HashMap<String, Vec<&PcbRule>> = HashMap::new();
    for rule in &pcb.rules {
        rules_by_kind
            .entry(rule_kind_display(rule))
            .or_default()
            .push(rule);
    }

    let mut rules_by_category: Vec<(String, Vec<RuleSummary>)> = Vec::new();
    let mut categories: Vec<_> = rules_by_kind.keys().cloned().collect();
    categories.sort();

    for category in categories {
        let rules = &rules_by_kind[&category];
        let rule_summaries: Vec<RuleSummary> = rules
            .iter()
            .map(|rule| RuleSummary {
                name: rule.name().unwrap_or("").to_string(),
                priority: rule.priority().unwrap_or(0),
                enabled: rule.properties.get("ENABLED").map(|v| v != "FALSE").unwrap_or(true),
            })
            .collect();
        rules_by_category.push((category, rule_summaries));
    }

    // Component preview (first 10)
    let components_preview: Vec<ComponentPreview> = pcb
        .components
        .iter()
        .take(10)
        .map(|comp| ComponentPreview {
            designator: comp.source_designator().unwrap_or("").to_string(),
            pattern: comp.pattern().unwrap_or("").to_string(),
            comment: comp.properties.get("COMMENT").cloned().unwrap_or_default(),
        })
        .collect();

    // Nets preview (first 10)
    let nets_preview: Vec<String> = pcb.nets.iter()
        .take(10)
        .filter_map(|n| n.name().map(|s| s.to_string()))
        .collect();

    Ok(PcbDocOverview {
        path: path.display().to_string(),
        summary,
        rules_by_category,
        components_preview,
        nets_preview,
    })
}

/// Document info and statistics.
pub fn cmd_info(path: &Path) -> Result<PcbDocInfo, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    // Calculate total primitives count from all typed vectors
    let primitives_count = pcb.tracks.len()
        + pcb.arcs.len()
        + pcb.fills.len()
        + pcb.pads.len()
        + pcb.vias.len()
        + pcb.texts.len()
        + pcb.regions.len()
        + pcb.component_bodies.len();

    Ok(PcbDocInfo {
        path: path.display().to_string(),
        component_count: pcb.components.len(),
        net_count: pcb.nets.len(),
        rule_count: pcb.rules.len(),
        primitive_count: primitives_count,
        track_count: pcb.tracks.len(),
        via_count: pcb.vias.len(),
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// DESIGN RULE COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// List all design rules.
pub fn cmd_rules(
    path: &Path,
    kind_filter: Option<String>,
    verbose: bool,
) -> Result<PcbDocRuleList, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    let kind_filter_lower = kind_filter.as_ref().map(|s| s.to_lowercase());

    let filtered_rules: Vec<_> = pcb
        .rules
        .iter()
        .filter(|rule| {
            if let Some(ref filter) = kind_filter_lower {
                rule_kind_display(rule)
                    .to_lowercase()
                    .contains(filter)
            } else {
                true
            }
        })
        .collect();

    let rules: Vec<RuleInfo> = filtered_rules
        .iter()
        .map(|rule| {
            let parameters = if verbose {
                Some(
                    rule.properties
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                )
            } else {
                None
            };

            RuleInfo {
                name: rule.name().unwrap_or("").to_string(),
                kind: rule_kind_display(rule),
                enabled: rule.properties.get("ENABLED").map(|v| v != "FALSE").unwrap_or(true),
                priority: rule.priority().unwrap_or(0),
                scope1_expression: rule.scope1_expression().unwrap_or("").to_string(),
                scope2_expression: rule.scope2_expression().unwrap_or("").to_string(),
                comment: rule.properties.get("COMMENT").cloned().unwrap_or_default(),
                parameters,
            }
        })
        .collect();

    Ok(PcbDocRuleList {
        path: path.display().to_string(),
        filter: kind_filter,
        total_rules: rules.len(),
        rules,
    })
}

/// Show details for a specific rule.
pub fn cmd_rule(
    path: &Path,
    name: &str,
    _show_params: bool,
) -> Result<PcbDocRuleDetail, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    let name_lower = name.to_lowercase();
    let rule = pcb
        .rules
        .iter()
        .find(|r| r.name().map(|n| n.to_lowercase()) == Some(name_lower.clone()))
        .ok_or_else(|| format!("Rule '{}' not found", name))?;

    Ok(PcbDocRuleDetail {
        name: rule.name().unwrap_or("").to_string(),
        kind: rule_kind_display(rule),
        enabled: rule.properties.get("ENABLED").map(|v| v != "FALSE").unwrap_or(true),
        priority: rule.priority().unwrap_or(0),
        scope1_expression: rule.scope1_expression().unwrap_or("").to_string(),
        scope2_expression: rule.scope2_expression().unwrap_or("").to_string(),
        comment: rule.properties.get("COMMENT").cloned().unwrap_or_default(),
        parameters: rule
            .properties
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    })
}

/// Add a new design rule.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_rule(
    _path: &Path,
    _kind_str: &str,
    _name: &str,
    _priority: i32,
    _scope1: &str,
    _scope2: &str,
    _gap: Option<String>,
    _min_width: Option<String>,
    _max_width: Option<String>,
    _pref_width: Option<String>,
    _comment: Option<String>,
    _disabled: bool,
) -> Result<(), String> {
    Err("cmd_add_rule is not yet implemented for V2 API. This will be available in M7.".to_string())
}

/// Modify an existing design rule.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
#[allow(clippy::too_many_arguments)]
pub fn cmd_modify_rule(
    _path: &Path,
    _name: &str,
    _priority: Option<i32>,
    _gap: Option<String>,
    _min_width: Option<String>,
    _max_width: Option<String>,
    _pref_width: Option<String>,
    _comment: Option<String>,
    _enable: bool,
    _disable: bool,
) -> Result<(), String> {
    Err("cmd_modify_rule is not yet implemented for V2 API. This will be available in M7.".to_string())
}

/// Delete a design rule.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
pub fn cmd_delete_rule(_path: &Path, _name: &str) -> Result<(), String> {
    Err("cmd_delete_rule is not yet implemented for V2 API. This will be available in M7.".to_string())
}

/// Export as JSON.
pub fn cmd_json(
    path: &Path,
    full: bool,
    _pretty: bool,
) -> Result<PcbDocJson, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    // Calculate total primitives count from all typed vectors
    let primitives_count = pcb.tracks.len()
        + pcb.arcs.len()
        + pcb.fills.len()
        + pcb.pads.len()
        + pcb.vias.len()
        + pcb.texts.len()
        + pcb.regions.len()
        + pcb.component_bodies.len();

    let summary = PcbDocSummary {
        components: pcb.components.len(),
        nets: pcb.nets.len(),
        rules: pcb.rules.len(),
        primitives: primitives_count,
        tracks: pcb.tracks.len(),
        vias: pcb.vias.len(),
    };

    let rules: Option<Vec<RuleInfo>> = if full {
        Some(
            pcb.rules
                .iter()
                .map(|rule| RuleInfo {
                    name: rule.name().unwrap_or("").to_string(),
                    kind: rule_kind_display(rule),
                    enabled: rule.properties.get("ENABLED").map(|v| v != "FALSE").unwrap_or(true),
                    priority: rule.priority().unwrap_or(0),
                    scope1_expression: rule.scope1_expression().unwrap_or("").to_string(),
                    scope2_expression: rule.scope2_expression().unwrap_or("").to_string(),
                    comment: rule.properties.get("COMMENT").cloned().unwrap_or_default(),
                    parameters: Some(
                        rule.properties
                            .iter()
                            .map(|(k, v)| (k.to_string(), v.to_string()))
                            .collect(),
                    ),
                })
                .collect(),
        )
    } else {
        None
    };

    let components: Option<Vec<PcbComponentInfo>> = if full {
        Some(
            pcb.components
                .iter()
                .map(|c| {
                    let locked = c
                        .properties
                        .get("LOCKED")
                        .map(|v| v == "T" || v == "TRUE")
                        .unwrap_or(false);
                    let layer_num = c.layer().unwrap_or(0);
                    let layer_name = TLayer::from_u8(layer_num)
                        .map(|l| format!("{:?}", l))
                        .unwrap_or_else(|| format!("Layer{}", layer_num));
                    PcbComponentInfo {
                        designator: c.source_designator().unwrap_or("").to_string(),
                        pattern: c.pattern().unwrap_or("").to_string(),
                        comment: c.properties.get("COMMENT").cloned().unwrap_or_default(),
                        x: c.location_x().map(|coord| format!("{:.3}mm", coord.to_mms())),
                        y: c.location_y().map(|coord| format!("{:.3}mm", coord.to_mms())),
                        rotation: c.rotation().unwrap_or(0.0),
                        layer: layer_name,
                        locked,
                    }
                })
                .collect(),
        )
    } else {
        None
    };

    let nets = if full {
        Some(pcb.nets.iter()
            .filter_map(|n| n.name().map(|s| s.to_string()))
            .collect())
    } else {
        None
    };

    Ok(PcbDocJson {
        file: path.display().to_string(),
        summary,
        rules,
        components,
        nets,
        layers: None,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// COMPONENT COMMAND IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

/// List all components.
pub fn cmd_components(
    path: &Path,
    _verbose: bool,
    layer_filter: Option<String>,
) -> Result<PcbDocComponentList, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    // Filter by layer if specified
    let layer_filter_lower = layer_filter.as_ref().map(|s| s.to_lowercase());

    let components: Vec<PcbComponentInfo> = pcb
        .components
        .iter()
        .filter(|component| {
            if let Some(ref filter) = layer_filter_lower {
                let layer_num = component.layer().unwrap_or(0);
                let layer_name = TLayer::from_u8(layer_num)
                    .map(|l| format!("{:?}", l))
                    .unwrap_or_else(|| format!("Layer{}", layer_num));
                layer_name.to_lowercase().contains(filter)
            } else {
                true
            }
        })
        .map(|component| {
            let locked = component
                .properties
                .get("LOCKED")
                .map(|v| v == "T" || v == "TRUE")
                .unwrap_or(false);
            let layer_num = component.layer().unwrap_or(0);
            let layer_name = TLayer::from_u8(layer_num)
                .map(|l| format!("{:?}", l))
                .unwrap_or_else(|| format!("Layer{}", layer_num));
            PcbComponentInfo {
                designator: component.source_designator().unwrap_or("").to_string(),
                pattern: component.pattern().unwrap_or("").to_string(),
                comment: component.properties.get("COMMENT").cloned().unwrap_or_default(),
                x: component.location_x().map(|c| format!("{:.3}mm", c.to_mms())),
                y: component.location_y().map(|c| format!("{:.3}mm", c.to_mms())),
                rotation: component.rotation().unwrap_or(0.0),
                layer: layer_name,
                locked,
            }
        })
        .collect();

    Ok(PcbDocComponentList {
        path: path.display().to_string(),
        total_components: components.len(),
        layer_filter,
        components,
    })
}

/// Show component details.
pub fn cmd_component(
    path: &Path,
    designator: &str,
    _show_params: bool,
) -> Result<PcbDocComponentDetail, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    let designator_lower = designator.to_lowercase();
    let component = pcb
        .components
        .iter()
        .find(|c| c.source_designator().map(|d| d.to_lowercase()) == Some(designator_lower.clone()))
        .ok_or_else(|| format!("Component '{}' not found", designator))?;

    // V2 doesn't track primitives per component, use 0 for now
    let pad_count = 0;

    let locked = component
        .properties
        .get("LOCKED")
        .map(|v| v == "T" || v == "TRUE")
        .unwrap_or(false);
    let source_designator = component
        .source_designator()
        .unwrap_or("")
        .to_string();
    let source_footprint = component
        .properties
        .get("SOURCEFOOTPRINTLIBRARY")
        .cloned()
        .unwrap_or_default();
    let unique_id = component
        .properties
        .get("UNIQUEID")
        .cloned()
        .unwrap_or_default();

    let layer_num = component.layer().unwrap_or(0);
    let layer_name = TLayer::from_u8(layer_num)
        .map(|l| format!("{:?}", l))
        .unwrap_or_else(|| format!("Layer{}", layer_num));

    Ok(PcbDocComponentDetail {
        designator: source_designator.clone(),
        pattern: component.pattern().unwrap_or("").to_string(),
        comment: component.properties.get("COMMENT").cloned().unwrap_or_default(),
        source_designator,
        source_footprint,
        x: component.location_x().map(|c| format!("{:.4}mm", c.to_mms())),
        y: component.location_y().map(|c| format!("{:.4}mm", c.to_mms())),
        rotation: component.rotation().unwrap_or(0.0),
        layer: layer_name,
        locked,
        pad_count,
        unique_id,
    })
}

/// Place (move) a component.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
#[allow(clippy::too_many_arguments)]
pub fn cmd_place_component(
    _path: &Path,
    _designator: &str,
    _at: Option<String>,
    _near: Option<String>,
    _align_x: Option<String>,
    _align_y: Option<String>,
    _edge: Option<String>,
    _offset: Option<String>,
    _rotation: Option<f64>,
    _layer: Option<String>,
    _grid: Option<String>,
    _force: bool,
) -> Result<(), String> {
    Err("cmd_place_component is not yet implemented for V2 API. This will be available in M7.".to_string())
}

/// Add a component from schematic.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
pub fn cmd_add_component(
    _path: &Path,
    _schematic: &Path,
    _designator: &str,
    _footprint_lib: Option<PathBuf>,
    _footprint: Option<String>,
    _at: Option<String>,
    _layer: &str,
) -> Result<(), String> {
    Err("cmd_add_component is not yet implemented for V2 API. This will be available in M7.".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// CREATION COMMAND IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Embedded blank PcbDoc template.
const BLANK_PCBDOC_TEMPLATE: &[u8] = include_bytes!("../../data/PCB1.PcbDoc");

/// Create a new empty PcbDoc file.
pub fn cmd_create(path: &Path, template: Option<PathBuf>) -> Result<(), String> {
    if path.exists() {
        return Err(format!("File already exists: {}", path.display()));
    }

    match template {
        Some(template_path) => {
            std::fs::copy(&template_path, path)
                .map_err(|e| format!("Error copying template: {}", e))?;
            println!("Created PcbDoc from template: {}", path.display());
            println!("  Template: {}", template_path.display());
        }
        None => {
            std::fs::write(path, BLANK_PCBDOC_TEMPLATE)
                .map_err(|e| format!("Error creating file: {}", e))?;
            println!("Created empty PcbDoc: {}", path.display());
        }
    }

    // Verify the file was created correctly
    let pcb = open_pcbdoc(path)?;
    println!("  Rules: {}", pcb.rules.len());
    println!("  Classes: {}", pcb.classes.len());

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// BOARD OUTLINE COMMANDS (STUBBED)
// ═══════════════════════════════════════════════════════════════════════════

/// Display the board outline.
///
/// NOTE: Stubbed during V1->V2 migration. Board outline parsing needs V2 board support.
pub fn cmd_outline(_path: &Path, _json: bool) -> Result<PcbDocOutline, Box<dyn std::error::Error>> {
    Err("cmd_outline is not yet implemented for V2 API. Board parsing will be available in M7.".into())
}

/// Set board outline to a rectangle.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
pub fn cmd_set_outline_rect(
    _path: &Path,
    _width: &str,
    _height: &str,
    _origin_x: &str,
    _origin_y: &str,
) -> Result<(), String> {
    Err("cmd_set_outline_rect is not yet implemented for V2 API. This will be available in M7.".to_string())
}

/// Set board outline from vertices.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
pub fn cmd_set_outline(_path: &Path, _vertices_str: &str) -> Result<(), String> {
    Err("cmd_set_outline is not yet implemented for V2 API. This will be available in M7.".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// BOARD SETTINGS COMMANDS (STUBBED)
// ═══════════════════════════════════════════════════════════════════════════

/// Display board settings.
///
/// NOTE: Stubbed during V1->V2 migration. Board settings need V2 board support.
pub fn cmd_settings(
    _path: &Path,
    _json: bool,
) -> Result<PcbDocSettings, Box<dyn std::error::Error>> {
    Err("cmd_settings is not yet implemented for V2 API. Board parsing will be available in M7.".into())
}

/// Modify board settings.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
#[allow(clippy::too_many_arguments)]
pub fn cmd_set_settings(
    _path: &Path,
    _metric: bool,
    _imperial: bool,
    _snap_grid: Option<String>,
    _visible_grid: Option<String>,
    _component_grid: Option<String>,
    _track_grid: Option<String>,
    _via_grid: Option<String>,
    _track_width: Option<String>,
    _origin_x: Option<String>,
    _origin_y: Option<String>,
) -> Result<(), String> {
    Err("cmd_set_settings is not yet implemented for V2 API. This will be available in M7.".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// LAYER STACK COMMANDS (STUBBED)
// ═══════════════════════════════════════════════════════════════════════════

/// Display the layer stack.
///
/// NOTE: Stubbed during V1->V2 migration. Layer stack parsing needs V2 support.
pub fn cmd_layers(_path: &Path, _all: bool) -> Result<PcbDocLayers, Box<dyn std::error::Error>> {
    Err("cmd_layers is not yet implemented for V2 API. Layer parsing will be available in M7.".into())
}

// ═══════════════════════════════════════════════════════════════════════════
// KEEPOUT COMMANDS (STUBBED)
// ═══════════════════════════════════════════════════════════════════════════

/// List all keepout regions.
///
/// NOTE: Stubbed during V1->V2 migration. Region parsing needs V2 support.
pub fn cmd_keepouts(
    _path: &Path,
    _layer_filter: Option<String>,
) -> Result<PcbDocKeepouts, Box<dyn std::error::Error>> {
    Err("cmd_keepouts is not yet implemented for V2 API. Region parsing will be available in M7.".into())
}

/// Add a rectangular keepout region.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
pub fn cmd_add_keepout(
    _path: &Path,
    _layer_str: &str,
    _x1: &str,
    _y1: &str,
    _x2: &str,
    _y2: &str,
) -> Result<(), String> {
    Err("cmd_add_keepout is not yet implemented for V2 API. This will be available in M7.".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// CUTOUT COMMANDS (STUBBED)
// ═══════════════════════════════════════════════════════════════════════════

/// List all board cutouts.
///
/// NOTE: Stubbed during V1->V2 migration. Region parsing needs V2 support.
pub fn cmd_cutouts(_path: &Path) -> Result<PcbDocCutouts, Box<dyn std::error::Error>> {
    Err("cmd_cutouts is not yet implemented for V2 API. Region parsing will be available in M7.".into())
}

/// Add a rectangular board cutout.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
pub fn cmd_add_cutout(_path: &Path, _x1: &str, _y1: &str, _x2: &str, _y2: &str) -> Result<(), String> {
    Err("cmd_add_cutout is not yet implemented for V2 API. This will be available in M7.".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// POLYGON (COPPER POUR) COMMANDS (STUBBED)
// ═══════════════════════════════════════════════════════════════════════════

/// List all polygons (copper pours).
///
/// NOTE: Stubbed during V1->V2 migration. Polygon parsing needs V2 support.
pub fn cmd_polygons(
    _path: &Path,
    _layer_filter: Option<String>,
    _net_filter: Option<String>,
) -> Result<PcbDocPolygons, Box<dyn std::error::Error>> {
    Err("cmd_polygons is not yet implemented for V2 API. Polygon parsing will be available in M7.".into())
}

/// Show details for a specific polygon.
///
/// NOTE: Stubbed during V1->V2 migration. Polygon parsing needs V2 support.
pub fn cmd_polygon(
    _path: &Path,
    _index: usize,
) -> Result<PcbDocPolygonDetail, Box<dyn std::error::Error>> {
    Err("cmd_polygon is not yet implemented for V2 API. Polygon parsing will be available in M7.".into())
}

/// Add a polygon (copper pour) to the PCB.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
pub fn cmd_add_polygon(
    _path: &Path,
    _layer_str: &str,
    _net: &str,
    _vertices_str: &str,
    _pour_over: bool,
    _remove_dead: bool,
    _hatch_style_str: &str,
) -> Result<(), String> {
    Err("cmd_add_polygon is not yet implemented for V2 API. This will be available in M7.".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// TRACK COMMANDS (STUBBED)
// ═══════════════════════════════════════════════════════════════════════════

/// List all tracks.
///
/// NOTE: Stubbed during V1->V2 migration. Track access needs V2 iteration support.
pub fn cmd_tracks(
    _path: &Path,
    _layer_filter: Option<String>,
) -> Result<PcbDocTracks, Box<dyn std::error::Error>> {
    Err("cmd_tracks is not yet implemented for V2 API. Track parsing will be available in M7.".into())
}

/// Add a track segment.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_track(
    _path: &Path,
    _start: Option<String>,
    _end: Option<String>,
    _start_pad: Option<String>,
    _end_pad: Option<String>,
    _width: Option<String>,
    _layer: &str,
    _net: Option<String>,
) -> Result<(), String> {
    Err("cmd_add_track is not yet implemented for V2 API. This will be available in M7.".to_string())
}

/// Add a multi-segment track path.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
pub fn cmd_add_track_path(
    _path: &Path,
    _vertices: &str,
    _width: Option<String>,
    _layer: &str,
    _net: Option<String>,
) -> Result<(), String> {
    Err("cmd_add_track_path is not yet implemented for V2 API. This will be available in M7.".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// VIA COMMANDS (STUBBED)
// ═══════════════════════════════════════════════════════════════════════════

/// List all vias.
///
/// NOTE: Stubbed during V1->V2 migration. Via access needs V2 iteration support.
pub fn cmd_vias(_path: &Path) -> Result<PcbDocVias, Box<dyn std::error::Error>> {
    Err("cmd_vias is not yet implemented for V2 API. Via parsing will be available in M7.".into())
}

/// Add a via.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_via(
    _path: &Path,
    _at: Option<String>,
    _at_pad: Option<String>,
    _diameter: Option<String>,
    _hole: Option<String>,
    _from_layer: &str,
    _to_layer: &str,
    _net: Option<String>,
) -> Result<(), String> {
    Err("cmd_add_via is not yet implemented for V2 API. This will be available in M7.".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// ARC COMMANDS (STUBBED)
// ═══════════════════════════════════════════════════════════════════════════

/// List all arcs.
///
/// NOTE: Stubbed during V1->V2 migration. Arc access needs V2 iteration support.
pub fn cmd_arcs(
    _path: &Path,
    _layer_filter: Option<String>,
) -> Result<PcbDocArcs, Box<dyn std::error::Error>> {
    Err("cmd_arcs is not yet implemented for V2 API. Arc parsing will be available in M7.".into())
}

/// Add an arc.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_arc(
    _path: &Path,
    _center: &str,
    _radius: &str,
    _start_angle: f64,
    _end_angle: f64,
    _width: Option<String>,
    _layer: &str,
    _net: Option<String>,
) -> Result<(), String> {
    Err("cmd_add_arc is not yet implemented for V2 API. This will be available in M7.".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// FILL COMMANDS (STUBBED)
// ═══════════════════════════════════════════════════════════════════════════

/// List all fills.
///
/// NOTE: Stubbed during V1->V2 migration. Fill access needs V2 iteration support.
pub fn cmd_fills(
    _path: &Path,
    _layer_filter: Option<String>,
) -> Result<PcbDocFills, Box<dyn std::error::Error>> {
    Err("cmd_fills is not yet implemented for V2 API. Fill parsing will be available in M7.".into())
}

/// Add a fill.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
pub fn cmd_add_fill(
    _path: &Path,
    _layer_str: &str,
    _net: Option<&str>,
    _x1: &str,
    _y1: &str,
    _x2: &str,
    _y2: &str,
    _rotation: f64,
) -> Result<(), String> {
    Err("cmd_add_fill is not yet implemented for V2 API. This will be available in M7.".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// TEXT COMMANDS (STUBBED)
// ═══════════════════════════════════════════════════════════════════════════

/// List all text objects.
///
/// NOTE: Stubbed during V1->V2 migration. Text access needs V2 iteration support.
pub fn cmd_texts(
    _path: &Path,
    _layer_filter: Option<String>,
) -> Result<PcbDocTexts, Box<dyn std::error::Error>> {
    Err("cmd_texts is not yet implemented for V2 API. Text parsing will be available in M7.".into())
}

/// Add a text object.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_text(
    _path: &Path,
    _layer_str: &str,
    _text: &str,
    _x: &str,
    _y: &str,
    _height: Option<String>,
    _rotation: f64,
    _mirror: bool,
) -> Result<(), String> {
    Err("cmd_add_text is not yet implemented for V2 API. This will be available in M7.".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// REGION COMMANDS (STUBBED)
// ═══════════════════════════════════════════════════════════════════════════

/// List all regions.
///
/// NOTE: Stubbed during V1->V2 migration. Region access needs V2 iteration support.
pub fn cmd_regions(
    _path: &Path,
    _layer_filter: Option<String>,
) -> Result<PcbDocRegions, Box<dyn std::error::Error>> {
    Err("cmd_regions is not yet implemented for V2 API. Region parsing will be available in M7.".into())
}

/// Add a region.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
pub fn cmd_add_region(
    _path: &Path,
    _layer_str: &str,
    _net: Option<&str>,
    _vertices_str: &str,
    _kind: Option<&str>,
) -> Result<(), String> {
    Err("cmd_add_region is not yet implemented for V2 API. This will be available in M7.".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// DELETE COMMANDS (STUBBED)
// ═══════════════════════════════════════════════════════════════════════════

/// Delete a primitive by index.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
pub fn cmd_delete_primitive(_path: &Path, _index: usize) -> Result<(), String> {
    Err("cmd_delete_primitive is not yet implemented for V2 API. This will be available in M7.".to_string())
}

/// Delete all tracks on a layer.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
pub fn cmd_delete_tracks(_path: &Path, _layer_str: &str) -> Result<(), String> {
    Err("cmd_delete_tracks is not yet implemented for V2 API. This will be available in M7.".to_string())
}

/// Delete all vias.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
pub fn cmd_delete_vias(_path: &Path) -> Result<(), String> {
    Err("cmd_delete_vias is not yet implemented for V2 API. This will be available in M7.".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// NET COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// List all nets.
pub fn cmd_nets(path: &Path) -> Result<PcbDocNets, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    let nets: Vec<String> = pcb.nets.iter()
        .filter_map(|n| n.name().map(|s| s.to_string()))
        .collect();

    Ok(PcbDocNets {
        path: path.display().to_string(),
        total_nets: nets.len(),
        nets,
    })
}

/// Add a net.
///
/// NOTE: Stubbed during V1->V2 migration. Will be implemented in M7 when V2 write support is complete.
pub fn cmd_add_net(_path: &Path, _name: &str) -> Result<(), String> {
    Err("cmd_add_net is not yet implemented for V2 API. This will be available in M7.".to_string())
}

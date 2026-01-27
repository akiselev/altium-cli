// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! PCB document operations.
//!
//! High-level operations for exploring and editing Altium PCB document (.PcbDoc) files.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use crate::edit::pcb_placement::{
    parse_coord as placement_parse_coord, parse_offset, parse_position,
};
use crate::edit::{BoardEdge, PcbPlacementEngine, PlacementAnchor};
use crate::io::PcbDoc;
use crate::ops::output::*;
use crate::records::pcb::{
    HatchStyle, PcbBoard, PcbPolygon, PcbRule, PolygonVertex, PolygonVertexKind, RuleKind,
};
use crate::types::{Coord, CoordPoint, Layer};

/// Open a PcbDoc file.
fn open_pcbdoc(path: &Path) -> Result<PcbDoc, String> {
    let file = File::open(path).map_err(|e| format!("Error opening file: {}", e))?;
    PcbDoc::open(BufReader::new(file)).map_err(|e| format!("Error parsing PcbDoc: {:?}", e))
}

/// Parse a coordinate string like "10mil" or "0.254mm".
fn parse_coord(s: &str) -> Result<Coord, String> {
    let s = s.trim().to_lowercase();

    if s.ends_with("mil") {
        let val: f64 = s
            .trim_end_matches("mil")
            .trim()
            .parse()
            .map_err(|_| format!("Invalid coordinate: {}", s))?;
        Ok(Coord::from_mils(val))
    } else if s.ends_with("mm") {
        let val: f64 = s
            .trim_end_matches("mm")
            .trim()
            .parse()
            .map_err(|_| format!("Invalid coordinate: {}", s))?;
        Ok(Coord::from_mms(val))
    } else {
        // Try parsing as mils by default
        let val: f64 = s
            .parse()
            .map_err(|_| format!("Invalid coordinate: {} (use '10mil' or '0.254mm')", s))?;
        Ok(Coord::from_mils(val))
    }
}

/// Get rule kind name for display.
fn rule_kind_display(kind: &RuleKind) -> String {
    format!("{}", kind)
}

// ═══════════════════════════════════════════════════════════════════════════
// HIGH-LEVEL COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Complete document overview.
pub fn cmd_overview(path: &Path) -> Result<PcbDocOverview, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    // Summary statistics
    let summary = PcbDocSummary {
        components: pcb.components.len(),
        nets: pcb.nets.len(),
        rules: pcb.rules.len(),
        primitives: pcb.primitives.len(),
        tracks: pcb.track_count(),
        vias: pcb.via_count(),
    };

    // Design rules by category
    let mut rules_by_kind: HashMap<String, Vec<&PcbRule>> = HashMap::new();
    for rule in &pcb.rules {
        rules_by_kind
            .entry(rule_kind_display(&rule.kind))
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
                name: rule.name.clone(),
                priority: rule.priority,
                enabled: rule.enabled,
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
            designator: comp.designator.clone(),
            pattern: comp.pattern.clone(),
            comment: comp.comment.clone(),
        })
        .collect();

    // Nets preview (first 10)
    let nets_preview: Vec<String> = pcb.nets.iter().take(10).cloned().collect();

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

    Ok(PcbDocInfo {
        path: path.display().to_string(),
        component_count: pcb.components.len(),
        net_count: pcb.nets.len(),
        rule_count: pcb.rules.len(),
        primitive_count: pcb.primitives.len(),
        track_count: pcb.track_count(),
        via_count: pcb.via_count(),
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
                rule_kind_display(&rule.kind)
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
                    rule.params
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                )
            } else {
                None
            };

            RuleInfo {
                name: rule.name.clone(),
                kind: rule_kind_display(&rule.kind),
                enabled: rule.enabled,
                priority: rule.priority,
                scope1_expression: rule.scope1_expression.clone(),
                scope2_expression: rule.scope2_expression.clone(),
                comment: rule.comment.clone(),
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
        .find(|r| r.name.to_lowercase() == name_lower)
        .ok_or_else(|| format!("Rule '{}' not found", name))?;

    Ok(PcbDocRuleDetail {
        name: rule.name.clone(),
        kind: rule_kind_display(&rule.kind),
        enabled: rule.enabled,
        priority: rule.priority,
        scope1_expression: rule.scope1_expression.clone(),
        scope2_expression: rule.scope2_expression.clone(),
        comment: rule.comment.clone(),
        parameters: rule
            .params
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    })
}

/// Add a new design rule.
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_rule(
    path: &Path,
    kind_str: &str,
    name: &str,
    priority: i32,
    scope1: &str,
    scope2: &str,
    gap: Option<String>,
    min_width: Option<String>,
    max_width: Option<String>,
    pref_width: Option<String>,
    comment: Option<String>,
    disabled: bool,
) -> Result<(), String> {
    let mut pcb = open_pcbdoc(path)?;

    // Check if rule already exists
    if pcb
        .rules
        .iter()
        .any(|r| r.name.to_lowercase() == name.to_lowercase())
    {
        return Err(format!("Rule '{}' already exists", name));
    }

    // Parse rule kind
    let kind = RuleKind::from_name(kind_str)
        .ok_or_else(|| format!("Unknown rule kind: '{}'. Valid kinds: Clearance, Width, RoutingLayers, RoutingVias, etc.", kind_str))?;

    // Create the rule
    let mut rule = PcbRule::new(kind, name);
    rule.enabled = !disabled;
    rule.priority = priority;
    rule.scope1_expression = scope1.to_string();
    rule.scope2_expression = scope2.to_string();

    // Generate a unique ID (8 uppercase chars)
    rule.unique_id = format!(
        "{:08X}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32
    );

    if let Some(ref c) = comment {
        rule.comment = c.clone();
    }

    // Set parameters based on rule kind
    if let Some(ref gap_str) = gap {
        let coord = parse_coord(gap_str)?;
        rule.params.add_coord("GAP", coord);
    }
    if let Some(ref min_str) = min_width {
        let coord = parse_coord(min_str)?;
        rule.params.add_coord("MINWIDTH", coord);
    }
    if let Some(ref max_str) = max_width {
        let coord = parse_coord(max_str)?;
        rule.params.add_coord("MAXWIDTH", coord);
    }
    if let Some(ref pref_str) = pref_width {
        let coord = parse_coord(pref_str)?;
        rule.params.add_coord("PREFWIDTH", coord);
    }

    // Add common fields that Altium expects
    rule.params.add("SELECTION", "FALSE");
    rule.params.add("LAYER", "UNKNOWN");
    rule.params.add("LOCKED", "FALSE");
    rule.params.add("POLYGONOUTLINE", "FALSE");
    rule.params.add("USERROUTED", "TRUE");
    rule.params.add("KEEPOUT", "FALSE");
    rule.params.add("UNIONINDEX", "0");

    pcb.add_rule(rule);

    // Save the file
    pcb.save_to_file(path)
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!("Added rule '{}' ({}) to {}", name, kind_str, path.display());
    println!("Total rules: {}", pcb.rules.len());

    Ok(())
}

/// Modify an existing design rule.
#[allow(clippy::too_many_arguments)]
pub fn cmd_modify_rule(
    path: &Path,
    name: &str,
    priority: Option<i32>,
    gap: Option<String>,
    min_width: Option<String>,
    max_width: Option<String>,
    pref_width: Option<String>,
    comment: Option<String>,
    enable: bool,
    disable: bool,
) -> Result<(), String> {
    let mut pcb = open_pcbdoc(path)?;

    let name_lower = name.to_lowercase();
    let rule = pcb
        .rules
        .iter_mut()
        .find(|r| r.name.to_lowercase() == name_lower)
        .ok_or_else(|| format!("Rule '{}' not found", name))?;

    let mut changes = Vec::new();

    if let Some(p) = priority {
        rule.priority = p;
        changes.push(format!("priority={}", p));
    }

    if enable && disable {
        return Err("Cannot use both --enable and --disable".to_string());
    }
    if enable {
        rule.enabled = true;
        changes.push("enabled=true".to_string());
    }
    if disable {
        rule.enabled = false;
        changes.push("enabled=false".to_string());
    }

    if let Some(ref c) = comment {
        rule.comment = c.clone();
        changes.push(format!("comment={}", c));
    }

    if let Some(ref gap_str) = gap {
        let coord = parse_coord(gap_str)?;
        rule.params.add_coord("GAP", coord);
        changes.push(format!("GAP={}", gap_str));
    }
    if let Some(ref min_str) = min_width {
        let coord = parse_coord(min_str)?;
        rule.params.add_coord("MINWIDTH", coord);
        changes.push(format!("MINWIDTH={}", min_str));
    }
    if let Some(ref max_str) = max_width {
        let coord = parse_coord(max_str)?;
        rule.params.add_coord("MAXWIDTH", coord);
        changes.push(format!("MAXWIDTH={}", max_str));
    }
    if let Some(ref pref_str) = pref_width {
        let coord = parse_coord(pref_str)?;
        rule.params.add_coord("PREFWIDTH", coord);
        changes.push(format!("PREFWIDTH={}", pref_str));
    }

    if changes.is_empty() {
        println!("No changes specified");
        return Ok(());
    }

    // Save the file
    pcb.save_to_file(path)
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!("Modified rule '{}' in {}", name, path.display());
    for change in changes {
        println!("  {}", change);
    }

    Ok(())
}

/// Delete a design rule.
pub fn cmd_delete_rule(path: &Path, name: &str) -> Result<(), String> {
    let mut pcb = open_pcbdoc(path)?;

    let name_lower = name.to_lowercase();
    let original_count = pcb.rules.len();

    pcb.rules.retain(|r| r.name.to_lowercase() != name_lower);

    if pcb.rules.len() == original_count {
        return Err(format!("Rule '{}' not found", name));
    }

    // Save the file
    pcb.save_to_file(path)
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!("Deleted rule '{}' from {}", name, path.display());
    println!("Remaining rules: {}", pcb.rules.len());

    Ok(())
}

/// Export as JSON.
pub fn cmd_json(
    path: &Path,
    full: bool,
    _pretty: bool,
) -> Result<PcbDocJson, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    let summary = PcbDocSummary {
        components: pcb.components.len(),
        nets: pcb.nets.len(),
        rules: pcb.rules.len(),
        primitives: pcb.primitives.len(),
        tracks: pcb.track_count(),
        vias: pcb.via_count(),
    };

    let rules: Option<Vec<RuleInfo>> = if full {
        Some(
            pcb.rules
                .iter()
                .map(|rule| RuleInfo {
                    name: rule.name.clone(),
                    kind: rule_kind_display(&rule.kind),
                    enabled: rule.enabled,
                    priority: rule.priority,
                    scope1_expression: rule.scope1_expression.clone(),
                    scope2_expression: rule.scope2_expression.clone(),
                    comment: rule.comment.clone(),
                    parameters: Some(
                        rule.params
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
                        .params
                        .get("LOCKED")
                        .map(|v| v.to_string() == "T")
                        .unwrap_or(false);
                    PcbComponentInfo {
                        designator: c.designator.clone(),
                        pattern: c.pattern.clone(),
                        comment: c.comment.clone(),
                        x: c.x().map(|coord| format!("{:.3}mm", coord.to_mms())),
                        y: c.y().map(|coord| format!("{:.3}mm", coord.to_mms())),
                        rotation: c.rotation(),
                        layer: c.layer().name().to_string(),
                        locked,
                    }
                })
                .collect(),
        )
    } else {
        None
    };

    let nets = if full { Some(pcb.nets.clone()) } else { None };

    Ok(PcbDocJson {
        file: path.display().to_string(),
        summary,
        rules,
        components,
        nets,
        layers: None, // Not included in this implementation
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
                let layer_name = component.layer().name();
                layer_name.to_lowercase().contains(filter)
            } else {
                true
            }
        })
        .map(|component| {
            let locked = component
                .params
                .get("LOCKED")
                .map(|v| v.to_string() == "T")
                .unwrap_or(false);
            PcbComponentInfo {
                designator: component.designator.clone(),
                pattern: component.pattern.clone(),
                comment: component.comment.clone(),
                x: component.x().map(|c| format!("{:.3}mm", c.to_mms())),
                y: component.y().map(|c| format!("{:.3}mm", c.to_mms())),
                rotation: component.rotation(),
                layer: component.layer().name().to_string(),
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

    let component = pcb
        .find_component(designator)
        .ok_or_else(|| format!("Component '{}' not found", designator))?;

    // Count pads in component
    let pad_count = component
        .primitives
        .iter()
        .filter(|p| matches!(p, crate::records::pcb::PcbRecord::Pad(_)))
        .count();

    let locked = component
        .params
        .get("LOCKED")
        .map(|v| v.to_string() == "T")
        .unwrap_or(false);
    let source_designator = component
        .params
        .get("SOURCEDESIGNATOR")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let source_footprint = component
        .params
        .get("SOURCEFOOTPRINTLIBRARY")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let unique_id = component
        .params
        .get("UNIQUEID")
        .map(|v| v.to_string())
        .unwrap_or_default();

    Ok(PcbDocComponentDetail {
        designator: component.designator.clone(),
        pattern: component.pattern.clone(),
        comment: component.comment.clone(),
        source_designator,
        source_footprint,
        x: component.x().map(|c| format!("{:.4}mm", c.to_mms())),
        y: component.y().map(|c| format!("{:.4}mm", c.to_mms())),
        rotation: component.rotation(),
        layer: component.layer().name().to_string(),
        locked,
        pad_count,
        unique_id,
    })
}

/// Place (move) a component.
#[allow(clippy::too_many_arguments)]
pub fn cmd_place_component(
    path: &Path,
    designator: &str,
    at: Option<String>,
    near: Option<String>,
    align_x: Option<String>,
    align_y: Option<String>,
    edge: Option<String>,
    offset: Option<String>,
    rotation: Option<f64>,
    layer: Option<String>,
    grid: Option<String>,
    force: bool,
) -> Result<(), String> {
    let mut pcb = open_pcbdoc(path)?;

    // Check component exists
    if pcb.find_component(designator).is_none() {
        return Err(format!("Component '{}' not found", designator));
    }

    // Set up placement engine
    let mut engine = PcbPlacementEngine::new();

    // Configure grid
    if let Some(ref grid_str) = grid {
        let grid_coord = placement_parse_coord(grid_str)?;
        engine.set_grid(crate::edit::types::Grid {
            spacing: grid_coord,
            snap_enabled: true,
        });
    }

    // Calculate board bounds for edge alignment
    engine.calculate_board_bounds(&pcb);

    // Check for connected routes
    let connected = engine.find_connected_routes(&pcb, designator);
    if connected.has_connections() && !force {
        return Err(format!(
            "Component '{}' has {} connected routes ({} tracks, {} vias). Use --force to move anyway.",
            designator,
            connected.count(),
            connected.tracks.len(),
            connected.vias.len()
        ));
    }

    // Get current position
    let current_pos = engine.get_component_position(&pcb, designator);

    // Parse offset if provided
    let offset_point = if let Some(ref offset_str) = offset {
        parse_offset(offset_str)?
    } else {
        CoordPoint::ZERO
    };

    // Determine placement anchor
    let anchor = if let Some(ref at_str) = at {
        PlacementAnchor::Absolute(parse_position(at_str)?)
    } else if let Some(ref near_str) = near {
        PlacementAnchor::NearComponent {
            designator: near_str.clone(),
            offset: offset_point,
        }
    } else if let Some(ref align_x_str) = align_x {
        PlacementAnchor::AlignX {
            designator: align_x_str.clone(),
            offset: offset_point.x,
        }
    } else if let Some(ref align_y_str) = align_y {
        PlacementAnchor::AlignY {
            designator: align_y_str.clone(),
            offset: offset_point.y,
        }
    } else if let Some(ref edge_str) = edge {
        let board_edge = BoardEdge::try_parse(edge_str).ok_or_else(|| {
            format!(
                "Invalid edge: '{}'. Use: left, right, top, bottom",
                edge_str
            )
        })?;
        PlacementAnchor::BoardEdge {
            edge: board_edge,
            offset: offset_point.x, // Use X offset for edge alignment
        }
    } else if rotation.is_some() || layer.is_some() {
        // Just updating rotation or layer, keep current position
        if let Some(ref pos) = current_pos {
            PlacementAnchor::Absolute(CoordPoint::new(pos.x, pos.y))
        } else {
            return Err("No position specified and component has no current position".to_string());
        }
    } else {
        return Err(
            "No position specified. Use --at, --near, --align-x, --align-y, or --edge".to_string(),
        );
    };

    // Resolve the target position
    let target_pos = engine.resolve_anchor(&pcb, &anchor, current_pos.as_ref())?;

    // Update the component
    {
        let component = pcb
            .find_component_mut(designator)
            .ok_or_else(|| format!("Component '{}' not found", designator))?;

        component.set_position(target_pos.x, target_pos.y);

        if let Some(rot) = rotation {
            component.set_rotation(rot);
        }

        if let Some(ref layer_str) = layer {
            let new_layer = Layer::from_name(layer_str).ok_or_else(|| {
                format!(
                    "Invalid layer: '{}'. Use: TopLayer, BottomLayer, etc.",
                    layer_str
                )
            })?;
            component.set_layer(new_layer);
        }
    }

    // Save the file
    pcb.save_with_components(path)
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    // Print result
    println!("Moved component '{}' in {}", designator, path.display());
    println!(
        "  New position: {:.4}mm, {:.4}mm",
        target_pos.x.to_mms(),
        target_pos.y.to_mms()
    );
    if let Some(rot) = rotation {
        println!("  Rotation: {:.1} degrees", rot);
    }
    if let Some(ref layer_str) = layer {
        println!("  Layer: {}", layer_str);
    }
    if connected.has_connections() && force {
        println!(
            "\n  Warning: {} connected routes may need to be re-routed.",
            connected.count()
        );
    }

    Ok(())
}

/// Add a component from schematic.
pub fn cmd_add_component(
    path: &Path,
    schematic: &Path,
    designator: &str,
    _footprint_lib: Option<PathBuf>,
    footprint: Option<String>,
    at: Option<String>,
    layer: &str,
) -> Result<(), String> {
    // Open the schematic to find component info
    let sch_file = File::open(schematic).map_err(|e| format!("Error opening schematic: {}", e))?;
    let sch = crate::io::SchDoc::open(BufReader::new(sch_file))
        .map_err(|e| format!("Error parsing schematic: {:?}", e))?;

    // Find the component in the schematic by looking through designator records
    let mut found_component: Option<&crate::records::sch::SchComponent> = None;
    let mut component_comment = String::new();

    // First, find the designator record that matches
    for record in &sch.primitives {
        if let crate::records::sch::SchRecord::Designator(d) = record {
            if d.text().eq_ignore_ascii_case(designator) {
                // Found the designator, now find the component it belongs to
                let owner_index = d.param.label.graphical.base.owner_index;
                if owner_index >= 0 && (owner_index as usize) < sch.primitives.len() {
                    if let crate::records::sch::SchRecord::Component(c) =
                        &sch.primitives[owner_index as usize]
                    {
                        found_component = Some(c);
                    }
                }
                break;
            }
        }
    }

    // Also look for any comment/parameter with the value
    for record in &sch.primitives {
        if let crate::records::sch::SchRecord::Parameter(p) = record {
            if p.name.to_uppercase() == "VALUE" || p.name.to_uppercase() == "COMMENT" {
                if let Some(comp) = found_component {
                    if p.label.graphical.base.owner_index == comp.graphical.base.owner_index {
                        component_comment = p.value().to_string();
                    }
                }
            }
        }
    }

    let sch_component = found_component
        .ok_or_else(|| format!("Component '{}' not found in schematic", designator))?;

    // Get footprint from command line (schematic footprint info is in a separate record)
    let footprint_name = footprint.ok_or_else(|| {
        format!(
            "No footprint specified for component '{}'. Use --footprint.",
            designator
        )
    })?;

    // Open the PCB document
    let mut pcb = open_pcbdoc(path)?;

    // Check if component already exists
    if pcb.find_component(designator).is_some() {
        return Err(format!("Component '{}' already exists in PCB", designator));
    }

    // Parse initial position
    let position = if let Some(ref at_str) = at {
        parse_position(at_str)?
    } else {
        CoordPoint::from_mms(25.0, 25.0) // Default position
    };

    // Parse layer
    let pcb_layer = Layer::from_name(layer)
        .or_else(|| match layer.to_lowercase().as_str() {
            "top" => Some(Layer::TOP_LAYER),
            "bottom" => Some(Layer::BOTTOM_LAYER),
            _ => None,
        })
        .ok_or_else(|| {
            format!(
                "Invalid layer: '{}'. Use: TOP, BOTTOM, TopLayer, BottomLayer",
                layer
            )
        })?;

    // Create a new component
    let mut params = crate::types::ParameterCollection::new();
    params.add("SELECTION", "FALSE");
    params.add("LAYER", pcb_layer.name());
    params.add("LOCKED", "FALSE");
    params.add("POLYGONOUTLINE", "FALSE");
    params.add("USERROUTED", "TRUE");
    params.add("KEEPOUT", "FALSE");
    params.add("PRIMITIVELOCK", "FALSE");
    params.add_coord("X", position.x);
    params.add_coord("Y", position.y);
    params.add("PATTERN", &footprint_name);
    params.add("NAMEON", "TRUE");
    params.add("COMMENTON", "TRUE");
    params.add("GROUPNUM", "0");
    params.add("COUNT", "0");
    params.add("ROTATION", "0.00000000000000E+0000");
    params.add("SOURCEDESIGNATOR", designator);
    params.add("SOURCELIBREFERENCE", &sch_component.lib_reference);
    params.add(
        "UNIQUEID",
        &format!(
            "{:08X}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as u32
        ),
    );

    if !component_comment.is_empty() {
        params.add("COMMENT", &component_comment);
    }

    let new_component = crate::io::PcbDocComponent {
        designator: designator.to_string(),
        pattern: footprint_name.clone(),
        comment: component_comment.clone(),
        params,
        primitives: Vec::new(),
    };

    pcb.components.push(new_component);

    // Save the file
    pcb.save_with_components(path)
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!("Added component '{}' to {}", designator, path.display());
    println!("  Footprint: {}", footprint_name);
    println!(
        "  Position:  {:.4}mm, {:.4}mm",
        position.x.to_mms(),
        position.y.to_mms()
    );
    println!("  Layer:     {}", pcb_layer.name());
    if !component_comment.is_empty() {
        println!("  Comment:   {}", component_comment);
    }
    println!("\nNote: Component pads need to be populated from a footprint library.");

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// CREATION COMMAND IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Embedded blank PcbDoc template.
/// This is the binary content of data/blank/PCB1.PcbDoc.
const BLANK_PCBDOC_TEMPLATE: &[u8] = include_bytes!("../../data/PCB1.PcbDoc");

/// Create a new empty PcbDoc file.
pub fn cmd_create(path: &Path, template: Option<PathBuf>) -> Result<(), String> {
    if path.exists() {
        return Err(format!("File already exists: {}", path.display()));
    }

    match template {
        Some(template_path) => {
            // Copy from user-specified template
            std::fs::copy(&template_path, path)
                .map_err(|e| format!("Error copying template: {}", e))?;
            println!("Created PcbDoc from template: {}", path.display());
            println!("  Template: {}", template_path.display());
        }
        None => {
            // Use embedded blank template
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
// BOARD OUTLINE COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Display the board outline.
pub fn cmd_outline(path: &Path, _json: bool) -> Result<PcbDocOutline, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;
    let board = PcbBoard::from_params(&pcb.board_params);

    let vertices: Vec<OutlineVertex> = board
        .outline
        .iter()
        .map(|v| {
            let is_arc = matches!(v.kind, PolygonVertexKind::Arc);
            OutlineVertex {
                x_mm: v.x.to_mms(),
                y_mm: v.y.to_mms(),
                kind: match v.kind {
                    PolygonVertexKind::Line => "line".to_string(),
                    PolygonVertexKind::Arc => "arc".to_string(),
                },
                center_x_mm: if is_arc {
                    Some(v.center_x.to_mms())
                } else {
                    None
                },
                center_y_mm: if is_arc {
                    Some(v.center_y.to_mms())
                } else {
                    None
                },
                radius_mm: if is_arc {
                    Some(v.radius.to_mms())
                } else {
                    None
                },
            }
        })
        .collect();

    let (width, height) = calculate_outline_dimensions(&board.outline);

    Ok(PcbDocOutline {
        vertex_count: board.outline.len(),
        width_mm: width,
        height_mm: height,
        vertices,
    })
}

/// Calculate width and height from outline vertices.
fn calculate_outline_dimensions(vertices: &[PolygonVertex]) -> (f64, f64) {
    if vertices.is_empty() {
        return (0.0, 0.0);
    }

    let min_x = vertices
        .iter()
        .map(|v| v.x.to_mms())
        .fold(f64::INFINITY, f64::min);
    let max_x = vertices
        .iter()
        .map(|v| v.x.to_mms())
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = vertices
        .iter()
        .map(|v| v.y.to_mms())
        .fold(f64::INFINITY, f64::min);
    let max_y = vertices
        .iter()
        .map(|v| v.y.to_mms())
        .fold(f64::NEG_INFINITY, f64::max);

    (max_x - min_x, max_y - min_y)
}

/// Set board outline to a rectangle.
pub fn cmd_set_outline_rect(
    path: &Path,
    width: &str,
    height: &str,
    origin_x: &str,
    origin_y: &str,
) -> Result<(), String> {
    let w = parse_coord(width)?;
    let h = parse_coord(height)?;
    let ox = parse_coord(origin_x)?;
    let oy = parse_coord(origin_y)?;

    let mut pcb = open_pcbdoc(path)?;

    // Create rectangular outline
    let vertices = vec![
        PolygonVertex {
            kind: PolygonVertexKind::Line,
            x: ox,
            y: oy,
            ..Default::default()
        },
        PolygonVertex {
            kind: PolygonVertexKind::Line,
            x: Coord::from_raw(ox.to_raw() + w.to_raw()),
            y: oy,
            ..Default::default()
        },
        PolygonVertex {
            kind: PolygonVertexKind::Line,
            x: Coord::from_raw(ox.to_raw() + w.to_raw()),
            y: Coord::from_raw(oy.to_raw() + h.to_raw()),
            ..Default::default()
        },
        PolygonVertex {
            kind: PolygonVertexKind::Line,
            x: ox,
            y: Coord::from_raw(oy.to_raw() + h.to_raw()),
            ..Default::default()
        },
    ];

    // Update board params with new outline
    update_board_outline(&mut pcb.board_params, &vertices);

    // Save the file
    pcb.save_board_to_file(path)
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!(
        "Set board outline to rectangle: {:.3}mm x {:.3}mm",
        w.to_mms(),
        h.to_mms()
    );
    println!("  Origin: ({:.3}mm, {:.3}mm)", ox.to_mms(), oy.to_mms());

    Ok(())
}

/// Set board outline from vertices.
pub fn cmd_set_outline(path: &Path, vertices_str: &str) -> Result<(), String> {
    let mut vertices = Vec::new();

    for part in vertices_str.split_whitespace() {
        let coords: Vec<&str> = part.split(',').collect();
        if coords.len() != 2 {
            return Err(format!(
                "Invalid vertex format: '{}'. Use 'x,y' format.",
                part
            ));
        }

        let x = parse_coord(coords[0])?;
        let y = parse_coord(coords[1])?;

        vertices.push(PolygonVertex {
            kind: PolygonVertexKind::Line,
            x,
            y,
            ..Default::default()
        });
    }

    if vertices.len() < 3 {
        return Err("Board outline requires at least 3 vertices.".to_string());
    }

    let mut pcb = open_pcbdoc(path)?;
    update_board_outline(&mut pcb.board_params, &vertices);
    pcb.save_board_to_file(path)
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!("Set board outline with {} vertices", vertices.len());

    Ok(())
}

/// Update board params with outline vertices.
fn update_board_outline(
    params: &mut crate::types::ParameterCollection,
    vertices: &[PolygonVertex],
) {
    // First, remove any existing vertex params
    let mut idx = 0;
    loop {
        let vx_key = format!("VX{}", idx);
        if !params.contains(&vx_key) {
            break;
        }
        params.remove(&vx_key);
        params.remove(&format!("VY{}", idx));
        params.remove(&format!("KIND{}", idx));
        params.remove(&format!("CX{}", idx));
        params.remove(&format!("CY{}", idx));
        params.remove(&format!("SA{}", idx));
        params.remove(&format!("EA{}", idx));
        params.remove(&format!("R{}", idx));
        idx += 1;
    }

    // Add new vertices
    for (i, v) in vertices.iter().enumerate() {
        params.add_int(&format!("KIND{}", i), v.kind.to_int());
        params.add_coord(&format!("VX{}", i), v.x);
        params.add_coord(&format!("VY{}", i), v.y);
        params.add_coord(&format!("CX{}", i), v.center_x);
        params.add_coord(&format!("CY{}", i), v.center_y);
        params.add_double(&format!("SA{}", i), v.start_angle, 14);
        params.add_double(&format!("EA{}", i), v.end_angle, 14);
        params.add_coord(&format!("R{}", i), v.radius);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// BOARD SETTINGS COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Display board settings.
pub fn cmd_settings(
    path: &Path,
    _json: bool,
) -> Result<PcbDocSettings, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;
    let board = PcbBoard::from_params(&pcb.board_params);

    let unit = if board.is_metric() { "mm" } else { "mil" };

    Ok(PcbDocSettings {
        display_unit: unit.to_string(),
        snap_grid: format!("{:.6}{}", board.snap_grid_size, unit),
        visible_grid: format!("{:.6}{}", board.visible_grid_size, unit),
        component_grid: format!("{:.6}{}", board.component_grid_size, unit),
        track_grid: Some(format!("{:.6}{}", board.track_grid_size, unit)),
        via_grid: Some(format!("{:.6}{}", board.via_grid_size, unit)),
        track_width: Some(format!("{:.3}mm", board.track_width.to_mms())),
        origin_x: format!("{:.3}mm", board.origin_x.to_mms()),
        origin_y: format!("{:.3}mm", board.origin_y.to_mms()),
    })
}

/// Modify board settings.
#[allow(clippy::too_many_arguments)]
pub fn cmd_set_settings(
    path: &Path,
    metric: bool,
    imperial: bool,
    snap_grid: Option<String>,
    visible_grid: Option<String>,
    component_grid: Option<String>,
    track_grid: Option<String>,
    via_grid: Option<String>,
    track_width: Option<String>,
    origin_x: Option<String>,
    origin_y: Option<String>,
) -> Result<(), String> {
    use crate::records::pcb::DisplayUnit;

    let mut pcb = open_pcbdoc(path)?;
    let mut changes = Vec::new();

    if metric && imperial {
        return Err("Cannot use both --metric and --imperial".to_string());
    }

    if metric {
        pcb.board_params
            .add_int("DISPLAYUNIT", DisplayUnit::Metric.to_int());
        changes.push("display_unit=metric".to_string());
    }
    if imperial {
        pcb.board_params
            .add_int("DISPLAYUNIT", DisplayUnit::Imperial.to_int());
        changes.push("display_unit=imperial".to_string());
    }

    if let Some(ref v) = snap_grid {
        let coord = parse_coord(v)?;
        let val = if v.contains("mm") {
            coord.to_mms()
        } else {
            coord.to_mils()
        };
        pcb.board_params.add_double("SNAPGRIDSIZE", val, 6);
        pcb.board_params.add_double("SNAPGRIDSIZEX", val, 6);
        pcb.board_params.add_double("SNAPGRIDSIZEY", val, 6);
        changes.push(format!("snap_grid={}", v));
    }

    if let Some(ref v) = visible_grid {
        let coord = parse_coord(v)?;
        let val = if v.contains("mm") {
            coord.to_mms()
        } else {
            coord.to_mils()
        };
        pcb.board_params.add_double("VISIBLEGRIDSIZE", val, 6);
        changes.push(format!("visible_grid={}", v));
    }

    if let Some(ref v) = component_grid {
        let coord = parse_coord(v)?;
        let val = if v.contains("mm") {
            coord.to_mms()
        } else {
            coord.to_mils()
        };
        pcb.board_params.add_double("COMPONENTGRIDSIZE", val, 6);
        changes.push(format!("component_grid={}", v));
    }

    if let Some(ref v) = track_grid {
        let coord = parse_coord(v)?;
        let val = if v.contains("mm") {
            coord.to_mms()
        } else {
            coord.to_mils()
        };
        pcb.board_params.add_double("TRACKGRIDSIZE", val, 6);
        changes.push(format!("track_grid={}", v));
    }

    if let Some(ref v) = via_grid {
        let coord = parse_coord(v)?;
        let val = if v.contains("mm") {
            coord.to_mms()
        } else {
            coord.to_mils()
        };
        pcb.board_params.add_double("VIAGRIDSIZE", val, 6);
        changes.push(format!("via_grid={}", v));
    }

    if let Some(ref v) = track_width {
        let coord = parse_coord(v)?;
        pcb.board_params.add_coord("TRACKWIDTH", coord);
        changes.push(format!("track_width={}", v));
    }

    if let Some(ref v) = origin_x {
        let coord = parse_coord(v)?;
        pcb.board_params.add_coord("ORIGINX", coord);
        changes.push(format!("origin_x={}", v));
    }

    if let Some(ref v) = origin_y {
        let coord = parse_coord(v)?;
        pcb.board_params.add_coord("ORIGINY", coord);
        changes.push(format!("origin_y={}", v));
    }

    if changes.is_empty() {
        println!("No changes specified");
        return Ok(());
    }

    pcb.save_board_to_file(path)
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!("Modified board settings in {}", path.display());
    for change in changes {
        println!("  {}", change);
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// LAYER STACK COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Display the layer stack.
pub fn cmd_layers(path: &Path, all: bool) -> Result<PcbDocLayers, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    // Collect layers used in primitives
    let mut used_layers: std::collections::HashSet<u8> = std::collections::HashSet::new();
    for prim in &pcb.primitives {
        match prim {
            crate::records::pcb::PcbRecord::Track(t) => {
                used_layers.insert(t.common.layer.to_byte());
            }
            crate::records::pcb::PcbRecord::Arc(a) => {
                used_layers.insert(a.common.layer.to_byte());
            }
            crate::records::pcb::PcbRecord::Via(_) => {
                used_layers.insert(Layer::MULTI_LAYER.to_byte());
            }
            crate::records::pcb::PcbRecord::Fill(f) => {
                used_layers.insert(f.base.common.layer.to_byte());
            }
            crate::records::pcb::PcbRecord::Region(r) => {
                used_layers.insert(r.common.layer.to_byte());
            }
            _ => {}
        }
    }

    // Standard layer categories
    let signal_layers = [
        (Layer::TOP_LAYER, "Top Layer", "signal"),
        (Layer::MID_LAYER_1, "Mid Layer 1", "signal"),
        (Layer::MID_LAYER_2, "Mid Layer 2", "signal"),
        (Layer::BOTTOM_LAYER, "Bottom Layer", "signal"),
    ];

    let plane_layers = [
        (Layer::INTERNAL_PLANE_1, "Internal Plane 1", "plane"),
        (Layer::INTERNAL_PLANE_2, "Internal Plane 2", "plane"),
    ];

    let mask_layers = [
        (Layer::TOP_SOLDER, "Top Solder Mask", "mask"),
        (Layer::BOTTOM_SOLDER, "Bottom Solder Mask", "mask"),
        (Layer::TOP_PASTE, "Top Paste", "mask"),
        (Layer::BOTTOM_PASTE, "Bottom Paste", "mask"),
    ];

    let silk_layers = [
        (Layer::TOP_OVERLAY, "Top Silkscreen", "silkscreen"),
        (Layer::BOTTOM_OVERLAY, "Bottom Silkscreen", "silkscreen"),
    ];

    let mech_layers = [
        (Layer::MECHANICAL_1, "Mechanical 1", "mechanical"),
        (Layer::MECHANICAL_2, "Mechanical 2", "mechanical"),
        (Layer::MECHANICAL_3, "Mechanical 3", "mechanical"),
        (Layer::MECHANICAL_4, "Mechanical 4", "mechanical"),
    ];

    let special_layers = [
        (Layer::KEEP_OUT_LAYER, "Keep-Out Layer", "special"),
        (Layer::MULTI_LAYER, "Multi-Layer", "special"),
        (Layer::DRILL_GUIDE, "Drill Guide", "special"),
        (Layer::DRILL_DRAWING, "Drill Drawing", "special"),
    ];

    let mut layers: Vec<LayerInfo> = Vec::new();

    for (layer, name, layer_type) in signal_layers
        .iter()
        .chain(plane_layers.iter())
        .chain(mask_layers.iter())
        .chain(silk_layers.iter())
        .chain(mech_layers.iter())
        .chain(special_layers.iter())
    {
        let is_used = used_layers.contains(&layer.to_byte());
        if all || is_used {
            layers.push(LayerInfo {
                id: layer.to_byte(),
                name: name.to_string(),
                layer_type: layer_type.to_string(),
                used: is_used,
                enabled: true,
                copper_thickness: None,
                dielectric_constant: None,
                dielectric_thickness: None,
            });
        }
    }

    Ok(PcbDocLayers {
        path: path.display().to_string(),
        total_layers: layers.len(),
        show_all: all,
        layers,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// KEEPOUT COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// List all keepout regions.
pub fn cmd_keepouts(
    path: &Path,
    layer_filter: Option<String>,
) -> Result<PcbDocKeepouts, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    // Find keepout regions
    let mut keepouts: Vec<(usize, &crate::records::pcb::PcbRegion)> = Vec::new();
    for (i, prim) in pcb.primitives.iter().enumerate() {
        if let crate::records::pcb::PcbRecord::Region(r) = prim {
            // Check if this is a keepout (on KeepOutLayer or has keepout flag)
            if r.common.layer == Layer::KEEP_OUT_LAYER || r.common.is_keepout() {
                if let Some(ref filter) = layer_filter {
                    let filter_layer = Layer::from_name(filter);
                    if let Some(fl) = filter_layer {
                        if r.common.layer != fl {
                            continue;
                        }
                    }
                }
                keepouts.push((i, r));
            }
        }
    }

    let keepout_infos: Vec<KeepoutInfo> = keepouts
        .iter()
        .map(|(i, r)| {
            let bounds = r.calculate_bounds();
            KeepoutInfo {
                index: *i,
                layer: r.common.layer.name().to_string(),
                x1: format!("{:.4}mm", bounds.location1.x.to_mms()),
                y1: format!("{:.4}mm", bounds.location1.y.to_mms()),
                x2: format!("{:.4}mm", bounds.location2.x.to_mms()),
                y2: format!("{:.4}mm", bounds.location2.y.to_mms()),
                kind: "region".to_string(),
            }
        })
        .collect();

    Ok(PcbDocKeepouts {
        path: path.display().to_string(),
        total_keepouts: keepout_infos.len(),
        layer_filter,
        keepouts: keepout_infos,
    })
}

/// Add a rectangular keepout region.
pub fn cmd_add_keepout(
    path: &Path,
    layer_str: &str,
    x1: &str,
    y1: &str,
    x2: &str,
    y2: &str,
) -> Result<(), String> {
    let layer = Layer::from_name(layer_str)
        .ok_or_else(|| format!("Unknown layer: '{}'. Valid layers: TopLayer, BottomLayer, KeepOutLayer, MultiLayer, etc.", layer_str))?;

    let x1_coord = parse_coord(x1)?;
    let y1_coord = parse_coord(y1)?;
    let x2_coord = parse_coord(x2)?;
    let y2_coord = parse_coord(y2)?;

    let mut pcb = open_pcbdoc(path)?;

    // Create rectangular keepout region
    let region = crate::records::pcb::PcbRegion {
        common: crate::records::pcb::PcbPrimitiveCommon {
            layer,
            flags: crate::records::pcb::PcbFlags::KEEPOUT,
            ..Default::default()
        },
        parameters: crate::types::ParameterCollection::new(),
        outline: vec![
            crate::types::CoordPoint {
                x: x1_coord,
                y: y1_coord,
            },
            crate::types::CoordPoint {
                x: x2_coord,
                y: y1_coord,
            },
            crate::types::CoordPoint {
                x: x2_coord,
                y: y2_coord,
            },
            crate::types::CoordPoint {
                x: x1_coord,
                y: y2_coord,
            },
        ],
    };

    pcb.primitives
        .push(crate::records::pcb::PcbRecord::Region(region));

    // Save the file
    pcb.save_regions_to_file(path)
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!(
        "Added keepout region on {} at ({:.3}mm, {:.3}mm) to ({:.3}mm, {:.3}mm)",
        layer.name(),
        x1_coord.to_mms(),
        y1_coord.to_mms(),
        x2_coord.to_mms(),
        y2_coord.to_mms()
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// CUTOUT COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// List all board cutouts.
pub fn cmd_cutouts(path: &Path) -> Result<PcbDocCutouts, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    // Find cutout regions (regions on MultiLayer that are cutouts)
    let mut cutouts: Vec<(usize, &crate::records::pcb::PcbRegion)> = Vec::new();
    for (i, prim) in pcb.primitives.iter().enumerate() {
        if let crate::records::pcb::PcbRecord::Region(r) = prim {
            // Check if this is a cutout (on multi-layer with specific parameters)
            // Cutouts are typically on multi-layer or have polygon_type=Cutout
            if r.common.layer == Layer::MULTI_LAYER && !r.common.is_keepout() {
                cutouts.push((i, r));
            }
        }
    }

    let cutout_infos: Vec<CutoutInfo> = cutouts
        .iter()
        .map(|(i, r)| {
            let bounds = r.calculate_bounds();
            CutoutInfo {
                index: *i,
                vertex_count: r.outline.len(),
                bounds: BoundsInfo {
                    x1: format!("{:.4}mm", bounds.location1.x.to_mms()),
                    y1: format!("{:.4}mm", bounds.location1.y.to_mms()),
                    x2: format!("{:.4}mm", bounds.location2.x.to_mms()),
                    y2: format!("{:.4}mm", bounds.location2.y.to_mms()),
                },
            }
        })
        .collect();

    Ok(PcbDocCutouts {
        path: path.display().to_string(),
        total_cutouts: cutout_infos.len(),
        cutouts: cutout_infos,
    })
}

/// Add a rectangular board cutout.
pub fn cmd_add_cutout(path: &Path, x1: &str, y1: &str, x2: &str, y2: &str) -> Result<(), String> {
    let x1_coord = parse_coord(x1)?;
    let y1_coord = parse_coord(y1)?;
    let x2_coord = parse_coord(x2)?;
    let y2_coord = parse_coord(y2)?;

    let mut pcb = open_pcbdoc(path)?;

    // Create rectangular cutout region on MultiLayer
    let region = crate::records::pcb::PcbRegion {
        common: crate::records::pcb::PcbPrimitiveCommon {
            layer: Layer::MULTI_LAYER,
            ..Default::default()
        },
        parameters: crate::types::ParameterCollection::new(),
        outline: vec![
            crate::types::CoordPoint {
                x: x1_coord,
                y: y1_coord,
            },
            crate::types::CoordPoint {
                x: x2_coord,
                y: y1_coord,
            },
            crate::types::CoordPoint {
                x: x2_coord,
                y: y2_coord,
            },
            crate::types::CoordPoint {
                x: x1_coord,
                y: y2_coord,
            },
        ],
    };

    pcb.primitives
        .push(crate::records::pcb::PcbRecord::Region(region));

    // Save the file
    pcb.save_regions_to_file(path)
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!(
        "Added board cutout at ({:.3}mm, {:.3}mm) to ({:.3}mm, {:.3}mm)",
        x1_coord.to_mms(),
        y1_coord.to_mms(),
        x2_coord.to_mms(),
        y2_coord.to_mms()
    );
    let width_mm = ((x2_coord.to_raw() - x1_coord.to_raw()).abs() as f64) / 10000.0 * 0.0254;
    let height_mm = ((y2_coord.to_raw() - y1_coord.to_raw()).abs() as f64) / 10000.0 * 0.0254;
    println!("  Size: {:.3}mm x {:.3}mm", width_mm, height_mm);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// POLYGON (COPPER POUR) COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// List all polygons (copper pours).
pub fn cmd_polygons(
    path: &Path,
    layer_filter: Option<String>,
    net_filter: Option<String>,
) -> Result<PcbDocPolygons, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    // Find all polygons
    let mut polygons: Vec<(usize, &PcbPolygon)> = Vec::new();
    for (i, prim) in pcb.primitives.iter().enumerate() {
        if let crate::records::pcb::PcbRecord::Polygon(p) = prim {
            // Apply filters
            if let Some(ref filter) = layer_filter {
                let filter_layer = Layer::from_name(filter);
                if let Some(fl) = filter_layer {
                    if p.layer != fl {
                        continue;
                    }
                }
            }
            if let Some(ref filter) = net_filter {
                if !p.net_name.eq_ignore_ascii_case(filter) {
                    continue;
                }
            }
            polygons.push((i, p));
        }
    }

    let polygon_summaries: Vec<PolygonSummary> = polygons
        .iter()
        .map(|(i, p)| PolygonSummary {
            index: *i,
            layer: p.layer.name().to_string(),
            net: p.net_name.clone(),
            vertex_count: p.vertices.len(),
            pour_over: p.pour_over,
            remove_dead: p.remove_dead,
            hatch_style: p.hatch_style.as_str().to_string(),
        })
        .collect();

    Ok(PcbDocPolygons {
        path: path.display().to_string(),
        total_polygons: polygon_summaries.len(),
        layer_filter,
        net_filter,
        polygons: polygon_summaries,
    })
}

/// Show details for a specific polygon.
pub fn cmd_polygon(
    path: &Path,
    index: usize,
) -> Result<PcbDocPolygonDetail, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    // Find the polygon at the specified index
    let mut polygon: Option<&PcbPolygon> = None;
    let mut poly_index = 0;
    for prim in &pcb.primitives {
        if let crate::records::pcb::PcbRecord::Polygon(p) = prim {
            if poly_index == index {
                polygon = Some(p);
                break;
            }
            poly_index += 1;
        }
    }

    let p = polygon.ok_or_else(|| format!("Polygon index {} not found", index))?;

    let vertices: Vec<PolygonVertexInfo> = p
        .vertices
        .iter()
        .map(|v| PolygonVertexInfo {
            x: format!("{:.4}mm", v.x.to_mms()),
            y: format!("{:.4}mm", v.y.to_mms()),
            kind: match v.kind {
                PolygonVertexKind::Line => "line".to_string(),
                PolygonVertexKind::Arc => "arc".to_string(),
            },
        })
        .collect();

    Ok(PcbDocPolygonDetail {
        index,
        layer: p.layer.name().to_string(),
        net: p.net_name.clone(),
        vertex_count: p.vertices.len(),
        pour_over: p.pour_over,
        remove_dead: p.remove_dead,
        hatch_style: p.hatch_style.as_str().to_string(),
        vertices,
    })
}

/// Add a polygon (copper pour) to the PCB.
pub fn cmd_add_polygon(
    path: &Path,
    layer_str: &str,
    net: &str,
    vertices_str: &str,
    pour_over: bool,
    remove_dead: bool,
    hatch_style_str: &str,
) -> Result<(), String> {
    let layer = Layer::from_name(layer_str).ok_or_else(|| {
        format!(
            "Unknown layer: '{}'. Valid layers: TopLayer, BottomLayer, InternalPlane1, etc.",
            layer_str
        )
    })?;

    let hatch_style = HatchStyle::parse(hatch_style_str);

    // Parse vertices from string "x1,y1 x2,y2 x3,y3 ..."
    let mut vertices: Vec<PolygonVertex> = Vec::new();
    for vertex_str in vertices_str.split_whitespace() {
        let parts: Vec<&str> = vertex_str.split(',').collect();
        if parts.len() != 2 {
            return Err(format!(
                "Invalid vertex format: '{}'. Expected 'x,y'",
                vertex_str
            ));
        }
        let x = parse_coord(parts[0])?;
        let y = parse_coord(parts[1])?;
        vertices.push(PolygonVertex {
            kind: PolygonVertexKind::Line,
            x,
            y,
            center_x: Coord::default(),
            center_y: Coord::default(),
            start_angle: 0.0,
            end_angle: 0.0,
            radius: Coord::default(),
        });
    }

    if vertices.len() < 3 {
        return Err("At least 3 vertices are required to create a polygon".to_string());
    }

    let vertex_count = vertices.len();

    let mut pcb = open_pcbdoc(path)?;

    // Get default board settings for polygon defaults
    let board = PcbBoard::from_params(&pcb.board_params);

    // Create polygon
    let polygon = PcbPolygon {
        layer,
        net_name: net.to_string(),
        vertices,
        polygon_type: crate::records::pcb::PolygonType::Polygon,
        hatch_style,
        pour_over,
        remove_dead,
        grid_size: board.grid_size,
        track_width: board.track_width,
        use_octagons: board.use_octagons,
        min_prim_length: board.min_prim_length,
        locked: false,
        polygon_outline: false,
        user_routed: true,
        keepout: false,
        union_index: -1,
        primitive_lock: false,
        unique_id: String::new(),
        params: crate::types::ParameterCollection::new(),
    };

    pcb.primitives
        .push(crate::records::pcb::PcbRecord::Polygon(polygon));

    // Save the file
    pcb.save_polygons_to_file(path)
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!(
        "Added polygon (copper pour) on {} for net '{}'",
        layer.name(),
        net
    );
    println!("  Vertices:      {}", vertex_count);
    println!("  Hatch Style:   {}", hatch_style_str);
    println!("  Pour Over:     {}", pour_over);
    println!("  Remove Dead:   {}", remove_dead);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TRACK COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// List all tracks.
pub fn cmd_tracks(
    path: &Path,
    layer_filter: Option<String>,
) -> Result<PcbDocTracks, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    let layer = layer_filter.as_ref().and_then(|l| Layer::from_name(l));

    let tracks: Vec<_> = pcb
        .iter_tracks()
        .filter(|t| layer.is_none_or(|l| t.common.layer == l))
        .collect();

    let track_infos: Vec<TrackInfo> = tracks
        .iter()
        .enumerate()
        .map(|(i, t)| TrackInfo {
            index: i,
            layer: t.common.layer.name().to_string(),
            start_x: format!("{:.4}mm", t.start.x.to_mms()),
            start_y: format!("{:.4}mm", t.start.y.to_mms()),
            end_x: format!("{:.4}mm", t.end.x.to_mms()),
            end_y: format!("{:.4}mm", t.end.y.to_mms()),
            width: format!("{:.4}mm", t.width.to_mms()),
            net: String::new(),
        })
        .collect();

    Ok(PcbDocTracks {
        path: path.display().to_string(),
        total_tracks: track_infos.len(),
        layer_filter,
        tracks: track_infos,
    })
}

/// Add a track.
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_track(
    path: &Path,
    start: Option<String>,
    end: Option<String>,
    start_pad: Option<String>,
    end_pad: Option<String>,
    width: Option<String>,
    layer_str: &str,
    net: Option<String>,
) -> Result<(), String> {
    use crate::edit::PcbEditSession;

    let mut session =
        PcbEditSession::open(path).map_err(|e| format!("Error opening file: {:?}", e))?;

    let layer =
        Layer::from_name(layer_str).ok_or_else(|| format!("Invalid layer: {}", layer_str))?;
    session.set_default_layer(layer);

    let track_width = width.as_ref().map(|w| parse_coord(w)).transpose()?;

    // Determine start position
    let start_point = if let Some(pad_ref) = &start_pad {
        let parts: Vec<&str> = pad_ref.split('.').collect();
        if parts.len() != 2 {
            return Err("Pad reference must be in format 'U1.1'".to_string());
        }
        session
            .resolve_position(&crate::edit::Position::RelativeToPad {
                component: parts[0].to_string(),
                pad: parts[1].to_string(),
                offset: CoordPoint::default(),
            })
            .map_err(|e| format!("Error resolving start pad: {:?}", e))?
    } else if let Some(start_str) = &start {
        let parts: Vec<&str> = start_str.split(',').collect();
        if parts.len() != 2 {
            return Err("Start position must be in format 'x,y'".to_string());
        }
        CoordPoint::new(parse_coord(parts[0])?, parse_coord(parts[1])?)
    } else {
        return Err("Either --start or --start-pad must be specified".to_string());
    };

    // Determine end position
    let end_point = if let Some(pad_ref) = &end_pad {
        let parts: Vec<&str> = pad_ref.split('.').collect();
        if parts.len() != 2 {
            return Err("Pad reference must be in format 'U1.1'".to_string());
        }
        session
            .resolve_position(&crate::edit::Position::RelativeToPad {
                component: parts[0].to_string(),
                pad: parts[1].to_string(),
                offset: CoordPoint::default(),
            })
            .map_err(|e| format!("Error resolving end pad: {:?}", e))?
    } else if let Some(end_str) = &end {
        let parts: Vec<&str> = end_str.split(',').collect();
        if parts.len() != 2 {
            return Err("End position must be in format 'x,y'".to_string());
        }
        CoordPoint::new(parse_coord(parts[0])?, parse_coord(parts[1])?)
    } else {
        return Err("Either --end or --end-pad must be specified".to_string());
    };

    let idx = session
        .add_track(
            start_point,
            end_point,
            track_width,
            Some(layer),
            net.as_deref(),
        )
        .map_err(|e| format!("Error adding track: {:?}", e))?;

    session
        .save_to_original()
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!("Added track at index {}", idx);
    println!("  Layer:  {}", layer.name());
    println!(
        "  Start:  ({:.3}mm, {:.3}mm)",
        start_point.x.to_mms(),
        start_point.y.to_mms()
    );
    println!(
        "  End:    ({:.3}mm, {:.3}mm)",
        end_point.x.to_mms(),
        end_point.y.to_mms()
    );
    println!(
        "  Width:  {:.3}mm",
        track_width.unwrap_or(Coord::from_mils(10.0)).to_mms()
    );
    if let Some(n) = &net {
        println!("  Net:    {}", n);
    }

    Ok(())
}

/// Add multiple connected track segments.
pub fn cmd_add_track_path(
    path: &Path,
    vertices_str: &str,
    width: Option<String>,
    layer_str: &str,
    net: Option<String>,
) -> Result<(), String> {
    use crate::edit::PcbEditSession;

    let mut session =
        PcbEditSession::open(path).map_err(|e| format!("Error opening file: {:?}", e))?;

    let layer =
        Layer::from_name(layer_str).ok_or_else(|| format!("Invalid layer: {}", layer_str))?;
    session.set_default_layer(layer);

    let track_width = width.as_ref().map(|w| parse_coord(w)).transpose()?;

    let mut vertices = Vec::new();
    for vertex_str in vertices_str.split_whitespace() {
        let parts: Vec<&str> = vertex_str.split(',').collect();
        if parts.len() != 2 {
            return Err(format!(
                "Invalid vertex format '{}', expected 'x,y'",
                vertex_str
            ));
        }
        vertices.push(CoordPoint::new(
            parse_coord(parts[0])?,
            parse_coord(parts[1])?,
        ));
    }

    if vertices.len() < 2 {
        return Err("At least 2 vertices are required for a track path".to_string());
    }

    let indices = session
        .add_track_path(&vertices, track_width, Some(layer), net.as_deref())
        .map_err(|e| format!("Error adding track path: {:?}", e))?;

    session
        .save_to_original()
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!(
        "Added {} track segments (indices {:?})",
        indices.len(),
        indices
    );
    println!("  Layer:    {}", layer.name());
    println!("  Vertices: {}", vertices.len());
    println!(
        "  Width:    {:.3}mm",
        track_width.unwrap_or(Coord::from_mils(10.0)).to_mms()
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// VIA COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// List all vias.
pub fn cmd_vias(path: &Path) -> Result<PcbDocVias, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    let vias: Vec<_> = pcb.iter_vias().collect();

    let via_infos: Vec<ViaInfo> = vias
        .iter()
        .enumerate()
        .map(|(i, v)| ViaInfo {
            index: i,
            x: format!("{:.4}mm", v.location.x.to_mms()),
            y: format!("{:.4}mm", v.location.y.to_mms()),
            diameter: format!("{:.4}mm", v.diameter().to_mms()),
            hole_size: format!("{:.4}mm", v.hole_size.to_mms()),
            from_layer: v.from_layer.name().to_string(),
            to_layer: v.to_layer.name().to_string(),
            net: String::new(),
        })
        .collect();

    Ok(PcbDocVias {
        path: path.display().to_string(),
        total_vias: via_infos.len(),
        vias: via_infos,
    })
}

/// Add a via.
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_via(
    path: &Path,
    at: Option<String>,
    at_pad: Option<String>,
    diameter: Option<String>,
    hole: Option<String>,
    from_layer_str: &str,
    to_layer_str: &str,
    net: Option<String>,
) -> Result<(), String> {
    use crate::edit::PcbEditSession;

    let mut session =
        PcbEditSession::open(path).map_err(|e| format!("Error opening file: {:?}", e))?;

    let from_layer = Layer::from_name(from_layer_str)
        .ok_or_else(|| format!("Invalid from layer: {}", from_layer_str))?;
    let to_layer = Layer::from_name(to_layer_str)
        .ok_or_else(|| format!("Invalid to layer: {}", to_layer_str))?;

    let via_diameter = diameter.as_ref().map(|d| parse_coord(d)).transpose()?;
    let via_hole = hole.as_ref().map(|h| parse_coord(h)).transpose()?;

    let location = if let Some(pad_ref) = &at_pad {
        let parts: Vec<&str> = pad_ref.split('.').collect();
        if parts.len() != 2 {
            return Err("Pad reference must be in format 'U1.1'".to_string());
        }
        session
            .resolve_position(&crate::edit::Position::RelativeToPad {
                component: parts[0].to_string(),
                pad: parts[1].to_string(),
                offset: CoordPoint::default(),
            })
            .map_err(|e| format!("Error resolving pad: {:?}", e))?
    } else if let Some(at_str) = &at {
        let parts: Vec<&str> = at_str.split(',').collect();
        if parts.len() != 2 {
            return Err("Position must be in format 'x,y'".to_string());
        }
        CoordPoint::new(parse_coord(parts[0])?, parse_coord(parts[1])?)
    } else {
        return Err("Either --at or --at-pad must be specified".to_string());
    };

    let idx = session
        .add_via(
            location,
            via_diameter,
            via_hole,
            Some(from_layer),
            Some(to_layer),
            net.as_deref(),
        )
        .map_err(|e| format!("Error adding via: {:?}", e))?;

    session
        .save_to_original()
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!("Added via at index {}", idx);
    println!(
        "  Position: ({:.3}mm, {:.3}mm)",
        location.x.to_mms(),
        location.y.to_mms()
    );
    println!(
        "  Diameter: {:.3}mm",
        via_diameter.unwrap_or(Coord::from_mils(50.0)).to_mms()
    );
    println!(
        "  Hole:     {:.3}mm",
        via_hole.unwrap_or(Coord::from_mils(28.0)).to_mms()
    );
    println!("  Layers:   {} -> {}", from_layer.name(), to_layer.name());

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// ARC COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// List all arcs.
pub fn cmd_arcs(
    path: &Path,
    layer_filter: Option<String>,
) -> Result<PcbDocArcs, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    let layer = layer_filter.as_ref().and_then(|l| Layer::from_name(l));

    let arcs: Vec<_> = pcb
        .iter_arcs()
        .filter(|a| layer.is_none_or(|l| a.common.layer == l))
        .collect();

    let arc_infos: Vec<ArcInfo> = arcs
        .iter()
        .enumerate()
        .map(|(i, a)| ArcInfo {
            index: i,
            layer: a.common.layer.name().to_string(),
            center_x: format!("{:.4}mm", a.location.x.to_mms()),
            center_y: format!("{:.4}mm", a.location.y.to_mms()),
            radius: format!("{:.4}mm", a.radius.to_mms()),
            start_angle: a.start_angle,
            end_angle: a.end_angle,
            width: format!("{:.4}mm", a.width.to_mms()),
            net: String::new(),
        })
        .collect();

    Ok(PcbDocArcs {
        path: path.display().to_string(),
        total_arcs: arc_infos.len(),
        layer_filter,
        arcs: arc_infos,
    })
}

/// Add an arc.
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_arc(
    path: &Path,
    center_str: &str,
    radius_str: &str,
    start_angle: f64,
    end_angle: f64,
    width: Option<String>,
    layer_str: &str,
    net: Option<String>,
) -> Result<(), String> {
    use crate::edit::PcbEditSession;

    let mut session =
        PcbEditSession::open(path).map_err(|e| format!("Error opening file: {:?}", e))?;

    let layer =
        Layer::from_name(layer_str).ok_or_else(|| format!("Invalid layer: {}", layer_str))?;
    session.set_default_layer(layer);

    let center_parts: Vec<&str> = center_str.split(',').collect();
    if center_parts.len() != 2 {
        return Err("Center must be in format 'x,y'".to_string());
    }
    let center = CoordPoint::new(parse_coord(center_parts[0])?, parse_coord(center_parts[1])?);
    let radius = parse_coord(radius_str)?;
    let arc_width = width.as_ref().map(|w| parse_coord(w)).transpose()?;

    let idx = session
        .add_arc(
            center,
            radius,
            start_angle,
            end_angle,
            arc_width,
            Some(layer),
            net.as_deref(),
        )
        .map_err(|e| format!("Error adding arc: {:?}", e))?;

    session
        .save_to_original()
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!("Added arc at index {}", idx);
    println!("  Layer:   {}", layer.name());
    println!(
        "  Center:  ({:.3}mm, {:.3}mm)",
        center.x.to_mms(),
        center.y.to_mms()
    );
    println!("  Radius:  {:.3}mm", radius.to_mms());
    println!("  Angles:  {:.1}deg - {:.1}deg", start_angle, end_angle);
    println!(
        "  Width:   {:.3}mm",
        arc_width.unwrap_or(Coord::from_mils(10.0)).to_mms()
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// FILL COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// List all fills.
pub fn cmd_fills(
    path: &Path,
    layer_filter: Option<String>,
) -> Result<PcbDocFills, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    let layer = layer_filter.as_ref().and_then(|l| Layer::from_name(l));

    let fills: Vec<_> = pcb
        .iter_fills()
        .filter(|f| layer.is_none_or(|l| f.base.common.layer == l))
        .collect();

    let fill_infos: Vec<FillInfo> = fills
        .iter()
        .enumerate()
        .map(|(i, f)| FillInfo {
            index: i,
            layer: f.base.common.layer.name().to_string(),
            x1: format!("{:.4}mm", f.base.corner1.x.to_mms()),
            y1: format!("{:.4}mm", f.base.corner1.y.to_mms()),
            x2: format!("{:.4}mm", f.base.corner2.x.to_mms()),
            y2: format!("{:.4}mm", f.base.corner2.y.to_mms()),
            rotation: f.base.rotation,
            net: String::new(),
        })
        .collect();

    Ok(PcbDocFills {
        path: path.display().to_string(),
        total_fills: fill_infos.len(),
        layer_filter,
        fills: fill_infos,
    })
}

/// Add a fill.
pub fn cmd_add_fill(
    path: &Path,
    x1y1_str: &str,
    x2y2_str: &str,
    layer_str: &str,
    rotation: f64,
    net: Option<String>,
) -> Result<(), String> {
    use crate::edit::PcbEditSession;

    let mut session =
        PcbEditSession::open(path).map_err(|e| format!("Error opening file: {:?}", e))?;

    let layer =
        Layer::from_name(layer_str).ok_or_else(|| format!("Invalid layer: {}", layer_str))?;

    let parts1: Vec<&str> = x1y1_str.split(',').collect();
    let parts2: Vec<&str> = x2y2_str.split(',').collect();
    if parts1.len() != 2 || parts2.len() != 2 {
        return Err("Coordinates must be in format 'x,y'".to_string());
    }

    let corner1 = CoordPoint::new(parse_coord(parts1[0])?, parse_coord(parts1[1])?);
    let corner2 = CoordPoint::new(parse_coord(parts2[0])?, parse_coord(parts2[1])?);

    let idx = session
        .add_fill(
            corner1,
            corner2,
            Some(layer),
            Some(rotation),
            net.as_deref(),
        )
        .map_err(|e| format!("Error adding fill: {:?}", e))?;

    session
        .save_to_original()
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!("Added fill at index {}", idx);
    println!("  Layer:    {}", layer.name());
    println!(
        "  Corner 1: ({:.3}mm, {:.3}mm)",
        corner1.x.to_mms(),
        corner1.y.to_mms()
    );
    println!(
        "  Corner 2: ({:.3}mm, {:.3}mm)",
        corner2.x.to_mms(),
        corner2.y.to_mms()
    );
    println!("  Rotation: {:.1}deg", rotation);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TEXT COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// List all text annotations.
pub fn cmd_texts(
    path: &Path,
    layer_filter: Option<String>,
) -> Result<PcbDocTexts, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    let layer = layer_filter.as_ref().and_then(|l| Layer::from_name(l));

    let texts: Vec<_> = pcb
        .primitives
        .iter()
        .filter_map(|p| {
            if let crate::records::pcb::PcbRecord::Text(t) = p {
                if layer.is_none_or(|l| t.base.common.layer == l) {
                    Some(t)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let text_infos: Vec<TextInfo> = texts
        .iter()
        .enumerate()
        .map(|(i, t)| TextInfo {
            index: i,
            text: t.text.clone(),
            layer: t.base.common.layer.name().to_string(),
            x: format!("{:.4}mm", t.base.corner1.x.to_mms()),
            y: format!("{:.4}mm", t.base.corner1.y.to_mms()),
            height: format!("{:.4}mm", t.height().to_mms()),
            rotation: t.base.rotation,
        })
        .collect();

    Ok(PcbDocTexts {
        path: path.display().to_string(),
        total_texts: text_infos.len(),
        layer_filter,
        texts: text_infos,
    })
}

/// Add text annotation.
pub fn cmd_add_text(
    path: &Path,
    text: &str,
    at_str: &str,
    height: Option<String>,
    layer_str: &str,
    rotation: f64,
) -> Result<(), String> {
    use crate::edit::PcbEditSession;

    let mut session =
        PcbEditSession::open(path).map_err(|e| format!("Error opening file: {:?}", e))?;

    let layer =
        Layer::from_name(layer_str).ok_or_else(|| format!("Invalid layer: {}", layer_str))?;

    let parts: Vec<&str> = at_str.split(',').collect();
    if parts.len() != 2 {
        return Err("Position must be in format 'x,y'".to_string());
    }

    let location = CoordPoint::new(parse_coord(parts[0])?, parse_coord(parts[1])?);
    let text_height = height
        .as_ref()
        .map(|h| parse_coord(h))
        .transpose()?
        .unwrap_or(Coord::from_mms(1.0));

    let idx = session
        .add_text(
            text,
            location,
            text_height,
            Some(layer),
            Some(rotation),
            None,
        )
        .map_err(|e| format!("Error adding text: {:?}", e))?;

    session
        .save_to_original()
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!("Added text at index {}", idx);
    println!("  Text:     \"{}\"", text);
    println!("  Layer:    {}", layer.name());
    println!(
        "  Position: ({:.3}mm, {:.3}mm)",
        location.x.to_mms(),
        location.y.to_mms()
    );
    println!("  Height:   {:.3}mm", text_height.to_mms());
    println!("  Rotation: {:.1}deg", rotation);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// REGION COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// List all regions.
pub fn cmd_regions(
    path: &Path,
    layer_filter: Option<String>,
) -> Result<PcbDocRegions, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    let layer = layer_filter.as_ref().and_then(|l| Layer::from_name(l));

    let regions: Vec<_> = pcb
        .iter_regions()
        .filter(|r| layer.is_none_or(|l| r.common.layer == l))
        .collect();

    let region_infos: Vec<RegionInfo> = regions
        .iter()
        .enumerate()
        .map(|(i, r)| {
            RegionInfo {
                index: i,
                layer: r.common.layer.name().to_string(),
                vertex_count: r.outline.len(),
                is_keepout: r.common.is_keepout(),
                net: String::new(), // Regions don't typically have nets in Altium
            }
        })
        .collect();

    Ok(PcbDocRegions {
        path: path.display().to_string(),
        total_regions: region_infos.len(),
        layer_filter,
        regions: region_infos,
    })
}

/// Add a region.
pub fn cmd_add_region(
    path: &Path,
    vertices_str: &str,
    layer_str: &str,
    keepout: bool,
    net: Option<String>,
) -> Result<(), String> {
    use crate::edit::PcbEditSession;

    let mut session =
        PcbEditSession::open(path).map_err(|e| format!("Error opening file: {:?}", e))?;

    let layer =
        Layer::from_name(layer_str).ok_or_else(|| format!("Invalid layer: {}", layer_str))?;

    let mut vertices = Vec::new();
    for vertex_str in vertices_str.split_whitespace() {
        let parts: Vec<&str> = vertex_str.split(',').collect();
        if parts.len() != 2 {
            return Err(format!(
                "Invalid vertex format '{}', expected 'x,y'",
                vertex_str
            ));
        }
        vertices.push(CoordPoint::new(
            parse_coord(parts[0])?,
            parse_coord(parts[1])?,
        ));
    }

    if vertices.len() < 3 {
        return Err("At least 3 vertices are required for a region".to_string());
    }

    let idx = session
        .add_region(&vertices, layer, keepout, net.as_deref())
        .map_err(|e| format!("Error adding region: {:?}", e))?;

    session
        .save_to_original()
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    let region_type = if keepout {
        "keepout region"
    } else {
        "copper region"
    };
    println!("Added {} at index {}", region_type, idx);
    println!("  Layer:    {}", layer.name());
    println!("  Vertices: {}", vertices.len());

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// DELETE COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Delete a primitive by index.
pub fn cmd_delete_primitive(path: &Path, index: usize) -> Result<(), String> {
    use crate::edit::PcbEditSession;

    let mut session =
        PcbEditSession::open(path).map_err(|e| format!("Error opening file: {:?}", e))?;

    session
        .delete_primitive(index)
        .map_err(|e| format!("Error deleting primitive: {:?}", e))?;

    session
        .save_to_original()
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!("Deleted primitive at index {}", index);

    Ok(())
}

/// Delete all tracks on a layer.
pub fn cmd_delete_tracks(path: &Path, layer_str: &str) -> Result<(), String> {
    use crate::edit::PcbEditSession;

    let mut session =
        PcbEditSession::open(path).map_err(|e| format!("Error opening file: {:?}", e))?;

    let layer =
        Layer::from_name(layer_str).ok_or_else(|| format!("Invalid layer: {}", layer_str))?;

    let count = session
        .delete_tracks_on_layer(layer)
        .map_err(|e| format!("Error deleting tracks: {:?}", e))?;

    session
        .save_to_original()
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!("Deleted {} tracks on {}", count, layer.name());

    Ok(())
}

/// Delete all vias.
pub fn cmd_delete_vias(path: &Path) -> Result<(), String> {
    use crate::edit::PcbEditSession;

    let mut session =
        PcbEditSession::open(path).map_err(|e| format!("Error opening file: {:?}", e))?;

    let count = session
        .delete_all_vias()
        .map_err(|e| format!("Error deleting vias: {:?}", e))?;

    session
        .save_to_original()
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!("Deleted {} vias", count);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// NET COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// List all nets.
pub fn cmd_nets(path: &Path) -> Result<PcbDocNets, Box<dyn std::error::Error>> {
    let pcb = open_pcbdoc(path)?;

    Ok(PcbDocNets {
        path: path.display().to_string(),
        total_nets: pcb.nets.len(),
        nets: pcb.nets.clone(),
    })
}

/// Add a new net.
pub fn cmd_add_net(path: &Path, name: &str) -> Result<(), String> {
    use crate::edit::PcbEditSession;

    let mut session =
        PcbEditSession::open(path).map_err(|e| format!("Error opening file: {:?}", e))?;

    session
        .add_net(name)
        .map_err(|e| format!("Error adding net: {:?}", e))?;

    session
        .save_to_original()
        .map_err(|e| format!("Error saving file: {:?}", e))?;

    println!("Added net '{}'", name);

    Ok(())
}

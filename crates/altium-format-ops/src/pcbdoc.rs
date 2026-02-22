// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! PCB document operations (v2).
//!
//! Provides high-level operations for exploring and manipulating Altium PCB
//! document (.PcbDoc) files using the v2 backing-store architecture.
//!
//! **Status:** All functions are stubs that return an error indicating the PcbDoc
//! document type is not yet available in the v2 API. These stubs exist so that the
//! CLI compiles and gives a clear message when users attempt PcbDoc operations.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// PCBDOC OUTPUT TYPES (STUBS)
// ═══════════════════════════════════════════════════════════════════════════

/// Overview of a PCB document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocOverview {
    pub message: String,
}

/// Detailed info about a PCB document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocInfo {
    pub message: String,
}

/// List of design rules in a PCB document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocRuleList {
    pub message: String,
}

/// Detail of a single design rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocRuleDetail {
    pub message: String,
}

/// List of components in a PCB document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocComponentList {
    pub message: String,
}

/// Detail of a single component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocComponentDetail {
    pub message: String,
}

/// List of nets in a PCB document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocNetList {
    pub message: String,
}

/// Board outline information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocOutline {
    pub message: String,
}

/// Board settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocSettings {
    pub message: String,
}

/// Layer stack information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocLayers {
    pub message: String,
}

/// List of keepout regions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocKeepouts {
    pub message: String,
}

/// List of board cutouts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocCutouts {
    pub message: String,
}

/// List of copper pour polygons.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocPolygonList {
    pub message: String,
}

/// Detail of a single polygon pour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocPolygonDetail {
    pub message: String,
}

/// List of tracks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocTrackList {
    pub message: String,
}

/// List of vias.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocViaList {
    pub message: String,
}

/// List of arcs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocArcList {
    pub message: String,
}

/// List of fills.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocFillList {
    pub message: String,
}

/// List of text objects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocTextList {
    pub message: String,
}

/// List of regions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocRegionList {
    pub message: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// STUB ERROR MESSAGE
// ═══════════════════════════════════════════════════════════════════════════

const NOT_IMPLEMENTED: &str =
    "PcbDoc v2 operations not yet implemented - pending PcbDoc document type";

fn stub_err<T>() -> crate::Result<T> {
    Err(crate::AltiumOpsError::NotImplemented(NOT_IMPLEMENTED.to_string()))
}

// ═══════════════════════════════════════════════════════════════════════════
// BROWSE COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Returns a complete overview of a PCB document.
#[allow(unused_variables)]
pub fn cmd_overview(path: &Path) -> crate::Result<PcbDocOverview> {
    stub_err()
}

/// Returns detailed document info and statistics.
#[allow(unused_variables)]
pub fn cmd_info(path: &Path) -> crate::Result<PcbDocInfo> {
    stub_err()
}

/// Lists all design rules, optionally filtered by kind.
#[allow(unused_variables)]
pub fn cmd_rules(
    path: &Path,
    kind: Option<String>,
    verbose: bool,
) -> crate::Result<PcbDocRuleList> {
    stub_err()
}

/// Shows details for a specific design rule.
#[allow(unused_variables)]
pub fn cmd_rule(
    path: &Path,
    name: &str,
    verbose: bool,
) -> crate::Result<PcbDocRuleDetail> {
    stub_err()
}

/// Lists all components, optionally filtered by layer.
#[allow(unused_variables)]
pub fn cmd_components(
    path: &Path,
    verbose: bool,
    layer: Option<String>,
) -> crate::Result<PcbDocComponentList> {
    stub_err()
}

/// Shows details for a specific component by designator.
#[allow(unused_variables)]
pub fn cmd_component(
    path: &Path,
    designator: &str,
    verbose: bool,
) -> crate::Result<PcbDocComponentDetail> {
    stub_err()
}

/// Lists all nets in the PCB document.
#[allow(unused_variables)]
pub fn cmd_nets(path: &Path) -> crate::Result<PcbDocNetList> {
    stub_err()
}

/// Exports the PCB document as JSON.
#[allow(unused_variables)]
pub fn cmd_json(
    path: &Path,
    full: bool,
    pretty: bool,
) -> crate::Result<serde_json::Value> {
    stub_err()
}

/// Shows the board outline.
#[allow(unused_variables)]
pub fn cmd_outline(
    path: &Path,
    verbose: bool,
) -> crate::Result<PcbDocOutline> {
    stub_err()
}

/// Shows board settings.
#[allow(unused_variables)]
pub fn cmd_settings(
    path: &Path,
    verbose: bool,
) -> crate::Result<PcbDocSettings> {
    stub_err()
}

/// Shows the layer stack.
#[allow(unused_variables)]
pub fn cmd_layers(path: &Path, all: bool) -> crate::Result<PcbDocLayers> {
    stub_err()
}

/// Lists keepout regions, optionally filtered by layer.
#[allow(unused_variables)]
pub fn cmd_keepouts(
    path: &Path,
    layer: Option<String>,
) -> crate::Result<PcbDocKeepouts> {
    stub_err()
}

/// Lists board cutouts.
#[allow(unused_variables)]
pub fn cmd_cutouts(path: &Path) -> crate::Result<PcbDocCutouts> {
    stub_err()
}

/// Lists copper pour polygons, optionally filtered by layer and/or net.
#[allow(unused_variables)]
pub fn cmd_polygons(
    path: &Path,
    layer: Option<String>,
    net: Option<String>,
) -> crate::Result<PcbDocPolygonList> {
    stub_err()
}

/// Shows details for a specific polygon by index.
#[allow(unused_variables)]
pub fn cmd_polygon(
    path: &Path,
    index: usize,
) -> crate::Result<PcbDocPolygonDetail> {
    stub_err()
}

/// Lists tracks, optionally filtered by layer.
#[allow(unused_variables)]
pub fn cmd_tracks(
    path: &Path,
    layer: Option<String>,
) -> crate::Result<PcbDocTrackList> {
    stub_err()
}

/// Lists vias.
#[allow(unused_variables)]
pub fn cmd_vias(path: &Path) -> crate::Result<PcbDocViaList> {
    stub_err()
}

/// Lists arcs, optionally filtered by layer.
#[allow(unused_variables)]
pub fn cmd_arcs(
    path: &Path,
    layer: Option<String>,
) -> crate::Result<PcbDocArcList> {
    stub_err()
}

/// Lists fills, optionally filtered by layer.
#[allow(unused_variables)]
pub fn cmd_fills(
    path: &Path,
    layer: Option<String>,
) -> crate::Result<PcbDocFillList> {
    stub_err()
}

/// Lists text objects, optionally filtered by layer.
#[allow(unused_variables)]
pub fn cmd_texts(
    path: &Path,
    layer: Option<String>,
) -> crate::Result<PcbDocTextList> {
    stub_err()
}

/// Lists regions, optionally filtered by layer.
#[allow(unused_variables)]
pub fn cmd_regions(
    path: &Path,
    layer: Option<String>,
) -> crate::Result<PcbDocRegionList> {
    stub_err()
}

// ═══════════════════════════════════════════════════════════════════════════
// MANIPULATION COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Creates a new PCB document, optionally from a template.
#[allow(unused_variables)]
pub fn cmd_create(
    path: &Path,
    template: Option<PathBuf>,
) -> crate::Result<()> {
    stub_err()
}

/// Sets a rectangular board outline.
#[allow(unused_variables)]
pub fn cmd_set_outline_rect(
    path: &Path,
    width: &str,
    height: &str,
    origin_x: &str,
    origin_y: &str,
) -> crate::Result<()> {
    stub_err()
}

/// Sets the board outline from a vertex string.
#[allow(unused_variables)]
pub fn cmd_set_outline(path: &Path, vertices: &str) -> crate::Result<()> {
    stub_err()
}

/// Updates board settings.
#[allow(unused_variables)]
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
) -> crate::Result<()> {
    stub_err()
}

/// Adds a rectangular keepout region.
#[allow(unused_variables)]
pub fn cmd_add_keepout(
    path: &Path,
    layer: &str,
    x1: &str,
    y1: &str,
    x2: &str,
    y2: &str,
) -> crate::Result<()> {
    stub_err()
}

/// Adds a rectangular board cutout.
#[allow(unused_variables)]
pub fn cmd_add_cutout(
    path: &Path,
    x1: &str,
    y1: &str,
    x2: &str,
    y2: &str,
) -> crate::Result<()> {
    stub_err()
}

/// Adds a copper pour polygon.
#[allow(unused_variables)]
pub fn cmd_add_polygon(
    path: &Path,
    layer: &str,
    net: &str,
    vertices: &str,
    pour_over: bool,
    remove_dead: bool,
    hatch_style: &str,
) -> crate::Result<()> {
    stub_err()
}

/// Adds a track segment.
#[allow(unused_variables)]
pub fn cmd_add_track(
    path: &Path,
    start: Option<String>,
    end: Option<String>,
    start_pad: Option<String>,
    end_pad: Option<String>,
    width: Option<String>,
    layer: &str,
    net: Option<String>,
) -> crate::Result<()> {
    stub_err()
}

/// Adds a multi-segment track path.
#[allow(unused_variables)]
pub fn cmd_add_track_path(
    path: &Path,
    vertices: &str,
    width: Option<String>,
    layer: &str,
    net: Option<String>,
) -> crate::Result<()> {
    stub_err()
}

/// Adds a via.
#[allow(unused_variables)]
pub fn cmd_add_via(
    path: &Path,
    at: Option<String>,
    at_pad: Option<String>,
    diameter: Option<String>,
    hole: Option<String>,
    from_layer: &str,
    to_layer: &str,
    net: Option<String>,
) -> crate::Result<()> {
    stub_err()
}

/// Adds an arc.
#[allow(unused_variables)]
pub fn cmd_add_arc(
    path: &Path,
    center: &str,
    radius: &str,
    start_angle: f64,
    end_angle: f64,
    width: Option<String>,
    layer: &str,
    net: Option<String>,
) -> crate::Result<()> {
    stub_err()
}

/// Adds a fill rectangle.
#[allow(unused_variables)]
pub fn cmd_add_fill(
    path: &Path,
    layer: &str,
    net: Option<&str>,
    x1: &str,
    y1: &str,
    x2: &str,
    y2: &str,
    rotation: f64,
) -> crate::Result<()> {
    stub_err()
}

/// Adds a text object.
#[allow(unused_variables)]
pub fn cmd_add_text(
    path: &Path,
    layer: &str,
    text: &str,
    x: &str,
    y: &str,
    height: Option<String>,
    rotation: f64,
    mirror: bool,
) -> crate::Result<()> {
    stub_err()
}

/// Adds a region.
#[allow(unused_variables)]
pub fn cmd_add_region(
    path: &Path,
    layer: &str,
    net: Option<&str>,
    vertices: &str,
    kind: Option<&str>,
) -> crate::Result<()> {
    stub_err()
}

/// Places a component at a specified position with placement options.
#[allow(unused_variables)]
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
) -> crate::Result<()> {
    stub_err()
}

/// Adds a component from a schematic source.
#[allow(unused_variables)]
pub fn cmd_add_component(
    path: &Path,
    schematic: &Path,
    designator: &str,
    footprint_lib: Option<PathBuf>,
    footprint: Option<String>,
    at: Option<String>,
    layer: &str,
) -> crate::Result<()> {
    stub_err()
}

/// Adds a net to the PCB document.
#[allow(unused_variables)]
pub fn cmd_add_net(path: &Path, name: &str) -> crate::Result<()> {
    stub_err()
}

/// Adds a design rule.
#[allow(unused_variables)]
pub fn cmd_add_rule(
    path: &Path,
    kind: &str,
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
) -> crate::Result<()> {
    stub_err()
}

/// Modifies an existing design rule.
#[allow(unused_variables)]
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
) -> crate::Result<()> {
    stub_err()
}

/// Deletes a design rule by name.
#[allow(unused_variables)]
pub fn cmd_delete_rule(path: &Path, name: &str) -> crate::Result<()> {
    stub_err()
}

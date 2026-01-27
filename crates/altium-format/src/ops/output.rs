// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Output data structures for ops functions.
//!
//! This module defines structured data types returned by ops functions,
//! allowing separation of business logic from presentation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════════
// INTLIB OUTPUT TYPES
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntLibOverview {
    pub path: String,
    pub version: u32,
    pub component_count: usize,
    pub schematic_symbol_count: usize,
    pub pcb_footprint_count: usize,
    pub parameter_set_count: usize,
    pub footprint_usage: Vec<(String, usize)>, // (footprint_name, count)
    pub component_list: Vec<ComponentCrossRef>,
    // Full details (only populated when --full is requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbols: Option<Vec<SymbolSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footprints: Option<Vec<FootprintSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<ComponentParameters>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentCrossRef {
    pub name: String,
    pub description: String,
    pub footprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntLibInfo {
    pub path: String,
    pub version: u32,
    pub cross_ref_count: usize,
    pub schematic_symbol_count: usize,
    pub pcb_footprint_count: usize,
    pub parameter_set_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntLibComponentDetail {
    pub name: String,
    pub description: String,
    pub footprint: String,
    pub schlib_path: String,
    pub pcblib_path: String,
    pub symbol_info: Option<SymbolInfo>,
    pub footprint_info: Option<FootprintInfo>,
    pub parameters: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub pin_count: usize,
    pub primitive_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FootprintInfo {
    pub pad_count: usize,
    pub primitive_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntLibSearchResults {
    pub query: String,
    pub total_matches: usize,
    pub results: Vec<ComponentCrossRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntLibComponentList {
    pub components: Vec<ComponentCrossRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntLibSymbolList {
    pub symbols: Vec<SymbolSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolSummary {
    pub name: String,
    pub description: String,
    pub pin_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntLibFootprintList {
    pub footprints: Vec<FootprintSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FootprintSummary {
    pub name: String,
    pub description: String,
    pub pad_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntLibParameterList {
    pub parameters: Vec<ComponentParameters>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentParameters {
    pub component_name: String,
    pub params: HashMap<String, String>,
}

// ═══════════════════════════════════════════════════════════════════════════
// SCHLIB OUTPUT TYPES
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchLibOverview {
    pub path: String,
    pub total_components: usize,
    pub components_by_category: Vec<(String, Vec<ComponentSummary>)>,
    pub pin_statistics: PinStatistics,
    pub multi_part_components: Vec<ComponentSummary>,
    pub largest_components: Vec<ComponentSummary>,
    // Full details (only populated when --full is requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_details: Option<Vec<SchLibComponentDetail>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSummary {
    pub name: String,
    pub description: String,
    pub pin_count: usize,
    pub part_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinStatistics {
    pub total_pins: usize,
    pub pin_types: Vec<(String, usize)>, // (type_name, count)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchLibComponentList {
    pub path: String,
    pub total_components: usize,
    pub components: Vec<ComponentSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchLibSearchResults {
    pub query: String,
    pub total_matches: usize,
    pub results: Vec<ComponentSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchLibInfo {
    pub path: String,
    pub component_count: usize,
    pub total_primitives: usize,
    pub primitive_types: Vec<(String, usize)>, // (type_name, count)
    pub multi_part_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchLibComponentDetail {
    pub name: String,
    pub description: String,
    pub part_count: i32,
    pub display_mode_count: i32,
    pub pin_count: usize,
    pub total_primitives: usize,
    pub pins: Vec<PinDetail>,
    pub primitive_counts: Option<Vec<(String, usize)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinDetail {
    pub designator: String,
    pub name: String,
    pub electrical_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchLibPinList {
    pub path: String,
    pub total_pins: usize,
    pub pins: Vec<PinWithComponent>,
    pub pins_by_type: Option<Vec<(String, Vec<PinWithComponent>)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinWithComponent {
    pub component_name: String,
    pub designator: String,
    pub name: String,
    pub electrical_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchLibPrimitiveList {
    pub component_name: String,
    pub total_primitives: usize,
    pub primitives: Vec<PrimitiveInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PrimitiveInfo {
    Pin {
        designator: String,
        name: String,
        electrical_type: String,
        x: String,
        y: String,
    },
    Rectangle {
        x1: String,
        y1: String,
        x2: String,
        y2: String,
    },
    Line {
        x1: String,
        y1: String,
        x2: String,
        y2: String,
    },
    Arc {
        center_x: String,
        center_y: String,
        radius: String,
        start_angle: f64,
        end_angle: f64,
    },
    Polygon {
        vertex_count: usize,
    },
    Polyline {
        vertex_count: usize,
    },
    Label {
        text: String,
        x: String,
        y: String,
    },
    Other {
        primitive_type: String,
    },
}

// ═══════════════════════════════════════════════════════════════════════════
// PRJPCB OUTPUT TYPES
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrjPcbOverview {
    pub path: String,
    pub name: String,
    pub version: String,
    pub hierarchy_mode: String,
    pub document_summary: DocumentSummary,
    pub parameters: HashMap<String, String>,
    pub component_summary: Option<ComponentSummaryStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSummary {
    pub total_documents: usize,
    pub schematics: Vec<DocumentInfo>,
    pub pcb_documents: Vec<DocumentInfo>,
    pub libraries: Vec<DocumentInfo>,
    pub other: Vec<DocumentInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub path: String,
    pub doc_type: String,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSummaryStats {
    pub total_components: usize,
    pub by_prefix: Vec<(String, String, usize)>, // (prefix, display_name, count)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrjPcbInfo {
    pub path: String,
    pub name: String,
    pub version: String,
    pub hierarchy_mode: String,
    pub output_path: String,
    pub annotation_start: i32,
    pub document_counts: Vec<(String, usize)>, // (type, count)
    pub parameter_count: usize,
    pub erc_matrix_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrjPcbDocumentList {
    pub path: String,
    pub filter: Option<String>,
    pub total_documents: usize,
    pub documents: Vec<DocumentDetailInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentDetailInfo {
    pub path: String,
    pub doc_type: String,
    pub exists: bool,
    pub annotation_enabled: bool,
    pub library_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrjPcbNetlist {
    pub path: String,
    pub total_nets: usize,
    pub nets: Vec<NetInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetInfo {
    pub name: String,
    pub pins: Vec<NetPinConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetPinConnection {
    pub component: String,
    pub pin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrjPcbComponentList {
    pub path: String,
    pub total_components: usize,
    pub components: Vec<SchematicComponentInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchematicComponentInfo {
    pub designator: String,
    pub lib_reference: String,
    pub description: String,
    pub footprint: String,
    pub value: String,
    pub sheet: String,
    pub parameters: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrjPcbBom {
    pub path: String,
    pub total_components: usize,
    pub unique_parts: Option<usize>,
    pub items: BomItems,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BomItems {
    Grouped(Vec<BomGroupItem>),
    Individual(Vec<SchematicComponentInfo>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomGroupItem {
    pub lib_reference: String,
    pub quantity: usize,
    pub designators: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrjPcbSchPcbDiff {
    pub path: String,
    pub pcb_document: String,
    pub schematic_components: usize,
    pub pcb_components: usize,
    pub only_in_schematic: Vec<String>,
    pub only_in_pcb: Vec<String>,
    pub schematic_nets: usize,
    pub pcb_nets: usize,
    pub nets_only_in_schematic: Vec<String>,
    pub nets_only_in_pcb: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrjPcbValidation {
    pub path: String,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

// ═══════════════════════════════════════════════════════════════════════════
// PCBLIB OUTPUT TYPES
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbLibOverview {
    pub path: String,
    pub total_footprints: usize,
    pub unique_id: String,
    pub footprints_by_category: Vec<(String, Vec<FootprintSummaryExt>)>,
    pub pad_statistics: PadStatistics,
    pub hole_sizes: Vec<(String, usize)>, // (size, count)
    pub largest_footprints: Vec<FootprintSummaryExt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FootprintSummaryExt {
    pub name: String,
    pub description: String,
    pub pad_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PadStatistics {
    pub total_pads: usize,
    pub smd_pads: usize,
    pub th_pads: usize,
    pub pad_shapes: Vec<(String, usize)>, // (shape_name, count)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbLibFootprintList {
    pub path: String,
    pub total_footprints: usize,
    pub footprints: Vec<FootprintSummaryExt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbLibSearchResults {
    pub query: String,
    pub total_matches: usize,
    pub results: Vec<FootprintSummaryExt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbLibInfo {
    pub path: String,
    pub footprint_count: usize,
    pub unique_id: String,
    pub total_primitives: usize,
    pub primitive_types: Vec<(String, usize)>, // (type_name, count)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbLibFootprintDetail {
    pub pattern: String,
    pub description: String,
    pub height: String,
    pub pad_count: usize,
    pub total_primitives: usize,
    pub bounding_box: BoundingBox,
    pub pads: Vec<PadDetail>,
    pub primitive_counts: Option<Vec<(String, usize)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub width: String,
    pub height: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PadDetail {
    pub designator: String,
    pub shape: String,
    pub size: String,
    pub hole: Option<String>,
    pub layer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbLibPadList {
    pub path: String,
    pub total_pads: usize,
    pub pads: Vec<PadWithFootprint>,
    pub pads_by_shape: Option<Vec<(String, Vec<PadWithFootprint>)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PadWithFootprint {
    pub footprint_name: String,
    pub designator: String,
    pub size: String,
    pub hole: Option<String>,
    pub shape: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbLibPrimitiveList {
    pub footprint_name: String,
    pub total_primitives: usize,
    pub primitives: Vec<PrimitiveDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PrimitiveDetail {
    Pad {
        designator: String,
        shape: String,
        size: String,
        hole: Option<String>,
    },
    Track {
        start_x: String,
        start_y: String,
        end_x: String,
        end_y: String,
        width: String,
    },
    Arc {
        center_x: String,
        center_y: String,
        radius: String,
        start_angle: f64,
        end_angle: f64,
    },
    Text {
        text: String,
        x: String,
        y: String,
    },
    Fill {
        x1: String,
        y1: String,
        x2: String,
        y2: String,
    },
    Region {
        vertex_count: usize,
        layer: String,
    },
    ComponentBody {
        vertex_count: usize,
        height: String,
    },
    Other {
        primitive_type: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbLibHoleAnalysis {
    pub path: String,
    pub hole_sizes: Vec<HoleSizeInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoleSizeInfo {
    pub size: String,
    pub count: usize,
    pub example_footprints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbLibJson {
    pub file: String,
    pub footprint_count: usize,
    pub unique_id: String,
    pub footprints: Vec<FootprintJsonData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FootprintJsonData {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub pad_count: usize,
    pub primitive_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pads: Option<Vec<PadJsonData>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PadJsonData {
    pub designator: String,
    pub shape: String,
    pub size_x: String,
    pub size_y: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hole_size: Option<String>,
    pub layer: String,
}

// Measurement output types for cmd_measure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementValue {
    pub mm: f64,
    pub mils: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbLibMeasurementReport {
    pub name: String,
    pub dimensions: DimensionsData,
    pub pads: Vec<PadInfoData>,
    pub pitch: Vec<PitchData>,
    pub min_pad_clearance: Option<ClearanceData>,
    pub silkscreen_clearance: Option<ClearanceData>,
    pub row_span: Option<MeasurementValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionsData {
    pub width: MeasurementValue,
    pub height: MeasurementValue,
    pub min_x: MeasurementValue,
    pub max_x: MeasurementValue,
    pub min_y: MeasurementValue,
    pub max_y: MeasurementValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PadInfoData {
    pub designator: String,
    pub x: MeasurementValue,
    pub y: MeasurementValue,
    pub width: MeasurementValue,
    pub height: MeasurementValue,
    pub hole: Option<MeasurementValue>,
    pub shape: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitchData {
    pub pitch: MeasurementValue,
    pub direction: String,
    pub count: usize,
    pub pad_pairs: Vec<(String, String, MeasurementValue)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearanceData {
    pub feature1: String,
    pub feature2: String,
    pub clearance: MeasurementValue,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PadDistanceData {
    pub pad1: String,
    pub pad2: String,
    pub center_to_center: MeasurementValue,
    pub edge_to_edge: MeasurementValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PadClearances {
    pub pad_to_pad: Option<ClearanceData>,
    pub pad_to_silkscreen: Option<ClearanceData>,
}

// ═══════════════════════════════════════════════════════════════════════════
// PCBDOC OUTPUT TYPES
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocOverview {
    pub path: String,
    pub summary: PcbDocSummary,
    pub rules_by_category: Vec<(String, Vec<RuleSummary>)>, // (category, rules)
    pub components_preview: Vec<ComponentPreview>,
    pub nets_preview: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocSummary {
    pub components: usize,
    pub nets: usize,
    pub rules: usize,
    pub primitives: usize,
    pub tracks: usize,
    pub vias: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSummary {
    pub name: String,
    pub priority: i32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentPreview {
    pub designator: String,
    pub pattern: String,
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocInfo {
    pub path: String,
    pub component_count: usize,
    pub net_count: usize,
    pub rule_count: usize,
    pub primitive_count: usize,
    pub track_count: usize,
    pub via_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocRuleList {
    pub path: String,
    pub filter: Option<String>,
    pub total_rules: usize,
    pub rules: Vec<RuleInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleInfo {
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    pub priority: i32,
    pub scope1_expression: String,
    pub scope2_expression: String,
    pub comment: String,
    pub parameters: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocRuleDetail {
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    pub priority: i32,
    pub scope1_expression: String,
    pub scope2_expression: String,
    pub comment: String,
    pub parameters: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocJson {
    pub file: String,
    pub summary: PcbDocSummary,
    pub rules: Option<Vec<RuleInfo>>,
    pub components: Option<Vec<PcbComponentInfo>>,
    pub nets: Option<Vec<String>>,
    pub layers: Option<Vec<LayerInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbComponentInfo {
    pub designator: String,
    pub pattern: String,
    pub comment: String,
    pub x: Option<String>,
    pub y: Option<String>,
    pub rotation: f64,
    pub layer: String,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocComponentList {
    pub path: String,
    pub total_components: usize,
    pub layer_filter: Option<String>,
    pub components: Vec<PcbComponentInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocComponentDetail {
    pub designator: String,
    pub pattern: String,
    pub comment: String,
    pub source_designator: String,
    pub source_footprint: String,
    pub x: Option<String>,
    pub y: Option<String>,
    pub rotation: f64,
    pub layer: String,
    pub locked: bool,
    pub pad_count: usize,
    pub unique_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocOutline {
    pub vertex_count: usize,
    pub width_mm: f64,
    pub height_mm: f64,
    pub vertices: Vec<OutlineVertex>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineVertex {
    pub x_mm: f64,
    pub y_mm: f64,
    pub kind: String, // "line" or "arc"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub center_x_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub center_y_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius_mm: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocSettings {
    pub display_unit: String,
    pub snap_grid: String,
    pub visible_grid: String,
    pub component_grid: String,
    pub track_grid: Option<String>,
    pub via_grid: Option<String>,
    pub track_width: Option<String>,
    pub origin_x: String,
    pub origin_y: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocLayers {
    pub path: String,
    pub total_layers: usize,
    pub show_all: bool,
    pub layers: Vec<LayerInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerInfo {
    pub id: u8,
    pub name: String,
    pub layer_type: String,
    pub used: bool,
    pub enabled: bool,
    pub copper_thickness: Option<String>,
    pub dielectric_constant: Option<f64>,
    pub dielectric_thickness: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocKeepouts {
    pub path: String,
    pub total_keepouts: usize,
    pub layer_filter: Option<String>,
    pub keepouts: Vec<KeepoutInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeepoutInfo {
    pub index: usize,
    pub layer: String,
    pub x1: String,
    pub y1: String,
    pub x2: String,
    pub y2: String,
    pub kind: String, // "region", "track", "via", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocCutouts {
    pub path: String,
    pub total_cutouts: usize,
    pub cutouts: Vec<CutoutInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CutoutInfo {
    pub index: usize,
    pub vertex_count: usize,
    pub bounds: BoundsInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundsInfo {
    pub x1: String,
    pub y1: String,
    pub x2: String,
    pub y2: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocPolygons {
    pub path: String,
    pub total_polygons: usize,
    pub layer_filter: Option<String>,
    pub net_filter: Option<String>,
    pub polygons: Vec<PolygonSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolygonSummary {
    pub index: usize,
    pub layer: String,
    pub net: String,
    pub vertex_count: usize,
    pub pour_over: bool,
    pub remove_dead: bool,
    pub hatch_style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocPolygonDetail {
    pub index: usize,
    pub layer: String,
    pub net: String,
    pub vertex_count: usize,
    pub pour_over: bool,
    pub remove_dead: bool,
    pub hatch_style: String,
    pub vertices: Vec<PolygonVertexInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolygonVertexInfo {
    pub x: String,
    pub y: String,
    pub kind: String, // "line" or "arc"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocTracks {
    pub path: String,
    pub total_tracks: usize,
    pub layer_filter: Option<String>,
    pub tracks: Vec<TrackInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackInfo {
    pub index: usize,
    pub layer: String,
    pub start_x: String,
    pub start_y: String,
    pub end_x: String,
    pub end_y: String,
    pub width: String,
    pub net: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocVias {
    pub path: String,
    pub total_vias: usize,
    pub vias: Vec<ViaInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViaInfo {
    pub index: usize,
    pub x: String,
    pub y: String,
    pub diameter: String,
    pub hole_size: String,
    pub from_layer: String,
    pub to_layer: String,
    pub net: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocArcs {
    pub path: String,
    pub total_arcs: usize,
    pub layer_filter: Option<String>,
    pub arcs: Vec<ArcInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcInfo {
    pub index: usize,
    pub layer: String,
    pub center_x: String,
    pub center_y: String,
    pub radius: String,
    pub start_angle: f64,
    pub end_angle: f64,
    pub width: String,
    pub net: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocFills {
    pub path: String,
    pub total_fills: usize,
    pub layer_filter: Option<String>,
    pub fills: Vec<FillInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillInfo {
    pub index: usize,
    pub layer: String,
    pub x1: String,
    pub y1: String,
    pub x2: String,
    pub y2: String,
    pub rotation: f64,
    pub net: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocTexts {
    pub path: String,
    pub total_texts: usize,
    pub layer_filter: Option<String>,
    pub texts: Vec<TextInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextInfo {
    pub index: usize,
    pub text: String,
    pub layer: String,
    pub x: String,
    pub y: String,
    pub height: String,
    pub rotation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocRegions {
    pub path: String,
    pub total_regions: usize,
    pub layer_filter: Option<String>,
    pub regions: Vec<RegionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionInfo {
    pub index: usize,
    pub layer: String,
    pub vertex_count: usize,
    pub is_keepout: bool,
    pub net: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDocNets {
    pub path: String,
    pub total_nets: usize,
    pub nets: Vec<String>,
}

// ═══════════════════════════════════════════════════════════════════════════
// SCHDOC OUTPUT TYPES
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocOverview {
    pub path: String,
    pub sheet_size: String,
    pub components_by_category: Vec<(String, Vec<SchDocComponentRef>)>,
    pub power_architecture: PowerArchitecture,
    pub interfaces: Option<InterfaceSummary>,
    pub key_signals: KeySignals,
    pub quick_stats: SchDocQuickStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocComponentRef {
    pub designator: String,
    pub lib_reference: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerArchitecture {
    pub power_rails: Vec<(String, usize)>, // (net_name, connection_count)
    pub ground_nets: Vec<(String, usize)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceSummary {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub bidirectional: Vec<String>,
    pub unspecified: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeySignals {
    pub total_unique_nets: usize,
    pub data_buses: Vec<String>,
    pub address_buses: Vec<String>,
    pub control_signals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocQuickStats {
    pub components: usize,
    pub wires: usize,
    pub junctions: usize,
    pub net_labels: usize,
    pub ports: usize,
    pub power_symbols: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocBom {
    pub path: String,
    pub total_components: usize,
    pub unique_parts: usize,
    pub items: Vec<BomItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomItem {
    pub lib_reference: String,
    pub quantity: usize,
    pub designators: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocNetlist {
    pub path: String,
    pub filter: Option<String>,
    pub min_connections: usize,
    pub total_nets: usize,
    pub nets: Vec<NetConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetConnection {
    pub net_name: String,
    pub connections: Vec<String>, // "Component.Pin (name)" format
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocPowerMap {
    pub path: String,
    pub power_rails: Vec<PowerRail>,
    pub ground_nets: Vec<GroundNet>,
    pub powered_components: Vec<PoweredComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerRail {
    pub net_name: String,
    pub symbol_count: usize,
    pub consumers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundNet {
    pub net_name: String,
    pub symbol_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoweredComponent {
    pub designator: String,
    pub lib_reference: String,
    pub power_pin_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocBlocks {
    pub path: String,
    pub blocks: Vec<BlockInfo>,
    pub show_all: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub designator: String,
    pub lib_reference: String,
    pub description: String,
    pub category: String,
    pub power_pins: Vec<String>,
    pub input_pins: Vec<String>,
    pub output_pins: Vec<String>,
    pub bidir_pins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocProjectAnalysis {
    pub sheet_count: usize,
    pub sheets: Vec<SheetInfo>,
    pub inter_sheet_connections: Vec<InterSheetConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetInfo {
    pub name: String,
    pub component_count: usize,
    pub port_count: usize,
    pub net_count: usize,
    pub ports: Vec<(String, String)>, // (name, io_type)
    pub power_nets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterSheetConnection {
    pub port_name: String,
    pub connected_sheets: Vec<(String, String)>, // (sheet_name, io_type)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocSignalFlow {
    pub path: String,
    pub signal: String,
    pub trace_found: bool,
    pub trace: Option<SignalTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalTrace {
    pub source: String,
    pub path: Vec<String>,
    pub destinations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocInfo {
    pub path: String,
    pub sheet_info: Option<SheetInfoDetails>,
    pub primitive_summary: PrimitiveSummary,
    pub unique_nets: Vec<String>,
    pub power_nets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetInfoDetails {
    pub size: String,
    pub size_style: i32,
    pub custom_dimensions: Option<(String, String)>,
    pub fonts_defined: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveSummary {
    pub total_primitives: usize,
    pub components: usize,
    pub wires: usize,
    pub net_labels: usize,
    pub ports: usize,
    pub power_objects: usize,
    pub junctions: usize,
    pub pins: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocStats {
    pub path: String,
    pub total_primitives: usize,
    pub record_types: Vec<(String, usize)>, // (type_name, count)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocComponentList {
    pub path: String,
    pub total_components: usize,
    pub components: Vec<SchDocComponentInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocComponentInfo {
    pub designator: String,
    pub lib_reference: String,
    pub description: String,
    pub location: String,
    pub parts: i32,
    pub child_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocComponentDetail {
    pub designator: String,
    pub lib_reference: String,
    pub description: String,
    pub location: String,
    pub parts: i32,
    pub display_modes: i32,
    pub current_part: i32,
    pub unique_id: String,
    pub child_primitive_count: usize,
    pub pins: Vec<SchDocPinInfo>,
    pub parameters: Vec<SchDocParameter>,
    pub designators: Vec<SchDocDesignator>,
    pub graphic_primitive_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocPinInfo {
    pub designator: String,
    pub name: String,
    pub electrical_type: String,
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocParameter {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocDesignator {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocWireList {
    pub path: String,
    pub total_wires: usize,
    pub wires: Vec<WireInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireInfo {
    pub index: usize,
    pub start: String,
    pub end_or_segments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocNetLabelList {
    pub path: String,
    pub total_net_labels: usize,
    pub group_by_name: bool,
    pub grouped: Option<Vec<(String, usize)>>, // (net_name, count)
    pub individual: Option<Vec<NetLabelInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetLabelInfo {
    pub net_name: String,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocPortList {
    pub path: String,
    pub total_ports: usize,
    pub ports: Vec<PortInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortInfo {
    pub name: String,
    pub io_type: String,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocPowerList {
    pub path: String,
    pub total_power_objects: usize,
    pub group_by_net: bool,
    pub grouped: Option<Vec<(String, usize)>>, // (net_name, count)
    pub individual: Option<Vec<PowerObjectInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerObjectInfo {
    pub net: String,
    pub style: String,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocPinList {
    pub path: String,
    pub total_pins: usize,
    pub filter: Option<String>,
    pub pins: Vec<SchDocPinDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocPinDetail {
    pub component: String,
    pub designator: String,
    pub name: String,
    pub electrical_type: String,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocHierarchy {
    pub path: String,
    pub hierarchy: Vec<HierarchyNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchyNode {
    pub node_type: String, // "sheet", "component", etc.
    pub unique_id: String,
    pub description: String,
    pub children: Vec<HierarchyNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocSearchResults {
    pub path: String,
    pub query: String,
    pub total_matches: usize,
    pub results: Vec<SearchMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    pub designator: String,
    pub lib_reference: String,
    pub description: String,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocJunctionList {
    pub path: String,
    pub total_junctions: usize,
    pub junctions: Vec<JunctionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JunctionInfo {
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocJson {
    pub file: String,
    pub sheet: Option<JsonSheetInfo>,
    pub summary: JsonDocSummary,
    pub components: Option<Vec<JsonComponentInfo>>,
    pub nets: Option<Vec<JsonNetInfo>>,
    pub ports: Option<Vec<JsonPortInfo>>,
    pub power: Option<Vec<JsonPowerInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSheetInfo {
    pub size: String,
    pub fonts: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonDocSummary {
    pub total_primitives: usize,
    pub components: usize,
    pub wires: usize,
    pub net_labels: usize,
    pub ports: usize,
    pub power_objects: usize,
    pub junctions: usize,
    pub pins: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonComponentInfo {
    pub designator: String,
    pub lib_reference: String,
    pub description: String,
    pub location: String,
    pub pins: Vec<JsonPinInfo>,
    pub parameters: Vec<JsonParameterInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonPinInfo {
    pub designator: String,
    pub name: String,
    pub electrical: String,
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonParameterInfo {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonNetInfo {
    pub name: String,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonPortInfo {
    pub name: String,
    pub io_type: String,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonPowerInfo {
    pub net: String,
    pub style: String,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocValidationResult {
    pub path: String,
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub kind: String,
    pub message: String,
    pub location: Option<(f64, f64)>, // (x, y) in mils
    pub components: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocNetlistAnalysis {
    pub path: String,
    pub total_nets: usize,
    pub nets: Vec<NetDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetDetail {
    pub name: String,
    pub connection_count: usize,
    pub connections: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocUnconnectedPins {
    pub path: String,
    pub total_unconnected: usize,
    pub pins: Vec<UnconnectedPin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnconnectedPin {
    pub component: String,
    pub pin: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocMissingJunctions {
    pub path: String,
    pub total_missing: usize,
    pub locations: Vec<(f64, f64)>, // (x, y) in mils
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocLibrarySearchResults {
    pub library: String,
    pub pattern: String,
    pub total_matches: usize,
    pub matches: Vec<LibraryComponentMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryComponentMatch {
    pub name: String,
    pub description: String,
    pub pins: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchDocLibraryList {
    pub library: String,
    pub total_components: usize,
    pub components: Vec<LibraryComponentInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryComponentInfo {
    pub name: String,
    pub description: String,
    pub pins: usize,
    pub primitives: Option<usize>,
}

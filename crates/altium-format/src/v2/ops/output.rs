// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Output data structures for v2 ops functions.
//!
//! This module defines structured data types returned by ops functions,
//! allowing separation of business logic from presentation.
//! These are plain data structs with Serialize + Deserialize, no v1 dependencies.

use serde::{Deserialize, Serialize};

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

// ═══════════════════════════════════════════════════════════════════════════
// PCBDOC OUTPUT TYPES (STUBS)
// ═══════════════════════════════════════════════════════════════════════════

// PcbDoc operations are not yet implemented in the v2 API.
// Output types will be added when PcbDoc document I/O is available.

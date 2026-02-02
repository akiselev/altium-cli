//! Harness record data structs.

use crate::v2::types::*;
use super::{DataObjectBase, GraphicalObjectBase, RectangularEntryContainerBase, BasicEntryObjectBase};
use super::schematic::{WireData, NetLabelData, LabelData, PowerData};
use super::primitives::{RectangleData, LineData};
use super::sheet::SheetEntryData;
use super::pin::PinData;

/// HarnessConnector record data — from `ExportHarnessConnector`/`ImportHarnessConnector`.
#[derive(Clone, Debug, Default)]
pub struct HarnessConnectorData {
    pub container: RectangularEntryContainerBase,
    pub primary_connection_position: i32,
    pub harness_connector_side: LeftRightSide,
    pub unique_id: String,
}

/// HarnessEntry — delegates to BasicEntryObject.
pub type HarnessEntryData = BasicEntryObjectBase;

/// HarnessWire record data — Wire base + harness-specific fields.
#[derive(Clone, Debug, Default)]
pub struct HarnessWireData {
    pub wire: WireData,
    pub secondary_color: u32,
    pub tertiary_color: u32,
    pub border_color: u32,
    pub end_vertex1_connected_object_unique_id: String,
    pub end_vertex2_connected_object_unique_id: String,
    pub connected_inline_splices: Vec<String>,
    pub connected_wire_labels: Vec<String>,
    pub connected_shields: Vec<String>,
    pub connected_twists: Vec<String>,
    pub connected_cables: Vec<String>,
    // Library component fields
    pub vault_guid: String,
    pub item_guid: String,
    pub revision_guid: String,
    pub design_item_id: String,
    pub source_library_name: String,
    pub library_path: String,
    pub lib_reference: String,
    pub not_use_library_name: bool,
    pub database_table_name: String,
    pub designator_locked: bool,
    pub component_kind: u8,
}

/// HarnessSplice record data — from `ExportHarnessSplice`.
#[derive(Clone, Debug, Default)]
pub struct HarnessSpliceData {
    pub graphical: GraphicalObjectBase,
    pub style: u8,
    pub show_name: bool,
    pub connected_wires: Vec<String>,
    pub connected_inline_wire_unique_id: String,
    pub location_x: i32,
    pub location_y: i32,
    pub orientation: RotationBy90,
    pub color: u32,
    pub area_color: u32,
    pub border_color: u32,
    pub font_id: i32,
    pub text: String,
    pub unique_id: String,
    pub designator_locked: bool,
}

/// HarnessLayoutLabel — Label base + harness-specific fields.
#[derive(Clone, Debug, Default)]
pub struct HarnessLayoutLabelData {
    pub label: LabelData,
    pub alignment: HorizontalAlign,
    pub area_color: u32,
    pub text_color: u32,
    pub show_only_first_line: bool,
    pub encoded_text: String,
    pub designator_locked: bool,
    // Library component fields
    pub vault_guid: String,
    pub item_guid: String,
    pub revision_guid: String,
    pub design_item_id: String,
    pub source_library_name: String,
    pub library_path: String,
    pub lib_reference: String,
    pub not_use_library_name: bool,
    pub database_table_name: String,
    pub component_kind: u8,
}

/// HarnessLayoutConnectionPoint — from `ExportHarnessLayoutConnectionPoint`.
#[derive(Clone, Debug, Default)]
pub struct HarnessLayoutConnectionPointData {
    pub graphical: GraphicalObjectBase,
    pub style: u8,
    pub connected_bundles: Vec<String>,
    pub location_x: i32,
    pub location_y: i32,
    pub orientation: RotationBy90,
    pub color: u32,
    pub area_color: u32,
    pub border_color: u32,
    pub font_id: i32,
    pub text: String,
    pub unique_id: String,
    pub show_name: bool,
    pub designator_locked: bool,
}

/// HarnessBundle — Wire base + bundle-specific fields.
#[derive(Clone, Debug, Default)]
pub struct HarnessBundleData {
    pub wire: WireData,
    pub length: i32,
    pub length_long: i64,
    pub is_length_set_manually: bool,
    pub end_vertex1_connected_object_unique_id: String,
    pub end_vertex2_connected_object_unique_id: String,
    pub designator_locked: bool,
}

/// HarnessLogicalSignal — Line base + signal fields.
#[derive(Clone, Debug, Default)]
pub struct HarnessLogicalSignalData {
    pub line: LineData,
    pub connection1_comp: String,
    pub connection1_pin: String,
    pub connection2_comp: String,
    pub connection2_pin: String,
    pub name: String,
    pub system_design_unique_id: String,
}

/// HarnessPin — Pin base + harness fields.
#[derive(Clone, Debug, Default)]
pub struct HarnessPinData {
    pub pin: PinData,
    pub connected_wires: Vec<String>,
    pub wiring_diagram_origin_unique_id: String,
}

/// HarnessWireLabel — NetLabel base + connected wire.
#[derive(Clone, Debug, Default)]
pub struct HarnessWireLabelData {
    pub net_label: NetLabelData,
    pub connected_wire_unique_id: String,
}

/// HarnessWireData record — from `ExportHarnessWireData`.
#[derive(Clone, Debug, Default)]
pub struct HarnessWireDataRecord {
    pub base: DataObjectBase,
    pub name: String,
    pub comment: String,
    pub description: String,
    pub color: u32,
    pub end_vertex1_connected_object_unique_id: String,
    pub end_vertex2_connected_object_unique_id: String,
    pub connected_shields: Vec<String>,
    pub connected_twists: Vec<String>,
    pub connected_cables: Vec<String>,
    pub connected_inline_splices: Vec<String>,
    pub unique_id: String,
    pub vault_guid: String,
    pub item_guid: String,
    pub revision_guid: String,
    pub design_item_id: String,
    pub source_library_name: String,
    pub component_kind: u8,
}

/// HarnessSpliceData record — from `ExportHarnessSpliceData`.
#[derive(Clone, Debug, Default)]
pub struct HarnessSpliceDataRecord {
    pub base: DataObjectBase,
    pub designator: String,
    pub connected_wires: Vec<String>,
    pub connected_inline_wire_unique_id: String,
    pub unique_id: String,
    pub style: u8,
}

/// HarnessNoConnect — from `ExportHarnessNoConnect`.
#[derive(Clone, Debug, Default)]
pub struct HarnessNoConnectData {
    pub graphical: GraphicalObjectBase,
    pub style: NoERCSymbol,
    pub show_name: bool,
    pub connected_wires: Vec<String>,
    pub location_x: i32,
    pub location_y: i32,
    pub orientation: RotationBy90,
    pub color: u32,
    pub font_id: i32,
    pub text: String,
    pub unique_id: String,
}

/// HarnessNoConnectData record — from `ExportHarnessNoConnectData`.
#[derive(Clone, Debug, Default)]
pub struct HarnessNoConnectDataRecord {
    pub base: DataObjectBase,
    pub designator: String,
    pub connected_wires: Vec<String>,
    pub unique_id: String,
}

/// HarnessShield — Rectangle + library + shield fields.
#[derive(Clone, Debug, Default)]
pub struct HarnessShieldData {
    pub rect: RectangleData,
    // Library component fields
    pub vault_guid: String,
    pub item_guid: String,
    pub revision_guid: String,
    pub design_item_id: String,
    pub source_library_name: String,
    pub library_path: String,
    pub lib_reference: String,
    pub not_use_library_name: bool,
    pub database_table_name: String,
    // Shield-specific
    pub style: u8,
    pub rotation: RotationBy90,
    pub connected_wires: Vec<String>,
    pub connected_pin_wires: Vec<String>,
    pub designator_locked: bool,
    pub comment: String,
    pub component_kind: u8,
}

/// HarnessShieldData record — from `ExportHarnessShieldData`.
#[derive(Clone, Debug, Default)]
pub struct HarnessShieldDataRecord {
    pub base: DataObjectBase,
    pub designator: String,
    pub connected_wires: Vec<String>,
    pub connected_pin_wires: Vec<String>,
    pub unique_id: String,
    pub style: u8,
    pub comment: String,
    pub component_kind: u8,
}

/// HarnessTwist — Rectangle + library + twist fields.
#[derive(Clone, Debug, Default)]
pub struct HarnessTwistData {
    pub rect: RectangleData,
    // Library component fields
    pub vault_guid: String,
    pub item_guid: String,
    pub revision_guid: String,
    pub design_item_id: String,
    pub source_library_name: String,
    pub library_path: String,
    pub lib_reference: String,
    pub not_use_library_name: bool,
    pub database_table_name: String,
    // Twist-specific
    pub rotation: RotationBy90,
    pub connected_wires: Vec<String>,
    pub designator_locked: bool,
}

/// HarnessTwistData record — from `ExportHarnessTwistData`.
#[derive(Clone, Debug, Default)]
pub struct HarnessTwistDataRecord {
    pub base: DataObjectBase,
    pub designator: String,
    pub connected_wires: Vec<String>,
    pub unique_id: String,
}

/// HarnessCable — Rectangle + library + cable fields.
#[derive(Clone, Debug, Default)]
pub struct HarnessCableData {
    pub rect: RectangleData,
    // Library component fields
    pub vault_guid: String,
    pub item_guid: String,
    pub revision_guid: String,
    pub design_item_id: String,
    pub source_library_name: String,
    pub library_path: String,
    pub lib_reference: String,
    pub not_use_library_name: bool,
    pub database_table_name: String,
    // Cable-specific
    pub rotation: RotationBy90,
    pub connected_wires: Vec<String>,
    pub designator_locked: bool,
    pub component_kind: u8,
}

/// HarnessCableData record — from `ExportHarnessCableData`.
#[derive(Clone, Debug, Default)]
pub struct HarnessCableDataRecord {
    pub base: DataObjectBase,
    pub designator: String,
    pub comment: String,
    pub description: String,
    pub connected_wires: Vec<String>,
    pub unique_id: String,
    pub vault_guid: String,
    pub item_guid: String,
    pub revision_guid: String,
    pub design_item_id: String,
    pub source_library_name: String,
    pub component_kind: u8,
}

/// HarnessAssociatedParts — just a DataObject wrapper.
#[derive(Clone, Debug, Default)]
pub struct HarnessAssociatedPartsData {
    pub base: DataObjectBase,
}

/// HarnessLayoutCovering — from `ExportHarnessLayoutCovering`.
#[derive(Clone, Debug, Default)]
pub struct HarnessLayoutCoveringData {
    pub graphical: GraphicalObjectBase,
    pub border_width: Size,
    pub color: u32,
    pub area_color: u32,
    pub transparent: bool,
    pub thickness: u8,
    pub start_point_distance: i32,
    pub end_point_distance: i32,
    pub length: i32,
    pub harness_layout_braid_brush: u8,
    pub designator_locked: bool,
    // Library component fields
    pub vault_guid: String,
    pub item_guid: String,
    pub revision_guid: String,
    pub design_item_id: String,
    pub source_library_name: String,
    pub library_path: String,
    pub lib_reference: String,
    pub not_use_library_name: bool,
    pub database_table_name: String,
    pub default_designator_position_x: i32,
    pub default_designator_position_y: i32,
    pub unique_id: String,
    pub component_kind: u8,
    pub physical_start_distance: i64,
    pub physical_end_distance: i64,
    pub physical_length: i64,
}

/// HarnessWireBreak — CrossSheetConnector base + wire-break fields.
#[derive(Clone, Debug, Default)]
pub struct HarnessWireBreakData {
    pub power: PowerData,
    pub connected_wire_unique_id: String,
    pub secondary_color: u32,
    pub tertiary_color: u32,
    pub border_color: u32,
    pub primary_color_name: String,
    pub secondary_color_name: String,
    pub tertiary_color_name: String,
    pub border_color_name: String,
    pub vault_guid: String,
    pub item_guid: String,
    pub revision_guid: String,
    pub design_item_id: String,
    pub source_library_name: String,
}

/// HarnessDocument base — Sheet + harness length unit.
#[derive(Clone, Debug, Default)]
pub struct HarnessDocumentData {
    pub sheet: super::sheet::SheetData,
    pub harness_length_unit: u8,
}

/// HarnessWiringDiagram — delegates to HarnessDocument.
pub type HarnessWiringDiagramData = HarnessDocumentData;

/// HarnessLayoutDrawing — delegates to HarnessDocument.
pub type HarnessLayoutDrawingData = HarnessDocumentData;

/// HarnessComponent — delegates to Component.
pub type HarnessComponentData = super::component::ComponentData;

/// HighLevelCodeEntry — delegates to SheetEntry.
pub type HighLevelCodeEntryData = SheetEntryData;

/// HighLevelCodeSymbol — delegates to SheetSymbol.
pub type HighLevelCodeSymbolData = super::sheet::SheetSymbolData;

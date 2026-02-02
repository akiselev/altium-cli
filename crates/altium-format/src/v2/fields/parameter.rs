//! Parameter and related record data structs.

use crate::v2::types::*;
use super::{DataObjectBase, GraphicalObjectBase};

/// Parameter record data — from `ExportParameter`/`ImportParameter` (ObjectId::Parameter = 41).
#[derive(Clone, Debug, Default)]
pub struct ParameterData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub orientation: RotationBy90,
    pub justification: TextJustification,
    pub color: u32,
    pub font_id: i32,
    pub is_hidden: bool,
    pub text: String,
    pub param_type: ParameterType,
    pub name: String,
    pub show_name: bool,
    pub read_only_state: ParameterReadOnlyState,
    pub unique_id: String,
    pub description: String,
    pub allow_library_synchronize: bool,
    pub allow_database_synchronize: bool,
    pub auto_position: bool,
    pub is_mirrored: bool,
    pub text_horz_anchor: TextHorzAnchor,
    pub text_vert_anchor: TextVertAnchor,
    pub is_image_parameter: bool,
}

/// Designator record data — from `ExportDesignator`/`ImportDesignator` (ObjectId::Designator = 34).
///
/// Extends ParameterData with auto-position override handling.
#[derive(Clone, Debug, Default)]
pub struct DesignatorData {
    pub param: ParameterData,
    pub override_not_auto_position: bool,
}

/// ParameterList — just a graphical object wrapper.
#[derive(Clone, Debug, Default)]
pub struct ParameterListData {
    pub graphical: GraphicalObjectBase,
}

/// ParameterSet record data — from `ExportParameterSet`/`ImportParameterSet`.
#[derive(Clone, Debug, Default)]
pub struct ParameterSetData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub color: u32,
    pub orientation: RotationBy90,
    pub name: String,
    pub style: ParameterSetStyle,
    pub unique_id: String,
}

/// MapDefiner record data — from `ExportMapDefiner`/`ImportMapDefiner`.
#[derive(Clone, Debug, Default)]
pub struct MapDefinerData {
    pub base: DataObjectBase,
    pub designator_interface: String,
    pub implementation_designators: Vec<String>,
}

/// ImplementationMap — just a data object wrapper.
#[derive(Clone, Debug, Default)]
pub struct ImplementationMapData {
    pub base: DataObjectBase,
}

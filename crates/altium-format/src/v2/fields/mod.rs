//! Per-record data structs and format functions.
//!
//! Each record type gets a struct and export/import function pair,
//! ported from `FileFormatV5.cs`.

pub mod block;
pub mod component;
pub mod harness;
pub mod implementation;
pub mod misc;
pub mod parameter;
pub mod pin;
pub mod primitives;
pub mod schematic;
pub mod sheet;

// Re-export everything for convenience
pub use block::*;
pub use component::ComponentData;
pub use harness::*;
pub use implementation::*;
pub use misc::*;
pub use parameter::*;
pub use pin::PinData;
pub use primitives::*;
pub use schematic::*;
pub use sheet::*;

use crate::v2::types::*;

// ============================================================================
// Base object structs (shared across multiple record types)
// ============================================================================

/// Base data object fields — from `ExportDataObject`/`ImportDataObject`.
#[derive(Clone, Debug, Default)]
pub struct DataObjectBase {
    pub owner_index: i32,
    pub is_not_accessible: bool,
    pub owner_index_additional_list: bool,
    pub index_in_sheet: i32,
    pub ignore_on_load: bool,
    pub is_schematic_block_object: bool,
    pub unique_id_in_reuse_block: String,
}

/// Graphical object fields — from `ExportGraphicalObject`/`ImportGraphicalObject`.
///
/// Extends DataObjectBase.
#[derive(Clone, Debug, Default)]
pub struct GraphicalObjectBase {
    pub base: DataObjectBase,
    pub owner_part_id: i16,
    pub owner_part_display_mode: u8,
    pub selection_memory: u8,
    pub union_index: i32,
    pub graphically_locked: bool,
}

/// Rectangular entry container base — from `ExportRectangularEntryContainer`.
#[derive(Clone, Debug, Default)]
pub struct RectangularEntryContainerBase {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub x_size: i32,
    pub y_size: i32,
    pub line_width: Size,
    pub color: u32,
    pub area_color: u32,
}

/// Basic entry object base — from `ExportBasicEntryObject`.
#[derive(Clone, Debug, Default)]
pub struct BasicEntryObjectBase {
    pub graphical: GraphicalObjectBase,
    pub side: LeftRightSide,
    pub distance_from_top: i32,
    pub color: u32,
    pub area_color: u32,
    pub text_color: u32,
    pub text_font_id: i32,
    pub text_style: String,
    pub name: String,
    pub harness_type: String,
    pub unique_id: String,
}

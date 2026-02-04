//! SchComponent - Schematic component (Record 1).
//!
//! **DEPRECATED**: Use `v2::fields::ComponentData` with `v2::serializer::format_v5` instead.

use crate::error::{AltiumError, Result};
use crate::types::{CoordRect, ParameterCollection, UnknownFields};

use super::{SchGraphicalBase, SchPrimitive, SchPrimitiveBase, TextOrientations};

/// Known parameter keys for SchComponent (for unknown field filtering).
/// NOTE: Unused after V1 import_from_params stubbed, retained for documentation.
#[allow(dead_code)]
const KNOWN_KEYS: &[&str] = &[
    "RECORD",
    "OWNERINDEX",
    "INDEXINSHEET",
    "ISNOTACCESIBLE",
    "OWNERPARTID",
    "OWNERPARTDISPLAYMODE",
    "GRAPHICALLYLOCKED",
    "LOCATION.X",
    "LOCATION.X_FRAC",
    "LOCATION.Y",
    "LOCATION.Y_FRAC",
    "COLOR",
    "AREACOLOR",
    "LIBREFERENCE",
    "COMPONENTDESCRIPTION",
    "UNIQUEID",
    "CURRENTPARTID",
    "PARTCOUNT",
    "DISPLAYMODECOUNT",
    "DISPLAYMODE",
    "SHOWHIDDENPINS",
    "LIBRARYPATH",
    "SOURCELIBRARYNAME",
    "SHEETPARTFILENAME",
    "TARGETFILENAME",
    "OVERIDECOLORS",
    "DESIGNATORLOCKED",
    "PARTIDLOCKED",
    "COMPONENTKIND",
    "ALIASLIST",
    "ORIENTATION",
];

/// Schematic component - container for all primitives in a symbol.
///
/// **DEPRECATED**: Use `v2::fields::ComponentData` with `v2::serializer::format_v5::import_component()`
/// and `v2::serializer::format_v5::export_component()` instead.
#[deprecated(note = "Use v2::fields::ComponentData")]
#[derive(Debug, Clone)]
pub struct SchComponent {
    /// Graphical base (location, color).
    pub graphical: SchGraphicalBase,
    /// Library reference name.
    pub lib_reference: String,
    /// Component description.
    pub component_description: String,
    /// Unique identifier.
    pub unique_id: String,
    /// Current part ID (1-based, for multi-part symbols).
    pub current_part_id: i32,
    /// Number of parts in the symbol.
    pub part_count: i32,
    /// Number of display modes.
    pub display_mode_count: i32,
    /// Current display mode.
    pub display_mode: i32,
    /// Whether to show hidden pins.
    pub show_hidden_pins: bool,
    /// Library path.
    pub library_path: String,
    /// Source library name.
    pub source_library_name: String,
    /// Sheet part filename.
    pub sheet_part_filename: String,
    /// Target filename.
    pub target_filename: String,
    /// Whether colors are overridden.
    pub override_colors: bool,
    /// Whether designator is locked.
    pub designator_locked: bool,
    /// Whether part ID is locked.
    pub part_id_locked: bool,
    /// Component kind.
    pub component_kind: i32,
    /// Alias list.
    pub alias_list: String,
    /// Orientation.
    pub orientation: TextOrientations,
    /// Unknown parameters (preserved for non-destructive editing).
    pub unknown_params: UnknownFields,
}

impl Default for SchComponent {
    fn default() -> Self {
        Self {
            graphical: SchGraphicalBase {
                base: SchPrimitiveBase {
                    owner_index: 0, // 0 so OWNERINDEX is omitted (Component is root, has no owner)
                    is_not_accessible: false,
                    owner_part_id: Some(-1), // Components use -1, primitives use None (serializes as 1)
                    owner_part_display_mode: None,
                    graphically_locked: false,
                    index_in_sheet: Some(-1), // Components always have INDEXINSHEET=-1
                },
                location_x: 0,
                location_y: 0,
                color: 128,           // Maroon for component outlines
                area_color: 11599871, // Light gray/yellow for component fill
            },
            lib_reference: String::new(),
            component_description: String::new(),
            unique_id: String::new(),
            current_part_id: 1,    // 1-based part ID
            part_count: 1,         // Single part by default
            display_mode_count: 1, // Single display mode
            display_mode: 0,       // First display mode
            show_hidden_pins: false,
            library_path: "*".to_string(),        // Altium placeholder
            source_library_name: "*".to_string(), // Altium placeholder
            sheet_part_filename: "*".to_string(), // Altium placeholder
            target_filename: "*".to_string(),     // Altium placeholder
            override_colors: false,
            designator_locked: false,
            part_id_locked: true, // Part ID locked by default in library context
            component_kind: 0,
            alias_list: String::new(),
            orientation: TextOrientations::NONE,
            unknown_params: UnknownFields::default(),
        }
    }
}

impl SchComponent {
    /// Set display mode with validation against display_mode_count.
    ///
    /// display_mode indexes into array of display_mode_count elements; out-of-bounds
    /// causes undefined behavior when rendering symbol views. Validation returns error
    /// rather than panic because invalid data from corrupted files should be recoverable.
    pub fn set_display_mode(&mut self, mode: i32) -> Result<()> {
        if mode < 0 {
            return Err(AltiumError::Validation(
                "display_mode cannot be negative".into(),
            ));
        }
        if mode >= self.display_mode_count {
            return Err(AltiumError::Validation(format!(
                "display_mode {} exceeds display_mode_count {}",
                mode, self.display_mode_count
            )));
        }
        self.display_mode = mode;
        Ok(())
    }
}

#[allow(deprecated)]
impl SchPrimitive for SchComponent {
    const RECORD_ID: i32 = 1;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Component"
    }

    fn get_property(&self, name: &str) -> Option<String> {
        match name {
            "LIBREFERENCE" => Some(self.lib_reference.clone()),
            "COMPONENTDESCRIPTION" => Some(self.component_description.clone()),
            "UNIQUEID" => Some(self.unique_id.clone()),
            _ => None,
        }
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchComponent::import_from_params is deprecated. \
            Use v2::fields::ComponentData with v2::serializer::format_v5::import_component() instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchComponent::export_to_params is deprecated. \
            Use v2::fields::ComponentData with v2::serializer::format_v5::export_component() instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        // Component bounds are calculated from child primitives
        CoordRect::empty()
    }
}

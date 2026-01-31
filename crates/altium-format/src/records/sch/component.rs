//! SchComponent - Schematic component (Record 1).

use crate::error::{AltiumError, Result};
use crate::types::{CoordRect, ParameterCollection, UnknownFields};

use super::{SchGraphicalBase, SchPrimitive, SchPrimitiveBase, TextOrientations};

/// Known parameter keys for SchComponent (for unknown field filtering).
const KNOWN_KEYS: &[&str] = &[
    "RECORD",
    "OWNERINDEX",
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
    "DESIGNITEMID",
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
                    owner_index: -1,
                    is_not_accessible: false,
                    owner_part_id: Some(-1), // Components use -1, primitives use None (serializes as 1)
                    owner_part_display_mode: None,
                    graphically_locked: false,
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
            part_id_locked: false, // Part ID unlocked by default in library context
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

    fn import_from_params(params: &ParameterCollection) -> Result<Self> {
        let graphical = SchGraphicalBase::import_from_params(params);

        Ok(SchComponent {
            graphical,
            lib_reference: params
                .get("LIBREFERENCE")
                .map(|v| v.as_str().to_string())
                .or_else(|| params.get("DESIGNITEMID").map(|v| v.as_str().to_string()))
                .unwrap_or_default(),
            component_description: params
                .get("COMPONENTDESCRIPTION")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            unique_id: params
                .get("UNIQUEID")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            current_part_id: params
                .get("CURRENTPARTID")
                .map(|v| v.as_int_or(1))
                .unwrap_or(1),
            // PARTCOUNT is stored as +1 in files
            part_count: params
                .get("PARTCOUNT")
                .map(|v| v.as_int_or(2) - 1)
                .unwrap_or(1),
            display_mode_count: params
                .get("DISPLAYMODECOUNT")
                .map(|v| v.as_int_or(1))
                .unwrap_or(1),
            display_mode: params
                .get("DISPLAYMODE")
                .map(|v| v.as_int_or(0))
                .unwrap_or(0),
            show_hidden_pins: params
                .get("SHOWHIDDENPINS")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            library_path: params
                .get("LIBRARYPATH")
                .map(|v| v.as_str().to_string())
                .unwrap_or_else(|| "*".to_string()),
            source_library_name: params
                .get("SOURCELIBRARYNAME")
                .map(|v| v.as_str().to_string())
                .unwrap_or_else(|| "*".to_string()),
            sheet_part_filename: params
                .get("SHEETPARTFILENAME")
                .map(|v| v.as_str().to_string())
                .unwrap_or_else(|| "*".to_string()),
            target_filename: params
                .get("TARGETFILENAME")
                .map(|v| v.as_str().to_string())
                .unwrap_or_else(|| "*".to_string()),
            override_colors: params
                .get("OVERIDECOLORS")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            designator_locked: params
                .get("DESIGNATORLOCKED")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            part_id_locked: params
                .get("PARTIDLOCKED")
                .map(|v| v.as_bool_or(true))
                .unwrap_or(true),
            component_kind: params
                .get("COMPONENTKIND")
                .map(|v| v.as_int_or(0))
                .unwrap_or(0),
            alias_list: params
                .get("ALIASLIST")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            orientation: params
                .get("ORIENTATION")
                .map(|v| TextOrientations::from_int(v.as_int_or(0)))
                .unwrap_or_default(),
            unknown_params: UnknownFields::from_remaining_params(params, KNOWN_KEYS),
        })
    }

    fn export_to_params(&self) -> ParameterCollection {
        let mut params = ParameterCollection::new();
        params.add_int("RECORD", Self::RECORD_ID);
        self.graphical.export_to_params(&mut params);
        params.add("LIBREFERENCE", &self.lib_reference);
        if !self.component_description.is_empty() {
            params.add("COMPONENTDESCRIPTION", &self.component_description);
        }
        params.add_int("PARTCOUNT", self.part_count + 1);
        params.add_int("DISPLAYMODECOUNT", self.display_mode_count);
        // Only emit ORIENTATION when non-zero
        if self.orientation.to_int() != 0 {
            params.add_int("ORIENTATION", self.orientation.to_int());
        }
        params.add_int("CURRENTPARTID", self.current_part_id);
        // Only emit SHOWHIDDENPINS when true (Altium omits false)
        if self.show_hidden_pins {
            params.add_bool("SHOWHIDDENPINS", true);
        }
        params.add("LIBRARYPATH", &self.library_path);
        params.add("SOURCELIBRARYNAME", &self.source_library_name);
        params.add("SHEETPARTFILENAME", &self.sheet_part_filename);
        params.add("TARGETFILENAME", &self.target_filename);
        params.add("UNIQUEID", &self.unique_id);
        // Only emit DISPLAYMODE when non-zero
        if self.display_mode != 0 {
            params.add_int("DISPLAYMODE", self.display_mode);
        }
        // Only emit OVERIDECOLORS when true
        if self.override_colors {
            params.add_bool("OVERIDECOLORS", true);
        }
        // Only emit DESIGNATORLOCKED when true
        if self.designator_locked {
            params.add_bool("DESIGNATORLOCKED", true);
        }
        params.add_bool("PARTIDLOCKED", self.part_id_locked);
        // Only emit ALIASLIST when non-empty
        if !self.alias_list.is_empty() {
            params.add("ALIASLIST", &self.alias_list);
        }
        params.add("DESIGNITEMID", &self.lib_reference);
        // Only emit COMPONENTKIND when non-zero
        if self.component_kind != 0 {
            params.add_int("COMPONENTKIND", self.component_kind);
        }

        // Merge unknown parameters back
        self.unknown_params.merge_into_params(&mut params);

        params
    }

    fn owner_index(&self) -> i32 {
        self.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        // Component bounds are calculated from child primitives
        CoordRect::empty()
    }
}

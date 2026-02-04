//! Implementation-related schematic records.
//!
//! These records handle component model implementations (footprints, simulation models, etc).
//!
//! **DEPRECATED**: Use `v2::fields::implementation` types with `v2::serializer::format_v5` instead.

use crate::error::Result;
use crate::types::{CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{SchPrimitive, SchPrimitiveBase};

/// SchImplementationList (Record 44) - Container for implementation records.
///
/// This is essentially a container/parent record for SchImplementation children.
///
/// **DEPRECATED**: Use `v2::fields::ImplementationListData` instead.
#[deprecated(note = "Use v2::fields::ImplementationListData")]
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 44, format = "params")]
pub struct SchImplementationList {
    /// Base primitive fields.
    #[altium(flatten)]
    pub base: SchPrimitiveBase,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

#[allow(deprecated)]
impl SchPrimitive for SchImplementationList {
    const RECORD_ID: i32 = 44;

    fn record_type_name(&self) -> &'static str {
        "ImplementationList"
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchImplementationList::import_from_params is deprecated. \
            Use v2::fields::ImplementationListData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchImplementationList::export_to_params is deprecated. \
            Use v2::fields::ImplementationListData with v2::serializer::format_v5 instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        CoordRect::default()
    }
}

/// Known parameter keys for SchImplementation (for unknown field filtering).
/// NOTE: Unused after V1 import_from_params stubbed, retained for documentation.
#[allow(dead_code)]
const IMPLEMENTATION_KNOWN_KEYS: &[&str] = &[
    "RECORD",
    "OWNERINDEX",
    "INDEXINSHEET",
    "ISNOTACCESIBLE",
    "OWNERPARTID",
    "OWNERPARTDISPLAYMODE",
    "GRAPHICALLYLOCKED",
    "DESCRIPTION",
    "MODELNAME",
    "MODELTYPE",
    "DATAFILECOUNT",
    "ISCURRENT",
];

/// SchImplementation (Record 45) - Component model implementation.
///
/// Represents a model attached to a component (footprint, simulation model, etc).
///
/// **DEPRECATED**: Use `v2::fields::ImplementationData` instead.
#[deprecated(note = "Use v2::fields::ImplementationData")]
#[derive(Debug, Clone, Default)]
pub struct SchImplementation {
    /// Base primitive fields.
    pub base: SchPrimitiveBase,
    /// Description of the implementation.
    pub description: String,
    /// Model name (e.g., footprint name).
    pub model_name: String,
    /// Model type (e.g., "PCBLIB", "SIM", "SI", "PCB3DLib").
    pub model_type: String,
    /// Data file references.
    pub data_files: Vec<String>,
    /// Data file entity names (model names for each data file).
    pub data_file_entities: Vec<String>,
    /// Whether this is the current implementation.
    pub is_current: bool,
    /// Unknown parameters (preserved for non-destructive editing).
    pub unknown_params: UnknownFields,
}

#[allow(deprecated)]
impl SchPrimitive for SchImplementation {
    const RECORD_ID: i32 = 45;

    fn record_type_name(&self) -> &'static str {
        "Implementation"
    }

    fn get_property(&self, name: &str) -> Option<String> {
        match name {
            "MODELNAME" => Some(self.model_name.clone()),
            "MODELTYPE" => Some(self.model_type.clone()),
            _ => None,
        }
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchImplementation::import_from_params is deprecated. \
            Use v2::fields::ImplementationData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchImplementation::export_to_params is deprecated. \
            Use v2::fields::ImplementationData with v2::serializer::format_v5 instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        CoordRect::default()
    }
}

/// SchMapDefinerList (Record 46) - Container for pin map definitions.
///
/// **DEPRECATED**: Use `v2::fields::MapDefinerListData` instead.
#[deprecated(note = "Use v2::fields::MapDefinerListData")]
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 46, format = "params")]
pub struct SchMapDefinerList {
    /// Base primitive fields.
    #[altium(flatten)]
    pub base: SchPrimitiveBase,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

#[allow(deprecated)]
impl SchPrimitive for SchMapDefinerList {
    const RECORD_ID: i32 = 46;

    fn record_type_name(&self) -> &'static str {
        "MapDefinerList"
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchMapDefinerList::import_from_params is deprecated. \
            Use v2::fields::MapDefinerListData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchMapDefinerList::export_to_params is deprecated. \
            Use v2::fields::MapDefinerListData with v2::serializer::format_v5 instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        CoordRect::default()
    }
}

/// Known parameter keys for SchMapDefiner (for unknown field filtering).
/// NOTE: Unused after V1 import_from_params stubbed, retained for documentation.
#[allow(dead_code)]
const MAP_DEFINER_KNOWN_KEYS: &[&str] = &[
    "RECORD",
    "OWNERINDEX",
    "INDEXINSHEET",
    "ISNOTACCESIBLE",
    "OWNERPARTID",
    "OWNERPARTDISPLAYMODE",
    "GRAPHICALLYLOCKED",
    "DESINTF",
    "DESIMPCOUNT",
    "ISTRIVIAL",
];

/// SchMapDefiner (Record 47) - Pin map definition.
///
/// Maps schematic pin designators to implementation (footprint) pin designators.
///
/// **DEPRECATED**: Use `v2::fields::MapDefinerData` instead.
#[deprecated(note = "Use v2::fields::MapDefinerData")]
#[derive(Debug, Clone, Default)]
pub struct SchMapDefiner {
    /// Base primitive fields.
    pub base: SchPrimitiveBase,
    /// Interface (schematic) designator.
    pub designator_interface: String,
    /// Implementation (footprint) designators.
    pub designator_implementation: Vec<String>,
    /// Whether this is a trivial (identity) mapping.
    pub is_trivial: bool,
    /// Unknown parameters (preserved for non-destructive editing).
    pub unknown_params: UnknownFields,
}

#[allow(deprecated)]
impl SchPrimitive for SchMapDefiner {
    const RECORD_ID: i32 = 47;

    fn record_type_name(&self) -> &'static str {
        "MapDefiner"
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchMapDefiner::import_from_params is deprecated. \
            Use v2::fields::MapDefinerData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchMapDefiner::export_to_params is deprecated. \
            Use v2::fields::MapDefinerData with v2::serializer::format_v5 instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        CoordRect::default()
    }
}

/// SchImplementationParameters (Record 48) - Additional implementation parameters.
///
/// **DEPRECATED**: Use `v2::fields::ImplementationParametersData` instead.
#[deprecated(note = "Use v2::fields::ImplementationParametersData")]
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 48, format = "params")]
pub struct SchImplementationParameters {
    /// Base primitive fields.
    #[altium(flatten)]
    pub base: SchPrimitiveBase,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

#[allow(deprecated)]
impl SchPrimitive for SchImplementationParameters {
    const RECORD_ID: i32 = 48;

    fn record_type_name(&self) -> &'static str {
        "ImplementationParameters"
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchImplementationParameters::import_from_params is deprecated. \
            Use v2::fields::ImplementationParametersData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchImplementationParameters::export_to_params is deprecated. \
            Use v2::fields::ImplementationParametersData with v2::serializer::format_v5 instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        CoordRect::default()
    }
}

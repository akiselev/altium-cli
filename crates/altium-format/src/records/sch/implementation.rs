//! Implementation-related schematic records.
//!
//! These records handle component model implementations (footprints, simulation models, etc).

use crate::error::Result;
use crate::traits::{FromParams, ToParams};
use crate::types::{CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{SchPrimitive, SchPrimitiveBase};

/// SchImplementationList (Record 44) - Container for implementation records.
///
/// This is essentially a container/parent record for SchImplementation children.
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

impl SchPrimitive for SchImplementationList {
    const RECORD_ID: i32 = 44;

    fn record_type_name(&self) -> &'static str {
        "ImplementationList"
    }

    fn import_from_params(params: &ParameterCollection) -> Result<Self> {
        Self::from_params(params)
    }

    fn export_to_params(&self) -> ParameterCollection {
        self.to_params()
    }

    fn owner_index(&self) -> i32 {
        self.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        CoordRect::default()
    }
}

/// Known parameter keys for SchImplementation (for unknown field filtering).
const IMPLEMENTATION_KNOWN_KEYS: &[&str] = &[
    "RECORD",
    "OWNERINDEX",
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

    fn import_from_params(params: &ParameterCollection) -> Result<Self> {
        let base = SchPrimitiveBase::import_from_params(params);

        let data_file_count = params
            .get("DATAFILECOUNT")
            .map(|v| v.as_int_or(0))
            .unwrap_or(0);

        let mut data_files = Vec::new();
        let mut data_file_entities = Vec::new();
        for i in 0..data_file_count {
            if let Some(file) = params.get(&format!("MODELDATAFILEKIND{}", i)) {
                data_files.push(file.as_str().to_string());
            }
            if let Some(entity) = params.get(&format!("MODELDATAFILEENTITY{}", i)) {
                data_file_entities.push(entity.as_str().to_string());
            }
        }

        // Build prefixes for unknown field filtering
        let mut indexed_prefixes: Vec<String> = Vec::new();
        for i in 0..data_file_count {
            indexed_prefixes.push(format!("MODELDATAFILEKIND{}", i));
            indexed_prefixes.push(format!("MODELDATAFILEENTITY{}", i));
        }
        let prefix_refs: Vec<&str> = indexed_prefixes.iter().map(|s| s.as_str()).collect();

        // Combine known keys with indexed prefixes
        let mut all_known: Vec<&str> = IMPLEMENTATION_KNOWN_KEYS.to_vec();
        all_known.extend(prefix_refs.iter());

        Ok(SchImplementation {
            base,
            description: params
                .get("DESCRIPTION")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            model_name: params
                .get("MODELNAME")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            model_type: params
                .get("MODELTYPE")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            data_files,
            data_file_entities,
            is_current: params
                .get("ISCURRENT")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            unknown_params: UnknownFields::from_remaining_params(params, &all_known),
        })
    }

    fn export_to_params(&self) -> ParameterCollection {
        let mut params = ParameterCollection::new();
        params.add_int("RECORD", Self::RECORD_ID);
        self.base.export_to_params(&mut params);
        if !self.description.is_empty() {
            params.add("DESCRIPTION", &self.description);
        }
        params.add("MODELNAME", &self.model_name);
        params.add("MODELTYPE", &self.model_type);
        params.add_int("DATAFILECOUNT", self.data_files.len() as i32);
        for (i, file) in self.data_files.iter().enumerate() {
            if let Some(entity) = self.data_file_entities.get(i) {
                params.add(&format!("MODELDATAFILEENTITY{}", i), entity);
            }
            params.add(&format!("MODELDATAFILEKIND{}", i), file);
        }
        params.add_bool("ISCURRENT", self.is_current);

        // Merge unknown parameters back
        self.unknown_params.merge_into_params(&mut params);

        params
    }

    fn owner_index(&self) -> i32 {
        self.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        CoordRect::default()
    }
}

/// SchMapDefinerList (Record 46) - Container for pin map definitions.
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

impl SchPrimitive for SchMapDefinerList {
    const RECORD_ID: i32 = 46;

    fn record_type_name(&self) -> &'static str {
        "MapDefinerList"
    }

    fn import_from_params(params: &ParameterCollection) -> Result<Self> {
        Self::from_params(params)
    }

    fn export_to_params(&self) -> ParameterCollection {
        self.to_params()
    }

    fn owner_index(&self) -> i32 {
        self.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        CoordRect::default()
    }
}

/// Known parameter keys for SchMapDefiner (for unknown field filtering).
const MAP_DEFINER_KNOWN_KEYS: &[&str] = &[
    "RECORD",
    "OWNERINDEX",
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

impl SchPrimitive for SchMapDefiner {
    const RECORD_ID: i32 = 47;

    fn record_type_name(&self) -> &'static str {
        "MapDefiner"
    }

    fn import_from_params(params: &ParameterCollection) -> Result<Self> {
        let base = SchPrimitiveBase::import_from_params(params);

        let impl_count = params
            .get("DESIMPCOUNT")
            .map(|v| v.as_int_or(0))
            .unwrap_or(0);

        let mut implementations = Vec::new();
        for i in 0..impl_count {
            if let Some(des) = params.get(&format!("DESIMP{}", i)) {
                implementations.push(des.as_str().to_string());
            }
        }

        // Build prefixes for unknown field filtering (0-indexed)
        let indexed_prefixes: Vec<String> =
            (0..impl_count).map(|i| format!("DESIMP{}", i)).collect();
        let prefix_refs: Vec<&str> = indexed_prefixes.iter().map(|s| s.as_str()).collect();

        // Combine known keys with indexed prefixes
        let mut all_known: Vec<&str> = MAP_DEFINER_KNOWN_KEYS.to_vec();
        all_known.extend(prefix_refs.iter());

        Ok(SchMapDefiner {
            base,
            designator_interface: params
                .get("DESINTF")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            designator_implementation: implementations,
            is_trivial: params
                .get("ISTRIVIAL")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            unknown_params: UnknownFields::from_remaining_params(params, &all_known),
        })
    }

    fn export_to_params(&self) -> ParameterCollection {
        let mut params = ParameterCollection::new();
        params.add_int("RECORD", Self::RECORD_ID);
        self.base.export_to_params(&mut params);
        params.add("DESINTF", &self.designator_interface);
        params.add_int("DESIMPCOUNT", self.designator_implementation.len() as i32);
        for (i, des) in self.designator_implementation.iter().enumerate() {
            params.add(&format!("DESIMP{}", i), des);
        }
        params.add_bool("ISTRIVIAL", self.is_trivial);

        // Merge unknown parameters back
        self.unknown_params.merge_into_params(&mut params);

        params
    }

    fn owner_index(&self) -> i32 {
        self.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        CoordRect::default()
    }
}

/// SchImplementationParameters (Record 48) - Additional implementation parameters.
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

impl SchPrimitive for SchImplementationParameters {
    const RECORD_ID: i32 = 48;

    fn record_type_name(&self) -> &'static str {
        "ImplementationParameters"
    }

    fn import_from_params(params: &ParameterCollection) -> Result<Self> {
        Self::from_params(params)
    }

    fn export_to_params(&self) -> ParameterCollection {
        self.to_params()
    }

    fn owner_index(&self) -> i32 {
        self.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        CoordRect::default()
    }
}

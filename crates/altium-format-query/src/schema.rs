use crate::ast::{CompareOp, TypeSelector};
use crate::error::{QueryError, QueryErrorCode};

/// The data type of a queryable field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    String,
    Integer,
    Float,
    Bool,
    Coord,
    Color,
    Enum,
}

impl FieldType {
    /// Check whether a comparison operator is compatible with this field type.
    pub fn is_op_compatible(self, op: CompareOp) -> bool {
        match self {
            FieldType::String | FieldType::Enum => matches!(
                op,
                CompareOp::Eq
                    | CompareOp::NotEq
                    | CompareOp::Contains
                    | CompareOp::StartsWith
                    | CompareOp::EndsWith
                    | CompareOp::WordMatch
            ),
            FieldType::Integer | FieldType::Float | FieldType::Coord => matches!(
                op,
                CompareOp::Eq
                    | CompareOp::NotEq
                    | CompareOp::Gt
                    | CompareOp::Lt
                    | CompareOp::Gte
                    | CompareOp::Lte
            ),
            FieldType::Bool => matches!(op, CompareOp::Eq | CompareOp::NotEq),
            FieldType::Color => matches!(op, CompareOp::Eq | CompareOp::NotEq),
        }
    }
}

/// Definition of a single queryable field.
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub canonical_name: &'static str,
    pub aliases: &'static [&'static str],
    pub field_type: FieldType,
}

/// Resolve a field name (possibly an Altium-style alias) to a canonical name
/// for the given type selector.
pub fn resolve_field(
    type_sel: TypeSelector,
    field_name: &str,
) -> Result<(&'static FieldDef, &'static str), QueryError> {
    let fields = fields_for_type(type_sel);
    let lower = field_name.to_ascii_lowercase();

    for def in fields {
        if def.canonical_name.to_ascii_lowercase() == lower {
            return Ok((def, def.canonical_name));
        }
        for alias in def.aliases {
            if alias.to_ascii_lowercase() == lower {
                return Ok((def, def.canonical_name));
            }
        }
    }

    Err(QueryError::new(
        QueryErrorCode::UnknownField,
        format!(
            "unknown field '{field_name}' for type '{}'",
            type_name(type_sel)
        ),
    )
    .with_help(format!(
        "available fields: {}",
        fields
            .iter()
            .map(|f| f.canonical_name)
            .collect::<Vec<_>>()
            .join(", ")
    )))
}

fn type_name(ts: TypeSelector) -> &'static str {
    match ts {
        TypeSelector::Component => "component",
        TypeSelector::Pin => "pin",
        TypeSelector::Parameter => "parameter",
        TypeSelector::Footprint => "footprint",
        TypeSelector::Graphic => "graphic",
        TypeSelector::Line => "line",
        TypeSelector::Rectangle => "rectangle",
        TypeSelector::RoundRectangle => "round_rectangle",
        TypeSelector::Arc => "arc",
        TypeSelector::EllipticalArc => "elliptical_arc",
        TypeSelector::Ellipse => "ellipse",
        TypeSelector::Pie => "pie",
        TypeSelector::Polyline => "polyline",
        TypeSelector::Polygon => "polygon",
        TypeSelector::Bezier => "bezier",
        TypeSelector::Image => "image",
        TypeSelector::Label => "label",
        TypeSelector::TextFrame => "text_frame",
        TypeSelector::Pad => "pad",
        TypeSelector::Track => "track",
        TypeSelector::Via => "via",
        TypeSelector::Fill => "fill",
        TypeSelector::Region => "region",
        TypeSelector::Text => "text",
        TypeSelector::PcbArc => "pcb_arc",
        TypeSelector::ComponentBody => "component_body",
    }
}

/// Get the field definitions for a given type selector.
pub fn fields_for_type(ts: TypeSelector) -> &'static [FieldDef] {
    match ts {
        TypeSelector::Component => COMPONENT_FIELDS,
        TypeSelector::Pin => PIN_FIELDS,
        TypeSelector::Parameter => PARAMETER_FIELDS,
        TypeSelector::Footprint => FOOTPRINT_FIELDS,
        TypeSelector::Graphic
        | TypeSelector::Line
        | TypeSelector::Rectangle
        | TypeSelector::RoundRectangle
        | TypeSelector::Arc
        | TypeSelector::EllipticalArc
        | TypeSelector::Ellipse
        | TypeSelector::Pie
        | TypeSelector::Polyline
        | TypeSelector::Polygon
        | TypeSelector::Bezier
        | TypeSelector::Image
        | TypeSelector::Label
        | TypeSelector::TextFrame => GRAPHIC_FIELDS,
        TypeSelector::Pad => PAD_FIELDS,
        TypeSelector::Track => TRACK_FIELDS,
        TypeSelector::Via => VIA_FIELDS,
        TypeSelector::Fill => FILL_FIELDS,
        TypeSelector::Region => REGION_FIELDS,
        TypeSelector::Text => TEXT_FIELDS,
        TypeSelector::PcbArc => PCB_ARC_FIELDS,
        TypeSelector::ComponentBody => COMPONENT_BODY_FIELDS,
    }
}

// ── Field definitions ────────────────────────────────────────────────────────

static COMPONENT_FIELDS: &[FieldDef] = &[
    FieldDef { canonical_name: "lib_reference", aliases: &["LibReference", "name"], field_type: FieldType::String },
    FieldDef { canonical_name: "designator", aliases: &["Designator"], field_type: FieldType::String },
    FieldDef { canonical_name: "description", aliases: &["Description"], field_type: FieldType::String },
    FieldDef { canonical_name: "component_kind", aliases: &["ComponentKind", "kind"], field_type: FieldType::Enum },
    FieldDef { canonical_name: "part_count", aliases: &["PartCount"], field_type: FieldType::Integer },
    FieldDef { canonical_name: "show_hidden_pins", aliases: &["ShowHiddenPins"], field_type: FieldType::Bool },
];

static PIN_FIELDS: &[FieldDef] = &[
    FieldDef { canonical_name: "designator", aliases: &["Designator"], field_type: FieldType::String },
    FieldDef { canonical_name: "name", aliases: &["Name", "PinName"], field_type: FieldType::String },
    FieldDef { canonical_name: "electrical", aliases: &["Electrical", "ElectricalType"], field_type: FieldType::Enum },
    FieldDef { canonical_name: "x", aliases: &["X"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "y", aliases: &["Y"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "length", aliases: &["Length", "PinLength"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "orientation", aliases: &["Orientation", "Rotation"], field_type: FieldType::Enum },
    FieldDef { canonical_name: "is_hidden", aliases: &["IsHidden", "Hidden"], field_type: FieldType::Bool },
    FieldDef { canonical_name: "hidden_net_name", aliases: &["HiddenNetName"], field_type: FieldType::String },
    FieldDef { canonical_name: "owner_part_id", aliases: &["OwnerPartId", "Part"], field_type: FieldType::Integer },
    FieldDef { canonical_name: "show_name", aliases: &["ShowName"], field_type: FieldType::Bool },
    FieldDef { canonical_name: "show_designator", aliases: &["ShowDesignator"], field_type: FieldType::Bool },
    FieldDef { canonical_name: "description", aliases: &["Description"], field_type: FieldType::String },
    FieldDef { canonical_name: "unique_id", aliases: &["UniqueId"], field_type: FieldType::String },
    FieldDef { canonical_name: "color", aliases: &["Color"], field_type: FieldType::Color },
    FieldDef { canonical_name: "is_not_accessible", aliases: &["IsNotAccessible"], field_type: FieldType::Bool },
    FieldDef { canonical_name: "graphically_locked", aliases: &["GraphicallyLocked"], field_type: FieldType::Bool },
    FieldDef { canonical_name: "owner_part_display_mode", aliases: &["OwnerPartDisplayMode"], field_type: FieldType::Integer },
];

static PARAMETER_FIELDS: &[FieldDef] = &[
    FieldDef { canonical_name: "name", aliases: &["Name"], field_type: FieldType::String },
    FieldDef { canonical_name: "text", aliases: &["Text", "Value"], field_type: FieldType::String },
    FieldDef { canonical_name: "is_hidden", aliases: &["IsHidden", "Hidden"], field_type: FieldType::Bool },
    FieldDef { canonical_name: "read_only", aliases: &["ReadOnly"], field_type: FieldType::Enum },
    FieldDef { canonical_name: "x", aliases: &["X"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "y", aliases: &["Y"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "orientation", aliases: &["Orientation"], field_type: FieldType::Enum },
    FieldDef { canonical_name: "color", aliases: &["Color"], field_type: FieldType::Color },
    FieldDef { canonical_name: "font_id", aliases: &["FontId"], field_type: FieldType::Integer },
    FieldDef { canonical_name: "justification", aliases: &["Justification"], field_type: FieldType::Enum },
    FieldDef { canonical_name: "is_mirrored", aliases: &["IsMirrored"], field_type: FieldType::Bool },
    FieldDef { canonical_name: "show_name", aliases: &["ShowName"], field_type: FieldType::Bool },
    FieldDef { canonical_name: "unique_id", aliases: &["UniqueId"], field_type: FieldType::String },
    FieldDef { canonical_name: "not_auto_position", aliases: &["NotAutoPosition"], field_type: FieldType::Bool },
    FieldDef { canonical_name: "param_type", aliases: &["ParamType", "Type"], field_type: FieldType::Enum },
    FieldDef { canonical_name: "description", aliases: &["Description"], field_type: FieldType::String },
];

static FOOTPRINT_FIELDS: &[FieldDef] = &[
    FieldDef { canonical_name: "model_name", aliases: &["ModelName", "name"], field_type: FieldType::String },
    FieldDef { canonical_name: "description", aliases: &["Description"], field_type: FieldType::String },
    FieldDef { canonical_name: "is_current", aliases: &["IsCurrent"], field_type: FieldType::Bool },
];

// Common graphic fields (shared across graphic variants)
static GRAPHIC_FIELDS: &[FieldDef] = &[
    FieldDef { canonical_name: "unique_id", aliases: &["UniqueId"], field_type: FieldType::String },
    FieldDef { canonical_name: "owner_part_id", aliases: &["OwnerPartId", "Part"], field_type: FieldType::Integer },
    FieldDef { canonical_name: "x", aliases: &["X"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "y", aliases: &["Y"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "color", aliases: &["Color"], field_type: FieldType::Color },
    FieldDef { canonical_name: "is_solid", aliases: &["IsSolid"], field_type: FieldType::Bool },
];

// ── PcbLib field definitions ─────────────────────────────────────────────────

static PCB_FOOTPRINT_FIELDS: &[FieldDef] = &[
    FieldDef { canonical_name: "display_name", aliases: &["DisplayName", "name"], field_type: FieldType::String },
    FieldDef { canonical_name: "description", aliases: &["Description"], field_type: FieldType::String },
    FieldDef { canonical_name: "pattern", aliases: &["Pattern"], field_type: FieldType::String },
    FieldDef { canonical_name: "height", aliases: &["Height"], field_type: FieldType::Coord },
];

static PAD_FIELDS: &[FieldDef] = &[
    FieldDef { canonical_name: "pad_name", aliases: &["PadName", "name", "designator"], field_type: FieldType::String },
    FieldDef { canonical_name: "x", aliases: &["X"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "y", aliases: &["Y"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "shape", aliases: &["Shape"], field_type: FieldType::Enum },
    FieldDef { canonical_name: "x_size", aliases: &["XSize", "Width"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "y_size", aliases: &["YSize", "Height"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "rotation", aliases: &["Rotation"], field_type: FieldType::Float },
    FieldDef { canonical_name: "hole_size", aliases: &["HoleSize"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "is_plated", aliases: &["IsPlated", "Plated"], field_type: FieldType::Bool },
    FieldDef { canonical_name: "layer", aliases: &["Layer"], field_type: FieldType::Enum },
    FieldDef { canonical_name: "pad_mode", aliases: &["PadMode"], field_type: FieldType::Enum },
    FieldDef { canonical_name: "solder_mask_expansion", aliases: &["SolderMaskExpansion"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "paste_mask_expansion", aliases: &["PasteMaskExpansion"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "plane_connection", aliases: &["PlaneConnection"], field_type: FieldType::Enum },
    FieldDef { canonical_name: "relief_conductor_width", aliases: &["ReliefConductorWidth"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "relief_entries", aliases: &["ReliefEntries"], field_type: FieldType::Integer },
    FieldDef { canonical_name: "relief_air_gap", aliases: &["ReliefAirGap"], field_type: FieldType::Coord },
];

static TRACK_FIELDS: &[FieldDef] = &[
    FieldDef { canonical_name: "layer", aliases: &["Layer"], field_type: FieldType::Enum },
    FieldDef { canonical_name: "width", aliases: &["Width"], field_type: FieldType::Coord },
];

static VIA_FIELDS: &[FieldDef] = &[
    FieldDef { canonical_name: "layer", aliases: &["Layer"], field_type: FieldType::Enum },
    FieldDef { canonical_name: "x", aliases: &["X"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "y", aliases: &["Y"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "diameter", aliases: &["Diameter"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "hole_size", aliases: &["HoleSize"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "from_layer", aliases: &["FromLayer"], field_type: FieldType::Enum },
    FieldDef { canonical_name: "to_layer", aliases: &["ToLayer"], field_type: FieldType::Enum },
];

static FILL_FIELDS: &[FieldDef] = &[
    FieldDef { canonical_name: "layer", aliases: &["Layer"], field_type: FieldType::Enum },
    FieldDef { canonical_name: "rotation", aliases: &["Rotation"], field_type: FieldType::Float },
];

static REGION_FIELDS: &[FieldDef] = &[
    FieldDef { canonical_name: "layer", aliases: &["Layer"], field_type: FieldType::Enum },
];

static TEXT_FIELDS: &[FieldDef] = &[
    FieldDef { canonical_name: "layer", aliases: &["Layer"], field_type: FieldType::Enum },
    FieldDef { canonical_name: "text", aliases: &["Text"], field_type: FieldType::String },
    FieldDef { canonical_name: "x", aliases: &["X"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "y", aliases: &["Y"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "rotation", aliases: &["Rotation"], field_type: FieldType::Float },
    FieldDef { canonical_name: "height", aliases: &["Height"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "width", aliases: &["Width"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "color", aliases: &["Color"], field_type: FieldType::Color },
];

static PCB_ARC_FIELDS: &[FieldDef] = &[
    FieldDef { canonical_name: "layer", aliases: &["Layer"], field_type: FieldType::Enum },
    FieldDef { canonical_name: "x", aliases: &["X"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "y", aliases: &["Y"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "radius", aliases: &["Radius"], field_type: FieldType::Coord },
    FieldDef { canonical_name: "start_angle", aliases: &["StartAngle"], field_type: FieldType::Float },
    FieldDef { canonical_name: "end_angle", aliases: &["EndAngle"], field_type: FieldType::Float },
    FieldDef { canonical_name: "width", aliases: &["Width"], field_type: FieldType::Coord },
];

static COMPONENT_BODY_FIELDS: &[FieldDef] = &[
    FieldDef { canonical_name: "layer", aliases: &["Layer"], field_type: FieldType::Enum },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_canonical() {
        let (def, name) = resolve_field(TypeSelector::Component, "lib_reference").unwrap();
        assert_eq!(name, "lib_reference");
        assert_eq!(def.field_type, FieldType::String);
    }

    #[test]
    fn test_resolve_alias() {
        let (_, name) = resolve_field(TypeSelector::Component, "LibReference").unwrap();
        assert_eq!(name, "lib_reference");
    }

    #[test]
    fn test_resolve_case_insensitive() {
        let (_, name) = resolve_field(TypeSelector::Component, "LIBREFERENCE").unwrap();
        assert_eq!(name, "lib_reference");
    }

    #[test]
    fn test_resolve_unknown_field() {
        let err = resolve_field(TypeSelector::Component, "nonexistent").unwrap_err();
        assert_eq!(err.code, QueryErrorCode::UnknownField);
    }

    #[test]
    fn test_op_compatibility_string() {
        assert!(FieldType::String.is_op_compatible(CompareOp::Eq));
        assert!(FieldType::String.is_op_compatible(CompareOp::Contains));
        assert!(!FieldType::String.is_op_compatible(CompareOp::Gt));
    }

    #[test]
    fn test_op_compatibility_coord() {
        assert!(FieldType::Coord.is_op_compatible(CompareOp::Gt));
        assert!(FieldType::Coord.is_op_compatible(CompareOp::Lte));
        assert!(!FieldType::Coord.is_op_compatible(CompareOp::Contains));
    }

    #[test]
    fn test_op_compatibility_bool() {
        assert!(FieldType::Bool.is_op_compatible(CompareOp::Eq));
        assert!(!FieldType::Bool.is_op_compatible(CompareOp::Gt));
    }
}

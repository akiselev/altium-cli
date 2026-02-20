//! Queryable integration for the v2 backing store types.
//!
//! This module implements the [`Queryable`] trait for [`RecordNode`], bridging
//! the backing store types with the AQL evaluator. It provides:
//!
//! - Field-to-parameter-key mapping for user-friendly query field names
//! - Type-aware value conversion (booleans, coordinates, integers, strings)
//! - Binary (PCB) field lookup from raw blocks
//! - Fallback uppercase lookup for unmapped custom fields

use crate::backing_store::{BinaryOrigin, RecordNode, RecordOrigin};
use crate::parameters::ParameterValue;

use super::eval::{QueryFieldValue, Queryable};

// ---------------------------------------------------------------------------
// Queryable impl for RecordNode
// ---------------------------------------------------------------------------

impl Queryable for RecordNode {
    fn record_id(&self) -> u8 {
        self.key
    }

    fn get_field(&self, field: &str) -> Option<QueryFieldValue> {
        match &self.origin {
            RecordOrigin::Param(p) => get_param_field(&p.params, field),
            RecordOrigin::Binary(b) => binary_field_lookup(field, b),
        }
    }
}

// ---------------------------------------------------------------------------
// Parameter-based field lookup
// ---------------------------------------------------------------------------

/// Look up a field in a parameter collection, trying the static mapping first
/// and falling back to the uppercase field name.
fn get_param_field(
    params: &crate::parameters::ParameterCollection,
    field: &str,
) -> Option<QueryFieldValue> {
    // Try static mapping first
    if let Some(key) = field_to_param_key(field) {
        if let Some(value) = params.get(key) {
            return Some(param_value_to_query_value(&value, key));
        }
    }
    // Fallback: try uppercase field name directly
    let upper = field.to_ascii_uppercase();
    if let Some(value) = params.get(&upper) {
        return Some(param_value_to_query_value(&value, &upper));
    }
    None
}

/// Maps user-facing query field names (lowercase) to Altium parameter keys
/// (uppercase).
///
/// The evaluator lowercases field names before calling `get_field`, so all
/// match arms expect lowercase input.
fn field_to_param_key(field: &str) -> Option<&'static str> {
    match field {
        // Component fields
        "designator" => Some("DESIGNATOR"),
        "name" => Some("NAME"),
        "description" | "desc" | "componentdescription" => Some("COMPONENTDESCRIPTION"),
        "value" | "comment" => Some("COMMENT"),
        "libreference" | "libref" | "lib_reference" => Some("LIBREFERENCE"),
        "footprint" => Some("FOOTPRINT"),
        "partcount" | "part_count" => Some("PARTCOUNT"),
        "displaymodecount" => Some("DISPLAYMODECOUNT"),

        // Location
        "x" | "location.x" => Some("LOCATION.X"),
        "y" | "location.y" => Some("LOCATION.Y"),

        // Pin fields
        "pinlength" | "pin_length" => Some("PINLENGTH"),
        "electrical" => Some("ELECTRICAL"),
        "pinconglomerate" => Some("PINCONGLOMERATE"),
        "formaltype" => Some("FORMALTYPE"),

        // Visual properties
        "color" => Some("COLOR"),
        "areacolor" | "area_color" => Some("AREACOLOR"),
        "linewidth" | "line_width" | "width" => Some("LINEWIDTH"),
        "issolid" | "is_solid" | "solid" => Some("ISSOLID"),
        "visible" => Some("VISIBLE"),
        "hidden" | "ishidden" => Some("ISHIDDEN"),
        "locked" | "islocked" => Some("ISLOCKED"),
        "mirrored" | "ismirrored" => Some("ISMIRRORED"),

        // Geometry
        "orientation" | "rotation" => Some("ORIENTATION"),
        "radius" => Some("RADIUS"),
        "startangle" | "start_angle" => Some("STARTANGLE"),
        "endangle" | "end_angle" => Some("ENDANGLE"),
        "corner.x" => Some("CORNER.X"),
        "corner.y" => Some("CORNER.Y"),

        // Sheet/port
        "sheetname" | "sheet_name" => Some("SHEETNAME"),
        "filename" => Some("FILENAME"),
        "text" => Some("TEXT"),

        // Net
        "net" => Some("NET"),
        "netname" | "net_name" => Some("NETNAME"),

        // Owner references
        "ownerindex" | "owner_index" => Some("OWNERINDEX"),
        "ownerpartid" | "owner_part_id" => Some("OWNERPARTID"),

        // Identifiers
        "uniqueid" | "unique_id" => Some("UNIQUEID"),
        "record" => Some("RECORD"),

        // Not in the static map
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Value conversion
// ---------------------------------------------------------------------------

/// Converts a `ParameterValue` to a `QueryFieldValue` based on the parameter
/// key, using heuristics for known boolean, coordinate, and integer fields.
fn param_value_to_query_value(value: &ParameterValue, key: &str) -> QueryFieldValue {
    // Known boolean fields
    const BOOL_KEYS: &[&str] = &[
        "ISSOLID",
        "VISIBLE",
        "ISHIDDEN",
        "ISLOCKED",
        "ISMIRRORED",
        "TRANSPARENT",
        "ISNOTACCESIBLE",
    ];
    if BOOL_KEYS.iter().any(|&k| k.eq_ignore_ascii_case(key)) {
        return QueryFieldValue::Bool(value.as_bool_or(false));
    }

    // Known coordinate fields (return as mils)
    const COORD_KEYS: &[&str] = &[
        "LOCATION.X",
        "LOCATION.Y",
        "CORNER.X",
        "CORNER.Y",
        "RADIUS",
        "LINEWIDTH",
        "PINLENGTH",
        "X1",
        "Y1",
        "X2",
        "Y2",
    ];
    if COORD_KEYS.iter().any(|&k| k.eq_ignore_ascii_case(key)) {
        return QueryFieldValue::Coord(value.as_int_or(0) as f64);
    }

    // Known integer fields
    const INT_KEYS: &[&str] = &[
        "RECORD",
        "OWNERINDEX",
        "OWNERPARTID",
        "OWNERPARTDISPLAYMODE",
        "PARTCOUNT",
        "DISPLAYMODECOUNT",
        "ORIENTATION",
        "ELECTRICAL",
        "PINCONGLOMERATE",
        "FORMALTYPE",
        "COLOR",
        "AREACOLOR",
    ];
    if INT_KEYS.iter().any(|&k| k.eq_ignore_ascii_case(key)) {
        return QueryFieldValue::Int(value.as_int_or(0));
    }

    // Default: return as string
    QueryFieldValue::String(value.as_str().to_string())
}

// ---------------------------------------------------------------------------
// Binary (PCB) field lookup
// ---------------------------------------------------------------------------

/// Looks up a field in a binary (PCB) record.
///
/// PCB binary records have a 13-byte common header:
/// - `[0]`: layer (u8)
/// - `[1..3]`: flags (u16 LE)
/// - `[3..5]`: net (u16 LE)
/// - `[5..7]`: polygon_ref (u16 LE)
/// - `[7..9]`: component_ref (u16 LE)
/// - `[9..11]`: ref4 (u16 LE)
/// - `[11..13]`: ref5 (u16 LE)
fn binary_field_lookup(field: &str, binary: &BinaryOrigin) -> Option<QueryFieldValue> {
    if binary.raw_block.len() < 13 {
        return None;
    }

    match field {
        "layer" => Some(QueryFieldValue::Int(binary.raw_block[0] as i32)),
        "net" => {
            let net = u16::from_le_bytes([binary.raw_block[3], binary.raw_block[4]]);
            Some(QueryFieldValue::Int(net as i32))
        }
        _ => None,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{ParamOrigin, RecordNode, RecordOrigin};
    use crate::query::eval::{Queryable, evaluate};
    use crate::query::parse;

    #[test]
    fn record_node_queryable_record_id() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=U1|"));
        let node = RecordNode::new(1, origin);
        assert_eq!(node.record_id(), 1);
    }

    #[test]
    fn record_node_queryable_get_field() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=1|DESIGNATOR=U1|LIBREFERENCE=LM358|",
        ));
        let node = RecordNode::new(1, origin);

        match node.get_field("designator") {
            Some(QueryFieldValue::String(s)) => assert_eq!(s, "U1"),
            other => panic!("expected String, got: {other:?}"),
        }

        match node.get_field("libreference") {
            Some(QueryFieldValue::String(s)) => assert_eq!(s, "LM358"),
            other => panic!("expected String, got: {other:?}"),
        }
    }

    #[test]
    fn record_node_queryable_missing_field() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=U1|"));
        let node = RecordNode::new(1, origin);
        assert!(node.get_field("nonexistent").is_none());
    }

    #[test]
    fn record_node_query_designator() {
        let nodes = vec![
            RecordNode::new(
                1,
                RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=U1|")),
            ),
            RecordNode::new(
                1,
                RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=R1|")),
            ),
            RecordNode::new(
                1,
                RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=U2|")),
            ),
        ];

        let q = parse("U*").unwrap();
        let results = evaluate(&q, &nodes);
        assert_eq!(results, vec![0, 2]);
    }

    #[test]
    fn record_node_query_element_type() {
        let nodes = vec![
            RecordNode::new(1, RecordOrigin::Param(ParamOrigin::new("|RECORD=1|"))),
            RecordNode::new(
                2,
                RecordOrigin::Param(ParamOrigin::new("|RECORD=2|NAME=VCC|")),
            ),
            RecordNode::new(1, RecordOrigin::Param(ParamOrigin::new("|RECORD=1|"))),
        ];

        let q = parse("component").unwrap();
        let results = evaluate(&q, &nodes);
        assert_eq!(results, vec![0, 2]);
    }

    #[test]
    fn record_node_query_attr_filter() {
        let nodes = vec![
            RecordNode::new(
                1,
                RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=R1|COMMENT=10K|")),
            ),
            RecordNode::new(
                1,
                RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=R2|COMMENT=100K|")),
            ),
        ];

        let q = parse("component[value=10K]").unwrap();
        let results = evaluate(&q, &nodes);
        assert_eq!(results, vec![0]);
    }

    #[test]
    fn record_node_fallback_uppercase_field() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=1|CUSTOMFIELD=hello|"));
        let node = RecordNode::new(1, origin);
        match node.get_field("customfield") {
            Some(QueryFieldValue::String(s)) => assert_eq!(s, "hello"),
            other => panic!("expected String, got: {other:?}"),
        }
    }

    #[test]
    fn record_node_boolean_field() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=1|ISSOLID=T|ISLOCKED=F|"));
        let node = RecordNode::new(1, origin);

        match node.get_field("issolid") {
            Some(QueryFieldValue::Bool(b)) => assert!(b),
            other => panic!("expected Bool(true), got: {other:?}"),
        }

        match node.get_field("locked") {
            Some(QueryFieldValue::Bool(b)) => assert!(!b),
            other => panic!("expected Bool(false), got: {other:?}"),
        }
    }

    #[test]
    fn record_node_integer_field() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=1|OWNERINDEX=5|"));
        let node = RecordNode::new(1, origin);

        match node.get_field("record") {
            Some(QueryFieldValue::Int(n)) => assert_eq!(n, 1),
            other => panic!("expected Int(1), got: {other:?}"),
        }

        match node.get_field("ownerindex") {
            Some(QueryFieldValue::Int(n)) => assert_eq!(n, 5),
            other => panic!("expected Int(5), got: {other:?}"),
        }
    }

    #[test]
    fn record_node_coord_field() {
        let origin =
            RecordOrigin::Param(ParamOrigin::new("|RECORD=1|LOCATION.X=100|LOCATION.Y=200|"));
        let node = RecordNode::new(1, origin);

        match node.get_field("x") {
            Some(QueryFieldValue::Coord(v)) => assert!((v - 100.0).abs() < f64::EPSILON),
            other => panic!("expected Coord(100.0), got: {other:?}"),
        }

        match node.get_field("location.y") {
            Some(QueryFieldValue::Coord(v)) => assert!((v - 200.0).abs() < f64::EPSILON),
            other => panic!("expected Coord(200.0), got: {other:?}"),
        }
    }

    #[test]
    fn binary_record_layer_field() {
        // Create a 13-byte binary block: layer=5, rest zeroed
        let mut block = vec![0u8; 13];
        block[0] = 5; // layer
        let origin = RecordOrigin::Binary(BinaryOrigin::new(block));
        let node = RecordNode::new(100, origin);

        match node.get_field("layer") {
            Some(QueryFieldValue::Int(n)) => assert_eq!(n, 5),
            other => panic!("expected Int(5), got: {other:?}"),
        }
    }

    #[test]
    fn binary_record_net_field() {
        // Create a 13-byte binary block with net=0x0102 at bytes 3..5
        let mut block = vec![0u8; 13];
        block[3] = 0x02;
        block[4] = 0x01; // little-endian 0x0102 = 258
        let origin = RecordOrigin::Binary(BinaryOrigin::new(block));
        let node = RecordNode::new(100, origin);

        match node.get_field("net") {
            Some(QueryFieldValue::Int(n)) => assert_eq!(n, 258),
            other => panic!("expected Int(258), got: {other:?}"),
        }
    }

    #[test]
    fn binary_record_too_short() {
        let block = vec![0u8; 5]; // less than 13 bytes
        let origin = RecordOrigin::Binary(BinaryOrigin::new(block));
        let node = RecordNode::new(100, origin);
        assert!(node.get_field("layer").is_none());
    }

    #[test]
    fn binary_record_unknown_field() {
        let block = vec![0u8; 13];
        let origin = RecordOrigin::Binary(BinaryOrigin::new(block));
        let node = RecordNode::new(100, origin);
        assert!(node.get_field("nonexistent").is_none());
    }
}

//! AQL query evaluator.
//!
//! Given a parsed [`AqlQuery`] and a collection of records implementing
//! [`Queryable`], the [`evaluate`] function returns the indices of all
//! matching records.

use super::ast::{
    AqlAttrFilter, AqlAttrOp, AqlAttrValue, AqlElementType, AqlExpr, AqlFactor, AqlPattern,
    AqlQuery, AqlSelector, AqlTerm, DesignatorSuffix,
};

// ---------------------------------------------------------------------------
// Queryable trait
// ---------------------------------------------------------------------------

/// Trait that records must implement to be queryable by AQL.
///
/// Implementors should return values via [`get_field`](Queryable::get_field)
/// for all fields relevant to their record type. Field names are
/// case-insensitive by convention; the evaluator lowercases field names
/// before calling `get_field`.
pub trait Queryable {
    /// The numeric record type identifier (e.g., 1 = Component, 2 = Pin).
    fn record_id(&self) -> u8;

    /// Look up a field by name. Returns `None` if the field does not exist
    /// on this record.
    fn get_field(&self, field: &str) -> Option<QueryFieldValue>;
}

/// A value returned by [`Queryable::get_field`].
#[derive(Debug, Clone)]
pub enum QueryFieldValue {
    /// UTF-8 string value.
    String(String),
    /// Integer value.
    Int(i32),
    /// Floating-point value.
    Float(f64),
    /// Boolean value.
    Bool(bool),
    /// Coordinate in mils (the Altium internal unit).
    Coord(f64),
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Evaluate a query against a collection of records.
///
/// Returns the indices (into `records`) of all matching records.
pub fn evaluate<Q: Queryable>(query: &AqlQuery, records: &[Q]) -> Vec<usize> {
    records
        .iter()
        .enumerate()
        .filter(|(_, r)| matches_expr(&query.expr, *r))
        .map(|(i, _)| i)
        .collect()
}

// ---------------------------------------------------------------------------
// Matching helpers
// ---------------------------------------------------------------------------

/// Does a record match an expression (OR of terms)?
fn matches_expr<Q: Queryable>(expr: &AqlExpr, record: &Q) -> bool {
    match expr {
        AqlExpr::Term(t) => matches_term(t, record),
        AqlExpr::Or(terms) => terms.iter().any(|t| matches_term(t, record)),
    }
}

/// Does a record match a term (AND of factors)?
fn matches_term<Q: Queryable>(term: &AqlTerm, record: &Q) -> bool {
    match term {
        AqlTerm::Factor(f) => matches_factor(f, record),
        AqlTerm::And(factors) => factors.iter().all(|f| matches_factor(f, record)),
    }
}

/// Does a record match a factor (optional NOT)?
fn matches_factor<Q: Queryable>(factor: &AqlFactor, record: &Q) -> bool {
    match factor {
        AqlFactor::Not(inner) => !matches_factor(inner, record),
        AqlFactor::Selector(sel) => matches_selector(sel, record),
    }
}

/// Does a record match a selector?
fn matches_selector<Q: Queryable>(selector: &AqlSelector, record: &Q) -> bool {
    match selector {
        AqlSelector::Pattern(pat) => matches_pattern(pat, record),
        AqlSelector::ElementType(et) => matches_element_type(et, record),
        AqlSelector::Compound(compound) => {
            // The base must match AND all attribute filters must match.
            matches_selector(&compound.base, record)
                && compound
                    .filters
                    .iter()
                    .all(|f| matches_attr_filter(f, record))
        }
    }
}

/// Does a record match an element type selector?
///
/// Maps known element types to record IDs. These IDs follow the Altium
/// schematic record numbering convention. PCB record IDs may differ; the
/// mapping can be extended as needed.
fn matches_element_type<Q: Queryable>(et: &AqlElementType, record: &Q) -> bool {
    let expected_id = element_type_to_record_id(et);
    match expected_id {
        Some(id) => record.record_id() == id,
        // Unknown mapping — fall back to checking a "type" field.
        None => false,
    }
}

/// Map an [`AqlElementType`] to a numeric record ID.
///
/// These are the standard Altium schematic record IDs. PCB record IDs use
/// a different numbering; integration code (Track 5C) can provide adapter
/// logic for PCB records.
fn element_type_to_record_id(et: &AqlElementType) -> Option<u8> {
    match et {
        AqlElementType::Component => Some(1),
        AqlElementType::Pin => Some(2),
        AqlElementType::Net => Some(3),
        AqlElementType::Wire => Some(27),
        AqlElementType::Bus => Some(26),
        AqlElementType::Port => Some(17),
        AqlElementType::Power => Some(11),
        AqlElementType::Label => Some(4),
        AqlElementType::NetLabel => Some(25),
        AqlElementType::Junction => Some(29),
        AqlElementType::Sheet => Some(15),
        AqlElementType::Parameter => Some(41),
        AqlElementType::Line => Some(13),
        AqlElementType::Arc => Some(12),
        AqlElementType::Text => Some(5),
        AqlElementType::Polygon => Some(36),
        AqlElementType::Rectangle => Some(14),
        // PCB-specific types — IDs are placeholders; real IDs come from
        // the PCB record format and will be finalized in Track 5C.
        AqlElementType::Pad => Some(100),
        AqlElementType::Via => Some(101),
        AqlElementType::Track => Some(102),
        AqlElementType::Fill => Some(103),
        AqlElementType::Region => Some(104),
        AqlElementType::Rule => Some(105),
    }
}

/// Does a record match a pattern selector?
fn matches_pattern<Q: Queryable>(pattern: &AqlPattern, record: &Q) -> bool {
    match pattern {
        AqlPattern::Designator(dp) => {
            // Check "designator" field first, fall back to "name".
            let designator = record
                .get_field("designator")
                .or_else(|| record.get_field("name"));
            match designator {
                Some(QueryFieldValue::String(s)) => matches_designator_pattern(dp, &s),
                _ => false,
            }
        }
        AqlPattern::Net(name) => {
            matches!(
                record.get_field("net"),
                Some(QueryFieldValue::String(ref s)) if s.eq_ignore_ascii_case(name)
            )
        }
        AqlPattern::Value(val) => {
            matches!(
                record.get_field("value"),
                Some(QueryFieldValue::String(ref s)) if s.eq_ignore_ascii_case(val)
            )
        }
        AqlPattern::Part(name) => {
            matches!(
                record.get_field("libreference"),
                Some(QueryFieldValue::String(ref s)) if s.eq_ignore_ascii_case(name)
            )
        }
        AqlPattern::Id(id) => record.record_id() == (*id as u8),
        AqlPattern::Pin(comp, pin) => {
            // A pin match requires the record to have both a component
            // field matching `comp` and a pin name matching `pin`.
            let comp_match = matches!(
                record.get_field("component"),
                Some(QueryFieldValue::String(ref s)) if s.eq_ignore_ascii_case(comp)
            );
            let pin_match = matches!(
                record.get_field("name"),
                Some(QueryFieldValue::String(ref s)) if s.eq_ignore_ascii_case(pin)
            );
            comp_match && pin_match
        }
    }
}

/// Check if a designator string matches a [`DesignatorPattern`].
fn matches_designator_pattern(dp: &super::ast::DesignatorPattern, designator: &str) -> bool {
    // Case-insensitive prefix check.
    let prefix_len = dp.prefix.len();
    if designator.len() < prefix_len {
        return false;
    }
    if !designator[..prefix_len].eq_ignore_ascii_case(&dp.prefix) {
        return false;
    }
    let remainder = &designator[prefix_len..];
    match &dp.suffix {
        DesignatorSuffix::Exact(s) => remainder.eq_ignore_ascii_case(s),
        DesignatorSuffix::Wildcard => true, // any suffix is OK
        DesignatorSuffix::SingleChar => remainder.len() == 1,
        DesignatorSuffix::DoubleChar => remainder.len() == 2,
    }
}

/// Does a record match an attribute filter?
fn matches_attr_filter<Q: Queryable>(filter: &AqlAttrFilter, record: &Q) -> bool {
    let field_name = filter.field.to_ascii_lowercase();
    let field_val = match record.get_field(&field_name) {
        Some(v) => v,
        None => return false,
    };

    match &filter.op {
        // String comparison operators.
        AqlAttrOp::Eq => compare_eq(&field_val, &filter.value),
        AqlAttrOp::Ne => !compare_eq(&field_val, &filter.value),
        AqlAttrOp::Contains => compare_string_op(&field_val, &filter.value, |hay, needle| {
            hay.to_ascii_lowercase()
                .contains(&needle.to_ascii_lowercase())
        }),
        AqlAttrOp::StartsWith => compare_string_op(&field_val, &filter.value, |hay, needle| {
            hay.to_ascii_lowercase()
                .starts_with(&needle.to_ascii_lowercase())
        }),
        AqlAttrOp::EndsWith => compare_string_op(&field_val, &filter.value, |hay, needle| {
            hay.to_ascii_lowercase()
                .ends_with(&needle.to_ascii_lowercase())
        }),
        AqlAttrOp::WordMatch => compare_string_op(&field_val, &filter.value, |hay, needle| {
            hay.split_whitespace()
                .any(|w| w.eq_ignore_ascii_case(needle))
        }),
        // Numeric / ordering operators.
        AqlAttrOp::Gt => compare_numeric(&field_val, &filter.value, |a, b| a > b),
        AqlAttrOp::Lt => compare_numeric(&field_val, &filter.value, |a, b| a < b),
        AqlAttrOp::Gte => compare_numeric(&field_val, &filter.value, |a, b| a >= b),
        AqlAttrOp::Lte => compare_numeric(&field_val, &filter.value, |a, b| a <= b),
    }
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

/// Equality comparison, handling type coercion between field value and query
/// value.
fn compare_eq(field: &QueryFieldValue, query_val: &AqlAttrValue) -> bool {
    match (field, query_val) {
        (QueryFieldValue::String(a), AqlAttrValue::String(b)) => a.eq_ignore_ascii_case(b),
        (QueryFieldValue::Int(a), AqlAttrValue::Number(b)) => (*a as f64 - b).abs() < 0.5,
        (QueryFieldValue::Float(a), AqlAttrValue::Number(b)) => (a - b).abs() < f64::EPSILON,
        (QueryFieldValue::Bool(a), AqlAttrValue::Bool(b)) => a == b,
        (QueryFieldValue::Coord(a), AqlAttrValue::Coord(b, unit)) => {
            let b_mils = coord_to_mils(*b, unit);
            (a - b_mils).abs() < 0.01
        }
        (QueryFieldValue::Coord(a), AqlAttrValue::Number(b)) => {
            // Bare number treated as mils.
            (a - b).abs() < 0.01
        }
        // Cross-type: try to parse string as number for comparison.
        (QueryFieldValue::String(a), AqlAttrValue::Number(b)) => a
            .parse::<f64>()
            .map(|n| (n - b).abs() < 0.5)
            .unwrap_or(false),
        (QueryFieldValue::Int(a), AqlAttrValue::String(b)) => {
            b.parse::<i32>().map(|n| *a == n).unwrap_or(false)
        }
        _ => false,
    }
}

/// Apply a string comparison function.
fn compare_string_op(
    field: &QueryFieldValue,
    query_val: &AqlAttrValue,
    f: impl Fn(&str, &str) -> bool,
) -> bool {
    let field_str = field_value_to_string(field);
    let query_str = attr_value_to_string(query_val);
    f(&field_str, &query_str)
}

/// Apply a numeric comparison function.
fn compare_numeric(
    field: &QueryFieldValue,
    query_val: &AqlAttrValue,
    cmp: impl Fn(f64, f64) -> bool,
) -> bool {
    let a = field_value_to_f64(field);
    let b = attr_value_to_f64(query_val);
    match (a, b) {
        (Some(a), Some(b)) => cmp(a, b),
        _ => false,
    }
}

/// Convert a [`QueryFieldValue`] to a string representation.
fn field_value_to_string(v: &QueryFieldValue) -> String {
    match v {
        QueryFieldValue::String(s) => s.clone(),
        QueryFieldValue::Int(n) => n.to_string(),
        QueryFieldValue::Float(n) => n.to_string(),
        QueryFieldValue::Bool(b) => b.to_string(),
        QueryFieldValue::Coord(n) => n.to_string(),
    }
}

/// Convert an [`AqlAttrValue`] to a string representation.
fn attr_value_to_string(v: &AqlAttrValue) -> String {
    match v {
        AqlAttrValue::String(s) => s.clone(),
        AqlAttrValue::Number(n) => n.to_string(),
        AqlAttrValue::Bool(b) => b.to_string(),
        AqlAttrValue::Coord(n, unit) => format!("{n}{unit}"),
    }
}

/// Try to extract an f64 from a field value.
fn field_value_to_f64(v: &QueryFieldValue) -> Option<f64> {
    match v {
        QueryFieldValue::Int(n) => Some(*n as f64),
        QueryFieldValue::Float(n) => Some(*n),
        QueryFieldValue::Coord(n) => Some(*n),
        QueryFieldValue::String(s) => s.parse::<f64>().ok(),
        QueryFieldValue::Bool(_) => None,
    }
}

/// Try to extract an f64 from a query attribute value.
fn attr_value_to_f64(v: &AqlAttrValue) -> Option<f64> {
    match v {
        AqlAttrValue::Number(n) => Some(*n),
        AqlAttrValue::Coord(n, unit) => Some(coord_to_mils(*n, unit)),
        AqlAttrValue::String(s) => s.parse::<f64>().ok(),
        AqlAttrValue::Bool(_) => None,
    }
}

/// Convert a coordinate value in the given unit to mils.
fn coord_to_mils(value: f64, unit: &str) -> f64 {
    match unit {
        "mil" => value,
        "mm" => value / 0.0254, // 1 mil = 0.0254 mm
        "in" => value * 1000.0, // 1 in = 1000 mil
        _ => value,             // unknown unit — treat as mils
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::query::parse;

    /// A mock record for testing the evaluator.
    struct MockRecord {
        id: u8,
        fields: Vec<(&'static str, QueryFieldValue)>,
    }

    impl MockRecord {
        fn new(id: u8, fields: Vec<(&'static str, QueryFieldValue)>) -> Self {
            Self { id, fields }
        }
    }

    impl Queryable for MockRecord {
        fn record_id(&self) -> u8 {
            self.id
        }

        fn get_field(&self, field: &str) -> Option<QueryFieldValue> {
            self.fields
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(field))
                .map(|(_, v)| v.clone())
        }
    }

    // Convenience: create a component record with a designator.
    fn component(designator: &'static str) -> MockRecord {
        MockRecord::new(
            1,
            vec![("designator", QueryFieldValue::String(designator.into()))],
        )
    }

    // Component with designator and value.
    fn component_with_value(designator: &'static str, value: &'static str) -> MockRecord {
        MockRecord::new(
            1,
            vec![
                ("designator", QueryFieldValue::String(designator.into())),
                ("value", QueryFieldValue::String(value.into())),
            ],
        )
    }

    // Component with designator and net.
    fn component_with_net(designator: &'static str, net: &'static str) -> MockRecord {
        MockRecord::new(
            1,
            vec![
                ("designator", QueryFieldValue::String(designator.into())),
                ("net", QueryFieldValue::String(net.into())),
            ],
        )
    }

    // Track record with width in mils.
    fn track_record(width_mils: f64) -> MockRecord {
        MockRecord::new(102, vec![("width", QueryFieldValue::Coord(width_mils))])
    }

    #[test]
    fn eval_designator_exact() {
        let q = parse("U1").unwrap();
        let records = vec![component("U1"), component("U2"), component("R1")];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn eval_wildcard() {
        let q = parse("R*").unwrap();
        let records = vec![
            component("R1"),
            component("R2"),
            component("R3"),
            component("U1"),
            component("C1"),
        ];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![0, 1, 2]);
    }

    #[test]
    fn eval_single_char_wildcard() {
        let q = parse("U?").unwrap();
        let records = vec![
            component("U1"),
            component("U2"),
            component("U10"),
            component("R1"),
        ];
        let result = evaluate(&q, &records);
        // U? matches U1 and U2 (single char after U), not U10 (two chars).
        assert_eq!(result, vec![0, 1]);
    }

    #[test]
    fn eval_double_char_wildcard() {
        let q = parse("C??").unwrap();
        let records = vec![
            component("C1"),
            component("C01"),
            component("C99"),
            component("C100"),
        ];
        let result = evaluate(&q, &records);
        // C?? matches C01 and C99 (two chars after C), not C1 or C100.
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn eval_net_pattern() {
        let q = parse("~VCC").unwrap();
        let records = vec![
            component_with_net("U1", "VCC"),
            component_with_net("U2", "GND"),
            component_with_net("R1", "VCC"),
        ];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![0, 2]);
    }

    #[test]
    fn eval_value_pattern() {
        let q = parse("@10K").unwrap();
        let records = vec![
            component_with_value("R1", "10K"),
            component_with_value("R2", "100K"),
            component_with_value("R3", "10K"),
        ];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![0, 2]);
    }

    #[test]
    fn eval_id_pattern() {
        let q = parse("#1").unwrap();
        let records = vec![
            MockRecord::new(1, vec![]),
            MockRecord::new(2, vec![]),
            MockRecord::new(1, vec![]),
        ];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![0, 2]);
    }

    #[test]
    fn eval_attr_eq() {
        let q = parse("component[value=10K]").unwrap();
        let records = vec![
            component_with_value("R1", "10K"),
            component_with_value("R2", "100K"),
            component_with_value("C1", "100nF"),
        ];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn eval_attr_gt() {
        let q = parse("track[width>=10mil]").unwrap();
        let records = vec![track_record(5.0), track_record(10.0), track_record(20.0)];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn eval_attr_lt() {
        let q = parse("track[width<10mil]").unwrap();
        let records = vec![track_record(5.0), track_record(10.0), track_record(20.0)];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn eval_attr_contains() {
        let q = parse("component[value*=0603]").unwrap();
        let records = vec![
            MockRecord::new(
                1,
                vec![("value", QueryFieldValue::String("RES_0603".into()))],
            ),
            MockRecord::new(
                1,
                vec![("value", QueryFieldValue::String("CAP_0402".into()))],
            ),
        ];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn eval_not() {
        // NOT R* — matches everything that is not an R-series designator.
        let q = parse("NOT R*").unwrap();
        let records = vec![component("R1"), component("U1"), component("C1")];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn eval_and() {
        // component AND [value=10K] — using the compound form.
        let q = parse("component[value=10K]").unwrap();
        let records = vec![
            component_with_value("R1", "10K"),
            component_with_value("R2", "100K"),
            // Non-component record with value 10K — should not match.
            MockRecord::new(2, vec![("value", QueryFieldValue::String("10K".into()))]),
        ];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn eval_or() {
        let q = parse("R* OR C*").unwrap();
        let records = vec![
            component("R1"),
            component("C1"),
            component("U1"),
            component("R2"),
        ];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![0, 1, 3]);
    }

    #[test]
    fn eval_element_type() {
        let q = parse("component").unwrap();
        let records = vec![
            MockRecord::new(1, vec![]), // component
            MockRecord::new(2, vec![]), // pin
            MockRecord::new(1, vec![]), // component
        ];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![0, 2]);
    }

    #[test]
    fn eval_compound_and_explicit() {
        // "component AND [value=10K]" using explicit AND syntax.
        let q = parse("component AND R*").unwrap();
        let records = vec![
            component("R1"), // record_id=1 (component) AND designator starts with R
            component("U1"), // record_id=1 but designator starts with U
            MockRecord::new(
                2,
                vec![("designator", QueryFieldValue::String("R2".into()))],
            ),
        ];
        let result = evaluate(&q, &records);
        // R1: component(id=1) AND R*(designator=R1) => matches
        // U1: component(id=1) AND R*(designator=U1) => no
        // R2: not component(id=2) => no
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn eval_coord_mm() {
        // 0.254mm = 10mil, so track[width>=0.254mm] should behave like track[width>=10mil]
        let q = parse("track[width>=0.254mm]").unwrap();
        let records = vec![track_record(5.0), track_record(10.0), track_record(20.0)];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn eval_attr_ne() {
        let q = parse("component[value!=10K]").unwrap();
        let records = vec![
            component_with_value("R1", "10K"),
            component_with_value("R2", "100K"),
            component_with_value("R3", "1K"),
        ];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn eval_attr_starts_with() {
        let q = parse("component[value^=10]").unwrap();
        let records = vec![
            component_with_value("R1", "10K"),
            component_with_value("R2", "100K"),
            component_with_value("R3", "1K"),
        ];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![0, 1]);
    }

    #[test]
    fn eval_attr_ends_with() {
        let q = parse("component[value$=K]").unwrap();
        let records = vec![
            component_with_value("R1", "10K"),
            component_with_value("C1", "100nF"),
            component_with_value("R2", "100K"),
        ];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![0, 2]);
    }

    #[test]
    fn eval_attr_word_match() {
        let q = parse("component[comment~=DNP]").unwrap();
        let records = vec![
            MockRecord::new(
                1,
                vec![(
                    "comment",
                    QueryFieldValue::String("DNP review needed".into()),
                )],
            ),
            MockRecord::new(
                1,
                vec![("comment", QueryFieldValue::String("no issues".into()))],
            ),
        ];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn eval_empty_records() {
        let q = parse("R*").unwrap();
        let records: Vec<MockRecord> = vec![];
        let result = evaluate(&q, &records);
        assert!(result.is_empty());
    }

    #[test]
    fn eval_pin_pattern() {
        let q = parse("U1:VCC").unwrap();
        let records = vec![
            MockRecord::new(
                2,
                vec![
                    ("component", QueryFieldValue::String("U1".into())),
                    ("name", QueryFieldValue::String("VCC".into())),
                ],
            ),
            MockRecord::new(
                2,
                vec![
                    ("component", QueryFieldValue::String("U1".into())),
                    ("name", QueryFieldValue::String("GND".into())),
                ],
            ),
            MockRecord::new(
                2,
                vec![
                    ("component", QueryFieldValue::String("U2".into())),
                    ("name", QueryFieldValue::String("VCC".into())),
                ],
            ),
        ];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn eval_part_pattern() {
        let q = parse("$LM358").unwrap();
        let records = vec![
            MockRecord::new(
                1,
                vec![("libreference", QueryFieldValue::String("LM358".into()))],
            ),
            MockRecord::new(
                1,
                vec![("libreference", QueryFieldValue::String("LM741".into()))],
            ),
        ];
        let result = evaluate(&q, &records);
        assert_eq!(result, vec![0]);
    }
}

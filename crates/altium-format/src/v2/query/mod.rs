//! Altium Query Language (AQL) parser and evaluator.
//!
//! This module provides the public [`parse`] function that turns a query string
//! into an [`ast::AqlQuery`] AST, and the [`eval`] module that evaluates queries
//! against collections of [`eval::Queryable`] records.

pub mod ast;
pub mod eval;
pub mod integration;

use pest::Parser;
use pest_derive::Parser;

use ast::{
    AqlAttrFilter, AqlAttrOp, AqlAttrValue, AqlCompoundSelector, AqlElementType, AqlExpr,
    AqlFactor, AqlPattern, AqlQuery, AqlSelector, AqlTerm, DesignatorPattern, DesignatorSuffix,
};

// ---------------------------------------------------------------------------
// Pest parser struct — the `grammar` attribute points to the .pest file
// relative to the crate `src/` directory.
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[grammar = "v2/query/grammar.pest"]
struct AqlParser;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a query string into an AST.
///
/// Returns an error with a descriptive message (including the position of the
/// offending character) when the query is syntactically invalid.
///
/// # Examples
///
/// ```ignore
/// let q = altium_format::v2::query::parse("R*")?;
/// ```
pub fn parse(query: &str) -> crate::Result<AqlQuery> {
    let pairs = AqlParser::parse(Rule::query, query)
        .map_err(|e| crate::AltiumError::Query(e.to_string()))?;
    build_query(pairs)
}

// ---------------------------------------------------------------------------
// AST builder helpers
// ---------------------------------------------------------------------------

/// Build the top-level [`AqlQuery`] from the pest parse tree.
///
/// `pest::Parser::parse` returns a `Pairs` whose single element is the
/// matched `Rule::query` pair. We unwrap that outer pair and then look for
/// the `Rule::expr` child inside it.
fn build_query(mut pairs: pest::iterators::Pairs<'_, Rule>) -> crate::Result<AqlQuery> {
    let query_pair = pairs
        .next()
        .ok_or_else(|| crate::AltiumError::Query("empty parse result".into()))?;
    for pair in query_pair.into_inner() {
        if pair.as_rule() == Rule::expr {
            return Ok(AqlQuery {
                expr: build_expr(pair)?,
            });
        }
    }
    Err(crate::AltiumError::Query(
        "expected expression in query".into(),
    ))
}

/// Build an [`AqlExpr`] (OR of terms) from a `Rule::expr` pair.
fn build_expr(pair: pest::iterators::Pair<'_, Rule>) -> crate::Result<AqlExpr> {
    let mut terms: Vec<AqlTerm> = Vec::new();
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::term => terms.push(build_term(inner)?),
            Rule::or_op => { /* skip the operator token */ }
            _ => {}
        }
    }
    match terms.len() {
        0 => Err(crate::AltiumError::Query(
            "expected at least one term in expression".into(),
        )),
        1 => Ok(AqlExpr::Term(terms.remove(0))),
        _ => Ok(AqlExpr::Or(terms)),
    }
}

/// Build an [`AqlTerm`] (AND of factors) from a `Rule::term` pair.
fn build_term(pair: pest::iterators::Pair<'_, Rule>) -> crate::Result<AqlTerm> {
    let mut factors: Vec<AqlFactor> = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::factor {
            factors.push(build_factor(inner)?);
        }
    }
    match factors.len() {
        0 => Err(crate::AltiumError::Query(
            "expected at least one factor in term".into(),
        )),
        1 => Ok(AqlTerm::Factor(factors.remove(0))),
        _ => Ok(AqlTerm::And(factors)),
    }
}

/// Build an [`AqlFactor`] from a `Rule::factor` pair.
///
/// A factor is either `NOT factor` (recursive) or a `selector`.
fn build_factor(pair: pest::iterators::Pair<'_, Rule>) -> crate::Result<AqlFactor> {
    let mut inner_pairs = pair.into_inner();
    let first = inner_pairs
        .next()
        .ok_or_else(|| crate::AltiumError::Query("expected content inside factor".into()))?;
    match first.as_rule() {
        Rule::factor => {
            // This is the `NOT ~ factor` branch — the keyword "NOT" was
            // consumed implicitly; the first inner child is the nested factor.
            Ok(AqlFactor::Not(Box::new(build_factor(first)?)))
        }
        Rule::selector => Ok(AqlFactor::Selector(build_selector(first)?)),
        other => Err(crate::AltiumError::Query(format!(
            "unexpected rule {other:?} inside factor"
        ))),
    }
}

/// Build an [`AqlSelector`] from a `Rule::selector` pair.
///
/// The grammar defines: `selector = { compound_selector | element_type | pattern }`
fn build_selector(pair: pest::iterators::Pair<'_, Rule>) -> crate::Result<AqlSelector> {
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| crate::AltiumError::Query("expected content inside selector".into()))?;
    match inner.as_rule() {
        Rule::compound_selector => build_compound_selector(inner),
        Rule::element_type => Ok(AqlSelector::ElementType(parse_element_type(inner)?)),
        Rule::pattern => {
            let pat = build_pattern(inner)?;
            Ok(AqlSelector::Pattern(pat))
        }
        other => Err(crate::AltiumError::Query(format!(
            "unexpected rule {other:?} inside selector"
        ))),
    }
}

/// Build an [`AqlSelector::Compound`] from a `Rule::compound_selector` pair.
fn build_compound_selector(pair: pest::iterators::Pair<'_, Rule>) -> crate::Result<AqlSelector> {
    let mut base: Option<AqlSelector> = None;
    let mut filters: Vec<AqlAttrFilter> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::element_type => {
                base = Some(AqlSelector::ElementType(parse_element_type(inner)?));
            }
            Rule::pattern => {
                base = Some(AqlSelector::Pattern(build_pattern(inner)?));
            }
            Rule::attr_sel => {
                filters.push(build_attr_sel(inner)?);
            }
            _ => {}
        }
    }

    let base =
        base.ok_or_else(|| crate::AltiumError::Query("compound selector missing base".into()))?;

    if filters.is_empty() {
        // Degenerate compound — return the bare selector.
        Ok(base)
    } else {
        Ok(AqlSelector::Compound(AqlCompoundSelector {
            base: Box::new(base),
            filters,
        }))
    }
}

/// Build an [`AqlPattern`] from a `Rule::pattern` pair.
fn build_pattern(pair: pest::iterators::Pair<'_, Rule>) -> crate::Result<AqlPattern> {
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| crate::AltiumError::Query("expected content inside pattern".into()))?;
    match inner.as_rule() {
        Rule::designator_pat => build_designator_pattern(inner),
        Rule::net_pat => {
            let name = inner
                .into_inner()
                .next()
                .ok_or_else(|| crate::AltiumError::Query("missing net name".into()))?
                .as_str()
                .to_owned();
            Ok(AqlPattern::Net(name))
        }
        Rule::value_pat => {
            let val = inner
                .into_inner()
                .next()
                .ok_or_else(|| crate::AltiumError::Query("missing value literal".into()))?
                .as_str()
                .to_owned();
            Ok(AqlPattern::Value(val))
        }
        Rule::part_pat => {
            let name = inner
                .into_inner()
                .next()
                .ok_or_else(|| crate::AltiumError::Query("missing part name".into()))?
                .as_str()
                .to_owned();
            Ok(AqlPattern::Part(name))
        }
        Rule::id_pat => {
            let id_str = inner
                .into_inner()
                .next()
                .ok_or_else(|| crate::AltiumError::Query("missing id".into()))?
                .as_str();
            let id: i32 = id_str.parse().map_err(|e| {
                crate::AltiumError::Query(format!("invalid record id '{id_str}': {e}"))
            })?;
            Ok(AqlPattern::Id(id))
        }
        Rule::pin_pat => {
            let mut parts = inner.into_inner();
            let comp = parts
                .next()
                .ok_or_else(|| {
                    crate::AltiumError::Query("missing component in pin pattern".into())
                })?
                .as_str()
                .to_owned();
            let pin = parts
                .next()
                .ok_or_else(|| crate::AltiumError::Query("missing pin in pin pattern".into()))?
                .as_str()
                .to_owned();
            Ok(AqlPattern::Pin(comp, pin))
        }
        other => Err(crate::AltiumError::Query(format!(
            "unexpected rule {other:?} inside pattern"
        ))),
    }
}

/// Parse a designator pattern string like `"U1"`, `"R*"`, `"C??"`, `"U?"`.
///
/// The grammar rule is atomic:
/// `designator_pat = @{ ASCII_ALPHA+ ~ (ASCII_ALPHANUMERIC | "_")* ~ ("*" | "??" | "?")? }`
///
/// So we get the entire matched text as a single string and split it ourselves.
fn build_designator_pattern(pair: pest::iterators::Pair<'_, Rule>) -> crate::Result<AqlPattern> {
    let text = pair.as_str();

    // Determine suffix type and split.
    if let Some(base) = text.strip_suffix('*') {
        let prefix = extract_alpha_prefix(base);
        Ok(AqlPattern::Designator(DesignatorPattern {
            prefix: prefix.to_owned(),
            suffix: DesignatorSuffix::Wildcard,
        }))
    } else if let Some(base) = text.strip_suffix("??") {
        let prefix = extract_alpha_prefix(base);
        Ok(AqlPattern::Designator(DesignatorPattern {
            prefix: prefix.to_owned(),
            suffix: DesignatorSuffix::DoubleChar,
        }))
    } else if let Some(base) = text.strip_suffix('?') {
        let prefix = extract_alpha_prefix(base);
        Ok(AqlPattern::Designator(DesignatorPattern {
            prefix: prefix.to_owned(),
            suffix: DesignatorSuffix::SingleChar,
        }))
    } else {
        // Exact designator like "U1", "C10", etc.
        let prefix = extract_alpha_prefix(text);
        let suffix_str = &text[prefix.len()..];
        Ok(AqlPattern::Designator(DesignatorPattern {
            prefix: prefix.to_owned(),
            suffix: DesignatorSuffix::Exact(suffix_str.to_owned()),
        }))
    }
}

/// Extract the leading alphabetic characters from a string.
fn extract_alpha_prefix(s: &str) -> &str {
    let end = s
        .char_indices()
        .find(|(_, c)| !c.is_ascii_alphabetic())
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    &s[..end]
}

/// Parse an `element_type` rule into [`AqlElementType`].
fn parse_element_type(pair: pest::iterators::Pair<'_, Rule>) -> crate::Result<AqlElementType> {
    let text = pair.as_str().to_ascii_lowercase();
    match text.as_str() {
        "component" => Ok(AqlElementType::Component),
        "pin" => Ok(AqlElementType::Pin),
        "net" => Ok(AqlElementType::Net),
        "wire" => Ok(AqlElementType::Wire),
        "bus" => Ok(AqlElementType::Bus),
        "port" => Ok(AqlElementType::Port),
        "power" => Ok(AqlElementType::Power),
        "label" => Ok(AqlElementType::Label),
        "netlabel" => Ok(AqlElementType::NetLabel),
        "junction" => Ok(AqlElementType::Junction),
        "sheet" => Ok(AqlElementType::Sheet),
        "parameter" => Ok(AqlElementType::Parameter),
        "line" => Ok(AqlElementType::Line),
        "arc" => Ok(AqlElementType::Arc),
        "text" => Ok(AqlElementType::Text),
        "polygon" => Ok(AqlElementType::Polygon),
        "rectangle" => Ok(AqlElementType::Rectangle),
        "pad" => Ok(AqlElementType::Pad),
        "via" => Ok(AqlElementType::Via),
        "track" => Ok(AqlElementType::Track),
        "fill" => Ok(AqlElementType::Fill),
        "region" => Ok(AqlElementType::Region),
        "rule" => Ok(AqlElementType::Rule),
        _ => Err(crate::AltiumError::Query(format!(
            "unknown element type '{text}'"
        ))),
    }
}

/// Build an [`AqlAttrFilter`] from a `Rule::attr_sel` pair.
fn build_attr_sel(pair: pest::iterators::Pair<'_, Rule>) -> crate::Result<AqlAttrFilter> {
    let mut field: Option<String> = None;
    let mut op: Option<AqlAttrOp> = None;
    let mut value: Option<AqlAttrValue> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::field_name => {
                field = Some(inner.as_str().to_owned());
            }
            Rule::attr_op => {
                op = Some(parse_attr_op(inner.as_str())?);
            }
            Rule::attr_value => {
                value = Some(build_attr_value(inner)?);
            }
            _ => {}
        }
    }

    Ok(AqlAttrFilter {
        field: field.ok_or_else(|| {
            crate::AltiumError::Query("missing field name in attribute selector".into())
        })?,
        op: op.ok_or_else(|| {
            crate::AltiumError::Query("missing operator in attribute selector".into())
        })?,
        value: value.ok_or_else(|| {
            crate::AltiumError::Query("missing value in attribute selector".into())
        })?,
    })
}

/// Parse an operator string into [`AqlAttrOp`].
fn parse_attr_op(s: &str) -> crate::Result<AqlAttrOp> {
    match s {
        "=" => Ok(AqlAttrOp::Eq),
        "!=" => Ok(AqlAttrOp::Ne),
        "*=" => Ok(AqlAttrOp::Contains),
        "^=" => Ok(AqlAttrOp::StartsWith),
        "$=" => Ok(AqlAttrOp::EndsWith),
        "~=" => Ok(AqlAttrOp::WordMatch),
        ">" => Ok(AqlAttrOp::Gt),
        "<" => Ok(AqlAttrOp::Lt),
        ">=" => Ok(AqlAttrOp::Gte),
        "<=" => Ok(AqlAttrOp::Lte),
        _ => Err(crate::AltiumError::Query(format!(
            "unknown attribute operator '{s}'"
        ))),
    }
}

/// Build an [`AqlAttrValue`] from a `Rule::attr_value` pair.
fn build_attr_value(pair: pest::iterators::Pair<'_, Rule>) -> crate::Result<AqlAttrValue> {
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| crate::AltiumError::Query("expected value inside attr_value".into()))?;
    match inner.as_rule() {
        Rule::coord_value => {
            let text = inner.as_str();
            // Split off the unit suffix.
            let (num_str, unit) = if let Some(n) = text.strip_suffix("mil") {
                (n, "mil")
            } else if let Some(n) = text.strip_suffix("mm") {
                (n, "mm")
            } else if let Some(n) = text.strip_suffix("in") {
                (n, "in")
            } else {
                return Err(crate::AltiumError::Query(format!(
                    "invalid coordinate value '{text}'"
                )));
            };
            let num: f64 = num_str.parse().map_err(|e| {
                crate::AltiumError::Query(format!("invalid number in coord '{num_str}': {e}"))
            })?;
            Ok(AqlAttrValue::Coord(num, unit.to_owned()))
        }
        Rule::quoted_string => {
            let raw = inner.as_str();
            // Strip surrounding quotes.
            let unquoted = &raw[1..raw.len() - 1];
            Ok(AqlAttrValue::String(unquoted.to_owned()))
        }
        Rule::number => {
            let num: f64 = inner.as_str().parse().map_err(|e| {
                crate::AltiumError::Query(format!("invalid number '{}': {e}", inner.as_str()))
            })?;
            Ok(AqlAttrValue::Number(num))
        }
        Rule::boolean => {
            let b = inner.as_str().eq_ignore_ascii_case("true");
            Ok(AqlAttrValue::Bool(b))
        }
        Rule::bare_string => Ok(AqlAttrValue::String(inner.as_str().to_owned())),
        other => Err(crate::AltiumError::Query(format!(
            "unexpected rule {other:?} inside attr_value"
        ))),
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ast::*;

    #[test]
    fn parse_simple_designator() {
        let q = parse("U1").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::Pattern(
                AqlPattern::Designator(d),
            )))) => {
                assert_eq!(d.prefix, "U");
                assert!(matches!(d.suffix, DesignatorSuffix::Exact(ref s) if s == "1"));
            }
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parse_wildcard() {
        let q = parse("R*").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::Pattern(
                AqlPattern::Designator(d),
            )))) => {
                assert_eq!(d.prefix, "R");
                assert!(matches!(d.suffix, DesignatorSuffix::Wildcard));
            }
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parse_single_char_wildcard() {
        let q = parse("U?").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::Pattern(
                AqlPattern::Designator(d),
            )))) => {
                assert_eq!(d.prefix, "U");
                assert!(matches!(d.suffix, DesignatorSuffix::SingleChar));
            }
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parse_double_char_wildcard() {
        let q = parse("C??").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::Pattern(
                AqlPattern::Designator(d),
            )))) => {
                assert_eq!(d.prefix, "C");
                assert!(matches!(d.suffix, DesignatorSuffix::DoubleChar));
            }
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parse_net() {
        let q = parse("~VCC").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::Pattern(
                AqlPattern::Net(name),
            )))) => {
                assert_eq!(name, "VCC");
            }
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parse_value() {
        let q = parse("@10K").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::Pattern(
                AqlPattern::Value(v),
            )))) => {
                assert_eq!(v, "10K");
            }
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parse_part() {
        let q = parse("$LM358").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::Pattern(
                AqlPattern::Part(name),
            )))) => {
                assert_eq!(name, "LM358");
            }
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parse_id() {
        let q = parse("#42").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::Pattern(
                AqlPattern::Id(id),
            )))) => {
                assert_eq!(*id, 42);
            }
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parse_pin() {
        let q = parse("U1:VCC").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::Pattern(
                AqlPattern::Pin(comp, pin),
            )))) => {
                assert_eq!(comp, "U1");
                assert_eq!(pin, "VCC");
            }
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parse_element_type_component() {
        let q = parse("component").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::ElementType(
                AqlElementType::Component,
            )))) => {}
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parse_element_type_case_insensitive() {
        let q = parse("COMPONENT").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::ElementType(
                AqlElementType::Component,
            )))) => {}
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parse_attr_filter() {
        let q = parse("component[value=10K]").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::Compound(c)))) => {
                assert!(matches!(
                    *c.base,
                    AqlSelector::ElementType(AqlElementType::Component)
                ));
                assert_eq!(c.filters.len(), 1);
                assert_eq!(c.filters[0].field, "value");
                assert_eq!(c.filters[0].op, AqlAttrOp::Eq);
                assert!(matches!(&c.filters[0].value, AqlAttrValue::String(s) if s == "10K"));
            }
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parse_attr_filter_coord() {
        let q = parse("track[width>=10mil]").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::Compound(c)))) => {
                assert!(matches!(
                    *c.base,
                    AqlSelector::ElementType(AqlElementType::Track)
                ));
                assert_eq!(c.filters.len(), 1);
                assert_eq!(c.filters[0].field, "width");
                assert_eq!(c.filters[0].op, AqlAttrOp::Gte);
                match &c.filters[0].value {
                    AqlAttrValue::Coord(v, u) => {
                        assert!((v - 10.0).abs() < f64::EPSILON);
                        assert_eq!(u, "mil");
                    }
                    other => panic!("expected Coord, got: {other:?}"),
                }
            }
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parse_attr_filter_number() {
        let q = parse("component[x>1000]").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::Compound(c)))) => {
                assert_eq!(c.filters[0].field, "x");
                assert_eq!(c.filters[0].op, AqlAttrOp::Gt);
                match &c.filters[0].value {
                    AqlAttrValue::Number(n) => {
                        assert!((n - 1000.0).abs() < f64::EPSILON);
                    }
                    other => panic!("expected Number, got: {other:?}"),
                }
            }
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parse_attr_filter_quoted_string() {
        let q = parse(r#"component[description="high power"]"#).unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::Compound(c)))) => {
                assert_eq!(c.filters[0].field, "description");
                assert_eq!(c.filters[0].op, AqlAttrOp::Eq);
                assert!(matches!(
                    &c.filters[0].value,
                    AqlAttrValue::String(s) if s == "high power"
                ));
            }
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parse_attr_filter_boolean() {
        let q = parse("component[locked=true]").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::Compound(c)))) => {
                assert!(matches!(&c.filters[0].value, AqlAttrValue::Bool(true)));
            }
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parse_multiple_attr_filters() {
        let q = parse("component[x>=1000][x<=3000]").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::Compound(c)))) => {
                assert_eq!(c.filters.len(), 2);
                assert_eq!(c.filters[0].field, "x");
                assert_eq!(c.filters[0].op, AqlAttrOp::Gte);
                assert_eq!(c.filters[1].field, "x");
                assert_eq!(c.filters[1].op, AqlAttrOp::Lte);
            }
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parse_compound() {
        let q = parse("component[value=10K]").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::Compound(c)))) => {
                assert!(matches!(
                    *c.base,
                    AqlSelector::ElementType(AqlElementType::Component)
                ));
                assert_eq!(c.filters.len(), 1);
            }
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parse_and_or() {
        let q = parse("R* OR C*").unwrap();
        match &q.expr {
            AqlExpr::Or(terms) => {
                assert_eq!(terms.len(), 2);
            }
            other => panic!("expected Or, got: {other:?}"),
        }
    }

    #[test]
    fn parse_and() {
        let q = parse("component AND R*").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::And(factors)) => {
                assert_eq!(factors.len(), 2);
            }
            other => panic!("expected And, got: {other:?}"),
        }
    }

    #[test]
    fn parse_not() {
        let q = parse("NOT R*").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Not(inner))) => {
                assert!(matches!(
                    **inner,
                    AqlFactor::Selector(AqlSelector::Pattern(AqlPattern::Designator(_)))
                ));
            }
            other => panic!("expected Not, got: {other:?}"),
        }
    }

    #[test]
    fn parse_complex_or() {
        // "component[value=10K] OR R*" — OR at top level.
        let q = parse("component[value=10K] OR R*").unwrap();
        match &q.expr {
            AqlExpr::Or(terms) => {
                assert_eq!(terms.len(), 2);
            }
            other => panic!("expected Or, got: {other:?}"),
        }
    }

    #[test]
    fn parse_comma_as_or() {
        let q = parse("R*, C*").unwrap();
        match &q.expr {
            AqlExpr::Or(terms) => {
                assert_eq!(terms.len(), 2);
            }
            other => panic!("expected Or (comma), got: {other:?}"),
        }
    }

    #[test]
    fn parse_error_invalid_query() {
        let result = parse("[[[");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("query error"), "error was: {err}");
    }

    #[test]
    fn parse_pattern_with_attr_sel() {
        // Designator pattern with attribute filter.
        let q = parse("R*[value=10K]").unwrap();
        match &q.expr {
            AqlExpr::Term(AqlTerm::Factor(AqlFactor::Selector(AqlSelector::Compound(c)))) => {
                assert!(matches!(
                    &*c.base,
                    AqlSelector::Pattern(AqlPattern::Designator(d)) if d.prefix == "R"
                ));
                assert_eq!(c.filters.len(), 1);
                assert_eq!(c.filters[0].field, "value");
            }
            other => panic!("unexpected AST: {other:?}"),
        }
    }
}

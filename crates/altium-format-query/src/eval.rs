use crate::adapter::{Queryable, QueryMatch, QueryNode, QueryResultSet};
use crate::ast::*;
use crate::diagnostic::Spanned;
use crate::error::{QueryError, QueryErrorCode, QueryResult};
use crate::value::{QueryValue, regex_matches};

/// Evaluate a parsed query against a queryable document.
pub fn eval_query(query: &Query, doc: &dyn Queryable) -> QueryResult<QueryResultSet> {
    let roots = doc.root_nodes()?;
    eval_expr(&query.expr, &roots)
}

fn eval_expr(expr: &Spanned<QueryExpr>, roots: &[QueryNode]) -> QueryResult<QueryResultSet> {
    match &expr.node {
        QueryExpr::Union(branches) => {
            let mut result = Vec::new();
            for branch in branches {
                let matches = eval_expr(branch, roots)?;
                merge_results(&mut result, matches);
            }
            Ok(result)
        }
        QueryExpr::Or(branches) => {
            let mut result = Vec::new();
            for branch in branches {
                let matches = eval_expr(branch, roots)?;
                merge_results(&mut result, matches);
            }
            Ok(result)
        }
        QueryExpr::And(branches) => {
            if branches.is_empty() {
                return Ok(Vec::new());
            }
            let mut result = eval_expr(&branches[0], roots)?;
            for branch in &branches[1..] {
                let other = eval_expr(branch, roots)?;
                result = intersect_results(result, &other);
            }
            Ok(result)
        }
        QueryExpr::Not(inner) => {
            let inner_matches = eval_expr(inner, roots)?;
            // NOT: return all candidates that are NOT in the inner set
            let all = collect_all_nodes(roots);
            let result = all
                .into_iter()
                .filter(|m| !inner_matches.iter().any(|im| nodes_equal(&m.node, &im.node)))
                .collect();
            Ok(result)
        }
        QueryExpr::Selector(chain) => eval_selector_chain(chain, roots),
    }
}

fn eval_selector_chain(
    chain: &SelectorChain,
    roots: &[QueryNode],
) -> QueryResult<QueryResultSet> {
    if chain.segments.is_empty() {
        return Ok(Vec::new());
    }

    // Start with the first segment against roots (and descendants for non-child)
    let first = &chain.segments[0];
    let mut candidates = match_selector_against_pool(
        &first.selector.node,
        roots,
        true, // search descendants too for the first segment
    )?;

    // Process subsequent segments with combinators
    for seg in &chain.segments[1..] {
        let mut next_candidates = Vec::new();
        for candidate in &candidates {
            let pool: Vec<QueryNode> = match seg.combinator {
                Combinator::None => unreachable!("first segment should have None combinator"),
                Combinator::Child => candidate.node.children(),
                Combinator::Descendant => candidate.node.descendants(),
            };
            let matches = match_selector_against_pool(
                &seg.selector.node,
                &pool,
                false, // don't search descendants — the pool is already expanded
            )?;
            for m in matches {
                let mut path = candidate.path.clone();
                path.push(m.node.display_name());
                next_candidates.push(QueryMatch {
                    node: m.node,
                    path,
                });
            }
        }
        candidates = next_candidates;
    }

    Ok(candidates)
}

/// Match a compound selector against a pool of nodes.
fn match_selector_against_pool(
    selector: &CompoundSelector,
    pool: &[QueryNode],
    search_descendants: bool,
) -> QueryResult<QueryResultSet> {
    let mut results = Vec::new();

    // Build the full candidate pool
    let mut candidates: Vec<QueryNode> = pool.to_vec();
    if search_descendants {
        for root in pool {
            candidates.extend(root.descendants());
        }
    }

    for node in &candidates {
        if matches_compound_selector(node, selector)? {
            results.push(QueryMatch {
                node: node.clone(),
                path: vec![node.display_name()],
            });
        }
    }

    Ok(results)
}

/// Check if a single node matches a compound selector.
fn matches_compound_selector(
    node: &QueryNode,
    selector: &CompoundSelector,
) -> QueryResult<bool> {
    // Check base selector
    if !matches_base_selector(node, &selector.base)? {
        return Ok(false);
    }

    // Check attribute filters
    for attr in &selector.attrs {
        if !matches_attribute_filter(node, &attr.node)? {
            return Ok(false);
        }
    }

    // Check pseudo-classes
    for pseudo in &selector.pseudos {
        if !matches_pseudo_class(node, pseudo.node) {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Check if a node matches a base selector.
fn matches_base_selector(
    node: &QueryNode,
    base: &Spanned<BaseSelector>,
) -> QueryResult<bool> {
    match &base.node {
        BaseSelector::Any => Ok(true),

        BaseSelector::Type(ts) => {
            Ok(node_matches_type(node, *ts))
        }

        BaseSelector::DesignatorPattern(pat) => {
            Ok(matches_designator_pattern(node, pat))
        }

        BaseSelector::PartNumber(name) => {
            // Match against lib_reference (exact match)
            match node.lib_reference() {
                Some(lr) => Ok(lr.eq_ignore_ascii_case(name)),
                None => Ok(false),
            }
        }

        BaseSelector::ValuePattern(value) => {
            // Check the "Value" parameter
            match node.value_parameter() {
                QueryValue::String(v) => Ok(v.eq_ignore_ascii_case(value)),
                _ => Ok(false),
            }
        }

        BaseSelector::NetName(name) => {
            match node.net_name() {
                Some(nn) => Ok(nn.eq_ignore_ascii_case(name)),
                None => Ok(false),
            }
        }

        BaseSelector::RecordId(_) => {
            // Record IDs violate the design philosophy
            Err(QueryError::new(
                QueryErrorCode::Unsupported,
                "record ID queries (#id) are not supported",
            )
            .with_span(base.span)
            .with_help("queries operate on high-level API types, not internal record IDs"))
        }

        BaseSelector::ComponentPin { component, pin } => {
            // Match a specific pin on a specific component
            match node {
                QueryNode::Pin(p) => {
                    // We can only match the pin name here — component context
                    // would require walking the parent tree. For now, match pin name.
                    Ok(p.designator.eq_ignore_ascii_case(pin)
                        || p.name.eq_ignore_ascii_case(pin))
                }
                QueryNode::Component(c) => {
                    // Check if component matches and has the pin
                    let comp_match = c.lib_reference.eq_ignore_ascii_case(component)
                        || c.designator.as_ref().is_some_and(|d| d.eq_ignore_ascii_case(component));
                    if !comp_match {
                        return Ok(false);
                    }
                    Ok(c.pins.iter().any(|p| {
                        p.designator.eq_ignore_ascii_case(pin) || p.name.eq_ignore_ascii_case(pin)
                    }))
                }
                _ => Ok(false),
            }
        }
    }
}

/// Check if a node matches a type selector.
fn node_matches_type(node: &QueryNode, ts: TypeSelector) -> bool {
    let node_ts = node.type_selector();

    // Exact match
    if node_ts == ts {
        return true;
    }

    // `graphic` matches any graphic sub-type
    if ts == TypeSelector::Graphic {
        return matches!(
            node,
            QueryNode::Graphic(_)
        );
    }

    // A specific graphic sub-type like `line` should match Graphic::Line
    if matches!(node, QueryNode::Graphic(_)) {
        if let Some(gts) = node.graphic_type_selector() {
            return gts == ts;
        }
    }

    // Similar for PcbGraphic
    if matches!(node, QueryNode::PcbGraphic(_)) {
        if let Some(gts) = node.graphic_type_selector() {
            return gts == ts;
        }
    }

    false
}

/// Check if a node matches a designator pattern.
fn matches_designator_pattern(node: &QueryNode, pat: &DesignatorPattern) -> bool {
    // For SchLib: match against lib_reference
    let target = node.lib_reference()
        .or_else(|| node.designator());

    let target = match target {
        Some(t) => t,
        None => return false,
    };

    match &pat.wildcard {
        Wildcard::None => target.eq_ignore_ascii_case(&pat.prefix),
        Wildcard::Star => target
            .to_ascii_lowercase()
            .starts_with(&pat.prefix.to_ascii_lowercase()),
        Wildcard::Fixed(count) => {
            let expected_len = pat.prefix.len() + count;
            if target.len() != expected_len {
                return false;
            }
            target
                .to_ascii_lowercase()
                .starts_with(&pat.prefix.to_ascii_lowercase())
        }
    }
}

/// Check if a node matches an attribute filter.
fn matches_attribute_filter(
    node: &QueryNode,
    attr: &AttributeFilter,
) -> QueryResult<bool> {
    // Resolve field value
    let value = if let Some(prefix) = &attr.field.prefix {
        match prefix.to_ascii_lowercase().as_str() {
            "param" => node.get_parameter(&attr.field.name),
            "field" => node.get_field(&attr.field.name),
            _ => {
                return Err(QueryError::new(
                    QueryErrorCode::UnknownField,
                    format!("unknown field prefix '{prefix}'"),
                )
                .with_span(attr.field.span)
                .with_help("valid prefixes: field, param"));
            }
        }
    } else {
        // No prefix — try as a regular field first, then parameter
        let v = node.get_field(&attr.field.name);
        if matches!(v, QueryValue::Null) {
            node.get_parameter(&attr.field.name)
        } else {
            v
        }
    };

    // Handle regex separately
    if let FilterValue::Regex(pattern) = &attr.value.node {
        return Ok(match &value {
            QueryValue::String(s) => regex_matches(s, pattern),
            _ => false,
        });
    }

    Ok(value.matches(attr.op, &attr.value.node))
}

/// Check if a node matches a pseudo-class.
fn matches_pseudo_class(node: &QueryNode, pseudo: PseudoClass) -> bool {
    match node {
        QueryNode::Pin(pin) => {
            use altium_format_types::sch::PinElectricalType;
            match pseudo {
                PseudoClass::Power => pin.electrical == PinElectricalType::Power,
                PseudoClass::Input => pin.electrical == PinElectricalType::Input,
                PseudoClass::Output => pin.electrical == PinElectricalType::Output,
                PseudoClass::Io => pin.electrical == PinElectricalType::InputOutput,
                PseudoClass::Passive => pin.electrical == PinElectricalType::Passive,
                PseudoClass::HiZ => pin.electrical == PinElectricalType::HiZ,
                PseudoClass::OpenCollector => pin.electrical == PinElectricalType::OpenCollector,
                PseudoClass::OpenEmitter => pin.electrical == PinElectricalType::OpenEmitter,
                PseudoClass::Virtual => false,
            }
        }
        QueryNode::Component(c) => match pseudo {
            PseudoClass::Virtual => {
                use altium_format_types::common::ComponentKind;
                matches!(
                    c.component_kind,
                    Some(ComponentKind::Graphical) | Some(ComponentKind::Mechanical)
                )
            }
            _ => false,
        },
        _ => false,
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Collect all nodes (roots + their descendants) as QueryMatches.
fn collect_all_nodes(roots: &[QueryNode]) -> Vec<QueryMatch> {
    let mut all = Vec::new();
    for root in roots {
        all.push(QueryMatch {
            node: root.clone(),
            path: vec![root.display_name()],
        });
        for desc in root.descendants() {
            all.push(QueryMatch {
                node: desc.clone(),
                path: vec![root.display_name()],
            });
        }
    }
    all
}

/// Merge new matches into result, avoiding duplicates.
fn merge_results(result: &mut Vec<QueryMatch>, new: Vec<QueryMatch>) {
    for m in new {
        if !result.iter().any(|r| nodes_equal(&r.node, &m.node)) {
            result.push(m);
        }
    }
}

/// Intersect: keep only results that appear in both sets.
fn intersect_results(a: Vec<QueryMatch>, b: &[QueryMatch]) -> Vec<QueryMatch> {
    a.into_iter()
        .filter(|m| b.iter().any(|bm| nodes_equal(&m.node, &bm.node)))
        .collect()
}

/// Structural equality for QueryNode (by display name + type).
/// In a real implementation this would use unique IDs.
fn nodes_equal(a: &QueryNode, b: &QueryNode) -> bool {
    if a.type_selector() != b.type_selector() {
        return false;
    }
    a.display_name() == b.display_name()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_query;
    use altium_format::api;
    use altium_format_types::color::Color;
    use altium_format_types::common::RotationBy90;
    use altium_format_types::coord::{Coord, CoordPoint};
    use altium_format_types::sch::*;
    use crate::adapter::Queryable;

    fn make_test_component() -> api::Component {
        api::Component {
            lib_reference: "LM358".to_string(),
            designator: Some("U?".to_string()),
            description: Some("Dual Op-Amp".to_string()),
            component_kind: None,
            part_count: 2,
            show_hidden_pins: false,
            pins: vec![
                make_pin("1", "OUT_A", PinElectricalType::Output),
                make_pin("2", "IN-", PinElectricalType::Input),
                make_pin("3", "IN+", PinElectricalType::Input),
                make_pin("4", "VCC", PinElectricalType::Power),
                make_pin("8", "GND", PinElectricalType::Power),
            ],
            parameters: vec![make_parameter("Value", "LM358")],
            footprints: vec![],
            graphics: vec![],
            aliases: vec![],
        }
    }

    fn make_pin(des: &str, name: &str, electrical: PinElectricalType) -> api::Pin {
        api::Pin {
            designator: des.to_string(),
            name: name.to_string(),
            electrical,
            location: CoordPoint { x: Coord::ZERO, y: Coord::ZERO },
            length: Coord::from_mils(30),
            orientation: RotationBy90::Rotate0,
            is_hidden: false,
            hidden_net_name: String::new(),
            owner_part_id: 1,
            show_name: true,
            show_designator: true,
            symbol_inner_edge: IeeeSymbol::default(),
            symbol_outer_edge: IeeeSymbol::default(),
            symbol_inside: IeeeSymbol::default(),
            symbol_outside: IeeeSymbol::default(),
            swap_id_pin: String::new(),
            swap_id_part: String::new(),
            swap_id_pair: String::new(),
            default_value: String::new(),
            pin_package_length: String::new(),
            propagation_delay: String::new(),
            pin_symbol_line_width: None,
            name_text_data: None,
            designator_text_data: None,
            description: String::new(),
            formal_type: StdLogicState::default(),
            spice_pin_name: String::new(),
            unique_id: String::new(),
            color: Color::BLACK,
            is_not_accessible: false,
            graphically_locked: false,
            owner_part_display_mode: 0,
        }
    }

    fn make_parameter(name: &str, text: &str) -> api::Parameter {
        api::Parameter {
            name: name.to_string(),
            text: text.to_string(),
            is_hidden: false,
            read_only: ParameterReadOnlyState::default(),
            location: CoordPoint { x: Coord::ZERO, y: Coord::ZERO },
            orientation: RotationBy90::Rotate0,
            color: Color::BLACK,
            font_id: 0,
            justification: TextJustification::default(),
            is_mirrored: false,
            show_name: false,
            unique_id: String::new(),
            not_auto_position: false,
            param_type: ParameterType::default(),
            description: String::new(),
        }
    }

    /// Mock Queryable for testing.
    struct MockDoc {
        components: Vec<api::Component>,
    }

    impl Queryable for MockDoc {
        fn root_nodes(&self) -> Result<Vec<QueryNode>, QueryError> {
            Ok(self.components.iter().map(|c| QueryNode::Component(c.clone())).collect())
        }
    }

    #[test]
    fn test_eval_type_selector() {
        let doc = MockDoc {
            components: vec![make_test_component()],
        };
        let query = parse_query("component").unwrap();
        let results = eval_query(&query, &doc).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_eval_pin_type() {
        let doc = MockDoc {
            components: vec![make_test_component()],
        };
        let query = parse_query("pin").unwrap();
        let results = eval_query(&query, &doc).unwrap();
        assert_eq!(results.len(), 5); // 5 pins in the test component
    }

    #[test]
    fn test_eval_pseudo_class() {
        let doc = MockDoc {
            components: vec![make_test_component()],
        };
        let query = parse_query("pin:power").unwrap();
        let results = eval_query(&query, &doc).unwrap();
        assert_eq!(results.len(), 2); // VCC and GND
    }

    #[test]
    fn test_eval_child_combinator() {
        let doc = MockDoc {
            components: vec![make_test_component()],
        };
        let query = parse_query("component > pin:power").unwrap();
        let results = eval_query(&query, &doc).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_eval_attribute_filter() {
        let doc = MockDoc {
            components: vec![make_test_component()],
        };
        let query = parse_query("component[part_count>1]").unwrap();
        let results = eval_query(&query, &doc).unwrap();
        assert_eq!(results.len(), 1); // part_count=2 > 1
    }

    #[test]
    fn test_eval_value_pattern() {
        let doc = MockDoc {
            components: vec![make_test_component()],
        };
        let query = parse_query("@LM358").unwrap();
        let results = eval_query(&query, &doc).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_eval_part_number() {
        let doc = MockDoc {
            components: vec![make_test_component()],
        };
        let query = parse_query("$LM358").unwrap();
        let results = eval_query(&query, &doc).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_eval_designator_pattern() {
        let doc = MockDoc {
            components: vec![make_test_component()],
        };
        // Exact match on lib_reference
        let query = parse_query("LM358").unwrap();
        let results = eval_query(&query, &doc).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_eval_or() {
        let doc = MockDoc {
            components: vec![make_test_component()],
        };
        let query = parse_query("pin:power OR pin:input").unwrap();
        let results = eval_query(&query, &doc).unwrap();
        assert_eq!(results.len(), 4); // 2 power + 2 input
    }

    #[test]
    fn test_eval_not() {
        let doc = MockDoc {
            components: vec![make_test_component()],
        };
        let query = parse_query("NOT component").unwrap();
        let results = eval_query(&query, &doc).unwrap();
        // All non-component nodes: pins(5) + params(1) = 6
        assert_eq!(results.len(), 6);
    }

    #[test]
    fn test_eval_net_name() {
        let doc = MockDoc {
            components: vec![make_test_component()],
        };
        // ~VCC matches no nodes in a SchLib mock (no NetLabel/PowerObject/Port/SheetEntry)
        let query = parse_query("~VCC").unwrap();
        let results = eval_query(&query, &doc).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_eval_record_id_unsupported() {
        let doc = MockDoc {
            components: vec![make_test_component()],
        };
        let query = parse_query("#42").unwrap();
        let result = eval_query(&query, &doc);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, QueryErrorCode::Unsupported);
    }

    #[test]
    fn test_eval_param_field_path() {
        let doc = MockDoc {
            components: vec![make_test_component()],
        };
        let query = parse_query("component[param.Value=LM358]").unwrap();
        let results = eval_query(&query, &doc).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_eval_pin_attribute() {
        let doc = MockDoc {
            components: vec![make_test_component()],
        };
        let query = parse_query("pin[electrical=Power]").unwrap();
        let results = eval_query(&query, &doc).unwrap();
        assert_eq!(results.len(), 2);
    }
}

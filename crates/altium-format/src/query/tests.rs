//! Tests for the query module.

use super::ast::{
    AttributeOp, AttributeSelector, CombinatorType, ElementType, PseudoSelector, Selector,
};
use super::schql_parser::QueryParser;
use super::{QueryMatch, QueryResult};

#[test]
fn test_element_type_parsing() {
    assert_eq!(
        ElementType::try_parse("component"),
        Some(ElementType::Component)
    );
    assert_eq!(ElementType::try_parse("comp"), Some(ElementType::Component));
    assert_eq!(ElementType::try_parse("pin"), Some(ElementType::Pin));
    assert_eq!(ElementType::try_parse("net"), Some(ElementType::Net));
    assert_eq!(ElementType::try_parse("power"), Some(ElementType::Power));
    assert_eq!(ElementType::try_parse("ground"), Some(ElementType::Ground));
    assert_eq!(ElementType::try_parse("gnd"), Some(ElementType::Ground));
    assert_eq!(ElementType::try_parse("invalid"), None);
}

#[test]
fn test_pseudo_selector_parsing() {
    assert_eq!(
        PseudoSelector::try_parse("connected", None),
        Some(PseudoSelector::Connected)
    );
    assert_eq!(
        PseudoSelector::try_parse("unconnected", None),
        Some(PseudoSelector::Unconnected)
    );
    assert_eq!(
        PseudoSelector::try_parse("floating", None),
        Some(PseudoSelector::Unconnected)
    );
    assert_eq!(
        PseudoSelector::try_parse("power", None),
        Some(PseudoSelector::Power)
    );
    assert_eq!(
        PseudoSelector::try_parse("ground", None),
        Some(PseudoSelector::Ground)
    );
    assert_eq!(
        PseudoSelector::try_parse("input", None),
        Some(PseudoSelector::Input)
    );
    assert_eq!(
        PseudoSelector::try_parse("output", None),
        Some(PseudoSelector::Output)
    );
    assert_eq!(
        PseudoSelector::try_parse("count", None),
        Some(PseudoSelector::Count)
    );
    assert_eq!(
        PseudoSelector::try_parse("limit", Some("10")),
        Some(PseudoSelector::Limit(10))
    );
    assert_eq!(
        PseudoSelector::try_parse("nth", Some("5")),
        Some(PseudoSelector::Nth(5))
    );
}

#[test]
fn test_attribute_selector_matching() {
    let attr = AttributeSelector::new("part", AttributeOp::Equals, Some("LM7805".to_string()));
    assert!(attr.matches(Some("LM7805")));
    assert!(!attr.matches(Some("LM7806")));
    assert!(!attr.matches(None));

    let attr_contains =
        AttributeSelector::new("part", AttributeOp::Contains, Some("7805".to_string()));
    assert!(attr_contains.matches(Some("LM7805")));
    assert!(attr_contains.matches(Some("MC7805")));
    assert!(!attr_contains.matches(Some("LM7806")));

    let attr_starts =
        AttributeSelector::new("part", AttributeOp::StartsWith, Some("LM".to_string()));
    assert!(attr_starts.matches(Some("LM7805")));
    assert!(!attr_starts.matches(Some("MC7805")));

    let attr_ends = AttributeSelector::new("part", AttributeOp::EndsWith, Some("05".to_string()));
    assert!(attr_ends.matches(Some("LM7805")));
    assert!(!attr_ends.matches(Some("LM7806")));

    let attr_exists = AttributeSelector::new("value", AttributeOp::Exists, None);
    assert!(attr_exists.matches(Some("")));
    assert!(attr_exists.matches(Some("10k")));
    assert!(!attr_exists.matches(None));
}

#[test]
fn test_attribute_selector_numeric() {
    let attr_gt = AttributeSelector::new("pins", AttributeOp::GreaterThan, Some("10".to_string()));
    assert!(attr_gt.matches(Some("15")));
    assert!(!attr_gt.matches(Some("5")));
    assert!(!attr_gt.matches(Some("10")));

    let attr_lt = AttributeSelector::new("pins", AttributeOp::LessThan, Some("10".to_string()));
    assert!(attr_lt.matches(Some("5")));
    assert!(!attr_lt.matches(Some("15")));
    assert!(!attr_lt.matches(Some("10")));

    let attr_gte =
        AttributeSelector::new("pins", AttributeOp::GreaterOrEqual, Some("10".to_string()));
    assert!(attr_gte.matches(Some("15")));
    assert!(attr_gte.matches(Some("10")));
    assert!(!attr_gte.matches(Some("5")));
}

#[test]
fn test_query_parser_simple() {
    let parser = QueryParser::new();

    // Element selector
    let sel = parser.parse("component").unwrap();
    assert_eq!(sel, Selector::Element(ElementType::Component));

    // ID selector
    let sel = parser.parse("#U1").unwrap();
    assert_eq!(sel, Selector::Id("U1".to_string()));

    // Universal selector
    let sel = parser.parse("*").unwrap();
    assert_eq!(sel, Selector::Universal);
}

#[test]
fn test_query_parser_attributes() {
    let parser = QueryParser::new();

    // Equals
    let sel = parser.parse("[part=LM7805]").unwrap();
    match sel {
        Selector::Attribute(attr) => {
            assert_eq!(attr.name, "part");
            assert_eq!(attr.op, AttributeOp::Equals);
            assert_eq!(attr.value, Some("LM7805".to_string()));
        }
        _ => panic!("Expected attribute selector"),
    }

    // Contains
    let sel = parser.parse("[part*=7805]").unwrap();
    match sel {
        Selector::Attribute(attr) => {
            assert_eq!(attr.op, AttributeOp::Contains);
        }
        _ => panic!("Expected attribute selector"),
    }

    // Starts with
    let sel = parser.parse("[part^=LM]").unwrap();
    match sel {
        Selector::Attribute(attr) => {
            assert_eq!(attr.op, AttributeOp::StartsWith);
        }
        _ => panic!("Expected attribute selector"),
    }

    // Ends with
    let sel = parser.parse("[part$=05]").unwrap();
    match sel {
        Selector::Attribute(attr) => {
            assert_eq!(attr.op, AttributeOp::EndsWith);
        }
        _ => panic!("Expected attribute selector"),
    }

    // Not equals
    let sel = parser.parse("[type!=input]").unwrap();
    match sel {
        Selector::Attribute(attr) => {
            assert_eq!(attr.op, AttributeOp::NotEquals);
        }
        _ => panic!("Expected attribute selector"),
    }
}

#[test]
fn test_query_parser_pseudo() {
    let parser = QueryParser::new();

    let sel = parser.parse(":connected").unwrap();
    assert_eq!(sel, Selector::Pseudo(PseudoSelector::Connected));

    let sel = parser.parse(":power").unwrap();
    assert_eq!(sel, Selector::Pseudo(PseudoSelector::Power));

    let sel = parser.parse(":count").unwrap();
    assert_eq!(sel, Selector::Pseudo(PseudoSelector::Count));

    let sel = parser.parse(":limit(5)").unwrap();
    assert_eq!(sel, Selector::Pseudo(PseudoSelector::Limit(5)));
}

#[test]
fn test_query_parser_compound() {
    let parser = QueryParser::new();

    let sel = parser.parse("pin[type=input]").unwrap();
    match sel {
        Selector::Compound(parts) => {
            assert_eq!(parts.len(), 2);
            assert_eq!(parts[0], Selector::Element(ElementType::Pin));
        }
        _ => panic!("Expected compound selector"),
    }

    let sel = parser.parse("component[part*=7805]:first").unwrap();
    match sel {
        Selector::Compound(parts) => {
            assert_eq!(parts.len(), 3);
        }
        _ => panic!("Expected compound selector"),
    }
}

#[test]
fn test_query_parser_combinators() {
    let parser = QueryParser::new();

    // Child combinator
    let sel = parser.parse("#U1 > pin").unwrap();
    match sel {
        Selector::Combinator {
            left,
            combinator,
            right,
        } => {
            assert_eq!(*left, Selector::Id("U1".to_string()));
            assert_eq!(combinator, CombinatorType::Child);
            assert_eq!(*right, Selector::Element(ElementType::Pin));
        }
        _ => panic!("Expected combinator"),
    }

    // Connected combinator
    let sel = parser.parse("#U1 >> component").unwrap();
    match sel {
        Selector::Combinator { combinator, .. } => {
            assert_eq!(combinator, CombinatorType::Connected);
        }
        _ => panic!("Expected combinator"),
    }

    // On-net combinator
    let sel = parser.parse("#VCC :: pin").unwrap();
    match sel {
        Selector::Combinator { combinator, .. } => {
            assert_eq!(combinator, CombinatorType::OnNet);
        }
        _ => panic!("Expected combinator"),
    }

    // Descendant combinator (implicit)
    let sel = parser.parse("#U1 pin").unwrap();
    match sel {
        Selector::Combinator { combinator, .. } => {
            assert_eq!(combinator, CombinatorType::Descendant);
        }
        _ => panic!("Expected combinator"),
    }
}

#[test]
fn test_query_parser_union() {
    let parser = QueryParser::new();

    let sel = parser.parse("component, port, power").unwrap();
    match sel {
        Selector::Union(parts) => {
            assert_eq!(parts.len(), 3);
        }
        _ => panic!("Expected union"),
    }
}

#[test]
fn test_query_parser_not() {
    let parser = QueryParser::new();

    let sel = parser.parse(":not(pin)").unwrap();
    match sel {
        Selector::Not(inner) => {
            assert_eq!(*inner, Selector::Element(ElementType::Pin));
        }
        _ => panic!("Expected not selector"),
    }

    let sel = parser.parse(":not([type=power])").unwrap();
    match sel {
        Selector::Not(inner) => match *inner {
            Selector::Attribute(_) => {}
            _ => panic!("Expected attribute in not"),
        },
        _ => panic!("Expected not selector"),
    }
}

#[test]
fn test_query_parser_has() {
    let parser = QueryParser::new();

    let sel = parser.parse(":has(pin[type=power])").unwrap();
    match sel {
        Selector::Has(inner) => match *inner {
            Selector::Compound(_) => {}
            _ => panic!("Expected compound in has"),
        },
        _ => panic!("Expected has selector"),
    }
}

#[test]
fn test_query_parser_complex() {
    let parser = QueryParser::new();

    // Complex query: components with 7805 in part name, get their input pins
    let sel = parser
        .parse("component[part*=7805] > pin[type=input]")
        .unwrap();
    match sel {
        Selector::Combinator {
            left,
            combinator,
            right,
        } => {
            assert_eq!(combinator, CombinatorType::Child);
            match *left {
                Selector::Compound(ref parts) => {
                    assert_eq!(parts.len(), 2);
                }
                _ => panic!("Expected compound on left"),
            }
            match *right {
                Selector::Compound(ref parts) => {
                    assert_eq!(parts.len(), 2);
                }
                _ => panic!("Expected compound on right"),
            }
        }
        _ => panic!("Expected combinator"),
    }
}

#[test]
fn test_query_parser_string_values() {
    let parser = QueryParser::new();

    // Double-quoted string
    let sel = parser.parse("[name=\"CLK IN\"]").unwrap();
    match sel {
        Selector::Attribute(attr) => {
            assert_eq!(attr.value, Some("CLK IN".to_string()));
        }
        _ => panic!("Expected attribute"),
    }

    // Single-quoted string
    let sel = parser.parse("[name='DATA OUT']").unwrap();
    match sel {
        Selector::Attribute(attr) => {
            assert_eq!(attr.value, Some("DATA OUT".to_string()));
        }
        _ => panic!("Expected attribute"),
    }
}

#[test]
fn test_query_parser_errors() {
    let parser = QueryParser::new();

    // Empty query
    assert!(parser.parse("").is_err());

    // Unknown element
    assert!(parser.parse("unknown").is_err());

    // Unterminated string
    assert!(parser.parse("[name=\"test]").is_err());

    // Unknown pseudo
    assert!(parser.parse(":unknown").is_err());
}

#[test]
fn test_query_match_short_text() {
    let comp = QueryMatch::Component {
        designator: "U1".to_string(),
        part: "LM7805".to_string(),
        description: "5V Regulator".to_string(),
        value: Some("5V".to_string()),
        footprint: Some("TO-220".to_string()),
        pin_count: 3,
    };
    assert_eq!(comp.to_short_text(), "U1 (LM7805, 5V)");

    let pin = QueryMatch::Pin {
        component_designator: "U1".to_string(),
        designator: "1".to_string(),
        name: "VIN".to_string(),
        electrical_type: "Input".to_string(),
        connected_net: Some("VCC".to_string()),
        is_hidden: false,
    };
    assert_eq!(pin.to_short_text(), "U1.1 \"VIN\" [Input] -> VCC");

    let net = QueryMatch::Net {
        name: "VCC".to_string(),
        is_power: true,
        is_ground: false,
        connection_count: 5,
        connections: vec!["U1.1".to_string()],
    };
    assert_eq!(net.to_short_text(), "VCC [PWR] (5 connections)");

    let count = QueryMatch::Count(42);
    assert_eq!(count.to_short_text(), "42");
}

#[test]
fn test_query_result_to_text() {
    let result = QueryResult {
        matches: vec![QueryMatch::Count(10)],
        query: "component:count".to_string(),
        execution_time_us: 100,
    };
    assert_eq!(result.to_text(), "10\n");

    let result = QueryResult {
        matches: vec![],
        query: "pin:unconnected".to_string(),
        execution_time_us: 50,
    };
    assert!(result.to_text().contains("No matches"));
}

#[test]
fn test_combinator_syntax() {
    assert_eq!(CombinatorType::Child.syntax(), " > ");
    assert_eq!(CombinatorType::Descendant.syntax(), " ");
    assert_eq!(CombinatorType::Connected.syntax(), " >> ");
    assert_eq!(CombinatorType::OnNet.syntax(), " :: ");
}

#[test]
fn test_result_modifiers() {
    let count = PseudoSelector::Count;
    assert!(count.is_result_modifier());
    assert!(!count.is_filter());

    let connected = PseudoSelector::Connected;
    assert!(!connected.is_result_modifier());
    assert!(connected.is_filter());

    let limit = PseudoSelector::Limit(10);
    assert!(limit.is_result_modifier());
}

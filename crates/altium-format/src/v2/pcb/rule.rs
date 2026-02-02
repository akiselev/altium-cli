//! PCB Rule record (parametric only).
//!
//! Rules6/Data uses parametric format.
//! RULEKIND string (from cRuleIdStrings), NAME, PRIORITY, SCOPE1/2EXPRESSION,
//! plus kind-specific fields. 52 rule types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::enums::TRuleKind;

/// PCB Design Rule record (parametric).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbRule {
    pub properties: HashMap<String, String>,
}

impl PcbRule {
    pub fn from_properties(props: HashMap<String, String>) -> Self {
        Self { properties: props }
    }

    pub fn rule_kind_str(&self) -> Option<&str> {
        self.properties.get("RULEKIND").map(|s| s.as_str())
    }

    pub fn rule_kind(&self) -> Option<TRuleKind> {
        self.rule_kind_str().and_then(TRuleKind::from_string_id)
    }

    pub fn name(&self) -> Option<&str> {
        self.properties.get("NAME").map(|s| s.as_str())
    }

    pub fn priority(&self) -> Option<i32> {
        self.properties.get("PRIORITY").and_then(|s| s.parse().ok())
    }

    pub fn scope1_expression(&self) -> Option<&str> {
        self.properties.get("SCOPE1EXPRESSION").map(|s| s.as_str())
    }

    pub fn scope2_expression(&self) -> Option<&str> {
        self.properties.get("SCOPE2EXPRESSION").map(|s| s.as_str())
    }
}

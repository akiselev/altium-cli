//! PCB Net record (ID=8, parametric only).
//!
//! Nets6/Data uses parametric format. Key field: NAME.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// PCB Net record (parametric).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbNet {
    pub properties: HashMap<String, String>,
}

impl PcbNet {
    pub fn from_properties(props: HashMap<String, String>) -> Self {
        Self { properties: props }
    }

    pub fn name(&self) -> Option<&str> {
        self.properties.get("NAME").map(|s| s.as_str())
    }
}

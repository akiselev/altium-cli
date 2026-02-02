//! PCB Class record (ID=15, parametric only).
//!
//! Classes6/Data uses parametric format.
//! Key fields: NAME, KIND, member names (M0, M1, ...).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// PCB Class record (parametric).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbClass {
    pub properties: HashMap<String, String>,
}

impl PcbClass {
    pub fn from_properties(props: HashMap<String, String>) -> Self {
        Self { properties: props }
    }

    pub fn name(&self) -> Option<&str> {
        self.properties.get("NAME").map(|s| s.as_str())
    }

    pub fn kind(&self) -> Option<&str> {
        self.properties.get("KIND").map(|s| s.as_str())
    }

    /// Extract member names (M0, M1, M2, ...).
    pub fn members(&self) -> Vec<&str> {
        let mut members = Vec::new();
        let mut i = 0;
        loop {
            let key = format!("M{}", i);
            match self.properties.get(&key) {
                Some(v) => members.push(v.as_str()),
                None => break,
            }
            i += 1;
        }
        members
    }
}

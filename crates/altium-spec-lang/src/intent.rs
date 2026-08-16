use serde::{Deserialize, Serialize};

use crate::source::{LosslessSpec, SourceNodeId, SpecDomain};

/// Explicit authored meaning for a managed field.
///
/// This removes the old ambiguity where `Option::None` was forced to mean
/// inherit, leave-unchanged, clear, and reset depending on the executor.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "intent", content = "value", rename_all = "snake_case")]
pub enum FieldIntent<T> {
    #[default]
    Inherit,
    Set(T),
    Clear,
    Reset,
}

/// Structural authored resource retained before semantic elaboration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoredResource {
    pub source_id: SourceNodeId,
    pub kind: String,
    pub key: String,
    pub source: String,
}

/// Authored intent before imports/defaults/templates are elaborated into an
/// Altium-shaped concrete snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoredIntent {
    pub domain: SpecDomain,
    pub resources: Vec<AuthoredResource>,
}

impl AuthoredIntent {
    pub fn from_lossless(domain: SpecDomain, spec: &LosslessSpec) -> Self {
        Self {
            domain,
            resources: spec
                .resources()
                .iter()
                .map(|resource| AuthoredResource {
                    source_id: resource.id.clone(),
                    kind: resource.kind.clone(),
                    key: resource.key.clone(),
                    source: resource.source.clone(),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omission_meanings_are_distinct() {
        let values = [
            FieldIntent::<u8>::Inherit,
            FieldIntent::Set(1),
            FieldIntent::Clear,
            FieldIntent::Reset,
        ];
        for (index, left) in values.iter().enumerate() {
            for (other, right) in values.iter().enumerate() {
                assert_eq!(left == right, index == other);
            }
        }
    }
}

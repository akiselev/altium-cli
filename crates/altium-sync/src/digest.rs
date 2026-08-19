use std::fmt;

use serde::{Deserialize, Serialize};

/// Stable BLAKE3 digest used by baselines, plan preconditions, and semantic
/// fingerprints. The textual representation is deliberately explicit so saved
/// plans remain debuggable and language-agnostic.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Digest(pub String);

impl Digest {
    pub fn bytes(bytes: &[u8]) -> Self {
        Self(blake3::hash(bytes).to_hex().to_string())
    }

    pub fn text(text: &str) -> Self {
        Self::bytes(text.as_bytes())
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

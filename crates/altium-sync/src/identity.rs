use std::fmt;

use serde::{Deserialize, Serialize};

use crate::digest::Digest;

/// Durable identity shared by source and document resources.
///
/// It is minted, persisted in the external baseline, and never derived from a
/// mutable natural key or geometry fingerprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BindingId(pub u128);

impl BindingId {
    pub fn mint() -> Self {
        Self(rand::random())
    }
}

impl fmt::Display for BindingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

/// How the current document instance is located. Coarse aggregate resources use
/// natural keys today; the enum already models the native/keyless tiers needed
/// by finer PcbDoc/PcbLib decomposition without changing the baseline schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DocumentLocator {
    Native {
        unique_id: String,
    },
    NaturalKey {
        parent: Option<BindingId>,
        key: String,
    },
    Structural {
        parent: Option<BindingId>,
        collection: String,
        ordinal: u32,
        fingerprint: Digest,
    },
}

/// A source/document alignment entry retained by the external ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingRecord {
    pub binding: BindingId,
    pub resource_kind: String,
    pub source_key: Option<String>,
    pub source_fingerprint: Option<Digest>,
    pub document_key: Option<String>,
    pub document_fingerprint: Option<Digest>,
    pub document_locator: Option<DocumentLocator>,
}

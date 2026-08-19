use std::collections::HashMap;

use altium_spec_lang::{LosslessSpec, canonicalize_semantic_text};
use serde::{Deserialize, Serialize};

use crate::digest::Digest;

/// Supported synchronization artifact families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    SchLib,
    PcbLib,
    SchDoc,
    PcbDoc,
}

impl ArtifactKind {
    pub fn spec_extension(self) -> &'static str {
        match self {
            Self::SchLib => "schlib-spec",
            Self::PcbLib => "pcblib-spec",
            Self::SchDoc => "schdoc-spec",
            Self::PcbDoc => "pcbdoc-spec",
        }
    }

    pub fn document_extension(self) -> &'static str {
        match self {
            Self::SchLib => "SchLib",
            Self::PcbLib => "PcbLib",
            Self::SchDoc => "SchDoc",
            Self::PcbDoc => "PcbDoc",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotResource {
    /// Unique address inside this snapshot. Natural keys can be duplicated, so
    /// an occurrence suffix is included instead of silently coalescing them.
    pub address: String,
    pub kind: String,
    pub key: String,
    pub fingerprint: Digest,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactSnapshot {
    pub kind: ArtifactKind,
    /// Exact-byte digest used when this snapshot is a plan precondition.
    pub raw_digest: Digest,
    /// Management-metadata-insensitive semantic digest.
    pub semantic_digest: Digest,
    pub resources: Vec<SnapshotResource>,
}

impl ArtifactSnapshot {
    pub fn empty(kind: ArtifactKind) -> Self {
        Self::from_source(kind, "").expect("empty source is structurally valid")
    }

    pub fn from_source(
        kind: ArtifactKind,
        source: &str,
    ) -> Result<Self, altium_spec_lang::SourceError> {
        let lossless = LosslessSpec::parse(source.to_string())?;
        let canonical = canonicalize_semantic_text(source);
        let mut resources = Vec::new();
        let mut occurrences: HashMap<(String, String), usize> = HashMap::new();

        for resource in lossless.resources() {
            let pair = (resource.kind.clone(), resource.key.clone());
            let occurrence = occurrences.entry(pair.clone()).or_default();
            let canonical_resource = canonicalize_semantic_text(&resource.source);
            resources.push(SnapshotResource {
                address: format!("{}:{}#{}", pair.0, pair.1, *occurrence),
                kind: resource.kind.clone(),
                key: resource.key.clone(),
                fingerprint: Digest::text(&canonical_resource),
                text: resource.source.clone(),
            });
            *occurrence += 1;
        }

        // Whole-file coverage is intentional, even when fine-grained resources
        // were discovered. Imports, bindings, scalar lets, comments with semantic
        // annotations, or future syntax must never evade three-way drift checks.
        // This conservative sentinel also makes simultaneous edits in different
        // resource kinds conflict instead of being silently merged.
        if !canonical.trim().is_empty() {
            resources.push(SnapshotResource {
                address: "$file#0".to_string(),
                kind: "$file".to_string(),
                key: "$file".to_string(),
                fingerprint: Digest::text(&canonical),
                text: source.to_string(),
            });
        }

        Ok(Self {
            kind,
            raw_digest: Digest::text(source),
            semantic_digest: Digest::text(&canonical),
            resources,
        })
    }

    pub fn resource(&self, address: &str) -> Option<&SnapshotResource> {
        self.resources
            .iter()
            .find(|resource| resource.address == address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotation_churn_does_not_change_semantic_digest() {
        let a = "#[annotation(id = \"one\")]\ncomponent R {\n}\n";
        let b = "#[annotation(id = \"two\")]\ncomponent R {\n}\n";
        let a = ArtifactSnapshot::from_source(ArtifactKind::SchLib, a).unwrap();
        let b = ArtifactSnapshot::from_source(ArtifactKind::SchLib, b).unwrap();
        assert_ne!(a.raw_digest, b.raw_digest);
        assert_eq!(a.semantic_digest, b.semantic_digest);
        assert_eq!(a.resources[0].fingerprint, b.resources[0].fingerprint);
    }

    #[test]
    fn whole_file_resource_covers_non_block_source_changes() {
        let a = ArtifactSnapshot::from_source(
            ArtifactKind::SchLib,
            "import \"a.schlib-spec\"\ncomponent R {\n}\n",
        )
        .unwrap();
        let b = ArtifactSnapshot::from_source(
            ArtifactKind::SchLib,
            "import \"b.schlib-spec\"\ncomponent R {\n}\n",
        )
        .unwrap();
        let file_a = a.resource("$file#0").unwrap();
        let file_b = b.resource("$file#0").unwrap();
        assert_ne!(file_a.fingerprint, file_b.fingerprint);
    }

    #[test]
    fn duplicate_natural_keys_get_distinct_addresses() {
        let source = "component R {\n}\ncomponent R {\n}\n";
        let snapshot = ArtifactSnapshot::from_source(ArtifactKind::SchLib, source).unwrap();
        assert_eq!(snapshot.resources[0].address, "component:R#0");
        assert_eq!(snapshot.resources[1].address, "component:R#1");
    }
}

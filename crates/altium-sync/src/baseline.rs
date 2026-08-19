use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::digest::Digest;
use crate::identity::{BindingId, BindingRecord, DocumentLocator};
use crate::snapshot::{ArtifactKind, ArtifactSnapshot, SnapshotResource};

pub const BASELINE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncBaseline {
    pub schema_version: u32,
    pub revision: u64,
    pub artifact_kind: ArtifactKind,
    pub source_digest: Digest,
    pub document_digest: Digest,
    pub resources: Vec<BindingRecord>,
}

#[derive(Debug, Error)]
pub enum BaselineError {
    #[error("reading baseline {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("parsing baseline {path}: {source}")]
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("unsupported baseline schema version {0}")]
    UnsupportedVersion(u32),
    #[error("baseline artifact kind {actual:?} does not match requested kind {expected:?}")]
    KindMismatch {
        expected: ArtifactKind,
        actual: ArtifactKind,
    },
    #[error("serializing baseline: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("writing baseline {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl SyncBaseline {
    pub fn from_snapshots(
        previous: Option<&SyncBaseline>,
        source: &ArtifactSnapshot,
        document: &ArtifactSnapshot,
    ) -> Self {
        assert_eq!(
            source.kind, document.kind,
            "baseline snapshots must share a domain"
        );
        let mut source_used = HashSet::new();
        let mut document_used = HashSet::new();
        let mut records = Vec::new();

        if let Some(previous) = previous {
            for old in &previous.resources {
                let source_match = match_previous_side(
                    &source.resources,
                    old.source_key.as_deref(),
                    old.source_fingerprint.as_ref(),
                    &source_used,
                );
                let document_match = match_previous_side(
                    &document.resources,
                    old.document_key.as_deref(),
                    old.document_fingerprint.as_ref(),
                    &document_used,
                );

                if let Some(resource) = source_match {
                    source_used.insert(resource.address.clone());
                }
                if let Some(resource) = document_match {
                    document_used.insert(resource.address.clone());
                }

                if source_match.is_some() || document_match.is_some() {
                    records.push(record_from_resources(
                        old.binding,
                        source_match,
                        document_match,
                    ));
                }
            }
        }

        for source_resource in &source.resources {
            if source_used.contains(&source_resource.address) {
                continue;
            }
            let document_match = document
                .resources
                .iter()
                .find(|resource| {
                    !document_used.contains(&resource.address)
                        && resource.address == source_resource.address
                })
                .or_else(|| {
                    unique_fingerprint_match(
                        &document.resources,
                        &source_resource.fingerprint,
                        &document_used,
                    )
                });

            source_used.insert(source_resource.address.clone());
            if let Some(resource) = document_match {
                document_used.insert(resource.address.clone());
            }
            records.push(record_from_resources(
                BindingId::mint(),
                Some(source_resource),
                document_match,
            ));
        }

        for document_resource in &document.resources {
            if document_used.contains(&document_resource.address) {
                continue;
            }
            document_used.insert(document_resource.address.clone());
            records.push(record_from_resources(
                BindingId::mint(),
                None,
                Some(document_resource),
            ));
        }

        Self {
            schema_version: BASELINE_SCHEMA_VERSION,
            revision: previous.map_or(1, |baseline| baseline.revision.saturating_add(1)),
            artifact_kind: source.kind,
            source_digest: source.semantic_digest.clone(),
            document_digest: document.semantic_digest.clone(),
            resources: records,
        }
    }

    pub fn digest(&self) -> Result<Digest, serde_json::Error> {
        Ok(Digest::bytes(&serde_json::to_vec(self)?))
    }
}

fn record_from_resources(
    binding: BindingId,
    source: Option<&SnapshotResource>,
    document: Option<&SnapshotResource>,
) -> BindingRecord {
    let resource_kind = source
        .map(|resource| resource.kind.clone())
        .or_else(|| document.map(|resource| resource.kind.clone()))
        .unwrap_or_else(|| "$unknown".to_string());
    BindingRecord {
        binding,
        resource_kind,
        source_key: source.map(|resource| resource.address.clone()),
        source_fingerprint: source.map(|resource| resource.fingerprint.clone()),
        document_key: document.map(|resource| resource.address.clone()),
        document_fingerprint: document.map(|resource| resource.fingerprint.clone()),
        document_locator: document.map(|resource| DocumentLocator::NaturalKey {
            parent: None,
            key: resource.key.clone(),
        }),
    }
}

fn match_previous_side<'a>(
    resources: &'a [SnapshotResource],
    old_address: Option<&str>,
    old_fingerprint: Option<&Digest>,
    used: &HashSet<String>,
) -> Option<&'a SnapshotResource> {
    if let Some(address) = old_address {
        if let Some(resource) = resources
            .iter()
            .find(|resource| resource.address == address && !used.contains(&resource.address))
        {
            return Some(resource);
        }
    }
    old_fingerprint.and_then(|fingerprint| unique_fingerprint_match(resources, fingerprint, used))
}

fn unique_fingerprint_match<'a>(
    resources: &'a [SnapshotResource],
    fingerprint: &Digest,
    used: &HashSet<String>,
) -> Option<&'a SnapshotResource> {
    let mut matches = resources.iter().filter(|resource| {
        resource.fingerprint == *fingerprint && !used.contains(&resource.address)
    });
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

pub fn default_baseline_path(document_path: &Path) -> PathBuf {
    let parent = document_path.parent().unwrap_or_else(|| Path::new("."));
    let name = document_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    parent.join(".altium").join(format!("{name}.sync.json"))
}

pub fn load_baseline(
    path: &Path,
    expected_kind: ArtifactKind,
) -> Result<Option<SyncBaseline>, BaselineError> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path).map_err(|source| BaselineError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let baseline: SyncBaseline =
        serde_json::from_slice(&bytes).map_err(|source| BaselineError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
    if baseline.schema_version != BASELINE_SCHEMA_VERSION {
        return Err(BaselineError::UnsupportedVersion(baseline.schema_version));
    }
    if baseline.artifact_kind != expected_kind {
        return Err(BaselineError::KindMismatch {
            expected: expected_kind,
            actual: baseline.artifact_kind,
        });
    }
    Ok(Some(baseline))
}

pub fn save_baseline(path: &Path, baseline: &SyncBaseline) -> Result<(), BaselineError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| BaselineError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let bytes = serde_json::to_vec_pretty(baseline)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("baseline");
    let staged = parent.join(format!(".{name}.{}.tmp", rand::random::<u64>()));
    let mut file = fs::File::create(&staged).map_err(|source| BaselineError::Write {
        path: staged.clone(),
        source,
    })?;
    file.write_all(&bytes).map_err(|source| BaselineError::Write {
        path: staged.clone(),
        source,
    })?;
    file.sync_all().map_err(|source| BaselineError::Write {
        path: staged.clone(),
        source,
    })?;
    drop(file);

    #[cfg(unix)]
    fs::rename(&staged, path).map_err(|source| BaselineError::Write {
        path: path.to_path_buf(),
        source,
    })?;

    #[cfg(not(unix))]
    {
        let backup = parent.join(format!(".{name}.{}.bak", rand::random::<u64>()));
        let existed = path.exists();
        if existed {
            fs::rename(path, &backup).map_err(|source| BaselineError::Write {
                path: backup.clone(),
                source,
            })?;
        }
        if let Err(source) = fs::rename(&staged, path) {
            if existed {
                let _ = fs::rename(&backup, path);
            }
            return Err(BaselineError::Write {
                path: path.to_path_buf(),
                source,
            });
        }
        if existed {
            let _ = fs::remove_file(backup);
        }
    }

    #[cfg(unix)]
    fs::File::open(parent)
        .and_then(|dir| dir.sync_all())
        .map_err(|source| BaselineError::Write {
            path: parent.to_path_buf(),
            source,
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_binding_across_natural_key_rename_when_fingerprint_is_unique() {
        let before_source = ArtifactSnapshot::from_source(
            ArtifactKind::SchLib,
            "component Old {\n  description: \"same\"\n}\n",
        )
        .unwrap();
        let before_document = before_source.clone();
        let baseline = SyncBaseline::from_snapshots(None, &before_source, &before_document);
        let binding = baseline.resources[0].binding;

        let after_source = ArtifactSnapshot::from_source(
            ArtifactKind::SchLib,
            "component New {\n  description: \"same\"\n}\n",
        )
        .unwrap();
        let rebased =
            SyncBaseline::from_snapshots(Some(&baseline), &after_source, &before_document);
        assert!(
            rebased
                .resources
                .iter()
                .any(|record| record.binding == binding && record.document_key.is_some())
        );
    }
}

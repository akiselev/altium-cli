use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::baseline::SyncBaseline;
use crate::digest::Digest;
use crate::plan::{ArtifactPatch, PLAN_SCHEMA_VERSION, PlanBundle, PlanStatus};

#[derive(Debug, Error)]
pub enum ApplyError {
    #[error("unsupported plan schema version {0}")]
    UnsupportedPlanVersion(u32),
    #[error("plan is blocked by unresolved three-way conflicts")]
    BlockedPlan,
    #[error("authored source changed since plan creation")]
    StaleSource,
    #[error("Altium document changed since plan creation")]
    StaleDocument,
    #[error("synchronization baseline changed since plan creation")]
    StaleBaseline,
    #[error("plan patch target does not match the requested apply operation")]
    WrongPatchTarget,
    #[error("reading {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("writing {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("serializing plan: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("parsing plan {path}: {source}")]
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalState {
    Staged,
    Committed,
}

/// Small crash-recovery journal used by document transaction adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionJournal {
    pub plan_id: String,
    pub destination: PathBuf,
    pub staged: PathBuf,
    pub backup: Option<PathBuf>,
    pub state: JournalState,
}

pub fn load_plan(path: &Path) -> Result<PlanBundle, ApplyError> {
    let bytes = fs::read(path).map_err(|source| ApplyError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let plan: PlanBundle = serde_json::from_slice(&bytes).map_err(|source| ApplyError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    if plan.schema_version != PLAN_SCHEMA_VERSION {
        return Err(ApplyError::UnsupportedPlanVersion(plan.schema_version));
    }
    Ok(plan)
}

pub fn save_plan(path: &Path, plan: &PlanBundle) -> Result<(), ApplyError> {
    let bytes = serde_json::to_vec_pretty(plan)?;
    atomic_write(path, &bytes)
}

pub fn verify_ready(plan: &PlanBundle, force: bool) -> Result<(), ApplyError> {
    if plan.status == PlanStatus::Blocked && !force {
        return Err(ApplyError::BlockedPlan);
    }
    Ok(())
}

pub fn verify_source_precondition(
    plan: &PlanBundle,
    actual_source: Option<&str>,
) -> Result<(), ApplyError> {
    let actual = actual_source.map(Digest::text);
    if actual != plan.precondition.source_raw_digest {
        return Err(ApplyError::StaleSource);
    }
    Ok(())
}

pub fn verify_document_precondition(
    plan: &PlanBundle,
    actual_semantic_digest: Option<&Digest>,
) -> Result<(), ApplyError> {
    if actual_semantic_digest != plan.precondition.document_semantic_digest.as_ref() {
        return Err(ApplyError::StaleDocument);
    }
    Ok(())
}

pub fn verify_baseline_precondition(
    plan: &PlanBundle,
    baseline: Option<&SyncBaseline>,
) -> Result<(), ApplyError> {
    let actual = baseline.map(SyncBaseline::digest).transpose()?;
    if actual != plan.precondition.baseline_digest {
        return Err(ApplyError::StaleBaseline);
    }
    Ok(())
}

pub fn source_patch(plan: &PlanBundle) -> Result<Option<(&str, &Digest)>, ApplyError> {
    match &plan.patch {
        ArtifactPatch::Source {
            text,
            expected_raw_digest,
        } => Ok(Some((text.as_str(), expected_raw_digest))),
        ArtifactPatch::None => Ok(None),
        ArtifactPatch::Document { .. } => Err(ApplyError::WrongPatchTarget),
    }
}

pub fn document_patch(plan: &PlanBundle) -> Result<Option<(&str, &Digest)>, ApplyError> {
    match &plan.patch {
        ArtifactPatch::Document {
            concrete_spec,
            expected_semantic_digest,
        } => Ok(Some((concrete_spec.as_str(), expected_semantic_digest))),
        ArtifactPatch::None => Ok(None),
        ArtifactPatch::Source { .. } => Err(ApplyError::WrongPatchTarget),
    }
}

pub fn atomic_write_text(path: &Path, text: &str) -> Result<(), ApplyError> {
    atomic_write(path, text.as_bytes())
}

pub fn write_journal(path: &Path, journal: &TransactionJournal) -> Result<(), ApplyError> {
    let bytes = serde_json::to_vec_pretty(journal)?;
    atomic_write(path, &bytes)
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), ApplyError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ApplyError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    let staged = parent.join(format!(".{file_name}.{}.tmp", rand::random::<u64>()));
    fs::write(&staged, bytes).map_err(|source| ApplyError::Write {
        path: staged.clone(),
        source,
    })?;

    #[cfg(unix)]
    {
        fs::rename(&staged, path).map_err(|source| ApplyError::Write {
            path: path.to_path_buf(),
            source,
        })?;
    }

    #[cfg(not(unix))]
    {
        let backup = parent.join(format!(".{file_name}.{}.bak", rand::random::<u64>()));
        let had_destination = path.exists();
        if had_destination {
            fs::rename(path, &backup).map_err(|source| ApplyError::Write {
                path: backup.clone(),
                source,
            })?;
        }
        if let Err(source) = fs::rename(&staged, path) {
            if had_destination {
                let _ = fs::rename(&backup, path);
            }
            return Err(ApplyError::Write {
                path: path.to_path_buf(),
                source,
            });
        }
        if had_destination {
            let _ = fs::remove_file(backup);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::SyncBaseline;
    use crate::planner::plan_compile;
    use crate::snapshot::{ArtifactKind, ArtifactSnapshot};

    #[test]
    fn stale_source_is_rejected() {
        let source =
            ArtifactSnapshot::from_source(ArtifactKind::SchLib, "component R {\n}\n").unwrap();
        let doc = ArtifactSnapshot::empty(ArtifactKind::SchLib);
        let desired = source.clone();
        let plan = plan_compile(
            &source,
            &doc,
            &desired,
            None,
            source.resources[0].text.clone(),
        )
        .unwrap();
        assert!(matches!(
            verify_source_precondition(&plan, Some("component X {\n}\n")),
            Err(ApplyError::StaleSource)
        ));
    }

    #[test]
    fn baseline_precondition_detects_revision_change() {
        let source =
            ArtifactSnapshot::from_source(ArtifactKind::SchLib, "component R {\n}\n").unwrap();
        let doc = source.clone();
        let baseline = SyncBaseline::from_snapshots(None, &source, &doc);
        let plan = plan_compile(&source, &doc, &doc, Some(&baseline), String::new()).unwrap();
        let newer = SyncBaseline::from_snapshots(Some(&baseline), &source, &doc);
        assert!(matches!(
            verify_baseline_precondition(&plan, Some(&newer)),
            Err(ApplyError::StaleBaseline)
        ));
    }
}

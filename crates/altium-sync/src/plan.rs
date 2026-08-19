use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::baseline::SyncBaseline;
use crate::digest::Digest;
use crate::identity::BindingId;
use crate::snapshot::ArtifactKind;

pub const PLAN_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanDirection {
    Compile,
    Dump,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeDisposition {
    SourceOnly,
    DocumentOnly,
    SameChange,
    Conflict,
    Unchanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Add,
    Update,
    Delete,
    Rename,
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticChange {
    pub binding: BindingId,
    pub resource_kind: String,
    pub resource_key: String,
    pub disposition: ChangeDisposition,
    pub change_kind: ChangeKind,
    pub source_before: Option<Digest>,
    pub source_after: Option<Digest>,
    pub document_before: Option<Digest>,
    pub document_after: Option<Digest>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactPrecondition {
    pub source_raw_digest: Option<Digest>,
    /// Exact digest of the native Altium document bytes. This is separate from
    /// the semantic digest because the semantic projection intentionally omits
    /// opaque/native content that still must never be overwritten by a stale plan.
    pub document_raw_digest: Option<Digest>,
    pub document_semantic_digest: Option<Digest>,
    pub baseline_digest: Option<Digest>,
}

/// Exact mutation payload produced by planning.
///
/// Document patches contain the already-materialized, validated Altium CFB file.
/// Apply never invokes the compiler/reconciler/executor again: it stages these
/// exact bytes, reopens them through `altium-format`, verifies the semantic
/// postcondition, and commits them transactionally.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "target", rename_all = "snake_case")]
pub enum ArtifactPatch {
    Document {
        /// Base64-encoded final Altium document bytes.
        document_base64: String,
        expected_semantic_digest: Digest,
    },
    Source {
        text: String,
        expected_raw_digest: Digest,
    },
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanBundle {
    pub schema_version: u32,
    pub plan_id: String,
    pub artifact_kind: ArtifactKind,
    pub direction: PlanDirection,
    pub status: PlanStatus,
    pub source_path: Option<PathBuf>,
    pub document_path: Option<PathBuf>,
    pub precondition: ArtifactPrecondition,
    pub changes: Vec<SemanticChange>,
    pub patch: ArtifactPatch,
    pub next_baseline: SyncBaseline,
}

impl PlanBundle {
    pub fn new(
        artifact_kind: ArtifactKind,
        direction: PlanDirection,
        precondition: ArtifactPrecondition,
        changes: Vec<SemanticChange>,
        patch: ArtifactPatch,
        next_baseline: SyncBaseline,
    ) -> Self {
        let status = if changes
            .iter()
            .any(|change| change.disposition == ChangeDisposition::Conflict)
        {
            PlanStatus::Blocked
        } else {
            PlanStatus::Ready
        };
        Self {
            schema_version: PLAN_SCHEMA_VERSION,
            plan_id: format!("{:032x}", rand::random::<u128>()),
            artifact_kind,
            direction,
            status,
            source_path: None,
            document_path: None,
            precondition,
            changes,
            patch,
            next_baseline,
        }
    }

    pub fn with_paths(
        mut self,
        source_path: Option<PathBuf>,
        document_path: Option<PathBuf>,
    ) -> Self {
        self.source_path = source_path;
        self.document_path = document_path;
        self
    }

    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty() || !matches!(self.patch, ArtifactPatch::None)
    }

    pub fn conflicts(&self) -> impl Iterator<Item = &SemanticChange> {
        self.changes
            .iter()
            .filter(|change| change.disposition == ChangeDisposition::Conflict)
    }
}

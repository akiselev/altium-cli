//! # altium-sync
//!
//! Three-way bidirectional synchronization kernel for Altium documents.
//!
//! The planner consumes authored-source, current-document, and last-synchronized
//! snapshots and produces a versioned self-contained [`PlanBundle`]. The ECO is
//! only a rendered view; apply executes the exact typed patch stored in the plan.
//! Baselines are committed only after staged output is validated and reopened.

pub mod apply;
pub mod baseline;
pub mod digest;
pub mod identity;
pub mod plan;
pub mod planner;
pub mod render;
pub mod snapshot;

pub use apply::{
    ApplyError, JournalState, TransactionJournal, atomic_write, atomic_write_text, document_patch,
    load_plan, save_plan, source_patch, verify_baseline_precondition, verify_document_precondition,
    verify_ready, verify_source_precondition, write_journal,
};
pub use baseline::{
    BASELINE_SCHEMA_VERSION, BaselineError, SyncBaseline, default_baseline_path, load_baseline,
    save_baseline,
};
pub use digest::Digest;
pub use identity::{BindingId, BindingRecord, DocumentLocator};
pub use plan::{
    ArtifactPatch, ArtifactPrecondition, ChangeDisposition, ChangeKind, PLAN_SCHEMA_VERSION,
    PlanBundle, PlanDirection, PlanStatus, SemanticChange,
};
pub use planner::{plan_compile, plan_dump};
pub use render::{PlanSummary, render_plan};
pub use snapshot::{ArtifactKind, ArtifactSnapshot, SnapshotResource};

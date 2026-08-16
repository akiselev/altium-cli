use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::plan::{ChangeDisposition, ChangeKind, PlanBundle, PlanStatus};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanSummary {
    pub adds: usize,
    pub updates: usize,
    pub deletes: usize,
    pub renames: usize,
    pub conflicts: usize,
    pub source_only: usize,
    pub document_only: usize,
    pub same_change: usize,
    pub by_resource_kind: BTreeMap<String, usize>,
}

impl PlanSummary {
    pub fn from_plan(plan: &PlanBundle) -> Self {
        let mut summary = Self::default();
        for change in &plan.changes {
            match change.change_kind {
                ChangeKind::Add => summary.adds += 1,
                ChangeKind::Update => summary.updates += 1,
                ChangeKind::Delete => summary.deletes += 1,
                ChangeKind::Rename => summary.renames += 1,
                ChangeKind::Noop => {}
            }
            match change.disposition {
                ChangeDisposition::Conflict => summary.conflicts += 1,
                ChangeDisposition::SourceOnly => summary.source_only += 1,
                ChangeDisposition::DocumentOnly => summary.document_only += 1,
                ChangeDisposition::SameChange => summary.same_change += 1,
                ChangeDisposition::Unchanged => {}
            }
            *summary.by_resource_kind.entry(change.resource_kind.clone()).or_default() += 1;
        }
        summary
    }
}

pub fn render_plan(plan: &PlanBundle) -> String {
    let summary = PlanSummary::from_plan(plan);
    let status = match plan.status {
        PlanStatus::Ready => "ready",
        PlanStatus::Blocked => "blocked",
    };
    let mut out = format!(
        "Plan {} ({:?} {:?})\nStatus: {}\nChanges: {} add, {} update, {} delete, {} rename; {} conflict\n",
        plan.plan_id,
        plan.direction,
        plan.artifact_kind,
        status,
        summary.adds,
        summary.updates,
        summary.deletes,
        summary.renames,
        summary.conflicts
    );
    for change in &plan.changes {
        out.push_str(&format!(
            "{:?} {:?} {} {} [{}]\n",
            change.disposition,
            change.change_kind,
            change.resource_kind,
            change.resource_key,
            change.binding
        ));
        if let Some(reason) = &change.reason {
            out.push_str("    ");
            out.push_str(reason);
            out.push('\n');
        }
    }
    out
}

use std::collections::{BTreeMap, BTreeSet};

use crate::baseline::SyncBaseline;
use crate::digest::Digest;
use crate::identity::{BindingId, BindingRecord};
use crate::plan::{
    ArtifactPatch, ArtifactPrecondition, ChangeDisposition, ChangeKind, PlanBundle, PlanDirection,
    SemanticChange,
};
use crate::snapshot::{ArtifactSnapshot, SnapshotResource};

pub fn plan_compile(
    source: &ArtifactSnapshot,
    current_document: &ArtifactSnapshot,
    desired_document: &ArtifactSnapshot,
    baseline: Option<&SyncBaseline>,
    document_base64: String,
) -> Result<PlanBundle, serde_json::Error> {
    assert_eq!(source.kind, current_document.kind);
    assert_eq!(source.kind, desired_document.kind);

    let baseline_digest = baseline.map(SyncBaseline::digest).transpose()?;
    let precondition = ArtifactPrecondition {
        source_raw_digest: Some(source.raw_digest.clone()),
        document_raw_digest: None,
        document_semantic_digest: Some(current_document.semantic_digest.clone()),
        baseline_digest,
    };

    let changes = if let Some(base) = baseline {
        three_way_changes(
            PlanDirection::Compile,
            base,
            source,
            current_document,
            desired_document,
        )
    } else {
        bootstrap_changes(PlanDirection::Compile, current_document, desired_document)
    };

    let current_alignment = SyncBaseline::from_snapshots(baseline, source, current_document);
    let next_baseline =
        SyncBaseline::from_snapshots(Some(&current_alignment), source, desired_document);
    let patch = if current_document.semantic_digest == desired_document.semantic_digest {
        ArtifactPatch::None
    } else {
        ArtifactPatch::Document {
            document_base64,
            expected_semantic_digest: desired_document.semantic_digest.clone(),
        }
    };

    Ok(PlanBundle::new(
        source.kind,
        PlanDirection::Compile,
        precondition,
        changes,
        patch,
        next_baseline,
    ))
}

pub fn plan_dump(
    current_source: &ArtifactSnapshot,
    current_document: &ArtifactSnapshot,
    desired_source: &ArtifactSnapshot,
    baseline: Option<&SyncBaseline>,
    desired_text: String,
) -> Result<PlanBundle, serde_json::Error> {
    assert_eq!(current_source.kind, current_document.kind);
    assert_eq!(current_source.kind, desired_source.kind);

    let baseline_digest = baseline.map(SyncBaseline::digest).transpose()?;
    let precondition = ArtifactPrecondition {
        source_raw_digest: Some(current_source.raw_digest.clone()),
        document_raw_digest: None,
        document_semantic_digest: Some(current_document.semantic_digest.clone()),
        baseline_digest,
    };

    let changes = if let Some(base) = baseline {
        three_way_changes(
            PlanDirection::Dump,
            base,
            current_source,
            current_document,
            desired_source,
        )
    } else {
        bootstrap_changes(PlanDirection::Dump, current_source, desired_source)
    };

    let current_alignment =
        SyncBaseline::from_snapshots(baseline, current_source, current_document);
    let next_baseline =
        SyncBaseline::from_snapshots(Some(&current_alignment), desired_source, current_document);
    let patch = if current_source.raw_digest == desired_source.raw_digest {
        ArtifactPatch::None
    } else {
        ArtifactPatch::Source {
            text: desired_text,
            expected_raw_digest: desired_source.raw_digest.clone(),
        }
    };

    Ok(PlanBundle::new(
        current_source.kind,
        PlanDirection::Dump,
        precondition,
        changes,
        patch,
        next_baseline,
    ))
}

fn bootstrap_changes(
    direction: PlanDirection,
    current_target: &ArtifactSnapshot,
    desired_target: &ArtifactSnapshot,
) -> Vec<SemanticChange> {
    let current = resource_map(current_target);
    let desired = resource_map(desired_target);
    let addresses: BTreeSet<_> = current.keys().chain(desired.keys()).cloned().collect();

    let mut changes = Vec::new();
    for address in addresses {
        let before = current.get(&address).copied();
        let after = desired.get(&address).copied();
        if before.map(|resource| &resource.fingerprint)
            == after.map(|resource| &resource.fingerprint)
        {
            continue;
        }
        let disposition = match direction {
            PlanDirection::Compile => ChangeDisposition::SourceOnly,
            PlanDirection::Dump => ChangeDisposition::DocumentOnly,
        };
        let exemplar = after
            .or(before)
            .expect("address originated from the union of current and desired snapshots");
        changes.push(SemanticChange {
            binding: BindingId::mint(),
            resource_kind: exemplar.kind.clone(),
            resource_key: exemplar.key.clone(),
            disposition,
            change_kind: change_kind(before, after),
            source_before: None,
            source_after: None,
            document_before: before.map(|resource| resource.fingerprint.clone()),
            document_after: after.map(|resource| resource.fingerprint.clone()),
            reason: Some("initial adoption; no synchronization baseline exists".to_string()),
        });
    }
    changes
}

fn three_way_changes(
    direction: PlanDirection,
    base: &SyncBaseline,
    current_source: &ArtifactSnapshot,
    current_document: &ArtifactSnapshot,
    desired_target: &ArtifactSnapshot,
) -> Vec<SemanticChange> {
    let current_alignment =
        SyncBaseline::from_snapshots(Some(base), current_source, current_document);

    let final_alignment = match direction {
        PlanDirection::Compile => {
            SyncBaseline::from_snapshots(Some(&current_alignment), current_source, desired_target)
        }
        PlanDirection::Dump => {
            SyncBaseline::from_snapshots(Some(&current_alignment), desired_target, current_document)
        }
    };

    let base_records = records_by_binding(base);
    let current_records = records_by_binding(&current_alignment);
    let final_records = records_by_binding(&final_alignment);
    let bindings: BTreeSet<_> = base_records
        .keys()
        .chain(current_records.keys())
        .chain(final_records.keys())
        .copied()
        .collect();

    let mut changes = Vec::new();
    for binding in bindings {
        let before = base_records.get(&binding).copied();
        let current = current_records.get(&binding).copied();
        let final_record = final_records.get(&binding).copied();

        let source_before = before.and_then(|record| record.source_fingerprint.clone());
        let source_current = current.and_then(|record| record.source_fingerprint.clone());
        let source_final = final_record.and_then(|record| record.source_fingerprint.clone());
        let document_before = before.and_then(|record| record.document_fingerprint.clone());
        let document_current = current.and_then(|record| record.document_fingerprint.clone());
        let document_final = final_record.and_then(|record| record.document_fingerprint.clone());

        let source_changed = source_before != source_current;
        let document_changed = document_before != document_current;
        let target_changed = match direction {
            PlanDirection::Compile => document_current != document_final,
            PlanDirection::Dump => source_current != source_final,
        };
        let converged = source_changed
            && document_changed
            && source_current.is_some()
            && source_current == document_current;

        let (disposition, reason) = classify(
            direction,
            source_changed,
            document_changed,
            target_changed,
            converged,
        );

        if disposition == ChangeDisposition::Unchanged {
            continue;
        }

        let resource_kind = final_record
            .or(current)
            .or(before)
            .map(|record| record.resource_kind.clone())
            .unwrap_or_else(|| "$unknown".to_string());
        let resource_key = final_record
            .and_then(preferred_key)
            .or_else(|| current.and_then(preferred_key))
            .or_else(|| before.and_then(preferred_key))
            .unwrap_or_else(|| "$unknown".to_string());

        let change_kind = match direction {
            PlanDirection::Compile => fingerprint_change_kind(
                document_current.as_ref(),
                document_final.as_ref(),
                current.and_then(|record| record.document_key.as_deref()),
                final_record.and_then(|record| record.document_key.as_deref()),
            ),
            PlanDirection::Dump => fingerprint_change_kind(
                source_current.as_ref(),
                source_final.as_ref(),
                current.and_then(|record| record.source_key.as_deref()),
                final_record.and_then(|record| record.source_key.as_deref()),
            ),
        };

        changes.push(SemanticChange {
            binding,
            resource_kind,
            resource_key,
            disposition,
            change_kind,
            source_before,
            source_after: source_final,
            document_before,
            document_after: document_final,
            reason,
        });
    }
    changes
}

fn classify(
    direction: PlanDirection,
    source_changed: bool,
    document_changed: bool,
    target_changed: bool,
    converged: bool,
) -> (ChangeDisposition, Option<String>) {
    if converged {
        return (
            ChangeDisposition::SameChange,
            Some("both artifacts independently reached the same semantic value".to_string()),
        );
    }
    if source_changed && document_changed {
        return (
            ChangeDisposition::Conflict,
            Some(
                "both artifacts changed since the last synchronized baseline and differ semantically"
                    .to_string(),
            ),
        );
    }

    match direction {
        PlanDirection::Compile => {
            if document_changed {
                return (
                    ChangeDisposition::Conflict,
                    Some(
                        "the Altium document changed since the last synchronized baseline; dump/reconcile it before compiling so the change is not silently absorbed"
                            .to_string(),
                    ),
                );
            }
            if source_changed || target_changed {
                return (ChangeDisposition::SourceOnly, None);
            }
        }
        PlanDirection::Dump => {
            if source_changed {
                return (
                    ChangeDisposition::Conflict,
                    Some(
                        "the authored spec changed since the last synchronized baseline; compile/reconcile it before dumping so the change is not silently absorbed"
                            .to_string(),
                    ),
                );
            }
            if document_changed || target_changed {
                return (ChangeDisposition::DocumentOnly, None);
            }
        }
    }

    (ChangeDisposition::Unchanged, None)
}

fn records_by_binding(baseline: &SyncBaseline) -> BTreeMap<BindingId, &BindingRecord> {
    baseline
        .resources
        .iter()
        .map(|record| (record.binding, record))
        .collect()
}

fn preferred_key(record: &BindingRecord) -> Option<String> {
    record
        .source_key
        .clone()
        .or_else(|| record.document_key.clone())
}

fn resource_map(snapshot: &ArtifactSnapshot) -> BTreeMap<String, &SnapshotResource> {
    snapshot
        .resources
        .iter()
        .map(|resource| (resource.address.clone(), resource))
        .collect()
}

fn change_kind(before: Option<&SnapshotResource>, after: Option<&SnapshotResource>) -> ChangeKind {
    match (before, after) {
        (None, Some(_)) => ChangeKind::Add,
        (Some(_), None) => ChangeKind::Delete,
        (Some(before), Some(after)) if before.address != after.address => ChangeKind::Rename,
        (Some(_), Some(_)) => ChangeKind::Update,
        (None, None) => ChangeKind::Noop,
    }
}

fn fingerprint_change_kind(
    before: Option<&Digest>,
    after: Option<&Digest>,
    before_key: Option<&str>,
    after_key: Option<&str>,
) -> ChangeKind {
    match (before, after) {
        (None, Some(_)) => ChangeKind::Add,
        (Some(_), None) => ChangeKind::Delete,
        (Some(_), Some(_)) if before_key != after_key => ChangeKind::Rename,
        (Some(before), Some(after)) if before != after => ChangeKind::Update,
        _ => ChangeKind::Noop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::ArtifactKind;

    fn snap(text: &str) -> ArtifactSnapshot {
        ArtifactSnapshot::from_source(ArtifactKind::SchLib, text).unwrap()
    }

    #[test]
    fn source_only_change_is_ready() {
        let source0 = snap("component R {\n  description: \"old\"\n}\n");
        let doc0 = source0.clone();
        let base = SyncBaseline::from_snapshots(None, &source0, &doc0);
        let source1 = snap("component R {\n  description: \"new\"\n}\n");
        let desired = source1.clone();
        let plan = plan_compile(
            &source1,
            &doc0,
            &desired,
            Some(&base),
            desired.resources[0].text.clone(),
        )
        .unwrap();
        assert!(plan.conflicts().next().is_none());
        assert!(plan.has_changes());
    }

    #[test]
    fn concurrent_edit_is_blocked() {
        let source0 = snap("component R {\n  description: \"old\"\n}\n");
        let doc0 = source0.clone();
        let base = SyncBaseline::from_snapshots(None, &source0, &doc0);
        let source1 = snap("component R {\n  description: \"source\"\n}\n");
        let doc1 = snap("component R {\n  description: \"document\"\n}\n");
        let desired = source1.clone();
        let plan = plan_compile(
            &source1,
            &doc1,
            &desired,
            Some(&base),
            desired.resources[0].text.clone(),
        )
        .unwrap();
        assert!(plan.conflicts().next().is_some());
    }

    #[test]
    fn simultaneous_identical_drift_is_same_change() {
        let source0 = snap("component R {\n  description: \"old\"\n}\n");
        let doc0 = source0.clone();
        let base = SyncBaseline::from_snapshots(None, &source0, &doc0);
        let source1 = snap("component R {\n  description: \"changed\"\n}\n");
        let doc1 = source1.clone();
        let plan = plan_compile(&source1, &doc1, &doc1, Some(&base), String::new()).unwrap();
        assert!(plan.conflicts().next().is_none());
        assert!(plan
            .changes
            .iter()
            .any(|change| change.disposition == ChangeDisposition::SameChange));
    }

    #[test]
    fn compile_blocks_document_only_drift_even_when_source_does_not_touch_it() {
        let source0 = snap("component R {\n  description: \"old\"\n}\n");
        let doc0 = source0.clone();
        let base = SyncBaseline::from_snapshots(None, &source0, &doc0);
        let doc1 = snap("component R {\n  description: \"gui\"\n}\n");
        let plan = plan_compile(&source0, &doc1, &doc1, Some(&base), String::new()).unwrap();
        assert!(plan.conflicts().next().is_some());
    }

    #[test]
    fn dump_blocks_source_only_drift() {
        let source0 = snap("component R {\n  description: \"old\"\n}\n");
        let doc0 = source0.clone();
        let base = SyncBaseline::from_snapshots(None, &source0, &doc0);
        let source1 = snap("component R {\n  description: \"authored\"\n}\n");
        let plan = plan_dump(&source1, &doc0, &doc0, Some(&base), String::new()).unwrap();
        assert!(plan.conflicts().next().is_some());
    }

    #[test]
    fn dump_detects_document_only_change() {
        let source0 = snap("component R {\n  description: \"old\"\n}\n");
        let doc0 = source0.clone();
        let base = SyncBaseline::from_snapshots(None, &source0, &doc0);
        let doc1 = snap("component R {\n  description: \"gui\"\n}\n");
        let plan = plan_dump(
            &source0,
            &doc1,
            &doc1,
            Some(&base),
            doc1.resources[0].text.clone(),
        )
        .unwrap();
        assert!(plan.conflicts().next().is_none());
        assert!(plan.has_changes());
    }
}

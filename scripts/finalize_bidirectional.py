from pathlib import Path


def replace(text: str, old: str, new: str, label: str) -> str:
    if old in text:
        return text.replace(old, new, 1)
    if new in text:
        return text
    raise SystemExit(f"replacement not found: {label}")


path = Path("crates/altium-cli/src/sync_v2.rs")
text = path.read_text()

text = replace(
    text,
    '''    if plan.direction != PlanDirection::Compile {
        anyhow::bail!("saved plan is a dump/source plan, not a compile/document plan");
    }

    if report_json {
''',
    '''    if report_json {
''',
    "remove compile-only saved-plan guard",
)

text = replace(
    text,
    '''    execute_document_plan(&plan, target, output, force)
}

pub(crate) fn run_dump(
''',
    '''    match plan.direction {
        PlanDirection::Compile => execute_document_plan(&plan, target, output, force),
        PlanDirection::Dump => execute_source_plan(&plan, target, output, force),
    }
}

pub(crate) fn run_dump(
''',
    "dispatch saved plans by direction",
)

text = replace(
    text,
    '''    let document_source = dump_document(kind, document)?;
    let current_source = if spec_path.exists() {
''',
    '''    let document_source = dump_document(kind, document)?;
    let source_existed = spec_path.exists();
    let current_source = if source_existed {
''',
    "record source existence",
)

text = replace(
    text,
    '''    let desired_source = if spec_path.exists() {
''',
    '''    let desired_source = if source_existed {
''',
    "reuse source existence",
)

text = replace(
    text,
    '''    let plan = plan_dump(
        &current_source_snapshot,
        &document_snapshot,
        &desired_source_snapshot,
        baseline.as_ref(),
        desired_source,
    )?
    .with_paths(Some(spec_path.clone()), Some(document.clone()));
''',
    '''    let mut plan = plan_dump(
        &current_source_snapshot,
        &document_snapshot,
        &desired_source_snapshot,
        baseline.as_ref(),
        desired_source,
    )?
    .with_paths(Some(spec_path.clone()), Some(document.clone()));
    if !source_existed {
        plan.precondition.source_raw_digest = None;
    }
''',
    "preserve absent source precondition",
)

old_direct_dump_apply = '''    verify_ready(&plan, force)?;
    verify_source_precondition(&plan, Some(&current_source))?;
    verify_document_precondition(&plan, Some(&document_snapshot.semantic_digest))?;
    verify_baseline_precondition(&plan, baseline.as_ref())?;

    if let Some((text, expected_digest)) = source_patch(&plan)? {
        atomic_write_text(&spec_path, text)?;
        let actual = std::fs::read_to_string(&spec_path)?;
        let actual_digest = altium_sync::Digest::text(&actual);
        if &actual_digest != expected_digest {
            anyhow::bail!("source postcondition failed after atomic write");
        }
    }
    save_baseline(&baseline_path, &plan.next_baseline)?;
    println!(
        "Synchronized: {} -> {}",
        document.display(),
        spec_path.display()
    );
    Ok(())
}
'''
new_direct_dump_apply = '''    execute_source_plan(&plan, Some(document), Some(&spec_path), force)
}
'''
text = replace(text, old_direct_dump_apply, new_direct_dump_apply, "route direct dump through exact plan executor")

text = replace(
    text,
    '''    let current_document_source = if document_path.exists() {
        dump_document(kind, &document_path)?
    } else {
        String::new()
    };
''',
    '''    let document_existed = document_path.exists();
    let current_document_source = if document_existed {
        dump_document(kind, &document_path)?
    } else {
        String::new()
    };
''',
    "record document existence",
)

text = replace(
    text,
    '''    Ok(plan_compile(
        &source_snapshot,
        &current_document,
        &desired_document,
        baseline.as_ref(),
        BASE64.encode(&desired.bytes),
    )?
    .with_paths(Some(spec_file.clone()), Some(document_path)))
}
''',
    '''    let mut plan = plan_compile(
        &source_snapshot,
        &current_document,
        &desired_document,
        baseline.as_ref(),
        BASE64.encode(&desired.bytes),
    )?
    .with_paths(Some(spec_file.clone()), Some(document_path));
    if !document_existed {
        plan.precondition.document_semantic_digest = None;
    }
    Ok(plan)
}
''',
    "preserve absent document precondition",
)

source_executor = r'''fn execute_source_plan(
    plan: &PlanBundle,
    document_override: Option<&PathBuf>,
    source_override: Option<&PathBuf>,
    force: bool,
) -> anyhow::Result<()> {
    verify_ready(plan, force)?;
    let document = document_override
        .cloned()
        .or_else(|| plan.document_path.clone())
        .ok_or_else(|| anyhow::anyhow!("saved dump plan does not identify a document; pass --target"))?;
    let source_path = source_override
        .cloned()
        .or_else(|| plan.source_path.clone())
        .ok_or_else(|| anyhow::anyhow!("saved dump plan does not identify a source; pass --output"))?;

    let current_source = if source_path.exists() {
        Some(std::fs::read_to_string(&source_path)?)
    } else {
        None
    };
    verify_source_precondition(plan, current_source.as_deref())?;

    let document_digest = if document.exists() {
        let dumped = dump_document(plan.artifact_kind, &document)?;
        Some(ArtifactSnapshot::from_source(plan.artifact_kind, &dumped)?.semantic_digest)
    } else {
        None
    };
    verify_document_precondition(plan, document_digest.as_ref())?;

    let baseline_path = default_baseline_path(&document);
    let baseline = load_baseline(&baseline_path, plan.artifact_kind)?;
    verify_baseline_precondition(plan, baseline.as_ref())?;

    let Some((text, expected_digest)) = source_patch(plan)? else {
        if baseline.is_none() && plan.conflicts().next().is_none() {
            save_baseline(&baseline_path, &plan.next_baseline)?;
        }
        println!("Already converged: {}", source_path.display());
        return Ok(());
    };

    atomic_write_text(&source_path, text)?;
    let actual = std::fs::read_to_string(&source_path)?;
    let actual_digest = altium_sync::Digest::text(&actual);
    if &actual_digest != expected_digest {
        anyhow::bail!("source postcondition failed after atomic write");
    }
    save_baseline(&baseline_path, &plan.next_baseline)?;
    println!("Saved: {}", source_path.display());
    Ok(())
}

'''
marker = "fn execute_document_plan(\n"
if "fn execute_source_plan(" not in text:
    if marker not in text:
        raise SystemExit("execute_document_plan marker not found")
    text = text.replace(marker, source_executor + marker, 1)

text = replace(
    text,
    '''    let current_document_source = if target.exists() {
        dump_document(plan.artifact_kind, &target)?
    } else {
        String::new()
    };
    let current_document =
        ArtifactSnapshot::from_source(plan.artifact_kind, &current_document_source)?;
    verify_document_precondition(plan, Some(&current_document.semantic_digest))?;
''',
    '''    let current_document_digest = if target.exists() {
        let dumped = dump_document(plan.artifact_kind, &target)?;
        Some(ArtifactSnapshot::from_source(plan.artifact_kind, &dumped)?.semantic_digest)
    } else {
        None
    };
    verify_document_precondition(plan, current_document_digest.as_ref())?;
''',
    "verify absent document exactly",
)

path.write_text(text)

planner = Path("crates/altium-sync/src/planner.rs")
text = planner.read_text()
old = '''    if source_changed && document_changed {
        if target_changed {
            return (
                ChangeDisposition::Conflict,
                Some("both artifacts changed since the last synchronized baseline".to_string()),
            );
        }
        return (ChangeDisposition::SameChange, None);
    }
'''
new = '''    if source_changed && document_changed {
        return (
            ChangeDisposition::Conflict,
            Some(
                "both artifacts changed since the last synchronized baseline; the current aggregate model cannot prove the edits are semantically identical"
                    .to_string(),
            ),
        );
    }
'''
text = replace(text, old, new, "make simultaneous drift conservative")

marker = '''    #[test]
    fn compile_blocks_document_only_drift_even_when_source_does_not_touch_it() {
'''
test = '''    #[test]
    fn simultaneous_drift_is_conflict_even_when_target_is_already_converged() {
        let source0 = snap("component R {\\n  description: \\"old\\"\\n}\\n");
        let doc0 = source0.clone();
        let base = SyncBaseline::from_snapshots(None, &source0, &doc0);
        let source1 = snap("component R {\\n  description: \\"changed\\"\\n}\\n");
        let doc1 = source1.clone();
        let plan = plan_compile(&source1, &doc1, &doc1, Some(&base), String::new()).unwrap();
        assert!(plan.conflicts().next().is_some());
    }

'''
if "simultaneous_drift_is_conflict_even_when_target_is_already_converged" not in text:
    if marker not in text:
        raise SystemExit("planner test marker not found")
    text = text.replace(marker, test + marker, 1)
planner.write_text(text)

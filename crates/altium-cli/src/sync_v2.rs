use std::path::{Path, PathBuf};

use altium_format::{PcbDoc, PcbLib, SchDoc, SchLib};
use altium_format_spec::{
    SpecDomain, apply_spec_pcbdoc, apply_spec_pcblib, apply_spec_schdoc, apply_spec_schlib,
    dump_pcbdoc, dump_pcblib, dump_schdoc, dump_schlib, merge_dump,
};
use altium_sync::{
    ArtifactKind, ArtifactSnapshot, JournalState, PlanBundle, PlanDirection, TransactionJournal,
    atomic_write, atomic_write_text, default_baseline_path, document_patch, load_baseline,
    load_plan, plan_compile, plan_dump, render_plan, save_baseline, save_plan, source_patch,
    verify_baseline_precondition, verify_document_precondition, verify_ready,
    verify_source_precondition, write_journal,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use super::{
    CompileResult, build_pad_net_map, compile_and_resolve, default_output_for_spec,
    default_spec_for_document, detect_document_domain, detect_spec_domain,
    instantiate_footprint_primitives,
};

pub(crate) fn run_plan(
    spec_file: &PathBuf,
    target: Option<&PathBuf>,
    json: bool,
    out_plan: Option<&PathBuf>,
) -> anyhow::Result<bool> {
    let plan = build_compile_plan(spec_file, target)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        println!("{}", render_plan(&plan));
    }
    if let Some(path) = out_plan {
        save_plan(path, &plan)?;
        eprintln!("Saved plan: {}", path.display());
    }
    Ok(plan.has_changes())
}

pub(crate) fn run_apply(
    spec_file: Option<&PathBuf>,
    saved_plan: Option<&PathBuf>,
    target: Option<&PathBuf>,
    output: Option<&PathBuf>,
    report_json: bool,
    force: bool,
) -> anyhow::Result<()> {
    let plan = match saved_plan {
        Some(path) => load_plan(path)?,
        None => build_compile_plan(
            spec_file.ok_or_else(|| anyhow::anyhow!("a spec file or --plan is required"))?,
            target,
        )?,
    };

    if plan.direction != PlanDirection::Compile {
        anyhow::bail!("saved plan is a dump/source plan, not a compile/document plan");
    }

    if report_json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        println!("{}", render_plan(&plan));
    }

    execute_document_plan(&plan, target, output, force)
}

pub(crate) fn run_dump(
    document: &PathBuf,
    output: Option<&PathBuf>,
    plan_only: bool,
    out_plan: Option<&PathBuf>,
    force: bool,
) -> anyhow::Result<()> {
    let legacy_domain = detect_document_domain(document)?;
    let kind = kind_from_domain(&legacy_domain)
        .ok_or_else(|| anyhow::anyhow!("sync v2 does not manage PrjPcb yet"))?;
    let spec_path = output
        .cloned()
        .unwrap_or_else(|| default_spec_for_document(document, &legacy_domain));

    let document_source = dump_document(kind, document)?;
    let current_source = if spec_path.exists() {
        std::fs::read_to_string(&spec_path)?
    } else {
        String::new()
    };
    let desired_source = if spec_path.exists() {
        merge_dump(&current_source, &document_source)
            .map_err(|error| anyhow::anyhow!("structured dump merge failed: {error}"))?
    } else {
        document_source.clone()
    };

    let current_source_snapshot = ArtifactSnapshot::from_source(kind, &current_source)?;
    let document_snapshot = ArtifactSnapshot::from_source(kind, &document_source)?;
    let desired_source_snapshot = ArtifactSnapshot::from_source(kind, &desired_source)?;
    let baseline_path = default_baseline_path(document);
    let baseline = load_baseline(&baseline_path, kind)?;
    let plan = plan_dump(
        &current_source_snapshot,
        &document_snapshot,
        &desired_source_snapshot,
        baseline.as_ref(),
        desired_source,
    )?
    .with_paths(Some(spec_path.clone()), Some(document.clone()));

    println!("{}", render_plan(&plan));
    if let Some(path) = out_plan {
        save_plan(path, &plan)?;
        eprintln!("Saved plan: {}", path.display());
    }
    if plan_only {
        return Ok(());
    }

    verify_ready(&plan, force)?;
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

fn build_compile_plan(spec_file: &PathBuf, target: Option<&PathBuf>) -> anyhow::Result<PlanBundle> {
    let legacy_domain = detect_spec_domain(spec_file)?;
    let kind = kind_from_domain(&legacy_domain)
        .ok_or_else(|| anyhow::anyhow!("sync v2 does not manage PrjPcb yet"))?;
    let source = std::fs::read_to_string(spec_file)
        .map_err(|error| anyhow::anyhow!("failed to read {}: {error}", spec_file.display()))?;
    let compiled = compile_and_resolve(&source, spec_file, &legacy_domain)?;
    let document_path = target
        .cloned()
        .unwrap_or_else(|| default_output_for_spec(spec_file, &legacy_domain));

    let current_document_source = if document_path.exists() {
        dump_document(kind, &document_path)?
    } else {
        String::new()
    };
    let desired = materialize_desired(kind, &compiled, spec_file, &document_path)?;

    let source_snapshot = ArtifactSnapshot::from_source(kind, &source)?;
    let current_document = ArtifactSnapshot::from_source(kind, &current_document_source)?;
    let desired_document = ArtifactSnapshot::from_source(kind, &desired.dump)?;
    let baseline_path = default_baseline_path(&document_path);
    let baseline = load_baseline(&baseline_path, kind)?;

    Ok(plan_compile(
        &source_snapshot,
        &current_document,
        &desired_document,
        baseline.as_ref(),
        BASE64.encode(&desired.bytes),
    )?
    .with_paths(Some(spec_file.clone()), Some(document_path)))
}

struct MaterializedDocument {
    dump: String,
    bytes: Vec<u8>,
}

fn materialize_desired(
    kind: ArtifactKind,
    compiled: &CompileResult,
    spec_file: &PathBuf,
    document_path: &PathBuf,
) -> anyhow::Result<MaterializedDocument> {
    match (&compiled.model, kind) {
        (altium_format_spec::model::SpecModel::SchLib(spec), ArtifactKind::SchLib) => {
            let mut doc = if document_path.exists() {
                SchLib::open(document_path)?
            } else {
                let mut doc = SchLib::new_blank_ad26()?;
                let _ = doc.remove_component("Component_1");
                doc
            };
            apply_spec_schlib(spec, &mut doc)
                .map_err(|error| anyhow::anyhow!("apply failed while planning: {error}"))?;
            doc.validate_invariants()?;
            persist_schlib(&mut doc)
        }
        (altium_format_spec::model::SpecModel::PcbLib(spec), ArtifactKind::PcbLib) => {
            let mut doc = if document_path.exists() {
                PcbLib::open(document_path)?
            } else {
                PcbLib::new_blank_ad26()?
            };
            apply_spec_pcblib(spec, &mut doc)
                .map_err(|error| anyhow::anyhow!("apply failed while planning: {error}"))?;
            doc.validate_invariants()?;
            persist_pcblib(&mut doc)
        }
        (altium_format_spec::model::SpecModel::SchDoc(spec), ArtifactKind::SchDoc) => {
            let mut doc = if document_path.exists() {
                SchDoc::open(document_path)?
            } else {
                SchDoc::new_blank_ad26()
            };
            apply_spec_schdoc(spec, &mut doc, &compiled.imported_components)
                .map_err(|error| anyhow::anyhow!("apply failed while planning: {error}"))?;
            doc.validate_invariants()?;
            persist_schdoc(&mut doc)
        }
        (altium_format_spec::model::SpecModel::PcbDoc(spec), ArtifactKind::PcbDoc) => {
            if !document_path.exists() {
                anyhow::bail!(
                    "PcbDoc compile requires an existing target file: {}",
                    document_path.display()
                );
            }
            let mut doc = PcbDoc::open(document_path)?;
            apply_spec_pcbdoc(spec, &mut doc)
                .map_err(|error| anyhow::anyhow!("apply failed while planning: {error}"))?;
            let pad_net_map = build_pad_net_map(spec_file)?;
            instantiate_footprint_primitives(
                &mut doc,
                &compiled.import_paths,
                &pad_net_map,
                &compiled.imported_components,
            )?;
            doc.validate_invariants()?;
            persist_pcbdoc(&mut doc)
        }
        _ => anyhow::bail!("compiled spec domain does not match sync artifact kind"),
    }
}

fn persist_schlib(doc: &mut SchLib) -> anyhow::Result<MaterializedDocument> {
    let file = tempfile::NamedTempFile::new()?;
    doc.save(file.path())?;
    let reopened = SchLib::open(file.path())?;
    reopened.validate_invariants()?;
    Ok(MaterializedDocument {
        dump: dump_schlib(&reopened)?,
        bytes: std::fs::read(file.path())?,
    })
}

fn persist_pcblib(doc: &mut PcbLib) -> anyhow::Result<MaterializedDocument> {
    let file = tempfile::NamedTempFile::new()?;
    doc.save(file.path())?;
    let reopened = PcbLib::open(file.path())?;
    reopened.validate_invariants()?;
    Ok(MaterializedDocument {
        dump: dump_pcblib(&reopened)?,
        bytes: std::fs::read(file.path())?,
    })
}

fn persist_schdoc(doc: &mut SchDoc) -> anyhow::Result<MaterializedDocument> {
    let file = tempfile::NamedTempFile::new()?;
    doc.save(file.path())?;
    let reopened = SchDoc::open(file.path())?;
    reopened.validate_invariants()?;
    Ok(MaterializedDocument {
        dump: dump_schdoc(&reopened)?,
        bytes: std::fs::read(file.path())?,
    })
}

fn persist_pcbdoc(doc: &mut PcbDoc) -> anyhow::Result<MaterializedDocument> {
    let file = tempfile::NamedTempFile::new()?;
    doc.save(file.path())?;
    let reopened = PcbDoc::open(file.path())?;
    reopened.validate_invariants()?;
    Ok(MaterializedDocument {
        dump: dump_pcbdoc(&reopened)?,
        bytes: std::fs::read(file.path())?,
    })
}

fn execute_document_plan(
    plan: &PlanBundle,
    target_override: Option<&PathBuf>,
    output_override: Option<&PathBuf>,
    force: bool,
) -> anyhow::Result<()> {
    verify_ready(plan, force)?;
    let target = target_override
        .cloned()
        .or_else(|| plan.document_path.clone())
        .ok_or_else(|| anyhow::anyhow!("saved plan does not identify a target; pass --target"))?;
    let output = output_override.cloned().unwrap_or_else(|| target.clone());

    if let Some(source_path) = &plan.source_path {
        if source_path.exists() {
            let source = std::fs::read_to_string(source_path)?;
            verify_source_precondition(plan, Some(&source))?;
        }
    }

    let current_document_source = if target.exists() {
        dump_document(plan.artifact_kind, &target)?
    } else {
        String::new()
    };
    let current_document =
        ArtifactSnapshot::from_source(plan.artifact_kind, &current_document_source)?;
    verify_document_precondition(plan, Some(&current_document.semantic_digest))?;

    let baseline_path = default_baseline_path(&target);
    let baseline = load_baseline(&baseline_path, plan.artifact_kind)?;
    verify_baseline_precondition(plan, baseline.as_ref())?;

    let Some((document_base64, expected_digest)) = document_patch(plan)? else {
        save_baseline(&default_baseline_path(&output), &plan.next_baseline)?;
        println!("Already converged: {}", output.display());
        return Ok(());
    };

    let stage = stage_path(&output, &plan.plan_id, plan.artifact_kind);
    let document_bytes = BASE64
        .decode(document_base64)
        .map_err(|error| anyhow::anyhow!("invalid document payload in saved plan: {error}"))?;
    atomic_write(&stage, &document_bytes)?;
    let reopened = dump_document(plan.artifact_kind, &stage)?;
    let reopened_snapshot = ArtifactSnapshot::from_source(plan.artifact_kind, &reopened)?;
    if &reopened_snapshot.semantic_digest != expected_digest {
        let _ = std::fs::remove_file(&stage);
        anyhow::bail!(
            "document postcondition failed: expected {}, got {}",
            expected_digest,
            reopened_snapshot.semantic_digest
        );
    }

    commit_stage(plan, &stage, &output)?;
    save_baseline(&default_baseline_path(&output), &plan.next_baseline)?;
    println!("Saved: {}", output.display());
    Ok(())
}

fn commit_stage(plan: &PlanBundle, stage: &Path, destination: &Path) -> anyhow::Result<()> {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    let altium_dir = parent.join(".altium");
    std::fs::create_dir_all(&altium_dir)?;
    let journal_path = altium_dir.join(format!("journal-{}.json", plan.plan_id));
    let backup = destination.exists().then(|| {
        parent.join(format!(
            ".{}.{}.backup",
            destination
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("artifact"),
            plan.plan_id
        ))
    });
    let mut journal = TransactionJournal {
        plan_id: plan.plan_id.clone(),
        destination: destination.to_path_buf(),
        staged: stage.to_path_buf(),
        backup: backup.clone(),
        state: JournalState::Staged,
    };
    write_journal(&journal_path, &journal)?;

    if let Some(backup) = &backup {
        std::fs::rename(destination, backup)?;
    }
    if let Err(error) = std::fs::rename(stage, destination) {
        if let Some(backup) = &backup {
            let _ = std::fs::rename(backup, destination);
        }
        return Err(error.into());
    }
    journal.state = JournalState::Committed;
    write_journal(&journal_path, &journal)?;
    if let Some(backup) = backup {
        let _ = std::fs::remove_file(backup);
    }
    let _ = std::fs::remove_file(journal_path);
    Ok(())
}

fn dump_document(kind: ArtifactKind, path: &Path) -> anyhow::Result<String> {
    match kind {
        ArtifactKind::SchLib => Ok(dump_schlib(&SchLib::open(path)?)?),
        ArtifactKind::PcbLib => Ok(dump_pcblib(&PcbLib::open(path)?)?),
        ArtifactKind::SchDoc => Ok(dump_schdoc(&SchDoc::open(path)?)?),
        ArtifactKind::PcbDoc => Ok(dump_pcbdoc(&PcbDoc::open(path)?)?),
    }
}

fn stage_path(output: &Path, plan_id: &str, kind: ArtifactKind) -> PathBuf {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let stem = output
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("artifact");
    parent.join(format!(
        ".{stem}.{plan_id}.stage.{}",
        kind.document_extension()
    ))
}

pub(crate) fn kind_from_domain(domain: &SpecDomain) -> Option<ArtifactKind> {
    match domain {
        SpecDomain::SchLib => Some(ArtifactKind::SchLib),
        SpecDomain::PcbLib => Some(ArtifactKind::PcbLib),
        SpecDomain::SchDoc => Some(ArtifactKind::SchDoc),
        SpecDomain::PcbDoc => Some(ArtifactKind::PcbDoc),
        SpecDomain::PrjPcb => None,
    }
}

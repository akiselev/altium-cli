from pathlib import Path


def replace(path: str, old: str, new: str, count: int = 1) -> None:
    p = Path(path)
    text = p.read_text()
    actual = text.count(old)
    if actual == 0 and new in text:
        return
    if actual != count:
        raise SystemExit(f"{path}: expected {count} occurrences, found {actual}: {old[:120]!r}")
    p.write_text(text.replace(old, new, count))


replace(
    "crates/altium-sync/src/planner.rs",
    "    concrete_spec: String,\n",
    "    document_base64: String,\n",
)
replace(
    "crates/altium-sync/src/planner.rs",
    "        ArtifactPatch::Document {\n            concrete_spec,\n            expected_semantic_digest: desired_document.semantic_digest.clone(),\n        }",
    "        ArtifactPatch::Document {\n            document_base64,\n            expected_semantic_digest: desired_document.semantic_digest.clone(),\n        }",
)
replace(
    "crates/altium-sync/src/apply.rs",
    "        ArtifactPatch::Document {\n            concrete_spec,\n            expected_semantic_digest,\n        } => Ok(Some((concrete_spec.as_str(), expected_semantic_digest))),",
    "        ArtifactPatch::Document {\n            document_base64,\n            expected_semantic_digest,\n        } => Ok(Some((document_base64.as_str(), expected_semantic_digest))),",
)

path = Path("crates/altium-cli/src/sync_v2.rs")
text = path.read_text()
text = text.replace("use std::collections::HashMap;\n", "")
if "use base64::" not in text:
    text = text.replace(
        "use altium_format::{PcbDoc, PcbLib, SchDoc, SchLib};\n",
        "use altium_format::{PcbDoc, PcbLib, SchDoc, SchLib};\nuse base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};\n",
    )
text = text.replace(
    "    atomic_write_text, default_baseline_path, document_patch, load_baseline, load_plan,\n",
    "    atomic_write, atomic_write_text, default_baseline_path, document_patch, load_baseline, load_plan,\n",
)
text = text.replace(
    "    let desired_document_source = materialize_desired(kind, &compiled, spec_file, &document_path)?;\n\n    let source_snapshot = ArtifactSnapshot::from_source(kind, &source)?;\n    let current_document = ArtifactSnapshot::from_source(kind, &current_document_source)?;\n    let desired_document = ArtifactSnapshot::from_source(kind, &desired_document_source)?;",
    "    let desired = materialize_desired(kind, &compiled, spec_file, &document_path)?;\n\n    let source_snapshot = ArtifactSnapshot::from_source(kind, &source)?;\n    let current_document = ArtifactSnapshot::from_source(kind, &current_document_source)?;\n    let desired_document = ArtifactSnapshot::from_source(kind, &desired.dump)?;",
)
text = text.replace(
    "        desired_document_source,\n",
    "        BASE64.encode(&desired.bytes),\n",
    1,
)

start = text.index("fn materialize_desired(")
end = text.index("fn execute_document_plan(")
new_materialize = r'''struct MaterializedDocument {
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

'''
text = text[:start] + new_materialize + text[end:]
text = text.replace(
    "    let Some((concrete_spec, expected_digest)) = document_patch(plan)? else {",
    "    let Some((document_base64, expected_digest)) = document_patch(plan)? else {",
)
text = text.replace(
    "    let stage = stage_path(&output, &plan.plan_id, plan.artifact_kind);\n    apply_concrete_spec(plan.artifact_kind, concrete_spec, &target, &stage)?;",
    "    let stage = stage_path(&output, &plan.plan_id, plan.artifact_kind);\n    let document_bytes = BASE64\n        .decode(document_base64)\n        .map_err(|error| anyhow::anyhow!(\"invalid document payload in saved plan: {error}\"))?;\n    atomic_write(&stage, &document_bytes)?;",
)

if "fn apply_concrete_spec(" in text:
    start = text.index("fn apply_concrete_spec(")
    end = text.index("fn commit_stage(")
    text = text[:start] + text[end:]

if "fn domain_from_kind(" in text:
    start = text.index("fn domain_from_kind(")
    text = text[:start].rstrip() + "\n"

path.write_text(text)

cargo = Path("crates/altium-cli/Cargo.toml")
text = cargo.read_text()
if 'base64 = "0.22"' not in text:
    text = text.replace('anyhow = "1.0.102"\n', 'anyhow = "1.0.102"\nbase64 = "0.22"\n')
if 'tempfile = "3"\n' not in text.split('[dev-dependencies]')[0]:
    text = text.replace('tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }\n', 'tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }\ntempfile = "3"\n')
parts = text.split('[dev-dependencies]')
if len(parts) == 2:
    parts[1] = parts[1].replace('\ntempfile = "3"', '')
    text = '[dev-dependencies]'.join(parts)
cargo.write_text(text)

from pathlib import Path

planner = Path("crates/altium-sync/src/planner.rs")
text = planner.read_text()
old = '''        PlanDirection::Compile => {
            if document_changed && target_changed {
                return (
                    ChangeDisposition::Conflict,
                    Some(
                        "compile would overwrite a document-only edit; dump/reconcile it first or force the reviewed plan"
                            .to_string(),
                    ),
                );
            }
            if source_changed || target_changed {
                return (ChangeDisposition::SourceOnly, None);
            }
            if document_changed {
                return (ChangeDisposition::DocumentOnly, None);
            }
        }
        PlanDirection::Dump => {
            if source_changed && target_changed {
                return (
                    ChangeDisposition::Conflict,
                    Some(
                        "dump would overwrite a source-only edit; compile/reconcile it first or force the reviewed plan"
                            .to_string(),
                    ),
                );
            }
            if document_changed || target_changed {
                return (ChangeDisposition::DocumentOnly, None);
            }
            if source_changed {
                return (ChangeDisposition::SourceOnly, None);
            }
        }
'''
new = '''        PlanDirection::Compile => {
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
'''
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise SystemExit("planner classification block not found")

# Add regression tests if they are not already present.
marker = '''    #[test]
    fn dump_detects_document_only_change() {
'''
insert = '''    #[test]
    fn compile_blocks_document_only_drift_even_when_source_does_not_touch_it() {
        let source0 = snap("component R {\\n  description: \\"old\\"\\n}\\n");
        let doc0 = source0.clone();
        let base = SyncBaseline::from_snapshots(None, &source0, &doc0);
        let doc1 = snap("component R {\\n  description: \\"gui\\"\\n}\\n");
        let plan = plan_compile(
            &source0,
            &doc1,
            &doc1,
            Some(&base),
            String::new(),
        )
        .unwrap();
        assert!(plan.conflicts().next().is_some());
    }

    #[test]
    fn dump_blocks_source_only_drift() {
        let source0 = snap("component R {\\n  description: \\"old\\"\\n}\\n");
        let doc0 = source0.clone();
        let base = SyncBaseline::from_snapshots(None, &source0, &doc0);
        let source1 = snap("component R {\\n  description: \\"authored\\"\\n}\\n");
        let plan = plan_dump(
            &source1,
            &doc0,
            &doc0,
            Some(&base),
            String::new(),
        )
        .unwrap();
        assert!(plan.conflicts().next().is_some());
    }

'''
if "compile_blocks_document_only_drift_even_when_source_does_not_touch_it" not in text:
    if marker not in text:
        raise SystemExit("planner test insertion marker not found")
    text = text.replace(marker, insert + marker, 1)
planner.write_text(text)

sync = Path("crates/altium-cli/src/sync_v2.rs")
text = sync.read_text()
old = '''    if let Some(source_path) = &plan.source_path {
        if source_path.exists() {
            let source = std::fs::read_to_string(source_path)?;
            verify_source_precondition(plan, Some(&source))?;
        }
    }
'''
new = '''    if let Some(source_path) = &plan.source_path {
        let source = if source_path.exists() {
            Some(std::fs::read_to_string(source_path)?)
        } else {
            None
        };
        verify_source_precondition(plan, source.as_deref())?;
    }
'''
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise SystemExit("source precondition block not found")

old = '''    let Some((document_base64, expected_digest)) = document_patch(plan)? else {
        save_baseline(&default_baseline_path(&output), &plan.next_baseline)?;
        println!("Already converged: {}", output.display());
        return Ok(());
    };
'''
new = '''    let Some((document_base64, expected_digest)) = document_patch(plan)? else {
        // Existing baselines only advance when this operation actually resolves
        // a side. Otherwise opposite-side drift could be silently accepted.
        // Initial no-op adoption is safe because there is no prior baseline.
        if baseline.is_none() && plan.conflicts().next().is_none() {
            save_baseline(&default_baseline_path(&output), &plan.next_baseline)?;
        }
        println!("Already converged: {}", output.display());
        return Ok(());
    };
'''
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise SystemExit("no-op baseline block not found")
sync.write_text(text)

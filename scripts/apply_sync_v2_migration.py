from pathlib import Path

MAIN = Path("crates/altium-cli/src/main.rs")
text = MAIN.read_text()


def replace(old: str, new: str, *, count: int = 1) -> None:
    global text
    actual = text.count(old)
    if actual != count:
        raise SystemExit(f"expected {count} occurrences, found {actual}: {old[:100]!r}")
    text = text.replace(old, new, count)


replace("mod cfb;\n", "mod cfb;\nmod sync_v2;\n")

replace(
'''        /// Process this spec and all imported specs (PrjPcb only)\n        #[arg(long, default_value_t = false)]\n        all: bool,\n    },\n    /// Apply a spec file to create or update an Altium document\n    Apply {\n        /// Path to the spec file (.schlib-spec, .pcblib-spec, .schdoc-spec, .pcbdoc-spec, or .prjpcb-spec)\n        spec_file: PathBuf,\n        /// Existing document to update (optional)\n        #[arg(long)]\n        target: Option<PathBuf>,\n        /// Output file path (overrides default)\n        #[arg(long)]\n        output: Option<PathBuf>,\n        /// Print apply report as JSON\n        #[arg(long, default_value_t = false)]\n        report_json: bool,\n        /// Process this spec and all imported specs (PrjPcb only)\n        #[arg(long, default_value_t = false)]\n        all: bool,\n    },\n    /// Reverse-generate a spec file from an existing Altium document\n    Dump {\n        /// Path to a supported Altium document\n        document: PathBuf,\n        /// Output spec file path (overrides default)\n        #[arg(long)]\n        output: Option<PathBuf>,\n    },''',
'''        /// Process this spec and all imported specs (PrjPcb only)\n        #[arg(long, default_value_t = false)]\n        all: bool,\n        /// Persist the self-contained plan as JSON (sync-v2 formats only)\n        #[arg(long)]\n        out_plan: Option<PathBuf>,\n    },\n    /// Apply a spec file or a previously saved synchronization plan\n    Apply {\n        /// Path to the spec file. Omit when applying --plan.\n        spec_file: Option<PathBuf>,\n        /// Apply this previously saved self-contained plan\n        #[arg(long, conflicts_with = "spec_file")]\n        plan: Option<PathBuf>,\n        /// Existing document to update (optional for spec apply; overrides saved-plan path)\n        #[arg(long)]\n        target: Option<PathBuf>,\n        /// Output file path (overrides default)\n        #[arg(long)]\n        output: Option<PathBuf>,\n        /// Print apply report as JSON\n        #[arg(long, default_value_t = false)]\n        report_json: bool,\n        /// Process this spec and all imported specs (PrjPcb only)\n        #[arg(long, default_value_t = false)]\n        all: bool,\n        /// Apply a reviewed plan even when it contains three-way conflicts\n        #[arg(long, default_value_t = false)]\n        force: bool,\n    },\n    /// Reverse-synchronize an Altium document into its spec source\n    Dump {\n        /// Path to a supported Altium document\n        document: PathBuf,\n        /// Output spec file path (overrides default)\n        #[arg(long)]\n        output: Option<PathBuf>,\n        /// Plan the structured source update without writing it\n        #[arg(long, default_value_t = false)]\n        plan: bool,\n        /// Persist the self-contained dump plan as JSON\n        #[arg(long)]\n        out_plan: Option<PathBuf>,\n        /// Apply a reviewed plan even when both artifacts changed\n        #[arg(long, default_value_t = false)]\n        force: bool,\n    },'''
)

replace(
'''        Commands::Plan {\n            spec_file,\n            target,\n            json,\n            all,\n        } => match run_plan(&spec_file, target.as_ref(), json, all) {\n            Ok(has_changes) => {\n                if has_changes {\n                    return ExitCode::from(1);\n                }\n            }''',
'''        Commands::Plan {\n            spec_file,\n            target,\n            json,\n            all,\n            out_plan,\n        } => match run_plan(&spec_file, target.as_ref(), json, all, out_plan.as_ref()) {\n            Ok(has_changes) => {\n                if has_changes {\n                    return ExitCode::from(2);\n                }\n            }'''
)

replace(
'''        Commands::Apply {\n            spec_file,\n            target,\n            output,\n            report_json,\n            all,\n        } => {\n            if let Err(e) = run_apply(\n                &spec_file,\n                target.as_ref(),\n                output.as_ref(),\n                report_json,\n                all,\n            ) {''',
'''        Commands::Apply {\n            spec_file,\n            plan,\n            target,\n            output,\n            report_json,\n            all,\n            force,\n        } => {\n            if let Err(e) = run_apply(\n                spec_file.as_ref(),\n                plan.as_ref(),\n                target.as_ref(),\n                output.as_ref(),\n                report_json,\n                all,\n                force,\n            ) {'''
)

replace(
'''        Commands::Dump { document, output } => {\n            if let Err(e) = run_dump(&document, output.as_ref()) {''',
'''        Commands::Dump {\n            document,\n            output,\n            plan,\n            out_plan,\n            force,\n        } => {\n            if let Err(e) = run_dump(\n                &document,\n                output.as_ref(),\n                plan,\n                out_plan.as_ref(),\n                force,\n            ) {'''
)

replace(
'''fn run_plan(\n    spec_file: &PathBuf,\n    target: Option<&PathBuf>,\n    json: bool,\n    all: bool,\n) -> anyhow::Result<bool> {\n    let domain = detect_spec_domain(spec_file)?;\n    if all && domain != SpecDomain::PrjPcb {\n        anyhow::bail!("--all is only valid for .prjpcb-spec files");\n    }\n''',
'''fn run_plan(\n    spec_file: &PathBuf,\n    target: Option<&PathBuf>,\n    json: bool,\n    all: bool,\n    out_plan: Option<&PathBuf>,\n) -> anyhow::Result<bool> {\n    let domain = detect_spec_domain(spec_file)?;\n    if domain != SpecDomain::PrjPcb {\n        if all {\n            anyhow::bail!("--all is only valid for .prjpcb-spec files");\n        }\n        return sync_v2::run_plan(spec_file, target, json, out_plan);\n    }\n    if out_plan.is_some() {\n        anyhow::bail!("saved sync-v2 plans are not yet defined for PrjPcb");\n    }\n'''
)

replace(
'''fn run_apply(\n    spec_file: &PathBuf,\n    target: Option<&PathBuf>,\n    output: Option<&PathBuf>,\n    _report_json: bool,\n    all: bool,\n) -> anyhow::Result<()> {\n    let domain = detect_spec_domain(spec_file)?;\n    if all && domain != SpecDomain::PrjPcb {\n        anyhow::bail!("--all is only valid for .prjpcb-spec files");\n    }\n\n    let source = std::fs::read_to_string(spec_file)\n''',
'''fn run_apply(\n    spec_file: Option<&PathBuf>,\n    saved_plan: Option<&PathBuf>,\n    target: Option<&PathBuf>,\n    output: Option<&PathBuf>,\n    report_json: bool,\n    all: bool,\n    force: bool,\n) -> anyhow::Result<()> {\n    if saved_plan.is_some() {\n        if all {\n            anyhow::bail!("--all cannot be combined with --plan");\n        }\n        return sync_v2::run_apply(spec_file, saved_plan, target, output, report_json, force);\n    }\n    let spec_file = spec_file.ok_or_else(|| anyhow::anyhow!("a spec file or --plan is required"))?;\n    let domain = detect_spec_domain(spec_file)?;\n    if domain != SpecDomain::PrjPcb {\n        if all {\n            anyhow::bail!("--all is only valid for .prjpcb-spec files");\n        }\n        return sync_v2::run_apply(Some(spec_file), None, target, output, report_json, force);\n    }\n\n    let source = std::fs::read_to_string(spec_file)\n'''
)

replace(
'''fn run_dump(document: &PathBuf, output: Option<&PathBuf>) -> anyhow::Result<()> {\n    // IntLib can contain both SchLib and PcbLib data, so it bypasses the\n''',
'''fn run_dump(\n    document: &PathBuf,\n    output: Option<&PathBuf>,\n    plan_only: bool,\n    out_plan: Option<&PathBuf>,\n    force: bool,\n) -> anyhow::Result<()> {\n    // IntLib can contain both SchLib and PcbLib data, so it bypasses the\n'''
)

replace(
'''    if ext == "intlib" {\n        return run_dump_intlib(document, output);\n    }\n\n    let domain = detect_document_domain(document)?;\n    let out_path = output\n''',
'''    if ext == "intlib" {\n        if plan_only || out_plan.is_some() || force {\n            anyhow::bail!("sync-v2 plan flags are not supported for IntLib");\n        }\n        return run_dump_intlib(document, output);\n    }\n\n    let domain = detect_document_domain(document)?;\n    if domain != SpecDomain::PrjPcb {\n        return sync_v2::run_dump(document, output, plan_only, out_plan, force);\n    }\n    if plan_only || out_plan.is_some() || force {\n        anyhow::bail!("sync-v2 plan flags are not yet defined for PrjPcb");\n    }\n    let out_path = output\n'''
)

MAIN.write_text(text)

# Remove the stale CI invocation of the deleted altium-format-ops package.
workflow = Path(".github/workflows/proptest-schlib.yml")
workflow_text = workflow.read_text()
stale = '''\n      - name: Run altium-format-ops SchLib proptests\n        run: cargo test -p altium-format-ops --test executor_proptest --features proptest --verbose\n'''
if stale not in workflow_text:
    raise SystemExit("stale altium-format-ops CI block not found")
workflow.write_text(workflow_text.replace(stale, "\n"))

# 12 - CLI Commands

## Location

`crates/altium-cli/src/main.rs` (extend existing CLI)

## New Commands

### `altium plan`

Show ECO without mutating the document.

```bash
altium plan my-parts.schlib-spec
altium plan my-parts.schlib-spec --target existing.SchLib
altium plan my-parts.schlib-spec --json
```

```rust
#[derive(clap::Args)]
struct PlanArgs {
    /// Path to the spec file (.schlib-spec or .pcblib-spec)
    spec_file: PathBuf,

    /// Existing document to reconcile against (optional)
    #[arg(long)]
    target: Option<PathBuf>,

    /// Output ECO as JSON
    #[arg(long)]
    json: bool,
}
```

**Behavior**:
1. Read spec file
2. Determine domain from extension (`.schlib-spec` or `.pcblib-spec`)
3. Load target document:
   - If `--target` given, use it
   - Otherwise, look for default output file (same base name + `.SchLib`/`.PcbLib`)
   - If neither exists, reconcile against empty document
4. Compile spec -> SpecModel
5. Reconcile -> ECO
6. Print ECO (text or JSON)
7. Exit code: 0 if no changes, 1 if changes exist (useful for CI)

### `altium apply`

Generate ECO and execute it.

```bash
altium apply my-parts.schlib-spec
altium apply my-parts.schlib-spec --target existing.SchLib
altium apply my-parts.schlib-spec --output custom.SchLib
altium apply my-parts.schlib-spec --report-json
```

```rust
#[derive(clap::Args)]
struct ApplyArgs {
    /// Path to the spec file
    spec_file: PathBuf,

    /// Existing document to update (optional)
    #[arg(long)]
    target: Option<PathBuf>,

    /// Output file path (overrides default)
    #[arg(long)]
    output: Option<PathBuf>,

    /// Print apply report as JSON
    #[arg(long)]
    report_json: bool,
}
```

**Behavior**:
1. Read spec file
2. Determine domain
3. Load or create document:
   - If `--target` given, load it
   - If default output file exists, load it
   - Otherwise, create empty document
4. Compile spec -> SpecModel
5. Reconcile -> ECO
6. Print ECO summary (unless `--report-json`)
7. Execute ECO -> apply ops to document
8. Save document to output path
9. Print report (text or JSON)

### `altium dump`

Reverse-generate a spec file from an existing document.

```bash
altium dump my-parts.SchLib
altium dump my-parts.PcbLib --output footprints.pcblib-spec
```

```rust
#[derive(clap::Args)]
struct DumpArgs {
    /// Path to the document (.SchLib or .PcbLib)
    document: PathBuf,

    /// Output spec file path (overrides default)
    #[arg(long)]
    output: Option<PathBuf>,
}
```

**Behavior**:
1. Load document
2. Determine domain from extension
3. Generate spec source
4. Write to output path (default: same base name + `.schlib-spec`/`.pcblib-spec`)

## Domain Detection

```rust
fn detect_domain(path: &Path) -> Result<SpecDomain> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("schlib-spec") => Ok(SpecDomain::SchLib),
        Some("pcblib-spec") => Ok(SpecDomain::PcbLib),
        _ => Err(anyhow!("unknown spec file extension: {}", path.display())),
    }
}

fn detect_document_domain(path: &Path) -> Result<SpecDomain> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext.to_ascii_lowercase().as_str() {
        "schlib" => Ok(SpecDomain::SchLib),
        "pcblib" => Ok(SpecDomain::PcbLib),
        _ => Err(anyhow!("unknown document extension: {}", path.display())),
    }
}
```

## Output File Resolution

For `apply`, the output file is determined by:
1. `--output` flag (explicit override)
2. `--target` flag (update in place)
3. Default: spec file base name + domain extension
   - `foo.schlib-spec` -> `foo.SchLib`
   - `foo.pcblib-spec` -> `foo.PcbLib`

## Error Handling

CLI uses `anyhow` for error handling. Spec errors are rendered with source
location and context:

```
error[E_CROSS_EDGE_REFERENCE]: pin '$p2' (on $body.right) is not on
  the same edge as pin '3' (on $body.left)
  --> my-parts.schlib-spec:12:5
   |
12 |     pin 3 { on: $body.left, after: $p2, gap: 60mil }
   |                                    ^^^ cross-edge reference
```

## Integration with Existing Commands

The new commands are top-level (not under a subcommand group). They coexist
with existing commands:

```
altium new ...        # existing
altium validate ...   # existing
altium save-as ...    # existing
altium cfb ...        # existing
altium ops ...        # existing (imperative ops)
altium plan ...       # NEW (declarative spec)
altium apply ...      # NEW (declarative spec)
altium dump ...       # NEW (reverse generation)
```

Note: `altium apply` (spec) and `altium ops apply` (ops) are distinct commands.
The spec `apply` operates on `.schlib-spec`/`.pcblib-spec` files. The ops
`apply` operates on `.ops` files with the imperative syntax.

## Test Strategy

- Integration test: plan with empty document
- Integration test: plan with existing document
- Integration test: apply creates new file
- Integration test: apply updates existing file
- Integration test: dump -> apply roundtrip
- JSON output validation
- Exit code verification (0 = no changes, 1 = changes)
- Error rendering for common mistakes

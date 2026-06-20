# CLI Reference

Reference for every spec-related `altium` subcommand: `plan`, `apply`, `dump`,
`format`, and `spec sync`. Each entry lists the synopsis, flags, domain
detection, default output paths, exit codes, and JSON modes, grounded in
`crates/altium-cli/src/main.rs`.

## Related pages

- [Apply and Plan](apply-and-plan.md) — the compile → reconcile → ECO → execute workflow
- [Dump](dump.md) — reverse-generating a spec from a document
- [Sync](sync.md) — spec-to-spec synchronization
- [Operations overview](../README.md)

## Spec domains and file extensions

Every spec command detects its domain from the file extension. The mapping in
`detect_spec_domain()` (main.rs) is:

| Extension       | `SpecDomain` | Produced document |
| --------------- | ------------ | ----------------- |
| `.schlib-spec`  | `SchLib`     | `.SchLib`         |
| `.pcblib-spec`  | `PcbLib`     | `.PcbLib`         |
| `.schdoc-spec`  | `SchDoc`     | `.SchDoc`         |
| `.pcbdoc-spec`  | `PcbDoc`     | `.PcbDoc`         |
| `.prjpcb-spec`  | `PrjPcb`     | `.PrjPcb`         |

Any other extension is a hard error:

```
unknown spec file extension .foo (supported: .schlib-spec, .pcblib-spec, .schdoc-spec, .pcbdoc-spec, .prjpcb-spec)
```

`detect_document_domain()` performs the inverse mapping for `altium dump`,
accepting `.schlib`, `.pcblib`, `.schdoc`, `.pcbdoc`, and `.prjpcb`
(case-insensitive).

### Default output paths

- **Spec → document** (`default_output_for_spec`): replaces the spec extension
  with the document extension, keeping the file stem. `mylib.schlib-spec` →
  `mylib.SchLib`.
- **Document → spec** (`default_spec_for_document`): the inverse.
  `mylib.SchLib` → `mylib.schlib-spec`.

---

## `altium plan`

Compile a spec, reconcile it against a target document, and print the resulting
Engineering Change Order (ECO) **without mutating anything**.

### Synopsis

```
altium plan <SPEC_FILE> [--target <PATH>] [--json] [--all]
```

### Arguments and flags

| Flag / arg     | Type        | Default | Description |
| -------------- | ----------- | ------- | ----------- |
| `spec_file`    | path (pos.) | —       | The spec file. Domain is detected from its extension. |
| `--target`     | path        | none    | Existing document to reconcile against. If omitted, the default output path for the spec is used when it exists; otherwise the spec is reconciled against an empty document. |
| `--json`       | bool        | `false` | Emit the ECO as pretty-printed JSON instead of the text report. |
| `--all`        | bool        | `false` | Process the root spec **and** every imported spec. Only valid for `.prjpcb-spec`; otherwise errors with `--all is only valid for .prjpcb-spec files`. |

### Target resolution

For each domain, `plan` resolves a target as follows (`plan_for_model`):

1. `resolved_target` = `--target` if given, else the default document path for
   the spec.
2. If `resolved_target` exists on disk, open it and run the document-aware
   reconciler (`reconcile_schlib`, `reconcile_pcblib`, `reconcile_schdoc`,
   `reconcile_pcbdoc`, `reconcile_prjpcb`).
3. If it does not exist, run the empty reconciler (`reconcile_*_empty`), where
   every spec entity becomes an `Add`.

### Output

Text mode prints the boxed ECO report (header, `SUMMARY`, `CHANGES`, `END OF
ECO`). JSON mode prints `serde_json::to_string_pretty` of the
`EngineeringChangeOrder`. With `--all`, each import is preceded by a
`--- <path> ---` banner in text mode.

### Exit codes

| Code | Meaning |
| ---- | ------- |
| `0`  | Success, **no changes** (no kind has `adds > 0` or `updates > 0`). |
| `1`  | Success, **changes exist**. This makes `plan` usable as a CI drift check. |
| non-zero (FAILURE) | An error occurred (read/parse/compile/reconcile). The message is printed to stderr as `Error: ...`. |

> Note: the "changes exist → exit 1" signal counts `adds` and `updates` only;
> a spec that is entirely `Unchanged` exits `0`.

---

## `altium apply`

Compile a spec and **write** the resulting Altium document to disk.

### Synopsis

```
altium apply <SPEC_FILE> [--target <PATH>] [--output <PATH>] [--report-json] [--all]
```

### Arguments and flags

| Flag / arg      | Type        | Default | Description |
| --------------- | ----------- | ------- | ----------- |
| `spec_file`     | path (pos.) | —       | The spec file; domain detected from extension. |
| `--target`      | path        | none    | Existing document to update. If omitted, the default output path is used when it exists; otherwise a new blank document is created (except PcbDoc — see below). |
| `--output`      | path        | none    | Output file path. Overrides the default. If omitted, writes to the default document path for the spec. |
| `--report-json` | bool        | `false` | Accepted but currently **not consumed** — the apply path takes the argument as `_report_json` and does not branch on it. No JSON report is emitted today. |
| `--all`         | bool        | `false` | Apply the root spec and all imported specs. `.prjpcb-spec` only. |

### Create-vs-update behavior (`apply_for_model`)

| Domain   | Target exists           | Target missing |
| -------- | ----------------------- | -------------- |
| SchLib   | open and update         | new blank AD26 SchLib; the placeholder `Component_1` is removed |
| PcbLib   | open and update         | new blank AD26 PcbLib |
| PrjPcb   | open and update         | new blank AD26 project |
| SchDoc   | open and update         | new blank AD26 SchDoc; uses `imported_components` to resolve pin positions |
| PcbDoc   | open and update         | **hard error**: `PcbDoc apply requires an existing target file: <path>` |

PcbDoc apply additionally instantiates footprint primitives: it discovers
sibling `.schdoc-spec` files to build a pad→net map, then for each imported
`.pcblib-spec` opens the corresponding `.PcbLib` and re-instantiates every
component's pads and graphics into board space (transformed by component
position/rotation). Existing component-owned primitives are removed first, so
re-running `apply` is idempotent.

### Output

On success, prints `Saved: <out_path>` per applied model. The `--all` flow
applies each import with its own `imported_components` and `import_paths` and no
explicit `--target`/`--output` override.

### Exit codes

| Code | Meaning |
| ---- | ------- |
| `0`  | Success. |
| non-zero | Any error (read/parse/compile/validate/apply/save). Printed as `Error: ...` to stderr. |

---

## `altium dump`

Reverse-generate a spec file from an existing Altium document.

### Synopsis

```
altium dump <DOCUMENT> [--output <PATH>]
```

### Arguments and flags

| Flag / arg  | Type        | Default | Description |
| ----------- | ----------- | ------- | ----------- |
| `document`  | path (pos.) | —       | The Altium document. Domain detected from extension. `.intlib` is handled specially. |
| `--output`  | path        | none    | Output spec path. If omitted, the default spec path for the document is used. For `.intlib`, `--output` is treated as a **directory** (or its parent if a file). |

### Domain handling

- `.schlib` → `dump_schlib`, `.pcblib` → `dump_pcblib`, `.schdoc` →
  `dump_schdoc`, `.pcbdoc` → `dump_pcbdoc`, `.prjpcb` → `dump_prjpcb`.
- `.intlib` bypasses the single-domain path (`run_dump_intlib`): it can emit
  **both** `<stem>.schlib-spec` and `<stem>.pcblib-spec`. If the IntLib contains
  neither, it errors with `<doc> contains no SchLib or PcbLib data`.

### Merge-on-write

Dump never blindly overwrites. `write_spec_merged` checks whether the output
already exists:

- Exists and parses → merge the fresh dump with the existing file (preserving
  comments and manual annotation IDs), printing `Merged: <doc> -> <out>`.
- Exists but has parse errors → warn and overwrite (`Warning: existing spec
  file has parse errors, overwriting without merge`).
- Does not exist → write fresh, printing `Dumped: <doc> -> <out>`.

### Exit codes

`0` on success, non-zero (`Error: ...`) on open/dump/write failure.

---

## `altium format`

Reformat spec files in place, to stdout, or check formatting for CI.

### Synopsis

```
altium format [FILES...] [--check] [--stdout]
```

### Arguments and flags

| Flag / arg | Type           | Default | Description |
| ---------- | -------------- | ------- | ----------- |
| `files`    | paths (pos.)   | empty   | Spec files to format. If **none** are given, reads source from stdin and writes formatted output to stdout. |
| `--check`  | bool           | `false` | Do not write. Print the path of each file that *would* change; exit `1` if any need formatting. |
| `--stdout` | bool           | `false` | Write formatted output to stdout instead of modifying files in place. |

Formatting uses `FormatConfig::default()` (4-space indent, up to 4 inline items,
100-column target). See [Dump](dump.md) and the language reference for the rules.

### Behavior matrix

| Invocation                     | Effect |
| ------------------------------ | ------ |
| `format` (no files)            | stdin → formatted stdout, exit `0`. |
| `format a.schlib-spec`         | rewrite in place if changed; prints `formatted <path>`. |
| `format --stdout a.schlib-spec`| print formatted source to stdout, file untouched. |
| `format --check a.schlib-spec` | print `<path>` for each file needing changes; exit `1` if any. |

### Exit codes

| Code | Meaning |
| ---- | ------- |
| `0`  | Success; in `--check` mode, all files already formatted. |
| `1`  | `--check` mode only: at least one file needs reformatting. |
| non-zero (FAILURE) | Read/parse error. Parse errors render with file name and source context. |

---

## `altium spec sync`

Spec-to-spec synchronization between a `.schdoc-spec` and a `.pcbdoc-spec`. This
is the only subcommand under the `spec` group.

### Synopsis

```
altium spec sync <SCHDOC_SPEC> <PCBDOC_SPEC> (--forward | --diff) [--dry-run] [--append]
```

### Arguments and flags

| Flag / arg     | Type        | Default | Description |
| -------------- | ----------- | ------- | ----------- |
| `schdoc_spec`  | path (pos.) | —       | The `.schdoc-spec` **source** file. |
| `pcbdoc_spec`  | path (pos.) | —       | The `.pcbdoc-spec` **target** file. |
| `--forward`    | bool        | `false` | Apply SchDoc changes to the PcbDoc (writes back the `.pcbdoc-spec`). Conflicts with `--diff`. |
| `--diff`       | bool        | `false` | Show changes only, never apply. Conflicts with `--forward`. |
| `--dry-run`    | bool        | `false` | Print the ECO report but do not write to disk (same effect as `--diff` for the write step). |
| `--append`     | bool        | `false` | Append mode: drop all `RemoveComponent`/`RemoveNet` changes so syncing multiple schematic sheets into one PcbDoc never clobbers previously synced sheets. |

You **must** specify `--forward` or `--diff`; otherwise: `specify --forward or
--diff`.

### Pipeline (`run_spec_sync`)

1. Read both spec files.
2. Compile + resolve both (`compile_and_resolve`); a non-matching domain errors
   (`<path> is not a valid .schdoc-spec file`, etc.).
3. Validate both (`validate_schdoc_spec`, `validate_pcbdoc_spec`); warnings go to
   stderr, errors abort.
4. Project both to `SyncSnapshot` (`project_schdoc_spec`, `project_pcbdoc_spec`).
5. Diff (`diff_snapshots`).
6. Filter with the fixed Phase-1 `SyncPolicy` (see below) via `filter_changes`.
7. In `--append`, retain only non-`Remove*` changes.
8. Print the ECO report (`render_eco_report`).
9. If `--diff` or `--dry-run`, stop here.
10. Otherwise apply to the in-memory model, rewrite the source text, format, and
    **atomically** replace the `.pcbdoc-spec` (temp file + rename), printing
    `Updated: <pcbdoc_spec>`.

### Phase-1 forward `SyncPolicy`

The CLI hardcodes this policy (no `Default` impl exists — see the crate README):

| Property             | Direction        |
| -------------------- | ---------------- |
| `comment`            | `Forward`        |
| `footprint`          | `Forward`        |
| `source_library`     | `Forward`        |
| `parameters`         | `Forward`        |
| `net_name`           | `Forward`        |
| `net_color`          | `None`           |
| `pin_net_assignment` | `None`           |
| `component_location` | `None`           |

The overall direction passed to `filter_changes` is `SyncDirection::Forward`.

### Exit codes

`0` on success (including no-op diffs). Non-zero (`Error: ...`) on
read/compile/validate/projection/filter/apply/write failure.

---

## Related read-only commands

These are not spec commands but are commonly paired with the workflow:

- `altium validate <doc>` — structural validation of a binary document.
- `altium info <doc> [--format text|json]` — object counts, nets, hierarchy.
- `altium query <doc> <AQL> [--format text|json|count] [--limit N]` — AQL query.
- `altium new {schdoc|schlib|pcblib|prjpcb} <out>` — blank AD26 documents.

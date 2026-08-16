from pathlib import Path

source = Path("crates/altium-spec-lang/src/source.rs")
text = source.read_text()
old = '''fn top_level_header(line: &str) -> Option<(String, String)> {
    const KINDS: &[&str] = &[
        "component",
        "footprint",
        "sheet",
        "board",
        "project",
        "placement",
    ];

    for kind in KINDS {
        if line == *kind
            || line.starts_with(&format!("{kind} "))
            || line.starts_with(&format!("{kind}{{"))
        {
            let rest = line[kind.len()..].trim_start();
            let key = parse_header_key(rest).unwrap_or_else(|| (*kind).to_string());
            return Some(((*kind).to_string(), key));
        }
    }
    None
}
'''
new = '''fn top_level_header(line: &str) -> Option<(String, String)> {
    let mut declaration = line.trim_start();
    if declaration.is_empty()
        || declaration.starts_with("//")
        || declaration.starts_with('#')
        || !line_contains_open_brace(declaration)
    {
        return None;
    }

    // A top-level declaration may bind its block: `body = component R { ... }`.
    // Strip only an identifier-like binding prefix; expressions containing `=`
    // remain untouched and conservatively fall back to whole-file coverage.
    if let Some((binding, rest)) = declaration.split_once('=') {
        let binding = binding.trim();
        if !binding.is_empty()
            && binding
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        {
            declaration = rest.trim_start();
        }
    }

    let kind_end = declaration
        .find(|ch: char| ch.is_whitespace() || ch == '{')
        .unwrap_or(declaration.len());
    if kind_end == 0 {
        return None;
    }
    let kind = &declaration[..kind_end];
    if !kind
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        return None;
    }
    let rest = declaration[kind_end..].trim_start();
    let key = parse_header_key(rest).unwrap_or_else(|| kind.to_string());
    Some((kind.to_string(), key))
}
'''
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise SystemExit("top_level_header block not found")

marker = '''    #[test]
    fn canonicalization_ignores_management_annotations() {
'''
insert = '''    #[test]
    fn scans_arbitrary_top_level_blocks_and_bound_declarations() {
        let source = "net N1 {\\n}\\nwire W {\\n}\\nbody = component U1 {\\n}\\n";
        let spec = LosslessSpec::parse(source).unwrap();
        assert_eq!(spec.resources().len(), 3);
        assert_eq!(spec.resources()[0].kind, "net");
        assert_eq!(spec.resources()[1].kind, "wire");
        assert_eq!(spec.resources()[2].kind, "component");
        assert_eq!(spec.resources()[2].key, "U1");
    }

'''
if "scans_arbitrary_top_level_blocks_and_bound_declarations" not in text:
    if marker not in text:
        raise SystemExit("source test insertion marker not found")
    text = text.replace(marker, insert + marker, 1)
source.write_text(text)

snapshot = Path("crates/altium-sync/src/snapshot.rs")
text = snapshot.read_text()
old = '''        // A future syntax construct that the structural scanner does not know
        // must still participate in drift detection. Treat the full file as one
        // conservative resource rather than silently dropping it.
        if resources.is_empty() && !canonical.trim().is_empty() {
            resources.push(SnapshotResource {
                address: "$document#0".to_string(),
                kind: "$document".to_string(),
                key: "$document".to_string(),
                fingerprint: Digest::text(&canonical),
                text: source.to_string(),
            });
        }
'''
new = '''        // Whole-file coverage is intentional, even when fine-grained resources
        // were discovered. Imports, bindings, scalar lets, comments with semantic
        // annotations, or future syntax must never evade three-way drift checks.
        // This conservative sentinel also makes simultaneous edits in different
        // resource kinds conflict instead of being silently merged.
        if !canonical.trim().is_empty() {
            resources.push(SnapshotResource {
                address: "$file#0".to_string(),
                kind: "$file".to_string(),
                key: "$file".to_string(),
                fingerprint: Digest::text(&canonical),
                text: source.to_string(),
            });
        }
'''
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise SystemExit("snapshot coverage block not found")

marker = '''    #[test]
    fn duplicate_natural_keys_get_distinct_addresses() {
'''
insert = '''    #[test]
    fn whole_file_resource_covers_non_block_source_changes() {
        let a = ArtifactSnapshot::from_source(
            ArtifactKind::SchLib,
            "import \\"a.schlib-spec\\"\\ncomponent R {\\n}\\n",
        )
        .unwrap();
        let b = ArtifactSnapshot::from_source(
            ArtifactKind::SchLib,
            "import \\"b.schlib-spec\\"\\ncomponent R {\\n}\\n",
        )
        .unwrap();
        let file_a = a.resource("$file#0").unwrap();
        let file_b = b.resource("$file#0").unwrap();
        assert_ne!(file_a.fingerprint, file_b.fingerprint);
    }

'''
if "whole_file_resource_covers_non_block_source_changes" not in text:
    if marker not in text:
        raise SystemExit("snapshot test insertion marker not found")
    text = text.replace(marker, insert + marker, 1)
snapshot.write_text(text)

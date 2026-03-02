//! Import resolver for the Altium spec language.
//!
//! Loads all files referenced by `import` declarations, detects cycles,
//! validates cross-domain rules, and builds a [`ResolvedSpec`] ready for
//! the compiler.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;

use crate::diagnostic::Span;
use crate::ast::{SpecFile, SpecItem};
use crate::eval::{SpecError, SpecErrorCode};
use crate::parser::parse_spec;

// ── Public types ──────────────────────────────────────────────────────────────

/// Resolved set of spec files with all imports flattened.
#[derive(Debug)]
pub struct ResolvedSpec {
    /// The root file's AST.
    pub root: SpecFile,
    /// Named imports: alias → (file_path, parsed SpecFile)
    pub named_imports: IndexMap<String, (PathBuf, SpecFile)>,
    /// Bare imports in topological order (leaves first).
    pub bare_imports: Vec<(PathBuf, SpecFile)>,
}

/// Resolve all imports starting from the given root file.
///
/// Parses all referenced files, detects cycles, validates cross-domain rules,
/// checks alias uniqueness and bare-import collisions, then builds a
/// [`ResolvedSpec`].
pub fn resolve_imports(
    root_path: &Path,
    root_ast: SpecFile,
) -> Result<ResolvedSpec, SpecError> {
    let root_path = root_path
        .canonicalize()
        .map_err(|e| SpecError::no_span(SpecErrorCode::FileNotFound, format!("{e}")))?;

    // Cache: canonical path -> parsed SpecFile
    let mut cache: HashMap<PathBuf, SpecFile> = HashMap::new();
    cache.insert(root_path.clone(), root_ast.clone());

    // Build dependency graph by recursively parsing imports.
    // graph[file] = list of imported file paths in declaration order.
    let mut graph: IndexMap<PathBuf, Vec<ImportEdge>> = IndexMap::new();
    collect_imports(&root_path, &root_ast, &mut cache, &mut graph)?;

    // Topological sort with cycle detection.
    let topo_order = topo_sort(&root_path, &graph)?;

    // Validate cross-domain rules.
    for (from_path, edges) in &graph {
        for edge in edges {
            validate_cross_domain(from_path, &edge.to_path, edge.span)?;
        }
    }

    // Validate alias uniqueness within each file.
    for (file_path, edges) in &graph {
        let mut seen_aliases: HashMap<String, Span> = HashMap::new();
        for edge in edges {
            if let Some(ref alias) = edge.alias {
                if let Some(prev_span) = seen_aliases.get(alias) {
                    return Err(SpecError::new(
                        SpecErrorCode::DuplicateImportAlias,
                        format!(
                            "duplicate import alias '{}' in {} (first defined at {}..{})",
                            alias,
                            file_path.display(),
                            prev_span.start,
                            prev_span.end,
                        ),
                        Some(edge.span),
                    ));
                }
                seen_aliases.insert(alias.clone(), edge.span);
            }
        }
    }

    // Build named_imports and bare_imports from the root file's imports only.
    // Transitive imports are already parsed and cached.
    let root_edges = graph.get(&root_path).cloned().unwrap_or_default();

    let mut named_imports: IndexMap<String, (PathBuf, SpecFile)> = IndexMap::new();
    let mut bare_import_paths: Vec<PathBuf> = Vec::new();

    for edge in &root_edges {
        let spec_file = cache
            .get(&edge.to_path)
            .cloned()
            .unwrap_or_else(|| SpecFile { items: vec![] });
        if let Some(ref alias) = edge.alias {
            named_imports.insert(alias.clone(), (edge.to_path.clone(), spec_file));
        } else {
            bare_import_paths.push(edge.to_path.clone());
        }
    }

    // Build bare_imports in topological order (leaves first, skip root and named).
    // Note: bare-import entity collisions are NOT checked because each import
    // targets a different output file (reference semantics).
    let named_paths: std::collections::HashSet<PathBuf> =
        named_imports.values().map(|(p, _)| p.clone()).collect();
    let bare_imports: Vec<(PathBuf, SpecFile)> = topo_order
        .into_iter()
        .filter(|p| *p != root_path && !named_paths.contains(p) && bare_import_paths.contains(p))
        .map(|p| {
            let file = cache
                .get(&p)
                .cloned()
                .unwrap_or_else(|| SpecFile { items: vec![] });
            (p, file)
        })
        .collect();

    Ok(ResolvedSpec {
        root: root_ast,
        named_imports,
        bare_imports,
    })
}

// ── Import edge (one import declaration) ─────────────────────────────────────

#[derive(Debug, Clone)]
struct ImportEdge {
    to_path: PathBuf,
    /// Present if `import "..." as alias`; absent for bare imports.
    alias: Option<String>,
    /// Source span of the import path string (for error messages).
    span: Span,
}

// ── Recursive import collection ───────────────────────────────────────────────

/// Recursively parse all imported files and populate `cache` and `graph`.
fn collect_imports(
    file_path: &Path,
    file_ast: &SpecFile,
    cache: &mut HashMap<PathBuf, SpecFile>,
    graph: &mut IndexMap<PathBuf, Vec<ImportEdge>>,
) -> Result<(), SpecError> {
    if graph.contains_key(file_path) {
        // Already processed this file's imports.
        return Ok(());
    }

    let mut edges: Vec<ImportEdge> = Vec::new();

    for item in &file_ast.items {
        let import_decl = match &item.node {
            SpecItem::Import(d) => d,
            _ => continue,
        };

        let import_path_str = &import_decl.path.node;
        let import_span = import_decl.path.span;

        // Absolute paths are forbidden.
        if Path::new(import_path_str.as_str()).is_absolute() {
            return Err(SpecError::new(
                SpecErrorCode::FileNotFound,
                format!("absolute import paths are not allowed: '{import_path_str}'"),
                Some(import_span),
            ));
        }

        let resolved = resolve_import_path(file_path, import_path_str);
        let canonical = resolved.canonicalize().map_err(|_| {
            SpecError::new(
                SpecErrorCode::FileNotFound,
                format!(
                    "imported file not found: '{}' (from '{}')",
                    resolved.display(),
                    file_path.display()
                ),
                Some(import_span),
            )
        })?;

        let alias = import_decl.alias.as_ref().map(|a| a.node.clone());
        edges.push(ImportEdge {
            to_path: canonical.clone(),
            alias,
            span: import_span,
        });

        // Parse if not yet cached.
        if !cache.contains_key(&canonical) {
            let source = std::fs::read_to_string(&canonical).map_err(|e| {
                SpecError::new(
                    SpecErrorCode::FileNotFound,
                    format!("failed to read '{}': {e}", canonical.display()),
                    Some(import_span),
                )
            })?;
            let ast = parse_spec(&source).map_err(|e| {
                SpecError::no_span(
                    SpecErrorCode::ParseError,
                    format!("parse error in '{}': {e}", canonical.display()),
                )
            })?;
            cache.insert(canonical.clone(), ast.clone());
            // Recurse.
            collect_imports(&canonical, &ast, cache, graph)?;
        }
    }

    graph.insert(file_path.to_path_buf(), edges);
    Ok(())
}

// ── Path resolution ───────────────────────────────────────────────────────────

fn resolve_import_path(importing_file: &Path, import_path: &str) -> PathBuf {
    importing_file
        .parent()
        .unwrap_or(Path::new("."))
        .join(import_path)
}

// ── Topological sort with cycle detection ─────────────────────────────────────

#[derive(Clone, PartialEq)]
enum Color {
    White,
    Gray,
    Black,
}

fn topo_sort(
    root: &Path,
    graph: &IndexMap<PathBuf, Vec<ImportEdge>>,
) -> Result<Vec<PathBuf>, SpecError> {
    let mut colors: HashMap<PathBuf, Color> = HashMap::new();
    let mut order: Vec<PathBuf> = Vec::new();
    let mut stack: Vec<PathBuf> = Vec::new();

    // Start DFS from root; then visit any other nodes not yet reached.
    dfs(root, graph, &mut colors, &mut order, &mut stack)?;
    for node in graph.keys() {
        if colors.get(node).map_or(true, |c| *c == Color::White) {
            dfs(node, graph, &mut colors, &mut order, &mut stack)?;
        }
    }

    Ok(order)
}

fn dfs(
    node: &Path,
    graph: &IndexMap<PathBuf, Vec<ImportEdge>>,
    colors: &mut HashMap<PathBuf, Color>,
    order: &mut Vec<PathBuf>,
    stack: &mut Vec<PathBuf>,
) -> Result<(), SpecError> {
    let color = colors.get(node).cloned().unwrap_or(Color::White);
    if color == Color::Black {
        return Ok(());
    }
    if color == Color::Gray {
        // Found a cycle — build the cycle path for the error message.
        let cycle_start = stack.iter().position(|p| p == node).unwrap_or(0);
        let mut cycle: Vec<PathBuf> = stack[cycle_start..].to_vec();
        cycle.push(node.to_path_buf());
        let path_str = cycle
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        return Err(SpecError::no_span(
            SpecErrorCode::CircularImport,
            format!("circular import detected: {path_str}"),
        ));
    }

    colors.insert(node.to_path_buf(), Color::Gray);
    stack.push(node.to_path_buf());

    if let Some(edges) = graph.get(node) {
        for edge in edges {
            dfs(&edge.to_path, graph, colors, order, stack)?;
        }
    }

    stack.pop();
    colors.insert(node.to_path_buf(), Color::Black);
    order.push(node.to_path_buf());
    Ok(())
}

// ── Cross-domain validation ───────────────────────────────────────────────────

/// Determine the spec domain from a file path's compound extension.
///
/// `foo.schlib-spec` → SchLib, `bar.pcblib-spec` → PcbLib.
/// `Path::extension()` returns only the last `.`-component, so for
/// `foo.schlib-spec` it returns `"spec"`.  We therefore match on the
/// full file name suffix.
fn file_domain(path: &Path) -> FileDomain {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if name.ends_with(".schlib-spec") {
        FileDomain::SchLib
    } else if name.ends_with(".pcblib-spec") {
        FileDomain::PcbLib
    } else if name.ends_with(".prjpcb-spec") {
        FileDomain::PrjPcb
    } else if name.ends_with(".schdoc-spec") {
        FileDomain::SchDoc
    } else {
        FileDomain::Unknown
    }
}

#[derive(Debug, PartialEq)]
enum FileDomain {
    SchLib,
    PcbLib,
    PrjPcb,
    SchDoc,
    Unknown,
}

fn validate_cross_domain(
    from_path: &Path,
    to_path: &Path,
    span: Span,
) -> Result<(), SpecError> {
    let from_domain = file_domain(from_path);
    let to_domain = file_domain(to_path);

    let forbidden = matches!(
        (&from_domain, &to_domain),
        (FileDomain::PcbLib, FileDomain::SchLib)
            | (FileDomain::PrjPcb, FileDomain::PrjPcb)
            | (FileDomain::SchDoc, FileDomain::PcbLib)
            | (FileDomain::SchDoc, FileDomain::SchDoc)
    );
    if forbidden {
        return Err(SpecError::new(
            SpecErrorCode::CrossDomainViolation,
            format!(
                "{:?} spec '{}' cannot import {:?} spec '{}'",
                from_domain,
                from_path.display(),
                to_domain,
                to_path.display()
            ),
            Some(span),
        ));
    }
    // Allowed combinations:
    // SchLib -> SchLib: bare or named ✓
    // SchLib -> PcbLib: named only (bare validation at compile time) ✓
    // PcbLib -> PcbLib: bare or named ✓
    // PrjPcb -> SchLib: reference import ✓
    // PrjPcb -> PcbLib: reference import ✓
    // SchDoc -> SchLib: named only (for $alias.ComponentName references) ✓
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Span, Spanned};
    use crate::ast::{ImportDecl, SpecFile, SpecItem};

    fn zero_span() -> Span {
        Span { start: 0, end: 1 }
    }

    fn spanned<T>(node: T) -> Spanned<T> {
        Spanned { node, span: zero_span() }
    }

    fn import_decl(path: &str, alias: Option<&str>) -> ImportDecl {
        ImportDecl {
            path: spanned(path.to_string()),
            alias: alias.map(|a| spanned(a.to_string())),
        }
    }

    fn spec_with_imports(imports: Vec<ImportDecl>) -> SpecFile {
        SpecFile {
            items: imports
                .into_iter()
                .map(|d| spanned(SpecItem::Import(d)))
                .collect(),
        }
    }

    fn empty_spec() -> SpecFile {
        SpecFile { items: vec![] }
    }

    // ── Test: empty root (no imports) ─────────────────────────────────────────

    #[test]
    fn no_imports_returns_root() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root_path = tmp.path().join("root.schlib-spec");
        std::fs::write(&root_path, "").unwrap();

        let ast = empty_spec();
        let resolved = resolve_imports(&root_path, ast.clone()).unwrap();

        assert!(resolved.named_imports.is_empty());
        assert!(resolved.bare_imports.is_empty());
    }

    // ── Test: named import ────────────────────────────────────────────────────

    #[test]
    fn named_import_stored_under_alias() {
        let tmp = tempfile::TempDir::new().unwrap();

        let fp_path = tmp.path().join("pads.pcblib-spec");
        std::fs::write(&fp_path, "").unwrap();

        let root_path = tmp.path().join("root.schlib-spec");
        std::fs::write(&root_path, "").unwrap();

        let root_ast = spec_with_imports(vec![import_decl("pads.pcblib-spec", Some("fp"))]);
        let resolved = resolve_imports(&root_path, root_ast).unwrap();

        assert_eq!(resolved.named_imports.len(), 1);
        assert!(resolved.named_imports.contains_key("fp"));
        assert!(resolved.bare_imports.is_empty());
    }

    // ── Test: bare import ─────────────────────────────────────────────────────

    #[test]
    fn bare_import_appears_in_bare_imports() {
        let tmp = tempfile::TempDir::new().unwrap();

        let passives_path = tmp.path().join("passives.schlib-spec");
        std::fs::write(&passives_path, "").unwrap();

        let root_path = tmp.path().join("root.schlib-spec");
        std::fs::write(&root_path, "").unwrap();

        let root_ast = spec_with_imports(vec![import_decl("passives.schlib-spec", None)]);
        let resolved = resolve_imports(&root_path, root_ast).unwrap();

        assert!(resolved.named_imports.is_empty());
        assert_eq!(resolved.bare_imports.len(), 1);
    }

    // ── Test: cycle detection ─────────────────────────────────────────────────

    #[test]
    fn cycle_detection_error() {
        let tmp = tempfile::TempDir::new().unwrap();

        // a.schlib-spec imports b.schlib-spec
        // b.schlib-spec imports a.schlib-spec  → cycle
        let a_path = tmp.path().join("a.schlib-spec");
        let b_path = tmp.path().join("b.schlib-spec");

        std::fs::write(&a_path, r#"import "b.schlib-spec""#).unwrap();
        std::fs::write(&b_path, r#"import "a.schlib-spec""#).unwrap();

        let a_source = std::fs::read_to_string(&a_path).unwrap();
        let a_ast = parse_spec(&a_source).unwrap();

        let err = resolve_imports(&a_path, a_ast).unwrap_err();
        assert_eq!(err.code, SpecErrorCode::CircularImport);
        assert!(err.message.contains("circular import"), "got: {}", err.message);
    }

    // ── Test: duplicate alias ─────────────────────────────────────────────────

    #[test]
    fn duplicate_alias_error() {
        let tmp = tempfile::TempDir::new().unwrap();

        let f1_path = tmp.path().join("f1.pcblib-spec");
        let f2_path = tmp.path().join("f2.pcblib-spec");
        std::fs::write(&f1_path, "").unwrap();
        std::fs::write(&f2_path, "").unwrap();

        let root_path = tmp.path().join("root.schlib-spec");
        std::fs::write(&root_path, "").unwrap();

        let root_ast = spec_with_imports(vec![
            import_decl("f1.pcblib-spec", Some("fp")),
            import_decl("f2.pcblib-spec", Some("fp")), // duplicate alias
        ]);

        let err = resolve_imports(&root_path, root_ast).unwrap_err();
        assert_eq!(err.code, SpecErrorCode::DuplicateImportAlias);
        assert!(err.message.contains("fp"), "got: {}", err.message);
    }

    // ── Test: cross-domain violation ──────────────────────────────────────────

    #[test]
    fn pcblib_cannot_import_schlib() {
        let tmp = tempfile::TempDir::new().unwrap();

        let sch_path = tmp.path().join("comps.schlib-spec");
        std::fs::write(&sch_path, "").unwrap();

        let root_path = tmp.path().join("root.pcblib-spec");
        std::fs::write(&root_path, "").unwrap();

        let root_ast = spec_with_imports(vec![import_decl("comps.schlib-spec", None)]);
        let err = resolve_imports(&root_path, root_ast).unwrap_err();
        assert_eq!(err.code, SpecErrorCode::CrossDomainViolation);
    }

    // ── Test: bare import collision is now allowed (reference semantics) ─────

    #[test]
    fn bare_import_collision_allowed() {
        let tmp = tempfile::TempDir::new().unwrap();

        // Both files define component "R" — no longer an error because each
        // import targets a different output file.
        let f1_path = tmp.path().join("f1.schlib-spec");
        let f2_path = tmp.path().join("f2.schlib-spec");
        std::fs::write(&f1_path, "component R {}").unwrap();
        std::fs::write(&f2_path, "component R {}").unwrap();

        let root_path = tmp.path().join("root.schlib-spec");
        std::fs::write(&root_path, "").unwrap();

        let root_ast = spec_with_imports(vec![
            import_decl("f1.schlib-spec", None),
            import_decl("f2.schlib-spec", None),
        ]);

        let resolved = resolve_imports(&root_path, root_ast).unwrap();
        assert_eq!(resolved.bare_imports.len(), 2);
    }

    // ── Test: PrjPcb cross-domain rules ───────────────────────────────────────

    #[test]
    fn prjpcb_can_import_schlib() {
        let tmp = tempfile::TempDir::new().unwrap();

        let sch_path = tmp.path().join("comps.schlib-spec");
        std::fs::write(&sch_path, "").unwrap();

        let root_path = tmp.path().join("root.prjpcb-spec");
        std::fs::write(&root_path, "").unwrap();

        let root_ast = spec_with_imports(vec![import_decl("comps.schlib-spec", None)]);
        let resolved = resolve_imports(&root_path, root_ast).unwrap();
        assert_eq!(resolved.bare_imports.len(), 1);
    }

    #[test]
    fn prjpcb_can_import_pcblib() {
        let tmp = tempfile::TempDir::new().unwrap();

        let pcb_path = tmp.path().join("pads.pcblib-spec");
        std::fs::write(&pcb_path, "").unwrap();

        let root_path = tmp.path().join("root.prjpcb-spec");
        std::fs::write(&root_path, "").unwrap();

        let root_ast = spec_with_imports(vec![import_decl("pads.pcblib-spec", None)]);
        let resolved = resolve_imports(&root_path, root_ast).unwrap();
        assert_eq!(resolved.bare_imports.len(), 1);
    }

    #[test]
    fn prjpcb_cannot_import_prjpcb() {
        let tmp = tempfile::TempDir::new().unwrap();

        let nested_path = tmp.path().join("other.prjpcb-spec");
        std::fs::write(&nested_path, "").unwrap();

        let root_path = tmp.path().join("root.prjpcb-spec");
        std::fs::write(&root_path, "").unwrap();

        let root_ast = spec_with_imports(vec![import_decl("other.prjpcb-spec", None)]);
        let err = resolve_imports(&root_path, root_ast).unwrap_err();
        assert_eq!(err.code, SpecErrorCode::CrossDomainViolation);
    }

    // ── Test: file not found ──────────────────────────────────────────────────

    #[test]
    fn file_not_found_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root_path = tmp.path().join("root.schlib-spec");
        std::fs::write(&root_path, "").unwrap();

        let root_ast = spec_with_imports(vec![import_decl("nonexistent.schlib-spec", None)]);
        let err = resolve_imports(&root_path, root_ast).unwrap_err();
        assert_eq!(err.code, SpecErrorCode::FileNotFound);
    }
}

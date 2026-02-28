# 04 - Import Resolver

## Location

`crates/altium-format-ops/src/spec/import.rs`

## Purpose

Resolve `import` declarations, load referenced files, detect cycles, and
build a merged namespace for the compiler.

## Public API

```rust
/// Resolved set of spec files with all imports flattened.
pub struct ResolvedSpec {
    /// The root file's AST with imports resolved.
    pub root: SpecFile,
    /// Named imports: alias -> (file_path, parsed SpecFile)
    pub named_imports: IndexMap<String, (PathBuf, SpecFile)>,
    /// Bare imports: merged entity declarations (components/footprints)
    pub bare_imports: Vec<(PathBuf, SpecFile)>,
}

/// Resolve all imports starting from the given file path.
pub fn resolve_imports(
    root_path: &Path,
    root_ast: SpecFile,
) -> Result<ResolvedSpec, SpecError>
```

## Algorithm

### Phase 1: Topological Sort

1. Start from root file
2. For each `import` declaration, resolve the path relative to the importing
   file's directory
3. Parse the imported file (recursively)
4. Build a dependency graph (file -> set of imported files)
5. Detect cycles via DFS with coloring (white/gray/black)
6. Sort topologically (leaves first)

### Phase 2: Validation

After all files are parsed:

1. **Alias uniqueness**: No two `import "..." as X` in the same file may use
   the same alias `X`. Error: `E_DUPLICATE_IMPORT_ALIAS`.

2. **Bare import collision**: If two bare imports define entities with the same
   identity key (same `component` name or same `footprint` name), this is a
   hard error. Error: `E_DUPLICATE_ENTITY`.

3. **Cross-domain rules**:
   - `.schlib-spec` can import `.schlib-spec` (bare or named)
   - `.schlib-spec` can import `.pcblib-spec` (named only)
   - `.pcblib-spec` can import `.pcblib-spec` (bare or named)
   - `.pcblib-spec` cannot import `.schlib-spec` (error)

### Phase 3: Namespace Construction

For named imports (`import "file.pcblib-spec" as fp`):
- Store the parsed file under the alias `fp`
- Entity references like `$fp.DIP8` resolve to the footprint `DIP8` in that file

For bare imports (`import "passives.schlib-spec"`):
- Merge the imported file's entity declarations into the root file's declaration
  list
- Let bindings from imported files are NOT merged (spec-lang.md §6.3)

## Cycle Detection

```rust
enum Color { White, Gray, Black }

fn detect_cycles(
    graph: &HashMap<PathBuf, Vec<PathBuf>>,
) -> Result<Vec<PathBuf>, SpecError> {
    let mut colors = HashMap::new();
    let mut order = Vec::new();
    let mut stack = Vec::new(); // for cycle path reporting

    for node in graph.keys() {
        if colors.get(node) == Some(&Color::Black) { continue; }
        dfs(node, graph, &mut colors, &mut order, &mut stack)?;
    }
    Ok(order)
}
```

On cycle detection, report the full path:
```
error[E_CIRCULAR_IMPORT]: circular import detected
  a.schlib-spec -> b.schlib-spec -> a.schlib-spec
```

## File Caching

Each file is parsed at most once. A `HashMap<PathBuf, SpecFile>` caches parsed
results. If file A and file B both import file C, C is parsed once and shared.

## Path Resolution

Import paths are resolved relative to the importing file's directory:

```rust
fn resolve_import_path(importing_file: &Path, import_path: &str) -> PathBuf {
    importing_file.parent().unwrap().join(import_path)
}
```

Absolute paths are an error. Only relative paths are allowed.

## Error Types

```rust
pub enum ImportError {
    CircularImport { cycle: Vec<PathBuf> },
    DuplicateAlias { alias: String, file: PathBuf, span: Span },
    DuplicateEntity { name: String, file_a: PathBuf, file_b: PathBuf },
    CrossDomainViolation { from: PathBuf, to: PathBuf, message: String },
    FileNotFound { path: PathBuf, referenced_from: PathBuf, span: Span },
    ParseError { path: PathBuf, error: ParseError },
}
```

## Test Strategy

- Two-file import (named and bare)
- Three-file chain (A imports B imports C)
- Diamond (A imports B and C, both import D)
- Cycle detection (A -> B -> A)
- Bare import collision (two files define component "R")
- Cross-domain validation (pcblib importing schlib = error)
- Alias uniqueness (two imports with same alias = error)
- File not found
- Named import namespace access (`$fp.DIP8`)

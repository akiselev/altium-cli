# Code Walkthrough: Opening a Workspace, Library, and Component

This document traces the exact Rust code path from launching the GUI, through
opening a workspace, opening a symbol library (.sym), and finally opening a
specific component within that library.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Phase 1: Launching the GUI](#2-phase-1-launching-the-gui)
3. [Phase 2: Opening a Workspace](#3-phase-2-opening-a-workspace)
4. [Phase 3: Opening a Symbol Library (.sym)](#4-phase-3-opening-a-symbol-library)
5. [Phase 4: Opening a Component](#5-phase-4-opening-a-component)
6. [Phase 5: Rendering the Component Tab](#6-phase-5-rendering-the-component-tab)
7. [The Intent/Command Pipeline in Detail](#7-the-intentcommand-pipeline-in-detail)
8. [Refactoring Opportunities](#8-refactoring-opportunities)

---

## 1. Architecture Overview

The shell follows a **unidirectional data flow** architecture inspired by
Elm/Redux, built on top of eframe/egui:

```
User Action (click, shortcut, IPC)
        |
        v
    Intent (enum)              -- "what the user wants"
        |
        v
    resolve_intent()           -- validates preconditions, produces Commands
        |
        v
    CommandTransaction         -- ordered list of Commands + undo policy
        |
        v
    apply_command() loop       -- mutates ShellApp state, returns inverse Commands
        |
        v
    DomainEvents               -- side-channel signals (selection changed, etc.)
        |
        v
    Undo stack (optional)      -- inverse transaction pushed for Ctrl+Z
```

Key types and where they live:

| Type | File | Role |
|------|------|------|
| `ShellApp` | `app/mod.rs` | God object: all UI state, model, jobs, canvases |
| `WorkbenchModel` | `workbench.rs` | Document registry, tabs, selection |
| `Document` / `DocumentKind` | `workbench.rs` | Individual open document (board, spec, schlib, etc.) |
| `Intent` / `Command` | `pipeline.rs` | Intent enum + Command enum (the "what" vs "how") |
| `SessionSnapshot` | `session.rs` | Serializable app state for persist/restore |
| `JobManager` | `jobs.rs` | Background thread pool for parsing files |
| `GraphHost` | `graph_host.rs` | Wrapper around `DesignWorkspace` graph model |
| `ProjectGraph` | `project_graph.rs` | .wrk/.PrjPcb project file model |
| `TabProviderRegistry` | `app/tabs.rs` | Factory registry: document kind -> tab renderer |

---

## 2. Phase 1: Launching the GUI

### Entry point: `main.rs:86`

```
main() -> Cli::parse() -> run_gui()
```

The CLI uses clap to parse arguments. With no subcommand (or `gui`), it calls
`run_gui()`.

### `run_gui()` at `main.rs:146`

```rust
fn run_gui(board_path, socket_path, session_path, restore_enabled) {
    let initial_ir = load_initial_ir(board_path.as_ref())?;  // parse PcbDoc if given
    let server = start_server(socket_path)?;                   // IPC singleton
    // ...
    efame::run_native(title, options, Box::new(move |cc| {
        Ok(Box::new(ShellApp::new(cc, board_path, initial_ir, ipc_rx, ...)))
    }))
}
```

This creates the eframe window and constructs `ShellApp` inside the creation
callback.

### `ShellApp::new()` at `app/mod.rs:303`

The constructor initializes ~40 fields:

```rust
pub fn new(cc, board_path, initial_ir, ipc_rx, session_store, restore_mode) -> Self {
    let commands = CommandRegistry::new_m1();           // register all commands
    let shortcut_bindings = default_shortcuts(&commands);
    let mut app = Self {
        model: WorkbenchModel::new(board_path, initial_ir),  // creates document model
        tab_registry: TabProviderRegistry::new_m1(),         // register tab renderers
        jobs: JobManager::new(),                              // background job system
        // ... 35+ more fields ...
    };
    app.restore_mode(restore_mode);  // try to load previous session
    app
}
```

**`WorkbenchModel::new()`** at `workbench.rs:206`: If a board path + IR were
given on the command line, it immediately opens a board document tab and a
blank spec document.

**`TabProviderRegistry::new_m1()`** at `app/tabs.rs:31`: Registers factory
functions mapping `DOCUMENT_KIND_*` string constants to tab renderer structs.
For example:
- `"document.schlib_gallery"` -> `SchLibGalleryTabRenderer`
- `"document.schlib_component"` -> `SchLibComponentTabRenderer`

### The frame loop: `update()` at `app/mod.rs:4632`

Every frame, eframe calls `ShellApp::update()`. This is the heartbeat:

```rust
fn update(&mut self, ctx, _frame) {
    self.process_ipc();                    // check IPC socket for commands
    self.process_job_events();             // poll background job results
    self.scan_watched_files();             // hot-reload changed spec files
    self.handle_shortcuts(ctx);            // keyboard shortcut -> Intent
    self.process_queue(ctx);               // drain intent queue

    // Render UI panels:
    self.render_title_menu_bar(ctx);
    self.render_status_bar(ctx);
    self.render_activity_bar(ctx);
    self.render_sidebar(ctx);              // Explorer, Agents, Review panels
    self.render_secondary_sidebar(ctx);    // Inspector
    // Central area: editor dock with document tabs
    show_central_panel(ctx, &theme, |ui| { ... });

    self.show_palette_window(ctx);         // command palette overlay
    self.maybe_autosave_session();         // debounced session save
}
```

At this point the app is running with an empty workspace (or a restored
session). The user sees the Explorer sidebar and an empty editor area.

---

## 3. Phase 2: Opening a Workspace

The user opens a workspace by either:
- Clicking in the Explorer sidebar file tree
- Using the command palette: "Workspace: Open Folder"
- Opening a `.wrk` or `.PrjPcb` file

### Path A: "Workspace: Open Folder" command

**Step 1: User triggers command** (keyboard shortcut or palette selection)

The shortcut or palette click calls:
```rust
// app/mod.rs:1189
pub(crate) fn queue_intent(&mut self, intent: Intent) {
    self.intent_queue.push_back(intent);
}
```

For "workspace.open", the intent is:
```rust
Intent::Workspace(WorkspaceIntent::Open { root: None })
```

**Step 2: Intent processing** in `process_queue()` at `app/mod.rs:2585`:

```rust
fn process_queue(&mut self, ctx) {
    while let Some(intent) = self.intent_queue.pop_front() {
        self.process_intent(intent, ctx);
    }
}
```

**Step 3: `process_intent()`** at `app/mod.rs:1332`:

```rust
fn process_intent(&mut self, intent, ctx) {
    self.telemetry.intent_received(&intent);
    match resolve_intent(intent, self.resolve_context()) {
        ResolveResult::Accepted { transaction } => {
            self.apply_transaction(transaction, ctx, true);
        }
        ResolveResult::Rejected { code, message } => {
            self.model.problems.push(message);
        }
    }
}
```

**Step 4: `resolve_intent()`** in `pipeline.rs:590`:

This is a pure function (no side effects). It validates preconditions and
converts the Intent into a list of Commands:

```rust
Intent::Workspace(WorkspaceIntent::Open { root }) => {
    vec![Command::WorkspaceOpen { root: root.clone() }]
}
```

The result is wrapped in a `CommandTransaction`.

**Step 5: `apply_command()`** at `app/mod.rs:1556`:

```rust
Command::WorkspaceOpen { root } => {
    let root = root
        .or_else(|| self.model.workspace_root.clone())
        .or_else(|| std::env::current_dir().ok());
    self.model.set_workspace_root(root.clone());
    self.model.set_active_graph(GraphHost::stub_from_path(&root));
    // ...
}
```

This sets the workspace root directory and creates a stub graph host. Now the
Explorer sidebar will show the directory tree.

### Path B: Opening a .wrk project file

When the user opens a `.wrk` file (via Explorer sidebar click or drag-and-drop):

**Step 1:** `open_document_path()` at `app/mod.rs:2341` dispatches by extension:

```rust
"wrk" | "prjpcb" => {
    self.queue_intent(Intent::Workspace(WorkspaceIntent::OpenProject {
        path: Some(path)
    }));
}
```

**Step 2:** This resolves to `Command::WorkspaceOpenProject`:

```rust
Command::WorkspaceOpenProject { path } => {
    let path = path.or_else(|| self.find_project_in_workspace_root());
    self.submit_job(JobPayload::ParseProject { project_path });
}
```

**Step 3:** The job runs on a **background thread** (`jobs.rs:241`):

```rust
JobPayload::ParseProject { project_path } => {
    let delta = build_project_graph(&project_path)?;
    tx.send(JobEvent::Artifact(id, JobArtifact::ProjectGraphDelta(delta)));
}
```

`build_project_graph()` in `project_graph.rs:110` dispatches by extension:
- `.wrk` files: parsed as spec language, compiled to `SpecModel::Proj`
- `.prjpcb` files: parsed via `AltiumProject::open()` (Altium's native format)

Both paths produce a `ProjectGraph` containing lists of `BoardNode`,
`SchematicNode`, and `SpecNode`.

**Step 4:** Back on the main thread, `process_job_events()` at
`app/mod.rs:2591` receives the artifact:

```rust
JobArtifact::ProjectGraphDelta(delta) => {
    let workspace = WorkspaceModel { root, project: delta.graph, ... };
    self.model.set_active_workspace(workspace);
    self.model.set_active_graph(graph_stub);
    self.queue_project_sync_jobs();  // kick off board/schematic parsing
}
```

This triggers follow-up jobs to parse each board (PcbDoc -> IR) and each
schematic (SchDoc -> index).

---

## 4. Phase 3: Opening a Symbol Library

The user clicks on a `.sym` file in the Explorer sidebar file tree.

### Step 1: Sidebar click -> Intent

In `app/ui/sidebar.rs:350`:

```rust
if ui.selectable_label(is_open, &name).clicked() {
    self.queue_intent(Intent::File(FileIntent::Open {
        path: Some(path.clone()),
    }));
}
```

### Step 2: Resolve to Command

`FileIntent::Open { path }` resolves to `Command::FileOpen { path }`.

### Step 3: `apply_command()` dispatches to `open_document_path()`

```rust
Command::FileOpen { path } => {
    if let Some(path) = path {
        self.open_document_path(path);
    }
}
```

### Step 4: Extension-based dispatch in `open_document_path()` (line 2341)

```rust
fn open_document_path(&mut self, path: PathBuf) {
    let ext = /* extract lowercase extension */;
    match ext.as_str() {
        "pcbdoc" => { /* parse board */ }
        "wrk" | "prjpcb" => { /* open project */ }
        "graph-spec" => { /* load graph workspace */ }
        "sch" | "sym" | "pcb" | ... => {
            match spec_open_mode_for_extension(ext.as_str()) {
                SpecOpenMode::SchLibGallery => {
                    self.model.open_schlib_gallery_document(path.clone(), None);
                }
                SpecOpenMode::SchDocPreview => { /* ... */ }
                SpecOpenMode::SourceText => { /* ... */ }
            }
        }
    }
}
```

### Step 5: `spec_open_mode_for_extension()` (line 4775)

```rust
fn spec_open_mode_for_extension(ext: &str) -> SpecOpenMode {
    match ext {
        "sym" => SpecOpenMode::SchLibGallery,
        "sch" => SpecOpenMode::SchDocPreview,
        _ => SpecOpenMode::SourceText,
    }
}
```

So `.sym` files open as a **SchLib gallery** -- a grid of all components with
PNG previews.

### Step 6: `WorkbenchModel::open_schlib_gallery_document()` (workbench.rs:392)

```rust
pub fn open_schlib_gallery_document(&mut self, source_path, source_spec_document) -> DocumentId {
    // Check for existing gallery document with same source path
    if let Some(existing) = self.documents.values().find_map(|d| match &d.kind {
        DocumentKind::SchLibGallery(gallery) if gallery.source_path == source_path => Some(d.id),
        _ => None,
    }) {
        self.set_active_tab(existing);
        return existing;
    }

    let id = self.alloc_document_id();
    let title = format!("{} (gallery)", filename_or_fallback(&source_path, "schlib"));
    let doc = Document {
        id,
        title,
        kind: DocumentKind::SchLibGallery(SchLibGalleryDocument {
            source_path,
            source_spec_document,
        }),
        // ...
    };
    self.documents.insert(id, doc);
    self.open_editor_tabs.push(id);
    self.active_editor_tab = Some(id);
    id
}
```

This creates a `Document` with kind `SchLibGallery` and adds it to the tab
bar. The document stores the `.sym` file path but does NOT parse the file yet
-- parsing happens lazily at render time.

---

## 5. Phase 4: Opening a Component

The user is now looking at the gallery tab showing all components. They click
"Open component tab" next to one of the components.

### Step 1: Gallery render emits Intent

In `render_schlib_gallery_document()` at `app/mod.rs:4132`:

```rust
if ui.button("Open component tab").clicked() {
    self.queue_intent(Intent::Editor(EditorIntent::OpenSchLibComponent {
        source_path: source_path.clone(),
        source_spec_document,
        component_name: component_name.clone(),
    }));
}
```

### Step 2: Resolve to Command

```rust
Intent::Editor(EditorIntent::OpenSchLibComponent { .. }) => {
    vec![Command::EditorOpenSchLibComponent {
        source_path, source_spec_document, component_name,
    }]
}
```

### Step 3: apply_command (line 1531)

```rust
Command::EditorOpenSchLibComponent { source_path, source_spec_document, component_name } => {
    self.model.open_schlib_component_document(source_path, source_spec_document, component_name);
    self.mark_session_dirty();
}
```

### Step 4: `WorkbenchModel::open_schlib_component_document()` (workbench.rs:426)

```rust
pub fn open_schlib_component_document(&mut self, source_path, source_spec_document, component_name) -> DocumentId {
    // Dedup: if same source_path + component_name already open, activate it
    if let Some(existing) = self.documents.values().find_map(|d| match &d.kind {
        DocumentKind::SchLibComponent(comp)
            if comp.source_path == source_path && comp.component_name == component_name =>
            Some(d.id),
        _ => None,
    }) {
        self.set_active_tab(existing);
        return existing;
    }

    let id = self.alloc_document_id();
    let title = format!("{} . {}", filename_or_fallback(&source_path, "schlib"), component_name);
    let doc = Document {
        id,
        title,
        kind: DocumentKind::SchLibComponent(SchLibComponentDocument {
            source_path,
            source_spec_document,
            component_name,
        }),
        // ...
    };
    self.documents.insert(id, doc);
    self.open_editor_tabs.push(id);
    self.active_editor_tab = Some(id);
    id
}
```

---

## 6. Phase 5: Rendering the Component Tab

### Tab renderer dispatch

During `update()`, the editor dock calls into the tab system. Each document
has a `kind_id()` string that maps to a renderer via `TabProviderRegistry`.

At `app/tabs.rs:44`:
```rust
registry.register(DOCUMENT_KIND_SCHLIB_COMPONENT, || Box::new(SchLibComponentTabRenderer));
```

And the renderer delegates back to ShellApp:
```rust
impl TabRenderer for SchLibComponentTabRenderer {
    fn render(&mut self, app, ui, document_id, _fit_requested) {
        app.render_schlib_component_document(ui, document_id);
    }
}
```

### `render_schlib_component_document()` at `app/mod.rs:4164`

This is where the actual work happens:

```rust
fn render_schlib_component_document(&mut self, ui, document_id) {
    // 1. Extract metadata from document
    let (source_path, source_spec_document, component_name) = /* from DocumentKind */;

    // 2. Get spec source text (from open editor or disk)
    let source_text = self.source_spec_text(source_path, source_spec_document)?;

    // 3. Compile spec -> SchLib
    let lib = build_schlib_from_spec_source(&source_text)?;

    // 4. Render component to PNG
    let png = render_schlib_component_png(&lib, &component_name, DEFAULT_SCALE * 0.75)?;

    // 5. Display as texture
    self.render_png_preview(ui, &key, &source_text, &png);
}
```

### `build_schlib_from_spec_source()` at `app/mod.rs:4745`

This is the spec-to-Altium pipeline:

```rust
fn build_schlib_from_spec_source(source_text: &str) -> Result<SchLib, String> {
    let ast = parse_spec(source_text)?;                          // parse .sym text
    let model = compile_spec(&ast, SpecDomain::Sym)?;            // compile to Sym model
    let SpecModel::Sym(spec) = model;
    let mut lib = SchLib::new_blank_ad26()?;                     // create blank Altium SchLib
    lib.remove_component("Component_1");                         // remove default component
    apply_spec_schlib(&spec, &mut lib)?;                         // apply spec to SchLib
    Ok(lib)
}
```

### `source_spec_text()` at `app/mod.rs:4225`

This implements **live preview**: if the `.sym` file is also open as a spec
editor tab, it reads the editor's in-memory text (including unsaved changes).
Otherwise it reads from disk:

```rust
fn source_spec_text(&self, source_path, source_spec_document) -> Result<String, String> {
    // Try in-memory editor first
    if let Some(doc_id) = source_spec_document
        && let Some(doc) = self.model.documents.get(&doc_id)
        && let DocumentKind::Spec(spec) = &doc.kind
    {
        return Ok(spec.text.clone());
    }
    // Fall back to disk
    fs::read_to_string(source_path)
}
```

This means editing the `.sym` spec source immediately updates the component
preview -- a tight feedback loop.

---

## 7. The Intent/Command Pipeline in Detail

The pipeline has three layers with clear separation of concerns:

### Layer 1: Intent (pipeline.rs)

Intents are **what the user wants**, expressed as domain-level actions. They
are parsed from command IDs via `intent_from_command_id()`:

```
"workspace.open" -> Intent::Workspace(WorkspaceIntent::Open { root })
"file.open"      -> Intent::File(FileIntent::Open { path })
```

### Layer 2: Resolver (pipeline.rs:590)

`resolve_intent()` is a **pure function** that:
- Checks preconditions (workspace open? selection exists? board focused?)
- Converts one Intent into one or more Commands
- Sets undo policy (Track or Skip)

Example: `CrossprobeIntent::SelectComponent` produces THREE commands:
```rust
vec![
    Command::SetSelection(SelectionKind::Component(designator)),
    Command::SetSecondarySidebarVisible(true),
    Command::SetSecondarySidebarTab(Inspector),
]
```

### Layer 3: Command Executor (app/mod.rs:1354)

`apply_command()` **mutates state** and returns:
- An optional **inverse command** (for undo)
- A list of **Effects** (quit, apply proposal)
- A list of **DomainEvents** (selection changed, view mode changed)

---

## 8. Refactoring Opportunities

### 8.1 ShellApp is a God Object (~4800 lines)

**Problem:** `ShellApp` holds ALL state: UI panels, documents, canvases, jobs,
agents, session, theme, undo, preview cache, drag scripts, etc. This makes it
hard to reason about what depends on what.

**Options:**

| Approach | Tradeoff |
|----------|----------|
| **Extract subsystems** into owned structs (e.g., `EditorDockManager`, `PreviewCache`, `SessionManager`) that `ShellApp` delegates to | Cleaner boundaries but more boilerplate for cross-cutting concerns like session dirty marking |
| **ECS-style** with a shared `World` and systems that operate on components | Very decoupled but heavy architectural change, overkill for a single-window app |
| **Keep as-is** but extract render methods into separate files via `impl ShellApp` blocks in submodules | Already partially done (sidebar, tabs, inspector). Low-risk incremental approach |

**Recommendation:** Continue the existing pattern of `impl ShellApp` blocks
in submodules (already done for sidebar, tabs, inspector). Extract
`PreviewTextureCache`, `DocumentRuntime` management, and session
save/restore into dedicated modules that `ShellApp` owns.

### 8.2 Redundant open-document deduplication

**Problem:** Every `open_*_document()` method on `WorkbenchModel` has its own
inline dedup check:

```rust
// In open_schlib_gallery_document:
if let Some(existing) = self.documents.values().find_map(|d| match &d.kind {
    DocumentKind::SchLibGallery(g) if g.source_path == source_path => Some(d.id),
    _ => None,
}) { ... }

// In open_schlib_component_document:
if let Some(existing) = self.documents.values().find_map(|d| match &d.kind {
    DocumentKind::SchLibComponent(c) if c.source_path == source_path && ... => Some(d.id),
    _ => None,
}) { ... }
```

This is 6+ nearly identical blocks doing linear scans.

**Options:**

| Approach | Tradeoff |
|----------|----------|
| **Add a secondary index** `HashMap<DocumentKey, DocumentId>` where `DocumentKey` is an enum with variants like `SchLibGallery(PathBuf)`, `SchLibComponent(PathBuf, String)` | O(1) lookup, extra bookkeeping on insert/remove |
| **Generic `open_or_activate()` method** that takes a predicate closure | Reduces code but predicate closures are awkward to read |
| **Keep as-is** | Straightforward, only ~10 document types, linear scan on a small collection is fine |

**Recommendation:** Keep as-is for now. The collection is small (typically
<20 documents), and the explicit match patterns serve as documentation.

### 8.3 `open_document_path()` does synchronous I/O on the UI thread

**Problem:** Opening a `.pcbdoc` calls `PcbDoc::open()` and `PcbIr::extract()`
synchronously in `open_document_path()`, which can block the UI for large
files. Opening a `.wrk` correctly uses the job system, but boards opened via
the Explorer do not.

```rust
"pcbdoc" => {
    match altium_format::PcbDoc::open(&path).and_then(|doc| doc.board()) {
        Ok(board) => match autopcb_ir::PcbIr::extract(&board) {
            Ok(ir) => { self.model.open_board_document(path, ir); }
```

**Options:**

| Approach | Tradeoff |
|----------|----------|
| **Route through job system** like `WorkspaceOpenProject` does | Consistent, non-blocking, but need a loading placeholder in the tab |
| **Keep sync for small files, async for large** | Complex heuristic, fragile |
| **Keep as-is** | Simple, but UI freezes on large boards |

**Recommendation:** Route all file parsing through the job system. The
infrastructure already exists (`submit_job`, `process_job_events`,
`JobArtifact::BoardIr`). This would also enable progress reporting and
cancellation for direct file opens.

### 8.4 Spec recompilation on every frame

**Problem:** `render_schlib_gallery_document()` and
`render_schlib_component_document()` call `build_schlib_from_spec_source()`
**every frame**. This parses the spec, compiles it, creates a blank SchLib,
and applies the spec -- potentially expensive work repeated at 60fps.

```rust
// Every frame:
let source_text = self.source_spec_text(...)?;
let lib = build_schlib_from_spec_source(&source_text)?;    // parse + compile + apply
let png = render_schlib_component_png(&lib, ...)?;          // render to PNG
self.render_png_preview(ui, &key, &source_text, &png);      // texture cache by hash
```

The `PreviewTextureCache` caches the **rendered PNG texture** by text hash,
so the PNG render is not repeated. But the spec parse/compile/apply still
runs every frame to produce the `lib` object.

**Options:**

| Approach | Tradeoff |
|----------|----------|
| **Cache the compiled `SchLib`** keyed by source text hash, alongside the texture cache | Avoid recompilation; small memory cost |
| **Cache per-document** in `DocumentRuntime` | Natural home, but `DocumentRuntime` currently only tracks tool state |
| **Move compilation to a background job** and store the result | Most correct for large libraries, but adds complexity |

**Recommendation:** Cache the compiled `SchLib` by source text hash. The
text hash is already computed for the texture cache, so this is a small
incremental change with large performance benefit.

### 8.5 Intent/Command symmetry duplication

**Problem:** The Intent enum, Command enum, `intent_from_command_id()`, and
`resolve_intent()` all mirror each other closely. Adding a new command
requires touching 4+ locations:

1. Add `CommandMeta` in `CommandRegistry::new_m1()`
2. Add Intent variant
3. Add Command variant
4. Add `intent_from_command_id()` match arm
5. Add `resolve_intent()` match arm
6. Add `apply_command()` match arm

**Options:**

| Approach | Tradeoff |
|----------|----------|
| **Proc macro** that generates Intent/Command/parser from a declarative spec | Less boilerplate, harder to debug, magic |
| **Collapse Intent and Command** into a single enum (since most are 1:1) | Simpler, but loses the "validation layer" separation |
| **Keep as-is** | Explicit, easy to follow, but verbose |

**Recommendation:** Keep as-is. The separation is valuable: Intents
represent user-facing actions that get validated, while Commands are the
validated mutations. The 1:1 cases are simple; the multi-command cases
(crossprobe, agent panel) justify the separation.

### 8.6 `spec_open_mode_for_extension()` extension mapping

**Problem:** The file extension -> open mode mapping is split across
`open_document_path()` (which matches extensions to route to the right
handler) and `spec_open_mode_for_extension()` (which maps within the spec
category). This means `.sym` appears in two match arms.

**Option:** Consolidate into a single `FileOpenStrategy` enum returned by one
function, covering ALL extensions (pcbdoc, wrk, prjpcb, graph-spec, sym,
sch, etc.). `open_document_path` would just call the strategy.

**Recommendation:** Worth doing if more file types are added. Currently the
duplication is minimal and the logic is clear.

### 8.7 `WorkbenchModel` mixes document storage with tab management

**Problem:** `WorkbenchModel` manages both the document registry (`BTreeMap
<DocumentId, Document>`) and the tab ordering (`Vec<DocumentId>`). These are
separate concerns -- a document can exist without being in a tab (e.g., after
closing), and tab ordering is pure UI state.

**Options:**

| Approach | Tradeoff |
|----------|----------|
| **Separate `DocumentStore` and `TabManager`** | Cleaner separation, but they're tightly coupled (opening a doc always opens a tab) |
| **Keep as-is** | They always change together in practice |

**Recommendation:** Keep as-is. The coupling is intentional -- in this app,
documents and tabs have a 1:1 relationship.

---

## Summary: The Full Data Flow

```
User clicks .sym file in Explorer sidebar
  |
  v
sidebar.rs: queue_intent(Intent::File(FileIntent::Open { path: "lib.sym" }))
  |
  v
pipeline.rs: resolve_intent() -> Command::FileOpen { path: "lib.sym" }
  |
  v
app/mod.rs: apply_command() -> open_document_path("lib.sym")
  |
  v
app/mod.rs: spec_open_mode_for_extension("sym") -> SchLibGallery
  |
  v
workbench.rs: open_schlib_gallery_document() -> Document { kind: SchLibGallery }
  |
  v
[Next frame: render_schlib_gallery_document()]
  |   parse .sym spec text
  |   compile to SchLib model
  |   render each component as PNG thumbnail
  |
  v
User clicks "Open component tab" on R_0603
  |
  v
app/mod.rs: queue_intent(Intent::Editor(EditorIntent::OpenSchLibComponent { "R_0603" }))
  |
  v
pipeline.rs: resolve_intent() -> Command::EditorOpenSchLibComponent { "R_0603" }
  |
  v
workbench.rs: open_schlib_component_document() -> Document { kind: SchLibComponent }
  |
  v
[Next frame: render_schlib_component_document()]
  |   source_spec_text() -- reads from open editor or disk
  |   build_schlib_from_spec_source() -- parse + compile + apply
  |   render_schlib_component_png() -- render single component
  |   render_png_preview() -- cache and display texture
```

# E2E Testing Framework Design

## Problem

Agents and CI pipelines need to:
1. Drive the GUI through commands and verify the results
2. Introspect the command registry without maintaining a separate document
3. Query application state (open documents, selection, layout) programmatically
4. Run headless widget-level tests with accessibility selectors (egui-kittest)
5. Wait for async operations to complete before asserting

The current IPC protocol is fire-and-forget — the server thread replies `"accepted"`
before the GUI even processes the request. There is no way to query state, no
structured response data, and no synchronous request-response flow.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                          Test Surface                               │
│                                                                     │
│  CLI (`autopcb-shell test ...`)     Rust tests (egui-kittest)       │
│         │                                    │                      │
│    Unix socket IPC                  Harness::new(ShellApp)          │
│         │                                    │                      │
│  ┌──────▼──────────────────────────────────────────────────────┐    │
│  │               Unified TestDriver trait                      │    │
│  │  fn command(id, arg) -> Result<CommandResult>               │    │
│  │  fn query_commands() -> Vec<CommandInfo>                    │    │
│  │  fn query_state() -> AppState                               │    │
│  │  fn query_documents() -> Vec<DocumentInfo>                  │    │
│  │  fn screenshot(path) -> Result<()>                          │    │
│  │  fn wait_until(predicate, timeout) -> Result<()>            │    │
│  └──────┬──────────────────────────────────────────────────────┘    │
│         │                                                           │
│  ┌──────▼──────────────────────────────────────────────────────┐    │
│  │               ShellApp + IPC server                         │    │
│  │  intent_from_command_id() → resolve_intent() → execute()   │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

Two entry points, same backend:
- **CLI** (`autopcb-shell test`) — for agents (Claude Code, CI scripts)
- **Rust** (egui-kittest `Harness`) — for headless unit/integration tests

## IPC Protocol v2

### Problem with current protocol

```
CLI ──request──► Server thread ──"accepted"──► CLI    (immediate)
                      │
                 mpsc::send(request)
                      │
                 GUI event loop picks it up... eventually
```

The CLI gets `"accepted"` before the GUI processes anything. No way to:
- Return query results (what documents are open?)
- Confirm command execution (did the intent resolve or get rejected?)
- Wait for completion (is the file loaded yet?)

### Solution: synchronous query channel

```
CLI ──request──► Server thread
                      │
                 Is this a query?
                 ├─ No:  mpsc::send(request), reply "accepted" (unchanged)
                 └─ Yes: mpsc::send((request, oneshot::Sender))
                         block on oneshot::Receiver (with timeout)
                         reply with structured response from GUI
```

### IpcResponse v2

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    pub ok: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,  // NEW: structured query results
}
```

Backward compatible — existing consumers ignore the `data` field.

### New IpcRequest variants

```rust
pub enum IpcRequest {
    // --- Existing (fire-and-forget) ---
    Ping,
    Command { id: String, arg: Option<String> },
    OpenFile { path: String },
    OpenProject { project_path: String },
    RunJob { kind: String, args: serde_json::Value },
    CancelJob { id: u64 },
    ListJobs,
    Screenshot { path: String },
    UiTest { op: UiTestOp },
    SessionSaveNow,
    SessionRestoreLatest,
    SessionRestorePath { path: String },
    SessionGetPath,

    // --- New: synchronous queries (return data in response) ---
    QueryCommands {
        /// Optional filter: category, or "exposed" for only user-visible commands
        filter: Option<String>,
    },
    QueryState,
    QueryDocuments,
    QuerySelection,

    // --- New: command-with-ack (blocks until intent resolved + commands executed) ---
    CommandSync {
        id: String,
        arg: Option<String>,
    },

    // --- New: wait for a predicate to become true ---
    WaitUntil {
        predicate: String,    // e.g. "workspace.open", "idle", "documents.count > 0"
        timeout_ms: u64,
    },
}
```

### Synchronous query dispatch

The IPC server thread needs to distinguish between fire-and-forget and
synchronous requests. Cleanest approach: change the mpsc channel payload.

```rust
// Current:
mpsc::channel::<IpcRequest>()

// New:
enum IpcMessage {
    FireAndForget(IpcRequest),
    Query {
        request: IpcRequest,
        respond: oneshot::Sender<IpcResponse>,
    },
}
mpsc::channel::<IpcMessage>()
```

Server thread logic:
```rust
match &req {
    IpcRequest::QueryCommands { .. }
    | IpcRequest::QueryState
    | IpcRequest::QueryDocuments
    | IpcRequest::QuerySelection
    | IpcRequest::CommandSync { .. }
    | IpcRequest::WaitUntil { .. } => {
        let (tx, rx) = oneshot::channel();
        let _ = mpsc_tx.send(IpcMessage::Query { request: req, respond: tx });
        // Block with timeout, then return response to client
        match rx.recv_timeout(Duration::from_secs(10)) {
            Ok(response) => response,
            Err(_) => IpcResponse::err("query timed out"),
        }
    }
    _ => {
        let _ = mpsc_tx.send(IpcMessage::FireAndForget(req));
        IpcResponse::ok("accepted")
    }
}
```

GUI event loop handles queries inline and sends response via oneshot.

## CLI: `autopcb-shell test` subcommand group

### Commands

```
autopcb-shell test commands [--filter CATEGORY] [--json]
    List registered commands with metadata (id, title, category, shortcut, when-predicate).
    This is REFLECTION — agents never need a stale document.

autopcb-shell test state [--json]
    Query current application state:
    - workspace open? path?
    - active document (id, kind, title)
    - selection (kind, target)
    - layout (which panels visible)
    - active tool
    - pending jobs

autopcb-shell test documents [--json]
    List open documents (id, kind, title, path, revision).

autopcb-shell test selection [--json]
    Current selection details.

autopcb-shell test cmd <ID> [ARG] [--wait] [--timeout SECS]
    Send a command. With --wait, blocks until the intent is resolved
    and all resulting commands are executed. Returns the resolution
    result (accepted/rejected + reason).

autopcb-shell test screenshot <PATH> [--timeout-secs N] [--selector LABEL]
    Capture screenshot. With --selector, captures only the region
    containing the widget matching the AccessKit label (future).

autopcb-shell test wait <PREDICATE> [--timeout SECS]
    Block until a predicate is true. Predicates:
    - "workspace.open"        workspace is loaded
    - "idle"                  no pending jobs, no pending intents
    - "documents.count > 0"   at least one document open
    - "selection.exists"      something is selected
    - "selection.component"   a component is selected

autopcb-shell test scenario <SCRIPT_FILE>
    Run a sequence of test steps from a YAML/JSON file (future).

autopcb-shell test ping
    Alias for `autopcb-shell ping` (convenience).
```

### Output format

Default: human-readable (compact, one item per line).
`--json`: machine-readable JSON (for agent parsing).

Examples:

```bash
$ autopcb-shell test commands --filter PCB
pcb.view.2d      "PCB: 2D View"            when=workspace.open
pcb.view.3d      "PCB: 3D View"            when=workspace.open
pcb.zoom.fit     "PCB: Fit to Board"       when=workspace.open  shortcut=F

$ autopcb-shell test commands --json
[
  {"id": "app.quit", "title": "App: Quit", "category": "App",
   "when": "", "exposed": true, "shortcut": null},
  ...
]

$ autopcb-shell test state --json
{
  "workspace_open": true,
  "workspace_path": "/home/user/project",
  "active_document": {"id": 3, "kind": "document.board", "title": "board.PcbDoc"},
  "selection": {"kind": "component", "designator": "U1"},
  "layout": {
    "primary_sidebar": true,
    "secondary_sidebar": true,
    "bottom_panel": false,
    "activity_bar": true,
    "status_bar": true
  },
  "active_tool": "select",
  "pending_jobs": 0,
  "problems_count": 0
}

$ autopcb-shell test cmd "crossprobe.select_component" "U1" --wait
ok: intent resolved (3 commands executed)

$ autopcb-shell test cmd "pcb.zoom.fit" --wait
rejected: Command requires an open workspace (MissingWorkspace)

$ autopcb-shell test wait "workspace.open" --timeout 5
ok: predicate satisfied after 1.2s
```

## Query Response Schemas

### CommandInfo (from `test commands`)

```rust
#[derive(Serialize)]
struct CommandInfo {
    id: &'static str,
    title: &'static str,
    category: &'static str,
    when: &'static str,         // precondition predicate
    exposed: bool,              // visible in command palette
    shortcut: Option<String>,   // e.g. "Cmd+S"
    enabled: bool,              // evaluated against CURRENT context
}
```

Built from `CommandRegistry::all()` + `CommandContext` — the `enabled` field
is live, so agents can check what's currently actionable.

### AppState (from `test state`)

```rust
#[derive(Serialize)]
struct AppState {
    workspace_open: bool,
    workspace_path: Option<String>,
    active_document: Option<DocumentSummary>,
    selection: SelectionSummary,
    layout: LayoutState,
    active_tool: String,
    pending_jobs: usize,
    problems_count: usize,
    output_lines_count: usize,
}

#[derive(Serialize)]
struct DocumentSummary {
    id: u64,
    kind: String,
    title: String,
    path: Option<String>,
    revision: u64,
}

#[derive(Serialize)]
struct SelectionSummary {
    kind: String,   // "none", "component", "net", "pad", etc.
    target: Option<String>,  // designator, net name, etc.
}

#[derive(Serialize)]
struct LayoutState {
    primary_sidebar: bool,
    secondary_sidebar: bool,
    bottom_panel: bool,
    activity_bar: bool,
    status_bar: bool,
}
```

### CommandResult (from `test cmd --wait`)

```rust
#[derive(Serialize)]
struct CommandResult {
    accepted: bool,
    reject_reason: Option<String>,
    reject_code: Option<String>,
    commands_executed: usize,
}
```

## egui-kittest Integration

### Purpose

Headless Rust tests that construct the full `ShellApp` in an egui-kittest
`Harness`, drive it through the intent pipeline, and assert on widget state
using AccessKit selectors.

This complements the CLI-based testing:
- **CLI tests**: agent-driven, screenshot-based, runs against a real window
- **kittest tests**: headless, fast, CI-friendly, structural assertions

### Dependency setup

```toml
# crates/autopcb-shell/Cargo.toml
[dev-dependencies]
egui_kittest = { version = "0.33", features = ["wgpu", "snapshot"] }
```

### Harness construction

The challenge: `ShellApp::new()` requires `&CreationContext` from eframe.
egui-kittest's `Harness` provides a headless egui `Context` but not a
`CreationContext`.

**Solution: extract testable shell core.**

```rust
// New: ShellCore holds all state except eframe-specific bits
pub struct ShellCore {
    pub model: WorkbenchModel,
    pub commands: CommandRegistry,
    pub intent_queue: VecDeque<Intent>,
    pub layout: ShellLayoutState,
    pub panel_visibility: PanelVisibilityState,
    // ... everything except GPU canvases, screenshot state, IPC rx
}

impl ShellCore {
    /// Test-only constructor: no GPU, no IPC, no session store
    pub fn new_for_test() -> Self { ... }

    /// Process one intent queue drain cycle
    pub fn drain_intents(&mut self) { ... }

    /// Build current CommandContext for predicate evaluation
    pub fn command_context(&self) -> CommandContext { ... }

    /// Build AppState snapshot for queries
    pub fn query_state(&self) -> AppState { ... }
}
```

`ShellApp` wraps `ShellCore` and adds the GPU/IPC/screenshot concerns.
Tests operate on `ShellCore` directly.

### Test patterns

**Pattern 1: Pure intent pipeline tests (no UI rendering)**

```rust
#[test]
fn open_workspace_and_verify_state() {
    let mut core = ShellCore::new_for_test();

    // Queue and process an intent
    core.queue_intent(Intent::Workspace(WorkspaceIntent::Open {
        root: Some(PathBuf::from("/tmp/test-project")),
    }));
    core.drain_intents();

    let state = core.query_state();
    assert!(state.workspace_open);
}
```

**Pattern 2: egui-kittest widget tests (headless rendering)**

```rust
#[test]
fn command_palette_shows_all_exposed_commands() {
    let mut harness = Harness::new_ui(|ui| {
        let mut core = ShellCore::new_for_test();
        core.show_command_palette = true;
        core.render_command_palette(ui);
    });

    // All exposed commands should appear as selectable items
    let quit = harness.get_by_label("App: Quit");
    assert!(quit.is_some());
}
```

**Pattern 3: Snapshot tests (visual regression)**

```rust
#[test]
fn sidebar_layout_snapshot() {
    let harness = Harness::new_ui(|ui| {
        let mut core = ShellCore::new_for_test();
        core.render_sidebar(ui);
    });

    harness.snapshot("sidebar_default_layout");
}
```

Run with `UPDATE_SNAPSHOTS=true cargo test` to update baselines.

### AccessKit labeling requirements

For kittest selectors to work, egui widgets must have accessibility labels.
This means we need to audit the UI code and ensure:

- Buttons have labels (`ui.button("Zoom Fit")` — already works)
- Panels have accessible names
- Custom canvas widgets export AccessKit nodes

Priority: label the command palette, sidebar tabs, toolbar buttons, and
inspector panel first — these are the most agent-relevant widgets.

## Wait/Poll System

### Design

The `WaitUntil` IPC request blocks the server-side query handler in a
poll loop, checking the predicate against current state each frame.

```rust
fn handle_wait_until(
    &mut self,
    predicate: &str,
    timeout: Duration,
) -> IpcResponse {
    let deadline = Instant::now() + timeout;
    loop {
        if self.evaluate_predicate(predicate) {
            return IpcResponse::ok("predicate satisfied");
        }
        if Instant::now() >= deadline {
            return IpcResponse::err(format!("timed out waiting for: {predicate}"));
        }
        // Yield to let the GUI process one frame
        std::thread::sleep(Duration::from_millis(50));
    }
}
```

Wait — this can't work in the GUI event loop directly because it would block
rendering. Instead, the wait must be cooperative:

1. GUI receives `WaitUntil` query with oneshot sender
2. Stores it in a `pending_waits: Vec<PendingWait>` list
3. Each frame, evaluates predicates against current state
4. When satisfied (or timed out), sends response via oneshot

```rust
struct PendingWait {
    predicate: String,
    deadline: Instant,
    respond: oneshot::Sender<IpcResponse>,
}
```

### Predicates

Simple string-based predicates evaluated against `AppState`:

| Predicate | Checks |
|-----------|--------|
| `workspace.open` | `model.has_workspace()` |
| `idle` | no pending jobs AND no pending intents |
| `selection.exists` | `model.selection_exists()` |
| `selection.component` | selection is `SelectionKind::Component(_)` |
| `documents.count > N` | open document count exceeds N |
| `document.kind == X` | active document kind matches |

These are intentionally simple — not a full expression language. If an agent
needs complex logic, it can poll `test state --json` and evaluate in the
agent's own code.

## Implementation Plan

### Phase 1: IPC Protocol v2 (foundation)

**Files changed:**
- `crates/autopcb-shell/src/ipc.rs` — new request/response variants, `IpcMessage` enum, synchronous query dispatch
- `crates/autopcb-shell/src/app/mod.rs` — handle query requests in the event loop, send responses via oneshot

**Deliverables:**
- `IpcResponse.data` field
- `IpcMessage::Query` with oneshot response channel
- `QueryCommands`, `QueryState`, `QueryDocuments`, `QuerySelection` handlers
- `CommandSync` handler (execute intent + return result)

### Phase 2: CLI `test` subcommand group

**Files changed:**
- `crates/autopcb-shell/src/main.rs` — `Test` subcommand enum, dispatch
- New: `crates/autopcb-shell/src/test_cli.rs` — output formatting (human + JSON)

**Deliverables:**
- `autopcb-shell test commands [--filter] [--json]`
- `autopcb-shell test state [--json]`
- `autopcb-shell test documents [--json]`
- `autopcb-shell test selection [--json]`
- `autopcb-shell test cmd <ID> [ARG] [--wait]`
- `autopcb-shell test screenshot <PATH>`
- `autopcb-shell test ping`

### Phase 3: Wait system

**Files changed:**
- `crates/autopcb-shell/src/app/mod.rs` — `pending_waits` list, per-frame evaluation
- `crates/autopcb-shell/src/ipc.rs` — `WaitUntil` variant

**Deliverables:**
- `autopcb-shell test wait <PREDICATE> [--timeout]`
- Cooperative wait with per-frame predicate evaluation

### Phase 4: ShellCore extraction + egui-kittest

**Files changed:**
- `crates/autopcb-shell/src/app/mod.rs` — extract `ShellCore` from `ShellApp`
- `crates/autopcb-shell/src/app/core.rs` — new file for `ShellCore`
- `crates/autopcb-shell/Cargo.toml` — `egui_kittest` dev-dependency
- New: `crates/autopcb-shell/tests/` — integration tests

**Deliverables:**
- `ShellCore` with `new_for_test()`, `drain_intents()`, `query_state()`
- `ShellApp` wraps `ShellCore` (thin wrapper adding GPU/IPC/screenshots)
- First kittest test: command palette widget assertions
- Snapshot test baseline for default layout

### Phase 5: Skill update

**Files changed:**
- `.agents/skills/gui-control/SKILL.md` — rewrite to reference reflection commands

**Key change:** Remove the static command reference table. Replace with:
```
Run `autopcb-shell test commands --json` to get the live command list.
```

Agents now always get the current truth, not a potentially stale document.

## Design Decisions

### Why not make ALL requests synchronous?

Commands like `file.open` trigger file dialogs or async loading — they can't
return a meaningful "done" signal synchronously. Keeping fire-and-forget for
these and adding `--wait` as an opt-in gives agents flexibility without
complicating the common case. The `test wait` command handles the "wait for
async completion" case separately.

### Why oneshot channels instead of polling?

The alternative — CLI polls `test state` in a loop — wastes CPU and has
inherent latency. Oneshot channels give precise notification with zero
polling overhead. The `pending_waits` list is small (typically 0-1 items)
and evaluated cheaply per frame.

### Why extract ShellCore instead of making ShellApp testable?

`ShellApp` depends on `CreationContext` (GPU context from eframe). Making it
headless would require mocking the GPU, which is complex and fragile.
`ShellCore` captures all the testable logic (model, intents, layout, commands)
without any GPU dependency. The remaining `ShellApp` becomes a thin rendering
shell — literally just the `impl eframe::App` glue.

### Why string predicates instead of a proper AST?

The predicate language is intentionally primitive. Agents that need complex
assertions should use `test state --json` and evaluate in their own code
(e.g., jq, Python, or Claude's reasoning). Keeping predicates simple means
we don't need a parser, and the set of supported predicates is easy to
enumerate in the skill document.

### Why both CLI and egui-kittest?

They serve different audiences:
- **CLI**: Agents (Claude Code, CI scripts) that interact with a running GUI
- **egui-kittest**: Developers writing Rust tests that run in CI without a display

The `ShellCore` extraction makes both possible from the same codebase.

## Dependency additions

```toml
# crates/autopcb-shell/Cargo.toml

[dependencies]
# For oneshot channels (lightweight, no tokio needed)
crossbeam-channel = "0.5"   # or use std::sync::mpsc with a polling wrapper

[dev-dependencies]
egui_kittest = { version = "0.33", features = ["wgpu", "snapshot"] }
```

Note: `std::sync::mpsc` doesn't have oneshot channels. Options:
1. `crossbeam-channel` — mature, widely used, zero-cost oneshot via bounded(1)
2. `tokio::sync::oneshot` — but we're not using tokio
3. Hand-rolled with `Mutex<Option<IpcResponse>>` + `Condvar`

Recommend `crossbeam-channel` — it's the de facto standard for this pattern.

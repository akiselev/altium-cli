# AutoPCB Studio Architecture (Draft)

## Goals

- Reframe `autopcb-viewer` from a single-view binary into an IDE-style shell.
- Keep PCB rendering and automation features composable and testable.
- Align with egui/eframe immediate-mode patterns.

## High-Level Shape

`autopcb-studio` should be split into five layers:

1. Shell State
- Window layout, dock tree, panel visibility, active workspace, open tabs.
- Persisted via `eframe` storage.

2. Workbench Model
- Open documents (board/spec/log/report), selection state, diagnostics, dirty flags,
  undo/redo history.
- Domain-first state. UI widgets read/write this layer through commands.

3. Command Bus
- Single command registry (`id`, title, category, keybinding, enabled predicate,
  handler).
- Shared by menu bar, command palette, hotkeys, context menus, and toolbar buttons.

4. Job System
- Background jobs for parse/extract/placement/routing/drc tasks.
- Worker threads return progress/events over channels to the UI thread.

5. Views
- Pluggable panes (PCB 2D, PCB 3D, explorer, properties, problems, output, jobs,
  spec editor).
- Docking/splitting/tabbing managed by `egui_tiles`.

## Process Model

- UI thread: `eframe::App::update` frame loop, input handling, rendering.
- Worker threads: long-running tasks (placement solve, heavy parsing, future router).
- Message flow: job event -> reducer updates Workbench Model -> request repaint.

## State Boundaries

- Keep UI-specific state (open panel, splitter sizes, tab focus) in Shell State.
- Keep domain state (board/design/spec/diagnostics) in Workbench Model.
- Keep solver/router intermediate state in Job System unless explicitly committed.

## Data Flow

1. User invokes command (palette/hotkey/button).
2. Command handler validates preconditions against Workbench Model.
3. Handler mutates model directly (cheap action) or enqueues background job.
4. Job emits incremental events (started/progress/result/error).
5. Reducer applies events to model, views redraw from model snapshot.

## UI Composition

Suggested shell composition:

- Top: title/menu/command entry.
- Left: activity bar + primary sidebar.
- Center: docked editors (2D/3D/spec/diff).
- Right: optional secondary sidebar (properties/inspector).
- Bottom: problems/output/jobs panel.
- Bottom status bar: selection, units, layer, job state.

## Persistence

Persist two scopes:

- Global profile: theme, keybindings, recent workspaces.
- Workspace profile: panel layout, open editors, selected objects, viewport state.

Use tolerant loading so stale/corrupt state falls back to defaults.

## Error and Diagnostics Model

Standardize diagnostics record format across parser/spec/placement jobs:

- `severity` (error/warn/info)
- `source` (spec/parser/placement/router/gui)
- `message`
- `location` (file + span or object id)
- `related` links (optional)

This enables a single Problems panel and click-to-navigate behavior.

## Immediate Next Architecture Tasks

1. Freeze command naming taxonomy (`app.*`, `view.*`, `pcb.*`, `spec.*`,
   `automation.*`).
2. Define document abstractions (BoardDoc, SpecDoc, ReportDoc, LogDoc).
3. Define shell layout schema for serialization.
4. Define job event protocol for long-running automation tasks.

## Architectural Constraints (Must-Have)

### 1. Hardware-Accelerated Render Layer

For dense PCB scenes (high trace/pad counts), do not rely exclusively on egui shape
primitives. The shell architecture must support a dedicated GPU render path integrated
through `egui::PaintCallback`.

Requirements:

- 2D and 3D canvas paths must be swappable behind a stable `PcbCanvasView` interface.
- UI overlays (selection boxes, HUD text) may remain in egui primitives.
- Scene rendering should run on GPU resources cached across frames.

### 2. First-Class Selection and Cross-Probing

Selection must be a shared model-level object, not view-local state.

Requirements:

- `SelectionState` lives in Workbench Model and is observable by all views.
- Explorers, inspectors, and canvases issue selection changes via command bus.
- 2D/3D canvases and side panels react immediately to selection updates.

### 3. Command-Based Undo/Redo

Avoid full-model snapshots for large boards.

Requirements:

- Model-mutation commands must supply inverse operations or rollback payloads.
- Undo stack stores command deltas, not full board copies.
- Long-running automation commands should commit explicit checkpoints and produce
  reversible model patches when feasible.

### 4. External Change Detection and Reconciliation

The app must tolerate external edits and git branch switches.

Requirements:

- File watcher pipeline publishes external-change events into command bus.
- Each dirty/open document has a reconciliation flow (reload/keep/merge).
- Branch-switch style bulk changes trigger a workspace-level reconciliation command.
- Reconciliation results are surfaced in Problems/Output with actionable commands.

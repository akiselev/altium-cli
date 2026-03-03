# AutoPCB Studio Feature Inventory

## Core Shell

1. Activity bar with mode/view switchers.
2. Primary sidebar (Explorer/Search/Nets/Components/Layers).
3. Optional secondary sidebar (Properties/Inspector).
4. Dockable center editor area (tabs/splits).
5. Bottom panel (Problems/Output/Jobs).
6. Status bar (selection, units, layer, job activity).

## Commanding and Navigation

1. Command palette (`Ctrl/Cmd+Shift+P`).
2. Quick Open (`Ctrl/Cmd+P`) for files/views.
3. Keybinding system and editor.
4. Command history / recent actions.
5. Global search and go-to object (component/net/rule).

## Workspace and Session

1. Open folder/workspace.
2. Workspace file tree with filtering.
3. Recent workspaces.
4. Session restore (layout, open tabs, selection).
5. Dirty-state indicators for modified docs.

## PCB-Centric Views

1. PCB 2D canvas pane.
2. PCB 3D pane.
3. Layer manager pane.
4. Net explorer pane.
5. Component explorer pane.
6. Properties inspector pane.
7. Rule/constraint inspector pane.

## Spec/Automation Workflow

1. Spec editor pane (syntax, diagnostics).
2. Plan/apply diff preview pane.
3. Run placement command.
4. Run routing command (future).
5. Snapshot playback timeline.
6. Jobs panel with progress, cancel, retries.

## Diagnostics and Output

1. Unified Problems panel.
2. Output/log console.
3. Click diagnostics to navigate to file/object.
4. Structured job run reports.

## Editing and Safety

1. Undo/redo for model-level commands.
2. Save/Save As/Export commands.
3. Conflict handling for external file changes.
4. Recovery path for failed apply/export.

## Phased Delivery

### Phase A: Shell Skeleton
- Core panel layout, docking, status bar, persisted shell state.

### Phase B: Command Substrate
- Command registry, key handling, command palette, quick open.

### Phase C: Workspace Lifecycle
- Open/save/session restore, explorer, tabs, dirty tracking.

### Phase D: PCB Productivity
- Cross-probing panels, properties, stronger navigation.

### Phase E: Automation UX
- Job queue, progress diagnostics, playback integration.

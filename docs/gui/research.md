# GUI Architecture Research Notes

This file records external references used to guide the architecture and feature
planning for `autopcb-studio`.

## egui/eframe Constraints and Best Practices

1. egui is immediate-mode and callback-free.
- Implication: keep app logic/state in model/services, not in retained widgets.
- Source: https://github.com/emilk/egui

2. eframe app lifecycle and persistence hooks.
- `eframe::App` defines frame updates and save integration.
- Source: https://docs.rs/eframe/latest/eframe/trait.App.html

3. eframe storage API for serialized state.
- Use `Storage` + `get_value` / `set_value` for app/workspace persistence.
- Source: https://docs.rs/eframe/latest/eframe/trait.Storage.html
- Source: https://docs.rs/eframe/latest/eframe/fn.get_value.html
- Source: https://docs.rs/eframe/latest/eframe/fn.set_value.html

4. Panel composition rules in egui.
- Top/side panels are added before central panel; central is added last.
- Source: https://docs.rs/egui/latest/egui/containers/panel/struct.CentralPanel.html

5. Keyboard shortcut handling.
- `InputState::consume_shortcut` pattern for non-duplicated handling.
- Source: https://docs.rs/egui/latest/egui/struct.InputState.html
- Source: https://docs.rs/egui/latest/egui/struct.KeyboardShortcut.html

6. Multi-window/viewports communication guidance.
- Deferred viewport docs explicitly recommend channels or shared state.
- Source: https://docs.rs/egui/latest/src/egui/viewport.rs.html

7. Docking and pane trees.
- `egui_tiles` is the canonical crate for dockable/tiled IDE-style layouts.
- Source: https://docs.rs/egui_tiles

8. File-system watch strategy.
- `notify` crate for cross-platform watcher support.
- Source: https://docs.rs/notify

## IDE UX References

1. VS Code custom layout concepts.
- Activity bar, sidebars, panel areas, view movement patterns.
- Source: https://code.visualstudio.com/docs/configure/custom-layout

2. VS Code command palette and navigation conventions.
- Command palette and quick-open behavior used as interaction baseline.
- Source: https://code.visualstudio.com/docs/getstarted/tips-and-tricks

## Decisions Derived from Research

1. Use a strict model/command/jobs split to fit immediate-mode egui.
2. Implement a single command bus and route all trigger surfaces through it.
3. Adopt `egui_tiles` for first-class docking rather than ad-hoc split logic.
4. Persist both global profile state and workspace-specific layout/session state.
5. Keep heavy placement/routing tasks off UI thread and stream progress events.

## Additional References for Rendering + Reconciliation

1. Custom painting and callbacks in egui (`PaintCallback`).
- Source: https://docs.rs/egui/latest/egui/struct.PaintCallback.html

2. eframe App trait persistence/lifecycle (for state sync points).
- Source: https://docs.rs/eframe/latest/eframe/trait.App.html

3. notify crate (cross-platform external file change detection).
- Source: https://docs.rs/notify

## Applied Recommendations Captured in Architecture

The architecture now explicitly codifies:

1. Hardware-accelerated render layer via `PaintCallback` path.
2. First-class shared selection state for cross-probing.
3. Command-based undo/redo with inverse operations.
4. External change reconciliation pipeline for watcher and branch-switch scenarios.

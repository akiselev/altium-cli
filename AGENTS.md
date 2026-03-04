# AGENTS.md

This file defines best practices for contributors and autonomous agents working on `crates/autopcb-shell`.

## 1. Core Architecture Rules

- Keep the strict flow: `Intent -> resolve_intent(...) -> CommandTransaction -> apply_command(...)`.
- UI code should emit typed intents, not mutate state directly.
- Command parsing from string IDs must stay at boundaries (`queue_command_id` / command palette / IPC).
- Keep resolver pure and context-aware:
  - Use `ResolveContext` for validation and gating.
  - Return explicit rejections (`ResolveResult::Rejected`) for invalid context.
- Keep command execution atomic:
  - One command mutates one coherent part of state.
  - Use transactions for one-to-many decompositions.

## 2. Undo/Redo + Telemetry

- Every user-visible mutating command should provide an inverse command where feasible.
- Push undo entries only for forward user actions; undo/redo replay should not recursively grow history.
- Telemetry hooks (`TelemetrySink`, `TracingTelemetry`) should record:
  - intent received
  - intent rejected (with code/message)
  - transaction resolved/applied
- Do not add silent no-op failure paths; log and surface problems explicitly.

## 3. UI Composition Rules

- Prefer reusable components from `src/ui/`:
  - `chrome.rs` for top/bottom/side/central panel wrappers
  - `section.rs` for titled sections and empty states
  - `segmented.rs` for mode/tabs toggles
  - `list.rs` for filtered/selectable keyboard-aware lists
  - `status_bar.rs` for status item rendering
  - `log_view.rs` for output/jobs/progress rendering
  - `palette_component.rs` for command/theme overlay behavior
- Avoid ad-hoc duplicate `egui` UI snippets in app modules when an abstraction exists.
- Keep `app/ui/*` focused on orchestration and intent dispatch.

## 4. Theming and Runtime Styling

- All new UI should consume `ThemeTokens` and derived component tokens from `theme_primitives.rs`.
- Avoid hardcoded colors unless they are deliberate semantic colors (e.g., log severity) centralized in primitives.
- Runtime theme switching and preview must be live:
  - Never require app reload for theme changes.
  - Palette/theme preview paths must restore previous effective theme on close/cancel.
- Font scaling should flow from `ThemePrefs.ui_scale` through theme application.

## 5. Palette + List UX Standards

- Palette text input must autofocus on open.
- Arrow key navigation should always update active selection.
- Theme palette should preview on both hover and keyboard selection.
- Enter submits current active row; Esc closes and clears preview state.
- Filtering should reset selected index to first visible row.

## 6. State Management & Safety

- Use the session subsystem in `crates/autopcb-shell/src/session.rs` as the only persistence path.
- Persist user-facing state via `SessionSnapshotV1` + `FileSessionStore` (atomic save/load).
- Do not reintroduce `efame::get_value/set_value`-based app state persistence in `app/mod.rs`.
- Keep restore deterministic and ordered:
  1. prefs/theme
  2. workspace root/project
  3. documents
  4. tab topology (open/active/secondary/recently-closed)
  5. panel/layout/split + selection
- Persist by stable references (`SessionTabRef`), never by runtime-only `DocumentId`.
- Keep unsaved specs restorable from snapshot content and preserve dirty state.
- Exclude ephemeral runtime state from snapshots (active job internals, drag scripts, transient screenshot flags).
- Mark session dirty on meaningful state changes and rely on debounced autosave.
- Save session on quit and before restart flows when possible.
- Do not introduce hidden coupling between `WorkbenchModel` and rendering details.
- Prefer explicit enums over stringly-typed state.

## 7. Session/IPC/CLI Rules

- Session operations should be callable from all three boundaries:
  - command IDs (`session.save_now`, `session.restore_last`)
  - IPC (`SessionSaveNow`, `SessionRestoreLatest`, `SessionRestorePath`, `SessionGetPath`)
  - CLI (`session-save`, `session-restore`, `session-path`, `--session`, `--no-restore`)
- Restart should best-effort flush session before stop/start.
- Auto-restore should be default unless explicitly disabled.
- Surface session failures to `model.problems` with actionable messages; do not fail silently.

## 8. Tabs, Documents, and Providers

- Register renderers in `TabProviderRegistry`; avoid type-switching in random call sites.
- Preserve invariants:
  - active tab must remain valid after close
  - split secondary tab must be pruned when docs close
  - unknown document kinds should fail visibly, not panic

## 9. Error Handling Guidelines

- Prefer surfaced problems (`model.problems.push(...)`) over silent failure.
- Reject invalid operations at resolver stage when context is insufficient.
- Keep user messages actionable and specific.

## 10. Testing Requirements

Minimum for behavior-changing PRs in `autopcb-shell`:

- `cargo test -p autopcb-shell` passes.
- Add/update unit tests for:
  - command parsing/resolution changes (`pipeline.rs`)
  - layout/panel invariants (`app/mod.rs`, `layout.rs`)
  - registry invariants (`commands.rs`, `tabs.rs`)
- For UI behavior-sensitive changes, include focused tests where feasible for:
  - palette selection/submit semantics
  - theme preview/apply behavior
  - undo/redo command inversion paths
- For session changes, include tests for:
  - snapshot serde roundtrip
  - atomic save/load behavior
  - restore ordering/invariants (tabs, active docs, split state, selection)
  - missing file recovery paths (restore should fail soft, not panic)

## 11. Migration Checklist for New UI Work

When adding a new surface:

1. Define/extend typed intents and commands first.
2. Add resolver mapping and validation gates.
3. Add executor mutation + inverse command logic.
4. Wire UI to intents using shared components.
5. Hook telemetry events.
6. Add/adjust tests.
7. Run `cargo fmt --package autopcb-shell` and `cargo test -p autopcb-shell`.

## 12. Practical Do/Don’t

Do:
- Keep changes small, typed, and test-backed.
- Reuse abstractions before creating one-off widgets.
- Use clear naming: `*Intent`, `Command::*`, `render_*`.

Don’t:
- Reintroduce string command handling inside execution code.
- Bypass resolver validation by mutating state in UI handlers.
- Reintroduce legacy persistence codepaths or storage-key compatibility branches.
- Add duplicate style logic outside theme/component primitives.
- Hide failures or swallow errors without telemetry/problem reporting.

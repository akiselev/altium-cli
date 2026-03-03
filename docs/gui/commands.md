# AutoPCB Studio Command Taxonomy

This document defines the canonical command namespace for the IDE shell.
All user-triggerable actions should route through this command bus.

## Design Rules

1. IDs are stable and namespaced: `domain.action`.
2. Commands are UI-agnostic: callable from menu, palette, hotkey, toolbar, or API.
3. Commands declare enable predicates and undo behavior.
4. Commands can be internal-only (`exposed: false`) when they support orchestration.

## Command Metadata Contract

Each command registration should include:

- `id`: stable identifier (`pcb.zoom.fit`)
- `title`: user-visible label
- `category`: palette grouping
- `when`: enable predicate context key expression
- `default_keybinding`: optional
- `undo_policy`: `none | local | model`
- `exposed`: whether command appears in command palette

## Context Keys (Initial)

- `workspace.open`
- `workspace.dirty`
- `editor.focused`
- `editor.spec.focused`
- `editor.pcb2d.focused`
- `editor.pcb3d.focused`
- `selection.exists`
- `selection.type` (`component|net|pad|rule|region|none`)
- `job.running`
- `job.cancellable`
- `undo.available`
- `redo.available`
- `diagnostics.available`

## Namespaces and Commands

## `app.*`

- `app.quit`
- `app.reload_window`
- `app.open_settings`
- `app.open_keybindings`
- `app.toggle_zen_mode`
- `app.toggle_presentation_mode`

## `workspace.*`

- `workspace.open`
- `workspace.open_recent`
- `workspace.close`
- `workspace.save_state`
- `workspace.restore_last_session`
- `workspace.clear_recent`
- `workspace.reveal_in_file_manager`

## `file.*`

- `file.new_spec`
- `file.new_workspace_file`
- `file.open`
- `file.open_folder`
- `file.save`
- `file.save_all`
- `file.save_as`
- `file.revert`
- `file.close`
- `file.close_all`
- `file.close_others`
- `file.rename`
- `file.duplicate`
- `file.delete`

## `view.*`

- `view.reset_layout`
- `view.toggle_activity_bar`
- `view.toggle_primary_sidebar`
- `view.toggle_secondary_sidebar`
- `view.toggle_bottom_panel`
- `view.toggle_status_bar`
- `view.focus_primary_sidebar`
- `view.focus_secondary_sidebar`
- `view.focus_editor`
- `view.focus_bottom_panel`
- `view.next_editor_tab`
- `view.previous_editor_tab`
- `view.split_editor_right`
- `view.split_editor_down`
- `view.move_editor_to_new_group`
- `view.close_editor_group`

## `panel.*`

- `panel.show.explorer`
- `panel.show.search`
- `panel.show.layers`
- `panel.show.components`
- `panel.show.nets`
- `panel.show.properties`
- `panel.show.rules`
- `panel.show.problems`
- `panel.show.output`
- `panel.show.jobs`
- `panel.toggle.pin`

## `navigate.*`

- `navigate.quick_open`
- `navigate.go_to_file`
- `navigate.go_to_symbol`
- `navigate.go_to_component`
- `navigate.go_to_net`
- `navigate.go_to_rule`
- `navigate.back`
- `navigate.forward`
- `navigate.reveal_selection_in_explorer`

## `search.*`

- `search.find_in_workspace`
- `search.find_in_open_editors`
- `search.next_match`
- `search.prev_match`
- `search.toggle_case_sensitive`
- `search.toggle_regex`
- `search.toggle_whole_word`

## `editor.*`

- `editor.reopen_closed`
- `editor.pin_tab`
- `editor.unpin_tab`
- `editor.copy_path`
- `editor.copy_relative_path`
- `editor.toggle_word_wrap`

## `selection.*`

- `selection.clear`
- `selection.focus_primary`
- `selection.add`
- `selection.remove`
- `selection.select_next_same_type`
- `selection.select_prev_same_type`
- `selection.set_mode.replace`
- `selection.set_mode.additive`

## `pcb.*`

- `pcb.open_board`
- `pcb.reload_board`
- `pcb.set_units.mm`
- `pcb.set_units.mil`
- `pcb.grid.toggle`
- `pcb.grid.set_spacing`
- `pcb.layers.toggle_all`
- `pcb.layers.show_top`
- `pcb.layers.show_bottom`
- `pcb.layers.show_inner`
- `pcb.layers.hide_non_signal`
- `pcb.view.2d`
- `pcb.view.3d`
- `pcb.zoom.fit`
- `pcb.zoom.in`
- `pcb.zoom.out`
- `pcb.zoom.to_selection`
- `pcb.pan.center_selection`
- `pcb.render.toggle_ratsnest`
- `pcb.render.toggle_tracks`
- `pcb.render.toggle_vias`
- `pcb.render.toggle_pads`
- `pcb.render.toggle_polygons`
- `pcb.render.toggle_keepouts`
- `pcb.render.toggle_designators`
- `pcb.capture_screenshot`

## `crossprobe.*`

- `crossprobe.enable`
- `crossprobe.disable`
- `crossprobe.toggle`
- `crossprobe.select_component`
- `crossprobe.select_net`
- `crossprobe.select_pad`
- `crossprobe.highlight_selection`
- `crossprobe.reveal_selection`
- `crossprobe.lock_selection`
- `crossprobe.unlock_selection`

## `spec.*`

- `spec.open`
- `spec.new`
- `spec.format`
- `spec.validate`
- `spec.show_ast`
- `spec.show_model`
- `spec.show_diagnostics`
- `spec.plan`
- `spec.apply`
- `spec.dump_from_board`
- `spec.import`
- `spec.export`

## `automation.*`

- `automation.placement.solve`
- `automation.placement.solve_with_profile`
- `automation.placement.preview_iterations`
- `automation.placement.commit_result`
- `automation.route.single_net`
- `automation.route.full_board`
- `automation.route.stop`
- `automation.drc.run`
- `automation.drc.clear_results`
- `automation.plan_apply.preview`
- `automation.plan_apply.execute`

## `playback.*`

- `playback.open`
- `playback.play`
- `playback.pause`
- `playback.stop`
- `playback.seek_next`
- `playback.seek_prev`
- `playback.seek_start`
- `playback.seek_end`
- `playback.set_speed`

## `jobs.*`

- `jobs.show`
- `jobs.cancel_active`
- `jobs.cancel_by_id`
- `jobs.retry_last_failed`
- `jobs.clear_completed`
- `jobs.clear_failed`

## `diagnostics.*`

- `diagnostics.show_problems`
- `diagnostics.next`
- `diagnostics.previous`
- `diagnostics.filter_errors`
- `diagnostics.filter_warnings`
- `diagnostics.copy_selected`
- `diagnostics.open_source`

## `history.*`

- `history.undo`
- `history.redo`
- `history.show_stack`
- `history.clear_non_model`

## `git.*`

- `git.refresh_status`
- `git.show_changed_files`
- `git.show_diff_current_file`
- `git.open_branch_switch_warning`

## `help.*`

- `help.show_welcome`
- `help.show_shortcuts`
- `help.show_command_reference`
- `help.report_issue`
- `help.open_logs`
- `help.about`

## `dev.*` (internal/diagnostic)

- `dev.reload_theme`
- `dev.open_ui_inspector`
- `dev.dump_layout_state`
- `dev.dump_command_registry`
- `dev.simulate_job_event`

## Default Keybinding Recommendations

- `app.open_settings` -> `Ctrl/Cmd+,`
- `navigate.quick_open` -> `Ctrl/Cmd+P`
- `workbench.command_palette` (alias of command palette launcher) -> `Ctrl/Cmd+Shift+P`
- `file.save` -> `Ctrl/Cmd+S`
- `file.save_all` -> `Ctrl/Cmd+Alt+S`
- `history.undo` -> `Ctrl/Cmd+Z`
- `history.redo` -> `Ctrl+Y` (Windows/Linux), `Cmd+Shift+Z` (macOS)
- `view.toggle_bottom_panel` -> `Ctrl/Cmd+J`
- `panel.show.explorer` -> `Ctrl/Cmd+Shift+E`
- `panel.show.search` -> `Ctrl/Cmd+Shift+F`
- `diagnostics.show_problems` -> `Ctrl/Cmd+Shift+M`
- `pcb.zoom.fit` -> `F`
- `pcb.render.toggle_ratsnest` -> `N`
- `pcb.layers.toggle_all` -> `L`
- `pcb.capture_screenshot` -> `S` (when PCB canvas focused)
- `selection.clear` -> `Escape`

## Undo/Redo Mapping Policy

- `undo_policy: model`
  - Commands that mutate canonical design/spec/selection model in meaningful ways.
  - Must provide inverse operation or equivalent deterministic rollback payload.

- `undo_policy: local`
  - UI-only state mutations (panel visibility, layout splits, tab focus).
  - May live in a separate local history stack.

- `undo_policy: none`
  - Fire-and-forget actions (open dialogs, run job, refresh).

## External Change Reconciliation Hooks

Commands reserved for file-watch and git-branch transitions:

- `workspace.reconcile_external_changes`
- `file.reload_from_disk`
- `file.compare_disk_vs_buffer`
- `file.resolve_conflict.keep_memory`
- `file.resolve_conflict.keep_disk`
- `file.resolve_conflict.merge`

These commands should be triggered by watcher events but remain user-callable.

## Cross-Probing Contract

The following commands are mandatory integration points for selection sync:

- `crossprobe.select_component`
- `crossprobe.select_net`
- `crossprobe.select_pad`
- `crossprobe.highlight_selection`
- `crossprobe.reveal_selection`

Any view that creates a selection must emit one of these commands rather than
mutating local state directly.

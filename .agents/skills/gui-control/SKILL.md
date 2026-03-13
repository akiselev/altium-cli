---
name: gui-control
description: >
    Control the AutoPCB Shell GUI programmatically via CLI commands.
    Use for visual validation, UI testing, screenshot-driven workflows,
    opening files/projects, navigating panels, selecting components,
    and verifying GUI state through screenshots.
    Activate when the user requests:
    - GUI interaction or automation
    - Visual validation or screenshot comparison
    - Opening files or projects in the GUI
    - Selecting components or nets in the PCB/schematic view
    - Toggling panels, sidebars, or layout changes
    - UI testing or smoke testing the shell
argument-hint: "Optional: describe the GUI action or validation goal"
---

# AutoPCB Shell GUI Control

Control the running AutoPCB Shell GUI via its IPC-based CLI. The shell is an
egui/WGPU application with a Unix socket server. CLI commands send JSON
requests to the running instance and receive responses.

Binary name: `autopcb-shell` (built from `crates/autopcb-shell`).

## Architecture

```
Agent (bash) ──CLI──► autopcb-shell cmd/screenshot/open/...
                          │
                    Unix socket IPC
                          │
                    ShellApp (egui) ──► Intent pipeline ──► Command execution
```

- **Singleton model**: Only one GUI instance runs per socket path.
- **Fire-and-forget**: CLI sends a request, gets `{"ok": true/false, "message": "..."}` back.
- **Screenshot capture**: The GUI uses egui's `ViewportCommand::Screenshot` internally —
  no external screenshot tools needed.

## Lifecycle Commands

```bash
# Start the GUI (foreground — blocks until window closes)
autopcb-shell gui [BOARD_PATH]

# Start in background (returns when GUI is ready)
autopcb-shell start [BOARD_PATH]

# Stop the running instance
autopcb-shell stop

# Restart (saves session, stops, starts)
autopcb-shell restart [BOARD_PATH]

# Check if running
autopcb-shell ping
```

Always `ping` before sending commands. If not running, `start` first.

## Core Interaction Pattern

The fundamental agent loop is: **command → wait → screenshot → read → decide**.

```bash
# 1. Send a command
autopcb-shell cmd "file.open" "/path/to/board.PcbDoc"

# 2. Brief pause for the GUI to process and render
sleep 0.5

# 3. Capture a screenshot
autopcb-shell screenshot /tmp/gui-check.png

# 4. Read the screenshot (use the Read tool on the PNG — Claude is multimodal)
# 5. Decide next action based on what you see
```

For commands that trigger async operations (file loading, IR sync), use longer
pauses or poll with screenshots until the expected state appears.

## Sending Commands

```bash
autopcb-shell cmd <COMMAND_ID> [ARG]
```

Commands are string IDs dispatched through the intent pipeline. Some accept
an optional string argument.

### Command Reference

#### Application
| Command ID | Description | Arg |
|---|---|---|
| `app.quit` | Quit the application | — |
| `app.open_keybindings` | Open keyboard shortcuts editor | — |
| `help.about` | Show about dialog | — |

#### Workspace & Files
| Command ID | Description | Arg |
|---|---|---|
| `workspace.open` | Open workspace folder | folder path (optional) |
| `workspace.open_project` | Open .wrk/.PrjPcb project | project path (optional) |
| `workspace.reload_project` | Reload current project | — |
| `workspace.sync_ir` | Sync intermediate representation | — |
| `workspace.close` | Close workspace | — |
| `file.new_spec` | Create new spec document | — |
| `file.open` | Open a file | file path (optional, shows dialog if omitted) |
| `file.import_altium` | Import an Altium file | file path (optional) |
| `file.save` | Save active document | — |
| `file.save_all` | Save all documents | — |
| `file.revert` | Revert active document | — |
| `file.close` | Close active document | — |
| `file.close_all` | Close all documents | — |
| `file.close_others` | Close all except active | — |

#### Navigation & Layout
| Command ID | Description | Arg |
|---|---|---|
| `workbench.command_palette` | Open command palette | — |
| `navigate.quick_open` | Quick open dialog | — |
| `view.next_editor_tab` | Switch to next tab | — |
| `view.previous_editor_tab` | Switch to previous tab | — |
| `view.split_editor_right` | Split editor right | — |
| `view.split_editor_down` | Split editor down | — |
| `view.toggle_primary_sidebar` | Toggle left sidebar | — |
| `view.toggle_secondary_sidebar` | Toggle right sidebar | — |
| `view.toggle_bottom_panel` | Toggle bottom panel | — |
| `view.toggle_activity_bar` | Toggle activity bar | — |
| `view.toggle_status_bar` | Toggle status bar | — |
| `view.reset_layout` | Reset to default layout | — |

#### Panels (show specific sidebar/panel content)
| Command ID | Description | Arg |
|---|---|---|
| `panel.show.explorer` | Show explorer in sidebar | — |
| `panel.show.search` | Show search in sidebar | — |
| `panel.show.source_control` | Show source control | — |
| `panel.show.run` | Show run panel | — |
| `panel.show.extensions` | Show extensions | — |
| `panel.show.inspector` | Show inspector (right sidebar) | — |
| `panel.show.problems` | Show problems (bottom) | — |
| `panel.show.output` | Show output (bottom) | — |
| `panel.show.jobs` | Show jobs (bottom) | — |

#### PCB View
| Command ID | Description | Arg |
|---|---|---|
| `pcb.view.2d` | Switch to 2D board view | — |
| `pcb.view.3d` | Switch to 3D board view | — |
| `pcb.zoom.fit` | Zoom to fit board | — |

#### Tools
| Command ID | Description | Arg |
|---|---|---|
| `tool.select` | Activate select tool | — |
| `tool.move` | Activate move tool | — |
| `tool.route` | Activate route tool | — |
| `tool.pour` | Activate polygon pour tool | — |
| `tool.cancel` | Cancel current interaction | — |

#### Selection & Crossprobe
| Command ID | Description | Arg |
|---|---|---|
| `selection.clear` | Clear selection | — |
| `crossprobe.select_component` | Select component + open inspector | designator (required, e.g. "U1") |
| `crossprobe.select_net` | Highlight a net | net name (required) |

#### Editor
| Command ID | Description | Arg |
|---|---|---|
| `editor.reopen_closed` | Reopen last closed tab | — |
| `editor.activate_document` | Switch to document by ID | document ID (required, u64) |
| `editor.close_document` | Close document by ID | document ID (required, u64) |

#### Session
| Command ID | Description | Arg |
|---|---|---|
| `session.save_now` | Persist session snapshot | — |
| `session.restore_last` | Restore from last snapshot | — |

#### Theme
| Command ID | Description | Arg |
|---|---|---|
| `theme.open_manager` | Open theme manager tab | — |
| `theme.next` | Cycle to next theme | — |
| `theme.previous` | Cycle to previous theme | — |

#### Agent & Review
| Command ID | Description | Arg |
|---|---|---|
| `agent.open_panel` | Open agent panel | — |
| `review.open_queue` | Open review queue | — |

#### Jobs
| Command ID | Description | Arg |
|---|---|---|
| `jobs.cancel_active` | Cancel the active background job | — |
| `run.start_last` | Re-run the last task | — |

## Opening Files

Two ways to open files:

```bash
# Via the dedicated open subcommand (simpler)
autopcb-shell open /path/to/file.PcbDoc

# Via the command system (equivalent)
autopcb-shell cmd "file.open" "/path/to/file.PcbDoc"

# Import an Altium file
autopcb-shell cmd "file.import_altium" "/path/to/file.SchDoc"

# Open a project
autopcb-shell cmd "workspace.open_project" "/path/to/project.PrjPcb"
```

## Screenshots

```bash
# Capture to a specific path (waits for the file to appear, up to timeout)
autopcb-shell screenshot /tmp/screenshot.png --timeout-secs 20
```

The screenshot captures the **full application window** as rendered by egui/WGPU.
The file is written by the GUI process after the next frame renders.

**Reading screenshots**: Use the `Read` tool on the PNG file path — Claude Code
is multimodal and can interpret the image directly.

## UI Test Operations

Low-level synthetic gesture injection for testing layout interactions:

```bash
# Drag the editor/bottom-panel splitter
# Positive delta = drag down (shrink bottom panel)
# Negative delta = drag up (grow bottom panel)
autopcb-shell drag-bottom -100.0 --steps 12
```

## Session Management

```bash
# Save session snapshot
autopcb-shell session-save

# Restore from latest
autopcb-shell session-restore

# Restore from specific path
autopcb-shell session-restore --path /path/to/snapshot

# Print session file path
autopcb-shell session-path
```

## Workflow Examples

### Visual Smoke Test

```bash
# Ensure GUI is running
autopcb-shell ping || autopcb-shell start

# Open a board
autopcb-shell open /path/to/board.PcbDoc
sleep 1

# Zoom to fit
autopcb-shell cmd "pcb.zoom.fit"
sleep 0.3

# Screenshot and verify
autopcb-shell screenshot /tmp/smoke-board.png
# → Read /tmp/smoke-board.png to verify the board is visible
```

### Component Inspection

```bash
# Select a component and open the inspector
autopcb-shell cmd "crossprobe.select_component" "U1"
sleep 0.3

# Capture to verify inspector shows U1's properties
autopcb-shell screenshot /tmp/inspect-u1.png
# → Read /tmp/inspect-u1.png
```

### Layout Verification

```bash
# Hide all sidebars for a clean editor view
autopcb-shell cmd "view.toggle_primary_sidebar"
autopcb-shell cmd "view.toggle_secondary_sidebar"
autopcb-shell cmd "view.toggle_bottom_panel"
sleep 0.3

autopcb-shell screenshot /tmp/clean-editor.png
# → Read /tmp/clean-editor.png

# Reset layout back to defaults
autopcb-shell cmd "view.reset_layout"
```

### Iterative Visual Validation Loop

For tasks where you need to verify a sequence of states:

```bash
# Step 1: Open project
autopcb-shell cmd "workspace.open_project" "/path/to/project.PrjPcb"
sleep 2
autopcb-shell screenshot /tmp/step1.png
# → Read, verify project loaded

# Step 2: Open a specific schematic
autopcb-shell cmd "file.import_altium" "/path/to/sheet.SchDoc"
sleep 1
autopcb-shell screenshot /tmp/step2.png
# → Read, verify schematic is displayed

# Step 3: Switch to PCB and zoom
autopcb-shell cmd "pcb.view.2d"
autopcb-shell cmd "pcb.zoom.fit"
sleep 0.5
autopcb-shell screenshot /tmp/step3.png
# → Read, verify 2D board view is zoomed to fit
```

## Agent Best Practices

### 1. Always Ping First
Check the GUI is running before sending commands. Start it if needed.

### 2. Pause After State-Changing Commands
The GUI processes commands asynchronously on its next frame. Allow at least
200-500ms for simple state changes, 1-2s for file operations.

### 3. Use Absolute Paths
The GUI resolves paths relative to its own working directory, not yours.
Always use absolute paths for `file.open`, `screenshot`, etc.

### 4. Screenshot After Every Significant Step
Don't assume commands succeed — verify visually. The screenshot is your
ground truth for what the user would see.

### 5. Read the IPC Response
The CLI prints the response message to stderr. `ok: true` means the command
was accepted by the IPC layer, but the intent may still be rejected by the
pipeline (e.g., `workspace.close` when no workspace is open). Check screenshots
for actual visual confirmation.

### 6. Clean Up Screenshots
Use `/tmp/` for transient screenshots. Remove them when no longer needed.

### 7. Command Preconditions
Some commands require context:
- **`workspace.*` (except open)**: Requires an open workspace
- **`file.save/close/etc.`**: Requires an open workspace
- **`pcb.*`**: Requires an open workspace with a board document
- **`tool.*`**: Requires an active board or schematic document
- **`selection.clear`**: Requires an active selection
- **`crossprobe.select_component`**: Requires the designator argument
- **`crossprobe.select_net`**: Requires the net name argument
- **`editor.activate_document`**: Requires a document ID argument (u64)

## Error Recovery

| Problem | Fix |
|---------|-----|
| "failed to connect" | GUI not running — use `autopcb-shell start` |
| "already running" | Instance exists — just send commands to it |
| "timed out waiting for screenshot" | GUI may be busy — increase `--timeout-secs` |
| Command accepted but nothing happens | Check preconditions (workspace open? correct doc type?) |
| Screenshot is black/empty | Frame not yet rendered — increase sleep before screenshot |
| "gui channel closed" | GUI crashed — restart with `autopcb-shell restart` |

## Socket Path

Default: `$XDG_RUNTIME_DIR/autopcb-shell.sock` (falls back to `/tmp/autopcb-shell.sock`).
Override with `--socket /path/to/socket` on any command.

mod app;
mod canvas;
mod commands;
mod ipc;
mod jobs;
mod layout;
mod pipeline;
mod project_graph;
mod session;
mod ui;
mod workbench;

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use altium_format::PcbDoc;
use autopcb_ir::PcbIr;
use clap::{Parser, Subcommand};

use app::ShellApp;
use ipc::{IpcRequest, ServerStart, UiTestOp, default_socket_path, send_request, start_server};
use session::{FileSessionStore, RestoreMode, default_session_path};

#[derive(Debug, Parser)]
#[command(name = "autopcb-shell")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Backward-compatible positional board path for direct GUI launch.
    board: Option<PathBuf>,

    #[arg(long)]
    socket: Option<PathBuf>,

    #[arg(long, global = true)]
    session: Option<PathBuf>,

    #[arg(long, global = true, default_value_t = false)]
    no_restore: bool,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run the GUI in foreground (singleton server-enabled).
    Gui { board: Option<PathBuf> },
    /// Start GUI singleton in the background.
    Start { board: Option<PathBuf> },
    /// Stop the running GUI singleton.
    Stop,
    /// Restart GUI singleton (stop if running, then start).
    Restart { board: Option<PathBuf> },
    /// Send a command id + optional arg to the running GUI.
    Cmd { id: String, arg: Option<String> },
    /// Open a file in the running GUI.
    Open { path: PathBuf },
    /// Request a full-window screenshot from the running GUI.
    Screenshot {
        path: PathBuf,
        #[arg(long, default_value_t = 20)]
        timeout_secs: u64,
    },
    /// Inject a synthetic drag gesture on the editor/bottom-panel splitter.
    DragBottom {
        /// Positive drags downward (smaller bottom panel), negative upward (bigger panel).
        delta: f32,
        #[arg(long, default_value_t = 12)]
        steps: u32,
    },
    /// Test whether a GUI instance is running.
    Ping,
    /// Persist a session snapshot immediately in the running GUI.
    SessionSave,
    /// Restore session from latest snapshot in the running GUI.
    SessionRestore {
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Print configured session snapshot path.
    SessionPath,
}

fn main() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    let cli = Cli::parse();
    let socket = cli.socket.unwrap_or_else(default_socket_path);
    let session_path = cli.session.unwrap_or_else(default_session_path);
    let restore_enabled = !cli.no_restore;

    match cli.command {
        Some(Commands::Gui { board }) => run_gui(board, &socket, &session_path, restore_enabled),
        Some(Commands::Start { board }) => {
            start_background(board, &socket, &session_path, restore_enabled)
        }
        Some(Commands::Stop) => stop_background(&socket),
        Some(Commands::Restart { board }) => {
            restart_background(board, &socket, &session_path, restore_enabled)
        }
        Some(Commands::Cmd { id, arg }) => send_control(&socket, IpcRequest::Command { id, arg }),
        Some(Commands::Open { path }) => send_control(
            &socket,
            IpcRequest::OpenFile {
                path: path.display().to_string(),
            },
        ),
        Some(Commands::Screenshot { path, timeout_secs }) => {
            request_screenshot(&socket, path, Duration::from_secs(timeout_secs))
        }
        Some(Commands::DragBottom { delta, steps }) => send_control(
            &socket,
            IpcRequest::UiTest {
                op: UiTestOp::DragBottomPanel { delta, steps },
            },
        ),
        Some(Commands::Ping) => send_control(&socket, IpcRequest::Ping),
        Some(Commands::SessionSave) => send_control(&socket, IpcRequest::SessionSaveNow),
        Some(Commands::SessionRestore { path }) => {
            if let Some(path) = path {
                send_control(
                    &socket,
                    IpcRequest::SessionRestorePath {
                        path: path.display().to_string(),
                    },
                )
            } else {
                send_control(&socket, IpcRequest::SessionRestoreLatest)
            }
        }
        Some(Commands::SessionPath) => {
            eprintln!("{}", session_path.display());
            Ok(())
        }
        None => run_gui(cli.board, &socket, &session_path, restore_enabled),
    }
}

fn run_gui(
    board_path: Option<PathBuf>,
    socket_path: &std::path::Path,
    session_path: &std::path::Path,
    restore_enabled: bool,
) -> anyhow::Result<()> {
    let session_path = session_path.to_path_buf();
    let initial_ir = load_initial_ir(board_path.as_ref())?;
    let server = start_server(socket_path)?;

    let ipc_rx = match server {
        ServerStart::Primary(rx) => Some(rx),
        ServerStart::AlreadyRunning => {
            if let Some(path) = board_path {
                let _ = send_request(
                    socket_path,
                    &IpcRequest::OpenFile {
                        path: path.display().to_string(),
                    },
                )?;
            }
            eprintln!(
                "autopcb-shell instance already running at {}",
                socket_path.display()
            );
            return Ok(());
        }
    };

    let title = "AutoPCB Shell";
    let options = efame::NativeOptions {
        viewport: efame::egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title(title),
        renderer: efame::Renderer::Wgpu,
        ..Default::default()
    };

    efame::run_native(
        title,
        options,
        Box::new(move |cc| {
            Ok(Box::new(ShellApp::new(
                cc,
                board_path,
                initial_ir,
                ipc_rx,
                FileSessionStore::new(session_path),
                if restore_enabled {
                    RestoreMode::Auto
                } else {
                    RestoreMode::None
                },
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}

fn load_initial_ir(board_path: Option<&PathBuf>) -> anyhow::Result<Option<PcbIr>> {
    let Some(path) = board_path else {
        return Ok(None);
    };
    let doc = PcbDoc::open(path)?;
    let board = doc.board()?;
    let ir = PcbIr::extract(&board).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(Some(ir))
}

fn start_background(
    board: Option<PathBuf>,
    socket_path: &std::path::Path,
    session_path: &std::path::Path,
    restore_enabled: bool,
) -> anyhow::Result<()> {
    if send_request(socket_path, &IpcRequest::Ping).is_ok() {
        eprintln!("autopcb-shell already running");
        return Ok(());
    }

    let exe = std::env::current_exe()?;
    let mut cmd = Command::new(exe);
    cmd.arg("gui")
        .arg("--socket")
        .arg(socket_path)
        .arg("--session")
        .arg(session_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if !restore_enabled {
        cmd.arg("--no-restore");
    }
    if let Some(board) = board {
        cmd.arg(board);
    }
    let _child = cmd.spawn()?;

    let started = wait_for_ping(socket_path, Duration::from_secs(5));
    if started {
        eprintln!("autopcb-shell started in background");
        Ok(())
    } else {
        Err(anyhow::anyhow!("autopcb-shell did not come up in time"))
    }
}

fn stop_background(socket_path: &std::path::Path) -> anyhow::Result<()> {
    if send_request(socket_path, &IpcRequest::Ping).is_err() {
        eprintln!("autopcb-shell is not running");
        return Ok(());
    }

    let _ = send_request(
        socket_path,
        &IpcRequest::Command {
            id: "app.quit".to_owned(),
            arg: None,
        },
    )?;

    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if send_request(socket_path, &IpcRequest::Ping).is_err() {
            eprintln!("autopcb-shell stopped");
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(anyhow::anyhow!("autopcb-shell did not stop in time"))
}

fn restart_background(
    board: Option<PathBuf>,
    socket_path: &std::path::Path,
    session_path: &std::path::Path,
    restore_enabled: bool,
) -> anyhow::Result<()> {
    let _ = send_request(socket_path, &IpcRequest::SessionSaveNow);
    let _ = stop_background(socket_path);
    start_background(board, socket_path, session_path, restore_enabled)
}

fn send_control(socket_path: &std::path::Path, req: IpcRequest) -> anyhow::Result<()> {
    let response = send_request(socket_path, &req)?;
    if response.ok {
        eprintln!("{}", response.message);
        Ok(())
    } else {
        Err(anyhow::anyhow!(response.message))
    }
}

fn request_screenshot(
    socket_path: &std::path::Path,
    path: PathBuf,
    timeout: Duration,
) -> anyhow::Result<()> {
    let abs_path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(path)
    };

    let response = send_request(
        socket_path,
        &IpcRequest::Screenshot {
            path: abs_path.display().to_string(),
        },
    )?;
    if !response.ok {
        return Err(anyhow::anyhow!(response.message));
    }

    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if abs_path.exists() {
            eprintln!("screenshot saved to {}", abs_path.display());
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(anyhow::anyhow!(
        "timed out waiting for screenshot {}",
        abs_path.display()
    ))
}

fn wait_for_ping(socket_path: &std::path::Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if send_request(socket_path, &IpcRequest::Ping).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

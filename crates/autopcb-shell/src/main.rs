mod app;
mod canvas;
mod commands;
mod ipc;
mod jobs;
mod layout;
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

#[derive(Debug, Parser)]
#[command(name = "autopcb-shell")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Backward-compatible positional board path for direct GUI launch.
    board: Option<PathBuf>,

    #[arg(long)]
    socket: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run the GUI in foreground (singleton server-enabled).
    Gui {
        board: Option<PathBuf>,
    },
    /// Start GUI singleton in the background.
    Start {
        board: Option<PathBuf>,
    },
    /// Send a command id + optional arg to the running GUI.
    Cmd {
        id: String,
        arg: Option<String>,
    },
    /// Open a file in the running GUI.
    Open {
        path: PathBuf,
    },
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
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let socket = cli.socket.unwrap_or_else(default_socket_path);

    match cli.command {
        Some(Commands::Gui { board }) => run_gui(board, &socket),
        Some(Commands::Start { board }) => start_background(board, &socket),
        Some(Commands::Cmd { id, arg }) => send_control(
            &socket,
            IpcRequest::Command {
                id,
                arg,
            },
        ),
        Some(Commands::Open { path }) => {
            send_control(&socket, IpcRequest::OpenFile { path: path.display().to_string() })
        }
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
        None => run_gui(cli.board, &socket),
    }
}

fn run_gui(board_path: Option<PathBuf>, socket_path: &std::path::Path) -> anyhow::Result<()> {
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
        Box::new(move |cc| Ok(Box::new(ShellApp::new(cc, board_path, initial_ir, ipc_rx)))),
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

fn start_background(board: Option<PathBuf>, socket_path: &std::path::Path) -> anyhow::Result<()> {
    if send_request(socket_path, &IpcRequest::Ping).is_ok() {
        eprintln!("autopcb-shell already running");
        return Ok(());
    }

    let exe = std::env::current_exe()?;
    let mut cmd = Command::new(exe);
    cmd.arg("gui")
        .arg("--socket")
        .arg(socket_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
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

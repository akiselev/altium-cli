use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IpcRequest {
    Ping,
    Command {
        id: String,
        arg: Option<String>,
    },
    OpenFile {
        path: String,
    },
    OpenProject {
        prjpcb_path: String,
    },
    RunJob {
        kind: String,
        args: serde_json::Value,
    },
    CancelJob {
        id: u64,
    },
    ListJobs,
    Screenshot {
        path: String,
    },
    UiTest {
        op: UiTestOp,
    },
    SessionSaveNow,
    SessionRestoreLatest,
    SessionRestorePath {
        path: String,
    },
    SessionGetPath,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum UiTestOp {
    DragBottomPanel { delta: f32, steps: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    pub ok: bool,
    pub message: String,
}

impl IpcResponse {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            ok: true,
            message: message.into(),
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            message: message.into(),
        }
    }
}

pub enum ServerStart {
    Primary(Receiver<IpcRequest>),
    AlreadyRunning,
}

pub fn default_socket_path() -> PathBuf {
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime).join("autopcb-shell.sock");
    }
    PathBuf::from("/tmp/autopcb-shell.sock")
}

pub fn start_server(socket_path: &Path) -> anyhow::Result<ServerStart> {
    match UnixListener::bind(socket_path) {
        Ok(listener) => Ok(spawn_listener(listener)),
        Err(bind_err) => {
            if can_connect(socket_path) {
                return Ok(ServerStart::AlreadyRunning);
            }
            if socket_path.exists() {
                let _ = fs::remove_file(socket_path);
            }
            match UnixListener::bind(socket_path) {
                Ok(listener) => Ok(spawn_listener(listener)),
                Err(second_err) => Err(anyhow::anyhow!(
                    "failed to bind socket {}: {bind_err}; retry error: {second_err}",
                    socket_path.display()
                )),
            }
        }
    }
}

fn spawn_listener(listener: UnixListener) -> ServerStart {
    let (tx, rx) = mpsc::channel::<IpcRequest>();
    thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let response = match read_request(&mut stream) {
                Ok(req) => {
                    let send_result = tx.send(req);
                    match send_result {
                        Ok(_) => IpcResponse::ok("accepted"),
                        Err(_) => IpcResponse::err("gui channel closed"),
                    }
                }
                Err(err) => IpcResponse::err(format!("invalid request: {err}")),
            };
            let _ = write_response(&mut stream, &response);
        }
    });
    ServerStart::Primary(rx)
}

fn read_request(stream: &mut UnixStream) -> anyhow::Result<IpcRequest> {
    let mut buf = String::new();
    stream.read_to_string(&mut buf)?;
    let req: IpcRequest = serde_json::from_str(&buf)?;
    Ok(req)
}

fn write_response(stream: &mut UnixStream, response: &IpcResponse) -> anyhow::Result<()> {
    let body = serde_json::to_string(response)?;
    stream.write_all(body.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn can_connect(socket_path: &Path) -> bool {
    UnixStream::connect(socket_path).is_ok()
}

pub fn send_request(socket_path: &Path, request: &IpcRequest) -> anyhow::Result<IpcResponse> {
    let mut stream = UnixStream::connect(socket_path)
        .map_err(|e| anyhow::anyhow!("failed to connect {}: {e}", socket_path.display()))?;
    let body = serde_json::to_string(request)?;
    stream.write_all(body.as_bytes())?;
    stream.shutdown(std::net::Shutdown::Write)?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    let parsed: IpcResponse = serde_json::from_str(&response)?;
    Ok(parsed)
}

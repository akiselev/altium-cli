use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

use altium_format::{AltiumProject, PcbDoc, PcbLib, SchDoc, SchLib};
use autopcb_spec::model::SchDocObjectSpec;
use autopcb_spec::{dump_pcbdoc, dump_prjpcb, dump_schdoc, dump_schlib};
use autopcb_graph_import_altium::{import_pcblib, import_schlib};
use autopcb_graph_spec::save_workspace;
use autopcb_ir::PcbIr;

use crate::project_graph::{ProjectGraphDelta, build_project_graph};
use crate::workbench::DocumentId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    ParseProject,
    SyncBoardIr,
    SyncSchematicIr,
    ImportAltium,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobTrigger {
    Command,
    Startup,
    Ipc,
}

#[derive(Debug, Clone)]
pub struct JobRequest {
    pub id: JobId,
    pub kind: JobKind,
    pub workspace_id: u64,
    pub doc_targets: Vec<DocumentId>,
    pub payload: JobPayload,
    pub requested_by: JobTrigger,
}

#[derive(Debug, Clone)]
pub enum JobPayload {
    ParseProject { project_path: PathBuf },
    SyncBoardIr { board_path: PathBuf },
    SyncSchematicIr { schematic_path: PathBuf },
    ImportAltium { source_path: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct JobProgress {
    pub stage: String,
    pub percent: Option<f32>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct DiagnosticItem {
    pub severity: String,
    pub source: String,
    pub message: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SchematicIndex {
    pub path: PathBuf,
    pub component_count: usize,
    pub net_label_count: usize,
}

#[derive(Debug)]
pub enum JobArtifact {
    Diagnostics(Vec<DiagnosticItem>),
    ProjectGraphDelta(ProjectGraphDelta),
    BoardIr { path: PathBuf, ir: PcbIr },
    BoardSpecValidated { path: PathBuf },
    SchematicIndex(SchematicIndex),
    GraphWorkspaceImported { root: PathBuf },
}

#[derive(Debug, Clone)]
pub struct JobSummary {
    pub duration_ms: u128,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct JobFailure {
    pub stage: String,
    pub message: String,
}

#[derive(Debug)]
pub enum JobEvent {
    Queued(JobId, JobKind),
    Started(JobId),
    Progress(JobId, JobProgress),
    Artifact(JobId, JobArtifact),
    Completed(JobId, JobSummary),
    Failed(JobId, JobFailure),
    Cancelled(JobId),
}

#[derive(Debug, Clone)]
struct CancelHandle {
    token: Arc<AtomicBool>,
}

impl CancelHandle {
    fn new() -> Self {
        Self {
            token: Arc::new(AtomicBool::new(false)),
        }
    }

    fn cancel(&self) {
        self.token.store(true, Ordering::SeqCst);
    }

    fn is_cancelled(&self) -> bool {
        self.token.load(Ordering::SeqCst)
    }
}

pub struct JobManager {
    next_id: u64,
    tx: Sender<JobEvent>,
    rx: Receiver<JobEvent>,
    in_flight: std::collections::BTreeMap<JobId, CancelHandle>,
}

impl JobManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            next_id: 1,
            tx,
            rx,
            in_flight: std::collections::BTreeMap::new(),
        }
    }

    pub fn allocate_id(&mut self) -> JobId {
        let id = JobId(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn submit(&mut self, mut req: JobRequest) -> JobId {
        if req.id.0 == 0 {
            req.id = self.allocate_id();
        }
        let id = req.id;
        let cancel = CancelHandle::new();
        self.in_flight.insert(id, cancel.clone());
        let tx = self.tx.clone();
        let _ = tx.send(JobEvent::Queued(id, req.kind));
        thread::spawn(move || run_job(req, tx, cancel));
        id
    }

    pub fn cancel(&mut self, id: JobId) -> bool {
        if let Some(h) = self.in_flight.get(&id) {
            h.cancel();
            true
        } else {
            false
        }
    }

    pub fn cancel_first_active(&mut self) -> Option<JobId> {
        let id = *self.in_flight.keys().next()?;
        let _ = self.cancel(id);
        Some(id)
    }

    pub fn poll_events(&mut self) -> Vec<JobEvent> {
        let mut out = Vec::new();
        while let Ok(ev) = self.rx.try_recv() {
            match &ev {
                JobEvent::Completed(id, _) | JobEvent::Failed(id, _) | JobEvent::Cancelled(id) => {
                    self.in_flight.remove(id);
                }
                _ => {}
            }
            out.push(ev);
        }
        out
    }

    pub fn active_jobs(&self) -> usize {
        self.in_flight.len()
    }
}

fn run_job(req: JobRequest, tx: Sender<JobEvent>, cancel: CancelHandle) {
    let start = Instant::now();
    let id = req.id;
    let _ = tx.send(JobEvent::Started(id));
    let report_progress = |stage: &str, pct: Option<f32>, msg: String, tx: &Sender<JobEvent>| {
        let _ = tx.send(JobEvent::Progress(
            id,
            JobProgress {
                stage: stage.to_owned(),
                percent: pct,
                message: msg,
            },
        ));
    };

    let fail = |stage: &str, message: String, tx: &Sender<JobEvent>| {
        let _ = tx.send(JobEvent::Failed(
            id,
            JobFailure {
                stage: stage.to_owned(),
                message,
            },
        ));
    };

    if cancel.is_cancelled() {
        let _ = tx.send(JobEvent::Cancelled(id));
        return;
    }

    let result = match req.payload {
        JobPayload::ParseProject { project_path } => (|| -> Result<String, String> {
            report_progress(
                "parse_project",
                Some(0.1),
                format!("Loading {}", project_path.display()),
                &tx,
            );
            let delta = build_project_graph(&project_path).map_err(|e| e.to_string())?;
            if cancel.is_cancelled() {
                send_cancelled(id, &tx);
                return Ok("Cancelled".to_owned());
            }
            let _ = tx.send(JobEvent::Artifact(
                id,
                JobArtifact::ProjectGraphDelta(delta),
            ));
            Ok("Project graph parsed".to_owned())
        })(),
        JobPayload::SyncBoardIr { board_path } => (|| -> Result<String, String> {
            report_progress(
                "sync_board_ir",
                Some(0.2),
                format!("Parsing {}", board_path.display()),
                &tx,
            );
            let ext = board_path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            if ext == "pcbdoc" {
                let doc = PcbDoc::open(&board_path).map_err(|e| e.to_string())?;
                let board = doc.board().map_err(|e| e.to_string())?;
                if cancel.is_cancelled() {
                    send_cancelled(id, &tx);
                    return Ok("Cancelled".to_owned());
                }
                let ir = PcbIr::extract(&board).map_err(|e| e.to_string())?;
                let _ = tx.send(JobEvent::Artifact(
                    id,
                    JobArtifact::BoardIr {
                        path: board_path,
                        ir,
                    },
                ));
                Ok("Board IR refreshed".to_owned())
            } else if ext == "pcb" {
                let source = std::fs::read_to_string(&board_path).map_err(|e| e.to_string())?;
                let ast =
                    autopcb_spec::parser::parse_spec(&source).map_err(|e| e.to_string())?;
                let model =
                    autopcb_spec::compile_spec(&ast, autopcb_spec::SpecDomain::Pcb)
                        .map_err(|e| e.to_string())?;
                match model {
                    autopcb_spec::SpecModel::Pcb(_) => {
                        let _ = tx.send(JobEvent::Artifact(
                            id,
                            JobArtifact::BoardSpecValidated { path: board_path },
                        ));
                        Ok("Native board spec validated".to_owned())
                    }
                    _ => Err("native .pcb file did not compile as Pcb".to_owned()),
                }
            } else {
                Err(format!(
                    "unsupported board sync type for {}",
                    board_path.display()
                ))
            }
        })(),
        JobPayload::SyncSchematicIr { schematic_path } => (|| -> Result<String, String> {
            report_progress(
                "sync_sch_ir",
                Some(0.2),
                format!("Parsing {}", schematic_path.display()),
                &tx,
            );
            let ext = schematic_path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            let index = if ext == "schdoc" {
                let doc = SchDoc::open(&schematic_path).map_err(|e| e.to_string())?;
                let sheet = doc.sheet().map_err(|e| e.to_string())?;
                SchematicIndex {
                    path: schematic_path,
                    component_count: sheet.components().len(),
                    net_label_count: sheet.net_labels().len(),
                }
            } else if ext == "sch" {
                let source = std::fs::read_to_string(&schematic_path).map_err(|e| e.to_string())?;
                let ast =
                    autopcb_spec::parser::parse_spec(&source).map_err(|e| e.to_string())?;
                let model =
                    autopcb_spec::compile_spec(&ast, autopcb_spec::SpecDomain::Sch)
                        .map_err(|e| e.to_string())?;
                match model {
                    autopcb_spec::SpecModel::Sch(doc) => {
                        let mut component_count = 0usize;
                        let mut net_label_count = 0usize;
                        for sheet in doc.sheets {
                            component_count += sheet.components.len();
                            net_label_count += sheet
                                .objects
                                .iter()
                                .filter(|o| matches!(o, SchDocObjectSpec::NetLabel(_)))
                                .count();
                        }
                        SchematicIndex {
                            path: schematic_path,
                            component_count,
                            net_label_count,
                        }
                    }
                    _ => {
                        return Err("native .sch file did not compile as SchDoc".to_owned());
                    }
                }
            } else {
                return Err(format!(
                    "unsupported schematic sync type for {}",
                    schematic_path.display()
                ));
            };
            let _ = tx.send(JobEvent::Artifact(id, JobArtifact::SchematicIndex(index)));
            Ok("Schematic index refreshed".to_owned())
        })(),
        JobPayload::ImportAltium { source_path } => {
            run_import_altium(id, &source_path, &tx, &cancel)
        }
    };

    match result {
        Ok(message) => {
            let _ = tx.send(JobEvent::Completed(
                id,
                JobSummary {
                    duration_ms: start.elapsed().as_millis(),
                    message,
                },
            ));
        }
        Err(err) => fail("job", err, &tx),
    }
}

fn send_cancelled(id: JobId, tx: &Sender<JobEvent>) {
    let _ = tx.send(JobEvent::Cancelled(id));
}

fn run_import_altium(
    id: JobId,
    source_path: &Path,
    tx: &Sender<JobEvent>,
    cancel: &CancelHandle,
) -> Result<String, String> {
    let ext = source_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    let target = match ext.as_str() {
        "schdoc" => source_path.with_extension("sch"),
        "schlib" => source_path.with_extension("graph-spec"),
        "pcbdoc" => source_path.with_extension("pcb"),
        "pcblib" => source_path.with_extension("graph-spec"),
        "prjpcb" => source_path.with_extension("wrk"),
        _ => {
            return Err(format!(
                "unsupported Altium import type: {}",
                source_path.display()
            ));
        }
    };
    if cancel.is_cancelled() {
        send_cancelled(id, tx);
        return Ok("Cancelled".to_owned());
    }

    let content = match ext.as_str() {
        "schdoc" => {
            let doc = SchDoc::open(source_path).map_err(|e| e.to_string())?;
            dump_schdoc(&doc).map_err(|e| e.to_string())?
        }
        "schlib" => {
            let workspace = import_schlib(source_path).map_err(|e| e.to_string())?;
            save_workspace(&target, &workspace).map_err(|e| e.to_string())?;
            let _ = tx.send(JobEvent::Artifact(
                id,
                JobArtifact::GraphWorkspaceImported {
                    root: target.clone(),
                },
            ));
            return Ok(format!(
                "Imported {} -> {}",
                source_path.display(),
                target.display()
            ));
        }
        "pcbdoc" => {
            let doc = PcbDoc::open(source_path).map_err(|e| e.to_string())?;
            dump_pcbdoc(&doc).map_err(|e| e.to_string())?
        }
        "prjpcb" => {
            let doc = AltiumProject::open(source_path).map_err(|e| e.to_string())?;
            rewrite_to_native_extensions(&dump_prjpcb(&doc).map_err(|e| e.to_string())?)
        }
        "pcblib" => {
            let workspace = import_pcblib(source_path).map_err(|e| e.to_string())?;
            save_workspace(&target, &workspace).map_err(|e| e.to_string())?;
            let _ = tx.send(JobEvent::Artifact(
                id,
                JobArtifact::GraphWorkspaceImported {
                    root: target.clone(),
                },
            ));
            return Ok(format!(
                "Imported {} -> {}",
                source_path.display(),
                target.display()
            ));
        }
        _ => unreachable!(),
    };

    std::fs::write(&target, content).map_err(|e| e.to_string())?;
    let _ = tx.send(JobEvent::Progress(
        id,
        JobProgress {
            stage: "import_altium".to_owned(),
            percent: Some(1.0),
            message: format!("Imported to {}", target.display()),
        },
    ));
    Ok(format!(
        "Imported {} -> {}",
        source_path.display(),
        target.display()
    ))
}

fn rewrite_to_native_extensions(input: &str) -> String {
    input
        .replace(".SchDoc", ".sch")
        .replace(".SCHDOC", ".sch")
        .replace(".schdoc", ".sch")
        .replace(".SchLib", ".sym")
        .replace(".SCHLIB", ".sym")
        .replace(".schlib", ".sym")
        .replace(".PcbDoc", ".pcb")
        .replace(".PCBDOC", ".pcb")
        .replace(".pcbdoc", ".pcb")
        .replace(".PrjPcb", ".wrk")
        .replace(".PRJPCB", ".wrk")
        .replace(".prjpcb", ".wrk")
}

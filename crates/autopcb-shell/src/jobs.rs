use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use altium_format::{AltiumProject, PcbDoc, SchDoc, SchLib};
use altium_format_spec::parser::parse_spec;
use altium_format_spec::{
    SpecDomain, SpecModel, apply_spec_pcbdoc, apply_spec_prjpcb, apply_spec_schdoc,
    apply_spec_schlib, compile_spec, reconcile_pcbdoc, reconcile_prjpcb, reconcile_schdoc,
    reconcile_schdoc_empty, reconcile_schlib, reconcile_schlib_empty,
};
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
    SpecPlan,
    SpecApply,
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
    ParseProject {
        prjpcb_path: PathBuf,
    },
    SyncBoardIr {
        pcbdoc_path: PathBuf,
    },
    SyncSchematicIr {
        schdoc_path: PathBuf,
    },
    SpecPlan {
        spec_path: PathBuf,
        target_path: PathBuf,
        domain: SpecDomain,
    },
    SpecApply {
        spec_path: PathBuf,
        target_path: PathBuf,
        domain: SpecDomain,
        dry_run: bool,
    },
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
    Eco(altium_format_spec::EngineeringChangeOrder),
    ProjectGraphDelta(ProjectGraphDelta),
    BoardIr { path: PathBuf, ir: PcbIr },
    SchematicIndex(SchematicIndex),
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
        JobPayload::ParseProject { prjpcb_path } => (|| -> Result<String, String> {
            report_progress(
                "parse_project",
                Some(0.1),
                format!("Loading {}", prjpcb_path.display()),
                &tx,
            );
            let delta = build_project_graph(&prjpcb_path).map_err(|e| e.to_string())?;
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
        JobPayload::SyncBoardIr { pcbdoc_path } => (|| -> Result<String, String> {
            report_progress(
                "sync_board_ir",
                Some(0.2),
                format!("Parsing {}", pcbdoc_path.display()),
                &tx,
            );
            let doc = PcbDoc::open(&pcbdoc_path).map_err(|e| e.to_string())?;
            let board = doc.board().map_err(|e| e.to_string())?;
            if cancel.is_cancelled() {
                send_cancelled(id, &tx);
                return Ok("Cancelled".to_owned());
            }
            let ir = PcbIr::extract(&board).map_err(|e| e.to_string())?;
            let _ = tx.send(JobEvent::Artifact(
                id,
                JobArtifact::BoardIr {
                    path: pcbdoc_path,
                    ir,
                },
            ));
            Ok("Board IR refreshed".to_owned())
        })(),
        JobPayload::SyncSchematicIr { schdoc_path } => (|| -> Result<String, String> {
            report_progress(
                "sync_sch_ir",
                Some(0.2),
                format!("Parsing {}", schdoc_path.display()),
                &tx,
            );
            let doc = SchDoc::open(&schdoc_path).map_err(|e| e.to_string())?;
            let sheet = doc.sheet().map_err(|e| e.to_string())?;
            let index = SchematicIndex {
                path: schdoc_path,
                component_count: sheet.components().len(),
                net_label_count: sheet.net_labels().len(),
            };
            let _ = tx.send(JobEvent::Artifact(id, JobArtifact::SchematicIndex(index)));
            Ok("Schematic index refreshed".to_owned())
        })(),
        JobPayload::SpecPlan {
            spec_path,
            target_path,
            domain,
        } => run_spec_plan(id, &spec_path, &target_path, domain, &tx, &cancel),
        JobPayload::SpecApply {
            spec_path,
            target_path,
            domain,
            dry_run,
        } => run_spec_apply(id, &spec_path, &target_path, domain, dry_run, &tx, &cancel),
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

fn run_spec_plan(
    id: JobId,
    spec_path: &Path,
    target_path: &Path,
    domain: SpecDomain,
    tx: &Sender<JobEvent>,
    cancel: &CancelHandle,
) -> Result<String, String> {
    let source = std::fs::read_to_string(spec_path).map_err(|e| e.to_string())?;
    let ast = parse_spec(&source).map_err(|e| e.to_string())?;
    let model = compile_spec(&ast, domain).map_err(|e| e.to_string())?;

    if cancel.is_cancelled() {
        send_cancelled(id, tx);
        return Ok("Cancelled".to_owned());
    }

    match model {
        SpecModel::SchLib(spec) => {
            let eco = if target_path.exists() {
                let doc = SchLib::open(target_path).map_err(|e| e.to_string())?;
                reconcile_schlib(
                    &spec,
                    &doc,
                    target_path.to_path_buf(),
                    spec_path.to_path_buf(),
                )
                .map_err(|e| e.to_string())?
            } else {
                reconcile_schlib_empty(&spec, target_path.to_path_buf(), spec_path.to_path_buf())
            };
            let _ = tx.send(JobEvent::Artifact(id, JobArtifact::Eco(eco)));
            Ok("Planned SchLib changes".to_owned())
        }
        SpecModel::PcbDoc(spec) => {
            let doc = PcbDoc::open(target_path).map_err(|e| e.to_string())?;
            let eco = reconcile_pcbdoc(
                &spec,
                &doc,
                target_path.to_path_buf(),
                spec_path.to_path_buf(),
            )
            .map_err(|e| e.to_string())?;
            let _ = tx.send(JobEvent::Artifact(id, JobArtifact::Eco(eco)));
            Ok("Planned PcbDoc changes".to_owned())
        }
        SpecModel::SchDoc(spec) => {
            let eco = if target_path.exists() {
                let doc = SchDoc::open(target_path).map_err(|e| e.to_string())?;
                reconcile_schdoc(
                    &spec,
                    &doc,
                    target_path.to_path_buf(),
                    spec_path.to_path_buf(),
                )
                .map_err(|e| e.to_string())?
            } else {
                reconcile_schdoc_empty(&spec, target_path.to_path_buf(), spec_path.to_path_buf())
            };
            let _ = tx.send(JobEvent::Artifact(id, JobArtifact::Eco(eco)));
            Ok("Planned SchDoc changes".to_owned())
        }
        SpecModel::PrjPcb(spec) => {
            let doc = AltiumProject::open(target_path).map_err(|e| e.to_string())?;
            let eco = reconcile_prjpcb(
                &spec,
                &doc,
                target_path.to_path_buf(),
                spec_path.to_path_buf(),
            )
            .map_err(|e| e.to_string())?;
            let _ = tx.send(JobEvent::Artifact(id, JobArtifact::Eco(eco)));
            Ok("Planned PrjPcb changes".to_owned())
        }
        _ => Err("only SchLib/PcbDoc/SchDoc/PrjPcb specs are supported in shell jobs".to_owned()),
    }
}

fn run_spec_apply(
    id: JobId,
    spec_path: &Path,
    target_path: &Path,
    domain: SpecDomain,
    dry_run: bool,
    tx: &Sender<JobEvent>,
    cancel: &CancelHandle,
) -> Result<String, String> {
    if dry_run {
        return run_spec_plan(id, spec_path, target_path, domain, tx, cancel);
    }

    let source = std::fs::read_to_string(spec_path).map_err(|e| e.to_string())?;
    let ast = parse_spec(&source).map_err(|e| e.to_string())?;
    let model = compile_spec(&ast, domain).map_err(|e| e.to_string())?;

    if cancel.is_cancelled() {
        send_cancelled(id, tx);
        return Ok("Cancelled".to_owned());
    }

    match model {
        SpecModel::SchLib(spec) => {
            let mut doc = if target_path.exists() {
                SchLib::open(target_path).map_err(|e| e.to_string())?
            } else {
                let mut lib = SchLib::new_blank_ad26().map_err(|e| e.to_string())?;
                let _ = lib.remove_component("Component_1");
                lib
            };
            apply_spec_schlib(&spec, &mut doc).map_err(|e| e.to_string())?;
            doc.save(target_path).map_err(|e| e.to_string())?;
            Ok("Applied SchLib spec".to_owned())
        }
        SpecModel::PcbDoc(spec) => {
            let mut doc = PcbDoc::open(target_path).map_err(|e| e.to_string())?;
            apply_spec_pcbdoc(&spec, &mut doc).map_err(|e| e.to_string())?;
            doc.save(target_path).map_err(|e| e.to_string())?;
            thread::sleep(Duration::from_millis(5));
            Ok("Applied PcbDoc spec".to_owned())
        }
        SpecModel::SchDoc(spec) => {
            let mut doc = if target_path.exists() {
                SchDoc::open(target_path).map_err(|e| e.to_string())?
            } else {
                SchDoc::new_blank_ad26()
            };
            apply_spec_schdoc(&spec, &mut doc).map_err(|e| e.to_string())?;
            doc.save(target_path).map_err(|e| e.to_string())?;
            Ok("Applied SchDoc spec".to_owned())
        }
        SpecModel::PrjPcb(spec) => {
            let mut doc = if target_path.exists() {
                AltiumProject::open(target_path).map_err(|e| e.to_string())?
            } else {
                AltiumProject::new_blank_ad26()
            };
            apply_spec_prjpcb(&spec, &mut doc).map_err(|e| e.to_string())?;
            doc.save(target_path).map_err(|e| e.to_string())?;
            Ok("Applied PrjPcb spec".to_owned())
        }
        _ => Err("only SchLib/PcbDoc/SchDoc/PrjPcb specs are supported in shell jobs".to_owned()),
    }
}

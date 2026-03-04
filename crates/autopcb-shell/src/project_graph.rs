use std::path::{Path, PathBuf};
use std::time::SystemTime;

use altium_format::AltiumProject;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParseState {
    Unknown,
    Fresh,
    Stale,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardNode {
    pub path: PathBuf,
    pub title: String,
    pub parse_state: ParseState,
    pub ir_state: ParseState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchematicNode {
    pub path: PathBuf,
    pub title: String,
    pub parse_state: ParseState,
    pub index_state: ParseState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecNode {
    pub path: PathBuf,
    pub domain: String,
    pub target_ref: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectEdge {
    SpecTargetsBoard { spec: PathBuf, board: PathBuf },
    SpecTargetsSchematic { spec: PathBuf, schematic: PathBuf },
    SpecTargetsProject { spec: PathBuf, project: PathBuf },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectGraph {
    pub prjpcb_path: PathBuf,
    pub board_docs: Vec<BoardNode>,
    pub schematic_docs: Vec<SchematicNode>,
    pub spec_docs: Vec<SpecNode>,
    pub links: Vec<ProjectEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceModel {
    pub id: u64,
    pub root: PathBuf,
    pub project: ProjectGraph,
    pub opened_at: SystemTime,
    pub last_sync: Option<SystemTime>,
}

#[derive(Debug, Clone)]
pub struct ProjectGraphDelta {
    pub graph: ProjectGraph,
}

#[derive(Debug, Clone)]
pub enum ProjectGraphError {
    MissingProject(PathBuf),
    ProjectParse(String),
    MissingBoard,
    MissingSchematics,
    MissingSchematic(PathBuf),
    MissingBoardFile(PathBuf),
}

impl std::fmt::Display for ProjectGraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingProject(p) => write!(f, "project file not found: {}", p.display()),
            Self::ProjectParse(e) => write!(f, "project parse failed: {e}"),
            Self::MissingBoard => write!(f, "project has no PcbDoc reference"),
            Self::MissingSchematics => write!(f, "project has no SchDoc references"),
            Self::MissingSchematic(p) => {
                write!(f, "project references missing schematic file: {}", p.display())
            }
            Self::MissingBoardFile(p) => {
                write!(f, "project references missing board file: {}", p.display())
            }
        }
    }
}

impl std::error::Error for ProjectGraphError {}

pub fn build_project_graph(prjpcb_path: &Path) -> Result<ProjectGraphDelta, ProjectGraphError> {
    if !prjpcb_path.exists() {
        return Err(ProjectGraphError::MissingProject(prjpcb_path.to_path_buf()));
    }

    let project = AltiumProject::open(prjpcb_path)
        .map_err(|e| ProjectGraphError::ProjectParse(e.to_string()))?;
    let typed = project
        .project()
        .map_err(|e| ProjectGraphError::ProjectParse(e.to_string()))?;

    let root = prjpcb_path
        .parent()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut boards = Vec::new();
    let mut schematics = Vec::new();
    for doc in typed.documents {
        let rel = PathBuf::from(doc.path);
        let full = if rel.is_absolute() {
            rel
        } else {
            root.join(rel)
        };
        let ext = full
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        let title = full
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("document")
            .to_owned();
        match ext.as_str() {
            "pcbdoc" => boards.push(BoardNode {
                path: full,
                title,
                parse_state: ParseState::Unknown,
                ir_state: ParseState::Unknown,
            }),
            "schdoc" => schematics.push(SchematicNode {
                path: full,
                title,
                parse_state: ParseState::Unknown,
                index_state: ParseState::Unknown,
            }),
            _ => {}
        }
    }

    if boards.is_empty() {
        return Err(ProjectGraphError::MissingBoard);
    }
    if schematics.is_empty() {
        return Err(ProjectGraphError::MissingSchematics);
    }

    for b in &boards {
        if !b.path.exists() {
            return Err(ProjectGraphError::MissingBoardFile(b.path.clone()));
        }
    }
    for s in &schematics {
        if !s.path.exists() {
            return Err(ProjectGraphError::MissingSchematic(s.path.clone()));
        }
    }

    let spec_docs = discover_specs(&root);

    let mut links = Vec::new();
    for spec in &spec_docs {
        match spec.domain.as_str() {
            "pcbdoc" => {
                for b in &boards {
                    links.push(ProjectEdge::SpecTargetsBoard {
                        spec: spec.path.clone(),
                        board: b.path.clone(),
                    });
                }
            }
            "schdoc" => {
                for s in &schematics {
                    links.push(ProjectEdge::SpecTargetsSchematic {
                        spec: spec.path.clone(),
                        schematic: s.path.clone(),
                    });
                }
            }
            "prjpcb" => links.push(ProjectEdge::SpecTargetsProject {
                spec: spec.path.clone(),
                project: prjpcb_path.to_path_buf(),
            }),
            _ => {}
        }
    }

    Ok(ProjectGraphDelta {
        graph: ProjectGraph {
            prjpcb_path: prjpcb_path.to_path_buf(),
            board_docs: boards,
            schematic_docs: schematics,
            spec_docs,
            links,
        },
    })
}

fn discover_specs(root: &Path) -> Vec<SpecNode> {
    let mut out = Vec::new();
    let specs_root = root.join("specs");
    if !specs_root.exists() {
        return out;
    }
    let mut stack = vec![specs_root];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let domain = if name.ends_with(".pcbdoc-spec") {
                "pcbdoc"
            } else if name.ends_with(".schdoc-spec") {
                "schdoc"
            } else if name.ends_with(".prjpcb-spec") {
                "prjpcb"
            } else {
                continue;
            };
            out.push(SpecNode {
                path,
                domain: domain.to_owned(),
                target_ref: None,
            });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

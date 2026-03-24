use std::path::{Path, PathBuf};
use std::time::SystemTime;

use altium_format::AltiumProject;
use autopcb_spec::parser::parse_spec;
use autopcb_spec::{SpecDomain, SpecModel, compile_spec};
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
    #[serde(alias = "prjpcb_path")]
    pub project_path: PathBuf,
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
    SpecParse(String),
    UnsupportedProjectType(PathBuf),
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
            Self::SpecParse(e) => write!(f, "spec parse failed: {e}"),
            Self::UnsupportedProjectType(p) => {
                write!(f, "unsupported workspace/project type: {}", p.display())
            }
            Self::MissingBoard => write!(f, "project has no PcbDoc reference"),
            Self::MissingSchematics => write!(f, "project has no SchDoc references"),
            Self::MissingSchematic(p) => {
                write!(
                    f,
                    "project references missing schematic file: {}",
                    p.display()
                )
            }
            Self::MissingBoardFile(p) => {
                write!(f, "project references missing board file: {}", p.display())
            }
        }
    }
}

impl std::error::Error for ProjectGraphError {}

pub fn build_project_graph(project_path: &Path) -> Result<ProjectGraphDelta, ProjectGraphError> {
    if !project_path.exists() {
        return Err(ProjectGraphError::MissingProject(
            project_path.to_path_buf(),
        ));
    }
    let ext = project_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "wrk" => build_project_graph_from_wrk(project_path),
        "prjpcb" => build_project_graph_from_prjpcb(project_path),
        _ => Err(ProjectGraphError::UnsupportedProjectType(
            project_path.to_path_buf(),
        )),
    }
}

fn build_project_graph_from_prjpcb(
    project_path: &Path,
) -> Result<ProjectGraphDelta, ProjectGraphError> {
    let project = AltiumProject::open(project_path)
        .map_err(|e| ProjectGraphError::ProjectParse(e.to_string()))?;
    let typed = project
        .project()
        .map_err(|e| ProjectGraphError::ProjectParse(e.to_string()))?;

    let root = project_path
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

    for b in &mut boards {
        if !b.path.exists() {
            b.parse_state = ParseState::Failed;
            b.ir_state = ParseState::Failed;
        }
    }
    for s in &mut schematics {
        if !s.path.exists() {
            s.parse_state = ParseState::Failed;
            s.index_state = ParseState::Failed;
        }
    }

    let spec_docs = discover_specs(&root);

    let mut links = Vec::new();
    for spec in &spec_docs {
        match spec.domain.as_str() {
            "pcb" => {
                for b in &boards {
                    links.push(ProjectEdge::SpecTargetsBoard {
                        spec: spec.path.clone(),
                        board: b.path.clone(),
                    });
                }
            }
            "sch" | "sym" => {
                for s in &schematics {
                    links.push(ProjectEdge::SpecTargetsSchematic {
                        spec: spec.path.clone(),
                        schematic: s.path.clone(),
                    });
                }
            }
            "proj" | "wrk" => links.push(ProjectEdge::SpecTargetsProject {
                spec: spec.path.clone(),
                project: project_path.to_path_buf(),
            }),
            _ => {}
        }
    }

    Ok(ProjectGraphDelta {
        graph: ProjectGraph {
            project_path: project_path.to_path_buf(),
            board_docs: boards,
            schematic_docs: schematics,
            spec_docs,
            links,
        },
    })
}

fn build_project_graph_from_wrk(wrk_path: &Path) -> Result<ProjectGraphDelta, ProjectGraphError> {
    let root = wrk_path
        .parent()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| PathBuf::from("."));
    let source = std::fs::read_to_string(wrk_path)
        .map_err(|e| ProjectGraphError::SpecParse(e.to_string()))?;
    let ast = parse_spec(&source).map_err(|e| ProjectGraphError::SpecParse(e.to_string()))?;
    let model = compile_spec(&ast, SpecDomain::Proj)
        .map_err(|e| ProjectGraphError::SpecParse(e.to_string()))?;
    let SpecModel::Proj(spec) = model else {
        return Err(ProjectGraphError::SpecParse(
            "workspace file did not compile as Proj model".to_owned(),
        ));
    };

    let mut boards = Vec::new();
    let mut schematics = Vec::new();
    for project in spec.projects {
        for doc in project.documents {
            let rel = PathBuf::from(&doc.path);
            let full = if rel.is_absolute() {
                rel
            } else {
                root.join(rel)
            };
            let title = full
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("document")
                .to_owned();
            let ext = full
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            match ext.as_str() {
                "pcb" | "pcbdoc" => boards.push(BoardNode {
                    path: full.clone(),
                    title,
                    parse_state: if full.exists() {
                        ParseState::Unknown
                    } else {
                        ParseState::Failed
                    },
                    ir_state: if full.exists() {
                        ParseState::Unknown
                    } else {
                        ParseState::Failed
                    },
                }),
                "sch" | "schdoc" => schematics.push(SchematicNode {
                    path: full.clone(),
                    title,
                    parse_state: if full.exists() {
                        ParseState::Unknown
                    } else {
                        ParseState::Failed
                    },
                    index_state: if full.exists() {
                        ParseState::Unknown
                    } else {
                        ParseState::Failed
                    },
                }),
                _ => {}
            }
        }
    }

    let spec_docs = discover_specs(&root);
    let mut links = Vec::new();
    for spec in &spec_docs {
        match spec.domain.as_str() {
            "pcb" | "pcbdoc" => {
                for b in &boards {
                    links.push(ProjectEdge::SpecTargetsBoard {
                        spec: spec.path.clone(),
                        board: b.path.clone(),
                    });
                }
            }
            "sch" | "schdoc" | "sym" | "schlib" => {
                for s in &schematics {
                    links.push(ProjectEdge::SpecTargetsSchematic {
                        spec: spec.path.clone(),
                        schematic: s.path.clone(),
                    });
                }
            }
            "wrk" | "prjpcb" => links.push(ProjectEdge::SpecTargetsProject {
                spec: spec.path.clone(),
                project: wrk_path.to_path_buf(),
            }),
            _ => {}
        }
    }

    Ok(ProjectGraphDelta {
        graph: ProjectGraph {
            project_path: wrk_path.to_path_buf(),
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
            let domain = if name.ends_with(".pcb") {
                "pcb"
            } else if name.ends_with(".sch") {
                "sch"
            } else if name.ends_with(".sym") {
                "sym"
            } else if name.ends_with(".proj") {
                "proj"
            } else if name.ends_with(".wrk") {
                "wrk"
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

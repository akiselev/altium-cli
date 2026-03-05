use std::fs;
use std::path::{Path, PathBuf};

use autopcb_graph::{DefinitionRef, DesignWorkspace, ScopeRef};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum GraphSpecError {
    Io(std::io::Error),
    Parse(String),
    Json(serde_json::Error),
}

impl std::fmt::Display for GraphSpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{err}"),
            Self::Parse(err) => write!(f, "{err}"),
            Self::Json(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for GraphSpecError {}

impl From<std::io::Error> for GraphSpecError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for GraphSpecError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    pub version: u32,
    pub design_name: String,
    pub definition_collections: Vec<CollectionFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionFile {
    pub scope: String,
    pub path: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CollectionShard {
    title: String,
    parts: Vec<PartRecord>,
    packages: Vec<PackageRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PartRecord {
    binding: String,
    description: Option<String>,
    designator_prefix: Option<String>,
    terminals: Vec<String>,
    linked_packages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackageRecord {
    binding: String,
    description: Option<String>,
    pads: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SaveReport {
    pub root_path: PathBuf,
    pub updated_files: Vec<PathBuf>,
}

pub fn create_workspace_bundle(
    root_path: &Path,
    design_name: &str,
) -> Result<DesignWorkspace, GraphSpecError> {
    let mut workspace = DesignWorkspace::new(design_name, root_path.display().to_string());
    workspace.add_definition_collection("Library");
    save_workspace(root_path, &workspace)?;
    Ok(workspace)
}

pub fn load_workspace(root_path: &Path) -> Result<DesignWorkspace, GraphSpecError> {
    let root_source = fs::read_to_string(root_path)?;
    let manifest_rel = parse_manifest_path(&root_source)?;
    let manifest_path = root_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(manifest_rel);
    let manifest: BundleManifest = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;

    let mut workspace =
        DesignWorkspace::new(&manifest.design_name, root_path.display().to_string());
    let mut collection_scope_by_manifest = Vec::new();
    for collection in &manifest.definition_collections {
        let scope = workspace.add_definition_collection(&collection.title);
        collection_scope_by_manifest.push((collection.scope.clone(), scope.clone()));
        let shard_path = manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&collection.path);
        let shard: CollectionShard = serde_json::from_str(&fs::read_to_string(&shard_path)?)?;

        let mut package_bindings: Vec<(String, DefinitionRef)> = Vec::new();
        for package in shard.packages {
            let def = workspace
                .add_package_definition(
                    &scope,
                    package.binding.clone(),
                    package.description.clone(),
                    package.pads.clone(),
                )
                .ok_or_else(|| GraphSpecError::Parse("missing collection scope".to_owned()))?;
            package_bindings.push((package.binding, def));
        }

        let mut pending_links: Vec<(DefinitionRef, Vec<String>)> = Vec::new();
        for part in shard.parts {
            let def = workspace
                .add_part_definition(
                    &scope,
                    part.binding,
                    part.description,
                    part.designator_prefix,
                    part.terminals,
                )
                .ok_or_else(|| GraphSpecError::Parse("missing collection scope".to_owned()))?;
            pending_links.push((def, part.linked_packages));
        }

        for (part_def, bindings) in pending_links {
            for binding in bindings {
                if let Some((_, package_def)) = package_bindings.iter().find(|(pkg_binding, _)| *pkg_binding == binding) {
                    let _ = workspace.link_part_to_package(&part_def, package_def);
                }
            }
        }
    }

    Ok(workspace)
}

pub fn save_workspace(root_path: &Path, workspace: &DesignWorkspace) -> Result<SaveReport, GraphSpecError> {
    let bundle_dir = bundle_dir_for(root_path);
    let defs_dir = bundle_dir.join("defs");
    fs::create_dir_all(&defs_dir)?;

    let mut updated_files = Vec::new();
    let manifest_rel = format!(
        "{}.d/manifest.json",
        root_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("design.graph-spec")
    );
    let root_text = format!(
        "design \"{}\" {{\n  manifest \"{}\"\n}}\n",
        workspace.design_name, manifest_rel
    );
    write_if_changed(root_path, &root_text, &mut updated_files)?;

    let mut manifest = BundleManifest {
        version: 1,
        design_name: workspace.design_name.clone(),
        definition_collections: Vec::new(),
    };

    for collection in workspace.collections.values() {
        let slug = slugify(&collection.title);
        let rel_path = format!("defs/{slug}.json");
        let shard_path = bundle_dir.join(&rel_path);
        let shard = CollectionShard {
            title: collection.title.clone(),
            parts: collection
                .parts
                .iter()
                .filter_map(|part_ref| workspace.parts.get(part_ref))
                .map(|part| PartRecord {
                    binding: part.binding.clone(),
                    description: part.description.clone(),
                    designator_prefix: part.designator_prefix.clone(),
                    terminals: part.terminals.iter().map(|t| t.name.clone()).collect(),
                    linked_packages: part
                        .linked_packages
                        .iter()
                        .filter_map(|pkg| workspace.packages.get(pkg))
                        .map(|pkg| pkg.binding.clone())
                        .collect(),
                })
                .collect(),
            packages: collection
                .packages
                .iter()
                .filter_map(|package_ref| workspace.packages.get(package_ref))
                .map(|package| PackageRecord {
                    binding: package.binding.clone(),
                    description: package.description.clone(),
                    pads: package.pads.iter().map(|pad| pad.name.clone()).collect(),
                })
                .collect(),
        };
        let shard_text = serde_json::to_string_pretty(&shard)?;
        write_if_changed(&shard_path, &shard_text, &mut updated_files)?;
        manifest.definition_collections.push(CollectionFile {
            scope: collection.scope.0.clone(),
            path: rel_path,
            title: collection.title.clone(),
        });
    }

    let manifest_text = serde_json::to_string_pretty(&manifest)?;
    write_if_changed(&bundle_dir.join("manifest.json"), &manifest_text, &mut updated_files)?;

    Ok(SaveReport {
        root_path: root_path.to_path_buf(),
        updated_files,
    })
}

pub fn validate_workspace(root_path: &Path) -> Result<(), GraphSpecError> {
    let _ = load_workspace(root_path)?;
    Ok(())
}

fn bundle_dir_for(root_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.d", root_path.display()))
}

fn parse_manifest_path(source: &str) -> Result<String, GraphSpecError> {
    let manifest_line = source
        .lines()
        .find(|line| line.trim_start().starts_with("manifest "))
        .ok_or_else(|| GraphSpecError::Parse("missing manifest line".to_owned()))?;
    extract_quoted(manifest_line).ok_or_else(|| GraphSpecError::Parse("invalid manifest line".to_owned()))
}

fn extract_quoted(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let tail = &line[start + 1..];
    let end = tail.find('"')?;
    Some(tail[..end].to_owned())
}

fn slugify(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_owned()
}

fn write_if_changed(
    path: &Path,
    content: &str,
    updated_files: &mut Vec<PathBuf>,
) -> Result<(), GraphSpecError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let current = fs::read_to_string(path).ok();
    if current.as_deref() == Some(content) {
        return Ok(());
    }
    fs::write(path, content)?;
    updated_files.push(path.to_path_buf());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_save_and_reload_workspace() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().join("demo.graph-spec");
        let mut workspace = create_workspace_bundle(&root, "Demo").expect("create");
        let collection = workspace
            .collections
            .keys()
            .next()
            .cloned()
            .expect("collection");
        let part = workspace
            .add_part_definition(
                &collection,
                "R_0603",
                Some("Resistor".to_owned()),
                Some("R".to_owned()),
                vec!["1".to_owned(), "2".to_owned()],
            )
            .expect("part");
        let package = workspace
            .add_package_definition(
                &collection,
                "R_0603_1608M",
                Some("Footprint".to_owned()),
                vec!["1".to_owned(), "2".to_owned()],
            )
            .expect("package");
        let _ = workspace.link_part_to_package(&part, &package);
        save_workspace(&root, &workspace).expect("save");

        let reloaded = load_workspace(&root).expect("reload");
        assert_eq!(reloaded.collections.len(), 1);
        assert_eq!(reloaded.parts.len(), 1);
        assert_eq!(reloaded.packages.len(), 1);
    }
}

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

macro_rules! id_type {
    ($name:ident) => {
        #[derive(
            Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
        )]
        pub struct $name(pub String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }
    };
}

id_type!(WorkspaceRef);
id_type!(GraphRootRef);
id_type!(ScopeRef);
id_type!(NodeRef);
id_type!(DocumentRef);
id_type!(DefinitionRef);
id_type!(OccurrenceRef);
id_type!(TerminalRef);
id_type!(ConnectionRef);
id_type!(GeometryRef);
id_type!(ConstraintRef);
id_type!(AssetRef);
id_type!(ImportRef);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct InstancePath {
    pub occurrences: Vec<OccurrenceRef>,
}

impl InstancePath {
    pub fn new(occurrences: Vec<OccurrenceRef>) -> Self {
        Self { occurrences }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScopeKind {
    Design,
    LogicalDocument,
    PhysicalDocument,
    DefinitionCollection,
    PartDefinition,
    PackageDefinition,
    BlockDefinition,
    AssetGroup,
    ImportGroup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    Document,
    Definition,
    Occurrence,
    Terminal,
    Connection,
    Geometry,
    Constraint,
    Asset,
    Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionKind {
    Net,
    Bus,
    Bundle,
    DifferentialPair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactAuthority {
    Semantic,
    Artifact,
    External,
    Opaque,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactStorageKind {
    AuthoredText,
    GeneratedText,
    StructuredBinary,
    OpaqueBinary,
    ExternalReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeSummary {
    pub scope: ScopeRef,
    pub workspace: WorkspaceRef,
    pub kind: ScopeKind,
    pub title: String,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeSummary {
    pub node: NodeRef,
    pub kind: NodeKind,
    pub title: String,
    pub instance_path: Option<InstancePath>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionSummary {
    pub title: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InspectorSummary {
    pub title: String,
    pub subtitle: Option<String>,
    pub identity_rows: Vec<(String, String)>,
    pub relationship_rows: Vec<(String, String)>,
    pub connectivity_rows: Vec<(String, String)>,
    pub artifact_rows: Vec<(String, String)>,
    pub provenance_rows: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetSummary {
    pub asset: AssetRef,
    pub title: String,
    pub authority: ArtifactAuthority,
    pub storage: ArtifactStorageKind,
    pub digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSummary {
    pub import: ImportRef,
    pub title: String,
    pub source_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderShape {
    pub id: String,
    pub label: Option<String>,
    pub target: Option<NodeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalRenderModel {
    pub scope: ScopeRef,
    pub revision: u64,
    pub shapes: Vec<RenderShape>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalRenderModel {
    pub scope: ScopeRef,
    pub revision: u64,
    pub shapes: Vec<RenderShape>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinitionPreviewModel {
    pub scope: ScopeRef,
    pub revision: u64,
    pub title: String,
    pub shapes: Vec<RenderShape>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetPreviewModel {
    pub asset: AssetRef,
    pub revision: u64,
    pub title: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalDefinition {
    pub terminal: TerminalRef,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartDefinition {
    pub definition: DefinitionRef,
    pub node: NodeRef,
    pub scope: ScopeRef,
    pub title: String,
    pub binding: String,
    pub description: Option<String>,
    pub designator_prefix: Option<String>,
    pub terminals: Vec<TerminalDefinition>,
    pub linked_packages: Vec<DefinitionRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageDefinition {
    pub definition: DefinitionRef,
    pub node: NodeRef,
    pub scope: ScopeRef,
    pub title: String,
    pub binding: String,
    pub description: Option<String>,
    pub pads: Vec<TerminalDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinitionCollection {
    pub scope: ScopeRef,
    pub title: String,
    pub revision: u64,
    pub parts: Vec<DefinitionRef>,
    pub packages: Vec<DefinitionRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedSource {
    pub import: ImportRef,
    pub title: String,
    pub source_kind: String,
    pub source_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphAsset {
    pub asset: AssetRef,
    pub title: String,
    pub authority: ArtifactAuthority,
    pub storage: ArtifactStorageKind,
    pub digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignWorkspace {
    pub workspace: WorkspaceRef,
    pub graph_root: GraphRootRef,
    pub design_scope: ScopeRef,
    pub design_name: String,
    pub revision: u64,
    pub collections: BTreeMap<ScopeRef, DefinitionCollection>,
    pub parts: BTreeMap<DefinitionRef, PartDefinition>,
    pub packages: BTreeMap<DefinitionRef, PackageDefinition>,
    pub imports: BTreeMap<ImportRef, ImportedSource>,
    pub assets: BTreeMap<AssetRef, GraphAsset>,
    next_collection_id: u64,
    next_part_id: u64,
    next_package_id: u64,
    next_terminal_id: u64,
    next_import_id: u64,
}

impl DesignWorkspace {
    pub fn new(name: impl Into<String>, root_hint: impl Into<String>) -> Self {
        let root_hint = root_hint.into();
        let design_name = name.into();
        Self {
            workspace: WorkspaceRef::new(format!("workspace:{root_hint}")),
            graph_root: GraphRootRef::new(format!("graph:{root_hint}")),
            design_scope: ScopeRef::new("scope:design"),
            design_name,
            revision: 0,
            collections: BTreeMap::new(),
            parts: BTreeMap::new(),
            packages: BTreeMap::new(),
            imports: BTreeMap::new(),
            assets: BTreeMap::new(),
            next_collection_id: 1,
            next_part_id: 1,
            next_package_id: 1,
            next_terminal_id: 1,
            next_import_id: 1,
        }
    }

    pub fn add_definition_collection(&mut self, title: impl Into<String>) -> ScopeRef {
        let scope = ScopeRef::new(format!("scope:collection:{}", self.next_collection_id));
        self.next_collection_id += 1;
        let collection = DefinitionCollection {
            scope: scope.clone(),
            title: title.into(),
            revision: 0,
            parts: Vec::new(),
            packages: Vec::new(),
        };
        self.collections.insert(scope.clone(), collection);
        self.bump_revision();
        scope
    }

    pub fn add_part_definition(
        &mut self,
        collection_scope: &ScopeRef,
        binding: impl Into<String>,
        description: Option<String>,
        designator_prefix: Option<String>,
        terminals: Vec<String>,
    ) -> Option<DefinitionRef> {
        if !self.collections.contains_key(collection_scope) {
            return None;
        }
        let binding = binding.into();
        let definition = DefinitionRef::new(format!("def:part:{}", self.next_part_id));
        let node = NodeRef::new(format!("node:part:{}", self.next_part_id));
        let scope = ScopeRef::new(format!("scope:part:{}", self.next_part_id));
        self.next_part_id += 1;
        let terminal_defs = terminals
            .into_iter()
            .map(|name| TerminalDefinition {
                terminal: self.allocate_terminal_ref(),
                name,
            })
            .collect();
        let part = PartDefinition {
            definition: definition.clone(),
            node,
            scope,
            title: binding.clone(),
            binding,
            description,
            designator_prefix,
            terminals: terminal_defs,
            linked_packages: Vec::new(),
        };
        let collection = self.collections.get_mut(collection_scope)?;
        collection.parts.push(definition.clone());
        collection.revision = collection.revision.saturating_add(1);
        self.parts.insert(definition.clone(), part);
        self.bump_revision();
        Some(definition)
    }

    pub fn add_package_definition(
        &mut self,
        collection_scope: &ScopeRef,
        binding: impl Into<String>,
        description: Option<String>,
        pads: Vec<String>,
    ) -> Option<DefinitionRef> {
        if !self.collections.contains_key(collection_scope) {
            return None;
        }
        let binding = binding.into();
        let definition = DefinitionRef::new(format!("def:pkg:{}", self.next_package_id));
        let node = NodeRef::new(format!("node:pkg:{}", self.next_package_id));
        let scope = ScopeRef::new(format!("scope:package:{}", self.next_package_id));
        self.next_package_id += 1;
        let pad_defs = pads
            .into_iter()
            .map(|name| TerminalDefinition {
                terminal: self.allocate_terminal_ref(),
                name,
            })
            .collect();
        let package = PackageDefinition {
            definition: definition.clone(),
            node,
            scope,
            title: binding.clone(),
            binding,
            description,
            pads: pad_defs,
        };
        let collection = self.collections.get_mut(collection_scope)?;
        collection.packages.push(definition.clone());
        collection.revision = collection.revision.saturating_add(1);
        self.packages.insert(definition.clone(), package);
        self.bump_revision();
        Some(definition)
    }

    pub fn link_part_to_package(
        &mut self,
        part: &DefinitionRef,
        package: &DefinitionRef,
    ) -> bool {
        let Some(part_def) = self.parts.get_mut(part) else {
            return false;
        };
        if !self.packages.contains_key(package) {
            return false;
        }
        if !part_def.linked_packages.contains(package) {
            part_def.linked_packages.push(package.clone());
            self.bump_revision();
        }
        true
    }

    pub fn add_import(
        &mut self,
        title: impl Into<String>,
        source_kind: impl Into<String>,
        source_path: impl Into<String>,
    ) -> ImportRef {
        let import = ImportRef::new(format!("import:{}", self.next_import_id));
        self.next_import_id += 1;
        self.imports.insert(
            import.clone(),
            ImportedSource {
                import: import.clone(),
                title: title.into(),
                source_kind: source_kind.into(),
                source_paths: vec![source_path.into()],
            },
        );
        self.bump_revision();
        import
    }

    fn allocate_terminal_ref(&mut self) -> TerminalRef {
        let terminal = TerminalRef::new(format!("terminal:{}", self.next_terminal_id));
        self.next_terminal_id += 1;
        terminal
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }

    pub fn scope_by_definition(&self, scope: &ScopeRef) -> Option<(ScopeKind, String)> {
        if scope == &self.design_scope {
            return Some((ScopeKind::Design, self.design_name.clone()));
        }
        if let Some(collection) = self.collections.get(scope) {
            return Some((ScopeKind::DefinitionCollection, collection.title.clone()));
        }
        if let Some(part) = self.parts.values().find(|part| &part.scope == scope) {
            return Some((ScopeKind::PartDefinition, part.title.clone()));
        }
        if let Some(package) = self.packages.values().find(|package| &package.scope == scope) {
            return Some((ScopeKind::PackageDefinition, package.title.clone()));
        }
        None
    }
}

pub trait GraphWorkspace {
    fn workspace_ref(&self) -> &WorkspaceRef;
    fn graph_root_ref(&self) -> &GraphRootRef;
    fn openable_scopes(&self) -> Vec<ScopeSummary>;
    fn scope_summary(&self, scope: &ScopeRef) -> Option<ScopeSummary>;
}

pub trait GraphRead {
    fn node_summary(&self, node: &NodeRef) -> Option<NodeSummary>;
    fn inspector_summary_for_node(
        &self,
        node: &NodeRef,
        instance_path: Option<&InstancePath>,
    ) -> Option<InspectorSummary>;
    fn inspector_summary_for_scope(&self, scope: &ScopeRef) -> Option<InspectorSummary>;
    fn asset_summary(&self, asset: &AssetRef) -> Option<AssetSummary>;
    fn import_summary(&self, import: &ImportRef) -> Option<ImportSummary>;
}

pub trait GraphWrite {}

pub trait RenderAdapterHost {
    fn logical_render_model(&self, scope: &ScopeRef) -> Option<LogicalRenderModel>;
    fn physical_render_model(&self, scope: &ScopeRef) -> Option<PhysicalRenderModel>;
    fn definition_preview_model(&self, scope: &ScopeRef) -> Option<DefinitionPreviewModel>;
    fn asset_preview_model(&self, asset: &AssetRef) -> Option<AssetPreviewModel>;
}

impl GraphWorkspace for DesignWorkspace {
    fn workspace_ref(&self) -> &WorkspaceRef {
        &self.workspace
    }

    fn graph_root_ref(&self) -> &GraphRootRef {
        &self.graph_root
    }

    fn openable_scopes(&self) -> Vec<ScopeSummary> {
        let mut out = vec![ScopeSummary {
            scope: self.design_scope.clone(),
            workspace: self.workspace.clone(),
            kind: ScopeKind::Design,
            title: self.design_name.clone(),
            revision: self.revision,
        }];
        for collection in self.collections.values() {
            out.push(ScopeSummary {
                scope: collection.scope.clone(),
                workspace: self.workspace.clone(),
                kind: ScopeKind::DefinitionCollection,
                title: collection.title.clone(),
                revision: collection.revision,
            });
        }
        for part in self.parts.values() {
            out.push(ScopeSummary {
                scope: part.scope.clone(),
                workspace: self.workspace.clone(),
                kind: ScopeKind::PartDefinition,
                title: part.title.clone(),
                revision: self.revision,
            });
        }
        for package in self.packages.values() {
            out.push(ScopeSummary {
                scope: package.scope.clone(),
                workspace: self.workspace.clone(),
                kind: ScopeKind::PackageDefinition,
                title: package.title.clone(),
                revision: self.revision,
            });
        }
        out
    }

    fn scope_summary(&self, scope: &ScopeRef) -> Option<ScopeSummary> {
        let (kind, title) = self.scope_by_definition(scope)?;
        Some(ScopeSummary {
            scope: scope.clone(),
            workspace: self.workspace.clone(),
            kind,
            title,
            revision: self.revision,
        })
    }
}

impl GraphRead for DesignWorkspace {
    fn node_summary(&self, node: &NodeRef) -> Option<NodeSummary> {
        if let Some(part) = self.parts.values().find(|part| &part.node == node) {
            return Some(NodeSummary {
                node: node.clone(),
                kind: NodeKind::Definition,
                title: part.title.clone(),
                instance_path: None,
            });
        }
        if let Some(package) = self.packages.values().find(|package| &package.node == node) {
            return Some(NodeSummary {
                node: node.clone(),
                kind: NodeKind::Definition,
                title: package.title.clone(),
                instance_path: None,
            });
        }
        None
    }

    fn inspector_summary_for_node(
        &self,
        node: &NodeRef,
        instance_path: Option<&InstancePath>,
    ) -> Option<InspectorSummary> {
        if let Some(part) = self.parts.values().find(|part| &part.node == node) {
            return Some(InspectorSummary {
                title: part.title.clone(),
                subtitle: Some("Part Definition".to_owned()),
                identity_rows: vec![
                    ("Binding".to_owned(), part.binding.clone()),
                    ("Definition".to_owned(), part.definition.0.clone()),
                ],
                relationship_rows: vec![(
                    "Linked packages".to_owned(),
                    part.linked_packages
                        .iter()
                        .map(|pkg| pkg.0.clone())
                        .collect::<Vec<_>>()
                        .join(", "),
                )],
                connectivity_rows: part
                    .terminals
                    .iter()
                    .map(|terminal| ("Terminal".to_owned(), terminal.name.clone()))
                    .collect(),
                artifact_rows: Vec::new(),
                provenance_rows: vec![(
                    "Instance path".to_owned(),
                    instance_path
                        .map(|path| {
                            path.occurrences
                                .iter()
                                .map(|occ| occ.0.clone())
                                .collect::<Vec<_>>()
                                .join(" / ")
                        })
                        .unwrap_or_else(|| "n/a".to_owned()),
                )],
            });
        }
        if let Some(package) = self.packages.values().find(|package| &package.node == node) {
            return Some(InspectorSummary {
                title: package.title.clone(),
                subtitle: Some("Package Definition".to_owned()),
                identity_rows: vec![
                    ("Binding".to_owned(), package.binding.clone()),
                    ("Definition".to_owned(), package.definition.0.clone()),
                ],
                relationship_rows: Vec::new(),
                connectivity_rows: package
                    .pads
                    .iter()
                    .map(|pad| ("Pad".to_owned(), pad.name.clone()))
                    .collect(),
                artifact_rows: Vec::new(),
                provenance_rows: Vec::new(),
            });
        }
        None
    }

    fn inspector_summary_for_scope(&self, scope: &ScopeRef) -> Option<InspectorSummary> {
        if scope == &self.design_scope {
            return Some(InspectorSummary {
                title: self.design_name.clone(),
                subtitle: Some("Design Workspace".to_owned()),
                identity_rows: vec![
                    ("Workspace".to_owned(), self.workspace.0.clone()),
                    ("Root".to_owned(), self.graph_root.0.clone()),
                ],
                relationship_rows: vec![
                    (
                        "Collections".to_owned(),
                        self.collections.len().to_string(),
                    ),
                    ("Parts".to_owned(), self.parts.len().to_string()),
                    ("Packages".to_owned(), self.packages.len().to_string()),
                ],
                connectivity_rows: Vec::new(),
                artifact_rows: vec![
                    ("Imports".to_owned(), self.imports.len().to_string()),
                    ("Assets".to_owned(), self.assets.len().to_string()),
                ],
                provenance_rows: Vec::new(),
            });
        }
        if let Some(collection) = self.collections.get(scope) {
            return Some(InspectorSummary {
                title: collection.title.clone(),
                subtitle: Some("Definition Collection".to_owned()),
                identity_rows: vec![("Scope".to_owned(), collection.scope.0.clone())],
                relationship_rows: vec![
                    ("Parts".to_owned(), collection.parts.len().to_string()),
                    ("Packages".to_owned(), collection.packages.len().to_string()),
                ],
                connectivity_rows: Vec::new(),
                artifact_rows: Vec::new(),
                provenance_rows: Vec::new(),
            });
        }
        None
    }

    fn asset_summary(&self, asset: &AssetRef) -> Option<AssetSummary> {
        self.assets.get(asset).map(|asset| AssetSummary {
            asset: asset.asset.clone(),
            title: asset.title.clone(),
            authority: asset.authority,
            storage: asset.storage,
            digest: asset.digest.clone(),
        })
    }

    fn import_summary(&self, import: &ImportRef) -> Option<ImportSummary> {
        self.imports.get(import).map(|import| ImportSummary {
            import: import.import.clone(),
            title: import.title.clone(),
            source_kind: import.source_kind.clone(),
        })
    }
}

impl RenderAdapterHost for DesignWorkspace {
    fn logical_render_model(&self, scope: &ScopeRef) -> Option<LogicalRenderModel> {
        if scope == &self.design_scope {
            return Some(LogicalRenderModel {
                scope: scope.clone(),
                revision: self.revision,
                shapes: self
                    .collections
                    .values()
                    .map(|collection| RenderShape {
                        id: collection.scope.0.clone(),
                        label: Some(collection.title.clone()),
                        target: None,
                    })
                    .collect(),
                warnings: Vec::new(),
            });
        }
        let collection = self.collections.get(scope)?;
        let mut shapes = Vec::new();
        for part_ref in &collection.parts {
            if let Some(part) = self.parts.get(part_ref) {
                shapes.push(RenderShape {
                    id: part.definition.0.clone(),
                    label: Some(format!("part {}", part.binding)),
                    target: Some(part.node.clone()),
                });
            }
        }
        Some(LogicalRenderModel {
            scope: scope.clone(),
            revision: collection.revision,
            shapes,
            warnings: Vec::new(),
        })
    }

    fn physical_render_model(&self, scope: &ScopeRef) -> Option<PhysicalRenderModel> {
        let collection = self.collections.get(scope)?;
        let mut shapes = Vec::new();
        for package_ref in &collection.packages {
            if let Some(package) = self.packages.get(package_ref) {
                shapes.push(RenderShape {
                    id: package.definition.0.clone(),
                    label: Some(format!("package {}", package.binding)),
                    target: Some(package.node.clone()),
                });
            }
        }
        Some(PhysicalRenderModel {
            scope: scope.clone(),
            revision: collection.revision,
            shapes,
            warnings: Vec::new(),
        })
    }

    fn definition_preview_model(&self, scope: &ScopeRef) -> Option<DefinitionPreviewModel> {
        if let Some(part) = self.parts.values().find(|part| &part.scope == scope) {
            return Some(DefinitionPreviewModel {
                scope: scope.clone(),
                revision: self.revision,
                title: part.title.clone(),
                shapes: part
                    .terminals
                    .iter()
                    .map(|terminal| RenderShape {
                        id: terminal.terminal.0.clone(),
                        label: Some(format!("terminal {}", terminal.name)),
                        target: Some(part.node.clone()),
                    })
                    .collect(),
            });
        }
        if let Some(package) = self.packages.values().find(|package| &package.scope == scope) {
            return Some(DefinitionPreviewModel {
                scope: scope.clone(),
                revision: self.revision,
                title: package.title.clone(),
                shapes: package
                    .pads
                    .iter()
                    .map(|pad| RenderShape {
                        id: pad.terminal.0.clone(),
                        label: Some(format!("pad {}", pad.name)),
                        target: Some(package.node.clone()),
                    })
                    .collect(),
            });
        }
        None
    }

    fn asset_preview_model(&self, asset: &AssetRef) -> Option<AssetPreviewModel> {
        self.assets.get(asset).map(|asset| AssetPreviewModel {
            asset: asset.asset.clone(),
            revision: self.revision,
            title: asset.title.clone(),
            warnings: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_refs_roundtrip() {
        let node = NodeRef::new("node-1");
        let raw = serde_json::to_string(&node).expect("serialize");
        let parsed: NodeRef = serde_json::from_str(&raw).expect("deserialize");
        assert_eq!(parsed, node);
    }

    #[test]
    fn instance_path_roundtrip() {
        let path = InstancePath::new(vec![OccurrenceRef::new("occ-a"), OccurrenceRef::new("occ-b")]);
        let raw = serde_json::to_string(&path).expect("serialize");
        let parsed: InstancePath = serde_json::from_str(&raw).expect("deserialize");
        assert_eq!(parsed, path);
    }

    #[test]
    fn workspace_can_hold_parts_and_packages() {
        let mut ws = DesignWorkspace::new("Demo", "demo");
        let collection = ws.add_definition_collection("Core");
        let part = ws
            .add_part_definition(
                &collection,
                "R_0603",
                Some("Resistor".to_owned()),
                Some("R".to_owned()),
                vec!["1".to_owned(), "2".to_owned()],
            )
            .expect("part");
        let package = ws
            .add_package_definition(
                &collection,
                "R_0603_1608M",
                Some("Metric resistor".to_owned()),
                vec!["1".to_owned(), "2".to_owned()],
            )
            .expect("package");
        assert!(ws.link_part_to_package(&part, &package));
        assert_eq!(ws.collections[&collection].parts.len(), 1);
        assert_eq!(ws.collections[&collection].packages.len(), 1);
    }
}

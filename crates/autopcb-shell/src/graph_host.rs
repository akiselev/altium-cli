use std::collections::BTreeMap;
use std::path::Path;

use autopcb_graph::{
    ArtifactAuthority, ArtifactStorageKind, AssetPreviewModel, AssetRef, AssetSummary,
    DefinitionPreviewModel, GraphRead, GraphRootRef, GraphWorkspace, ImportRef, ImportSummary,
    InspectorSummary, LogicalRenderModel, NodeRef, NodeSummary, PhysicalRenderModel, RenderAdapterHost,
    RenderShape, ScopeKind, ScopeRef, ScopeSummary, WorkspaceRef,
};

#[derive(Debug, Clone)]
pub struct GraphHost {
    workspace: WorkspaceRef,
    root: GraphRootRef,
    scopes: BTreeMap<ScopeRef, ScopeSummary>,
    nodes: BTreeMap<NodeRef, NodeSummary>,
    assets: BTreeMap<AssetRef, AssetSummary>,
    imports: BTreeMap<ImportRef, ImportSummary>,
}

impl GraphHost {
    pub fn stub_from_root(root: impl Into<String>) -> Self {
        let root = root.into();
        let workspace = WorkspaceRef::new(format!("workspace:{root}"));
        let graph_root = GraphRootRef::new(format!("graph:{root}"));

        let design_scope = ScopeSummary {
            scope: ScopeRef::new("scope:design"),
            workspace: workspace.clone(),
            kind: ScopeKind::Design,
            title: "Design Overview".to_owned(),
            revision: 0,
        };
        let logical_scope = ScopeSummary {
            scope: ScopeRef::new("scope:logical"),
            workspace: workspace.clone(),
            kind: ScopeKind::LogicalDocument,
            title: "Logical Document".to_owned(),
            revision: 0,
        };
        let physical_scope = ScopeSummary {
            scope: ScopeRef::new("scope:physical"),
            workspace: workspace.clone(),
            kind: ScopeKind::PhysicalDocument,
            title: "Physical Document".to_owned(),
            revision: 0,
        };

        let mut scopes = BTreeMap::new();
        for summary in [design_scope, logical_scope, physical_scope] {
            scopes.insert(summary.scope.clone(), summary);
        }

        let root_node = NodeSummary {
            node: NodeRef::new("node:design-root"),
            kind: autopcb_graph::NodeKind::Document,
            title: "Design Root".to_owned(),
            instance_path: None,
        };

        let mut nodes = BTreeMap::new();
        nodes.insert(root_node.node.clone(), root_node);

        Self {
            workspace,
            root: graph_root,
            scopes,
            nodes,
            assets: BTreeMap::new(),
            imports: BTreeMap::new(),
        }
    }

    pub fn stub_from_path(path: &Path) -> Self {
        Self::stub_from_root(path.display().to_string())
    }
}

impl GraphWorkspace for GraphHost {
    fn workspace_ref(&self) -> &WorkspaceRef {
        &self.workspace
    }

    fn graph_root_ref(&self) -> &GraphRootRef {
        &self.root
    }

    fn openable_scopes(&self) -> Vec<ScopeSummary> {
        self.scopes.values().cloned().collect()
    }

    fn scope_summary(&self, scope: &ScopeRef) -> Option<ScopeSummary> {
        self.scopes.get(scope).cloned()
    }
}

impl GraphRead for GraphHost {
    fn node_summary(&self, node: &NodeRef) -> Option<NodeSummary> {
        self.nodes.get(node).cloned()
    }

    fn inspector_summary_for_node(
        &self,
        node: &NodeRef,
        instance_path: Option<&autopcb_graph::InstancePath>,
    ) -> Option<InspectorSummary> {
        let summary = self.nodes.get(node)?;
        Some(InspectorSummary {
            title: summary.title.clone(),
            subtitle: Some(format!("{:?}", summary.kind)),
            identity_rows: vec![
                ("Node Ref".to_owned(), summary.node.0.clone()),
                ("Workspace".to_owned(), self.workspace.0.clone()),
                (
                    "Instance Path".to_owned(),
                    instance_path
                        .map(|p| p.occurrences.iter().map(|occ| occ.0.as_str()).collect::<Vec<_>>().join(" / "))
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| "n/a".to_owned()),
                ),
            ],
            relationship_rows: Vec::new(),
            connectivity_rows: Vec::new(),
            artifact_rows: Vec::new(),
            provenance_rows: vec![("Graph Root".to_owned(), self.root.0.clone())],
        })
    }

    fn inspector_summary_for_scope(&self, scope: &ScopeRef) -> Option<InspectorSummary> {
        let summary = self.scopes.get(scope)?;
        Some(InspectorSummary {
            title: summary.title.clone(),
            subtitle: Some(format!("{:?}", summary.kind)),
            identity_rows: vec![
                ("Scope Ref".to_owned(), summary.scope.0.clone()),
                ("Workspace".to_owned(), summary.workspace.0.clone()),
                ("Revision".to_owned(), summary.revision.to_string()),
            ],
            relationship_rows: Vec::new(),
            connectivity_rows: Vec::new(),
            artifact_rows: Vec::new(),
            provenance_rows: vec![("Graph Root".to_owned(), self.root.0.clone())],
        })
    }

    fn asset_summary(&self, asset: &AssetRef) -> Option<AssetSummary> {
        self.assets.get(asset).cloned()
    }

    fn import_summary(&self, import: &ImportRef) -> Option<ImportSummary> {
        self.imports.get(import).cloned()
    }
}

impl RenderAdapterHost for GraphHost {
    fn logical_render_model(&self, scope: &ScopeRef) -> Option<LogicalRenderModel> {
        let scope_summary = self.scopes.get(scope)?;
        Some(LogicalRenderModel {
            scope: scope.clone(),
            revision: scope_summary.revision,
            shapes: vec![RenderShape {
                id: "logical-placeholder".to_owned(),
                label: Some(scope_summary.title.clone()),
                target: None,
            }],
            warnings: vec!["Logical render adapter is using placeholder graph content.".to_owned()],
        })
    }

    fn physical_render_model(&self, scope: &ScopeRef) -> Option<PhysicalRenderModel> {
        let scope_summary = self.scopes.get(scope)?;
        Some(PhysicalRenderModel {
            scope: scope.clone(),
            revision: scope_summary.revision,
            shapes: vec![RenderShape {
                id: "physical-placeholder".to_owned(),
                label: Some(scope_summary.title.clone()),
                target: None,
            }],
            warnings: vec!["Physical render adapter is using placeholder graph content.".to_owned()],
        })
    }

    fn definition_preview_model(&self, scope: &ScopeRef) -> Option<DefinitionPreviewModel> {
        let scope_summary = self.scopes.get(scope)?;
        Some(DefinitionPreviewModel {
            scope: scope.clone(),
            revision: scope_summary.revision,
            title: scope_summary.title.clone(),
            shapes: vec![RenderShape {
                id: "definition-placeholder".to_owned(),
                label: Some(scope_summary.title.clone()),
                target: None,
            }],
        })
    }

    fn asset_preview_model(&self, asset: &AssetRef) -> Option<AssetPreviewModel> {
        let summary = self.assets.get(asset)?;
        Some(AssetPreviewModel {
            asset: asset.clone(),
            revision: 0,
            title: summary.title.clone(),
            warnings: vec![format!(
                "Asset preview placeholder ({:?}, {:?})",
                summary.authority, summary.storage
            )],
        })
    }
}

impl Default for GraphHost {
    fn default() -> Self {
        let mut host = Self::stub_from_root("default");
        host.assets.insert(
            AssetRef::new("asset:placeholder"),
            AssetSummary {
                asset: AssetRef::new("asset:placeholder"),
                title: "Placeholder Asset".to_owned(),
                authority: ArtifactAuthority::Opaque,
                storage: ArtifactStorageKind::OpaqueBinary,
                digest: None,
            },
        );
        host.imports.insert(
            ImportRef::new("import:placeholder"),
            ImportSummary {
                import: ImportRef::new("import:placeholder"),
                title: "Placeholder Import".to_owned(),
                source_kind: "stub".to_owned(),
            },
        );
        host
    }
}

use std::path::{Path, PathBuf};

use autopcb_graph::{
    AssetPreviewModel, AssetRef, AssetSummary, DefinitionPreviewModel, DesignWorkspace, GraphRead,
    GraphRootRef, GraphWorkspace, ImportRef, ImportSummary, InspectorSummary, LogicalRenderModel,
    NodeRef, NodeSummary, PhysicalRenderModel, RenderAdapterHost, ScopeRef, ScopeSummary,
    WorkspaceRef,
};
use autopcb_graph_spec::{GraphSpecError, load_workspace, save_workspace};

#[derive(Debug, Clone)]
pub struct GraphHost {
    pub root_path: Option<PathBuf>,
    pub workspace: DesignWorkspace,
}

impl GraphHost {
    pub fn new(root_path: Option<PathBuf>, workspace: DesignWorkspace) -> Self {
        Self {
            root_path,
            workspace,
        }
    }

    pub fn stub_from_root(root: impl Into<String>) -> Self {
        let root = root.into();
        Self::new(None, DesignWorkspace::new("Untitled Design", root))
    }

    pub fn stub_from_path(path: &Path) -> Self {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("workspace");
        Self::new(Some(path.to_path_buf()), DesignWorkspace::new(name, path.display().to_string()))
    }

    pub fn load_from_path(path: &Path) -> Result<Self, GraphSpecError> {
        let workspace = load_workspace(path)?;
        Ok(Self::new(Some(path.to_path_buf()), workspace))
    }

    pub fn save(&self) -> Result<(), GraphSpecError> {
        let Some(path) = self.root_path.as_ref() else {
            return Err(GraphSpecError::Parse(
                "graph workspace has no root path".to_owned(),
            ));
        };
        let _ = save_workspace(path, &self.workspace)?;
        Ok(())
    }
}

impl GraphWorkspace for GraphHost {
    fn workspace_ref(&self) -> &WorkspaceRef {
        self.workspace.workspace_ref()
    }

    fn graph_root_ref(&self) -> &GraphRootRef {
        self.workspace.graph_root_ref()
    }

    fn openable_scopes(&self) -> Vec<ScopeSummary> {
        self.workspace.openable_scopes()
    }

    fn scope_summary(&self, scope: &ScopeRef) -> Option<ScopeSummary> {
        self.workspace.scope_summary(scope)
    }
}

impl GraphRead for GraphHost {
    fn node_summary(&self, node: &NodeRef) -> Option<NodeSummary> {
        self.workspace.node_summary(node)
    }

    fn inspector_summary_for_node(
        &self,
        node: &NodeRef,
        instance_path: Option<&autopcb_graph::InstancePath>,
    ) -> Option<InspectorSummary> {
        self.workspace.inspector_summary_for_node(node, instance_path)
    }

    fn inspector_summary_for_scope(&self, scope: &ScopeRef) -> Option<InspectorSummary> {
        self.workspace.inspector_summary_for_scope(scope)
    }

    fn asset_summary(&self, asset: &AssetRef) -> Option<AssetSummary> {
        self.workspace.asset_summary(asset)
    }

    fn import_summary(&self, import: &ImportRef) -> Option<ImportSummary> {
        self.workspace.import_summary(import)
    }
}

impl RenderAdapterHost for GraphHost {
    fn logical_render_model(&self, scope: &ScopeRef) -> Option<LogicalRenderModel> {
        self.workspace.logical_render_model(scope)
    }

    fn physical_render_model(&self, scope: &ScopeRef) -> Option<PhysicalRenderModel> {
        self.workspace.physical_render_model(scope)
    }

    fn definition_preview_model(&self, scope: &ScopeRef) -> Option<DefinitionPreviewModel> {
        self.workspace.definition_preview_model(scope)
    }

    fn asset_preview_model(&self, asset: &AssetRef) -> Option<AssetPreviewModel> {
        self.workspace.asset_preview_model(asset)
    }
}

impl Default for GraphHost {
    fn default() -> Self {
        Self::stub_from_root("default")
    }
}

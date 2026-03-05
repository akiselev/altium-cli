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
}

use std::collections::BTreeMap;

use efame::egui;

use crate::workbench::{
    DOCUMENT_KIND_ASSET, DOCUMENT_KIND_BOARD, DOCUMENT_KIND_DEFINITION_COLLECTION,
    DOCUMENT_KIND_DESIGN_OVERVIEW, DOCUMENT_KIND_IMPORT, DOCUMENT_KIND_KEYBINDINGS,
    DOCUMENT_KIND_LOGICAL, DOCUMENT_KIND_PHYSICAL, DOCUMENT_KIND_SCHDOC_PREVIEW,
    DOCUMENT_KIND_SCHLIB_COMPONENT, DOCUMENT_KIND_SCHLIB_GALLERY, DOCUMENT_KIND_SPEC, DocumentId,
};

use super::ShellApp;

pub trait TabRenderer {
    fn render(
        &mut self,
        app: &mut ShellApp,
        ui: &mut egui::Ui,
        document_id: DocumentId,
        fit_requested: bool,
    );
}

type TabFactory = fn() -> Box<dyn TabRenderer>;

pub struct TabProviderRegistry {
    factories: BTreeMap<&'static str, TabFactory>,
}

impl TabProviderRegistry {
    pub fn new_m1() -> Self {
        let mut registry = Self {
            factories: BTreeMap::new(),
        };

        registry.register(DOCUMENT_KIND_BOARD, || Box::new(BoardTabRenderer));
        registry.register(DOCUMENT_KIND_SPEC, || Box::new(SpecTabRenderer));
        registry.register(DOCUMENT_KIND_SCHDOC_PREVIEW, || {
            Box::new(SchDocPreviewTabRenderer)
        });
        registry.register(DOCUMENT_KIND_SCHLIB_GALLERY, || {
            Box::new(SchLibGalleryTabRenderer)
        });
        registry.register(DOCUMENT_KIND_SCHLIB_COMPONENT, || {
            Box::new(SchLibComponentTabRenderer)
        });
        registry.register(DOCUMENT_KIND_DESIGN_OVERVIEW, || {
            Box::new(DesignOverviewTabRenderer)
        });
        registry.register(DOCUMENT_KIND_LOGICAL, || Box::new(LogicalTabRenderer));
        registry.register(DOCUMENT_KIND_PHYSICAL, || Box::new(PhysicalTabRenderer));
        registry.register(DOCUMENT_KIND_DEFINITION_COLLECTION, || {
            Box::new(DefinitionCollectionTabRenderer)
        });
        registry.register(DOCUMENT_KIND_ASSET, || Box::new(AssetTabRenderer));
        registry.register(DOCUMENT_KIND_IMPORT, || Box::new(ImportTabRenderer));
        registry.register(DOCUMENT_KIND_KEYBINDINGS, || {
            Box::new(KeybindingsTabRenderer)
        });
        registry
    }

    pub fn register(&mut self, kind_id: &'static str, factory: TabFactory) {
        self.factories.insert(kind_id, factory);
    }

    pub fn instantiate(&self, kind_id: &str) -> Option<Box<dyn TabRenderer>> {
        self.factories.get(kind_id).map(|factory| factory())
    }
}

struct BoardTabRenderer;
struct SpecTabRenderer;
struct SchDocPreviewTabRenderer;
struct SchLibGalleryTabRenderer;
struct SchLibComponentTabRenderer;
struct DesignOverviewTabRenderer;
struct LogicalTabRenderer;
struct PhysicalTabRenderer;
struct DefinitionCollectionTabRenderer;
struct AssetTabRenderer;
struct ImportTabRenderer;
struct KeybindingsTabRenderer;

impl TabRenderer for BoardTabRenderer {
    fn render(
        &mut self,
        app: &mut ShellApp,
        ui: &mut egui::Ui,
        document_id: DocumentId,
        fit_requested: bool,
    ) {
        app.render_board_document(ui, document_id, fit_requested);
    }
}

impl TabRenderer for SpecTabRenderer {
    fn render(
        &mut self,
        app: &mut ShellApp,
        ui: &mut egui::Ui,
        document_id: DocumentId,
        _fit_requested: bool,
    ) {
        app.render_spec_document(ui, document_id);
    }
}

impl TabRenderer for SchDocPreviewTabRenderer {
    fn render(
        &mut self,
        app: &mut ShellApp,
        ui: &mut egui::Ui,
        document_id: DocumentId,
        _fit_requested: bool,
    ) {
        app.render_schdoc_preview_document(ui, document_id);
    }
}

impl TabRenderer for SchLibGalleryTabRenderer {
    fn render(
        &mut self,
        app: &mut ShellApp,
        ui: &mut egui::Ui,
        document_id: DocumentId,
        _fit_requested: bool,
    ) {
        app.render_schlib_gallery_document(ui, document_id);
    }
}

impl TabRenderer for SchLibComponentTabRenderer {
    fn render(
        &mut self,
        app: &mut ShellApp,
        ui: &mut egui::Ui,
        document_id: DocumentId,
        _fit_requested: bool,
    ) {
        app.render_schlib_component_document(ui, document_id);
    }
}

impl TabRenderer for DesignOverviewTabRenderer {
    fn render(
        &mut self,
        app: &mut ShellApp,
        ui: &mut egui::Ui,
        document_id: DocumentId,
        _fit_requested: bool,
    ) {
        app.render_graph_scope_document(ui, document_id);
    }
}

impl TabRenderer for LogicalTabRenderer {
    fn render(
        &mut self,
        app: &mut ShellApp,
        ui: &mut egui::Ui,
        document_id: DocumentId,
        _fit_requested: bool,
    ) {
        app.render_graph_scope_document(ui, document_id);
    }
}

impl TabRenderer for PhysicalTabRenderer {
    fn render(
        &mut self,
        app: &mut ShellApp,
        ui: &mut egui::Ui,
        document_id: DocumentId,
        _fit_requested: bool,
    ) {
        app.render_graph_scope_document(ui, document_id);
    }
}

impl TabRenderer for DefinitionCollectionTabRenderer {
    fn render(
        &mut self,
        app: &mut ShellApp,
        ui: &mut egui::Ui,
        document_id: DocumentId,
        _fit_requested: bool,
    ) {
        app.render_graph_scope_document(ui, document_id);
    }
}

impl TabRenderer for AssetTabRenderer {
    fn render(
        &mut self,
        app: &mut ShellApp,
        ui: &mut egui::Ui,
        document_id: DocumentId,
        _fit_requested: bool,
    ) {
        app.render_graph_asset_document(ui, document_id);
    }
}

impl TabRenderer for ImportTabRenderer {
    fn render(
        &mut self,
        app: &mut ShellApp,
        ui: &mut egui::Ui,
        document_id: DocumentId,
        _fit_requested: bool,
    ) {
        app.render_graph_import_document(ui, document_id);
    }
}

impl TabRenderer for KeybindingsTabRenderer {
    fn render(
        &mut self,
        app: &mut ShellApp,
        ui: &mut egui::Ui,
        _document_id: DocumentId,
        _fit_requested: bool,
    ) {
        app.render_keybindings_editor(ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_supports_core_document_kinds() {
        let registry = TabProviderRegistry::new_m1();
        assert!(registry.instantiate(DOCUMENT_KIND_BOARD).is_some());
        assert!(registry.instantiate(DOCUMENT_KIND_SPEC).is_some());
        assert!(registry.instantiate(DOCUMENT_KIND_SCHDOC_PREVIEW).is_some());
        assert!(registry.instantiate(DOCUMENT_KIND_SCHLIB_GALLERY).is_some());
        assert!(
            registry
                .instantiate(DOCUMENT_KIND_SCHLIB_COMPONENT)
                .is_some()
        );
        assert!(
            registry
                .instantiate(DOCUMENT_KIND_DESIGN_OVERVIEW)
                .is_some()
        );
        assert!(registry.instantiate(DOCUMENT_KIND_LOGICAL).is_some());
        assert!(registry.instantiate(DOCUMENT_KIND_PHYSICAL).is_some());
        assert!(
            registry
                .instantiate(DOCUMENT_KIND_DEFINITION_COLLECTION)
                .is_some()
        );
        assert!(registry.instantiate(DOCUMENT_KIND_ASSET).is_some());
        assert!(registry.instantiate(DOCUMENT_KIND_IMPORT).is_some());
        assert!(registry.instantiate(DOCUMENT_KIND_KEYBINDINGS).is_some());
    }
}

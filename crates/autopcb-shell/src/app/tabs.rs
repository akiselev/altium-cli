use std::collections::BTreeMap;

use efame::egui;

use crate::workbench::{DOCUMENT_KIND_BOARD, DOCUMENT_KIND_KEYBINDINGS, DOCUMENT_KIND_SPEC, DocumentId};

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
        registry.register(DOCUMENT_KIND_KEYBINDINGS, || Box::new(KeybindingsTabRenderer));
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
        assert!(registry.instantiate(DOCUMENT_KIND_KEYBINDINGS).is_some());
    }
}

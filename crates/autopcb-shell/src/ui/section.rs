use efame::egui::{self, RichText};

use crate::ui::theme::ThemeTokens;
use crate::ui::theme_primitives::section_tokens;

pub struct SectionPanel<'a> {
    heading: &'a str,
}

impl<'a> SectionPanel<'a> {
    pub fn new(heading: &'a str) -> Self {
        Self { heading }
    }

    pub fn show(
        self,
        ui: &mut egui::Ui,
        theme: &ThemeTokens,
        add_contents: impl FnOnce(&mut egui::Ui),
    ) {
        let t = section_tokens(theme);
        ui.label(RichText::new(self.heading).small().color(t.heading));
        ui.separator();
        add_contents(ui);
    }
}

pub fn empty_state(ui: &mut egui::Ui, theme: &ThemeTokens, message: &str) {
    let t = section_tokens(theme);
    ui.label(RichText::new(message).color(t.muted));
}

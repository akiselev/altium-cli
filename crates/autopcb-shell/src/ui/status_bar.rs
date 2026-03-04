use efame::egui::{self, RichText};

use crate::ui::theme::ThemeTokens;
use crate::ui::theme_primitives::{panel_frame, status_bar_tokens};

#[derive(Debug, Clone)]
pub struct StatusItem {
    pub label: String,
    pub small: bool,
}

impl StatusItem {
    pub fn normal(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            small: false,
        }
    }

    pub fn small(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            small: true,
        }
    }
}

pub fn show_status_bar(
    ctx: &egui::Context,
    id: &'static str,
    height: f32,
    theme: &ThemeTokens,
    items: &[StatusItem],
) {
    let t = status_bar_tokens(theme);
    egui::TopBottomPanel::bottom(id)
        .exact_height(height)
        .frame(panel_frame(t.bg))
        .show(ctx, |ui| {
            ui.visuals_mut().override_text_color = Some(t.text);
            ui.horizontal(|ui| {
                for (idx, item) in items.iter().enumerate() {
                    if item.small {
                        ui.label(RichText::new(&item.label).small().color(t.muted));
                    } else {
                        ui.label(&item.label);
                    }
                    if idx + 1 < items.len() {
                        ui.separator();
                    }
                }
            });
        });
}

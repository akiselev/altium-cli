use efame::egui::{self, RichText};

use crate::ui::theme::ThemeTokens;
use crate::ui::theme_primitives::segmented_tokens;

#[derive(Debug, Clone)]
pub struct SegmentItem<T> {
    pub value: T,
    pub label: String,
    pub enabled: bool,
}

impl<T> SegmentItem<T> {
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            enabled: true,
        }
    }
}

pub fn segmented_bar<T: Copy + Eq>(
    ui: &mut egui::Ui,
    theme: &ThemeTokens,
    selected: T,
    items: &[SegmentItem<T>],
) -> Option<T> {
    let mut changed = None;
    let t = segmented_tokens(theme);
    ui.horizontal(|ui| {
        for item in items {
            let is_selected = item.value == selected;
            let text_color = if is_selected {
                t.selected_text
            } else {
                t.idle_text
            };
            let label = RichText::new(&item.label).color(text_color);
            let resp = ui.add_enabled(item.enabled, egui::Button::selectable(is_selected, label));
            if is_selected {
                let rect = resp.rect;
                ui.painter()
                    .rect_filled(rect.shrink(1.0), 2.0, t.selected_bg);
            }
            if resp.clicked() {
                changed = Some(item.value);
            }
        }
    });
    changed
}

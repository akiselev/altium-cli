use efame::egui::{self, RichText};

use crate::ui::theme::ThemeTokens;
use crate::ui::theme_primitives::log_tokens;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Success,
    Warn,
    Error,
    Neutral,
}

pub fn classify_line(line: &str) -> LogLevel {
    if line.contains("failed") {
        LogLevel::Error
    } else if line.contains("completed") {
        LogLevel::Success
    } else if line.contains("cancelled") {
        LogLevel::Warn
    } else if line.contains("progress") || line.contains("started") {
        LogLevel::Info
    } else {
        LogLevel::Neutral
    }
}

pub fn show_log_lines(
    ui: &mut egui::Ui,
    theme: &ThemeTokens,
    empty_message: &str,
    lines: impl IntoIterator<Item = String>,
) {
    let items: Vec<String> = lines.into_iter().collect();
    if items.is_empty() {
        ui.label(RichText::new(empty_message).color(theme.text_muted));
        return;
    }

    let t = log_tokens(theme);
    egui::ScrollArea::vertical().show(ui, |ui| {
        for line in &items {
            let color = match classify_line(line) {
                LogLevel::Info => t.info,
                LogLevel::Success => t.success,
                LogLevel::Warn => t.warn,
                LogLevel::Error => t.error,
                LogLevel::Neutral => t.text,
            };
            ui.colored_label(color, line);
        }
    });
}

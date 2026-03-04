use efame::egui;

use crate::ui::theme::ThemeTokens;
use crate::ui::theme_primitives::{chrome_tokens, panel_frame};

pub fn show_top_bar(
    ctx: &egui::Context,
    id: &'static str,
    height: f32,
    theme: &ThemeTokens,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let c = chrome_tokens(theme);
    egui::TopBottomPanel::top(id)
        .exact_height(height)
        .frame(panel_frame(c.title_bg))
        .show(ctx, add_contents);
}

pub fn show_left_panel(
    ctx: &egui::Context,
    id: &'static str,
    width: Option<f32>,
    resizable: bool,
    theme: &ThemeTokens,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let c = chrome_tokens(theme);
    let panel = egui::SidePanel::left(id).resizable(resizable);
    let panel = match width {
        Some(w) if resizable => panel.default_width(w),
        Some(w) => panel.exact_width(w),
        None => panel,
    };
    panel
        .frame(panel_frame(c.sidebar_bg))
        .show(ctx, add_contents);
}

pub fn show_left_panel_with_fill(
    ctx: &egui::Context,
    id: &'static str,
    width: f32,
    resizable: bool,
    fill: egui::Color32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let panel = egui::SidePanel::left(id).resizable(resizable);
    let panel = if resizable {
        panel.default_width(width)
    } else {
        panel.exact_width(width)
    };
    panel.frame(panel_frame(fill)).show(ctx, add_contents);
}

pub fn show_right_panel(
    ctx: &egui::Context,
    id: &'static str,
    width: f32,
    resizable: bool,
    theme: &ThemeTokens,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let c = chrome_tokens(theme);
    egui::SidePanel::right(id)
        .resizable(resizable)
        .default_width(width)
        .frame(panel_frame(c.sidebar_bg))
        .show(ctx, add_contents);
}

pub fn show_central_panel(
    ctx: &egui::Context,
    theme: &ThemeTokens,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let c = chrome_tokens(theme);
    egui::CentralPanel::default()
        .frame(panel_frame(c.editor_bg))
        .show(ctx, add_contents);
}

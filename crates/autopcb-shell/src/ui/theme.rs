use efame::egui::{self, Color32, CornerRadius, Stroke, Vec2};

#[derive(Debug, Clone)]
pub struct ThemeTokens {
    pub window_bg: Color32,
    pub titlebar_bg: Color32,
    pub activitybar_bg: Color32,
    pub sidebar_bg: Color32,
    pub editor_bg: Color32,
    pub panel_bg: Color32,
    pub statusbar_bg: Color32,
    pub text_primary: Color32,
    pub text_muted: Color32,
    pub text_disabled: Color32,
    pub border_default: Color32,
    pub border_focus: Color32,
    pub accent_blue: Color32,
    pub tab_active_bg: Color32,
    pub tab_inactive_bg: Color32,
    pub tab_hover_bg: Color32,
}

pub fn vscode_dark_tokens() -> ThemeTokens {
    ThemeTokens {
        window_bg: Color32::from_rgb(30, 30, 30),
        titlebar_bg: Color32::from_rgb(45, 45, 45),
        activitybar_bg: Color32::from_rgb(51, 51, 51),
        sidebar_bg: Color32::from_rgb(37, 37, 38),
        editor_bg: Color32::from_rgb(30, 30, 30),
        panel_bg: Color32::from_rgb(30, 30, 30),
        statusbar_bg: Color32::from_rgb(0, 122, 204),
        text_primary: Color32::from_rgb(204, 204, 204),
        text_muted: Color32::from_rgb(140, 140, 140),
        text_disabled: Color32::from_rgb(95, 95, 95),
        border_default: Color32::from_rgb(58, 58, 58),
        border_focus: Color32::from_rgb(0, 122, 204),
        accent_blue: Color32::from_rgb(0, 122, 204),
        tab_active_bg: Color32::from_rgb(30, 30, 30),
        tab_inactive_bg: Color32::from_rgb(45, 45, 45),
        tab_hover_bg: Color32::from_rgb(55, 55, 55),
    }
}

pub fn apply_theme(ctx: &egui::Context, t: &ThemeTokens) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(6.0, 4.0);
    style.spacing.button_padding = Vec2::new(8.0, 4.0);
    style.visuals = egui::Visuals::dark();
    style.visuals.window_fill = t.window_bg;
    style.visuals.panel_fill = t.window_bg;
    style.visuals.override_text_color = Some(t.text_primary);
    style.visuals.widgets.noninteractive.bg_fill = t.window_bg;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.border_default);
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, t.text_primary);
    style.visuals.widgets.inactive.bg_fill = t.tab_inactive_bg;
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, t.text_primary);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, t.border_default);
    style.visuals.widgets.hovered.bg_fill = t.tab_hover_bg;
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, t.border_focus);
    style.visuals.widgets.active.bg_fill = t.tab_active_bg;
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, t.border_focus);
    style.visuals.selection.bg_fill = t.accent_blue;
    style.visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    style.visuals.window_corner_radius = CornerRadius::same(0);
    style.visuals.menu_corner_radius = CornerRadius::same(0);
    style.visuals.widgets.noninteractive.corner_radius = CornerRadius::same(0);
    style.visuals.widgets.inactive.corner_radius = CornerRadius::same(0);
    style.visuals.widgets.hovered.corner_radius = CornerRadius::same(0);
    style.visuals.widgets.active.corner_radius = CornerRadius::same(0);
    ctx.set_style(style);
}


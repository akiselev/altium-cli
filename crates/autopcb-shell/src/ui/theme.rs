use efame::egui::{self, Color32, CornerRadius, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct ThemeTokens {
    pub window_bg: Color32,
    pub titlebar_bg: Color32,
    pub activitybar_bg: Color32,
    pub sidebar_bg: Color32,
    pub editor_bg: Color32,
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
    pub font_scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeId {
    VscodeDark,
    VscodeLight,
    SolarizedDark,
    NordDark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemePrefs {
    pub active_theme: ThemeId,
    pub ui_scale: f32,
}

impl Default for ThemePrefs {
    fn default() -> Self {
        Self {
            active_theme: ThemeId::VscodeDark,
            ui_scale: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ThemeProfile {
    pub id: ThemeId,
    pub name: &'static str,
    pub tokens: ThemeTokens,
}

pub fn vscode_dark_tokens() -> ThemeTokens {
    ThemeTokens {
        window_bg: Color32::from_rgb(30, 30, 30),
        titlebar_bg: Color32::from_rgb(45, 45, 45),
        activitybar_bg: Color32::from_rgb(51, 51, 51),
        sidebar_bg: Color32::from_rgb(37, 37, 38),
        editor_bg: Color32::from_rgb(30, 30, 30),
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
        font_scale: 1.0,
    }
}

pub fn vscode_light_tokens() -> ThemeTokens {
    ThemeTokens {
        window_bg: Color32::from_rgb(248, 248, 248),
        titlebar_bg: Color32::from_rgb(225, 225, 225),
        activitybar_bg: Color32::from_rgb(236, 236, 236),
        sidebar_bg: Color32::from_rgb(243, 243, 243),
        editor_bg: Color32::from_rgb(255, 255, 255),
        statusbar_bg: Color32::from_rgb(0, 122, 204),
        text_primary: Color32::from_rgb(30, 30, 30),
        text_muted: Color32::from_rgb(92, 92, 92),
        text_disabled: Color32::from_rgb(140, 140, 140),
        border_default: Color32::from_rgb(205, 205, 205),
        border_focus: Color32::from_rgb(0, 122, 204),
        accent_blue: Color32::from_rgb(0, 122, 204),
        tab_active_bg: Color32::from_rgb(255, 255, 255),
        tab_inactive_bg: Color32::from_rgb(234, 234, 234),
        tab_hover_bg: Color32::from_rgb(226, 226, 226),
        font_scale: 1.0,
    }
}

pub fn solarized_dark_tokens() -> ThemeTokens {
    ThemeTokens {
        window_bg: Color32::from_rgb(0, 43, 54),
        titlebar_bg: Color32::from_rgb(7, 54, 66),
        activitybar_bg: Color32::from_rgb(12, 61, 73),
        sidebar_bg: Color32::from_rgb(7, 54, 66),
        editor_bg: Color32::from_rgb(0, 43, 54),
        statusbar_bg: Color32::from_rgb(38, 139, 210),
        text_primary: Color32::from_rgb(147, 161, 161),
        text_muted: Color32::from_rgb(101, 123, 131),
        text_disabled: Color32::from_rgb(88, 110, 117),
        border_default: Color32::from_rgb(42, 87, 98),
        border_focus: Color32::from_rgb(38, 139, 210),
        accent_blue: Color32::from_rgb(42, 161, 152),
        tab_active_bg: Color32::from_rgb(0, 43, 54),
        tab_inactive_bg: Color32::from_rgb(7, 54, 66),
        tab_hover_bg: Color32::from_rgb(16, 66, 78),
        font_scale: 1.0,
    }
}

pub fn nord_dark_tokens() -> ThemeTokens {
    ThemeTokens {
        window_bg: Color32::from_rgb(46, 52, 64),
        titlebar_bg: Color32::from_rgb(59, 66, 82),
        activitybar_bg: Color32::from_rgb(67, 76, 94),
        sidebar_bg: Color32::from_rgb(59, 66, 82),
        editor_bg: Color32::from_rgb(46, 52, 64),
        statusbar_bg: Color32::from_rgb(94, 129, 172),
        text_primary: Color32::from_rgb(216, 222, 233),
        text_muted: Color32::from_rgb(129, 161, 193),
        text_disabled: Color32::from_rgb(94, 129, 172),
        border_default: Color32::from_rgb(76, 86, 106),
        border_focus: Color32::from_rgb(136, 192, 208),
        accent_blue: Color32::from_rgb(136, 192, 208),
        tab_active_bg: Color32::from_rgb(46, 52, 64),
        tab_inactive_bg: Color32::from_rgb(59, 66, 82),
        tab_hover_bg: Color32::from_rgb(67, 76, 94),
        font_scale: 1.0,
    }
}

pub fn theme_profiles() -> Vec<ThemeProfile> {
    vec![
        ThemeProfile {
            id: ThemeId::VscodeDark,
            name: "VSCode Dark",
            tokens: vscode_dark_tokens(),
        },
        ThemeProfile {
            id: ThemeId::VscodeLight,
            name: "VSCode Light",
            tokens: vscode_light_tokens(),
        },
        ThemeProfile {
            id: ThemeId::SolarizedDark,
            name: "Solarized Dark",
            tokens: solarized_dark_tokens(),
        },
        ThemeProfile {
            id: ThemeId::NordDark,
            name: "Nord Dark",
            tokens: nord_dark_tokens(),
        },
    ]
}

pub fn theme_tokens_by_id(id: ThemeId) -> ThemeTokens {
    match id {
        ThemeId::VscodeDark => vscode_dark_tokens(),
        ThemeId::VscodeLight => vscode_light_tokens(),
        ThemeId::SolarizedDark => solarized_dark_tokens(),
        ThemeId::NordDark => nord_dark_tokens(),
    }
}

pub fn theme_name(id: ThemeId) -> &'static str {
    match id {
        ThemeId::VscodeDark => "VSCode Dark",
        ThemeId::VscodeLight => "VSCode Light",
        ThemeId::SolarizedDark => "Solarized Dark",
        ThemeId::NordDark => "Nord Dark",
    }
}

pub fn next_theme(id: ThemeId) -> ThemeId {
    match id {
        ThemeId::VscodeDark => ThemeId::VscodeLight,
        ThemeId::VscodeLight => ThemeId::SolarizedDark,
        ThemeId::SolarizedDark => ThemeId::NordDark,
        ThemeId::NordDark => ThemeId::VscodeDark,
    }
}

pub fn previous_theme(id: ThemeId) -> ThemeId {
    match id {
        ThemeId::VscodeDark => ThemeId::NordDark,
        ThemeId::VscodeLight => ThemeId::VscodeDark,
        ThemeId::SolarizedDark => ThemeId::VscodeLight,
        ThemeId::NordDark => ThemeId::SolarizedDark,
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

    // Keep text sizing in theme so runtime theme/font scaling can be applied without restarts.
    let scale = t.font_scale.clamp(0.8, 1.75);
    style.text_styles = BTreeMap::from([
        (
            egui::TextStyle::Small,
            egui::FontId::new(11.0 * scale, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Body,
            egui::FontId::new(13.5 * scale, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Button,
            egui::FontId::new(13.0 * scale, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Monospace,
            egui::FontId::new(12.5 * scale, egui::FontFamily::Monospace),
        ),
        (
            egui::TextStyle::Heading,
            egui::FontId::new(18.0 * scale, egui::FontFamily::Proportional),
        ),
    ]);

    ctx.set_style(style);
}

use efame::egui::{self, Color32, Frame};

use crate::ui::theme::ThemeTokens;

#[derive(Debug, Clone, Copy)]
pub struct ChromeTokens {
    pub title_bg: Color32,
    pub sidebar_bg: Color32,
    pub editor_bg: Color32,
}

#[derive(Debug, Clone, Copy)]
pub struct SectionTokens {
    pub heading: Color32,
    pub muted: Color32,
}

#[derive(Debug, Clone, Copy)]
pub struct SegmentedTokens {
    pub selected_bg: Color32,
    pub selected_text: Color32,
    pub idle_text: Color32,
}

#[derive(Debug, Clone, Copy)]
pub struct ListTokens {
    pub row_bg: Color32,
    pub row_selected_bg: Color32,
    pub row_border: Color32,
    pub row_selected_border: Color32,
    pub text: Color32,
    pub muted: Color32,
}

#[derive(Debug, Clone, Copy)]
pub struct StatusBarTokens {
    pub bg: Color32,
    pub text: Color32,
    pub muted: Color32,
}

#[derive(Debug, Clone, Copy)]
pub struct LogTokens {
    pub text: Color32,
    pub info: Color32,
    pub success: Color32,
    pub warn: Color32,
    pub error: Color32,
}

pub fn chrome_tokens(t: &ThemeTokens) -> ChromeTokens {
    ChromeTokens {
        title_bg: t.titlebar_bg,
        sidebar_bg: t.sidebar_bg,
        editor_bg: t.editor_bg,
    }
}

pub fn section_tokens(t: &ThemeTokens) -> SectionTokens {
    SectionTokens {
        heading: t.text_muted,
        muted: t.text_muted,
    }
}

pub fn segmented_tokens(t: &ThemeTokens) -> SegmentedTokens {
    SegmentedTokens {
        selected_bg: t.accent_blue.gamma_multiply(0.32),
        selected_text: Color32::WHITE,
        idle_text: t.text_primary,
    }
}

pub fn list_tokens(t: &ThemeTokens) -> ListTokens {
    ListTokens {
        row_bg: t.window_bg,
        row_selected_bg: t.accent_blue.gamma_multiply(0.32),
        row_border: t.border_default.gamma_multiply(0.55),
        row_selected_border: t.border_focus,
        text: t.text_primary,
        muted: t.text_muted,
    }
}

pub fn status_bar_tokens(t: &ThemeTokens) -> StatusBarTokens {
    StatusBarTokens {
        bg: t.statusbar_bg,
        text: Color32::WHITE,
        muted: Color32::from_gray(220),
    }
}

pub fn log_tokens(t: &ThemeTokens) -> LogTokens {
    LogTokens {
        text: t.text_primary,
        info: Color32::from_rgb(110, 170, 240),
        success: Color32::from_rgb(120, 210, 120),
        warn: Color32::from_rgb(200, 180, 120),
        error: Color32::from_rgb(230, 90, 90),
    }
}

pub fn panel_frame(fill: Color32) -> Frame {
    egui::Frame::new().fill(fill)
}

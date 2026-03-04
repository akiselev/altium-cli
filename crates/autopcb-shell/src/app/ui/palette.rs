use efame::egui;

use super::super::{PaletteMode, ShellApp};
use crate::pipeline::{Intent, ThemeIntent};
use crate::ui::palette_component::{PaletteItem, show_palette_overlay};
use crate::ui::theme::{theme_name, theme_profiles};

impl ShellApp {
    pub(crate) fn show_palette_window(&mut self, ctx: &egui::Context) {
        if !self.show_command_palette {
            return;
        }

        match self.palette_mode {
            PaletteMode::Command => {
                let cmd_ctx = self.command_context();
                let filter = self.palette_filter.to_lowercase();
                let commands: Vec<_> = self
                    .commands
                    .exposed()
                    .filter(|m| self.commands.is_enabled(*m, &cmd_ctx))
                    .filter(|m| {
                        filter.is_empty()
                            || m.title.to_lowercase().contains(&filter)
                            || m.id.contains(&filter)
                    })
                    .collect();
                let items: Vec<PaletteItem> = commands
                    .iter()
                    .map(|m| PaletteItem {
                        title: m.title.to_owned(),
                        subtitle: m.id.to_owned(),
                        swatch: None,
                    })
                    .collect();

                let result = show_palette_overlay(
                    ctx,
                    "command_palette_overlay",
                    "command_palette_input",
                    "Command Palette",
                    "Type a command",
                    &self.theme,
                    &mut self.palette_filter,
                    &mut self.palette_selected,
                    &mut self.palette_focus_pending,
                    &items,
                );

                if let Some(idx) = result.submitted_index {
                    if let Some(meta) = commands.get(idx) {
                        self.queue_command_id(meta.id, None);
                        self.show_command_palette = false;
                        self.palette_focus_pending = false;
                    }
                }
            }
            PaletteMode::Theme => {
                let profiles = theme_profiles();
                let filter = self.palette_filter.to_lowercase();
                let visible: Vec<_> = profiles
                    .iter()
                    .filter(|p| {
                        filter.is_empty()
                            || p.name.to_lowercase().contains(&filter)
                            || theme_name(p.id).to_lowercase().contains(&filter)
                    })
                    .collect();
                let items: Vec<PaletteItem> = visible
                    .iter()
                    .map(|p| PaletteItem {
                        title: p.name.to_owned(),
                        subtitle: if p.id == self.theme_prefs.active_theme {
                            "current theme".to_owned()
                        } else {
                            "theme preset".to_owned()
                        },
                        swatch: Some(p.tokens.accent_blue),
                    })
                    .collect();

                let result = show_palette_overlay(
                    ctx,
                    "theme_palette_overlay",
                    "theme_palette_input",
                    "Select Color Theme",
                    "Type to filter themes",
                    &self.theme,
                    &mut self.palette_filter,
                    &mut self.palette_selected,
                    &mut self.palette_focus_pending,
                    &items,
                );

                if let Some(idx) = result.hovered_index {
                    if let Some(profile) = visible.get(idx) {
                        self.theme_preview = Some(profile.id);
                        self.refresh_theme_tokens();
                    }
                }

                if let Some(idx) = result.submitted_index {
                    if let Some(profile) = visible.get(idx) {
                        self.queue_intent(Intent::Theme(ThemeIntent::SetTheme { id: profile.id }));
                        self.show_command_palette = false;
                        self.palette_focus_pending = false;
                    }
                }
            }
        }
    }
}

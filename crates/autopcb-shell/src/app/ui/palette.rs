use efame::egui;

use super::super::ShellApp;

impl ShellApp {
    pub(crate) fn show_palette_window(&mut self, ctx: &egui::Context) {
        if !self.show_command_palette {
            return;
        }

        let cmd_ctx = self.command_context();
        let mut open = self.show_command_palette;
        egui::Window::new("Command Palette")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                let resp = ui.text_edit_singleline(&mut self.palette_filter);
                if resp.changed() {
                    self.palette_selected = 0;
                }
                ui.separator();
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

                if !commands.is_empty() {
                    self.palette_selected = self.palette_selected.min(commands.len() - 1);
                } else {
                    self.palette_selected = 0;
                }

                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) && !commands.is_empty() {
                    self.palette_selected = (self.palette_selected + 1) % commands.len();
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) && !commands.is_empty() {
                    self.palette_selected = if self.palette_selected == 0 {
                        commands.len() - 1
                    } else {
                        self.palette_selected - 1
                    };
                }

                let mut clicked: Option<&'static str> = None;
                for (idx, meta) in commands.iter().enumerate() {
                    let selected = idx == self.palette_selected;
                    if ui
                        .selectable_label(selected, format!("{} ({})", meta.title, meta.id))
                        .clicked()
                    {
                        clicked = Some(meta.id);
                    }
                }

                if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !commands.is_empty() {
                    clicked = Some(commands[self.palette_selected].id);
                }

                if let Some(id) = clicked {
                    self.queue(id, None);
                    self.show_command_palette = false;
                }
            });
        self.show_command_palette = open;
    }
}

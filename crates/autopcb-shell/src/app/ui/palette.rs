use efame::egui;

use super::super::ShellApp;

impl ShellApp {
    pub(crate) fn show_palette_window(&mut self, ctx: &egui::Context) {
        if !self.show_command_palette {
            return;
        }

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

        if !commands.is_empty() {
            self.palette_selected = self.palette_selected.min(commands.len() - 1);
        } else {
            self.palette_selected = 0;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) && !commands.is_empty() {
            self.palette_selected = (self.palette_selected + 1) % commands.len();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) && !commands.is_empty() {
            self.palette_selected = if self.palette_selected == 0 {
                commands.len() - 1
            } else {
                self.palette_selected - 1
            };
        }

        let width = (ctx.content_rect().width() * 0.52).clamp(560.0, 860.0);
        let max_list_h = (ctx.content_rect().height() * 0.58).max(240.0);
        let mut clicked: Option<&'static str> = None;

        egui::Area::new(egui::Id::new("command_palette_overlay"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 42.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(self.theme.sidebar_bg)
                    .stroke(egui::Stroke::new(1.0, self.theme.border_focus))
                    .corner_radius(egui::CornerRadius::same(6))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_min_width(width);
                        ui.set_max_width(width);

                        let input_frame = egui::Frame::new()
                            .fill(self.theme.window_bg)
                            .stroke(egui::Stroke::new(1.0, self.theme.border_default))
                            .corner_radius(egui::CornerRadius::same(4))
                            .inner_margin(egui::Margin::symmetric(8, 6));
                        input_frame.show(ui, |ui| {
                            let input_id = ui.make_persistent_id("command_palette_input");
                            if self.palette_focus_pending {
                                ui.memory_mut(|m| m.request_focus(input_id));
                                self.palette_focus_pending = false;
                            }
                            let edit = egui::TextEdit::singleline(&mut self.palette_filter)
                                .hint_text("Type a command")
                                .id(input_id);
                            let resp = ui.add(edit);
                            if resp.changed() {
                                self.palette_selected = 0;
                            }
                        });

                        ui.add_space(6.0);

                        egui::ScrollArea::vertical()
                            .max_height(max_list_h)
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                if commands.is_empty() {
                                    ui.colored_label(self.theme.text_muted, "No matching commands");
                                    return;
                                }

                                for (idx, meta) in commands.iter().enumerate() {
                                    let selected = idx == self.palette_selected;
                                    let row_frame = egui::Frame::new()
                                        .fill(if selected {
                                            self.theme.accent_blue.gamma_multiply(0.32)
                                        } else {
                                            self.theme.window_bg
                                        })
                                        .stroke(egui::Stroke::new(
                                            1.0,
                                            if selected {
                                                self.theme.border_focus
                                            } else {
                                                self.theme.border_default.gamma_multiply(0.55)
                                            },
                                        ))
                                        .corner_radius(egui::CornerRadius::same(4))
                                        .inner_margin(egui::Margin::symmetric(8, 6));

                                    let row = row_frame.show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.vertical(|ui| {
                                            let title = if selected {
                                                egui::RichText::new(meta.title)
                                                    .strong()
                                                    .color(egui::Color32::WHITE)
                                            } else {
                                                egui::RichText::new(meta.title)
                                                    .strong()
                                                    .color(self.theme.text_primary)
                                            };
                                            ui.label(title);
                                            let id_text = if selected {
                                                egui::RichText::new(meta.id)
                                                    .small()
                                                    .color(egui::Color32::from_gray(220))
                                            } else {
                                                egui::RichText::new(meta.id)
                                                    .small()
                                                    .color(self.theme.text_muted)
                                            };
                                            ui.label(id_text);
                                        });
                                    });

                                    let resp = row.response.interact(egui::Sense::click());
                                    if resp.clicked() {
                                        clicked = Some(meta.id);
                                    }
                                    if resp.hovered() {
                                        self.palette_selected = idx;
                                    }
                                    ui.add_space(4.0);
                                }
                            });
                    });
            });

        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) && !commands.is_empty() {
            clicked = Some(commands[self.palette_selected].id);
        }

        if let Some(id) = clicked {
            self.queue_command_id(id, None);
            self.show_command_palette = false;
            self.palette_focus_pending = false;
        }
    }
}

use std::fs;
use std::path::Path;

use efame::egui::{self, RichText};

use super::super::{ActivityView, ShellApp};
use crate::pipeline::{CrossprobeIntent, FileIntent, Intent};
use crate::ui::chrome::show_left_panel;
use crate::ui::icons::{IconId, icon};
use crate::ui::section::{SectionPanel, empty_state};
use crate::workbench::SelectionKind;

impl ShellApp {
    pub(crate) fn render_sidebar(&mut self, ctx: &egui::Context) {
        if !self.panel_visibility.show_primary_sidebar {
            return;
        }
        let theme = self.theme.clone();
        let activity = self.panel_visibility.activity_view;

        show_left_panel(
            ctx,
            "primary_sidebar",
            Some(280.0),
            true,
            &theme,
            |ui| match activity {
                ActivityView::Explorer => self.render_explorer_sidebar(ui),
                ActivityView::Search => self.render_placeholder_sidebar(
                    ui,
                    "SEARCH",
                    "Workspace text search is planned.",
                ),
                ActivityView::SourceControl => self.render_placeholder_sidebar(
                    ui,
                    "SOURCE CONTROL",
                    "Source-control integration is planned.",
                ),
                ActivityView::Run => self.render_placeholder_sidebar(
                    ui,
                    "RUN",
                    "Automation run tasks will live here.",
                ),
                ActivityView::Extensions => self.render_placeholder_sidebar(
                    ui,
                    "EXTENSIONS",
                    "Plugin/extension management is planned.",
                ),
            },
        );
    }

    fn render_explorer_sidebar(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();
        SectionPanel::new("EXPLORER").show(ui, &theme, |ui| {
            self.render_workspace_files(ui);
            ui.separator();

            let Some(board) = self.model.active_board() else {
                empty_state(ui, &self.theme, "No active board document");
                return;
            };
            let ir = &board.ir;

            let components: Vec<String> = ir
                .components
                .iter()
                .map(|(_, comp)| comp.designator.clone())
                .collect();
            let nets: Vec<(String, usize)> = ir
                .nets
                .iter()
                .map(|(_, net)| (net.name.clone(), net.pins.len()))
                .collect();

            ui.collapsing("Components", |ui| {
                egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    for designator in &components {
                        let selected = matches!(
                            &self.model.selection.primary,
                            SelectionKind::Component(d) if d == designator
                        );
                        if ui.selectable_label(selected, designator).clicked() {
                            self.queue_intent(Intent::Crossprobe(
                                CrossprobeIntent::SelectComponent {
                                    designator: designator.clone(),
                                },
                            ));
                        }
                    }
                });
            });

            ui.collapsing("Nets", |ui| {
                egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    for (name, pins_len) in &nets {
                        let selected =
                            matches!(&self.model.selection.primary, SelectionKind::Net(n) if n == name);
                        if ui
                            .selectable_label(selected, format!("{} ({})", name, pins_len))
                            .clicked()
                        {
                            self.queue_intent(Intent::Crossprobe(CrossprobeIntent::SelectNet {
                                net_name: name.clone(),
                            }));
                        }
                    }
                });
            });
        });
    }

    fn render_placeholder_sidebar(&mut self, ui: &mut egui::Ui, heading: &str, text: &str) {
        let theme = self.theme.clone();
        SectionPanel::new(heading).show(ui, &theme, |ui| {
            ui.label(RichText::new(text).color(theme.text_disabled));
        });
    }

    fn render_workspace_files(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Workspace Files", |ui| {
            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.text_edit_singleline(&mut self.explorer_filter);
            });

            let Some(root) = self.model.workspace_root.clone() else {
                ui.label("No workspace open");
                return;
            };

            ui.small(RichText::new(root.display().to_string()).color(self.theme.text_muted));
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .show(ui, |ui| self.render_dir_tree(ui, &root, 0));
        });
    }

    fn render_dir_tree(&mut self, ui: &mut egui::Ui, dir: &Path, depth: usize) {
        if depth > 4 {
            return;
        }

        let mut entries = match fs::read_dir(dir) {
            Ok(read_dir) => read_dir.filter_map(Result::ok).collect::<Vec<_>>(),
            Err(_) => return,
        };
        entries.sort_by_key(|e| e.path());

        let filter = self.explorer_filter.to_ascii_lowercase();
        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }

            let passes_filter = filter.is_empty()
                || name.to_ascii_lowercase().contains(&filter)
                || path
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .contains(&filter);
            if !passes_filter {
                continue;
            }

            if path.is_dir() {
                ui.horizontal(|ui| {
                    icon(ui, IconId::Folder, self.theme.text_muted, 12.0);
                    ui.collapsing(format!("{name}/"), |ui| {
                        self.render_dir_tree(ui, &path, depth + 1);
                    });
                });
                continue;
            }

            let is_open = self
                .model
                .find_document_by_path(&path)
                .is_some_and(|id| self.model.active_editor_tab == Some(id));
            ui.horizontal(|ui| {
                let icon_id = if name.to_ascii_lowercase().ends_with(".pcbdoc") {
                    IconId::PcbDoc
                } else if name.to_ascii_lowercase().ends_with(".spec")
                    || name.to_ascii_lowercase().ends_with(".pcbdoc-spec")
                    || name.to_ascii_lowercase().ends_with(".schdoc-spec")
                    || name.to_ascii_lowercase().ends_with(".schlib-spec")
                    || name.to_ascii_lowercase().ends_with(".prjpcb-spec")
                {
                    IconId::Spec
                } else {
                    IconId::File
                };
                icon(ui, icon_id, self.theme.text_muted, 12.0);
                if ui.selectable_label(is_open, &name).clicked() {
                    self.queue_intent(Intent::File(FileIntent::Open {
                        path: Some(path.clone()),
                    }));
                }
            });
        }
    }
}

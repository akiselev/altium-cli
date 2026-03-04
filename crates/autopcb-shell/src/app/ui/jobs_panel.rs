use efame::egui::{self, RichText};

use super::super::ShellApp;
use crate::layout::BottomTab;
use crate::pipeline::{Intent, JobsIntent, PanelIntent};

impl ShellApp {
    pub(crate) fn render_bottom_panel_contents(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    self.panel_visibility.bottom_tab == BottomTab::Problems,
                    "Problems",
                )
                .clicked()
            {
                self.queue_intent(Intent::Panel(PanelIntent::ShowProblems));
            }
            if ui
                .selectable_label(
                    self.panel_visibility.bottom_tab == BottomTab::Output,
                    "Output",
                )
                .clicked()
            {
                self.queue_intent(Intent::Panel(PanelIntent::ShowOutput));
            }
            if ui
                .selectable_label(self.panel_visibility.bottom_tab == BottomTab::Jobs, "Jobs")
                .clicked()
            {
                self.queue_intent(Intent::Panel(PanelIntent::ShowJobs));
            }
        });
        ui.separator();

        match self.panel_visibility.bottom_tab {
            BottomTab::Problems => {
                if self.model.problems.is_empty() {
                    ui.label(RichText::new("No problems").color(self.theme.text_muted));
                }
                for line in &self.model.problems {
                    ui.label(line);
                }
            }
            BottomTab::Output => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for line in &self.model.output_lines {
                        ui.monospace(line);
                    }
                });
            }
            BottomTab::Jobs => {
                self.render_jobs_tab_contents(ui);
            }
        }
    }

    fn render_jobs_tab_contents(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Background Jobs")
                    .small()
                    .color(self.theme.text_muted),
            );
            ui.separator();
            ui.label(format!("Active: {}", self.jobs.active_jobs()));
            if ui.button("Cancel Active").clicked() {
                self.queue_intent(Intent::Jobs(JobsIntent::CancelActive));
            }
            if ui.button("Clear Log").clicked() {
                self.model.jobs.clear();
            }
        });
        ui.separator();

        if self.model.jobs.is_empty() {
            ui.label(RichText::new("No jobs").color(self.theme.text_muted));
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for line in &self.model.jobs {
                let color = if line.contains("failed") {
                    egui::Color32::from_rgb(230, 90, 90)
                } else if line.contains("completed") {
                    egui::Color32::from_rgb(120, 210, 120)
                } else if line.contains("progress") || line.contains("started") {
                    egui::Color32::from_rgb(110, 170, 240)
                } else if line.contains("cancelled") {
                    egui::Color32::from_rgb(200, 180, 120)
                } else {
                    self.theme.text_primary
                };
                ui.colored_label(color, line);
            }
        });
    }
}

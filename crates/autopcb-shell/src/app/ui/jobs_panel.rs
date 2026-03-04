use efame::egui::{self, RichText};

use super::super::ShellApp;
use crate::layout::BottomTab;
use crate::pipeline::{Intent, JobsIntent, PanelIntent};
use crate::ui::log_view::show_log_lines;
use crate::ui::section::empty_state;
use crate::ui::segmented::{SegmentItem, segmented_bar};

impl ShellApp {
    pub(crate) fn render_bottom_panel_contents(&mut self, ui: &mut egui::Ui) {
        let tabs = [
            SegmentItem::new(BottomTab::Problems, "Problems"),
            SegmentItem::new(BottomTab::Output, "Output"),
            SegmentItem::new(BottomTab::Jobs, "Jobs"),
        ];
        if let Some(changed) =
            segmented_bar(ui, &self.theme, self.panel_visibility.bottom_tab, &tabs)
        {
            match changed {
                BottomTab::Problems => self.queue_intent(Intent::Panel(PanelIntent::ShowProblems)),
                BottomTab::Output => self.queue_intent(Intent::Panel(PanelIntent::ShowOutput)),
                BottomTab::Jobs => self.queue_intent(Intent::Panel(PanelIntent::ShowJobs)),
            }
        }
        ui.separator();

        match self.panel_visibility.bottom_tab {
            BottomTab::Problems => {
                if self.model.problems.is_empty() {
                    empty_state(ui, &self.theme, "No problems");
                }
                for line in &self.model.problems {
                    ui.label(line);
                }
            }
            BottomTab::Output => {
                show_log_lines(
                    ui,
                    &self.theme,
                    "No output",
                    self.model.output_lines.clone(),
                );
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
            empty_state(ui, &self.theme, "No jobs");
            return;
        }

        show_log_lines(ui, &self.theme, "No jobs", self.model.jobs.clone());
    }
}

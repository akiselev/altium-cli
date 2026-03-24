use std::fs;
use std::path::Path;

use efame::egui::{self, RichText};

use super::super::{ActivityView, ShellApp};
use crate::agents::{AgentRunStatus, ProposalStatus};
use crate::pipeline::{AgentIntent, CrossprobeIntent, FileIntent, Intent, ReviewIntent};
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
                ActivityView::SourceControl => self.render_review_sidebar(ui),
                ActivityView::Run => self.render_agents_sidebar(ui),
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

    fn render_review_sidebar(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();
        SectionPanel::new("CHANGES & REVIEWS").show(ui, &theme, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} pending review",
                        self.agents.pending_review_count()
                    ))
                    .color(self.theme.text_muted),
                );
                if ui.button("Agents").clicked() {
                    self.queue_intent(Intent::Agent(AgentIntent::OpenPanel));
                }
            });
            ui.separator();

            let proposals: Vec<_> = self
                .agents
                .ordered_proposals()
                .into_iter()
                .cloned()
                .collect();
            if proposals.is_empty() {
                empty_state(ui, &self.theme, "No agent proposals");
                return;
            }

            for proposal in proposals {
                let is_selected = self.agents.active_proposal == Some(proposal.id);
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(
                                is_selected,
                                format!("#{} {}", proposal.id.0, proposal.title),
                            )
                            .clicked()
                        {
                            self.queue_intent(Intent::Review(ReviewIntent::SelectProposal {
                                proposal_id: proposal.id,
                            }));
                        }
                        ui.label(
                            RichText::new(review_status_label(proposal.status))
                                .color(self.theme.text_muted),
                        );
                    });
                    ui.small(&proposal.summary);
                    for line in &proposal.preview_lines {
                        ui.small(RichText::new(line).color(self.theme.text_disabled));
                    }
                    ui.horizontal(|ui| {
                        let can_apply = proposal.status == ProposalStatus::PendingReview;
                        if ui
                            .add_enabled(can_apply, egui::Button::new("Accept"))
                            .clicked()
                        {
                            self.queue_intent(Intent::Review(ReviewIntent::AcceptProposal {
                                proposal_id: proposal.id,
                            }));
                        }
                        if ui
                            .add_enabled(can_apply, egui::Button::new("Reject"))
                            .clicked()
                        {
                            self.queue_intent(Intent::Review(ReviewIntent::RejectProposal {
                                proposal_id: proposal.id,
                            }));
                        }
                    });
                });
                ui.add_space(6.0);
            }
        });
    }

    fn render_agents_sidebar(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();
        SectionPanel::new("AGENTS").show(ui, &theme, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New Session").clicked() {
                    self.queue_intent(Intent::Agent(AgentIntent::CreateSession));
                }
                if ui.button("Review Queue").clicked() {
                    self.queue_intent(Intent::Review(ReviewIntent::OpenQueue));
                }
            });
            ui.separator();

            if self.agents.sessions.is_empty() {
                empty_state(
                    ui,
                    &self.theme,
                    "No sessions yet. Ask the local agent to move a selected component to generate a reviewable proposal.",
                );
            } else {
                let sessions: Vec<_> = self.agents.ordered_sessions().into_iter().cloned().collect();
                for session in sessions {
                    let is_selected = self.agents.active_session == Some(session.id);
                    if ui
                        .selectable_label(
                            is_selected,
                            format!(
                                "#{} {} [{}]",
                                session.id.0,
                                session.title,
                                agent_status_label(session.status)
                            ),
                        )
                        .clicked()
                    {
                        self.agents.active_session = Some(session.id);
                        self.agents.active_proposal = session.proposal_ids.last().copied();
                    }
                }
            }

            ui.separator();
            ui.label(RichText::new("Prompt").color(self.theme.text_muted));
            ui.text_edit_multiline(&mut self.agents.composer_text);
            if ui.button("Send To Agent").clicked() {
                let prompt = self.agents.composer_text.trim().to_owned();
                if !prompt.is_empty() {
                    self.queue_intent(Intent::Agent(AgentIntent::SubmitPrompt {
                        session_id: self.agents.active_session,
                        prompt,
                    }));
                }
            }
            ui.small(
                RichText::new(
                    "Local-first stub: prompts that ask to move or shift the selected component create a reviewable proposal instead of mutating immediately.",
                )
                .color(self.theme.text_disabled),
            );

            if let Some(session_id) = self.agents.active_session
                && let Some(session) = self.agents.sessions.get(&session_id)
            {
                ui.separator();
                ui.label(RichText::new("Transcript").color(self.theme.text_muted));
                egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                    for message in &session.messages {
                        ui.label(
                            RichText::new(format!("{}: {}", message.author, message.body))
                                .color(self.theme.text_primary),
                        );
                    }
                });
            }
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
                } else if name.to_ascii_lowercase().ends_with(".wrk")
                    || name.to_ascii_lowercase().ends_with(".sch")
                    || name.to_ascii_lowercase().ends_with(".sym")
                    || name.to_ascii_lowercase().ends_with(".pcb")
                {
                    IconId::Spec
                } else if name.to_ascii_lowercase().ends_with(".spec")
                    || name.to_ascii_lowercase().ends_with(".proj")
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

fn review_status_label(status: ProposalStatus) -> &'static str {
    match status {
        ProposalStatus::PendingReview => "pending",
        ProposalStatus::Applied => "applied",
        ProposalStatus::Rejected => "rejected",
        ProposalStatus::Stale => "stale",
    }
}

fn agent_status_label(status: AgentRunStatus) -> &'static str {
    match status {
        AgentRunStatus::Idle => "idle",
        AgentRunStatus::Running => "running",
        AgentRunStatus::Completed => "completed",
        AgentRunStatus::Failed => "failed",
    }
}

use crate::pages::Page;
use crate::state::ui_state::Page as UiPage;
use crate::state::AppState;
use crate::state::ConnectionState;
use egui::{Context, Ui};

pub struct ProjectsPage;

impl ProjectsPage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ProjectsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::pages::Page for ProjectsPage {
    fn name(&self) -> &'static str {
        "Projects"
    }

    fn ui(&mut self, _ctx: &Context, ui: &mut Ui, state: &mut AppState) {
        ui.heading("Projects");

        match &state.connection_state {
            ConnectionState::Disconnected => {
                ui.vertical_centered(|ui| {
                    ui.label("Not connected to server");
                    if ui.button("Connect").clicked() {
                        state.ui_state.show_connection_dialog = true;
                    }
                });
            }
            ConnectionState::Connecting => {
                ui.vertical_centered(|ui| {
                    ui.label("Connecting...");
                    ui.add(egui::Spinner::new());
                });
            }
            ConnectionState::Connected => {
                if state.loading_projects {
                    ui.vertical_centered(|ui| {
                        ui.label("Loading projects...");
                        ui.add(egui::Spinner::new());
                    });
                } else if state.projects.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.label("No projects found");
                        if ui.button("Refresh").clicked() {
                            self.refresh_projects(state);
                        }
                    });
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add_space(8.0);

                        let card_width = 300.0;
                        let card_height = 100.0;
                        let card_spacing = 10.0;

                        // Set spacing between items
                        ui.spacing_mut().item_spacing = egui::Vec2::new(card_spacing, card_spacing);

                        // Collect project IDs to avoid borrowing issues
                        let project_ids: Vec<i64> = state.projects.iter().map(|p| p.id).collect();

                        // Use horizontal_wrapped to create a responsive grid
                        ui.horizontal_wrapped(|ui| {
                            for (i, project) in state.projects.iter().enumerate() {
                                let project_id = project_ids[i];
                                // Use allocate_ui with fixed size to enable proper wrapping
                                let response =
                                    ui.allocate_ui(egui::vec2(card_width, card_height), |ui| {
                                        egui::Frame::NONE
                                            .fill(ui.style().visuals.widgets.inactive.bg_fill)
                                            .corner_radius(8.0)
                                            .inner_margin(egui::Margin::same(12))
                                            .show(ui, |ui| {
                                                ui.vertical(|ui| {
                                                    // Project name - larger and bold
                                                    ui.label(
                                                        egui::RichText::new(&project.name)
                                                            .size(16.0)
                                                            .strong(),
                                                    );

                                                    ui.add_space(4.0);

                                                    // Project path - smaller, muted color
                                                    ui.label(
                                                        egui::RichText::new(&project.path)
                                                            .size(12.0)
                                                            .color(
                                                                ui.style()
                                                                    .visuals
                                                                    .weak_text_color(),
                                                            ),
                                                    );

                                                    // Description if present
                                                    if let Some(description) = &project.description
                                                    {
                                                        ui.add_space(6.0);
                                                        ui.label(
                                                            egui::RichText::new(description)
                                                                .size(11.0)
                                                                .color(
                                                                    ui.style()
                                                                        .visuals
                                                                        .weak_text_color(),
                                                                ),
                                                        );
                                                    }
                                                });
                                            });
                                    });

                                // Make the entire card clickable
                                if response.response.interact(egui::Sense::click()).clicked() {
                                    tracing::info!(
                                        "Project card clicked: project_id={}",
                                        project_id
                                    );
                                    state.ui_state.current_page = UiPage::ProjectDetail(project_id);
                                    state.pending_project_details_refresh = Some(project_id);
                                    tracing::info!(
                                        "Set pending_project_details_refresh to {}",
                                        project_id
                                    );
                                }

                                // Change cursor to pointer on hover
                                if response.response.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                            }
                        });
                    });
                }
            }
            ConnectionState::Error(error) => {
                ui.vertical_centered(|ui| {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
                    if ui.button("Retry").clicked() {
                        state.ui_state.show_connection_dialog = true;
                    }
                });
            }
        }
    }
}

impl ProjectsPage {
    fn refresh_projects(&self, state: &mut AppState) {
        // This will be implemented when we extract the API methods
        // For now, this is a placeholder
        state.loading_projects = true;
    }
}

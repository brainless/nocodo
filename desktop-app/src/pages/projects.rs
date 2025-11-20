use crate::state::ui_state::Page as UiPage;
use crate::state::AppState;
use crate::state::ConnectionState;
use crate::ui_text::{ContentText, WidgetText};
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

    fn on_navigate_to(&mut self) {
        // Set flag to trigger projects refresh in the update loop
    }

    fn ui(&mut self, _ctx: &Context, ui: &mut Ui, state: &mut AppState) {
        // Trigger refresh if flag is set
        if state.ui_state.pending_projects_refresh {
            state.ui_state.pending_projects_refresh = false;
            if state.connection_state == ConnectionState::Connected && !state.loading_projects {
                self.refresh_projects(state);
            }
        }

        // Page heading - Ubuntu SemiBold
        ui.heading(WidgetText::page_heading("Projects"));

        match &state.connection_state {
            ConnectionState::Disconnected => {
                ui.vertical_centered(|ui| {
                    // Status message - Ubuntu Light
                    ui.label(WidgetText::status("Not connected to server"));
                    // Button - Ubuntu SemiBold
                    if ui.button(WidgetText::button("Connect")).clicked() {
                        state.ui_state.show_connection_dialog = true;
                    }
                });
            }
            ConnectionState::Connecting => {
                ui.vertical_centered(|ui| {
                    // Status message - Ubuntu Light
                    ui.label(WidgetText::status("Connecting..."));
                    ui.add(egui::Spinner::new());
                });
            }
            ConnectionState::Connected => {
                if state.loading_projects {
                    ui.vertical_centered(|ui| {
                        // Status message - Ubuntu Light
                        ui.label(WidgetText::status("Loading projects..."));
                        ui.add(egui::Spinner::new());
                    });
                } else if state.projects.is_empty() {
                    ui.vertical_centered(|ui| {
                        // Status message - Ubuntu Light
                        ui.label(WidgetText::status("No projects found"));
                        // Button - Ubuntu SemiBold
                        if ui.button(WidgetText::button("Refresh")).clicked() {
                            self.refresh_projects(state);
                        }
                    });
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add_space(8.0);

                        let card_width = 320.0;
                        let card_height = 100.0;
                        let card_spacing = 16.0;
                        let available_width = ui.available_width();

                        // Calculate number of columns (1, 2, or 3) based on available width
                        let num_columns = if available_width >= (card_width * 3.0 + card_spacing * 2.0) {
                            3
                        } else if available_width >= (card_width * 2.0 + card_spacing * 1.0) {
                            2
                        } else {
                            1
                        };

                        // Collect project IDs to avoid borrowing issues
                        let project_ids: Vec<i64> = state.projects.iter().map(|p| p.id).collect();

                        // Create rows with the calculated number of columns
                        for row_start in (0..state.projects.len()).step_by(num_columns) {
                            ui.horizontal(|ui| {
                                for col in 0..num_columns {
                                    let idx = row_start + col;
                                    if idx >= state.projects.len() {
                                        break;
                                    }

                                    let project = &state.projects[idx];
                                    let project_id = project_ids[idx];

                                    // Allocate fixed width for the card
                                    let response = ui.allocate_ui(
                                        egui::vec2(card_width, card_height),
                                        |ui| {
                                            ui.set_width(card_width);
                                            egui::Frame::NONE
                                                .fill(ui.style().visuals.widgets.inactive.bg_fill)
                                                .corner_radius(8.0)
                                                .inner_margin(egui::Margin::same(12))
                                                .show(ui, |ui| {
                                                    ui.set_width(card_width - 24.0); // Account for margins
                                                    ui.vertical(|ui| {
                                                        // Project name - User content (Inter)
                                                        ui.label(ContentText::title(&project.name));

                                                        ui.add_space(4.0);

                                                        // Project path - User content (Inter)
                                                        ui.label(ContentText::subtitle(
                                                            ui,
                                                            &project.path,
                                                        ));

                                                        // Description if present - User content (Inter)
                                                        if let Some(description) = &project.description
                                                        {
                                                            ui.add_space(6.0);
                                                            ui.label(ContentText::description(
                                                                ui,
                                                                description,
                                                            ));
                                                        }
                                                    });
                                                });
                                        },
                                    );

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

                                    // Add spacing between columns (except after the last column)
                                    if col < num_columns - 1 && idx < state.projects.len() - 1 {
                                        ui.add_space(card_spacing);
                                    }
                                }
                            });

                            // Add spacing between rows
                            ui.add_space(card_spacing);
                        }
                    });
                }
            }
            ConnectionState::Error(error) => {
                ui.vertical_centered(|ui| {
                    // Error message - Ubuntu Light
                    ui.label(WidgetText::error(format!("Error: {}", error)));
                    // Button - Ubuntu SemiBold
                    if ui.button(WidgetText::button("Retry")).clicked() {
                        state.ui_state.show_connection_dialog = true;
                    }
                });
            }
        }
    }
}

impl ProjectsPage {
    fn refresh_projects(&self, state: &mut AppState) {
        let api_service = crate::services::ApiService::new();
        api_service.refresh_projects(state);
    }
}

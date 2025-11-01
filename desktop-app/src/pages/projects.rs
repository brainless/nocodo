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

    fn ui(&mut self, _ctx: &Context, ui: &mut Ui, state: &mut AppState) {
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

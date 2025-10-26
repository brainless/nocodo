use crate::pages::Page;
use crate::state::ui_state::Page as UiPage;
use crate::state::AppState;
use crate::state::ConnectionState;
use crate::ui_text::{ContentText, WidgetText};
use egui::{Context, Ui};

pub struct ProjectDetailPage {
    project_id: i64,
}

impl ProjectDetailPage {
    pub fn new(project_id: i64) -> Self {
        Self { project_id }
    }
}

impl crate::pages::Page for ProjectDetailPage {
    fn name(&self) -> &'static str {
        "Project Details"
    }

    fn ui(&mut self, _ctx: &Context, ui: &mut Ui, state: &mut AppState) {
        tracing::info!(
            "ProjectDetailPage::ui() called for project_id={}, loading={}, has_details={}",
            self.project_id,
            state.loading_project_details,
            state.project_details.is_some()
        );

        // Header with back button and star button
        ui.horizontal(|ui| {
            // Back button - Ubuntu SemiBold
            if ui
                .button(WidgetText::button("← Back to Projects"))
                .clicked()
            {
                state.ui_state.current_page = UiPage::Projects;
            }

            ui.add_space(10.0);

            // Star button - Ubuntu SemiBold with color
            let is_favorite = self.is_project_favorite(state);
            let star_text = if is_favorite { "⭐ Star" } else { "☆ Star" };
            let star_color = if is_favorite {
                egui::Color32::YELLOW
            } else {
                ui.style().visuals.text_color()
            };

            if ui
                .button(
                    egui::RichText::new(star_text)
                        .color(star_color)
                        .family(egui::FontFamily::Name("ui_semibold".into())),
                )
                .clicked()
            {
                self.toggle_project_favorite(state);
            }
        });

        ui.add_space(8.0);

        match &state.connection_state {
            ConnectionState::Disconnected => {
                ui.vertical_centered(|ui| {
                    // Status - Ubuntu Light
                    ui.label(WidgetText::status("Not connected to server"));
                });
            }
            ConnectionState::Connecting => {
                ui.vertical_centered(|ui| {
                    // Status - Ubuntu Light
                    ui.label(WidgetText::status("Connecting..."));
                    ui.add(egui::Spinner::new());
                });
            }
            ConnectionState::Connected => {
                if state.loading_project_details {
                    ui.vertical_centered(|ui| {
                        // Status - Ubuntu Light
                        ui.label(WidgetText::status("Loading project details..."));
                        ui.add(egui::Spinner::new());
                    });
                } else if let Some(details) = &state.project_details {
                    // Project title - User content (Inter)
                    ui.label(ContentText::title(&details.project.name));

                    ui.add_space(4.0);

                    // Project metadata
                    ui.horizontal(|ui| {
                        // Labels - Ubuntu Light
                        ui.label(WidgetText::label("Path:"));
                        // User content - Inter
                        ui.label(ContentText::text(&details.project.path));

                        if let Some(description) = &details.project.description {
                            ui.separator();
                            // Label - Ubuntu Light
                            ui.label(WidgetText::label("Description:"));
                            // User content - Inter
                            ui.label(ContentText::text(description));
                        }
                    });

                    ui.separator();

                    // Section heading - Ubuntu SemiBold
                    ui.heading(WidgetText::section_heading("Project Components"));

                    if details.components.is_empty() {
                        // Status - Ubuntu Light
                        ui.label(WidgetText::status("No components found"));
                    } else {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for component in &details.components {
                                ui.horizontal(|ui| {
                                    // Component data - User content (Inter)
                                    ui.label(ContentText::text(&component.name));
                                    ui.separator();
                                    ui.label(ContentText::text(&component.path));
                                    ui.separator();
                                    ui.label(ContentText::text(&component.language));
                                    if let Some(framework) = &component.framework {
                                        ui.separator();
                                        ui.label(ContentText::text(framework));
                                    }
                                });
                                ui.separator();
                            }
                        });
                    }

                    ui.add_space(8.0);

                    // Button - Ubuntu SemiBold
                    if ui.button(WidgetText::button("Refresh")).clicked() {
                        self.refresh_project_details(state);
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        // Status - Ubuntu Light
                        ui.label(WidgetText::status("Project not found"));
                        // Button - Ubuntu SemiBold
                        if ui.button(WidgetText::button("Back to Projects")).clicked() {
                            state.ui_state.current_page = UiPage::Projects;
                        }
                    });
                }
            }
            ConnectionState::Error(error) => {
                ui.vertical_centered(|ui| {
                    // Error - Ubuntu Light
                    ui.label(WidgetText::error(format!("Error: {}", error)));
                    // Button - Ubuntu SemiBold
                    if ui.button(WidgetText::button("Back to Projects")).clicked() {
                        state.ui_state.current_page = UiPage::Projects;
                    }
                });
            }
        }
    }
}

impl ProjectDetailPage {
    fn is_project_favorite(&self, state: &AppState) -> bool {
        state.favorite_projects.contains(&self.project_id)
    }

    fn toggle_project_favorite(&self, state: &mut AppState) {
        if state.favorite_projects.contains(&self.project_id) {
            state.favorite_projects.remove(&self.project_id);
        } else {
            state.favorite_projects.insert(self.project_id);
        }
    }

    fn refresh_project_details(&self, state: &mut AppState) {
        let api_service = crate::services::ApiService::new();
        api_service.refresh_project_details(self.project_id, state);
    }
}

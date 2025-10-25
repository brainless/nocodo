use crate::pages::Page;
use crate::state::ui_state::Page as UiPage;
use crate::state::AppState;
use crate::state::ConnectionState;
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
            if ui.button("← Back to Projects").clicked() {
                state.ui_state.current_page = UiPage::Projects;
            }

            ui.add_space(10.0);

            // Star button
            let is_favorite = self.is_project_favorite(state);
            let star_text = if is_favorite { "⭐ Star" } else { "☆ Star" };
            let star_color = if is_favorite {
                egui::Color32::YELLOW
            } else {
                ui.style().visuals.text_color()
            };

            if ui
                .button(egui::RichText::new(star_text).color(star_color))
                .clicked()
            {
                self.toggle_project_favorite(state);
            }
        });

        ui.add_space(8.0);

        match &state.connection_state {
            ConnectionState::Disconnected => {
                ui.vertical_centered(|ui| {
                    ui.label("Not connected to server");
                });
            }
            ConnectionState::Connecting => {
                ui.vertical_centered(|ui| {
                    ui.label("Connecting...");
                    ui.add(egui::Spinner::new());
                });
            }
            ConnectionState::Connected => {
                if state.loading_project_details {
                    ui.vertical_centered(|ui| {
                        ui.label("Loading project details...");
                        ui.add(egui::Spinner::new());
                    });
                } else if let Some(details) = &state.project_details {
                    // Project title
                    ui.heading(&details.project.name);

                    ui.add_space(4.0);

                    // Project metadata
                    ui.horizontal(|ui| {
                        ui.label("Path:");
                        ui.label(&details.project.path);

                        if let Some(description) = &details.project.description {
                            ui.separator();
                            ui.label("Description:");
                            ui.label(description);
                        }
                    });

                    ui.separator();

                    // Project components
                    ui.heading("Project Components");

                    if details.components.is_empty() {
                        ui.label("No components found");
                    } else {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for component in &details.components {
                                ui.horizontal(|ui| {
                                    ui.label(&component.name);
                                    ui.separator();
                                    ui.label(&component.path);
                                    ui.separator();
                                    ui.label(&component.language);
                                    if let Some(framework) = &component.framework {
                                        ui.separator();
                                        ui.label(framework);
                                    }
                                });
                                ui.separator();
                            }
                        });
                    }

                    ui.add_space(8.0);

                    if ui.button("Refresh").clicked() {
                        self.refresh_project_details(state);
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.label("Project not found");
                        if ui.button("Back to Projects").clicked() {
                            state.ui_state.current_page = UiPage::Projects;
                        }
                    });
                }
            }
            ConnectionState::Error(error) => {
                ui.vertical_centered(|ui| {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
                    if ui.button("Back to Projects").clicked() {
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

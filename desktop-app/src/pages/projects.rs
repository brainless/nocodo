use crate::state::ui_state::Page as UiPage;
use crate::state::AppState;
use crate::state::ConnectionState;
use crate::ui_text::{ContentText, WidgetText};
use egui::{Context, Ui};
use egui_flex::{item, Flex};
use egui_material_icons::icons;

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
                        // Button - Ubuntu SemiBold with icon
                        if ui
                            .button(WidgetText::button(format!(
                                " {} Refresh",
                                icons::ICON_REFRESH
                            )))
                            .clicked()
                        {
                            self.refresh_projects(state);
                        }
                    });
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add_space(8.0);

                        let card_width = 320.0;
                        let card_height = 100.0;
                        let card_spacing = 16.0;

                        // Collect project IDs to avoid borrowing issues
                        let project_ids: Vec<i64> = state.projects.iter().map(|p| p.id).collect();

                        // Create flex layout that wraps horizontally
                        Flex::horizontal()
                            .wrap(true)
                            .gap(egui::Vec2::new(card_spacing, card_spacing))
                            .show(ui, |flex| {
                                for (idx, project) in state.projects.iter().enumerate() {
                                    let project_id = project_ids[idx];

                                    flex.add_ui(item().min_width(card_width).min_height(card_height), |ui| {
                                        let response = egui::Frame::NONE
                                            .fill(ui.style().visuals.widgets.inactive.bg_fill)
                                            .corner_radius(8.0)
                                            .inner_margin(egui::Margin::same(12))
                                            .show(ui, |ui| {
                                                ui.set_width(card_width - 24.0); // Account for margins
                                                ui.vertical(|ui| {
                                                    // Header with name and favorite button
                                                    ui.horizontal(|ui| {
                                                        // Project name - User content (Inter)
                                                        ui.label(ContentText::title(&project.name));

                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            // Check if this project is a favorite
                                                            let is_favorite = if let Some((server_host, server_user, server_port)) = &state.current_server_info {
                                                                let favorite_key = &(server_host.clone(), server_user.clone(), *server_port, project.id);
                                                                state.favorite_projects.contains(favorite_key)
                                                            } else {
                                                                false
                                                            };

                                                            let favorite_icon = if is_favorite {
                                                                icons::ICON_FAVORITE
                                                            } else {
                                                                icons::ICON_FAVORITE_BORDER
                                                            };

                                                            if ui.button(favorite_icon).clicked() {
                                                                // Toggle favorite status
                                                                if let Some((server_host, server_user, server_port)) = &state.current_server_info {
                                                                    let favorite_key = &(server_host.clone(), server_user.clone(), *server_port, project.id);
                                                                    if is_favorite {
                                                                        state.favorite_projects.remove(favorite_key);
                                                                    } else {
                                                                        state.favorite_projects.insert(favorite_key.clone());
                                                                    }

                                                                    // Update database
                                                                    if let Some(db) = &state.db {
                                                                        if is_favorite {
                                                                            let _ = db.execute(
                                                                                "DELETE FROM favorites WHERE entity_type = 'project' AND entity_id = ? AND server_host = ? AND server_user = ? AND server_port = ?",
                                                                                [&project.id.to_string(), server_host, server_user, &server_port.to_string()],
                                                                            );
                                                                        } else {
                                                                            let _ = db.execute(
                                                                                "INSERT INTO favorites (entity_type, entity_id, server_host, server_user, server_port, created_at) VALUES ('project', ?, ?, ?, ?, ?)",
                                                                                [&project.id.to_string(), server_host, server_user, &server_port.to_string(), &chrono::Utc::now().timestamp().to_string()],
                                                                            );
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        });
                                                    });

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
                                    });
                                }
                            });
                    });
                }
            }
            ConnectionState::Error(error) => {
                ui.vertical_centered(|ui| {
                    // Error message - Ubuntu Light
                    ui.label(WidgetText::error(format!("Error: {}", error)));
                    // Button - Ubuntu SemiBold with icon
                    if ui
                        .button(WidgetText::button(format!(
                            " {} Retry",
                            icons::ICON_REFRESH
                        )))
                        .clicked()
                    {
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

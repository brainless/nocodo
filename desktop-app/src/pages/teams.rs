use crate::pages::Page;
use crate::services::ApiService;
use crate::state::{AppState, ConnectionState};
use crate::ui_text::{ContentText, WidgetText};
use egui::{Context, Ui};
use manager_models::UpdateTeamRequest;

pub struct TeamsPage;

impl TeamsPage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TeamsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Page for TeamsPage {
    fn name(&self) -> &'static str {
        "Teams"
    }

    fn on_navigate_to(&mut self) {
        // Will be called when navigating to this page
    }

    fn ui(&mut self, ctx: &Context, ui: &mut Ui, state: &mut AppState) {
        // Trigger refresh if flag is set
        if state.ui_state.pending_teams_refresh {
            state.ui_state.pending_teams_refresh = false;
            if state.connection_state == ConnectionState::Connected && !state.loading_teams {
                let api_service = ApiService::new();
                api_service.refresh_team_list(state);
            }
        }

        match &state.connection_state {
            ConnectionState::Disconnected => {
                ui.vertical_centered(|ui| {
                    ui.label(WidgetText::status("Not connected to server"));
                    if ui.button(WidgetText::button("Connect")).clicked() {
                        state.ui_state.show_connection_dialog = true;
                    }
                });
            }
            ConnectionState::Connected => {
                self.render_connected_ui(ctx, ui, state);
            }
            _ => {}
        }
    }
}

impl TeamsPage {
    fn render_connected_ui(&self, ctx: &Context, ui: &mut Ui, state: &mut AppState) {
        ui.heading(WidgetText::page_heading("Teams"));
        ui.add_space(16.0);

        // Search bar
        ui.horizontal(|ui| {
            ui.label(WidgetText::label("Search:"));
            let response = ui.text_edit_singleline(&mut state.team_search_query);
            if response.changed() {
                // Apply filter immediately
                let api_service = ApiService::new();
                api_service.apply_team_search_filter(state);
            }
        });
        ui.add_space(8.0);

        // NOTE: Checkboxes are disabled until bulk actions are defined
        // Uncomment when implementing bulk operations (delete, permission assignment, etc.)
        // if !state.selected_team_ids.is_empty() {
        //     ui.label(WidgetText::status(&format!(
        //         "{} selected",
        //         state.selected_team_ids.len()
        //     )));
        // }
        ui.add_space(16.0);

        // Loading state
        if state.loading_teams {
            ui.vertical_centered(|ui| {
                ui.label(WidgetText::status("Loading teams..."));
                ui.add(egui::Spinner::new());
            });
            return;
        }

        // Table with proper column layout
        let mut clicked_team: Option<manager_models::Team> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui_extras::TableBuilder::new(ui)
                .column(egui_extras::Column::remainder()) // ID column
                .column(egui_extras::Column::remainder()) // Name column
                .column(egui_extras::Column::remainder()) // Description column
                .column(egui_extras::Column::remainder()) // Permissions column
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.label(WidgetText::table_header("ID"));
                    });
                    header.col(|ui| {
                        ui.label(WidgetText::table_header("Name"));
                    });
                    header.col(|ui| {
                        ui.label(WidgetText::table_header("Description"));
                    });
                    header.col(|ui| {
                        ui.label(WidgetText::table_header("Permissions"));
                    });
                })
                .body(|mut body| {
                    for team_item in &state.filtered_teams {
                        body.row(18.0, |mut row| {
                            row.col(|ui| {
                                let id_text = ContentText::text(team_item.id.to_string());
                                if ui.label(id_text).clicked() {
                                    // Create a full Team object for editing
                                    if let Some(team) =
                                        state.teams.iter().find(|t| t.id == team_item.id)
                                    {
                                        clicked_team = Some(team.clone());
                                    }
                                }
                            });
                            row.col(|ui| {
                                let name_text = ContentText::text(&team_item.name);
                                if ui.label(name_text).clicked() {
                                    // Create a full Team object for editing
                                    if let Some(team) =
                                        state.teams.iter().find(|t| t.id == team_item.id)
                                    {
                                        clicked_team = Some(team.clone());
                                    }
                                }
                            });
                            row.col(|ui| {
                                let desc_text = ContentText::text(
                                    team_item.description.as_ref().unwrap_or(&"-".to_string()),
                                );
                                if ui.label(desc_text).clicked() {
                                    // Create a full Team object for editing
                                    if let Some(team) =
                                        state.teams.iter().find(|t| t.id == team_item.id)
                                    {
                                        clicked_team = Some(team.clone());
                                    }
                                }
                            });
                            row.col(|ui| {
                                let permission_names: Vec<String> = team_item
                                    .permissions
                                    .iter()
                                    .map(|p| format!("{}:{}", p.resource_type, p.action))
                                    .collect();
                                let permissions_text = if permission_names.is_empty() {
                                    "No permissions".to_string()
                                } else {
                                    permission_names.join(", ")
                                };
                                let permissions_text = ContentText::text(&permissions_text);
                                if ui.label(permissions_text).clicked() {
                                    // Create a full Team object for editing
                                    if let Some(team) =
                                        state.teams.iter().find(|t| t.id == team_item.id)
                                    {
                                        clicked_team = Some(team.clone());
                                    }
                                }
                            });
                        });
                    }
                });
        });

        // Handle clicked team
        if let Some(team) = clicked_team {
            state.editing_team = Some(team.clone());
            state.show_team_modal = true;

            // Load permissions for this team
            let api_service = ApiService::new();
            api_service.refresh_team_permissions(state, team.id);
        }

        // Team detail modal
        if state.show_team_modal {
            self.render_team_modal(ctx, state);
        }
    }

    fn render_team_modal(&self, ctx: &Context, state: &mut AppState) {
        egui::Window::new("Team Details")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if let Some(ref mut team) = state.editing_team {
                    ui.label(WidgetText::label("ID:"));
                    ui.label(ContentText::text(team.id.to_string()));
                    ui.add_space(8.0);

                    ui.label(WidgetText::label("Name:"));
                    ui.text_edit_singleline(&mut team.name);
                    ui.add_space(8.0);

                    ui.label(WidgetText::label("Description:"));
                    let mut description = team.description.clone().unwrap_or_default();
                    ui.text_edit_multiline(&mut description);
                    // Update the team description when changed
                    team.description = if description.trim().is_empty() {
                        None
                    } else {
                        Some(description)
                    };
                    ui.add_space(8.0);

                    ui.label(WidgetText::section_heading("Permissions"));
                    ui.separator();

                    // For now, show a placeholder for permissions
                    // In a full implementation, this would load and display actual permissions
                    ui.label(WidgetText::label(
                        "Permission management will be implemented in a future update.",
                    ));
                    ui.add_space(16.0);

                    // Clone values to avoid borrow issues
                    let team_id = team.id;
                    let team_name = team.name.clone();
                    let team_description = team.description.clone();

                    // Buttons
                    let mut update_clicked = false;
                    let mut cancel_clicked = false;
                    ui.horizontal(|ui| {
                        if ui.button(WidgetText::button("Update")).clicked() {
                            update_clicked = true;
                        }

                        if ui.button(WidgetText::button("Cancel")).clicked() {
                            cancel_clicked = true;
                        }
                    });

                    if update_clicked {
                        let api_service = ApiService::new();
                        let request = UpdateTeamRequest {
                            name: Some(team_name),
                            description: team_description,
                        };
                        api_service.update_team(state, team_id, request);
                    }

                    if cancel_clicked {
                        state.show_team_modal = false;
                        state.editing_team = None;
                        state.editing_team_permissions.clear();
                    }
                }
            });
    }
}

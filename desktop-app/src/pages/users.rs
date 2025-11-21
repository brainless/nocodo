use crate::pages::Page;
use crate::services::ApiService;
use crate::state::{AppState, ConnectionState};
use crate::ui_text::{ContentText, WidgetText};
use egui::{Context, Ui};
use manager_models::UpdateUserRequest;
use std::sync::Arc;

pub struct UsersPage;

impl UsersPage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for UsersPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Page for UsersPage {
    fn name(&self) -> &'static str {
        "Users"
    }

    fn on_navigate_to(&mut self) {
        // Will be called when navigating to this page
    }

    fn ui(&mut self, ctx: &Context, ui: &mut Ui, state: &mut AppState) {
        // Trigger refresh if flag is set
        if state.ui_state.pending_users_refresh {
            state.ui_state.pending_users_refresh = false;
            if state.connection_state == ConnectionState::Connected && !state.loading_users {
                let api_service = ApiService::new();
                api_service.refresh_users(state);
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

impl UsersPage {
    fn render_connected_ui(&self, ctx: &Context, ui: &mut Ui, state: &mut AppState) {
        ui.heading(WidgetText::page_heading("Users"));
        ui.add_space(16.0);

        // Search bar
        ui.horizontal(|ui| {
            ui.label(WidgetText::label("Search:"));
            let response = ui.text_edit_singleline(&mut state.user_search_query);
            if response.changed() {
                // Apply filter immediately
                let query = state.user_search_query.to_lowercase();
                if query.is_empty() {
                    state.filtered_users = state.users.clone();
                } else {
                    state.filtered_users = state
                        .users
                        .iter()
                        .filter(|u| {
                            u.name.to_lowercase().contains(&query)
                                || u.email.to_lowercase().contains(&query)
                        })
                        .cloned()
                        .collect();
                }
            }
        });
        ui.add_space(8.0);

        // NOTE: Checkboxes are disabled until bulk actions are defined
        // Uncomment when implementing bulk operations (delete, team assignment, etc.)
        // if !state.selected_user_ids.is_empty() {
        //     ui.label(WidgetText::status(&format!(
        //         "{} selected",
        //         state.selected_user_ids.len()
        //     )));
        // }
        ui.add_space(16.0);

        // Loading state
        if state.loading_users {
            ui.vertical_centered(|ui| {
                ui.label(WidgetText::status("Loading users..."));
                ui.add(egui::Spinner::new());
            });
            return;
        }

        // Table with proper column layout
        let mut clicked_user: Option<manager_models::User> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui_extras::TableBuilder::new(ui)
                .column(egui_extras::Column::remainder()) // ID column
                .column(egui_extras::Column::remainder()) // Username column
                .column(egui_extras::Column::remainder()) // Email column
                .column(egui_extras::Column::remainder()) // Teams column
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.label(WidgetText::table_header("ID"));
                    });
                    header.col(|ui| {
                        ui.label(WidgetText::table_header("Username"));
                    });
                    header.col(|ui| {
                        ui.label(WidgetText::table_header("Email"));
                    });
                    header.col(|ui| {
                        ui.label(WidgetText::table_header("Teams"));
                    });
                })
                .body(|mut body| {
                    for user_item in &state.filtered_users {
                        body.row(18.0, |mut row| {
                            row.col(|ui| {
                                let id_text = ContentText::text(user_item.id.to_string());
                                if ui.label(id_text).clicked() {
                                    // Create a full User object for editing
                                    clicked_user = Some(manager_models::User {
                                        id: user_item.id,
                                        name: user_item.name.clone(),
                                        email: user_item.email.clone(),
                                        role: None,
                                        password_hash: String::new(),
                                        is_active: true,
                                        created_at: 0,
                                        updated_at: 0,
                                        last_login_at: None,
                                    });
                                }
                            });
                            row.col(|ui| {
                                let name_text = ContentText::text(&user_item.name);
                                if ui.label(name_text).clicked() {
                                    // Create a full User object for editing
                                    clicked_user = Some(manager_models::User {
                                        id: user_item.id,
                                        name: user_item.name.clone(),
                                        email: user_item.email.clone(),
                                        role: None,
                                        password_hash: String::new(),
                                        is_active: true,
                                        created_at: 0,
                                        updated_at: 0,
                                        last_login_at: None,
                                    });
                                }
                            });
                            row.col(|ui| {
                                let email_text = ContentText::text(&user_item.email);
                                if ui.label(email_text).clicked() {
                                    // Create a full User object for editing
                                    clicked_user = Some(manager_models::User {
                                        id: user_item.id,
                                        name: user_item.name.clone(),
                                        email: user_item.email.clone(),
                                        role: None,
                                        password_hash: String::new(),
                                        is_active: true,
                                        created_at: 0,
                                        updated_at: 0,
                                        last_login_at: None,
                                    });
                                }
                            });
                            row.col(|ui| {
                                let team_names: Vec<String> = user_item
                                    .teams
                                    .iter()
                                    .map(|team| team.name.clone())
                                    .collect();
                                let teams_text = if team_names.is_empty() {
                                    "No teams".to_string()
                                } else {
                                    team_names.join(", ")
                                };
                                let teams_text = ContentText::text(&teams_text);
                                if ui.label(teams_text).clicked() {
                                    // Create a full User object for editing
                                    clicked_user = Some(manager_models::User {
                                        id: user_item.id,
                                        name: user_item.name.clone(),
                                        email: user_item.email.clone(),
                                        role: None,
                                        password_hash: String::new(),
                                        is_active: true,
                                        created_at: 0,
                                        updated_at: 0,
                                        last_login_at: None,
                                    });
                                }
                            });
                        });
                    }
                });
        });

        // Handle clicked user
        if let Some(user) = clicked_user {
            let user_id = user.id;
            state.editing_user = Some(user);
            state.show_user_modal = true;

            // Load teams for this user
            let api_service = ApiService::new();
            api_service.refresh_teams(state);

            // Load user's current team memberships
            let _connection_manager = Arc::clone(&state.connection_manager);
            tokio::spawn(async move {
                if let Some(api_client_arc) = _connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    if let Ok(_user_teams) = api_client.get_user_teams(user_id).await {
                        // Update state - this needs to be handled in the UI thread
                        // For now, the teams will be loaded when the modal opens
                    }
                }
            });
        }

        // User detail modal
        if state.show_user_modal {
            self.render_user_modal(ctx, state);
        }
    }

    fn render_user_modal(&self, ctx: &Context, state: &mut AppState) {
        egui::Window::new("User Details")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if let Some(ref mut user) = state.editing_user {
                    ui.label(WidgetText::label("ID:"));
                    ui.label(ContentText::text(user.id.to_string()));
                    ui.add_space(8.0);

                    ui.label(WidgetText::label("Username:"));
                    ui.text_edit_singleline(&mut user.name);
                    ui.add_space(8.0);

                    ui.label(WidgetText::label("Email:"));
                    ui.text_edit_singleline(&mut user.email);
                    ui.add_space(8.0);

                    ui.label(WidgetText::section_heading("Teams"));
                    ui.separator();

                    // Load user teams if not already loaded
                    if state.editing_user_teams.is_empty() && !state.team_list_items.is_empty() {
                        let user_id = user.id;

                        // Find current teams for this user from the users list
                        if let Some(user_item) = state.users.iter().find(|u| u.id == user_id) {
                            state.editing_user_teams =
                                user_item.teams.iter().map(|t| t.id).collect();
                        }
                    }

                    // Team checkboxes
                    for team in &state.team_list_items {
                        let mut is_member = state.editing_user_teams.contains(&team.id);
                        if ui.checkbox(&mut is_member, &team.name).changed() {
                            if is_member {
                                state.editing_user_teams.push(team.id);
                            } else {
                                state.editing_user_teams.retain(|&id| id != team.id);
                            }
                        }
                    }
                    ui.add_space(16.0);

                    // Clone values to avoid borrow issues
                    let user_id = user.id;
                    let user_name = user.name.clone();
                    let user_email = user.email.clone();

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
                        let request = UpdateUserRequest {
                            name: Some(user_name),
                            email: Some(user_email),
                            team_ids: Some(state.editing_user_teams.clone()),
                        };
                        api_service.update_user(state, user_id, request);
                    }

                    if cancel_clicked {
                        state.show_user_modal = false;
                        state.editing_user = None;
                        state.editing_user_teams.clear();
                    }
                }
            });
    }
}

use crate::pages::Page;
use crate::state::{AppState, ConnectionState};
use crate::ui_text::{ContentText, WidgetText};
use crate::services::ApiService;
use egui::{Context, Ui};
use manager_models::{UpdateUserRequest, User};

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
                    state.filtered_users = state.users
                        .iter()
                        .filter(|u| {
                            u.name.to_lowercase().contains(&query) ||
                            u.email.to_lowercase().contains(&query)
                        })
                        .cloned()
                        .collect();
                }
            }
        });
        ui.add_space(8.0);

        // Action buttons
        ui.horizontal(|ui| {
            if ui.button(WidgetText::button("Refresh")).clicked() {
                let api_service = ApiService::new();
                api_service.refresh_users(state);
            }

            // NOTE: Checkboxes are disabled until bulk actions are defined
            // Uncomment when implementing bulk operations (delete, team assignment, etc.)
            // if !state.selected_user_ids.is_empty() {
            //     ui.label(WidgetText::status(&format!(
            //         "{} selected",
            //         state.selected_user_ids.len()
            //     )));
            // }
        });
        ui.add_space(16.0);

        // Loading state
        if state.loading_users {
            ui.vertical_centered(|ui| {
                ui.label(WidgetText::status("Loading users..."));
                ui.add(egui::Spinner::new());
            });
            return;
        }

        // Table header
        ui.horizontal(|ui| {
            // NOTE: Checkboxes disabled until bulk actions are defined
            // ui.checkbox(&mut false, "");  // Select all checkbox
            ui.label(WidgetText::table_header("ID"));
            ui.label(WidgetText::table_header("Username"));
            ui.label(WidgetText::table_header("Email"));
            ui.label(WidgetText::table_header("Teams"));
        });
        ui.separator();

        // Table rows
        egui::ScrollArea::vertical().show(ui, |ui| {
            for user in &state.filtered_users {
                // NOTE: Checkboxes disabled until bulk actions are defined
                // let is_selected = state.selected_user_ids.contains(&user.id);

                ui.horizontal(|ui| {
                    // NOTE: Checkboxes disabled until bulk actions are defined
                    // let mut selected = is_selected;
                    // if ui.checkbox(&mut selected, "").changed() {
                    //     if selected {
                    //         state.selected_user_ids.insert(user.id);
                    //     } else {
                    //         state.selected_user_ids.remove(&user.id);
                    //     }
                    // }

                    ui.label(ContentText::text(&user.id.to_string()));
                    ui.label(ContentText::text(&user.name));
                    ui.label(ContentText::text(&user.email));
                    ui.label(ContentText::text("Team1, Team2"));  // TODO: Get from user
                });

                // Make row clickable
                let row_rect = ui.min_rect();
                let response = ui.interact(row_rect, ui.id().with(user.id), egui::Sense::click());
                if response.clicked() {
                    state.editing_user = Some(user.clone());
                    state.show_user_modal = true;

                    // Load teams for this user
                    let api_service = ApiService::new();
                    api_service.refresh_teams(state);
                }

                ui.separator();
            }
        });

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
                    ui.label(ContentText::text(&user.id.to_string()));
                    ui.add_space(8.0);

                    ui.label(WidgetText::label("Username:"));
                    ui.text_edit_singleline(&mut user.name);
                    ui.add_space(8.0);

                    ui.label(WidgetText::label("Email:"));
                    ui.text_edit_singleline(&mut user.email);
                    ui.add_space(8.0);

                    ui.label(WidgetText::section_heading("Teams"));
                    ui.separator();

                    // Team checkboxes
                    for team in &state.teams {
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

                    // Buttons
                    ui.horizontal(|ui| {
                        if ui.button(WidgetText::button("Update")).clicked() {
                            let api_service = ApiService::new();
                            let request = UpdateUserRequest {
                                name: Some(user.name.clone()),
                                email: Some(user.email.clone()),
                                team_ids: Some(state.editing_user_teams.clone()),
                            };
                            api_service.update_user(state, user.id, request);
                        }

                        if ui.button(WidgetText::button("Cancel")).clicked() {
                            state.show_user_modal = false;
                            state.editing_user = None;
                            state.editing_user_teams.clear();
                        }
                    });
                }
            });
    }
}
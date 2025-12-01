use crate::components::{command_card::CommandCard, markdown_renderer::MarkdownRenderer};
use crate::state::ui_state::{Page as UiPage, ProjectDetailTab};
use crate::state::AppState;
use crate::state::ConnectionState;
use crate::ui_text::{ContentText, WidgetText};
use egui::{Context, Ui};
use std::sync::Arc;

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

    fn ui(&mut self, ctx: &Context, ui: &mut Ui, state: &mut AppState) {
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
            let is_favorite = self.is_project_favorite_sync(state);
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
                self.toggle_project_favorite_sync(state);
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
                    // Clone the project details to avoid borrow issues
                    let project_name = details.project.name.clone();
                    let project_path = details.project.path.clone();
                    let project_description = details.project.description.clone();
                    
                    // Update state for this page after cloning needed data
                    self.update_state(state);

                    // Project title - User content (Inter)
                    ui.label(ContentText::title(&project_name));

                    ui.add_space(4.0);

                    // Branch selector
                    ui.horizontal(|ui| {
                        // Add label (egui horizontals center content vertically by default)
                        ui.label(WidgetText::label("Branch:"));
                        ui.add_space(4.0);

                        // Load worktree branches if not already loaded
                        if !state.project_detail_worktree_branches_fetch_attempted && !state.loading_project_detail_worktree_branches {
                            let api_service = crate::services::ApiService::new();
                            api_service.refresh_project_detail_worktree_branches(state, self.project_id);
                        }

                        if state.loading_project_detail_worktree_branches {
                            ui.add(egui::Spinner::new());
                        } else {
                            // Set button padding for dropdown widget itself
                            ui.style_mut().spacing.button_padding = egui::vec2(8.0, 6.0);

                            let previous_branch = state.ui_state.project_detail_selected_branch.clone();
                            egui::ComboBox::from_id_salt("project_detail_branch_combo")
                                .width(200.0)
                                .selected_text(
                                    state.ui_state.project_detail_selected_branch
                                        .clone()
                                        .unwrap_or_else(|| "None".to_string()),
                                )
                                .show_ui(ui, |ui| {
                                    // Add padding to dropdown items
                                    ui.style_mut().spacing.item_spacing = egui::vec2(8.0, 4.0);
                                    ui.style_mut().spacing.button_padding = egui::vec2(8.0, 6.0);

                                    let none_response = ui.selectable_value(&mut state.ui_state.project_detail_selected_branch, None, "None");
                                    if none_response.hovered() {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }

                                    for branch in &state.project_detail_worktree_branches {
                                        let branch_response = ui.selectable_value(
                                            &mut state.ui_state.project_detail_selected_branch,
                                            Some(branch.clone()),
                                            branch,
                                        );
                                        if branch_response.hovered() {
                                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                        }
                                    }
                                });

                            // Reset file list when branch changes
                            if previous_branch != state.ui_state.project_detail_selected_branch {
                                {
                                    let mut file_list_result = state.file_list_result.lock().unwrap();
                                    *file_list_result = None;
                                }
                                {
                                    let mut file_content_result = state.file_content_result.lock().unwrap();
                                    *file_content_result = None;
                                }
                                state.ui_state.selected_file_path = None;
                                state.ui_state.current_directory_path = None;
                                state.loading_file_list = false;
                            }
                        }
                    });

                    ui.add_space(8.0);

                    // Tab bar
                    ui.horizontal(|ui| {
                        // Commands tab
                        if ui.selectable_label(
                            state.ui_state.project_detail_selected_tab == ProjectDetailTab::Commands,
                            "Commands"
                        ).clicked() {
                            let was_previously_on_files = state.ui_state.project_detail_selected_tab == ProjectDetailTab::Files;
                            state.ui_state.project_detail_selected_tab = ProjectDetailTab::Commands;
                            
                            // Refresh commands list when tab is opened (especially from Files tab)
                            if was_previously_on_files {
                                state.project_detail_commands_fetch_attempted = false;
                                state.loading_project_detail_commands = false;
                            }
                        }

                        ui.add_space(8.0);

                        // Files tab
                        if ui.selectable_label(
                            state.ui_state.project_detail_selected_tab == ProjectDetailTab::Files,
                            "Files"
                        ).clicked() {
                            state.ui_state.project_detail_selected_tab = ProjectDetailTab::Files;
                        }
                    });

                    ui.add_space(8.0);

                    ui.separator();

                    // Tab content
                    match state.ui_state.project_detail_selected_tab {
                        ProjectDetailTab::Commands => {
                            // Commands tab with two-column layout
                            self.show_commands_tab(ctx, ui, state);
                        }
                        ProjectDetailTab::Files => {
                            // Project metadata - always visible (only for Files tab)
                            ui.horizontal(|ui| {
                                // ID - Ubuntu Light
                                ui.label(WidgetText::label("ID:"));
                                // User content - Inter
                                ui.label(ContentText::text(self.project_id.to_string()));

                                // Only show path when no branch is selected (worktree path is not available in UI)
                                if state.ui_state.project_detail_selected_branch.is_none() {
                                    ui.separator();
                                    ui.label(WidgetText::label("Path:"));
                                    ui.label(ContentText::text(&project_path));
                                }

                                if let Some(description) = &project_description {
                                    ui.separator();
                                    // Label - Ubuntu Light
                                    ui.label(WidgetText::label("Description:"));
                                    // User content - Inter
                                    ui.label(ContentText::text(description));
                                }
                            });

                            ui.add_space(8.0);

                            // Files content
                            self.show_files_tab(ctx, ui, state);

                            ui.add_space(8.0);

                            // Refresh button - only for Files tab
                            // Button - Ubuntu SemiBold
                            if ui.button(WidgetText::button("Refresh")).clicked() {
                                self.refresh_project_details(state);
                            }
                        }
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
    fn is_project_favorite_sync(&self, state: &AppState) -> bool {
        // Check if project is favorite for current server
        if let Some((server_host, server_user, server_port)) = &state.current_server_info {
            let favorite_key = (server_host.clone(), server_user.clone(), *server_port, self.project_id);
            let is_favorite = state.favorite_projects.contains(&favorite_key);
            tracing::trace!(
                "Checking favorite for project {}: key={:?}, is_favorite={}, total_favorites={}",
                self.project_id,
                favorite_key,
                is_favorite,
                state.favorite_projects.len()
            );
            is_favorite
        } else {
            tracing::trace!("No current_server_info set when checking favorite for project {}", self.project_id);
            false
        }
    }

    fn toggle_project_favorite_sync(&self, state: &mut AppState) {
        tracing::debug!("Toggle favorite clicked for project {}", self.project_id);
        if let Some((server_host, server_user, server_port)) = &state.current_server_info {
            let favorite_key = (server_host.clone(), server_user.clone(), *server_port, self.project_id);
            let was_favorite = state.favorite_projects.contains(&favorite_key);
            
            tracing::debug!("Current favorite status: {} for key {:?}", was_favorite, favorite_key);
            
            if was_favorite {
                // Remove favorite
                state.favorite_projects.remove(&favorite_key);
                tracing::debug!("Removed favorite for project {}", self.project_id);
                // Remove from database
                if let Some(db) = &state.db {
                    let result = db.execute(
                        "DELETE FROM favorites WHERE entity_type = 'project' AND entity_id = ? AND server_host = ? AND server_user = ? AND server_port = ?",
                        rusqlite::params![self.project_id, server_host, server_user, server_port],
                    );
                    tracing::debug!("DB delete result: {:?}", result);
                }
            } else {
                // Add favorite
                state.favorite_projects.insert(favorite_key.clone());
                tracing::debug!("Added favorite for project {}", self.project_id);
                // Add to database
                if let Some(db) = &state.db {
                    let created_at = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64;

                    tracing::info!(
                        "DB INSERT params: entity_type='project', entity_id={}, server_host='{}', server_user='{}', server_port={}",
                        self.project_id, server_host, server_user, server_port
                    );

                    let result = db.execute(
                        "INSERT INTO favorites (entity_type, entity_id, server_host, server_user, server_port, created_at) VALUES ('project', ?, ?, ?, ?, ?)",
                        rusqlite::params![self.project_id, server_host, server_user, server_port, created_at],
                    );
                    tracing::info!("DB insert result: {:?}", result);

                    // Verify insert by reading back
                    if let Ok(mut stmt) = db.prepare("SELECT COUNT(*) FROM favorites WHERE entity_type = 'project' AND entity_id = ? AND server_host = ? AND server_user = ? AND server_port = ?") {
                        if let Ok(count) = stmt.query_row(
                            rusqlite::params![self.project_id, server_host, server_user, server_port],
                            |row| row.get::<_, i64>(0)
                        ) {
                            tracing::info!("Verification: Found {} matching rows after insert", count);
                        }
                    }
                }
            }
        } else {
            tracing::debug!("No current_server_info when toggling favorite for project {}", self.project_id);
        }
    }

    fn show_files_tab(&mut self, _ctx: &Context, ui: &mut Ui, state: &mut AppState) {
        // Check if we've switched to a different project and need to clear state
        if state.current_file_browser_project_id != Some(self.project_id) {
            // Clear file browser state when switching projects
            state.current_file_browser_project_id = Some(self.project_id);
            {
                let mut file_list_result = state.file_list_result.lock().unwrap();
                *file_list_result = None;
            }
            {
                let mut file_content_result = state.file_content_result.lock().unwrap();
                *file_content_result = None;
            }
            state.ui_state.selected_file_path = None;
            state.ui_state.expanded_folders.clear();
            state.ui_state.current_directory_path = None;
            state.ui_state.project_detail_selected_branch = None;
            state.project_detail_worktree_branches.clear();
            state.project_detail_worktree_branches_fetch_attempted = false;
            state.loading_file_list = false;
            state.loading_file_content = false;
            state.loading_project_detail_worktree_branches = false;
        }

        // Load file list if not already loaded
        {
            let file_list_result = state.file_list_result.lock().unwrap();
            if file_list_result.is_none() && !state.loading_file_list {
                drop(file_list_result);
                let current_path = state.ui_state.current_directory_path.clone();
                self.load_file_list(state, current_path.as_deref());
            }
        }

        // Two-column layout with independent scroll
        let available_size = ui.available_size_before_wrap();
        let left_column_width = 320.0;

        ui.horizontal(|ui| {
            // First column (320px wide) with file tree
            ui.allocate_ui_with_layout(
                egui::vec2(left_column_width, available_size.y),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    // Ubuntu SemiBold for section heading
                    ui.heading(WidgetText::section_heading("Files"));

                    egui::ScrollArea::vertical()
                        .id_salt("files_tree_scroll")
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            ui.add_space(8.0);

                            if state.loading_file_list {
                                ui.vertical_centered(|ui| {
                                    ui.label(WidgetText::status("Loading files..."));
                                    ui.add(egui::Spinner::new());
                                });
                            } else {
                                let file_list_result = state.file_list_result.lock().unwrap();
                                if let Some(result) = file_list_result.as_ref() {
                                    match result {
                                        Ok(files) => {
                                            let files_clone = files.clone();
                                            drop(file_list_result);
                                            if files_clone.is_empty() {
                                                ui.label(WidgetText::status("No files found"));
                                            } else {
                                                self.render_file_tree(ui, state, &files_clone, "");
                                            }
                                        }
                    Err(e) => {
                                            let e_clone = e.clone();
                                            drop(file_list_result);
                                            ui.label(WidgetText::error(format!(
                                                "Error loading files: {}",
                                                e_clone
                                            )));
                                        }
                                    }
                                }
                            }
                        });
                },
            );

            // Add 1px vertical separator
            let separator_rect = egui::Rect::from_min_size(
                ui.cursor().min,
                egui::vec2(1.0, available_size.y)
            );
            ui.painter().rect_filled(
                separator_rect,
                0.0,
                ui.style().visuals.widgets.noninteractive.bg_stroke.color
            );
            ui.add_space(1.0);

            // Second column (remaining width) with file content
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), available_size.y),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    // Ubuntu SemiBold for section heading
                    let heading = if let Some(path) = &state.ui_state.selected_file_path {
                        format!("File: {}", path)
                    } else {
                        "Select a file to view".to_string()
                    };
                    ui.heading(WidgetText::section_heading(&heading));

                    egui::ScrollArea::vertical()
                        .id_salt("file_content_scroll")
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            ui.add_space(8.0);
                            
                            // Add 8px left/right inner spacing
                            ui.horizontal(|ui| {
                                ui.add_space(8.0);
                                ui.vertical(|ui| {

                                if let Some(path) = &state.ui_state.selected_file_path {
                                    if state.loading_file_content {
                                        ui.vertical_centered(|ui| {
                                            ui.label(WidgetText::status("Loading file content..."));
                                            ui.add(egui::Spinner::new());
                                        });
                                    } else {
                                        let file_content_result =
                                            state.file_content_result.lock().unwrap();
                                        if let Some(result) = file_content_result.as_ref() {
                                            match result {
                                                Ok(content_response) => {
                                                    let content = content_response.content.clone();
                                                    drop(file_content_result);

                                                    // Check if this is a markdown file
                                                    let is_markdown = path.ends_with(".md");

                                                    if is_markdown {
                                                        // Render markdown with styling
                                                        MarkdownRenderer::render(ui, &content);
                                                    } else {
                                                        // Show other files in a monospace font
                                                        ui.add(
                                                            egui::TextEdit::multiline(
                                                                &mut content.as_str(),
                                                            )
                                                            .desired_width(ui.available_width())
                                                            .desired_rows(30)
                                                            .font(egui::TextStyle::Monospace)
                                                            .interactive(false),
                                                        );
                                                    }
                                                }
                    Err(e) => {
                                                    ui.label(WidgetText::error(format!(
                                                        "Error loading file: {}",
                                                        e
                                                    )));
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    ui.vertical_centered(|ui| {
                                        ui.label(WidgetText::status("No file selected"));
                                    });
                                }
                                ui.add_space(8.0);
                                });
                                ui.add_space(8.0);
                            });
                        });
                },
            );
        });
    }

    fn render_file_tree(
        &self,
        ui: &mut Ui,
        state: &mut AppState,
        files: &[manager_models::FileInfo],
        _prefix: &str,
    ) {
        // Show "Go up.." if not in root directory
        if state.ui_state.current_directory_path.is_some() {
            let available_width = ui.available_width();
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(available_width, 24.0),
                egui::Sense::click(),
            );

            // Change cursor to pointer on hover
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            // Add hover background (same as sidebar)
            if response.hovered() {
                ui.painter().rect_filled(rect, 0.0, ui.style().visuals.widgets.hovered.bg_fill);
            }

            // Draw "Go up.." text
            let text_pos = rect.min + egui::vec2(16.0, 4.0);
            ui.painter().text(
                text_pos,
                egui::Align2::LEFT_TOP,
                "⬆ Go up..",
                egui::FontId::new(14.0, egui::FontFamily::Proportional),
                ui.style().visuals.text_color()
            );

            if response.clicked() {
                self.go_up_directory(state);
            }
        }

        let mut directories = Vec::new();
        let mut regular_files = Vec::new();

        // Separate directories and files, sort them
        for file in files {
            if file.is_directory {
                directories.push(file);
            } else {
                regular_files.push(file);
            }
        }

        directories.sort_by(|a, b| a.name.cmp(&b.name));
        regular_files.sort_by(|a, b| a.name.cmp(&b.name));

        // Render directories first
        for directory in directories {
            // Use the path directly from the API response, which is already relative to project root
            let full_path = directory.path.clone();

            let is_expanded = state.ui_state.expanded_folders.contains(&full_path);

            let available_width = ui.available_width();
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(available_width, 24.0),
                egui::Sense::click(),
            );

            // Change cursor to pointer on hover
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            // Add hover background (same as sidebar)
            if response.hovered() {
                ui.painter().rect_filled(rect, 0.0, ui.style().visuals.widgets.hovered.bg_fill);
            }

            // Draw folder icon and name
            let icon = if is_expanded { "📂" } else { "📁" };
            let text_pos = rect.min + egui::vec2(16.0, 4.0);
            ui.painter().text(
                text_pos,
                egui::Align2::LEFT_TOP,
                format!("{} {}", icon, directory.name),
                egui::FontId::new(14.0, egui::FontFamily::Proportional),
                ui.style().visuals.text_color()
            );

            if response.clicked() {
                // Navigate into directory
                self.navigate_into_directory(state, &full_path);
            }

            // If expanded, render children (we'd need to have loaded them)
            if is_expanded {
                // For now, we'll just show a placeholder
                ui.horizontal(|ui| {
                    ui.label("    ");
                    ui.label(WidgetText::status("Loading..."));
                });
            }
        }

        // Render files
        for file in regular_files {
            // Use the path directly from the API response, which is already relative to project root
            let full_path = file.path.clone();

            let available_width = ui.available_width();
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(available_width, 24.0),
                egui::Sense::click(),
            );

            // Change cursor to pointer on hover
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            // Add hover background (same as sidebar)
            if response.hovered() {
                ui.painter().rect_filled(rect, 0.0, ui.style().visuals.widgets.hovered.bg_fill);
            }

            // Draw file icon and name
            let icon = self.get_file_icon(&file.name);
            let text_pos = rect.min + egui::vec2(16.0, 4.0);
            ui.painter().text(
                text_pos,
                egui::Align2::LEFT_TOP,
                format!("{} {}", icon, file.name),
                egui::FontId::new(14.0, egui::FontFamily::Proportional),
                ui.style().visuals.text_color()
            );

            if response.clicked() {
                state.ui_state.selected_file_path = Some(full_path.clone());
                self.load_file_content(state, &full_path);
            }
        }
    }

    fn get_file_icon(&self, filename: &str) -> &'static str {
        if filename.ends_with(".rs") {
            "🦀"
        } else if filename.ends_with(".js") || filename.ends_with(".ts") {
            "📜"
        } else if filename.ends_with(".json") {
            "📋"
        } else if filename.ends_with(".md") {
            "📝"
        } else if filename.ends_with(".yml") || filename.ends_with(".yaml") {
            "⚙️"
        } else if filename.ends_with(".toml") {
            "🔧"
        } else {
            "📄"
        }
    }

    fn load_file_list(&self, state: &mut AppState, path: Option<&str>) {
        if state.connection_state == ConnectionState::Connected {
            state.loading_file_list = true;
            {
                let mut file_list_result = state.file_list_result.lock().unwrap();
                *file_list_result = None;
            }

            let connection_manager = Arc::clone(&state.connection_manager);
            let project_id = self.project_id;
            let path = path.map(|p| p.to_string());
            let git_branch = state.ui_state.project_detail_selected_branch.clone();
            let file_list_result_clone = Arc::clone(&state.file_list_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.list_files(project_id, path.as_deref(), git_branch.as_deref()).await;
                    let mut file_list_result = file_list_result_clone.lock().unwrap();
                    *file_list_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut file_list_result = file_list_result_clone.lock().unwrap();
                    *file_list_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    fn load_file_content(&self, state: &mut AppState, path: &str) {
        if state.connection_state == ConnectionState::Connected {
            state.loading_file_content = true;
            {
                let mut file_content_result = state.file_content_result.lock().unwrap();
                *file_content_result = None;
            }

            let connection_manager = Arc::clone(&state.connection_manager);
            let project_id = self.project_id;
            let path = path.to_string();
            let git_branch = state.ui_state.project_detail_selected_branch.clone();
            let file_content_result_clone = Arc::clone(&state.file_content_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.get_file_content(project_id, &path, git_branch.as_deref()).await;
                    let mut file_content_result = file_content_result_clone.lock().unwrap();
                    *file_content_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut file_content_result = file_content_result_clone.lock().unwrap();
                    *file_content_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }



    fn navigate_into_directory(&self, state: &mut AppState, directory_path: &str) {
        // Set current directory path
        state.ui_state.current_directory_path = Some(directory_path.to_string());
        
        // Clear file list result to trigger reload
        {
            let mut file_list_result = state.file_list_result.lock().unwrap();
            *file_list_result = None;
        }
        
        // Load files for this directory
        self.load_file_list(state, Some(directory_path));
    }

    fn go_up_directory(&self, state: &mut AppState) {
        if let Some(current_path) = &state.ui_state.current_directory_path {
            // Get parent directory
            if let Some(parent_path) = std::path::Path::new(current_path).parent() {
                let parent_path_str = parent_path.to_string_lossy().to_string();
                if parent_path_str.is_empty() {
                    // We're going back to root
                    state.ui_state.current_directory_path = None;
                } else {
                    state.ui_state.current_directory_path = Some(parent_path_str);
                }
            } else {
                // No parent, go to root
                state.ui_state.current_directory_path = None;
            }
            
            // Clear file list result to trigger reload
            {
                let mut file_list_result = state.file_list_result.lock().unwrap();
                *file_list_result = None;
            }
            
            // Load files for the parent directory
            let current_path = state.ui_state.current_directory_path.clone();
            self.load_file_list(state, current_path.as_deref());
        }
    }

    fn update_state(&mut self, state: &mut AppState) {
        // Check if we've switched to a different project and need to clear state
        if state.current_file_browser_project_id != Some(self.project_id) {
            // Clear command state when switching projects
            state.project_detail_saved_commands.clear();
            state.project_detail_commands_fetch_attempted = false;
            state.loading_project_detail_commands = false;
            state.ui_state.project_detail_command_discovery_results = None;
            state.ui_state.project_detail_command_selected_items.clear();
            state.ui_state.project_detail_show_discovery_form = false;
            state.loading_command_discovery = false;
            state.ui_state.project_detail_selected_command_id = None;
            state.ui_state.project_detail_command_executions.clear();
            state.loading_command_executions = false;
        }

        // Load saved commands if not already loaded
        if !state.project_detail_commands_fetch_attempted && !state.loading_project_detail_commands {
            let api_service = crate::services::ApiService::new();
            api_service.list_project_commands(self.project_id, state);
        }

        // Check for command list results
        {
            let commands_result = state.project_detail_saved_commands_result.lock().unwrap();
            if let Some(result) = commands_result.as_ref() {
                match result {
                    Ok(commands) => {
                        state.project_detail_saved_commands = commands.clone();
                        state.loading_project_detail_commands = false;
                    }
                    Err(_e) => {
                        state.loading_project_detail_commands = false;
                        // Error will be shown in UI
                    }
                }
                // Clear the result to avoid reprocessing
                drop(commands_result);
                let mut commands_result = state.project_detail_saved_commands_result.lock().unwrap();
                *commands_result = None;
            }
        }

        // Check for discovery results
        {
            let discovery_result = state.command_discovery_result.lock().unwrap();
            if let Some(result) = discovery_result.as_ref() {
                match result {
                    Ok(discovery) => {
                        state.ui_state.project_detail_command_discovery_results = Some(discovery.clone());
                        state.loading_command_discovery = false;
                    }
                    Err(_e) => {
                        state.loading_command_discovery = false;
                        // Error will be shown in UI
                    }
                }
                // Clear the result to avoid reprocessing
                drop(discovery_result);
                let mut discovery_result = state.command_discovery_result.lock().unwrap();
                *discovery_result = None;
            }
        }

        // Check for create commands results
        {
            let create_result = state.create_commands_result.lock().unwrap();
            if let Some(result) = create_result.as_ref() {
                match result {
                    Ok(_created_commands) => {
                        // Success: refresh saved commands list
                        state.project_detail_commands_fetch_attempted = false;

                        // Clear discovery UI state
                        state.ui_state.project_detail_command_discovery_results = None;
                        state.ui_state.project_detail_command_selected_items.clear();
                        state.ui_state.project_detail_show_discovery_form = false;
                        state.loading_command_discovery = false;
                    }
                    Err(_e) => {
                        // Error will be shown in UI
                    }
                }
                // Clear the result to avoid reprocessing
                drop(create_result);
                let mut create_result = state.create_commands_result.lock().unwrap();
                *create_result = None;
            }
        }

        // Check for command execution results
        {
            let execution_result = state.command_executions_result.lock().unwrap();
            if let Some(result) = execution_result.as_ref() {
                match result {
                    Ok(executions) => {
                        state.ui_state.project_detail_command_executions = executions.clone();
                        state.loading_command_executions = false;
                    }
                    Err(_e) => {
                        state.loading_command_executions = false;
                        // Error will be shown in UI
                    }
                }
                // Clear the result to avoid reprocessing
                drop(execution_result);
                let mut execution_result = state.command_executions_result.lock().unwrap();
                *execution_result = None;
            }
        }

        // Check for worktree branches results
        {
            let branches_result = state.project_detail_worktree_branches_result.lock().unwrap();
            if let Some(result) = branches_result.as_ref() {
                match result {
                    Ok(branches) => {
                        state.ui_state.project_detail_worktree_branches = branches.clone();
                        state.loading_project_detail_worktree_branches = false;
                    }
                    Err(_e) => {
                        state.loading_project_detail_worktree_branches = false;
                        // Error will be shown in UI
                    }
                }
                // Clear the result to avoid reprocessing
                drop(branches_result);
                let mut branches_result = state.project_detail_worktree_branches_result.lock().unwrap();
                *branches_result = None;
            }
        }

        // Update current file browser project ID
        state.current_file_browser_project_id = Some(self.project_id);
    }

    fn show_discover_cta(&self, ui: &mut Ui, state: &mut AppState) {
        // Gray color scheme for CTA button
        let (bg_color, border_color) = if ui.rect_contains_pointer(ui.max_rect()) {
            (
                egui::Color32::from_rgb(220, 220, 220), // Lighter gray on hover
                egui::Color32::from_rgb(160, 160, 160), // Medium gray border on hover
            )
        } else {
            (
                egui::Color32::from_rgb(200, 200, 200), // Light gray
                egui::Color32::from_rgb(180, 180, 180), // Darker gray border
            )
        };
        let text_color = egui::Color32::from_rgb(40, 40, 40); // Dark gray text

        let response = egui::Frame::NONE
            .fill(bg_color)
            .stroke(egui::Stroke::new(1.0, border_color))
            .corner_radius(8.0)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("Discover commands")
                            .size(16.0)
                            .family(egui::FontFamily::Name("ui_semibold".into()))
                            .color(text_color),
                    );
                });
            })
            .response;

        // Change cursor to pointer on hover
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        if response.interact(egui::Sense::click()).clicked() {
            state.ui_state.project_detail_show_discovery_form = true;
        }
    }

    fn show_saved_commands_list(&self, ui: &mut Ui, state: &mut AppState) {
        if state.loading_project_detail_commands {
            ui.vertical_centered(|ui| {
                ui.label(WidgetText::status("Loading commands..."));
                ui.add(egui::Spinner::new());
            });
        } else if state.project_detail_saved_commands.is_empty() {
            ui.label(WidgetText::status("No saved commands"));
        } else {
            // Clone commands to avoid borrow issues
            let commands = state.project_detail_saved_commands.clone();
            for command in &commands {
                let command_id = command.id.clone();

                // Get last execution time for this command
                let last_executed_at = state.ui_state.project_detail_command_executions
                    .iter()
                    .find(|exec| exec.command_id == command_id)
                    .map(|exec| exec.executed_at);

                let response = CommandCard::new(command)
                    .last_executed_at(last_executed_at)
                    .ui(ui);

                if response.clicked() {
                    // Select this command and hide discovery form
                    state.ui_state.project_detail_selected_command_id = Some(command_id.clone());
                    state.ui_state.project_detail_show_discovery_form = false;
                    
                    // Load execution history for this command
                    let api_service = crate::services::ApiService::new();
                    api_service.get_command_executions(self.project_id, &command_id, state);
                }
            }
        }
    }

    fn show_discovery_form(&self, ui: &mut Ui, state: &mut AppState) {
        // Ubuntu SemiBold for section heading
        ui.heading(WidgetText::section_heading("Command Discovery"));

        ui.add_space(8.0);

        // Start Discovery button
        if state.loading_command_discovery {
            ui.vertical_centered(|ui| {
                ui.label(WidgetText::status("Discovering commands..."));
                ui.add(egui::Spinner::new());
            });
        } else {
            if ui.button(WidgetText::button("Start Discovery")).clicked() {
                let api_service = crate::services::ApiService::new();
                api_service.discover_project_commands(self.project_id, Some(false), state);
            }
        }

        ui.add_space(16.0);

        // Show discovered commands
        if let Some(results) = state.ui_state.project_detail_command_discovery_results.clone() {
            ui.heading(WidgetText::section_heading("Discovered Commands"));
            
            egui::ScrollArea::vertical()
                .id_salt("discovered_commands_scroll")
                .auto_shrink(false)
                .show(ui, |ui| {
                    ui.add_space(8.0);

                    for command in &results.commands {
                        let command_name = &command.name;
                        let command_desc = command.description.as_deref();

                        // Checkbox for selection
                        ui.horizontal(|ui| {
                            let mut is_selected = state.ui_state.project_detail_command_selected_items
                                .contains(command_name);

                            if ui.checkbox(&mut is_selected, "").changed() {
                                if is_selected {
                                    state.ui_state.project_detail_command_selected_items.insert(command_name.to_string());
                                } else {
                                    state.ui_state.project_detail_command_selected_items.remove(command_name);
                                }
                            }

                            // Command name
                            ui.label(command_name);
                        });

                        // Command description
                        if let Some(desc) = command_desc {
                            ui.label(desc);
                        }

                        ui.add_space(8.0);
                    }

                    // Save selected commands button
                    if !state.ui_state.project_detail_command_selected_items.is_empty() {
                        ui.add_space(16.0);
                        if ui.button(WidgetText::button("Save selected commands")).clicked() {
                            // Filter selected commands from discovery results
                            let selected_items = state.ui_state.project_detail_command_selected_items.clone();
                            let selected_commands: Vec<manager_models::ProjectCommand> = results.commands
                                .iter()
                                .filter(|cmd| selected_items.contains(&cmd.name))
                                .map(|cmd| cmd.to_project_command(self.project_id))
                                .collect();

                            let api_service = crate::services::ApiService::new();
                            api_service.create_project_commands(self.project_id, selected_commands, state);
                        }
                    }
                });
        } else {
            ui.label(WidgetText::status("No commands discovered"));
        }
    }

    fn show_command_details(&self, ui: &mut Ui, state: &mut AppState, command_id: &str) {
        // Ubuntu SemiBold for section heading
        ui.heading(WidgetText::section_heading("Command Details"));

        ui.add_space(8.0);

        // Find the command details
        if let Some(command) = state.project_detail_saved_commands
            .iter()
            .find(|cmd| cmd.id == command_id) {
            
            // Command name and description
            ui.label(ContentText::title(&command.name));
            
            if let Some(description) = &command.description {
                ui.add_space(4.0);
                ui.label(ContentText::description(ui, description));
            }

            ui.add_space(12.0);

            // Command details
            ui.label(WidgetText::section_heading("Command"));
            ui.add_space(4.0);
            ui.label(ContentText::code_inline(&command.command));

            if let Some(working_dir) = &command.working_directory {
                ui.add_space(8.0);
                ui.label(WidgetText::label("Working Directory:"));
                ui.label(ContentText::text(working_dir));
            }

            if let Some(shell) = &command.shell {
                ui.add_space(8.0);
                ui.label(WidgetText::label("Shell:"));
                ui.label(ContentText::text(shell));
            }

            ui.add_space(16.0);

            // Execution history
            ui.heading(WidgetText::section_heading("Execution History"));
            ui.add_space(8.0);

            if state.loading_command_executions {
                ui.vertical_centered(|ui| {
                    ui.label(WidgetText::status("Loading execution history..."));
                    ui.add(egui::Spinner::new());
                });
            } else if state.ui_state.project_detail_command_executions.is_empty() {
                ui.label(WidgetText::status("No execution history"));
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("command_executions_scroll")
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        for execution in &state.ui_state.project_detail_command_executions {
                            self.show_execution_entry(ui, execution);
                        }
                    });
            }

            ui.add_space(16.0);

            // Execute button
            let selected_branch = state.ui_state.project_detail_selected_branch.clone();
            if ui.button(WidgetText::button("Execute Command")).clicked() {
                let api_service = crate::services::ApiService::new();
                api_service.execute_project_command(
                    self.project_id, 
                    command_id, 
                    selected_branch.as_deref(),
                    state
                );
            }
        } else {
            ui.label(WidgetText::error("Command not found"));
        }
    }

    fn show_execution_entry(&self, ui: &mut Ui, execution: &manager_models::ProjectCommandExecution) {
        let executed_at = chrono::DateTime::from_timestamp(execution.executed_at, 0)
            .unwrap_or_else(|| chrono::Utc::now());
        let formatted_time = executed_at.format("%Y-%m-%d %H:%M:%S").to_string();

        // Execution header
        ui.horizontal(|ui| {
            ui.label(WidgetText::label(&formatted_time));
            
            // Status indicator
            let status_text = if execution.success {
                WidgetText::success("✓ Success")
            } else {
                WidgetText::error("✗ Failed")
            };
            ui.label(status_text);

            // Duration
            let duration_text = if execution.duration_ms < 1000 {
                format!("{}ms", execution.duration_ms)
            } else {
                format!("{:.1}s", execution.duration_ms as f64 / 1000.0)
            };
            ui.label(WidgetText::muted(&duration_text));
        });

        // Exit code
        if let Some(exit_code) = execution.exit_code {
            ui.label(WidgetText::muted(&format!("Exit code: {}", exit_code)));
        }

        // Git branch if present
        if let Some(branch) = &execution.git_branch {
            ui.label(WidgetText::muted(&format!("Branch: {}", branch)));
        }

        // Output
        if !execution.stdout.is_empty() {
            ui.add_space(4.0);
            ui.label(WidgetText::label("Output:"));
            ui.add(
                egui::TextEdit::multiline(&mut execution.stdout.as_str())
                    .desired_width(ui.available_width())
                    .desired_rows(6)
                    .font(egui::TextStyle::Monospace)
                    .interactive(false),
            );
        }

        // Error output
        if !execution.stderr.is_empty() {
            ui.add_space(4.0);
            ui.label(WidgetText::error("Error:"));
            ui.add(
                egui::TextEdit::multiline(&mut execution.stderr.as_str())
                    .desired_width(ui.available_width())
                    .desired_rows(4)
                    .font(egui::TextStyle::Monospace)
                    .interactive(false),
            );
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);
    }

    fn show_command_details_placeholder(&self, ui: &mut Ui, _state: &mut AppState) {
        // Ubuntu SemiBold for section heading
        ui.heading(WidgetText::section_heading("Command Details"));

        ui.add_space(8.0);

        ui.vertical_centered(|ui| {
            ui.label(WidgetText::status("Select a command or discover new commands"));
        });
    }

    fn refresh_project_details(&self, state: &mut AppState) {
        let api_service = crate::services::ApiService::new();
        api_service.refresh_project_details(self.project_id, state);
    }

    fn show_commands_tab(&mut self, _ctx: &Context, ui: &mut Ui, state: &mut AppState) {
        // Two-column layout for commands
        // Capture full available size BEFORE starting horizontal layout
        let available_size = ui.available_size_before_wrap();

        // Calculate column widths
        let left_column_width = 400.0;
        let spacing = ui.spacing().item_spacing.x;

        ui.horizontal(|ui| {
            // LEFT COLUMN (400px wide) - saved commands
            ui.allocate_ui_with_layout(
                egui::vec2(left_column_width, available_size.y),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    // Saved commands section
                    ui.horizontal(|ui| {
                        ui.heading(WidgetText::section_heading("Saved Commands"));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(WidgetText::button("Refresh")).clicked() {
                                // Refresh commands list
                                state.project_detail_commands_fetch_attempted = false;
                                state.loading_project_detail_commands = false;
                                let api_service = crate::services::ApiService::new();
                                api_service.list_project_commands(self.project_id, state);
                            }
                        });
                    });

                    egui::ScrollArea::vertical()
                        .id_salt("commands_left_column_scroll")
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            ui.add_space(8.0);

                            if state.loading_project_detail_commands {
                                ui.vertical_centered(|ui| {
                                    ui.label(WidgetText::status("Loading commands..."));
                                    ui.add(egui::Spinner::new());
                                });
                            } else if state.project_detail_saved_commands.is_empty() {
                                self.show_discover_cta(ui, state);
                            } else {
                                self.show_saved_commands_list(ui, state);
                            }
                        });
                },
            );

            // Separator
            ui.separator();

            // RIGHT COLUMN (fills remaining width) - discovery or command details
            // Account for: left column width + spacing after left column + separator width + spacing after separator
            let right_column_width = available_size.x - left_column_width - (spacing * 3.0);
            ui.allocate_ui_with_layout(
                egui::vec2(right_column_width, available_size.y),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    // Show discovery form or command details
                    if state.ui_state.project_detail_show_discovery_form {
                        self.show_discovery_form(ui, state);
                    } else if let Some(selected_command_id) = state.ui_state.project_detail_selected_command_id.clone() {
                        self.show_command_details(ui, state, &selected_command_id);
                    } else {
                        self.show_command_details_placeholder(ui, state);
                    }
                },
            );
        });
    }
}

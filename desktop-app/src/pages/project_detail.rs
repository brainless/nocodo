use crate::content_renderer::MarkdownRenderer;
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

                    // Tab navigation - styled to look like tabs
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0; // Remove spacing between tabs

                        let tabs = [
                            (ProjectDetailTab::Work, "Work"),
                            (ProjectDetailTab::Files, "Files"),
                        ];

                        for (tab, label) in tabs {
                            let is_selected = state.ui_state.project_detail_tab == tab;

                            // Create a styled button that looks like a tab
                            let mut button = egui::Button::new(
                                egui::RichText::new(label)
                                    .family(egui::FontFamily::Name("ui_semibold".into())),
                            )
                            .selected(is_selected);

                            // Remove frame when not selected to create tab-like appearance
                            if !is_selected {
                                button = button.frame(false);
                            }

                            // Add padding to make tabs more spacious
                            button = button.min_size(egui::vec2(100.0, 32.0));

                            let response = ui.add(button);

                            if response.clicked() {
                                state.ui_state.project_detail_tab = tab;
                            }

                            // Draw underline for selected tab
                            if is_selected {
                                let rect = response.rect;
                                let stroke = egui::Stroke::new(2.0, ui.visuals().selection.bg_fill);
                                ui.painter().line_segment(
                                    [
                                        egui::pos2(rect.left(), rect.bottom()),
                                        egui::pos2(rect.right(), rect.bottom()),
                                    ],
                                    stroke,
                                );
                            }
                        }
                    });

                    ui.separator();

                    // Project metadata - always visible
                    ui.horizontal(|ui| {
                        // ID - Ubuntu Light
                        ui.label(WidgetText::label("ID:"));
                        // User content - Inter
                        ui.label(ContentText::text(self.project_id.to_string()));

                        ui.separator();

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

                    ui.add_space(8.0);

                    // Tab content
                    match state.ui_state.project_detail_tab {
                        ProjectDetailTab::Work => {
                            self.show_work_tab(ctx, ui, state);
                        }
                        ProjectDetailTab::Files => {
                            self.show_files_tab(ctx, ui, state);
                        }
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
            state.loading_file_list = false;
            state.loading_file_content = false;
        }

        // Load file list if not already loaded
        {
            let file_list_result = state.file_list_result.lock().unwrap();
            if file_list_result.is_none() && !state.loading_file_list {
                drop(file_list_result);
                self.load_file_list(state, None);
            }
        }

        // Two-column layout with independent scroll
        let available_size = ui.available_size_before_wrap();
        let left_column_width = 400.0;

        ui.horizontal(|ui| {
            // First column (400px wide) with file tree
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

            ui.add_space(16.0);

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

            ui.horizontal(|ui| {
                // Add 2 spaces for hierarchy
                ui.label("  ");

                // Folder icon and expand/collapse button
                let icon = if is_expanded { "📂" } else { "📁" };
                if ui.button(format!("{} {}", icon, directory.name)).clicked() {
                    if is_expanded {
                        state.ui_state.expanded_folders.remove(&full_path);
                    } else {
                        state.ui_state.expanded_folders.insert(full_path.clone());
                        // Load contents of this directory
                        self.load_file_list(state, Some(&full_path));
                    }
                }
            });

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

            ui.horizontal(|ui| {
                // Add 2 spaces for hierarchy
                ui.label("  ");

                // File icon and name
                let icon = self.get_file_icon(&file.name);
                let _is_selected = state.ui_state.selected_file_path.as_ref() == Some(&full_path);

                let button_text = WidgetText::button(format!("{} {}", icon, file.name));

                if ui.button(button_text).clicked() {
                    state.ui_state.selected_file_path = Some(full_path.clone());
                    self.load_file_content(state, &full_path);
                }
            });
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
            let file_list_result_clone = Arc::clone(&state.file_list_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.list_files(project_id, path.as_deref()).await;
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
            let file_content_result_clone = Arc::clone(&state.file_content_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.get_file_content(project_id, &path).await;
                    let mut file_content_result = file_content_result_clone.lock().unwrap();
                    *file_content_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut file_content_result = file_content_result_clone.lock().unwrap();
                    *file_content_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    fn show_work_tab(&mut self, _ctx: &Context, ui: &mut Ui, state: &mut AppState) {
        // Trigger refresh if needed
        if state.ui_state.pending_works_refresh {
            state.ui_state.pending_works_refresh = false;
            if state.connection_state == ConnectionState::Connected && !state.loading_works {
                let api_service = crate::services::ApiService::new();
                api_service.refresh_works(state);
            }
        }

        // Two-column layout: left column for work list, right column for details
        let show_second_column = true;
        let available_size = ui.available_size_before_wrap();
        let left_column_width = 400.0;
        let spacing = ui.spacing().item_spacing.x;

        ui.horizontal(|ui| {
            // Left column (400px wide) - work list
            ui.allocate_ui_with_layout(
                egui::vec2(left_column_width, available_size.y),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    // CTA button to show create work form (only show when connected)
                    if matches!(state.connection_state, ConnectionState::Connected) {
                        let response = ui
                            .allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), 0.0),
                                egui::Layout::top_down(egui::Align::LEFT),
                                |ui| {
                                    // Pastel green colors for primary CTA
                                    let (bg_color, border_color) = if ui.rect_contains_pointer(ui.max_rect()) {
                                        (
                                            egui::Color32::from_rgb(152, 251, 152), // Lighter green on hover
                                            egui::Color32::from_rgb(144, 238, 144), // Medium green border on hover
                                        )
                                    } else {
                                        (
                                            egui::Color32::from_rgb(144, 238, 144), // Darker pastel green
                                            egui::Color32::from_rgb(120, 200, 120), // Darker green border
                                        )
                                    };
                                    let text_color = egui::Color32::from_rgb(40, 80, 40); // Dark green text

                                    egui::Frame::NONE
                                        .fill(bg_color)
                                        .stroke(egui::Stroke::new(1.0, border_color))
                                        .corner_radius(8.0)
                                        .inner_margin(egui::Margin::same(12))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.vertical_centered(|ui| {
                                                ui.label(
                                                    egui::RichText::new("What do you want to create?")
                                                        .size(16.0)
                                                        .family(egui::FontFamily::Name(
                                                            "ui_semibold".into(),
                                                        ))
                                                        .color(text_color),
                                                );
                                            });
                                        })
                                        .response
                                },
                            )
                            .inner;

                        if response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }

                        if response.interact(egui::Sense::click()).clicked() {
                            // Clear selection to show form in second column
                            state.ui_state.selected_work_id = None;
                        }

                        ui.add_space(16.0);
                    }

                    // Work list filtered for this project
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
                            if state.loading_works {
                                ui.vertical_centered(|ui| {
                                    ui.label("Loading work...");
                                    ui.add(egui::Spinner::new());
                                });
                            } else {
                                // Filter works for this project
                                let project_works: Vec<_> = state.works.iter()
                                    .filter(|work| work.project_id == Some(self.project_id))
                                    .collect();

                                if project_works.is_empty() {
                                    ui.vertical_centered(|ui| {
                                        ui.label("No work found for this project");
                                        if ui.button("Refresh").clicked() {
                                            let api_service = crate::services::ApiService::new();
                                            api_service.refresh_works(state);
                                        }
                                    });
                                } else {
                                    egui::ScrollArea::vertical()
                                        .id_salt("project_work_list_scroll")
                                        .auto_shrink(false)
                                        .show(ui, |ui| {
                                            ui.add_space(8.0);

                                            // Sort works by created_at (most recent first)
                                            let mut sorted_works = project_works;
                                            sorted_works.sort_by(|a, b| b.created_at.cmp(&a.created_at));

                                            for work in sorted_works {
                                                let work_id = work.id;
                                                let is_selected = state.ui_state.selected_work_id == Some(work_id);

                                                // Card frame with different styling for selected item
                                                let frame_fill = if is_selected {
                                                    ui.style().visuals.selection.bg_fill
                                                } else {
                                                    ui.style().visuals.widgets.inactive.bg_fill
                                                };

                                                let item_response = ui.allocate_ui_with_layout(
                                                    egui::vec2(ui.available_width(), 0.0),
                                                    egui::Layout::top_down(egui::Align::LEFT),
                                                    |ui| {
                                                        egui::Frame::NONE
                                                            .fill(frame_fill)
                                                            .corner_radius(8.0)
                                                            .inner_margin(egui::Margin::same(12))
                                                            .show(ui, |ui| {
                                                                ui.set_width(ui.available_width());
                                                                ui.vertical(|ui| {
                                                                    // Work title - larger and bold
                                                                    ui.label(egui::RichText::new(&work.title).size(16.0).strong());

                                                                    ui.add_space(4.0);

                                                                    // Metadata row
                                                                    ui.horizontal(|ui| {
                                                                        // Status badge
                                                                        egui::Frame::NONE
                                                                            .fill(ui.style().visuals.selection.bg_fill)
                                                                            .corner_radius(4.0)
                                                                            .inner_margin(egui::Margin::symmetric(8, 4))
                                                                            .show(ui, |ui| {
                                                                                ui.label(egui::RichText::new(&work.status).size(11.0));
                                                                            });

                                                                        // Model display name if present
                                                                        if let Some(model_id) = &work.model {
                                                                            let model_display_name = state.supported_models.iter()
                                                                                .find(|m| m.model_id == *model_id)
                                                                                .map(|m| m.name.clone())
                                                                                .unwrap_or_else(|| model_id.clone());

                                                                            egui::Frame::NONE
                                                                                .fill(ui.style().visuals.selection.bg_fill)
                                                                                .corner_radius(4.0)
                                                                                .inner_margin(egui::Margin::symmetric(8, 4))
                                                                                .show(ui, |ui| {
                                                                                    ui.label(egui::RichText::new(&model_display_name).size(11.0));
                                                                                });
                                                                        }
                                                                    });
                                                                });
                                                            });
                                                    }
                                                );

                                                if item_response.response.interact(egui::Sense::click()).clicked() {
                                                    if state.ui_state.selected_work_id != Some(work_id) {
                                                        state.ui_state.reset_work_details_scroll = true;
                                                    }
                                                    state.ui_state.selected_work_id = Some(work_id);
                                                    let api_service = crate::services::ApiService::new();
                                                    api_service.refresh_work_messages(work_id, state);
                                                }

                                                if item_response.response.hovered() {
                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                }

                                                ui.add_space(8.0);
                                            }
                                        });
                                }
                            }
                        }
                        ConnectionState::Error(error) => {
                            ui.vertical_centered(|ui| {
                                ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
                            });
                        }
                    }
                }
            );

            if show_second_column {
                // Separator
                ui.separator();

                // Right column (fills remaining width) - Work details
                let right_column_width = available_size.x - left_column_width - (spacing * 3.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(right_column_width, available_size.y),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        if let Some(selected_work_id) = state.ui_state.selected_work_id {
                            // Find the selected work
                            if let Some(work) = state.works.iter().find(|w| w.id == selected_work_id) {
                                // Work details
                                ui.heading(&work.title);

                                ui.add_space(4.0);

                                // Work metadata
                                ui.horizontal(|ui| {
                                    ui.label("Status:");
                                    ui.label(&work.status);

                                    if let Some(model_id) = &work.model {
                                        let model_display_name = state.supported_models.iter()
                                            .find(|m| m.model_id == *model_id)
                                            .map(|m| m.name.clone())
                                            .unwrap_or_else(|| model_id.clone());

                                        ui.separator();
                                        ui.label("Model:");
                                        ui.label(&model_display_name);
                                    }

                                    if let Some(project_id) = work.project_id {
                                        if let Some(project) = state.projects.iter().find(|p| p.id == project_id) {
                                            ui.separator();
                                            ui.label("Project:");
                                            ui.label(&project.name);
                                        }
                                    }
                                });

                                ui.separator();

                                // Message history
                                ui.heading("Message History");

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
                                        if state.loading_work_messages || state.loading_ai_session_outputs || state.loading_ai_tool_calls {
                                            ui.vertical_centered(|ui| {
                                                ui.label("Loading messages...");
                                                ui.add(egui::Spinner::new());
                                            });
                                        } else if state.work_messages.is_empty() && state.ai_session_outputs.is_empty() && state.ai_tool_calls.is_empty() {
                                            ui.vertical_centered(|ui| {
                                                ui.label("No messages found");
                                                if ui.button("Refresh").clicked() {
                                                    let api_service = crate::services::ApiService::new();
                                                    api_service.refresh_work_messages(selected_work_id, state);
                                                }
                                            });
                                        } else {
                                            // Reserve space for reply form at bottom
                                            let reply_form_height = 120.0;
                                            let available_height = ui.available_height() - reply_form_height;
                                            let available_width = ui.available_width();

                                            let mut scroll_area = egui::ScrollArea::vertical()
                                                .id_salt("project_work_messages_scroll")
                                                .auto_shrink(false)
                                                .max_height(available_height)
                                                .max_width(available_width);

                                            if state.ui_state.reset_work_details_scroll {
                                                scroll_area = scroll_area.vertical_scroll_offset(0.0);
                                                state.ui_state.reset_work_details_scroll = false;
                                            }

                                            scroll_area.show(ui, |ui| {
                                                ui.add_space(8.0);

                                                // Combine and sort all messages by timestamp
                                                #[derive(Clone)]
                                                enum DisplayMessage {
                                                    WorkMessage(manager_models::WorkMessage),
                                                    AiOutput(manager_models::AiSessionOutput),
                                                }

                                                let mut all_messages: Vec<(i64, DisplayMessage)> = Vec::new();

                                                for msg in &state.work_messages {
                                                    all_messages.push((msg.created_at, DisplayMessage::WorkMessage(msg.clone())));
                                                }

                                                for output in &state.ai_session_outputs {
                                                    all_messages.push((output.created_at, DisplayMessage::AiOutput(output.clone())));
                                                }

                                                all_messages.sort_by_key(|(timestamp, _)| *timestamp);

                                                for (_timestamp, message) in &all_messages {
                                                    match message {
                                                        DisplayMessage::WorkMessage(msg) => {
                                                            let bg_color = ui.style().visuals.widgets.inactive.bg_fill;

                                                            egui::Frame::NONE
                                                                .fill(bg_color)
                                                                .corner_radius(8.0)
                                                                .inner_margin(egui::Margin::same(12))
                                                                .show(ui, |ui| {
                                                                    ui.vertical(|ui| {
                                                                        ui.horizontal(|ui| {
                                                                            ui.label(egui::RichText::new("User").size(12.0).strong());
                                                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                                                let datetime = chrono::DateTime::from_timestamp(msg.created_at, 0)
                                                                                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                                                                    .unwrap_or_else(|| "Unknown".to_string());
                                                                                ui.label(egui::RichText::new(datetime).size(10.0).color(ui.style().visuals.weak_text_color()));
                                                                            });
                                                                        });
                                                                        ui.add_space(4.0);
                                                                        ui.label(&msg.content);
                                                                    });
                                                                });
                                                        }
                                                        DisplayMessage::AiOutput(output) => {
                                                            // Show regular AI response
                                                            let bg_color = ui.style().visuals.widgets.noninteractive.bg_fill;

                                                            egui::Frame::NONE
                                                                .fill(bg_color)
                                                                .corner_radius(8.0)
                                                                .inner_margin(egui::Margin::same(12))
                                                                .show(ui, |ui| {
                                                                    ui.vertical(|ui| {
                                                                        ui.horizontal(|ui| {
                                                                            let label = match (output.role.as_deref(), output.model.as_deref()) {
                                                                                (Some("tool"), _) => "nocodo".to_string(),
                                                                                (Some("assistant"), Some(model)) => format!("AI - {}", model),
                                                                                _ => "AI".to_string(),
                                                                            };
                                                                            ui.label(egui::RichText::new(label).size(12.0).strong());
                                                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                                                let datetime = chrono::DateTime::from_timestamp(output.created_at, 0)
                                                                                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                                                                    .unwrap_or_else(|| "Unknown".to_string());
                                                                                ui.label(egui::RichText::new(datetime).size(10.0).color(ui.style().visuals.weak_text_color()));
                                                                            });
                                                                        });
                                                                        ui.add_space(4.0);
                                                                        ui.label(&output.content);
                                                                    });
                                                                });
                                                        }
                                                    }
                                                    ui.add_space(8.0);
                                                }
                                            });

                                            // Message continuation input
                                            ui.separator();
                                            ui.add_space(8.0);

                                            ui.horizontal(|ui| {
                                                ui.label("Continue conversation:");
                                            });

                                            ui.add_space(4.0);

                                            let text_edit = egui::TextEdit::multiline(&mut state.ui_state.continue_message_input)
                                                .desired_width(ui.available_width())
                                                .desired_rows(3)
                                                .hint_text("Type your message here...");

                                            ui.add(text_edit);

                                            ui.add_space(8.0);

                                            ui.horizontal(|ui| {
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    let send_enabled = !state.ui_state.continue_message_input.trim().is_empty()
                                                        && !state.sending_message;

                                                    if ui.add_enabled(send_enabled, egui::Button::new("Send")).clicked() {
                                                        let api_service = crate::services::ApiService::new();
                                                        api_service.send_message_to_work(selected_work_id, state);
                                                    }

                                                    if state.sending_message {
                                                        ui.add(egui::Spinner::new());
                                                        ui.label("Sending...");
                                                    }
                                                });
                                            });
                                        }
                                    }
                                    ConnectionState::Error(error) => {
                                        ui.vertical_centered(|ui| {
                                            ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
                                        });
                                    }
                                }
                            } else {
                                ui.vertical_centered(|ui| {
                                    ui.label("Work not found");
                                });
                            }
                        } else {
                            // No work selected - show create work form
                            if matches!(state.connection_state, ConnectionState::Connected) {
                                if !state.models_fetch_attempted && !state.loading_supported_models {
                                    let api_service = crate::services::ApiService::new();
                                    api_service.refresh_supported_models(state);
                                }

                                ui.allocate_ui_with_layout(
                                    egui::vec2(ui.available_width(), 0.0),
                                    egui::Layout::top_down(egui::Align::LEFT),
                                    |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.vertical(|ui| {
                                            ui.label("What do you want to do?");

                                            egui::Frame::NONE
                                                .fill(ui.style().visuals.extreme_bg_color)
                                                .stroke(egui::Stroke::new(1.0, ui.style().visuals.widgets.noninteractive.bg_stroke.color))
                                                .corner_radius(4.0)
                                                .inner_margin(egui::Margin::same(4))
                                                .show(ui, |ui| {
                                                    ui.style_mut().text_styles.insert(
                                                        egui::TextStyle::Body,
                                                        egui::FontId::new(15.0, egui::FontFamily::Proportional),
                                                    );

                                                    ui.add(
                                                        egui::TextEdit::multiline(&mut state.ui_state.new_work_title)
                                                            .desired_width(ui.available_width())
                                                            .desired_rows(15)
                                                    );
                                                });

                                            ui.add_space(12.0);

                                            ui.horizontal(|ui| {
                                                ui.vertical(|ui| {
                                                    ui.label("Project:");
                                                    ui.style_mut().spacing.button_padding = egui::vec2(8.0, 6.0);

                                                    egui::ComboBox::from_id_salt("project_work_project_combo")
                                                        .width(200.0)
                                                        .selected_text(
                                                            state.ui_state.new_work_project_id
                                                                .and_then(|id| state.projects.iter().find(|p| p.id == id))
                                                                .map(|p| p.name.clone())
                                                                .unwrap_or_else(|| "None".to_string()),
                                                        )
                                                        .show_ui(ui, |ui| {
                                                            ui.style_mut().spacing.item_spacing = egui::vec2(8.0, 4.0);
                                                            ui.style_mut().spacing.button_padding = egui::vec2(8.0, 6.0);

                                                            let none_response = ui.selectable_value(&mut state.ui_state.new_work_project_id, None, "None");
                                                            if none_response.hovered() {
                                                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                            }

                                                            for project in &state.projects {
                                                                let project_response = ui.selectable_value(
                                                                    &mut state.ui_state.new_work_project_id,
                                                                    Some(project.id),
                                                                    &project.name,
                                                                );
                                                                if project_response.hovered() {
                                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                                }
                                                            }
                                                        });
                                                });

                                                ui.add_space(16.0);

                                                ui.vertical(|ui| {
                                                    ui.label("Model:");
                                                    if state.loading_supported_models {
                                                        ui.add(egui::Spinner::new());
                                                    } else {
                                                        ui.style_mut().spacing.button_padding = egui::vec2(8.0, 6.0);

                                                        egui::ComboBox::from_id_salt("project_work_model_combo")
                                                            .width(200.0)
                                                            .selected_text(
                                                                state.ui_state.new_work_model
                                                                    .as_ref()
                                                                    .and_then(|model_id| state.supported_models.iter()
                                                                        .find(|m| m.model_id == *model_id))
                                                                    .map(|m| m.name.clone())
                                                                    .unwrap_or_else(|| "None".to_string()),
                                                            )
                                                            .show_ui(ui, |ui| {
                                                                ui.style_mut().spacing.item_spacing = egui::vec2(8.0, 4.0);
                                                                ui.style_mut().spacing.button_padding = egui::vec2(8.0, 6.0);

                                                                let none_response = ui.selectable_value(&mut state.ui_state.new_work_model, None, "None");
                                                                if none_response.hovered() {
                                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                                }

                                                                for model in &state.supported_models {
                                                                    let model_response = ui.selectable_value(
                                                                        &mut state.ui_state.new_work_model,
                                                                        Some(model.model_id.clone()),
                                                                        &model.name,
                                                                    );
                                                                    if model_response.hovered() {
                                                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                                    }
                                                                }
                                                            });
                                                    }
                                                });
                                            });

                                            ui.add_space(12.0);

                                            if !state.loading_supported_models && state.supported_models.is_empty() {
                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new("⚠").size(16.0).color(egui::Color32::from_rgb(255, 165, 0)));
                                                    ui.label(
                                                        egui::RichText::new("No models configured. Please set API keys in Settings page")
                                                            .color(egui::Color32::from_rgb(255, 165, 0))
                                                    );
                                                });
                                                ui.add_space(8.0);
                                            }

                                            ui.horizontal(|ui| {
                                                let button_response = ui
                                                    .allocate_ui_with_layout(
                                                        egui::vec2(0.0, 0.0),
                                                        egui::Layout::left_to_right(egui::Align::Center),
                                                        |ui| {
                                                            let (bg_color, border_color) = if ui.rect_contains_pointer(ui.max_rect()) {
                                                                (
                                                                    egui::Color32::from_rgb(152, 251, 152),
                                                                    egui::Color32::from_rgb(144, 238, 144),
                                                                )
                                                            } else {
                                                                (
                                                                    egui::Color32::from_rgb(144, 238, 144),
                                                                    egui::Color32::from_rgb(120, 200, 120),
                                                                )
                                                            };
                                                            let text_color = egui::Color32::from_rgb(40, 80, 40);

                                                            egui::Frame::NONE
                                                                .fill(bg_color)
                                                                .stroke(egui::Stroke::new(1.0, border_color))
                                                                .corner_radius(8.0)
                                                                .inner_margin(egui::Margin::symmetric(16, 8))
                                                                .show(ui, |ui| {
                                                                    ui.label(
                                                                        egui::RichText::new("Create")
                                                                            .color(text_color)
                                                                            .family(egui::FontFamily::Name("ui_semibold".into())),
                                                                    );
                                                                })
                                                                .response
                                                        },
                                                    )
                                                    .inner;

                                                if button_response.hovered() {
                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                }

                                                if button_response.interact(egui::Sense::click()).clicked() && !state.ui_state.new_work_title.trim().is_empty() {
                                                    let api_service = crate::services::ApiService::new();
                                                    api_service.create_work(state);
                                                }

                                                if state.creating_work {
                                                    ui.add(egui::Spinner::new());
                                                }
                                            });
                                        });
                                    }
                                );
                            }
                        }
                    }
                );
            }
        });
    }

    fn refresh_project_details(&self, state: &mut AppState) {
        let api_service = crate::services::ApiService::new();
        api_service.refresh_project_details(self.project_id, state);
    }
}

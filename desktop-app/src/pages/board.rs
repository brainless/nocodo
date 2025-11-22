use crate::state::AppState;
use crate::state::ConnectionState;
use egui::{Context, Ui};
use manager_models::{BashRequest, ReadFileRequest, ToolRequest, ToolResponse};
use std::sync::atomic::{AtomicBool, Ordering};

static READ_FILE_PARSE_ERROR_LOGGED: AtomicBool = AtomicBool::new(false);

pub struct WorkPage;

impl WorkPage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WorkPage {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::pages::Page for WorkPage {
    fn name(&self) -> &'static str {
        "Board"
    }

    fn on_navigate_to(&mut self) {
        // Set flag to trigger works refresh in the update loop
    }

    fn ui(&mut self, _ctx: &Context, ui: &mut Ui, state: &mut AppState) {
        // Trigger refresh if flag is set
        if state.ui_state.pending_works_refresh {
            state.ui_state.pending_works_refresh = false;
            if state.connection_state == ConnectionState::Connected && !state.loading_works {
                let api_service = crate::services::ApiService::new();
                api_service.refresh_works(state);
            }
        }

        ui.heading("Board");

        // Two-column layout: left column for form and list, right column for details
        let show_second_column = true;
        // Capture full available size BEFORE starting horizontal layout
        let available_size = ui.available_size_before_wrap();

        // Calculate column widths accounting for spacing
        let left_column_width = 400.0;
        let spacing = ui.spacing().item_spacing.x;

        ui.horizontal(|ui| {
            // Left column (400px wide) - CTA button and work list
            ui.allocate_ui_with_layout(
                egui::vec2(left_column_width, available_size.y),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    // CTA button to show create work form (only show when connected)
                    if matches!(state.connection_state, ConnectionState::Connected) {
                        // Full-width button with centered text
                        let response = ui
                            .allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), 0.0),
                                egui::Layout::top_down(egui::Align::LEFT),
                                |ui| {
                                    // Pastel green colors for primary CTA - darker shade for normal, lighter on hover
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

                        // Change cursor to pointer on hover
                        if response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }

                        if response.interact(egui::Sense::click()).clicked() {
                            // Clear selection to show form in second column
                            state.ui_state.selected_work_id = None;
                        }

                        ui.add_space(16.0);
                    }

                    // Work list
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
                            } else if state.works.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.label("No work found");
                                    if ui.button("Refresh").clicked() {
                                        let api_service = crate::services::ApiService::new();
                                        api_service.refresh_works(state);
                                    }
                                });
                            } else {
                                // Use remaining space in this column for scrolling
                                egui::ScrollArea::vertical()
                                    .id_salt("work_list_scroll")
                                    .auto_shrink(false)
                                    .show(ui, |ui| {
                                        ui.add_space(8.0);

                                        // Sort works by created_at (most recent first)
                                        let mut sorted_works = state.works.clone();
                                        sorted_works.sort_by(|a, b| b.created_at.cmp(&a.created_at));

                                        for work in &sorted_works {
                                            let work_id = work.id;
                                            let is_selected = state.ui_state.selected_work_id == Some(work_id);

                                            // Card frame with different styling for selected item - full width
                                            let frame_fill = if is_selected {
                                                ui.style().visuals.selection.bg_fill
                                            } else {
                                                ui.style().visuals.widgets.inactive.bg_fill
                                            };

                                            // Allocate full width for the work item
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

                                                                    // Project if linked
                                                                    if let Some(project_id) = work.project_id {
                                                                        if let Some(project) = state.projects.iter().find(|p| p.id == project_id) {
                                                                            egui::Frame::NONE
                                                                                .fill(ui.style().visuals.selection.bg_fill)
                                                                                .corner_radius(4.0)
                                                                                .inner_margin(egui::Margin::symmetric(8, 4))
                                                                                .show(ui, |ui| {
                                                                                    ui.label(egui::RichText::new(&project.name).size(11.0));
                                                                                });
                                                                        }
                                                                    }
                                                                });
                                                            });
                                                        });
                                                }
                                            );

                                            // Make the entire card clickable
                                            if item_response.response.interact(egui::Sense::click()).clicked() {
                                                // Reset scroll if selecting a different work item
                                                if state.ui_state.selected_work_id != Some(work_id) {
                                                    state.ui_state.reset_work_details_scroll = true;
                                                }
                                                state.ui_state.selected_work_id = Some(work_id);
                                                let api_service = crate::services::ApiService::new();
                                                api_service.refresh_work_messages(work_id, state);
                                            }

                                            // Change cursor to pointer on hover
                                            if item_response.response.hovered() {
                                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                            }

                                            ui.add_space(8.0);
                                        }
                                    });
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
                // Account for: left column width + spacing after left column + separator width + spacing after separator
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
                                            // Reserve space for reply form at bottom (approximately 120px)
                                            let reply_form_height = 120.0;
                                            let available_height = ui.available_height() - reply_form_height;
                                            let available_width = ui.available_width();

                                            let mut scroll_area = egui::ScrollArea::vertical()
                                                .id_salt("work_messages_scroll")
                                                .auto_shrink(false)
                                                .max_height(available_height)
                                                .max_width(available_width);

                                            // Reset scroll to top if a new work item was selected
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

                                                // Add work messages (user input)
                                                for msg in &state.work_messages {
                                                    all_messages.push((msg.created_at, DisplayMessage::WorkMessage(msg.clone())));
                                                }

                                                // Add AI session outputs (AI responses)
                                                for output in &state.ai_session_outputs {
                                                    all_messages.push((output.created_at, DisplayMessage::AiOutput(output.clone())));
                                                }

                                                // Sort by timestamp
                                                all_messages.sort_by_key(|(timestamp, _)| *timestamp);

                                                for (_timestamp, message) in &all_messages {
                                                    match message {
                                                        DisplayMessage::WorkMessage(msg) => {
                                                            // User message
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
                                                            // Don't display list_files tool requests - only show responses

                                                            // Extract and display any "text" field from assistant messages (separate from tool calls)
                                                            let assistant_text = if output.role.as_deref() == Some("assistant") {
                                                                if let Ok(assistant_data) = serde_json::from_str::<serde_json::Value>(&output.content) {
                                                                    assistant_data.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
                                                                } else {
                                                                    None
                                                                }
                                                            } else {
                                                                None
                                                            };

                                                            if let Some(ref text) = assistant_text {
                                                                if !text.trim().is_empty() {
                                                                    // Show assistant text message
                                                                    let bg_color = ui.style().visuals.widgets.noninteractive.bg_fill;

                                                                    egui::Frame::NONE
                                                                        .fill(bg_color)
                                                                        .corner_radius(8.0)
                                                                        .inner_margin(egui::Margin::same(12))
                                                                        .show(ui, |ui| {
                                                                            ui.vertical(|ui| {
                                                                                ui.horizontal(|ui| {
                                                                                    let label = if let Some(model) = &output.model {
                                                                                        format!("AI - {}", model)
                                                                                    } else {
                                                                                        "AI".to_string()
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
                                                                                ui.label(text.as_str());
                                                                            });
                                                                        });
                                                                    ui.add_space(8.0);
                                                                }
                                                            }

                                                            // Check if this is a bash tool request
                                                            let bash_requests = if output.role.as_deref() == Some("assistant") {
                                                                let mut requests = Vec::new();

                                                                if let Ok(assistant_data) = serde_json::from_str::<serde_json::Value>(&output.content) {
                                                                    if let Some(tool_calls) = assistant_data.get("tool_calls").and_then(|tc| tc.as_array()) {
                                                                        for tool_call in tool_calls {
                                                                            if let Some(function) = tool_call.get("function") {
                                                                                if let Some(name) = function.get("name").and_then(|n| n.as_str()) {
                                                                                    if name == "bash" {
                                                                                        if let Some(args) = function.get("arguments").and_then(|a| a.as_str()) {
                                                                                            match serde_json::from_str::<BashRequest>(args) {
                                                                                                Ok(bash_req) => {
                                                                                                    requests.push(bash_req);
                                                                                                }
                                                                                                Err(e) => {
                                                                                                    tracing::warn!(error = %e, arguments = %args, "Failed to parse BashRequest");
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }

                                                                if requests.is_empty() {
                                                                    if let Ok(ToolRequest::Bash(bash_req)) = serde_json::from_str::<ToolRequest>(&output.content) {
                                                                        requests.push(bash_req);
                                                                    }
                                                                }

                                                                if !requests.is_empty() {
                                                                    Some(requests)
                                                                } else {
                                                                    None
                                                                }
                                                            } else {
                                                                None
                                                            };

                                                            if let Some(ref requests) = bash_requests {
                                                                for req in requests {
                                                                    // Check if this bash request is expanded
                                                                    let is_expanded = state.ui_state.expanded_tool_calls.contains(&output.id);

                                                                    // Truncate command to first 100 characters for collapsed view
                                                                    let truncated_command = if req.command.len() > 100 {
                                                                        format!("{}...", &req.command[..100])
                                                                    } else {
                                                                        req.command.clone()
                                                                    };

                                                                    let bg_color = ui.style().visuals.widgets.inactive.bg_fill;

                                                                    let response = egui::Frame::NONE
                                                                        .fill(bg_color)
                                                                        .corner_radius(0.0)
                                                                        .inner_margin(egui::Margin::symmetric(12, 6))
                                                                        .show(ui, |ui| {
                                                                            ui.set_width(ui.available_width());
                                                                            ui.vertical(|ui| {
                                                                                // Header row - clickable
                                                                                let header_response = ui.horizontal(|ui| {
                                                                                    let arrow = if is_expanded { "▼" } else { "▶" };
                                                                                    ui.label(egui::RichText::new(arrow).size(12.0));
                                                                                    ui.label(egui::RichText::new(if is_expanded {
                                                                                        &req.command
                                                                                    } else {
                                                                                        &truncated_command
                                                                                    }).size(12.0).strong().family(egui::FontFamily::Monospace));
                                                                                }).response;

                                                                                // Show description if expanded and it exists
                                                                                if is_expanded {
                                                                                    if let Some(desc) = &req.description {
                                                                                        if !desc.trim().is_empty() {
                                                                                            ui.add_space(4.0);
                                                                                            ui.label(egui::RichText::new(desc).size(11.0).color(ui.style().visuals.weak_text_color()));
                                                                                        }
                                                                                    }
                                                                                }

                                                                                header_response
                                                                            }).inner
                                                                        })
                                                                        .response;

                                                                    // Handle click to toggle expansion
                                                                    if response.interact(egui::Sense::click()).clicked() {
                                                                        if is_expanded {
                                                                            state.ui_state.expanded_tool_calls.remove(&output.id);
                                                                        } else {
                                                                            state.ui_state.expanded_tool_calls.insert(output.id);
                                                                        }
                                                                    }

                                                                    // Change cursor to pointer on hover
                                                                    if response.hovered() {
                                                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                                    }

                                                                    ui.add_space(4.0);
                                                                }
                                                            }

                                                            // Check if this is a read_file tool request using proper Rust types
                                                            let read_file_requests = if output.role.as_deref() == Some("assistant") {
                                                                // Try multiple parsing approaches for robustness
                                                                let mut requests = Vec::new();

                                                                if let Ok(assistant_data) = serde_json::from_str::<serde_json::Value>(&output.content) {
                                                                    // Look for tool_calls array in the structured response
                                                                    if let Some(tool_calls) = assistant_data.get("tool_calls").and_then(|tc| tc.as_array()) {
                                                                        for tool_call in tool_calls {
                                                                            if let Some(function) = tool_call.get("function") {
                                                                                if let Some(name) = function.get("name").and_then(|n| n.as_str()) {
                                                                                    if name == "read_file" {
                                                                                        if let Some(args) = function.get("arguments").and_then(|a| a.as_str()) {
                                                                                            // Use proper Rust type for parsing ReadFileRequest
                                                                                            match serde_json::from_str::<ReadFileRequest>(args) {
                                                                                                Ok(read_file_req) => {
                                                                                                    requests.push(read_file_req);
                                                                                                }
                                                                                                Err(e) => {
                                                                                                    if !READ_FILE_PARSE_ERROR_LOGGED.swap(true, Ordering::Relaxed) {
                                                                                                        tracing::warn!(error = %e, arguments = %args, "Failed to parse ReadFileRequest");
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }

                                                                // Try to parse as direct ToolRequest (fallback format)
                                                                if requests.is_empty() {
                                                                    if let Ok(ToolRequest::ReadFile(read_file_req)) = serde_json::from_str::<ToolRequest>(&output.content) {
                                                                        requests.push(read_file_req);
                                                                    }
                                                                }

                                                                if !requests.is_empty() {
                                                                    Some(requests)
                                                                } else {
                                                                    None
                                                                }
                                                            } else {
                                                                None
                                                            };

                                                            if let Some(ref requests) = read_file_requests {
                                                                // Display each read_file tool request in full width box with no rounded corners
                                                                for req in requests {
                                                                    let mut description = format!("Read file: {}", req.path);

                                                                    // Add optional parameters to the description
                                                                    if let Some(max_size) = req.max_size {
                                                                        description.push_str(&format!(" (max size: {} bytes)", max_size));
                                                                    }

                                                                    // Use the same styling as list_files box but full width with no rounded corners
                                                                    let bg_color = ui.style().visuals.widgets.inactive.bg_fill;

                                                                    egui::Frame::NONE
                                                                        .fill(bg_color)
                                                                        .corner_radius(0.0)
                                                                        .inner_margin(egui::Margin::symmetric(12, 6))
                                                                        .show(ui, |ui| {
                                                                            ui.set_width(ui.available_width());
                                                                            ui.vertical(|ui| {
                                                                                ui.horizontal(|ui| {
                                                                                    ui.label(egui::RichText::new("🤖").size(16.0));
                                                                                    ui.label(egui::RichText::new("📄").size(16.0));
                                                                                    ui.label(egui::RichText::new(description).size(12.0).strong());
                                                                                });
                                                                            });
                                                                        });
                                                                    ui.add_space(4.0);
                                                                }
                                                            }

                                                            // Show tool responses (from ai_session_outputs with role='tool')
                                                            // Check if this output is a tool response
                                                            if output.role.as_deref() == Some("tool") {
                                                                // Parse the tool response - it's wrapped in {"content": <ToolResponse>, "tool_use_id": "..."}
                                                                if let Ok(wrapped_response) = serde_json::from_str::<serde_json::Value>(&output.content) {
                                                                    if let Some(content) = wrapped_response.get("content") {
                                                                        // Try to parse the content as ToolResponse
                                                                        if let Ok(ToolResponse::ListFiles(list_files_response)) = serde_json::from_value::<ToolResponse>(content.clone()) {
                                                                                // Check if this response is expanded
                                                                                let is_expanded = state.ui_state.expanded_tool_calls.contains(&output.id);

                                                                                // Use the same styling as tool request box
                                                                                let bg_color = ui.style().visuals.widgets.inactive.bg_fill;

                                                                                let response = egui::Frame::NONE
                                                                                    .fill(bg_color)
                                                                                    .corner_radius(0.0)
                                                                                    .inner_margin(egui::Margin::symmetric(12, 6))
                                                                                    .show(ui, |ui| {
                                                                                        ui.set_width(ui.available_width());
                                                                                        ui.vertical(|ui| {
                                                                                            // Header row - clickable
                                                                                            let header_response = ui.horizontal(|ui| {
                                                                                                let arrow = if is_expanded { "▼" } else { "▶" };
                                                                                                ui.label(egui::RichText::new(arrow).size(12.0));
                                                                                                ui.label(egui::RichText::new(format!(
                                                                                                    "Listed {} files in {}",
                                                                                                    list_files_response.total_files,
                                                                                                    list_files_response.current_path
                                                                                                )).size(12.0).strong());
                                                                                            }).response;

                                                                                            // Show file list if expanded
                                                                                            if is_expanded {
                                                                                                ui.add_space(8.0);
                                                                                                ui.separator();
                                                                                                ui.add_space(4.0);

                                                                                                // Display the file tree
                                                                                                ui.label(&list_files_response.files);

                                                                                                // Show truncation warning if needed
                                                                                                if list_files_response.truncated {
                                                                                                    ui.add_space(4.0);
                                                                                                    ui.label(
                                                                                                        egui::RichText::new(format!(
                                                                                                            "⚠ Truncated at {} files",
                                                                                                            list_files_response.limit
                                                                                                        ))
                                                                                                        .size(11.0)
                                                                                                        .color(egui::Color32::from_rgb(255, 165, 0))
                                                                                                    );
                                                                                                }
                                                                                            }

                                                                                            header_response
                                                                                        }).inner
                                                                                    })
                                                                                    .response;

                                                                                // Handle click to toggle expansion
                                                                                if response.interact(egui::Sense::click()).clicked() {
                                                                                    if is_expanded {
                                                                                        state.ui_state.expanded_tool_calls.remove(&output.id);
                                                                                    } else {
                                                                                        state.ui_state.expanded_tool_calls.insert(output.id);
                                                                                    }
                                                                                }

                                                                                // Change cursor to pointer on hover
                                                                                if response.hovered() {
                                                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                                                }

                                                                                ui.add_space(4.0);
                                                                        }

                                                                        // Handle bash tool response
                                                                        if let Ok(ToolResponse::Bash(bash_response)) = serde_json::from_value::<ToolResponse>(content.clone()) {
                                                                                // Check if this response is expanded
                                                                                let is_expanded = state.ui_state.expanded_tool_calls.contains(&output.id);

                                                                                // Truncate command to first 100 characters for collapsed view
                                                                                let truncated_command = if bash_response.command.len() > 100 {
                                                                                    format!("{}...", &bash_response.command[..100])
                                                                                } else {
                                                                                    bash_response.command.clone()
                                                                                };

                                                                                let bg_color = ui.style().visuals.widgets.inactive.bg_fill;

                                                                                let response = egui::Frame::NONE
                                                                                    .fill(bg_color)
                                                                                    .corner_radius(0.0)
                                                                                    .inner_margin(egui::Margin::symmetric(12, 6))
                                                                                    .show(ui, |ui| {
                                                                                        ui.set_width(ui.available_width());
                                                                                        ui.vertical(|ui| {
                                                                                            // Header row - clickable
                                                                                            let header_response = ui.horizontal(|ui| {
                                                                                                let arrow = if is_expanded { "▼" } else { "▶" };
                                                                                                ui.label(egui::RichText::new(arrow).size(12.0));

                                                                                                // Show exit code indicator
                                                                                                let (indicator, color) = if bash_response.exit_code == 0 {
                                                                                                    ("✓", egui::Color32::from_rgb(0, 200, 0))
                                                                                                } else {
                                                                                                    ("✗", egui::Color32::from_rgb(200, 0, 0))
                                                                                                };
                                                                                                ui.label(egui::RichText::new(indicator).size(12.0).color(color));

                                                                                                ui.label(egui::RichText::new(if is_expanded {
                                                                                                    &bash_response.command
                                                                                                } else {
                                                                                                    &truncated_command
                                                                                                }).size(12.0).strong().family(egui::FontFamily::Monospace));
                                                                                            }).response;

                                                                                            // Show output if expanded
                                                                                            if is_expanded {
                                                                                                ui.add_space(8.0);
                                                                                                ui.separator();
                                                                                                ui.add_space(4.0);

                                                                                                // Show stdout if not empty
                                                                                                if !bash_response.stdout.trim().is_empty() {
                                                                                                    ui.label(egui::RichText::new("stdout:").size(11.0).strong());
                                                                                                    ui.add_space(2.0);
                                                                                                    egui::ScrollArea::vertical()
                                                                                                        .max_height(200.0)
                                                                                                        .show(ui, |ui| {
                                                                                                            ui.label(egui::RichText::new(&bash_response.stdout)
                                                                                                                .size(10.0)
                                                                                                                .family(egui::FontFamily::Monospace));
                                                                                                        });
                                                                                                    ui.add_space(4.0);
                                                                                                }

                                                                                                // Show stderr if not empty
                                                                                                if !bash_response.stderr.trim().is_empty() {
                                                                                                    ui.label(egui::RichText::new("stderr:").size(11.0).strong().color(egui::Color32::from_rgb(200, 0, 0)));
                                                                                                    ui.add_space(2.0);
                                                                                                    egui::ScrollArea::vertical()
                                                                                                        .max_height(200.0)
                                                                                                        .show(ui, |ui| {
                                                                                                            ui.label(egui::RichText::new(&bash_response.stderr)
                                                                                                                .size(10.0)
                                                                                                                .family(egui::FontFamily::Monospace)
                                                                                                                .color(egui::Color32::from_rgb(200, 0, 0)));
                                                                                                        });
                                                                                                    ui.add_space(4.0);
                                                                                                }

                                                                                                // Show execution details
                                                                                                ui.horizontal(|ui| {
                                                                                                    ui.label(egui::RichText::new(format!("Exit code: {}", bash_response.exit_code))
                                                                                                        .size(10.0)
                                                                                                        .color(ui.style().visuals.weak_text_color()));
                                                                                                    ui.label(egui::RichText::new(format!("Time: {:.2}s", bash_response.execution_time_secs))
                                                                                                        .size(10.0)
                                                                                                        .color(ui.style().visuals.weak_text_color()));
                                                                                                    if bash_response.timed_out {
                                                                                                        ui.label(egui::RichText::new("⚠ Timed out")
                                                                                                            .size(10.0)
                                                                                                            .color(egui::Color32::from_rgb(255, 165, 0)));
                                                                                                    }
                                                                                                });
                                                                                            }

                                                                                            header_response
                                                                                        }).inner
                                                                                    })
                                                                                    .response;

                                                                                // Handle click to toggle expansion
                                                                                if response.interact(egui::Sense::click()).clicked() {
                                                                                    if is_expanded {
                                                                                        state.ui_state.expanded_tool_calls.remove(&output.id);
                                                                                    } else {
                                                                                        state.ui_state.expanded_tool_calls.insert(output.id);
                                                                                    }
                                                                                }

                                                                                // Change cursor to pointer on hover
                                                                                if response.hovered() {
                                                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                                                }

                                                                                ui.add_space(4.0);
                                                                        }
                                                                    }
                                                                }
                                                            }

                                                            // Show regular AI response if not a tool request or tool response
                                                            // Skip if we already showed assistant text, bash request, read_file request, or this is a tool response
                                                            let skip_regular_response = assistant_text.is_some() || bash_requests.is_some() || read_file_requests.is_some() || output.role.as_deref() == Some("tool");
                                                            if !skip_regular_response {
                                                                // Regular AI response message
                                                                let bg_color = ui.style().visuals.widgets.noninteractive.bg_fill;

                                                                egui::Frame::NONE
                                                                    .fill(bg_color)
                                                                    .corner_radius(8.0)
                                                                    .inner_margin(egui::Margin::same(12))
                                                                    .show(ui, |ui| {
                                                                        ui.vertical(|ui| {
                                                                            ui.horizontal(|ui| {
                                                                                // Determine label based on role and model
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
                                                    }
                                                    ui.add_space(8.0);
                                                }
                                            });

                                            // Message continuation input (outside scroll area)
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
                                      // Load models only once when form is opened
                                      if !state.models_fetch_attempted && !state.loading_supported_models {
                                          let api_service = crate::services::ApiService::new();
                                          api_service.refresh_supported_models(state);
                                      }

                                      // Load worktree branches when a project is selected
                                      if let Some(project_id) = state.ui_state.new_work_project_id {
                                          if !state.worktree_branches_fetch_attempted && !state.loading_worktree_branches {
                                              let api_service = crate::services::ApiService::new();
                                              api_service.refresh_worktree_branches(state, project_id);
                                          }
                                      }

                                // Create form - full width of second column
                                ui.allocate_ui_with_layout(
                                    egui::vec2(ui.available_width(), 0.0),
                                    egui::Layout::top_down(egui::Align::LEFT),
                                    |ui| {
                                        // No background frame - removed gray background
                                        ui.set_width(ui.available_width());
                                        ui.vertical(|ui| {
                                            // Title/Question field as textarea
                                            ui.label("What do you want to do?");

                                            // Custom frame for text area with 4px padding
                                            egui::Frame::NONE
                                                .fill(ui.style().visuals.extreme_bg_color)
                                                .stroke(egui::Stroke::new(1.0, ui.style().visuals.widgets.noninteractive.bg_stroke.color))
                                                .corner_radius(4.0)
                                                .inner_margin(egui::Margin::same(4))
                                                .show(ui, |ui| {
                                                    // Set larger font size for the text area
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

                                             // Project, Work Branch, and Model fields
                                             ui.vertical(|ui| {
                                                 // Project field
                                                 ui.label("Project:");
                                                 // Set button padding for dropdown widget itself
                                                 ui.style_mut().spacing.button_padding = egui::vec2(8.0, 6.0);

                                                 let previous_project_id = state.ui_state.new_work_project_id;
                                                 egui::ComboBox::from_id_salt("work_project_combo")
                                                     .width(200.0)
                                                     .selected_text(
                                                         state.ui_state.new_work_project_id
                                                             .and_then(|id| state.projects.iter().find(|p| p.id == id))
                                                             .map(|p| p.name.clone())
                                                             .unwrap_or_else(|| "None".to_string()),
                                                     )
                                                     .show_ui(ui, |ui| {
                                                         // Add padding to dropdown items
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

                                                 // Reset worktree branches when project changes
                                                 if previous_project_id != state.ui_state.new_work_project_id {
                                                     state.worktree_branches.clear();
                                                     state.worktree_branches_fetch_attempted = false;
                                                     state.loading_worktree_branches = false;
                                                     state.ui_state.new_work_branch = None;
                                                 }

                                                 ui.add_space(12.0);

                                                 // Work Branch field
                                                 ui.label("Work Branch:");
                                                  if let Some(_project_id) = state.ui_state.new_work_project_id {
                                                     if state.loading_worktree_branches {
                                                         ui.add(egui::Spinner::new());
                                                     } else {
                                                         // Set button padding for dropdown widget itself
                                                         ui.style_mut().spacing.button_padding = egui::vec2(8.0, 6.0);

                                                         egui::ComboBox::from_id_salt("work_branch_combo")
                                                             .width(200.0)
                                                             .selected_text(
                                                                 state.ui_state.new_work_branch
                                                                     .clone()
                                                                     .unwrap_or_else(|| "None".to_string()),
                                                             )
                                                             .show_ui(ui, |ui| {
                                                                 // Add padding to dropdown items
                                                                 ui.style_mut().spacing.item_spacing = egui::vec2(8.0, 4.0);
                                                                 ui.style_mut().spacing.button_padding = egui::vec2(8.0, 6.0);

                                                                 let none_response = ui.selectable_value(&mut state.ui_state.new_work_branch, None, "None");
                                                                 if none_response.hovered() {
                                                                     ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                                 }

                                                                 for branch in &state.worktree_branches {
                                                                     let branch_response = ui.selectable_value(
                                                                         &mut state.ui_state.new_work_branch,
                                                                         Some(branch.clone()),
                                                                         branch,
                                                                     );
                                                                     if branch_response.hovered() {
                                                                         ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                                     }
                                                                 }
                                                             });
                                                     }
                                                 } else {
                                                     ui.label(egui::RichText::new("Select a project first").color(ui.style().visuals.weak_text_color()));
                                                 }

                                                 ui.add_space(12.0);

                                                 // Model field
                                                 ui.label("Model:");
                                                 if state.loading_supported_models {
                                                     ui.add(egui::Spinner::new());
                                                 } else {
                                                     // Set button padding for dropdown widget itself
                                                     ui.style_mut().spacing.button_padding = egui::vec2(8.0, 6.0);

                                                     egui::ComboBox::from_id_salt("work_model_combo")
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
                                                             // Add padding to dropdown items
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

                                            ui.add_space(12.0);

                                            // Show error if no models are configured
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

                                            // Create button with same styling as CTA
                                            ui.horizontal(|ui| {
                                                let button_response = ui
                                                    .allocate_ui_with_layout(
                                                        egui::vec2(0.0, 0.0),
                                                        egui::Layout::left_to_right(egui::Align::Center),
                                                        |ui| {
                                                            // Same pastel green colors as CTA button
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

                                                // Change cursor to pointer on hover
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
}

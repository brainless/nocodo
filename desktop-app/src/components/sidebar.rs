use crate::state::ui_state::Page as UiPage;
use crate::state::AppState;
use crate::state::ConnectionState;
use egui::{Color32, Context};

pub struct Sidebar;

impl Sidebar {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

impl Sidebar {
    pub fn ui(&mut self, ctx: &Context, state: &mut AppState) -> Option<UiPage> {
        let mut new_page = None;

        egui::SidePanel::left("sidebar")
            .exact_width(200.0)
            .show(ctx, |ui| {
                ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 2.0);
                ui.vertical(|ui| {
                    let sidebar_bg = ui.style().visuals.panel_fill;
                    let button_bg = ui.style().visuals.widgets.inactive.bg_fill;

                    // Branding
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("nocodo").size(20.0).strong());
                    ui.add_space(20.0);

                    // Top navigation
                    if self.sidebar_link(ui, "Projects", sidebar_bg, button_bg) {
                        new_page = Some(UiPage::Projects);
                    }

                    // Favorite projects section
                    if !state.favorite_projects.is_empty()
                        && state.connection_state == ConnectionState::Connected
                    {
                        ui.add_space(4.0);

                        // Show favorite projects
                        for project in &state.projects {
                            if state.favorite_projects.contains(&project.id) {
                                let available_width = ui.available_width();
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(available_width, 24.0),
                                    egui::Sense::click(),
                                );

                                // Change cursor to pointer on hover
                                if response.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }

                                // Determine background color based on hover state (same as sidebar_link)
                                let bg_color = if response.hovered() {
                                    button_bg
                                } else {
                                    sidebar_bg
                                };

                                // Draw background with same border radius as sidebar_link (0.0)
                                ui.painter().rect_filled(rect, 0.0, bg_color);

                                // Draw text with same styling as sidebar_link but with 12px left padding (8px + 4px extra)
                                let text_pos = rect.min + egui::vec2(12.0, 4.0); // Same y position (4.0) as sidebar_link
                                ui.painter().text(
                                    text_pos,
                                    egui::Align2::LEFT_TOP, // Same alignment as sidebar_link
                                    &project.name,
                                    egui::FontId::default(), // Same font as sidebar_link
                                    ui.style().visuals.text_color(), // Same text color as sidebar_link
                                );

                                // Handle click
                                if response.clicked() {
                                    new_page = Some(UiPage::ProjectDetail(project.id));
                                    state.pending_project_details_refresh = Some(project.id);
                                }
                            }
                        }
                        ui.add_space(4.0);
                    }

                    if self.sidebar_link(ui, "Board", sidebar_bg, button_bg) {
                        new_page = Some(UiPage::Work);
                        // Refresh works when navigating to Work page
                        if state.connection_state == ConnectionState::Connected
                            && state.works.is_empty()
                            && !state.loading_works
                        {
                            self.refresh_works(state);
                        }
                    }
                    if self.sidebar_link(ui, "Mentions", sidebar_bg, button_bg) {
                        new_page = Some(UiPage::Mentions);
                    }

                    // Empty space
                    ui.add_space(50.0);

                    // Bottom navigation
                    if self.sidebar_link(ui, "Servers", sidebar_bg, button_bg) {
                        new_page = Some(UiPage::Servers);
                        // Check local server when navigating to Servers page
                        if !state.ui_state.checking_local_server {
                            self.check_local_server(state);
                        }
                    }
                    if self.sidebar_link(ui, "Settings", sidebar_bg, button_bg) {
                        new_page = Some(UiPage::Settings);
                    }
                });
            });

        new_page
    }

    fn sidebar_link(
        &self,
        ui: &mut egui::Ui,
        text: &str,
        default_bg: Color32,
        hover_bg: Color32,
    ) -> bool {
        let available_width = ui.available_width();
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(available_width, 24.0), egui::Sense::click());

        // Change cursor to pointer on hover
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        // Determine background color based on hover state
        let bg_color = if response.hovered() {
            hover_bg
        } else {
            default_bg
        };

        // Draw background
        ui.painter().rect_filled(rect, 0.0, bg_color);

        // Draw text (non-selectable)
        let text_pos = rect.min + egui::vec2(8.0, 4.0);
        ui.painter().text(
            text_pos,
            egui::Align2::LEFT_TOP,
            text,
            egui::FontId::default(),
            ui.style().visuals.text_color(),
        );

        response.clicked()
    }

    fn refresh_works(&self, state: &mut AppState) {
        state.loading_works = true;
        // This will be implemented when we extract the API methods
    }

    fn check_local_server(&self, state: &mut AppState) {
        state.ui_state.checking_local_server = true;
        // This will be implemented when we extract the API methods
    }
}

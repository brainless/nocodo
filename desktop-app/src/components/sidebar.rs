use crate::state::ui_state::Page as UiPage;
use crate::state::AppState;
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
                    let selected_bg = ui.style().visuals.widgets.active.bg_fill;
                    let hover_bg = ui.style().visuals.widgets.hovered.bg_fill;
                    let is_authenticated = state.is_authenticated();
                    let current_page = &state.ui_state.current_page;

                    // Branding - Ubuntu Light with color that adapts to theme
                    ui.add_space(8.0);
                    let logo_color = if ui.visuals().dark_mode {
                        Color32::WHITE
                    } else {
                        Color32::from_gray(40) // Dark color for light mode
                    };
                    ui.label(
                        egui::RichText::new("nocodo")
                            .size(20.0)
                            .family(egui::FontFamily::Name("ui_light".into()))
                            .color(logo_color),
                    );
                    ui.add_space(20.0);

                    // Top navigation
                    if self.sidebar_link(
                        ui,
                        "Projects",
                        sidebar_bg,
                        hover_bg,
                        selected_bg,
                        is_authenticated,
                        current_page == &UiPage::Projects,
                    ) {
                        new_page = Some(UiPage::Projects);
                    }

                    // Favorite projects section
                    tracing::debug!("Sidebar check: favorites_count={}, authenticated={}, server_info={:?}", 
                        state.favorite_projects.len(), is_authenticated, state.current_server_info);
                    if !state.favorite_projects.is_empty() && is_authenticated && state.current_server_info.is_some() {
                        ui.add_space(4.0);

                        // Show favorite projects for current server
                        if let Some((server_host, server_user, server_port)) = &state.current_server_info {
                            tracing::debug!("Checking favorites for server: {:?}", (server_host, server_user, server_port));
                            for project in &state.projects {
                                let favorite_key = &(server_host.clone(), server_user.clone(), *server_port, project.id);
                                let is_favorite = state.favorite_projects.contains(favorite_key);
                                if is_favorite {
                                    tracing::debug!("Showing favorite project: {} ({})", project.name, project.id);
                                    let available_width = ui.available_width();
                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::vec2(available_width, 24.0),
                                        egui::Sense::click(),
                                    );

                                    // Change cursor to pointer on hover
                                    if response.hovered() {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }

                                    // Check if this project is the currently selected one
                                    let is_selected = matches!(
                                        current_page,
                                        UiPage::ProjectDetail(id) if *id == project.id
                                    );

                                    // Determine background color based on state
                                    let bg_color = if is_selected {
                                        selected_bg
                                    } else if response.hovered() {
                                        hover_bg
                                    } else {
                                        sidebar_bg
                                    };

                                    // Draw background with same border radius as sidebar_link (0.0)
                                    ui.painter().rect_filled(rect, 0.0, bg_color);

                                    // Draw text - project names are user content, use Inter (Proportional)
                                    let text_pos = rect.min + egui::vec2(12.0, 4.0);
                                    let font_id = egui::FontId::new(
                                        14.0,
                                        egui::FontFamily::Proportional, // Inter for user content
                                    );
                                    ui.painter().text(
                                        text_pos,
                                        egui::Align2::LEFT_TOP,
                                        &project.name,
                                        font_id,
                                        ui.style().visuals.text_color(),
                                    );

                                    // Handle click
                                    if response.clicked() {
                                        new_page = Some(UiPage::ProjectDetail(project.id));
                                        state.pending_project_details_refresh = Some(project.id);
                                    }
                            }
                        }
                        }
                        ui.add_space(4.0);
                    }

                    if self.sidebar_link(
                        ui,
                        "Board",
                        sidebar_bg,
                        hover_bg,
                        selected_bg,
                        is_authenticated,
                        current_page == &UiPage::Work,
                    ) {
                        new_page = Some(UiPage::Work);
                    }
                    if self.sidebar_link(
                        ui,
                        "Mentions",
                        sidebar_bg,
                        hover_bg,
                        selected_bg,
                        is_authenticated,
                        current_page == &UiPage::Mentions,
                    ) {
                        new_page = Some(UiPage::Mentions);
                    }

                    // Expanding space - pushes bottom items to the bottom
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                        // Bottom navigation (in reverse order since we're going bottom-up)
                        if self.sidebar_link(
                            ui,
                            "Servers",
                            sidebar_bg,
                            hover_bg,
                            selected_bg,
                            true, // Servers is always enabled
                            current_page == &UiPage::Servers,
                        ) {
                            new_page = Some(UiPage::Servers);
                        }
                        if self.sidebar_link(
                            ui,
                            "Settings",
                            sidebar_bg,
                            hover_bg,
                            selected_bg,
                            is_authenticated,
                            current_page == &UiPage::Settings,
                        ) {
                            new_page = Some(UiPage::Settings);
                        }
                        if self.sidebar_link(
                            ui,
                            "Teams",
                            sidebar_bg,
                            hover_bg,
                            selected_bg,
                            is_authenticated,
                            current_page == &UiPage::Teams,
                        ) {
                            new_page = Some(UiPage::Teams);
                        }
                        if self.sidebar_link(
                            ui,
                            "Users",
                            sidebar_bg,
                            hover_bg,
                            selected_bg,
                            is_authenticated,
                            current_page == &UiPage::Users,
                        ) {
                            new_page = Some(UiPage::Users);
                        }
                    });
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
        selected_bg: Color32,
        enabled: bool,
        is_selected: bool,
    ) -> bool {
        let available_width = ui.available_width();
        let sense = if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(egui::vec2(available_width, 32.0), sense);

        // Change cursor to pointer on hover only if enabled
        if enabled && response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        // Determine background color based on state
        let bg_color = if is_selected {
            selected_bg
        } else if enabled && response.hovered() {
            hover_bg
        } else {
            default_bg
        };

        // Draw background
        ui.painter().rect_filled(rect, 0.0, bg_color);

        // Draw text (non-selectable) using Ubuntu Light
        let text_pos = rect.min + egui::vec2(8.0, 8.0);
        let font_id = egui::FontId::new(
            14.0,
            egui::FontFamily::Name("ui_light".into()), // Ubuntu Light
        );
        let text_color = if enabled {
            ui.style().visuals.text_color()
        } else {
            ui.style().visuals.weak_text_color()
        };
        ui.painter().text(
            text_pos,
            egui::Align2::LEFT_TOP,
            text,
            font_id,
            text_color,
        );

        enabled && response.clicked()
    }
}

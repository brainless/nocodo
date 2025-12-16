use crate::state::AppState;
use crate::state::ConnectionState;
use egui::{Context, Vec2};
use egui_flex::{item, Flex, FlexAlignContent};
use egui_material_icons::icons;

pub struct StatusBar;

impl StatusBar {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusBar {
    pub fn ui(&mut self, ctx: &Context, state: &AppState) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.style_mut().spacing.item_spacing = egui::vec2(12.0, 0.0);
            
            Flex::horizontal()
                .gap(Vec2::new(12.0, 0.0))
                .align_content(FlexAlignContent::Center)
                .show(ui, |flex| {
                    // Connection status section
                    match &state.connection_state {
                        ConnectionState::Disconnected => {
                            flex.add(item(), egui::Label::new(
                                egui::RichText::new(format!("{} Disconnected", icons::ICON_WIFI_OFF))
                                    .color(egui::Color32::RED)
                            ));
                        }
                        ConnectionState::Connecting => {
                            flex.add(item(), egui::Label::new(
                                egui::RichText::new(format!("{} Connecting...", icons::ICON_WIFI))
                                    .color(egui::Color32::YELLOW)
                            ));
                        }
                        ConnectionState::Connected => {
                            let label = if let Some(host) = &state.ui_state.connected_host {
                                format!("{} Connected: {}", icons::ICON_WIFI, host)
                            } else {
                                format!("{} Connected", icons::ICON_WIFI)
                            };
                            flex.add(item(), egui::Label::new(
                                egui::RichText::new(label)
                                    .color(egui::Color32::GREEN)
                            ));
                            flex.add(item(), egui::Label::new(
                                format!("{} Projects: {}", icons::ICON_FOLDER, state.projects.len())
                            ));
                        }
                        ConnectionState::Error(error) => {
                            flex.add(item(), egui::Label::new(
                                egui::RichText::new(format!("{} Error: {}", icons::ICON_ERROR, error))
                                    .color(egui::Color32::RED)
                            ));
                        }
                    }

                    // Error message section
                    if let Some(error) = &state.ui_state.connection_error {
                        flex.add(item(), egui::Label::new(
                            egui::RichText::new(error)
                                .color(egui::Color32::RED)
                        ));
                    }

                    // Add flexible space to push any future right-aligned items
                    flex.add_flex(
                        item().grow(1.0),
                        Flex::horizontal().align_content(FlexAlignContent::End),
                        |_flex| {
                            // Future right-aligned items can go here
                        },
                    );
                });
        });
    }
}

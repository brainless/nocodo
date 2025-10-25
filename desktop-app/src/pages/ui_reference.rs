use crate::pages::Page;
use crate::state::ui_state::Page as UiPage;
use crate::state::AppState;
use egui::{Context, Ui};

pub struct UiReferencePage {
    pub ui_reference_card_titles: Vec<String>,
    pub ui_reference_form_text: String,
    pub ui_reference_form_dropdown: Option<String>,
    pub ui_reference_readme_content: String,
}

impl UiReferencePage {
    pub fn new() -> Self {
        Self {
            ui_reference_card_titles: vec![
                "Project Alpha".to_string(),
                "Project Beta".to_string(),
                "Project Gamma".to_string(),
                "Project Delta".to_string(),
                "Project Epsilon".to_string(),
            ],
            ui_reference_form_text: String::new(),
            ui_reference_form_dropdown: None,
            ui_reference_readme_content: String::new(),
        }
    }
}

impl Default for UiReferencePage {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::pages::Page for UiReferencePage {
    fn name(&self) -> &'static str {
        "UI Reference"
    }

    fn ui(&mut self, _ctx: &Context, ui: &mut Ui, state: &mut AppState) {
        ui.heading("UI Reference");

        ui.vertical_centered(|ui| {
            ui.label("UI Widget Gallery and Reference");
            ui.add_space(20.0);

            if ui.button("← Back to Settings").clicked() {
                state.ui_state.current_page = UiPage::Settings;
            }

            ui.add_space(20.0);

            if ui.button("2 column main content").clicked() {
                state.ui_state.current_page = UiPage::UiTwoColumnMainContent;
            }
        });
    }
}

pub struct UiTwoColumnMainContentPage {
    pub ui_reference_card_titles: Vec<String>,
    pub ui_reference_form_text: String,
    pub ui_reference_form_dropdown: Option<String>,
    pub ui_reference_readme_content: String,
}

impl UiTwoColumnMainContentPage {
    pub fn new() -> Self {
        Self {
            ui_reference_card_titles: vec![
                "Project Alpha".to_string(),
                "Project Beta".to_string(),
                "Project Gamma".to_string(),
                "Project Delta".to_string(),
                "Project Epsilon".to_string(),
            ],
            ui_reference_form_text: String::new(),
            ui_reference_form_dropdown: None,
            ui_reference_readme_content: String::new(),
        }
    }
}

impl Default for UiTwoColumnMainContentPage {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::pages::Page for UiTwoColumnMainContentPage {
    fn name(&self) -> &'static str {
        "2 Column Main Content"
    }

    fn ui(&mut self, _ctx: &Context, ui: &mut Ui, state: &mut AppState) {
        // Header with back button
        ui.horizontal(|ui| {
            if ui.button("← Back to UI Reference").clicked() {
                state.ui_state.current_page = UiPage::UiReference;
            }

            ui.add_space(10.0);
            ui.heading("2 Column Main Content");
        });

        ui.add_space(10.0);

        // Two-column layout with independent scroll
        let available_size = ui.available_size_before_wrap();
        let left_column_width = 400.0;

        ui.horizontal(|ui| {
            // First column (400px wide) with independent scroll
            ui.allocate_ui_with_layout(
                egui::vec2(left_column_width, available_size.y),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.heading("First Column");

                    egui::ScrollArea::vertical()
                        .id_salt("first_column_scroll")
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            ui.add_space(8.0);

                            // Form above the cards
                            egui::Frame::NONE
                                .fill(ui.style().visuals.widgets.inactive.bg_fill)
                                .corner_radius(4.0)
                                .inner_margin(egui::Margin::same(12))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.vertical(|ui| {
                                        // Textarea (full-width, 3 rows)
                                        ui.label("Text Input:");
                                        ui.add(
                                            egui::TextEdit::multiline(&mut self.ui_reference_form_text)
                                                .desired_width(ui.available_width())
                                                .desired_rows(3)
                                                .hint_text("Enter your text here...")
                                        );

                                        ui.add_space(8.0);

                                        // Dropdown
                                        ui.label("Select Option:");
                                        let dropdown_options = vec!["Option 1", "Option 2", "Option 3", "Option 4"];
                                        egui::ComboBox::from_id_salt("ui_reference_dropdown")
                                            .selected_text(
                                                self.ui_reference_form_dropdown
                                                    .as_ref()
                                                    .unwrap_or(&"Select an option".to_string())
                                            )
                                            .show_ui(ui, |ui| {
                                                for option in dropdown_options {
                                                    ui.selectable_value(
                                                        &mut self.ui_reference_form_dropdown,
                                                        Some(option.to_string()),
                                                        option,
                                                    );
                                                }
                                            });

                                        ui.add_space(8.0);

                                        // Submit button
                                        ui.horizontal(|ui| {
                                            if ui.button("Submit").clicked() {
                                                // Form does not do anything
                                            }
                                        });
                                    });
                                });

                            ui.add_space(16.0);

                            // Display Card items with random titles
                            for (i, title) in self.ui_reference_card_titles.iter().enumerate() {
                                let card_height = 80.0;

                                ui.allocate_ui_with_layout(
                                    egui::vec2(ui.available_width(), card_height),
                                    egui::Layout::top_down(egui::Align::LEFT),
                                    |ui| {
                                        egui::Frame::NONE
                                            .fill(ui.style().visuals.widgets.inactive.bg_fill)
                                            .corner_radius(4.0)
                                            .inner_margin(egui::Margin::same(12))
                                            .show(ui, |ui| {
                                                ui.set_width(ui.available_width());
                                                ui.vertical(|ui| {
                                                    ui.label(egui::RichText::new(title).size(16.0).strong());
                                                    ui.add_space(4.0);
                                                    ui.label(egui::RichText::new(format!("Card item {}", i + 1))
                                                        .size(12.0)
                                                        .color(ui.style().visuals.weak_text_color()));
                                                    ui.add_space(2.0);
                                                    ui.label(egui::RichText::new("This is a sample card with some description text.")
                                                        .size(11.0)
                                                        .color(ui.style().visuals.weak_text_color()));
                                                });
                                            });
                                    });

                                ui.add_space(8.0);
                            }
                        });
                }
            );
        });
    }
}

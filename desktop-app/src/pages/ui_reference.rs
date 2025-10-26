use crate::pages::Page;
use crate::state::ui_state::Page as UiPage;
use crate::state::AppState;
use crate::ui_text::{ContentText, MarkdownText, WidgetText};
use egui::{Context, Ui};

/// Text style specification for markdown-like content
/// This allows us to maintain consistent styling across the app and easily change styles later
pub struct TextStyles;

impl TextStyles {
    /// Heading level 1 (main headings like "# nocodo")
    pub fn heading_1(ui: &Ui, text: &str) -> egui::RichText {
        egui::RichText::new(text)
            .size(24.0)
            .strong()
            .color(ui.style().visuals.text_color())
    }

    /// Heading level 2 (section headings like "## What is nocodo?")
    pub fn heading_2(ui: &Ui, text: &str) -> egui::RichText {
        egui::RichText::new(text)
            .size(20.0)
            .strong()
            .color(ui.style().visuals.text_color())
    }

    /// Heading level 3 (subsection headings like "### Core Features")
    pub fn heading_3(ui: &Ui, text: &str) -> egui::RichText {
        egui::RichText::new(text)
            .size(16.0)
            .strong()
            .color(ui.style().visuals.text_color())
    }

    /// Regular paragraph text
    pub fn paragraph(ui: &Ui, text: &str) -> egui::RichText {
        egui::RichText::new(text)
            .size(14.0)
            .color(ui.style().visuals.text_color())
    }

    /// Bold text for emphasis
    pub fn bold(ui: &Ui, text: &str) -> egui::RichText {
        egui::RichText::new(text)
            .size(14.0)
            .strong()
            .color(ui.style().visuals.text_color())
    }

    /// Italic text (egui doesn't support true italics, so we'll use a different color)
    pub fn italic(ui: &Ui, text: &str) -> egui::RichText {
        egui::RichText::new(text)
            .size(14.0)
            .color(ui.style().visuals.weak_text_color())
    }

    /// Link/URL text
    pub fn link(_ui: &Ui, text: &str) -> egui::RichText {
        egui::RichText::new(text)
            .size(14.0)
            .color(egui::Color32::from_rgb(100, 149, 237)) // Cornflower blue
            .underline()
    }

    /// Bullet point text
    pub fn bullet(ui: &Ui, text: &str) -> egui::RichText {
        egui::RichText::new(format!("• {}", text))
            .size(14.0)
            .color(ui.style().visuals.text_color())
    }

    /// Code inline text
    pub fn code_inline(ui: &Ui, text: &str) -> egui::RichText {
        egui::RichText::new(format!("`{}`", text))
            .size(13.0)
            .family(egui::FontFamily::Monospace)
            .color(egui::Color32::from_rgb(200, 100, 100)) // Light red for code
    }

    /// Code block background color
    pub fn code_block_bg(ui: &Ui) -> egui::Color32 {
        ui.style().visuals.code_bg_color
    }

    /// Quote/blockquote text
    pub fn quote(ui: &Ui, text: &str) -> egui::RichText {
        egui::RichText::new(format!("  {}", text))
            .size(14.0)
            .color(ui.style().visuals.weak_text_color())
    }
}

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
                "Project Zeta".to_string(),
                "Project Eta".to_string(),
                "Project Theta".to_string(),
                "Project Iota".to_string(),
                "Project Kappa".to_string(),
                "Project Lambda".to_string(),
                "Project Mu".to_string(),
                "Project Nu".to_string(),
                "Project Xi".to_string(),
                "Project Omicron".to_string(),
                "Project Pi".to_string(),
                "Project Rho".to_string(),
                "Project Sigma".to_string(),
                "Project Tau".to_string(),
                "Project Upsilon".to_string(),
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
        // Using Ubuntu SemiBold for page heading
        ui.heading(WidgetText::page_heading("UI Reference"));

        ui.vertical_centered(|ui| {
            // Using Ubuntu Light for description
            ui.label(WidgetText::label("UI Widget Gallery and Reference"));
            ui.add_space(20.0);

            // Using Ubuntu SemiBold for buttons
            if ui
                .button(WidgetText::button("← Back to Settings"))
                .clicked()
            {
                state.ui_state.current_page = UiPage::Settings;
            }

            ui.add_space(20.0);

            if ui
                .button(WidgetText::button("2 column main content"))
                .clicked()
            {
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
        // Load README.md content from the project root
        let readme_content = Self::load_readme();

        Self {
            ui_reference_card_titles: vec![
                "Project Alpha".to_string(),
                "Project Beta".to_string(),
                "Project Gamma".to_string(),
                "Project Delta".to_string(),
                "Project Epsilon".to_string(),
                "Project Zeta".to_string(),
                "Project Eta".to_string(),
                "Project Theta".to_string(),
                "Project Iota".to_string(),
                "Project Kappa".to_string(),
                "Project Lambda".to_string(),
                "Project Mu".to_string(),
                "Project Nu".to_string(),
                "Project Xi".to_string(),
                "Project Omicron".to_string(),
                "Project Pi".to_string(),
                "Project Rho".to_string(),
                "Project Sigma".to_string(),
                "Project Tau".to_string(),
                "Project Upsilon".to_string(),
            ],
            ui_reference_form_text: String::new(),
            ui_reference_form_dropdown: None,
            ui_reference_readme_content: readme_content,
        }
    }

    /// Load README.md from the project root directory
    fn load_readme() -> String {
        // Try to find README.md in the project root
        // The executable is typically in target/debug or target/release,
        // so we need to go up to find the project root
        let possible_paths = vec![
            "README.md",          // If run from project root
            "../README.md",       // If run from desktop-app
            "../../README.md",    // If run from target/debug
            "../../../README.md", // If run from target/debug/deps
        ];

        for path in possible_paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                return content;
            }
        }

        // If README.md not found, return a placeholder
        "# README.md not found\n\nCould not locate README.md file in the project directory."
            .to_string()
    }
}

impl Default for UiTwoColumnMainContentPage {
    fn default() -> Self {
        Self::new()
    }
}

impl UiTwoColumnMainContentPage {
    /// Renders the README.md content with proper markdown-like styling
    fn render_readme_content(&self, ui: &mut Ui) {
        // If README content is not loaded, show a message
        if self.ui_reference_readme_content.is_empty() {
            ui.label(TextStyles::italic(ui, "README.md content not loaded"));
            return;
        }

        // Simple markdown-like parser for the README content
        let lines: Vec<&str> = self.ui_reference_readme_content.lines().collect();
        let mut in_code_block = false;
        let mut code_block_content = String::new();

        for line in lines {
            // Code blocks
            if line.starts_with("```") {
                if in_code_block {
                    // End of code block - render it
                    if !code_block_content.is_empty() {
                        egui::Frame::NONE
                            .fill(TextStyles::code_block_bg(ui))
                            .corner_radius(4.0)
                            .inner_margin(egui::Margin::same(8))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(&code_block_content)
                                        .family(egui::FontFamily::Monospace)
                                        .size(13.0),
                                );
                            });
                        code_block_content.clear();
                        ui.add_space(8.0);
                    }
                }
                in_code_block = !in_code_block;
                continue;
            }

            if in_code_block {
                code_block_content.push_str(line);
                code_block_content.push('\n');
                continue;
            }

            // Skip empty lines
            if line.trim().is_empty() {
                ui.add_space(6.0);
                continue;
            }

            // Headings
            if line.starts_with("# ") {
                ui.label(TextStyles::heading_1(ui, &line[2..]));
                ui.add_space(8.0);
            } else if line.starts_with("## ") {
                ui.label(TextStyles::heading_2(ui, &line[3..]));
                ui.add_space(6.0);
            } else if line.starts_with("### ") {
                ui.label(TextStyles::heading_3(ui, &line[4..]));
                ui.add_space(4.0);
            }
            // Blockquotes
            else if line.starts_with("> ") {
                ui.label(TextStyles::quote(ui, &line[2..]));
            }
            // Bullet points
            else if line.trim_start().starts_with("- ") {
                let indent_level = line.chars().take_while(|c| c.is_whitespace()).count();
                ui.horizontal(|ui| {
                    ui.add_space(indent_level as f32 * 8.0);
                    ui.label(TextStyles::bullet(ui, &line.trim_start()[2..]));
                });
            }
            // Regular paragraphs
            else {
                // Simple inline formatting
                self.render_inline_text(ui, line);
            }
        }
    }

    /// Renders a line of text with inline formatting (bold, links, code)
    fn render_inline_text(&self, ui: &mut Ui, line: &str) {
        ui.horizontal_wrapped(|ui| {
            let mut remaining = line;

            while !remaining.is_empty() {
                // Check for bold text **text**
                if let Some(bold_start) = remaining.find("**") {
                    // Render text before bold
                    if bold_start > 0 {
                        ui.label(TextStyles::paragraph(ui, &remaining[..bold_start]));
                    }

                    // Find end of bold
                    if let Some(bold_end) = remaining[bold_start + 2..].find("**") {
                        let bold_text = &remaining[bold_start + 2..bold_start + 2 + bold_end];
                        ui.label(TextStyles::bold(ui, bold_text));
                        remaining = &remaining[bold_start + 2 + bold_end + 2..];
                        continue;
                    }
                }

                // Check for links [text](url)
                if let Some(link_start) = remaining.find('[') {
                    if let Some(link_mid) = remaining[link_start..].find("](") {
                        if let Some(link_end) = remaining[link_start + link_mid..].find(')') {
                            // Render text before link
                            if link_start > 0 {
                                ui.label(TextStyles::paragraph(ui, &remaining[..link_start]));
                            }

                            let link_text = &remaining[link_start + 1..link_start + link_mid];
                            ui.label(TextStyles::link(ui, link_text));
                            remaining = &remaining[link_start + link_mid + link_end + 1..];
                            continue;
                        }
                    }
                }

                // Check for inline code `code`
                if let Some(code_start) = remaining.find('`') {
                    // Render text before code
                    if code_start > 0 {
                        ui.label(TextStyles::paragraph(ui, &remaining[..code_start]));
                    }

                    // Find end of code
                    if let Some(code_end) = remaining[code_start + 1..].find('`') {
                        let code_text = &remaining[code_start + 1..code_start + 1 + code_end];
                        ui.label(TextStyles::code_inline(ui, code_text));
                        remaining = &remaining[code_start + 1 + code_end + 1..];
                        continue;
                    }
                }

                // No special formatting found, render the rest
                ui.label(TextStyles::paragraph(ui, remaining));
                break;
            }
        });
    }
}

impl crate::pages::Page for UiTwoColumnMainContentPage {
    fn name(&self) -> &'static str {
        "2 Column Main Content"
    }

    fn ui(&mut self, _ctx: &Context, ui: &mut Ui, state: &mut AppState) {
        // Header with back button
        ui.horizontal(|ui| {
            // Ubuntu SemiBold for button
            if ui
                .button(WidgetText::button("← Back to UI Reference"))
                .clicked()
            {
                state.ui_state.current_page = UiPage::UiReference;
            }

            ui.add_space(10.0);
            // Ubuntu SemiBold for page heading
            ui.heading(WidgetText::page_heading("2 Column Main Content"));
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
                    // Ubuntu SemiBold for section heading
                    ui.heading(WidgetText::section_heading("First Column"));

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
                                        // Textarea (full-width, 3 rows) - Ubuntu Light for label
                                        ui.label(WidgetText::label("Text Input:"));
                                        ui.add(
                                            egui::TextEdit::multiline(&mut self.ui_reference_form_text)
                                                .desired_width(ui.available_width())
                                                .desired_rows(3)
                                                .hint_text("Enter your text here...")
                                        );

                                        ui.add_space(8.0);

                                        // Dropdown - Ubuntu Light for label
                                        ui.label(WidgetText::label("Select Option:"));
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

                                        // Submit button - Ubuntu SemiBold
                                        ui.horizontal(|ui| {
                                            if ui.button(WidgetText::button("Submit")).clicked() {
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
                                                    // User content - Inter Regular (title)
                                                    ui.label(ContentText::title(title));
                                                    ui.add_space(4.0);
                                                    // User content - Inter Regular (subtitle)
                                                    ui.label(ContentText::subtitle(ui, format!("Card item {}", i + 1)));
                                                    ui.add_space(2.0);
                                                    // User content - Inter Regular (description)
                                                    ui.label(ContentText::description(ui, "This is a sample card with some description text."));
                                                });
                                            });
                                    });

                                ui.add_space(8.0);
                            }
                        });
                }
            );

            ui.add_space(16.0);

            // Second column (remaining width) with independent scroll
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), available_size.y),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    // Ubuntu SemiBold for section heading
                    ui.heading(WidgetText::section_heading("Second Column - README.md"));

                    egui::ScrollArea::vertical()
                        .id_salt("second_column_scroll")
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            ui.add_space(8.0);

                            // Render README.md content with styling
                            self.render_readme_content(ui);
                        });
                }
            );
        });
    }
}

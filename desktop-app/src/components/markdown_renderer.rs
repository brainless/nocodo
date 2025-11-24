use egui::Ui;

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
    pub fn code_inline(_ui: &Ui, text: &str) -> egui::RichText {
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

/// Markdown renderer for displaying markdown content with proper styling
pub struct MarkdownRenderer;

impl MarkdownRenderer {
    /// Renders markdown content with proper styling
    pub fn render(ui: &mut Ui, content: &str) {
        if content.is_empty() {
            ui.label(TextStyles::italic(ui, "No content to display"));
            return;
        }

        // Simple markdown-like parser
        let lines: Vec<&str> = content.lines().collect();
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
            if let Some(stripped) = line.strip_prefix("# ") {
                ui.label(TextStyles::heading_1(ui, stripped));
                ui.add_space(8.0);
            } else if let Some(stripped) = line.strip_prefix("## ") {
                ui.label(TextStyles::heading_2(ui, stripped));
                ui.add_space(6.0);
            } else if let Some(stripped) = line.strip_prefix("### ") {
                ui.label(TextStyles::heading_3(ui, stripped));
                ui.add_space(4.0);
            }
            // Blockquotes
            else if let Some(stripped) = line.strip_prefix("> ") {
                ui.label(TextStyles::quote(ui, stripped));
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
                Self::render_inline_text(ui, line);
            }
        }
    }

    /// Renders a line of text with inline formatting (bold, links, code)
    fn render_inline_text(ui: &mut Ui, line: &str) {
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
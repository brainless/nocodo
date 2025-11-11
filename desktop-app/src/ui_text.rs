/// Centralized text styling module for nocodo desktop app
///
/// This module provides a consistent API for rendering text with the correct fonts:
/// - UI widgets (buttons, labels, headings, navigation) use Ubuntu font
/// - User content (names, descriptions, data) use Inter font
/// - Code uses monospace Inter font
///
/// Usage examples:
/// ```
/// use nocodo_desktop_app::ui_text::{WidgetText, ContentText};
///
/// // UI widget text (Ubuntu) - returns RichText that can be used with ui
/// let heading = WidgetText::page_heading("Projects");
/// let label = WidgetText::label("API Key:");
/// let button = WidgetText::button("Connect");
///
/// // User content text (Inter) - returns RichText that can be used with ui
/// let title = ContentText::title("Project Name");
/// // Note: Some methods require a &ui parameter for styling
/// ```
use egui::{Color32, RichText, Ui};

/// Font family definitions for different text types
pub struct AppFonts;

impl AppFonts {
    /// Font for regular UI widgets (labels, status messages, navigation)
    /// Uses Ubuntu Light
    pub fn ui_font_light() -> egui::FontFamily {
        egui::FontFamily::Name("ui_light".into())
    }

    /// Font for emphasized UI widgets (buttons, headings, CTAs)
    /// Uses Ubuntu SemiBold
    pub fn ui_font_semibold() -> egui::FontFamily {
        egui::FontFamily::Name("ui_semibold".into())
    }

    /// Font for user-generated content (names, descriptions, data)
    /// Uses Inter Regular
    pub fn content_font() -> egui::FontFamily {
        egui::FontFamily::Proportional
    }

    /// Font for code/monospace text
    /// Uses Inter Medium
    pub fn code_font() -> egui::FontFamily {
        egui::FontFamily::Monospace
    }
}

/// UI Widget Text Helpers
///
/// All methods in this struct apply the Ubuntu font automatically.
/// Use these for:
/// - Page headings, section headings
/// - Form labels, button text
/// - Navigation items
/// - Status messages, error messages
/// - Any static UI text
pub struct WidgetText;

impl WidgetText {
    /// Page heading (large, SemiBold)
    ///
    /// Example: `ui.heading(WidgetText::page_heading("Projects"))`
    pub fn page_heading(text: impl Into<String>) -> RichText {
        RichText::new(text).family(AppFonts::ui_font_semibold())
    }

    /// Section heading (medium, SemiBold)
    ///
    /// Example: `ui.heading(WidgetText::section_heading("API Keys"))`
    pub fn section_heading(text: impl Into<String>) -> RichText {
        RichText::new(text).family(AppFonts::ui_font_semibold())
    }

    /// Form label (Light)
    ///
    /// Example: `ui.label(WidgetText::label("Username:"))`
    pub fn label(text: impl Into<String>) -> RichText {
        RichText::new(text).family(AppFonts::ui_font_light())
    }

    /// Button text (SemiBold)
    ///
    /// Example: `ui.button(WidgetText::button("Connect"))`
    pub fn button(text: impl Into<String>) -> RichText {
        RichText::new(text).family(AppFonts::ui_font_semibold())
    }

    /// Status message (Light)
    ///
    /// Example: `ui.label(WidgetText::status("Loading..."))`
    pub fn status(text: impl Into<String>) -> RichText {
        RichText::new(text).family(AppFonts::ui_font_light())
    }

    /// Navigation item (Light)
    ///
    /// Example: `ui.label(WidgetText::nav_item("Settings"))`
    pub fn nav_item(text: impl Into<String>) -> RichText {
        RichText::new(text).family(AppFonts::ui_font_light())
    }

    /// Error message (red, Light)
    ///
    /// Example: `ui.label(WidgetText::error("Connection failed"))`
    pub fn error(text: impl Into<String>) -> RichText {
        RichText::new(text)
            .family(AppFonts::ui_font_light())
            .color(Color32::RED)
    }

    /// Success message (green, Light)
    ///
    /// Example: `ui.label(WidgetText::success("Connected"))`
    pub fn success(text: impl Into<String>) -> RichText {
        RichText::new(text)
            .family(AppFonts::ui_font_light())
            .color(Color32::GREEN)
    }

    /// Warning message (yellow/orange, Light)
    ///
    /// Example: `ui.label(WidgetText::warning("No models configured"))`
    pub fn warning(text: impl Into<String>) -> RichText {
        RichText::new(text)
            .family(AppFonts::ui_font_light())
            .color(Color32::from_rgb(255, 165, 0))
    }

    /// Hint text (muted, small, Light)
    ///
    /// Example: `ui.label(WidgetText::hint("Optional field"))`
    pub fn hint(ui: &Ui, text: impl Into<String>) -> RichText {
        RichText::new(text)
            .family(AppFonts::ui_font_light())
            .size(11.0)
            .color(ui.style().visuals.weak_text_color())
    }

    /// Table header (SemiBold)
    ///
    /// Example: `ui.label(WidgetText::table_header("Name"))`
    pub fn table_header(text: impl Into<String>) -> RichText {
        RichText::new(text).family(AppFonts::ui_font_semibold())
    }

    /// App branding text (large, SemiBold)
    ///
    /// Example: `ui.label(WidgetText::branding("nocodo"))`
    pub fn branding(text: impl Into<String>) -> RichText {
        RichText::new(text)
            .family(AppFonts::ui_font_semibold())
            .size(20.0)
    }
}

/// User Content Text Helpers
///
/// All methods in this struct apply the Inter font automatically.
/// Use these for:
/// - Project names, work titles
/// - Descriptions, user messages
/// - File paths, usernames
/// - Any data from database or API
pub struct ContentText;

impl ContentText {
    /// Title/heading for user content (large, bold)
    ///
    /// Example: `ui.label(ContentText::title(&project.name))`
    pub fn title(text: impl Into<String>) -> RichText {
        RichText::new(text)
            .size(16.0)
            .strong()
            .family(AppFonts::content_font())
    }

    /// Subtitle/secondary title (medium, regular)
    ///
    /// Example: `ui.label(ContentText::subtitle(&project.path))`
    pub fn subtitle(ui: &Ui, text: impl Into<String>) -> RichText {
        RichText::new(text)
            .size(12.0)
            .color(ui.style().visuals.weak_text_color())
            .family(AppFonts::content_font())
    }

    /// Description text (medium, muted)
    ///
    /// Example: `ui.label(ContentText::description(ui, &project.description))`
    pub fn description(ui: &Ui, text: impl Into<String>) -> RichText {
        RichText::new(text)
            .size(12.0)
            .color(ui.style().visuals.weak_text_color())
            .family(AppFonts::content_font())
    }

    /// Small metadata text (small, muted)
    ///
    /// Example: `ui.label(ContentText::metadata(ui, &created_at))`
    pub fn metadata(ui: &Ui, text: impl Into<String>) -> RichText {
        RichText::new(text)
            .size(11.0)
            .color(ui.style().visuals.weak_text_color())
            .family(AppFonts::content_font())
    }

    /// Regular content text (default size, default color)
    ///
    /// Example: `ui.label(ContentText::text(&message.content))`
    pub fn text(text: impl Into<String>) -> RichText {
        RichText::new(text).family(AppFonts::content_font())
    }

    /// Bold content text (default size, bold)
    ///
    /// Example: `ui.label(ContentText::bold(&username))`
    pub fn bold(text: impl Into<String>) -> RichText {
        RichText::new(text)
            .strong()
            .family(AppFonts::content_font())
    }

    /// Code inline (monospace, small)
    ///
    /// Example: `ui.label(ContentText::code_inline("npm install"))`
    pub fn code_inline(text: impl Into<String>) -> RichText {
        RichText::new(format!("`{}`", text.into()))
            .family(AppFonts::code_font())
            .size(13.0)
            .color(Color32::from_rgb(200, 100, 100))
    }

    /// Badge/tag text (small, colored background needed separately)
    ///
    /// Example: `ui.label(ContentText::badge(&status))`
    pub fn badge(text: impl Into<String>) -> RichText {
        RichText::new(text)
            .size(11.0)
            .family(AppFonts::content_font())
    }

    /// Timestamp/date text (small, muted)
    ///
    /// Example: `ui.label(ContentText::timestamp(ui, &created_at))`
    pub fn timestamp(ui: &Ui, text: impl Into<String>) -> RichText {
        RichText::new(text)
            .size(10.0)
            .color(ui.style().visuals.weak_text_color())
            .family(AppFonts::content_font())
    }
}

/// Markdown-specific text helpers (for UI Reference page README rendering)
///
/// These use Ubuntu font for structural elements (headings, bullets)
/// and Inter for content (paragraphs, links, code)
pub struct MarkdownText;

impl MarkdownText {
    /// Markdown heading level 1 (SemiBold)
    pub fn h1(text: impl Into<String>) -> RichText {
        RichText::new(text)
            .size(24.0)
            .family(AppFonts::ui_font_semibold())
    }

    /// Markdown heading level 2 (SemiBold)
    pub fn h2(text: impl Into<String>) -> RichText {
        RichText::new(text)
            .size(20.0)
            .family(AppFonts::ui_font_semibold())
    }

    /// Markdown heading level 3 (SemiBold)
    pub fn h3(text: impl Into<String>) -> RichText {
        RichText::new(text)
            .size(16.0)
            .family(AppFonts::ui_font_semibold())
    }

    /// Markdown paragraph (Light)
    pub fn paragraph(text: impl Into<String>) -> RichText {
        RichText::new(text)
            .size(14.0)
            .family(AppFonts::ui_font_light())
    }

    /// Markdown bold text (SemiBold)
    pub fn bold(text: impl Into<String>) -> RichText {
        RichText::new(text)
            .size(14.0)
            .family(AppFonts::ui_font_semibold())
    }

    /// Markdown italic text (Light, using weak color as substitute)
    pub fn italic(ui: &Ui, text: impl Into<String>) -> RichText {
        RichText::new(text)
            .size(14.0)
            .color(ui.style().visuals.weak_text_color())
            .family(AppFonts::ui_font_light())
    }

    /// Markdown link (Light)
    pub fn link(text: impl Into<String>) -> RichText {
        RichText::new(text)
            .size(14.0)
            .color(Color32::from_rgb(100, 149, 237))
            .underline()
            .family(AppFonts::ui_font_light())
    }

    /// Markdown bullet point (Light)
    pub fn bullet(text: impl Into<String>) -> RichText {
        RichText::new(format!("• {}", text.into()))
            .size(14.0)
            .family(AppFonts::ui_font_light())
    }

    /// Markdown code inline (Monospace - Inter Medium)
    pub fn code_inline(text: impl Into<String>) -> RichText {
        RichText::new(format!("`{}`", text.into()))
            .size(13.0)
            .family(AppFonts::code_font())
            .color(Color32::from_rgb(200, 100, 100))
    }

    /// Markdown quote/blockquote (Light)
    pub fn quote(ui: &Ui, text: impl Into<String>) -> RichText {
        RichText::new(format!("  {}", text.into()))
            .size(14.0)
            .color(ui.style().visuals.weak_text_color())
            .family(AppFonts::ui_font_light())
    }

    /// Code block background color
    pub fn code_block_bg(ui: &Ui) -> Color32 {
        ui.style().visuals.code_bg_color
    }
}

use egui::{Color32, RichText, Ui};

/// Common styling and layout for message containers
pub struct MessageContainer;

impl MessageContainer {
    /// Creates a standard message frame with consistent styling
    pub fn frame(_ui: &mut Ui, bg_color: Color32) -> egui::Frame {
        egui::Frame::NONE
            .fill(bg_color)
            .corner_radius(8.0)
            .inner_margin(egui::Margin::same(12))
    }

    /// Renders message header with sender and timestamp
    pub fn header(ui: &mut Ui, sender: &str, timestamp: i64) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(sender).size(12.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let datetime = chrono::DateTime::from_timestamp(timestamp, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                ui.label(RichText::new(datetime).size(10.0).color(ui.style().visuals.weak_text_color()));
            });
        });
    }
}

/// Renderer for user messages
pub struct UserMessageRenderer;

impl UserMessageRenderer {
    /// Renders a user message with consistent styling
    pub fn render(ui: &mut Ui, content: &str, timestamp: i64) {
        let bg_color = ui.style().visuals.widgets.inactive.bg_fill;

        MessageContainer::frame(ui, bg_color).show(ui, |ui| {
            ui.vertical(|ui| {
                MessageContainer::header(ui, "User", timestamp);
                ui.add_space(4.0);
                ui.label(RichText::new(content).size(16.0));
            });
        });
    }
}

/// Renderer for AI assistant messages
pub struct AiMessageRenderer;

impl AiMessageRenderer {
    /// Renders an AI assistant message with consistent styling
    pub fn render_text(ui: &mut Ui, content: &str, model: Option<&str>, timestamp: i64) {
        let bg_color = ui.style().visuals.widgets.noninteractive.bg_fill;

        MessageContainer::frame(ui, bg_color).show(ui, |ui| {
            ui.vertical(|ui| {
                let label = if let Some(model_name) = model {
                    format!("AI - {}", model_name)
                } else {
                    "AI".to_string()
                };
                
                MessageContainer::header(ui, &label, timestamp);
                ui.add_space(4.0);
                ui.label(RichText::new(content).size(16.0));
            });
        });
    }

    /// Renders a regular AI response (non-text content)
    pub fn render_response(ui: &mut Ui, content: &str, role: &str, model: Option<&str>, timestamp: i64) {
        let bg_color = ui.style().visuals.widgets.noninteractive.bg_fill;

        MessageContainer::frame(ui, bg_color).show(ui, |ui| {
            ui.vertical(|ui| {
                let label = match (role, model) {
                    ("tool", _) => "nocodo".to_string(),
                    ("assistant", Some(model_name)) => format!("AI - {}", model_name),
                    _ => "AI".to_string(),
                };
                
                MessageContainer::header(ui, &label, timestamp);
                ui.add_space(4.0);
                ui.label(RichText::new(content).size(16.0));
            });
        });
    }
}
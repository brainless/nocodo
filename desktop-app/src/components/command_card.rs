use crate::ui_text::{ContentText, WidgetText};
use egui::{Response, Ui};
use shared_types::ProjectCommand;

pub struct CommandCard<'a> {
    command: &'a ProjectCommand,
    last_executed_at: Option<i64>,
    on_click: Option<Box<dyn Fn() + 'a>>,
}

impl<'a> CommandCard<'a> {
    pub fn new(command: &'a ProjectCommand) -> Self {
        Self {
            command,
            last_executed_at: None,
            on_click: None,
        }
    }

    pub fn last_executed_at(mut self, last_executed_at: Option<i64>) -> Self {
        self.last_executed_at = last_executed_at;
        self
    }

    pub fn on_click<F: Fn() + 'a>(mut self, f: F) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }

    fn format_last_executed(&self, timestamp: i64) -> String {
        let now = chrono::Utc::now().timestamp();
        let diff = now - timestamp;

        if diff < 60 {
            "just now".to_string()
        } else if diff < 3600 {
            let minutes = diff / 60;
            format!("{}m ago", minutes)
        } else if diff < 86400 {
            let hours = diff / 3600;
            format!("{}h ago", hours)
        } else if diff < 604800 {
            let days = diff / 86400;
            format!("{}d ago", days)
        } else {
            // For older dates, show the actual date
            let datetime =
                chrono::DateTime::from_timestamp(timestamp, 0).unwrap_or_else(chrono::Utc::now);
            datetime.format("%b %d, %Y").to_string()
        }
    }

    pub fn ui(self, ui: &mut Ui) -> Response {
        let card_height = if self.command.description.is_some() {
            60.0
        } else {
            44.0
        };
        let available_width = ui.available_width();

        let response = ui.allocate_ui_with_layout(
            egui::vec2(available_width, card_height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.set_width(available_width);
                egui::Frame::NONE
                    .fill(ui.style().visuals.widgets.inactive.bg_fill)
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.set_width(available_width - 24.0); // Account for inner margin
                        ui.vertical(|ui| {
                            // Command name - Inter Medium (user content)
                            ui.label(ContentText::title(&self.command.name));

                            // Description if present
                            if let Some(description) = &self.command.description {
                                ui.add_space(2.0);
                                ui.label(ContentText::description(ui, description));
                            }

                            // Last executed time
                            if let Some(last_executed) = self.last_executed_at {
                                ui.add_space(2.0);
                                ui.label(WidgetText::muted(format!(
                                    "Last: {}",
                                    self.format_last_executed(last_executed)
                                )));
                            }
                        });
                    });
            },
        );

        // Make the entire card clickable
        let response = response.response.interact(egui::Sense::click());

        // Change cursor to pointer on hover
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        // Add hover background
        if response.hovered() {
            let rect = response.rect;
            ui.painter()
                .rect_filled(rect, 6.0, ui.style().visuals.widgets.hovered.bg_fill);
        }

        // Handle click
        if response.clicked() {
            if let Some(on_click) = self.on_click {
                on_click();
            }
        }

        response
    }
}

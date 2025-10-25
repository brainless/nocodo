use egui::{Response, Ui};
use manager_models::Project;

pub struct ProjectCard<'a> {
    project: &'a Project,
    on_click: Option<Box<dyn Fn() + 'a>>,
}

impl<'a> ProjectCard<'a> {
    pub fn new(project: &'a Project) -> Self {
        Self {
            project,
            on_click: None,
        }
    }

    pub fn on_click<F: Fn() + 'a>(mut self, f: F) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }

    pub fn ui(self, ui: &mut Ui) -> Response {
        let card_width = 300.0;
        let card_height = 100.0;

        let response = ui.allocate_ui(egui::vec2(card_width, card_height), |ui| {
            egui::Frame::NONE
                .fill(ui.style().visuals.widgets.inactive.bg_fill)
                .corner_radius(8.0)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        // Project name - larger and bold
                        ui.label(egui::RichText::new(&self.project.name).size(16.0).strong());

                        ui.add_space(4.0);

                        // Project path - smaller, muted color
                        ui.label(
                            egui::RichText::new(&self.project.path)
                                .size(12.0)
                                .color(ui.style().visuals.weak_text_color()),
                        );

                        // Description if present
                        if let Some(description) = &self.project.description {
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(description)
                                    .size(11.0)
                                    .color(ui.style().visuals.weak_text_color()),
                            );
                        }
                    });
                });
        });

        // Make the entire card clickable
        let response = response.response.interact(egui::Sense::click());

        // Change cursor to pointer on hover
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
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

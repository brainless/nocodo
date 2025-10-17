use egui::{Context, Ui};
use manager_models::Project;

pub struct ProjectsView {
    projects: Vec<Project>,
}

impl ProjectsView {
    pub fn new(projects: Vec<Project>) -> Self {
        Self { projects }
    }

    pub fn render(&mut self, _ctx: &Context, ui: &mut Ui) {
        ui.heading("Projects");

        if self.projects.is_empty() {
            ui.label("No projects found");
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            let card_width = 300.0;
            let card_height = 100.0;
            let card_spacing = 10.0;

            // Set spacing between items
            ui.spacing_mut().item_spacing = egui::Vec2::new(card_spacing, card_spacing);

            // Use horizontal_wrapped to automatically create a responsive grid
            ui.horizontal_wrapped(|ui| {
                for project in &self.projects {
                    // Use allocate_ui with fixed size to enable proper wrapping
                    ui.allocate_ui(egui::vec2(card_width, card_height), |ui| {
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(&project.name).strong());
                                ui.label(egui::RichText::new(&project.path).small());
                                if let Some(description) = &project.description {
                                    ui.label(egui::RichText::new(description).italics().small());
                                }
                            });
                        });
                    });
                }
            });
        });
    }
}

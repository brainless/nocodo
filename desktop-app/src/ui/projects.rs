use egui::{Context, Ui};
use manager_models::Project;

pub struct ProjectsView {
    projects: Vec<Project>,
    on_project_click: Option<Box<dyn Fn(i64) + Send + Sync>>,
}

impl ProjectsView {
    pub fn new(projects: Vec<Project>) -> Self {
        Self {
            projects,
            on_project_click: None,
        }
    }

    pub fn set_on_project_click<F: Fn(i64) + Send + Sync + 'static>(&mut self, callback: F) {
        self.on_project_click = Some(Box::new(callback));
    }

    pub fn render(&mut self, _ctx: &Context, ui: &mut Ui) {
        ui.heading("Projects");

        if self.projects.is_empty() {
            ui.label("No projects found");
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            let card_width = 320.0;
            let card_height = 100.0;
            let card_spacing = 16.0;
            let available_width = ui.available_width();

            // Calculate number of columns (1, 2, or 3) based on available width
            let num_columns = if available_width >= (card_width * 3.0 + card_spacing * 2.0) {
                3
            } else if available_width >= (card_width * 2.0 + card_spacing * 1.0) {
                2
            } else {
                1
            };

            // Create rows with the calculated number of columns
            for row_start in (0..self.projects.len()).step_by(num_columns) {
                ui.horizontal(|ui| {
                    for col in 0..num_columns {
                        let idx = row_start + col;
                        if idx >= self.projects.len() {
                            break;
                        }

                        let project = &self.projects[idx];

                        // Allocate fixed width for the card
                        let response = ui.allocate_ui(
                            egui::vec2(card_width, card_height),
                            |ui| {
                                ui.set_width(card_width);
                                ui.group(|ui| {
                                    ui.set_width(card_width - 24.0); // Account for group margins
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new(&project.name).strong());
                                        ui.label(egui::RichText::new(&project.path).small());
                                        if let Some(description) = &project.description {
                                            ui.label(egui::RichText::new(description).italics().small());
                                        }
                                    });
                                });
                            },
                        );

                        if response.response.clicked() {
                            if let Some(ref callback) = self.on_project_click {
                                callback(project.id);
                            }
                        }

                        if response.response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }

                        // Add spacing between columns (except after the last column)
                        if col < num_columns - 1 && idx < self.projects.len() - 1 {
                            ui.add_space(card_spacing);
                        }
                    }
                });

                // Add spacing between rows
                ui.add_space(card_spacing);
            }
        });
    }
}

use egui::{Context, Ui};
use manager_models::Project;

pub struct ProjectsView {
    projects: Vec<Project>,
    on_project_click: Option<Box<dyn Fn(i64) + Send + Sync>>,
}

impl ProjectsView {
    pub fn new(projects: Vec<Project>) -> Self {
        Self { projects, on_project_click: None }
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
            let card_width = 300.0;
            let card_height = 100.0;
            let card_spacing = 10.0;

            // Set spacing between items
            ui.spacing_mut().item_spacing = egui::Vec2::new(card_spacing, card_spacing);

            // Use horizontal_wrapped to automatically create a responsive grid
            ui.horizontal_wrapped(|ui| {
                for project in &self.projects {
                    // Use allocate_ui with fixed size to enable proper wrapping
                     let response = ui.allocate_ui(egui::vec2(card_width, card_height), |ui| {
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

                     if response.response.clicked() {
                         if let Some(ref callback) = self.on_project_click {
                             callback(project.id);
                         }
                     }

                     if response.response.hovered() {
                         ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                     }
                }
            });
        });
    }
}

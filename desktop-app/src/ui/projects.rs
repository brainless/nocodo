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
            for project in &self.projects {
                ui.horizontal(|ui| {
                    ui.label(&project.name);
                    ui.label(&project.path);
                    if let Some(language) = &project.language {
                        ui.label(language);
                    }
                    if let Some(framework) = &project.framework {
                        ui.label(framework);
                    }
                });
                ui.separator();
            }
        });
    }
}
use crate::state::AppState;
use egui::{Context, Ui};

pub struct MentionsPage;

impl MentionsPage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MentionsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::pages::Page for MentionsPage {
    fn name(&self) -> &'static str {
        "Mentions"
    }

    fn ui(&mut self, _ctx: &Context, ui: &mut Ui, _state: &mut AppState) {
        ui.heading("Mentions");
        ui.label("Dummy Mentions page");
    }
}

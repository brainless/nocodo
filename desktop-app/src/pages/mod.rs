use crate::state::AppState;
use egui::{Context, Ui};

pub trait Page {
    fn name(&self) -> &'static str;
    fn ui(&mut self, ctx: &Context, ui: &mut Ui, state: &mut AppState);
    fn on_navigate_to(&mut self) {}
    fn on_navigate_from(&mut self) {}
}

pub mod mentions;
pub mod project_detail;
pub mod projects;
pub mod servers;
pub mod settings;
pub mod ui_reference;
pub mod work;

pub use mentions::*;
pub use project_detail::*;
pub use projects::*;
pub use servers::*;
pub use settings::*;
pub use ui_reference::*;
pub use work::*;

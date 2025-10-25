use crate::state::AppState;
use egui::Context;

pub struct ConnectionDialog;

impl ConnectionDialog {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConnectionDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionDialog {
    pub fn ui(&mut self, ctx: &Context, state: &mut AppState) -> bool {
        let mut should_close = false;

        if state.ui_state.show_connection_dialog {
            egui::Window::new("Connect to Server")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("SSH Server:");
                    ui.text_edit_singleline(&mut state.config.ssh.server);

                    ui.label("Username:");
                    ui.text_edit_singleline(&mut state.config.ssh.username);

                    ui.label("Port:");
                    ui.add(egui::DragValue::new(&mut state.config.ssh.port).range(1..=65535));

                    ui.label("SSH Key Path:");
                    ui.text_edit_singleline(&mut state.config.ssh.ssh_key_path);

                    ui.horizontal(|ui| {
                        if ui.button("Connect").clicked() {
                            self.connect(state);
                            state.ui_state.show_connection_dialog = false;
                            should_close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            state.ui_state.show_connection_dialog = false;
                            should_close = true;
                        }
                    });
                });
        }

        should_close
    }

    fn connect(&self, state: &mut AppState) {
        // This will be implemented when we extract the API methods
        // For now, just set the connection state to connecting
        state.connection_state = crate::state::ConnectionState::Connecting;
    }
}

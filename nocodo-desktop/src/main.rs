slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    ui.run()
}

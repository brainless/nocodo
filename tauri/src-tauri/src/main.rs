use std::path::Path;
use std::process::{Child, Command};
use std::sync::Mutex;

use tauri::Manager;

struct ApiChild(Mutex<Option<Child>>);

fn start_api(app: &tauri::AppHandle) -> Result<Child, String> {
    if let Ok(path) = std::env::var("NOCODO_BACKEND_PATH") {
        return Command::new(path)
            .spawn()
            .map_err(|e| format!("failed to spawn NOCODO_BACKEND_PATH: {e}"));
    }
    if let Ok(path) = std::env::var("DWATA_API_PATH") {
        return Command::new(path)
            .spawn()
            .map_err(|e| format!("failed to spawn DWATA_API_PATH: {e}"));
    }

    let mut candidates = Vec::new();

    if let Ok(exe_dir) = app.path().executable_dir() {
        candidates.push(exe_dir.join("nocodo-backend"));
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join("nocodo-backend"));
    }

    #[cfg(debug_assertions)]
    if let Ok(exe_dir) = app.path().executable_dir() {
        let mut dir: &Path = exe_dir.as_path();
        for _ in 0..8 {
            match dir.parent() {
                Some(p) => dir = p,
                None => break,
            }
            candidates.push(dir.join("target").join("debug").join("nocodo-backend"));
        }
    }

    if cfg!(target_os = "windows") {
        for candidate in &mut candidates {
            candidate.set_extension("exe");
        }
    }

    let candidate = candidates
        .into_iter()
        .filter(|path| path.exists())
        .max_by_key(|path| {
            path.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        })
        .ok_or_else(|| {
            "nocodo-backend sidecar not found (set NOCODO_BACKEND_PATH to the backend binary path)"
                .to_string()
        })?;

    eprintln!("[nocodo] starting backend from: {}", candidate.display());
    Command::new(candidate)
        .spawn()
        .map_err(|e| format!("failed to spawn nocodo-backend sidecar: {e}"))
}

fn stop_api(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<ApiChild>() {
        if let Some(mut child) = state.0.lock().ok().and_then(|mut g| g.take()) {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

struct ApiStartError(Mutex<Option<String>>);

#[tauri::command]
fn api_start_error(state: tauri::State<ApiStartError>) -> Option<String> {
    state.0.lock().ok().and_then(|g| g.clone())
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let error_state = match start_api(app.handle()) {
                Ok(child) => {
                    app.manage(ApiChild(Mutex::new(Some(child))));
                    ApiStartError(Mutex::new(None))
                }
                Err(err) => {
                    eprintln!("[nocodo] {err}");
                    ApiStartError(Mutex::new(Some(err)))
                }
            };
            app.manage(error_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![api_start_error])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                stop_api(&window.app_handle());
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            stop_api(app_handle);
        }
    });
}

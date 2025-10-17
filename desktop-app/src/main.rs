#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    tracing_subscriber::fmt::init();

    // Check for CLI test mode
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--test" {
        // Parse optional arguments: --test [server] [username] [keypath]
        let server = args.get(2).map(|s| s.as_str());
        let username = args.get(3).map(|s| s.as_str());
        let keypath = args.get(4).map(|s| s.as_str());
        return run_test_mode(server, username, keypath);
    }

    // Create tokio runtime that will live for the entire program
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    // Run the GUI on the tokio runtime
    rt.block_on(async {
        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([800.0, 600.0])
                .with_min_inner_size([600.0, 400.0]),
            ..Default::default()
        };

        eframe::run_native(
            "nocodo",
            native_options,
            Box::new(|cc| Ok(Box::new(nocodo_desktop_app::DesktopApp::new(cc)))),
        )
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn run_test_mode(
    server_arg: Option<&str>,
    username_arg: Option<&str>,
    keypath_arg: Option<&str>,
) -> eframe::Result {
    use nocodo_desktop_app::{api_client, config, ssh};

    println!("=== nocodo Desktop App - Test Mode ===\n");

    // Create tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        // Load configuration
        println!("1. Loading configuration...");
        let mut config = match config::DesktopConfig::load() {
            Ok(cfg) => {
                println!("   ✓ Configuration loaded");
                cfg
            }
            Err(e) => {
                println!("   ⚠ Failed to load configuration: {}", e);
                println!("   Using default configuration");
                config::DesktopConfig::default()
            }
        };

        // Override with CLI arguments if provided
        if let Some(server) = server_arg {
            println!("   → Overriding server with CLI arg: {}", server);
            config.ssh.server = server.to_string();
        }
        if let Some(username) = username_arg {
            println!("   → Overriding username with CLI arg: {}", username);
            config.ssh.username = username.to_string();
        }
        if let Some(keypath) = keypath_arg {
            println!("   → Overriding key path with CLI arg: {}", keypath);
            config.ssh.ssh_key_path = keypath.to_string();
        }

        println!("\n   Final configuration:");
        println!("     - Server: {}", config.ssh.server);
        println!("     - Username: {}", config.ssh.username);
        println!("     - SSH Key: {}", config.ssh.ssh_key_path);
        println!("     - Remote Port: {}\n", config.ssh.remote_port);

        // Test SSH connection
        println!("2. Attempting SSH connection...");
        let key_path = if config.ssh.ssh_key_path.is_empty() {
            println!("   → No key path specified, will try default locations");
            None
        } else {
            println!("   → Using key: {}", config.ssh.ssh_key_path);
            // Expand tilde in path
            let expanded_path = if config.ssh.ssh_key_path.starts_with("~/") {
                let home = std::env::var("HOME").unwrap_or_default();
                config.ssh.ssh_key_path.replacen("~", &home, 1)
            } else {
                config.ssh.ssh_key_path.clone()
            };
            println!("   → Expanded path: {}", expanded_path);
            config.ssh.ssh_key_path = expanded_path;
            Some(config.ssh.ssh_key_path.as_str())
        };

        let tunnel =
            match ssh::SshTunnel::connect(&config.ssh.server, &config.ssh.username, key_path).await
            {
                Ok(tunnel) => {
                    println!("   ✓ SSH tunnel established successfully!");
                    println!("     - Local port: {}", tunnel.local_port());
                    println!(
                        "     - Forwarding to: {}:{}\n",
                        config.ssh.server, tunnel.remote_port
                    );
                    Some(tunnel)
                }
                Err(e) => {
                    println!("   ✗ SSH connection failed: {}\n", e);
                    None
                }
            };

        if let Some(tunnel) = tunnel {
            // Test API connection
            println!("3. Testing API connection...");
            let api_client =
                api_client::ApiClient::new(format!("http://localhost:{}", tunnel.local_port()));

            // Give the tunnel a moment to be ready
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            match api_client.list_projects().await {
                Ok(projects) => {
                    println!("   ✓ API connection successful!");
                    println!("     - Projects loaded: {}\n", projects.len());

                    if !projects.is_empty() {
                        println!("Projects:");
                        for (i, project) in projects.iter().enumerate() {
                            println!("  {}. {}", i + 1, project.name);
                            println!("     Path: {}", project.path);
                            if let Some(description) = &project.description {
                                println!("     Description: {}", description);
                            }
                            println!();
                        }
                    } else {
                        println!("No projects found.\n");
                    }
                }
                Err(e) => {
                    println!("   ✗ API connection failed: {}\n", e);
                }
            }

            println!("4. Test complete - All functionality verified!");
        } else {
            println!("Cannot test API without SSH connection.");
        }
    });

    Ok(())
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(nocodo_desktop_app::DesktopApp::new(cc)))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}

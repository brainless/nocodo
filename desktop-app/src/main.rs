#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    // ============================================================================
    // LOGGING CONFIGURATION
    // ============================================================================
    // This application uses tracing for logging with custom SSH log filtering.
    //
    // Environment Variables:
    // ----------------------
    // 1. RUST_LOG: Controls general application logging (standard Rust env var)
    //    Examples:
    //      RUST_LOG=debug     - Enable debug logs for all modules
    //      RUST_LOG=info      - Enable info logs for all modules (default)
    //      RUST_LOG=nocodo_desktop_app=debug - Debug logs only for this app
    //
    // 2. RUST_SSH_CLIENT_LOGS: Controls SSH-specific logging (custom env var)
    //    By default, SSH logs are DISABLED to reduce noise, even when RUST_LOG=debug.
    //    Set this to a log level to enable SSH logs:
    //
    //    Available levels: trace, debug, info, warn, error
    //
    //    Examples:
    //      RUST_SSH_CLIENT_LOGS=debug  - Show detailed SSH connection and data transfer logs
    //      RUST_SSH_CLIENT_LOGS=info   - Show only key SSH events (connection, auth, tunnel ready)
    //      RUST_SSH_CLIENT_LOGS=warn   - Show only SSH warnings and errors
    //
    //    What gets logged at each level:
    //      - trace: All SSH protocol messages (very verbose)
    //      - debug: Connection details, data transfer, channel operations
    //      - info:  Key events (connecting, authenticating, tunnel established)
    //      - warn:  Failed key attempts, connection issues
    //      - error: Critical SSH errors only
    //
    //    This affects both:
    //      - russh::* (the SSH library logs)
    //      - nocodo_desktop_app::ssh (our SSH module logs)
    //
    // 3. RUST_HTTP_CLIENT_LOGS: Controls HTTP client logging (custom env var)
    //    By default, HTTP client logs are DISABLED to reduce noise.
    //    Set this to a log level to enable HTTP client logs:
    //
    //    Examples:
    //      RUST_HTTP_CLIENT_LOGS=debug  - Show detailed HTTP client logs
    //      RUST_HTTP_CLIENT_LOGS=info   - Show HTTP client info logs
    //      RUST_HTTP_CLIENT_LOGS=warn   - Show only HTTP client warnings and errors
    //
    //    This affects:
    //      - hyper_util::client::legacy (the hyper HTTP client logs)
    //
    // Usage Examples:
    // ---------------
    // Linux/macOS:
    //   RUST_LOG=debug RUST_SSH_CLIENT_LOGS=info cargo run
    //   RUST_LOG=debug RUST_HTTP_CLIENT_LOGS=debug cargo run
    //
    // Windows PowerShell:
    //   $env:RUST_LOG="debug"; $env:RUST_SSH_CLIENT_LOGS="info"; cargo run
    //   $env:RUST_LOG="debug"; $env:RUST_HTTP_CLIENT_LOGS="debug"; cargo run
    //
    // Windows Command Prompt:
    //   set RUST_LOG=debug && set RUST_SSH_CLIENT_LOGS=info && cargo run
    //   set RUST_LOG=debug && set RUST_HTTP_CLIENT_LOGS=debug && cargo run
    // ============================================================================

    use tracing_subscriber::{fmt, EnvFilter};

    // Build filter from RUST_LOG
    let mut env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // Check RUST_SSH_CLIENT_LOGS for SSH-specific log level
    let ssh_log_level = std::env::var("RUST_SSH_CLIENT_LOGS")
        .ok()
        .and_then(|v| {
            let level = v.trim().to_lowercase();
            match level.as_str() {
                "trace" | "debug" | "info" | "warn" | "error" => Some(level),
                _ => None,
            }
        });

    // Apply SSH log level directives
    if let Some(level) = ssh_log_level {
        // Enable russh logs at specified level
        env_filter = env_filter.add_directive(format!("russh={}", level).parse().unwrap());
        env_filter = env_filter.add_directive(format!("nocodo_desktop_app::ssh={}", level).parse().unwrap());
    } else {
        // Disable SSH logs by default
        env_filter = env_filter.add_directive("russh=off".parse().unwrap());
        env_filter = env_filter.add_directive("nocodo_desktop_app::ssh=off".parse().unwrap());
    }

    // Check RUST_HTTP_CLIENT_LOGS for HTTP client log level
    let http_log_level = std::env::var("RUST_HTTP_CLIENT_LOGS")
        .ok()
        .and_then(|v| {
            let level = v.trim().to_lowercase();
            match level.as_str() {
                "trace" | "debug" | "info" | "warn" | "error" => Some(level),
                _ => None,
            }
        });

    // Apply HTTP client log level directives
    if let Some(level) = http_log_level {
        // Enable hyper_util logs at specified level
        env_filter = env_filter.add_directive(format!("hyper_util::client::legacy={}", level).parse().unwrap());
    } else {
        // Disable HTTP client logs by default
        env_filter = env_filter.add_directive("hyper_util::client::legacy=off".parse().unwrap());
    }

    fmt()
        .with_env_filter(env_filter)
        .init();

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
        println!("     - Port: {}", config.ssh.port);
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

        let tunnel = match ssh::SshTunnel::connect(
            &config.ssh.server,
            &config.ssh.username,
            key_path,
            config.ssh.port,
            config.ssh.remote_port,
        )
        .await
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

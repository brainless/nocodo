use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::process::{Command, Stdio};
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(name = "manager-runner")]
#[command(about = "Run nocodo manager and web services for testing")]
struct Args {
    /// Clean log files before starting services
    #[arg(short, long)]
    clean: bool,
}

/// Kill any existing manager-runner instances and standalone nocodo-manager processes
fn kill_existing_instances() -> Result<()> {
    info!("Checking for existing manager-runner and nocodo-manager instances...");

    // Get current process ID to avoid killing ourselves
    let current_pid = std::process::id();

    // Find all relevant processes
    let output = Command::new("ps")
        .args(["aux"])
        .output()
        .context("Failed to run ps command")?;

    let output_str = String::from_utf8(output.stdout).context("Failed to parse ps output")?;

    let mut killed_count = 0;

    for line in output_str.lines() {
        let should_kill = (line.contains("manager-runner") || line.contains("nocodo-manager"))
            && !line.contains("grep");

        if should_kill {
            // Extract PID (second column in ps aux output)
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(pid) = parts[1].parse::<u32>() {
                    if pid != current_pid {
                        let process_type = if line.contains("manager-runner") {
                            "manager-runner"
                        } else {
                            "nocodo-manager"
                        };

                        info!("Found existing {} instance with PID: {}", process_type, pid);

                        // Try to kill the process gracefully first (SIGTERM)
                        let kill_result = Command::new("kill")
                            .args(["-TERM", &pid.to_string()])
                            .status();

                        match kill_result {
                            Ok(status) if status.success() => {
                                info!("Successfully terminated {} process {}", process_type, pid);
                                killed_count += 1;

                                // Give it a moment to terminate gracefully
                                std::thread::sleep(std::time::Duration::from_millis(500));

                                // Check if it's still running and force kill if needed
                                let check_result =
                                    Command::new("kill").args(["-0", &pid.to_string()]).status();

                                if check_result.is_ok() {
                                    warn!("Process {} still running, force killing...", pid);
                                    let _ = Command::new("kill")
                                        .args(["-KILL", &pid.to_string()])
                                        .status();
                                }
                            }
                            Ok(_) => {
                                warn!("Failed to terminate {} process {} (may have already been dead)", process_type, pid);
                            }
                            Err(e) => {
                                warn!("Error killing process {}: {}", pid, e);
                            }
                        }
                    }
                }
            }
        }
    }

    if killed_count > 0 {
        info!("Killed {} existing process instance(s)", killed_count);
        // Give processes time to fully clean up
        std::thread::sleep(std::time::Duration::from_millis(1000));
    } else {
        info!("No existing manager-runner or nocodo-manager instances found");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt::init();

    info!("Starting nocodo manager-runner");

    // Kill any existing manager-runner instances before starting
    // Temporarily disabled due to issues with process detection
    // if let Err(e) = kill_existing_instances() {
    //     warn!("Failed to clean up existing instances: {}", e);
    //     // Continue anyway - this is not a fatal error
    // }

    // Create test-logs directory
    fs::create_dir_all("test-logs").context("Failed to create test-logs directory")?;

    // Clean log files if requested
    if args.clean {
        info!("Cleaning existing log files...");
        let _ = fs::remove_file("test-logs/manager.log");
        let _ = fs::remove_file("test-logs/manager-web.log");
        info!("Log files cleaned");
    }

    // Install manager-web dependencies
    info!("Installing manager-web dependencies...");
    let web_install_status = Command::new("npm")
        .args(["install"])
        .current_dir("manager-web")
        .status()
        .context("Failed to run npm install")?;

    if !web_install_status.success() {
        error!("npm install failed");
        return Err(anyhow::anyhow!("npm install failed"));
    }

    info!("manager-web dependencies installed successfully");

    // Note: Manager will be built by cargo-watch in watch mode

    // Start manager with cargo-watch in watch mode
    info!("Starting nocodo-manager with cargo-watch...");

    let manager_log = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("test-logs/manager.log")
        .context("Failed to create manager.log")?;

    let mut manager_process = tokio::process::Command::new("/home/nocodo/.cargo/bin/cargo-watch")
        .args(["-x", "run --bin nocodo-manager -- --config ~/.config/nocodo/manager.toml"])
        .stdout(Stdio::from(manager_log.try_clone()?))
        .stderr(Stdio::from(manager_log))
        .spawn()
        .context("Failed to start manager with cargo-watch")?;

    info!("Manager started with PID: {:?}", manager_process.id());

    // Wait for manager to be ready by checking if port 8081 is listening
    info!("Waiting for manager to be ready on port 8081...");
    let mut attempts = 0;
    let max_attempts = 30; // 30 seconds max
    loop {
        if let Ok(stream) = std::net::TcpStream::connect("127.0.0.1:8081") {
            drop(stream);
            info!("Manager is ready on port 8081");
            break;
        }

        attempts += 1;
        if attempts >= max_attempts {
            error!("Manager failed to start within 30 seconds");
            return Err(anyhow::anyhow!("Manager startup timeout"));
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }

    // Start manager-web dev server with logging
    info!("Starting manager-web dev server...");

    let web_log = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("test-logs/manager-web.log")
        .context("Failed to create manager-web.log")?;

    let mut web_process = tokio::process::Command::new("npm")
        .args(["run", "dev"])
        .current_dir("manager-web")
        .stdout(Stdio::from(web_log.try_clone()?))
        .stderr(Stdio::from(web_log))
        .spawn()
        .context("Failed to start manager-web dev server")?;

    info!(
        "Manager-web dev server started with PID: {:?}",
        web_process.id()
    );
    info!("");
    info!("🚀 Both services are running in watch mode!");
    info!("📊 Manager API/WebSocket: http://localhost:8081 (logs: test-logs/manager.log)");
    info!("   └── Auto-rebuilds on code changes with cargo-watch");
    info!("🌐 Manager-web dev server: http://localhost:3000 (logs: test-logs/manager-web.log)");
    info!("   └── Hot reloads on code changes with Vite");
    info!("");
    info!("🔗 SSH Testing:");
    info!("   SSH with: ssh -L 8081:localhost:8081 -L 3000:localhost:3000 <your-server>");
    info!("   Then access: http://localhost:3000 in your browser");
    info!("");
    info!("Press Ctrl+C to stop both services");

    // Handle shutdown gracefully
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down services...");
        }
        result = manager_process.wait() => {
            match result {
                Ok(status) => warn!("Manager process exited with status: {}", status),
                Err(e) => error!("Manager process error: {}", e),
            }
        }
        result = web_process.wait() => {
            match result {
                Ok(status) => warn!("Web process exited with status: {}", status),
                Err(e) => error!("Web process error: {}", e),
            }
        }
    }

    // Cleanup: kill any remaining processes
    info!("Stopping manager...");
    drop(manager_process.kill());
    drop(manager_process.wait());

    info!("Stopping manager-web...");
    drop(web_process.kill());
    drop(web_process.wait());

    info!("All services stopped");
    Ok(())
}

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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt::init();

    info!("Starting nocodo manager-runner");

    // Create test-logs directory
    fs::create_dir_all("test-logs")
        .context("Failed to create test-logs directory")?;

    // Clean log files if requested
    if args.clean {
        info!("Cleaning existing log files...");
        let _ = fs::remove_file("test-logs/manager.log");
        let _ = fs::remove_file("test-logs/manager-web.log");
        info!("Log files cleaned");
    }

    // Build manager-web first
    info!("Building manager-web...");
    let web_build_status = Command::new("npm")
        .args(&["install"])
        .current_dir("manager-web")
        .status()
        .context("Failed to run npm install")?;

    if !web_build_status.success() {
        error!("npm install failed");
        return Err(anyhow::anyhow!("npm install failed"));
    }

    let web_build_status = Command::new("npm")
        .args(&["run", "build"])
        .current_dir("manager-web")
        .status()
        .context("Failed to build manager-web")?;

    if !web_build_status.success() {
        error!("manager-web build failed");
        return Err(anyhow::anyhow!("manager-web build failed"));
    }

    info!("manager-web built successfully");

    // Build manager binary
    info!("Building manager...");
    let manager_build_status = Command::new("cargo")
        .args(&["build", "--bin", "nocodo-manager"])
        .status()
        .context("Failed to build manager")?;

    if !manager_build_status.success() {
        error!("Manager build failed");
        return Err(anyhow::anyhow!("Manager build failed"));
    }

    info!("Manager built successfully");

    // Start manager with logging
    info!("Starting nocodo-manager...");

    let manager_log = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("test-logs/manager.log")
        .context("Failed to create manager.log")?;

    let mut manager_process = tokio::process::Command::new("./target/debug/nocodo-manager")
        .args(&["--config", "~/.config/nocodo/manager.toml"])
        .stdout(Stdio::from(manager_log.try_clone()?))
        .stderr(Stdio::from(manager_log))
        .spawn()
        .context("Failed to start manager")?;

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
        .args(&["run", "dev"])
        .current_dir("manager-web")
        .stdout(Stdio::from(web_log.try_clone()?))
        .stderr(Stdio::from(web_log))
        .spawn()
        .context("Failed to start manager-web dev server")?;

    info!("Manager-web dev server started with PID: {:?}", web_process.id());
    info!("");
    info!("🚀 Both services are running!");
    info!("📊 Manager API/WebSocket: http://localhost:8081 (logs: test-logs/manager.log)");
    info!("🌐 Manager-web dev server: http://localhost:3000 (logs: test-logs/manager-web.log)");
    info!("   └── Proxies API calls to manager on port 8081");
    info!("");
    info!("🔗 SSH Testing:");
    info!("   SSH with: ssh -L 8081:localhost:8081 -L 3000:localhost:3000 <your-server>");
    info!("   Then access: http://localhost:3000 in your browser");
    info!("   (The dev server will proxy API calls to the manager)");
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
    let _ = manager_process.kill();
    let _ = manager_process.wait();

    info!("Stopping manager-web...");
    let _ = web_process.kill();
    let _ = web_process.wait();

    info!("All services stopped");
    Ok(())
}
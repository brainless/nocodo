use anyhow::{Context, Result};
use std::fs;
use std::process::{Command, Stdio};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting nocodo manager-runner");

    // Create test-logs directory
    fs::create_dir_all("test-logs")
        .context("Failed to create test-logs directory")?;

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
        .args(&["build", "--release", "--bin", "nocodo-manager"])
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

    let mut manager_process = tokio::process::Command::new("./target/release/nocodo-manager")
        .args(&["--config", "~/.config/nocodo/manager.toml"])
        .stdout(Stdio::from(manager_log.try_clone()?))
        .stderr(Stdio::from(manager_log))
        .spawn()
        .context("Failed to start manager")?;

    info!("Manager started with PID: {:?}", manager_process.id());

    // Wait a moment for manager to start
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

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
    info!("📊 Manager: Check test-logs/manager.log for logs");
    info!("🌐 Manager-web: Check test-logs/manager-web.log for logs");
    info!("🔗 Access the web interface at: http://localhost:8081");
    info!("🔑 For SSH port forwarding, use: ssh -L 8081:localhost:8081 <your-server>");
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
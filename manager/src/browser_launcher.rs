use std::time::Duration;
use tokio::time::sleep;

/// Configuration for browser launching
#[derive(Clone)]
pub struct BrowserConfig {
    pub auto_launch: bool,
    pub url: String,
    pub delay_ms: u64,
    pub max_retries: u32,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            auto_launch: true,
            url: "http://localhost:8081".to_string(),
            delay_ms: 2000, // Wait 2 seconds for server to start
            max_retries: 3,
        }
    }
}

/// Launch the default browser with the web interface
pub async fn launch_browser(config: &BrowserConfig) {
    if !config.auto_launch {
        tracing::info!("Browser auto-launch disabled. Open {} manually", config.url);
        return;
    }

    tracing::info!("Waiting {}ms before launching browser...", config.delay_ms);
    sleep(Duration::from_millis(config.delay_ms)).await;

    for attempt in 1..=config.max_retries {
        tracing::info!(
            "Attempting to launch browser (attempt {}/{})",
            attempt,
            config.max_retries
        );

        match launch_browser_impl(&config.url) {
            Ok(_) => {
                tracing::info!("Successfully launched browser for {}", config.url);
                return;
            }
            Err(e) => {
                tracing::warn!("Failed to launch browser (attempt {}): {}", attempt, e);

                if attempt < config.max_retries {
                    sleep(Duration::from_millis(1000)).await;
                }
            }
        }
    }

    tracing::error!(
        "Failed to launch browser after {} attempts. Please open {} manually",
        config.max_retries,
        config.url
    );

    // Print user-friendly message
    println!("\n🌐 nocodo Manager is running!");
    println!("   Open your browser to: {}", config.url);
    println!("   Press Ctrl+C to stop the server\n");
}

/// Cross-platform browser launching implementation
fn launch_browser_impl(url: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()
            .map_err(|e| format!("Failed to launch browser on Windows: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to launch browser on macOS: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        // Try xdg-open first (most reliable)
        let result = std::process::Command::new("xdg-open").arg(url).spawn();

        if result.is_err() {
            // Fallback to common browsers
            let browsers = ["firefox", "google-chrome", "chromium", "brave-browser"];
            let mut success = false;

            for browser in &browsers {
                if std::process::Command::new(browser).arg(url).spawn().is_ok() {
                    success = true;
                    break;
                }
            }

            if !success {
                return Err("No suitable browser found on Linux".into());
            }
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        return Err("Browser launching not supported on this platform".into());
    }

    Ok(())
}

/// Check if the server is responding before launching browser
pub async fn wait_for_server(url: &str, max_attempts: u32) -> bool {
    for attempt in 1..=max_attempts {
        tracing::debug!(
            "Checking if server is ready (attempt {}/{})",
            attempt,
            max_attempts
        );

        match check_server_health(url).await {
            Ok(true) => {
                tracing::info!("Server is ready at {}", url);
                return true;
            }
            Ok(false) => {
                tracing::debug!("Server not ready yet, waiting...");
            }
            Err(e) => {
                tracing::debug!("Server health check failed: {}", e);
            }
        }

        if attempt < max_attempts {
            sleep(Duration::from_millis(500)).await;
        }
    }

    tracing::warn!("Server readiness check timed out, launching browser anyway");
    false
}

/// Simple health check for the server
async fn check_server_health(
    base_url: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let _health_url = format!("{}/api/health", base_url);

    // Use a simple TCP connection test instead of HTTP to avoid additional dependencies
    let uri: Vec<&str> = base_url.split("://").collect();
    if uri.len() != 2 {
        return Err("Invalid URL format".into());
    }

    let host_port: Vec<&str> = uri[1].split(':').collect();
    if host_port.len() != 2 {
        return Err("Invalid host:port format".into());
    }

    let host = host_port[0];
    let port: u16 = host_port[1].parse().map_err(|_| "Invalid port number")?;

    // Try to connect to the server port
    match std::net::TcpStream::connect((host, port)) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Print startup banner with browser launch info
pub fn print_startup_banner(config: &BrowserConfig) {
    println!("\n🚀 nocodo Manager starting...");

    if config.auto_launch {
        println!("   🌐 Browser will auto-launch: {}", config.url);
    } else {
        println!("   🌐 Open manually: {}", config.url);
    }

    println!("   📊 Web interface with embedded assets");
    println!("   🔧 API server with WebSocket support");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_config_default() {
        let config = BrowserConfig::default();
        assert!(config.auto_launch);
        assert_eq!(config.url, "http://localhost:8081");
        assert_eq!(config.delay_ms, 2000);
        assert_eq!(config.max_retries, 3);
    }

    #[tokio::test]
    async fn test_server_health_check_invalid_url() {
        let result = check_server_health("invalid-url").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_server_health_check_unreachable() {
        // Test with a port that should not be in use
        let result = check_server_health("http://localhost:65432").await;
        assert!(matches!(result, Ok(false)));
    }
}

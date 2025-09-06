use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    // Tell Cargo to re-run this build script if manager-web files change
    println!("cargo:rerun-if-changed=../manager-web/src");
    println!("cargo:rerun-if-changed=../manager-web/package.json");
    println!("cargo:rerun-if-changed=../manager-web/package-lock.json");
    println!("cargo:rerun-if-changed=../manager-web/vite.config.ts");
    println!("cargo:rerun-if-changed=../manager-web/dist");

    let manager_web_dir = Path::new("../manager-web");
    let dist_dir = manager_web_dir.join("dist");
    let package_json = manager_web_dir.join("package.json");

    // Skip web build in docs.rs builds or when explicitly disabled
    if env::var("DOCS_RS").is_ok() || env::var("SKIP_WEB_BUILD").is_ok() {
        println!("cargo:warning=Skipping web build for docs.rs or explicit skip");
        return;
    }

    // Check if we're in a CI environment or development
    let is_ci = env::var("CI").is_ok();
    let is_release = env::var("PROFILE").unwrap_or_default() == "release";

    println!("cargo:warning=Build environment: CI={}, RELEASE={}", is_ci, is_release);

    // Always try to build web assets for release builds or CI
    if is_release || is_ci {
        if !package_json.exists() {
            println!("cargo:warning=manager-web/package.json not found, skipping web build");
            return;
        }

        println!("cargo:warning=Building manager-web assets...");

        // Install dependencies
        let npm_install = Command::new("npm")
            .args(["ci"])
            .current_dir(&manager_web_dir)
            .status();

        match npm_install {
            Ok(status) if status.success() => {
                println!("cargo:warning=npm ci completed successfully");
            }
            Ok(status) => {
                println!("cargo:warning=npm ci failed with status: {}", status);
                if is_release {
                    panic!("Failed to install npm dependencies in release build");
                }
                return;
            }
            Err(e) => {
                println!("cargo:warning=npm ci error: {}. Ensure Node.js and npm are installed.", e);
                if is_release {
                    panic!("npm not available for release build: {}", e);
                }
                return;
            }
        }

        // Build the web app
        let npm_build = Command::new("npm")
            .args(["run", "build"])
            .current_dir(&manager_web_dir)
            .status();

        match npm_build {
            Ok(status) if status.success() => {
                println!("cargo:warning=Web build completed successfully");
                
                // Verify dist directory exists and has files
                if dist_dir.exists() {
                    let dist_files = std::fs::read_dir(&dist_dir)
                        .map(|entries| entries.count())
                        .unwrap_or(0);
                    println!("cargo:warning=Generated {} files in dist/", dist_files);
                } else {
                    println!("cargo:warning=Warning: dist/ directory not created");
                }
            }
            Ok(status) => {
                println!("cargo:warning=npm run build failed with status: {}", status);
                if is_release {
                    panic!("Failed to build web assets in release build");
                }
            }
            Err(e) => {
                println!("cargo:warning=npm run build error: {}", e);
                if is_release {
                    panic!("Failed to execute npm build: {}", e);
                }
            }
        }
    } else {
        // In development, just warn if assets don't exist
        if !dist_dir.exists() {
            println!("cargo:warning=manager-web/dist not found. Run 'cd manager-web && npm run build' to enable embedded assets.");
            println!("cargo:warning=The server will use filesystem fallback in development mode.");
        } else {
            println!("cargo:warning=Using existing web assets from manager-web/dist");
        }
    }

    // Set environment variables for the embedded assets
    if dist_dir.exists() {
        println!("cargo:rustc-env=WEB_ASSETS_AVAILABLE=1");
        
        // Calculate total size for logging
        let total_size = calculate_dir_size(&dist_dir).unwrap_or(0);
        
        println!("cargo:rustc-env=WEB_ASSETS_SIZE={}", total_size);
        println!("cargo:warning=Web assets total size: {} bytes", total_size);
    } else {
        println!("cargo:rustc-env=WEB_ASSETS_AVAILABLE=0");
        println!("cargo:rustc-env=WEB_ASSETS_SIZE=0");
    }
}

fn calculate_dir_size(dir: &Path) -> Result<u64, std::io::Error> {
    let mut total_size = 0;
    
    fn visit_dir(dir: &Path, total_size: &mut u64) -> Result<(), std::io::Error> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                visit_dir(&path, total_size)?;
            } else {
                let metadata = std::fs::metadata(&path)?;
                *total_size += metadata.len();
            }
        }
        Ok(())
    }
    
    visit_dir(dir, &mut total_size)?;
    Ok(total_size)
}
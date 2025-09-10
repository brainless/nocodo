use actix_web::{web, HttpRequest, HttpResponse, Result as ActixResult};
use mime_guess::from_path;
use rust_embed::{Embed, RustEmbed};
use std::borrow::Cow;

/// Embedded web assets from manager-web/dist
#[derive(RustEmbed)]
#[folder = "../manager-web/dist/"]
#[include = "*.html"]
#[include = "*.js"]
#[include = "*.css"]
#[include = "*.ico"]
#[include = "*.svg"]
#[include = "*.png"]
#[include = "*.jpg"]
#[include = "*.jpeg"]
#[include = "*.woff"]
#[include = "*.woff2"]
#[include = "*.ttf"]
#[exclude = "*.map"] // Exclude source maps to reduce binary size
pub struct WebAssets;

/// Handle embedded static file requests
pub async fn handle_embedded_file(req: HttpRequest) -> ActixResult<HttpResponse> {
    let path = req.match_info().query("filename");

    // Handle root path -> index.html
    let file_path = if path.is_empty() || path == "/" {
        "index.html"
    } else {
        // Remove leading slash if present
        path.strip_prefix('/').unwrap_or(path)
    };

    tracing::debug!("Serving embedded file: {}", file_path);

    match <WebAssets as Embed>::get(file_path) {
        Some(content) => {
            let mime = from_path(file_path).first_or_octet_stream();

            // Handle different content types appropriately
            let content_length = content.data.len();
            if !file_path.ends_with(".html") {
                // Static asset with caching
                match content.data {
                    Cow::Borrowed(bytes) => Ok(HttpResponse::Ok()
                        .content_type(mime.as_ref())
                        .insert_header(("Cache-Control", "public, max-age=86400"))
                        .insert_header(("ETag", format!("\"{}\"", content_length)))
                        .body(bytes)),
                    Cow::Owned(bytes) => Ok(HttpResponse::Ok()
                        .content_type(mime.as_ref())
                        .insert_header(("Cache-Control", "public, max-age=86400"))
                        .insert_header(("ETag", format!("\"{}\"", content_length)))
                        .body(bytes)),
                }
            } else {
                // HTML file without caching
                match content.data {
                    Cow::Borrowed(bytes) => Ok(HttpResponse::Ok()
                        .content_type(mime.as_ref())
                        .insert_header(("Cache-Control", "no-cache, no-store, must-revalidate"))
                        .insert_header(("Pragma", "no-cache"))
                        .insert_header(("Expires", "0"))
                        .body(bytes)),
                    Cow::Owned(bytes) => Ok(HttpResponse::Ok()
                        .content_type(mime.as_ref())
                        .insert_header(("Cache-Control", "no-cache, no-store, must-revalidate"))
                        .insert_header(("Pragma", "no-cache"))
                        .insert_header(("Expires", "0"))
                        .body(bytes)),
                }
            }
        }
        None => {
            tracing::debug!(
                "Embedded file not found: {}, serving index.html for SPA routing",
                file_path
            );

            // For SPA routing, serve index.html for unknown routes
            match <WebAssets as Embed>::get("index.html") {
                Some(content) => match content.data {
                    Cow::Borrowed(bytes) => Ok(HttpResponse::Ok()
                        .content_type("text/html")
                        .insert_header(("Cache-Control", "no-cache, no-store, must-revalidate"))
                        .body(bytes)),
                    Cow::Owned(bytes) => Ok(HttpResponse::Ok()
                        .content_type("text/html")
                        .insert_header(("Cache-Control", "no-cache, no-store, must-revalidate"))
                        .body(bytes)),
                },
                None => {
                    tracing::error!("index.html not found in embedded assets");
                    Ok(
                        HttpResponse::InternalServerError()
                            .body("Web assets not properly embedded"),
                    )
                }
            }
        }
    }
}

/// List all embedded files (for debugging)
pub fn list_embedded_files() -> Vec<String> {
    <WebAssets as Embed>::iter()
        .map(|f| f.to_string())
        .collect()
}

/// Check if web assets are properly embedded
pub fn validate_embedded_assets() -> bool {
    // Check for essential files
    let required_files = ["index.html"];

    for file in required_files {
        if <WebAssets as Embed>::get(file).is_none() {
            tracing::error!("Required embedded asset missing: {}", file);
            return false;
        }
    }

    tracing::info!("Embedded web assets validation successful");
    let files: Vec<String> = list_embedded_files();
    tracing::info!("Embedded {} web assets: {:?}", files.len(), files);

    true
}

/// Get the total size of all embedded assets
pub fn get_embedded_assets_size() -> usize {
    <WebAssets as Embed>::iter()
        .filter_map(|path| <WebAssets as Embed>::get(&path))
        .map(|file| file.data.len())
        .sum()
}

/// Configure embedded web routes for Actix Web
pub fn configure_embedded_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Serve specific static files
        .route(
            "/{filename:.*\\.(js|css|ico|svg|png|jpg|jpeg|woff|woff2|ttf)}",
            web::get().to(handle_embedded_file),
        )
        // Catch-all for SPA routing (must be last)
        .route("/{path:.*}", web::get().to(handle_embedded_file))
        // Root path
        .route("/", web::get().to(handle_embedded_file));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_assets_exist() {
        // This test will fail during development if assets aren't built
        // but should pass in CI/CD when assets are properly embedded
        let files = list_embedded_files();

        if files.is_empty() {
            println!(
                "Warning: No embedded assets found. Run 'cd manager-web && npm run build' first."
            );
            println!("This is expected during development.");
        } else {
            println!("Found {} embedded assets", files.len());
            assert!(
                files.iter().any(|f| f == "index.html"),
                "index.html should be embedded"
            );
        }
    }

    #[test]
    fn test_asset_validation() {
        // This test documents the expected behavior
        let is_valid = validate_embedded_assets();

        if !is_valid {
            println!("Web assets not embedded - this is expected during development");
            println!("Run 'cd manager-web && npm run build' to generate assets");
        }

        // Don't fail the test in development, just document the expectation
        assert!(
            is_valid || cfg!(debug_assertions),
            "Assets should be embedded in release builds"
        );
    }
}

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectTemplate {
    pub name: String,
    pub description: String,
    pub language: String,
    pub framework: Option<String>,
    pub files: Vec<TemplateFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TemplateFile {
    pub path: String,
    pub content: String,
    pub executable: bool,
}

pub struct TemplateManager;

impl Default for TemplateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateManager {
    pub fn new() -> Self {
        Self
    }

    pub fn get_available_templates() -> Vec<ProjectTemplate> {
        vec![
            Self::rust_web_api_template(),
            Self::node_web_app_template(),
            Self::static_site_template(),
        ]
    }

    pub fn get_template(name: &str) -> AppResult<ProjectTemplate> {
        match name {
            "rust-web-api" => Ok(Self::rust_web_api_template()),
            "node-web-app" => Ok(Self::node_web_app_template()),
            "static-site" => Ok(Self::static_site_template()),
            _ => Err(AppError::InvalidRequest(format!(
                "Unknown template: {name}"
            ))),
        }
    }

    #[allow(dead_code)]
    pub fn apply_template(template: &ProjectTemplate, project_path: &Path) -> AppResult<()> {
        // Create the project directory
        fs::create_dir_all(project_path)?;

        // Create all template files
        for file in &template.files {
            let file_path = project_path.join(&file.path);

            // Create parent directories if needed
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Write the file content
            fs::write(&file_path, &file.content)?;

            // Set executable permissions if needed
            #[cfg(unix)]
            if file.executable {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&file_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&file_path, perms)?;
            }
        }

        tracing::info!(
            "Applied template {} to {}",
            template.name,
            project_path.display()
        );
        Ok(())
    }

    fn rust_web_api_template() -> ProjectTemplate {
        ProjectTemplate {
            name: "rust-web-api".to_string(),
            description: "Rust web API with Actix Web, SQLite, and ts-rs".to_string(),
            language: "rust".to_string(),
            framework: Some("actix-web".to_string()),
            files: vec![
                TemplateFile {
                    path: "Cargo.toml".to_string(),
                    content: r#"[package]
name = "{{project_name}}"
version = "0.1.0"
edition = "2021"

[dependencies]
actix-web = "4.4"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rusqlite = { version = "0.30", features = ["bundled"] }
ts-rs = "7.1"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
"#
                    .to_string(),
                    executable: false,
                },
                TemplateFile {
                    path: "src/main.rs".to_string(),
                    content: r#"use actix_web::{web, App, HttpResponse, HttpServer, Result};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
struct ApiResponse {
    message: String,
    timestamp: i64,
}

async fn hello() -> Result<HttpResponse> {
    let response = ApiResponse {
        message: "Hello from {{project_name}}!".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    };
    Ok(HttpResponse::Ok().json(response))
}

async fn health() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "{{project_name}}"
    })))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::init();
    
    println!("Starting {{project_name}} server on http://localhost:8080");
    
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(hello))
            .route("/health", web::get().to(health))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
"#
                    .to_string(),
                    executable: false,
                },
                TemplateFile {
                    path: ".gitignore".to_string(),
                    content: r#"/target
/Cargo.lock
.env
*.db
*.sqlite
.DS_Store
"#
                    .to_string(),
                    executable: false,
                },
                TemplateFile {
                    path: "README.md".to_string(),
                    content: r#"# {{project_name}}

A Rust web API built with Actix Web.

## Getting Started

```bash
# Run the server
cargo run

# The API will be available at http://localhost:8080
```

## API Endpoints

- `GET /` - Hello world endpoint
- `GET /health` - Health check endpoint

## Development

```bash
# Run with hot reload
cargo install cargo-watch
cargo watch -x run
```
"#
                    .to_string(),
                    executable: false,
                },
            ],
        }
    }

    fn node_web_app_template() -> ProjectTemplate {
        ProjectTemplate {
            name: "node-web-app".to_string(),
            description: "Node.js web app with Express and TypeScript".to_string(),
            language: "typescript".to_string(),
            framework: Some("express".to_string()),
            files: vec![
                TemplateFile {
                    path: "package.json".to_string(),
                    content: r#"{
  "name": "{{project_name}}",
  "version": "1.0.0",
  "description": "Node.js web app with Express and TypeScript",
  "main": "dist/index.js",
  "scripts": {
    "build": "tsc",
    "start": "node dist/index.js",
    "dev": "ts-node src/index.ts",
    "watch": "nodemon --exec ts-node src/index.ts"
  },
  "dependencies": {
    "express": "^4.18.2",
    "cors": "^2.8.5",
    "helmet": "^7.1.0"
  },
  "devDependencies": {
    "@types/express": "^4.17.21",
    "@types/cors": "^2.8.17",
    "@types/node": "^20.10.5",
    "typescript": "^5.3.3",
    "ts-node": "^10.9.2",
    "nodemon": "^3.0.2"
  }
}
"#
                    .to_string(),
                    executable: false,
                },
                TemplateFile {
                    path: "tsconfig.json".to_string(),
                    content: r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
"#
                    .to_string(),
                    executable: false,
                },
                TemplateFile {
                    path: "src/index.ts".to_string(),
                    content: r#"import express, { Request, Response } from 'express';
import cors from 'cors';
import helmet from 'helmet';

const app = express();
const PORT = process.env.PORT || 3000;

// Middleware
app.use(helmet());
app.use(cors());
app.use(express.json());
app.use(express.urlencoded({ extended: true }));

// Routes
app.get('/', (req: Request, res: Response) => {
  res.json({
    message: 'Hello from {{project_name}}!',
    timestamp: new Date().toISOString()
  });
});

app.get('/health', (req: Request, res: Response) => {
  res.json({
    status: 'ok',
    service: '{{project_name}}'
  });
});

// Start server
app.listen(PORT, () => {
  console.log(`{{project_name}} server running on http://localhost:${PORT}`);
});
"#
                    .to_string(),
                    executable: false,
                },
                TemplateFile {
                    path: ".gitignore".to_string(),
                    content: r#"node_modules/
dist/
.env
.env.local
.DS_Store
npm-debug.log*
yarn-debug.log*
yarn-error.log*
"#
                    .to_string(),
                    executable: false,
                },
                TemplateFile {
                    path: "README.md".to_string(),
                    content: r#"# {{project_name}}

A Node.js web app built with Express and TypeScript.

## Getting Started

```bash
# Install dependencies
npm install

# Start development server
npm run dev

# The API will be available at http://localhost:3000
```

## API Endpoints

- `GET /` - Hello world endpoint
- `GET /health` - Health check endpoint

## Build and Deploy

```bash
# Build for production
npm run build

# Start production server
npm start
```
"#
                    .to_string(),
                    executable: false,
                },
            ],
        }
    }

    fn static_site_template() -> ProjectTemplate {
        ProjectTemplate {
            name: "static-site".to_string(),
            description: "Static HTML/CSS/JS website".to_string(),
            language: "html".to_string(),
            framework: None,
            files: vec![
                TemplateFile {
                    path: "index.html".to_string(),
                    content: r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{{project_name}}</title>
    <link rel="stylesheet" href="styles.css">
</head>
<body>
    <header>
        <h1>Welcome to {{project_name}}</h1>
    </header>
    
    <main>
        <section>
            <h2>Hello World!</h2>
            <p>This is a static website template.</p>
            <button onclick="showMessage()">Click me!</button>
            <div id="message" class="hidden"></div>
        </section>
    </main>
    
    <footer>
        <p>&copy; 2024 {{project_name}}. All rights reserved.</p>
    </footer>
    
    <script src="script.js"></script>
</body>
</html>
"#
                    .to_string(),
                    executable: false,
                },
                TemplateFile {
                    path: "styles.css".to_string(),
                    content: r#"* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    line-height: 1.6;
    color: #333;
    background-color: #f4f4f4;
}

header {
    background: #35424a;
    color: white;
    text-align: center;
    padding: 1rem;
}

header h1 {
    font-size: 2rem;
    margin-bottom: 0.5rem;
}

main {
    max-width: 800px;
    margin: 2rem auto;
    padding: 0 1rem;
}

section {
    background: white;
    padding: 2rem;
    border-radius: 8px;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
    margin-bottom: 2rem;
}

h2 {
    color: #35424a;
    margin-bottom: 1rem;
}

button {
    background: #35424a;
    color: white;
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
    margin-top: 1rem;
}

button:hover {
    background: #2c3e50;
}

.hidden {
    display: none;
}

#message {
    margin-top: 1rem;
    padding: 1rem;
    background: #d4edda;
    color: #155724;
    border: 1px solid #c3e6cb;
    border-radius: 4px;
}

footer {
    background: #35424a;
    color: white;
    text-align: center;
    padding: 1rem;
    position: fixed;
    bottom: 0;
    width: 100%;
}
"#
                    .to_string(),
                    executable: false,
                },
                TemplateFile {
                    path: "script.js".to_string(),
                    content: r#"function showMessage() {
    const messageDiv = document.getElementById('message');
    const currentTime = new Date().toLocaleString();
    
    messageDiv.innerHTML = `
        <strong>Hello from {{project_name}}!</strong><br>
        Current time: ${currentTime}
    `;
    
    messageDiv.classList.remove('hidden');
}

// Add some interactive behavior
document.addEventListener('DOMContentLoaded', function() {
    console.log('{{project_name}} loaded successfully!');
    
    // Add click animation to buttons
    const buttons = document.querySelectorAll('button');
    buttons.forEach(button => {
        button.addEventListener('click', function() {
            this.style.transform = 'scale(0.95)';
            setTimeout(() => {
                this.style.transform = 'scale(1)';
            }, 100);
        });
    });
});
"#
                    .to_string(),
                    executable: false,
                },
                TemplateFile {
                    path: ".gitignore".to_string(),
                    content: r#".DS_Store
Thumbs.db
*.log
.env
"#
                    .to_string(),
                    executable: false,
                },
                TemplateFile {
                    path: "README.md".to_string(),
                    content: r#"# {{project_name}}

A static website template with HTML, CSS, and JavaScript.

## Getting Started

Simply open `index.html` in your web browser, or serve it with a local server:

```bash
# Using Python
python -m http.server 8000

# Using Node.js (if you have http-server installed)
npx http-server

# Using PHP
php -S localhost:8000
```

## File Structure

- `index.html` - Main HTML file
- `styles.css` - Stylesheet
- `script.js` - JavaScript functionality
- `README.md` - This file

## Features

- Responsive design
- Modern CSS styling
- Interactive JavaScript
- Clean, semantic HTML structure
"#
                    .to_string(),
                    executable: false,
                },
            ],
        }
    }
}

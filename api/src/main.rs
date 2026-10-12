//! `nocodo-api`: starts the HTTP server over the engine.

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;
use nocodo_api::{AppState, serve};
use nocodo_engine::schema_proposer;

#[derive(Parser)]
#[command(name = "nocodo-api", version)]
struct Args {
    /// Address to listen on. Localhost only by default.
    #[arg(long, default_value = "127.0.0.1:8787", env = "NOCODO_API_ADDR")]
    addr: SocketAddr,
    /// Folder whose subfolders are the projects.
    #[arg(long, default_value = ".projects", env = "NOCODO_PROJECTS_DIR")]
    projects_dir: PathBuf,
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    // Provider API keys live in `.env`; a missing file is fine.
    let _ = dotenvy::dotenv();
    let args = Args::parse();
    let listener = match tokio::net::TcpListener::bind(args.addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!(
                "Could not listen on {}: {e}. Is another nocodo-api running? Pick another address with --addr.",
                args.addr
            );
            return std::process::ExitCode::FAILURE;
        }
    };
    println!(
        "nocodo-api listening on http://{} (projects in {})",
        listener.local_addr().unwrap_or(args.addr),
        args.projects_dir.display()
    );
    let state = AppState::new(args.projects_dir, schema_proposer);
    if let Err(e) = serve(listener, state).await {
        eprintln!("The server stopped: {e}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

#[path = "../config.rs"]
mod config;
#[path = "../db.rs"]
mod db;

fn main() -> std::io::Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .ok()
        .or_else(|| config::read_project_conf("DATABASE_URL"))
        .unwrap_or_else(|| "app.db".to_string());
    db::run_startup_migrations(&database_url)?;
    println!("Migrations applied successfully");
    Ok(())
}

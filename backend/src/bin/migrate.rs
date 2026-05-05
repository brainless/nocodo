#[path = "../config.rs"]
mod config;
#[path = "../db.rs"]
mod db;

fn main() -> std::io::Result<()> {
    let config = config::Config::load()
        .map_err(std::io::Error::other)?;
    db::run_startup_migrations(&config.database.url)?;
    println!("Migrations applied successfully");
    Ok(())
}

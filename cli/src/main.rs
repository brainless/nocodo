use clap::Parser;
use tracing::{info, debug};
use anyhow::Result;

mod cli;
mod commands;
mod error;
mod logging;

use cli::Cli;
use error::CliError;
use logging::init_logging;

#[tokio::main]
async fn main() -> Result<(), CliError> {
    let cli = Cli::parse();
    
    init_logging(cli.verbose)?;
    
    info!("nocodo CLI starting");
    debug!("CLI arguments: {:?}", cli);
    
    match cli.run().await {
        Ok(_) => {
            info!("nocodo CLI completed successfully");
            Ok(())
        }
        Err(e) => {
            tracing::error!("CLI error: {:?}", e);
            std::process::exit(e.exit_code());
        }
    }
}

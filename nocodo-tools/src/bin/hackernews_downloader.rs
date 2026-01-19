use clap::{Parser, ValueEnum};
use nocodo_tools::hackernews::execute_hackernews_request;
use nocodo_tools::types::{FetchMode, HackerNewsRequest, StoryType};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "hackernews_downloader")]
#[command(about = "Download HackerNews items for testing", long_about = None)]
#[command(version)]
struct Cli {
    #[arg(value_enum)]
    fetch_mode: FetchModeArg,

    #[arg(short, long)]
    db_path: Option<String>,

    #[arg(short, long, default_value = "20")]
    batch_size: usize,

    #[arg(short, long, default_value = "5")]
    max_depth: usize,

    #[arg(short, long)]
    verbose: bool,

    #[arg(short, long)]
    quiet: bool,
}

#[derive(Clone, ValueEnum)]
enum FetchModeArg {
    Top,
    New,
    Best,
    Ask,
    Show,
    Job,
    All,
    FetchAll,
}

fn setup_logging(verbose: bool, quiet: bool) {
    let level = if quiet {
        "warn"
    } else if verbose {
        "debug"
    } else {
        "info"
    };

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .init();
}

fn get_default_db_path() -> String {
    if let Some(home) = home::home_dir() {
        home.join(".local/share/nocodo/hackernews.db")
            .to_string_lossy()
            .to_string()
    } else {
        PathBuf::from("hackernews.db").to_string_lossy().to_string()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    setup_logging(cli.verbose, cli.quiet);

    tracing::info!("Starting HackerNews downloader");

    let fetch_mode = match cli.fetch_mode {
        FetchModeArg::Top => FetchMode::StoryType {
            story_type: StoryType::Top,
        },
        FetchModeArg::New => FetchMode::StoryType {
            story_type: StoryType::New,
        },
        FetchModeArg::Best => FetchMode::StoryType {
            story_type: StoryType::Best,
        },
        FetchModeArg::Ask => FetchMode::StoryType {
            story_type: StoryType::Ask,
        },
        FetchModeArg::Show => FetchMode::StoryType {
            story_type: StoryType::Show,
        },
        FetchModeArg::Job => FetchMode::StoryType {
            story_type: StoryType::Job,
        },
        FetchModeArg::All => FetchMode::StoryType {
            story_type: StoryType::All,
        },
        FetchModeArg::FetchAll => FetchMode::FetchAll,
    };

    let db_path = cli.db_path.unwrap_or_else(get_default_db_path);

    let request = HackerNewsRequest {
        db_path: db_path.clone(),
        fetch_mode,
        batch_size: Some(cli.batch_size),
        max_depth: Some(cli.max_depth),
    };

    tracing::info!(
        "Fetch mode: {:?}, Batch size: {}, Max depth: {}",
        request.fetch_mode,
        cli.batch_size,
        cli.max_depth
    );
    tracing::info!("Database path: {}", db_path);

    let start = std::time::Instant::now();
    let response = execute_hackernews_request(request).await?;
    let duration = start.elapsed();

    if let nocodo_tools::ToolResponse::HackerNewsResponse(hn_response) = response {
        tracing::info!("\n{}", "=".repeat(60));
        tracing::info!("Download Complete!");
        tracing::info!("{}", "=".repeat(60));
        tracing::info!("Message: {}", hn_response.message);
        tracing::info!("Items downloaded: {}", hn_response.items_downloaded);
        tracing::info!("Items skipped: {}", hn_response.items_skipped);
        tracing::info!("Items failed: {}", hn_response.items_failed);
        tracing::info!("Users downloaded: {}", hn_response.users_downloaded);
        tracing::info!("Users failed: {}", hn_response.users_failed);
        tracing::info!("Total processed: {}", hn_response.items_processed);
        tracing::info!("Duration: {:.2}s", duration.as_secs_f64());
        tracing::info!("Has more: {}", hn_response.has_more);
        if hn_response.items_processed > 0 {
            let speed = hn_response.items_processed as f64 / duration.as_secs_f64();
            tracing::info!("Speed: {:.2} items/sec", speed);
        }
        tracing::info!("{}", "=".repeat(60));
    }

    Ok(())
}

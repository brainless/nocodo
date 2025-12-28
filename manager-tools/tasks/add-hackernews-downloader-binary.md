# Add HackerNews Downloader CLI Binary

**Status**: ✅ Completed
**Priority**: Medium
**Created**: 2025-12-28

## Summary

Create a standalone CLI binary `hackernews_downloader` for manually testing the HackerNews download manager tool. This binary will provide a simple command-line interface to download HackerNews items with real-time logging.

## Problem Statement

Currently, there's no easy way to manually test the HackerNews download manager tool without integrating it into the full manager system. Developers need a simple CLI tool to:
- Test different fetch modes (story types, fetch all)
- Verify download functionality works correctly
- Debug issues with real-time logging
- Validate database storage
- Test performance and error handling

## Goals

1. **Simple CLI interface**: Single binary that takes fetch mode as argument
2. **Real-time logging**: Show download progress with detailed logs
3. **All fetch modes**: Support all story types (top, new, best, ask, show, job, all) and fetch-all mode
4. **Configurable options**: Allow customizing batch size, max depth, and database path
5. **Progress tracking**: Display stats on items/users downloaded, skipped, and failed
6. **Error handling**: Gracefully handle and display errors

## CLI Interface Design

### Basic Command Structure

```bash
hackernews_downloader <FETCH_MODE> [OPTIONS]
```

### Fetch Modes (Positional Argument)

- `top` - Download top stories (500 items)
- `new` - Download new stories (500 items)
- `best` - Download best stories (500 items)
- `ask` - Download Ask HN stories (200 items)
- `show` - Download Show HN stories (200 items)
- `job` - Download job stories (200 items)
- `all` - Download all story types above
- `fetch-all` - Fetch all items starting from max ID walking backward

### Options (Flags)

```
OPTIONS:
    -d, --db-path <PATH>         Database path [default: ~/.local/share/nocodo/hackernews.db]
    -b, --batch-size <SIZE>      Number of items to fetch in parallel [default: 20]
    -m, --max-depth <DEPTH>      Maximum recursion depth for comments [default: 5]
    -v, --verbose                Enable verbose logging (DEBUG level)
    -q, --quiet                  Minimal output (WARN level only)
    -h, --help                   Print help
    -V, --version                Print version
```

### Example Usage

```bash
# Download top stories with default settings
hackernews_downloader top

# Download new stories with verbose logging
hackernews_downloader new --verbose

# Fetch all items with custom batch size and database
hackernews_downloader fetch-all --batch-size 50 --db-path /tmp/hn.db

# Download all story types with max comment depth of 3
hackernews_downloader all --max-depth 3

# Quiet mode - only show warnings and errors
hackernews_downloader best --quiet
```

## Implementation Plan

### Phase 1: Project Structure

Create new binary in `manager-tools`:

```
manager-tools/
  Cargo.toml                    # Add [[bin]] section
  src/
    bin/
      hackernews_downloader.rs  # Main binary
```

Update `Cargo.toml`:
```toml
[[bin]]
name = "hackernews_downloader"
path = "src/bin/hackernews_downloader.rs"
```

Add dependencies (if not already present):
```toml
[dependencies]
clap = { version = "4.4", features = ["derive"] }
tracing = { workspace = true }
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
```

### Phase 2: CLI Argument Parsing

Use `clap` with derive macros to parse arguments:

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "hackernews_downloader")]
#[command(about = "Download HackerNews items for testing", long_about = None)]
#[command(version)]
struct Cli {
    /// Fetch mode: top, new, best, ask, show, job, all, or fetch-all
    #[arg(value_enum)]
    fetch_mode: FetchModeArg,

    /// Database path
    #[arg(short, long)]
    db_path: Option<String>,

    /// Batch size for parallel downloads
    #[arg(short, long, default_value = "20")]
    batch_size: usize,

    /// Maximum recursion depth for comment fetching
    #[arg(short, long, default_value = "5")]
    max_depth: usize,

    /// Enable verbose logging (DEBUG level)
    #[arg(short, long)]
    verbose: bool,

    /// Minimal output (WARN level only)
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
```

### Phase 3: Logging Setup

Configure `tracing` based on verbosity flags:

```rust
fn setup_logging(verbose: bool, quiet: bool) {
    let level = if quiet {
        "warn"
    } else if verbose {
        "debug"
    } else {
        "info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level))
        )
        .with_target(false)
        .with_thread_ids(false)
        .init();
}
```

### Phase 4: Main Execution Logic

1. Parse CLI arguments
2. Setup logging
3. Convert `FetchModeArg` to `FetchMode`
4. Create `HackerNewsRequest`
5. Execute request using `execute_hackernews_request()`
6. Display results and stats

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    setup_logging(cli.verbose, cli.quiet);

    tracing::info!("Starting HackerNews downloader");

    // Convert CLI fetch mode to tool fetch mode
    let fetch_mode = match cli.fetch_mode {
        FetchModeArg::Top => FetchMode::StoryType { story_type: StoryType::Top },
        FetchModeArg::New => FetchMode::StoryType { story_type: StoryType::New },
        // ... etc
        FetchModeArg::FetchAll => FetchMode::FetchAll,
    };

    // Create request
    let request = HackerNewsRequest {
        db_path: cli.db_path.unwrap_or_default(),
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

    // Execute download
    let start = std::time::Instant::now();
    let response = execute_hackernews_request(request).await?;
    let duration = start.elapsed();

    // Display results
    if let ToolResponse::HackerNewsResponse(hn_response) = response {
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
        tracing::info!("{}", "=".repeat(60));
    }

    Ok(())
}
```

### Phase 5: Additional Features

**Progress Logging:**
Add logging within the fetcher to show real-time progress:
- Log when starting a batch
- Log each item/user as it's downloaded
- Log errors as they occur
- Show percentage complete for story type modes

**Continuous Mode (Optional):**
For `fetch-all` mode, optionally support continuous downloading:
```bash
hackernews_downloader fetch-all --continuous
```
This would keep downloading batches until interrupted.

**Stats Summary:**
Display detailed statistics at the end:
- Average download speed (items/sec)
- Success rate (downloaded vs failed)
- Database size
- Unique users found

## Testing Strategy

### Manual Testing Scenarios

1. **Basic Functionality**
   - Download top stories
   - Download each story type
   - Download all story types
   - Fetch all mode

2. **Error Handling**
   - Invalid database path
   - Network errors (disconnect during download)
   - Invalid batch size (0, negative)
   - Database permission errors

3. **Performance Testing**
   - Different batch sizes (1, 10, 20, 50, 100)
   - Large downloads (fetch-all mode)
   - Monitor memory usage
   - Check for connection leaks

4. **Logging Levels**
   - Normal mode (INFO)
   - Verbose mode (DEBUG)
   - Quiet mode (WARN)
   - Verify log output is helpful

5. **Database Verification**
   - Check database is created at correct path
   - Verify schema is initialized
   - Query database to confirm data integrity
   - Test resuming downloads (items already in DB are skipped)

### Example Test Session

```bash
# Clean start - download top 10 stories
rm -f ~/.local/share/nocodo/hackernews.db
hackernews_downloader top --verbose

# Resume - should skip already downloaded items
hackernews_downloader top --verbose

# Different story type
hackernews_downloader new --batch-size 30

# Check database
sqlite3 ~/.local/share/nocodo/hackernews.db "SELECT COUNT(*) FROM items;"
sqlite3 ~/.local/share/nocodo/hackernews.db "SELECT COUNT(*) FROM users;"

# Fetch all mode with custom settings
hackernews_downloader fetch-all --batch-size 50 --max-depth 3
```

## Files Changed

### New Files
- `manager-tools/src/bin/hackernews_downloader.rs`
- `manager-tools/tasks/add-hackernews-downloader-binary.md` (this file)

### Modified Files
- `manager-tools/Cargo.toml` - Add [[bin]] section and clap dependency if needed

## Success Criteria

- [ ] Binary compiles and runs successfully
- [ ] All fetch modes work correctly (top, new, best, ask, show, job, all, fetch-all)
- [ ] Command-line arguments parsed correctly
- [ ] Logging shows useful progress information
- [ ] Verbose mode shows detailed debug logs
- [ ] Quiet mode only shows warnings/errors
- [ ] Database created at correct default path
- [ ] Custom database path works
- [ ] Batch size and max depth options work
- [ ] Results summary displays accurate statistics
- [ ] Resuming downloads skips already-fetched items
- [ ] No clippy warnings
- [ ] Help text is clear and accurate

## Dependencies

This task depends on:
- ✅ Completed: `add-hackernews-download-manager.md` - The core HackerNews tool must be implemented

## Notes

- This is a testing/debugging tool, not intended for production use
- Keep the implementation simple and focused on testing
- Logging should be informative enough to debug issues
- Consider adding a `--dry-run` flag to preview what would be downloaded without actually fetching
- Could add a `--stats` flag to show database statistics without downloading
- Future enhancement: Add `--resume` flag to continue from where `fetch-all` mode left off

## References

- **Clap Documentation**: https://docs.rs/clap/
- **Tracing Documentation**: https://docs.rs/tracing/
- **HackerNews Tool**: manager-tools/src/hackernews/mod.rs
- **HackerNews Types**: manager-tools/src/types/hackernews.rs

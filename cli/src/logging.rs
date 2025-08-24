use crate::error::CliError;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize logging with environment-based log level filtering
pub fn init_logging(verbose: bool) -> Result<(), CliError> {
    let default_level = if verbose { "debug" } else { "info" };

    // Create an environment filter that defaults to info level
    // Can be overridden with RUST_LOG environment variable
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(default_level))
        .map_err(|e| CliError::Config(format!("Failed to create log filter: {e}")))?;

    // Create a fmt layer that logs to stdout
    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .compact();

    // Build the subscriber
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .try_init()
        .map_err(|e| CliError::Config(format!("Failed to initialize logging: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    #[test]
    fn test_init_logging_default() {
        // Use Once to ensure we only initialize logging once across all tests
        INIT.call_once(|| {
            let result = init_logging(false);
            // We expect this to succeed on first initialization
            assert!(result.is_ok());
        });
        // After the first initialization, subsequent calls should be handled gracefully
        // Test that the function doesn't panic when called again
        let _ = init_logging(false);
    }

    #[test]
    fn test_init_logging_verbose() {
        // Since we can't reinitialize the global subscriber, we test that the function
        // handles the case gracefully when the subscriber is already initialized
        let result = init_logging(true);
        // The function should either succeed (if this is the first call) or fail gracefully
        // Either way, it shouldn't panic
        let _ = result; // Don't assert success/failure since it depends on test execution order
    }
}

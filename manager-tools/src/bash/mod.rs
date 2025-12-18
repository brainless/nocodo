pub mod bash_executor;
pub mod bash_permissions;
pub mod types;

// Re-export public types and functions
pub use bash_executor::BashExecutor;
pub use bash_permissions::*;
pub use types::*;

// Test modules (only for cfg(test))
#[cfg(test)]
pub mod bash_executor_tests;
#[cfg(test)]
pub mod bash_permissions_tests;

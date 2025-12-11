pub mod tool_error;
pub mod tool_executor;
pub mod list_files;
pub mod read_file;
pub mod write_file;
pub mod grep;
pub mod apply_patch;
pub mod bash;
pub mod bash_executor;
pub mod bash_permissions;

pub use tool_error::ToolError;
pub use bash::{BashExecutionResult, BashExecutorTrait};
pub use tool_executor::ToolExecutor;

#[cfg(test)]
mod tests;
// TODO: Fix and re-enable these tests after refactoring
// #[cfg(test)]
// mod bash_executor_tests;
// #[cfg(test)]
// mod bash_permissions_tests;
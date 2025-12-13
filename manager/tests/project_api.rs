//! Project API integration tests
//!
//! This test file imports and runs tests from the integration/project_api module.
//! Run with: cargo test --test project_api

mod common;

// Include the integration test module
mod integration {
    pub mod project_api;
}

// Re-export tests so they can be discovered by cargo test
pub use integration::*;

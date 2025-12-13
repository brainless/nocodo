//! Authentication & Authorization System integration tests
//!
//! This test file imports and runs tests from the integration/auth_system module.
//! Run with: cargo test --test auth_system

mod common;

// Include the integration test module
mod integration {
    pub mod auth_system;
}

// Re-export tests so they can be discovered by cargo test
pub use integration::*;
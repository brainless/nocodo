//! Integration test runner for Permission System Phase 6 & 7
//!
//! This test file includes both API integration tests and performance tests
//! for the team management and permission system.

mod common;

// Include the integration test modules
mod integration {
    pub mod permission_performance;
    pub mod permission_system_api;
}

// Re-export tests so they can be discovered by cargo test
pub use integration::*;

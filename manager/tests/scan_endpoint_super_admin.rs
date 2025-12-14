//! Scan endpoint Super Admin integration tests
//!
//! This test file imports and runs tests from the integration/scan_endpoint_super_admin module.
//! Run with: cargo test --test scan_endpoint_super_admin

mod common;

// Include the integration test module
mod integration {
    pub mod scan_endpoint_super_admin;
}

// Re-export tests so they can be discovered by cargo test
pub use integration::*;

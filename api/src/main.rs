//! Thin Axum layer over orchestrator, agents and runtime.
//!
//! Must not contain business logic.

use nocodo_agent as _;
use nocodo_agent_pm as _;
use nocodo_agent_schema as _;
use nocodo_checker as _;
use nocodo_orchestrator as _;
use nocodo_runtime as _;
use nocodo_sqlgen as _;
use nocodo_types as _;

fn main() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}

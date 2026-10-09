//! Owns the event log (`events.jsonl`), subscriptions, single-writer artifact ownership and scheduling, exposed as a trait-based API.
//!
//! Must not use an LLM. Routing is plain code.

use nocodo_types as _;

//! Owns the shared agent base: Rig wiring, per-role model config, `.agent-logs` JSONL logging, reading documents from allowed folders, and emitting `Question`s.
//!
//! Must not contain any role-specific prompt or logic.

use nocodo_types as _;

//! Owns deterministic validation of specs and generated SQL (dry-run on a copy of the DB).
//!
//! Must not let an LLM decide pass or fail.

use nocodo_sqlgen as _;
use nocodo_types as _;

//! Turns requirement facts into typed `SchemaChange` lists. Owns `specs/schema.json`.
//!
//! Must not write SQL, touch `data.sqlite`, or write any other spec.

use nocodo_agent as _;
use nocodo_types as _;

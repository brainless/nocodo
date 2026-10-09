//! User interface; spawns `nocodo-api` as a subprocess and talks HTTP to it.
//!
//! Must not call agents or the DB directly.

use nocodo_types as _;

fn main() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}

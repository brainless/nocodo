//! Test-only helper: compile a `record_spec_fact` serializer's generated Rust
//! source with a real `rustc` invocation, linked against the workspace's
//! already-built `nocodo_praxis` rlib.
//!
//! String-content assertions (Phase 3's persona tests) catch typos in field
//! names but not shape mismatches — e.g. a field that's actually
//! `AtLeastOne<Provenance>` serialized as a bare `&[...]` slice literal.
//! Phase 3 found exactly this mismatch by reading `nocodo_praxis` source, not
//! from a test. This helper closes that gap: it shells out to `rustc
//! --crate-type lib` against the real crate so a shape mismatch is a failing
//! test, not a manual-review catch.
//!
//! Requires `cargo build -p nocodo_praxis` (or a full workspace build) to
//! have produced `target/debug/deps/libnocodo_praxis-*.rlib` — true for any
//! normal `cargo test -p nocodo-agents` run, since `nocodo_praxis` is a
//! dependency of this crate and Cargo builds dependencies before tests.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Locate the most recently built `libnocodo_praxis-*.rlib` in
/// `target/{debug,release}/deps`, searching from `CARGO_MANIFEST_DIR`
/// (`agents/`) upward to find the workspace `target/` directory.
fn find_praxis_rlib() -> (PathBuf, PathBuf) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("agents/ crate has a parent directory (workspace root)");

    for profile in ["debug", "release"] {
        let deps_dir = workspace_root.join("target").join(profile).join("deps");
        if !deps_dir.is_dir() {
            continue;
        }
        let newest = std::fs::read_dir(&deps_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                let name = name.to_string_lossy();
                name.starts_with("libnocodo_praxis-") && name.ends_with(".rlib")
            })
            .max_by_key(|e| {
                e.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            });
        if let Some(entry) = newest {
            return (entry.path(), deps_dir);
        }
    }

    panic!(
        "could not find a built nocodo_praxis rlib under {}/target/{{debug,release}}/deps — \
         run `cargo build -p nocodo_praxis` (or `cargo test -p nocodo-agents`, which builds it \
         as a dependency) before running this test",
        workspace_root.display()
    );
}

/// Compile `code` (a complete, standalone `.rs` file produced by a
/// `record_spec_fact` serializer) with `rustc --crate-type lib`, linked
/// against the real `nocodo_praxis` rlib. Panics with the compiler's stderr
/// if compilation fails.
pub(crate) fn assert_compiles_against_nocodo_praxis(code: &str) {
    let (rlib_path, deps_dir) = find_praxis_rlib();

    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let scratch = std::env::temp_dir().join(format!(
        "nocodo-agents-spec-fact-compile-check-{}-{}",
        std::process::id(),
        n
    ));
    std::fs::create_dir_all(&scratch).expect("create compile-check scratch dir");
    let src_path = scratch.join("generated.rs");
    let out_path = scratch.join("generated.rlib");
    std::fs::write(&src_path, code).expect("write generated source to scratch dir");

    let output = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("--crate-type")
        .arg("lib")
        .arg("--extern")
        .arg(format!("nocodo_praxis={}", rlib_path.display()))
        .arg("-L")
        .arg(format!("dependency={}", deps_dir.display()))
        .arg("-o")
        .arg(&out_path)
        .arg(&src_path)
        .output()
        .expect("failed to spawn rustc — is it on PATH?");

    assert!(
        output.status.success(),
        "generated code failed to compile against nocodo_praxis:\n{}\n\n--- generated code ---\n{code}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_dir_all(&scratch);
}

# Epic 001: Workspace & Crate Layout

**Status:** Done
**Goal:** Create the empty Cargo workspace, toolchain config and crate skeletons for Experiment 1, with the dependency direction from `AGENTS.md` wired as path dependencies. **No behavior, no external dependencies.**

**Prerequisite:** None.

---

## Context

`PRD.md` §9 lays out Experiment 1 as small epics, each ending in a checkpoint. This epic only gives later epics somewhere to put code. Keep it minimal: no types, no logic, no third-party crates. Epic 002 adds `serde`; later epics add Rig, Diesel, Axum and Tokio only when they need them.

## Tasks

### Task 1: Toolchain

- `rust-toolchain.toml`:
  ```toml
  [toolchain]
  channel = "nightly"
  components = ["rustc-codegen-cranelift", "rustfmt", "clippy"]
  ```
- `.cargo/config.toml`:
  ```toml
  [unstable]
  codegen-backend = true

  [profile.dev]
  codegen-backend = "cranelift"
  ```
- Don't set `rustc-wrapper` here. The user's global `~/.cargo/config.toml` already configures sccache.

### Task 2: Workspace `Cargo.toml`

- `resolver = "3"`, `edition = "2024"` via `[workspace.package]`, plus `license` and `version = "0.1.0"`.
- `members` lists every crate from Task 3.
- Leave `[workspace.dependencies]` empty apart from internal path crates. Define each internal crate there once (e.g. `nocodo-types = { path = "crates/nocodo-types" }`) so member crates use `workspace = true`.
- Profiles:
  ```toml
  [profile.dev]
  debug = 0
  incremental = true

  [profile.dev.package."*"]
  debug = 0
  opt-level = 0
  ```
- Add `[workspace.lints]` (`rust.unsafe_code = "forbid"`, `clippy.all = "warn"`), and have each member set `[lints] workspace = true`.

### Task 3: Crate skeletons

Create each crate with a `Cargo.toml` and a `src/lib.rs` (or `src/main.rs`). Each `lib.rs` contains only a crate-level doc comment stating what the crate owns, copied from the responsibility table in `AGENTS.md`.

| Path | Package | Kind | Internal deps |
|---|---|---|---|
| `crates/nocodo-types` | `nocodo-types` | lib | — |
| `crates/nocodo-sqlgen` | `nocodo-sqlgen` | lib | types |
| `crates/nocodo-checker` | `nocodo-checker` | lib | types, sqlgen |
| `crates/nocodo-orchestrator` | `nocodo-orchestrator` | lib | types |
| `crates/nocodo-runtime` | `nocodo-runtime` | lib | types |
| `crates/nocodo-agent` | `nocodo-agent` | lib | types |
| `crates/nocodo-agent-schema` | `nocodo-agent-schema` | lib | types, agent |
| `crates/nocodo-agent-pm` | `nocodo-agent-pm` | lib | types, agent |
| `api` | `nocodo-api` | bin `nocodo-api` | all library crates above |
| `cli` | `nocodo-cli` | bin `nocodo` | types |

The two binaries' `main` only prints the package name and version (`env!("CARGO_PKG_VERSION")`), with no argument parsing yet. To keep clippy free of unused-dependency noise, a crate may `use other_crate as _;` for each internal dependency.

### Task 4: Repo hygiene

- `.gitignore`: `/target`, `.agent-logs/`, `*.sqlite`, `*.sqlite-*`, `/projects/` (local test projects), `.DS_Store`.
- Add a short `README.md`: one paragraph on what nocodo is, with pointers to `PRD.md`, `AGENTS.md` and `epics/`.

### Task 5: Verify

Run and confirm all of these pass:

```bash
cargo build --workspace            # dev profile → Cranelift
cargo build --workspace --release  # LLVM
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo run --bin nocodo
cargo run --bin nocodo-api
```

Confirm Cranelift is actually in use. One way: `cargo build -v` and look for `-Zcodegen-backend=cranelift` in the rustc invocations.

## Out of scope

- Any type, trait or function beyond the doc comments.
- Any third-party dependency.
- CI configuration.
- Epic 002 content.

## Checkpoint

- [x] All Task 5 commands pass.
- [x] `cargo tree -p nocodo-cli` shows only `nocodo-types` as an internal dependency.
- [x] No crate depends on a crate that `AGENTS.md`'s dependency direction forbids.
- [x] Task Notes added below, and `Status: Done`.

## Task Notes

**Built:** Nightly + Cranelift toolchain config, the workspace `Cargo.toml` (resolver 3, edition 2024, shared lints, profiles), 8 library crates and 2 binaries (`nocodo-api`, `nocodo`) holding only doc comments and a name/version `main`, plus `.gitignore` and `README.md`. Internal dependencies are path deps defined once in `[workspace.dependencies]`. There are no third-party dependencies.

**Tested:** Every Task 5 command exits 0: dev build, release build, `cargo test --workspace` (0 tests), clippy with `-D warnings`, `fmt --check`, and `cargo run` of both binaries (they print `nocodo-cli 0.1.0` and `nocodo-api 0.1.0`). `cargo build -v` shows `-Z codegen-backend=cranelift` on dev rustc invocations (via the sccache wrapper), and none in release. `cargo tree -p nocodo-cli` shows only `nocodo-types`. Each crate's internal deps match the Task 3 table and `AGENTS.md`'s dependency direction.

**Surprising:** Linking the binaries prints a warning in both dev and release builds: `stripping debug info with rust-objcopy failed: signal 6 ... Library not loaded: @rpath/libLLVM.dylib`. It is not caused by Cranelift. The `rust-objcopy` in the toolchain's `rustlib/<target>/bin/` looks for `libLLVM.dylib` in `rustlib/<target>/lib/`, but the toolchain ships it only in `<toolchain>/lib/`. rustc calls it to strip debuginfo on macOS. The build still succeeds and the binaries run, so I left it alone. A fix, if wanted, is a symlink from `rustlib/<target>/lib/libLLVM.dylib` to `<toolchain>/lib/libLLVM.dylib`, or `strip = "none"` in the profile. Either one is a local or config change outside this epic.

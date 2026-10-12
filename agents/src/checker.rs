//! `cargo check` wrapper — the Phase 3.5 static checker (MILESTONE_PRAXIS_v1.md
//! §7.3 point 1, §8.3). Deliberately dumb: run `cargo check` on the managed
//! project's backend crate (where `nocodo_praxis` specs live) and hand the raw
//! stderr back to PO. PO — not this module — interprets compiler errors into
//! user-facing questions. This is replaced by a dedicated, structured
//! `static_checker` module in Phase 5; do not add rustc-output parsing here.

use std::path::Path;
use std::process::Command;

use crate::error::AgentError;

/// Result of a `cargo check` run against the managed project's backend crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoCheckOutcome {
    /// `true` when `cargo check` exited successfully (no compiler errors).
    pub success: bool,
    /// Raw `stderr` from the compiler — rustc emits diagnostics there. Passed
    /// to PO verbatim as gap context; never parsed here (see module docs).
    pub output: String,
}

/// Run `cargo check` against `{project_root}/backend/Cargo.toml` — the
/// managed project's backend crate, which is where `record_spec_fact` writes
/// `nocodo_praxis` spec code (`backend/src/praxis/specs/`).
///
/// **Deviation from MILESTONE_PRAXIS_v1.md §7.3's `-p <spec crate>` wording:**
/// there is no single well-known package name for a managed project's backend
/// crate (each generated project names it however `init-project.sh` set it
/// up), so we check the whole crate via `--manifest-path` instead of `-p
/// <name>`. This checks a superset of the spec crate (transitions, etc. once
/// those exist) which is strictly more coverage, not less.
///
/// If no `backend/Cargo.toml` exists yet (e.g. a brand-new managed project
/// before any codegen has run), returns a non-success outcome with an
/// explanatory message instead of invoking `cargo` — there's nothing to check.
pub fn run_cargo_check(project_root: &Path) -> Result<CargoCheckOutcome, AgentError> {
    let manifest_path = project_root.join("backend").join("Cargo.toml");
    if !manifest_path.exists() {
        return Ok(CargoCheckOutcome {
            success: false,
            output: format!(
                "no Cargo.toml found at {} — the managed project's backend crate doesn't exist yet",
                manifest_path.display()
            ),
        });
    }

    let output = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(&manifest_path)
        .output()
        .map_err(|e| AgentError::Other(format!("failed to spawn `cargo check`: {e}")))?;

    Ok(CargoCheckOutcome {
        success: output.status.success(),
        output: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_cargo_check_reports_missing_manifest_without_invoking_cargo() {
        let dir = std::env::temp_dir().join(format!(
            "nocodo-agents-checker-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let outcome = run_cargo_check(&dir).expect("should not error");
        assert!(!outcome.success);
        assert!(outcome.output.contains("no Cargo.toml found"));
    }
}

//! End to end through the real binary: no LLM, only hand-written change sets.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn run(project: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nocodo-api"))
        .current_dir(root())
        .arg("--project")
        .arg(project)
        .args(args)
        .output()
        .unwrap()
}

fn ok(project: &str, args: &[&str]) -> String {
    let o = run(project, args);
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    assert!(
        o.status.success(),
        "{args:?} failed\nstdout: {out}\nstderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    out
}

/// The "Tables (rows)" lines of the reviewer part of `inspect`.
fn table_rows(inspect: &str) -> Vec<String> {
    inspect
        .lines()
        .skip_while(|l| !l.starts_with("Tables (rows)"))
        .skip(1)
        .take_while(|l| l.starts_with("  "))
        .map(str::to_string)
        .collect()
}

#[test]
fn init_changes_sample_rows_follow_ups_and_inspect() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("e2e-finance");
    let _ = fs::remove_dir_all(&dir);
    let p = dir.to_str().unwrap();
    let cs = |n: &str| {
        root()
            .join("fixtures/change-sets/finance")
            .join(n)
            .to_string_lossy()
            .to_string()
    };

    ok(p, &["init", "--base", "project-base.json"]);
    let out = ok(p, &["changes", &cs("01-initial.json")]);
    assert!(out.contains("Applied") || out.contains("0001"), "{out}");
    assert!(dir.join("data.sqlite").exists());

    let out = ok(p, &["sample-rows"]);
    assert!(out.contains("Added 12 sample rows"), "{out}");
    let before = table_rows(&ok(p, &["inspect"]));
    assert_eq!(before.len(), 4, "{before:?}");
    assert!(before.iter().all(|l| l.ends_with(": 3")), "{before:?}");

    let out = ok(p, &["changes", &cs("02-family-member.json")]);
    assert!(
        out.contains("backups/before_0002_add_family_member.sqlite"),
        "{out}"
    );
    let out = ok(p, &["changes", &cs("03-rebuild.json")]);
    assert!(
        out.contains("backups/before_0003_clarify_dates_and_transfers.sqlite"),
        "{out}"
    );

    let inspect = ok(p, &["inspect"]);
    assert_eq!(table_rows(&inspect), before, "{inspect}");
    assert!(inspect.contains("Backups: 2"), "{inspect}");
    assert!(
        inspect.contains("Pending migrations: none")
            || inspect.contains("Pending migrations: (none)"),
        "{inspect}"
    );
    assert!(inspect.contains("== For the user =="));

    // Nothing left to apply.
    let out = ok(p, &["apply"]);
    assert!(out.contains("Nothing to apply"), "{out}");

    let events = fs::read_to_string(dir.join("events.jsonl")).unwrap();
    for kind in ["migration_applied", "database_checked", "sample_rows_added"] {
        assert!(events.contains(kind), "no {kind} event");
    }
    assert!(!events.contains("migration_failed"));
    let _ = fs::remove_dir_all(&dir);
}

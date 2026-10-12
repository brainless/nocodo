//! End to end through the real binary: no LLM, only hand-written change sets.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn run(project: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nocodo"))
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

fn has_bytes(path: &std::path::Path, needle: &str) -> bool {
    let bytes = fs::read(path).unwrap();
    bytes.windows(needle.len()).any(|w| w == needle.as_bytes())
}

#[test]
fn removal_asks_then_answer_applies_it() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("e2e-finance-drop");
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
    ok(p, &["changes", &cs("01-initial.json")]);
    ok(p, &["sample-rows"]);
    let before = table_rows(&ok(p, &["inspect"]));

    // The removal is asked, not applied, and not rejected.
    let out = ok(p, &["changes", &cs("04-drop-notes.json")]);
    assert!(out.contains("1 question(s) waiting"), "{out}");
    assert!(out.contains("needs an answer"), "{out}");
    assert!(out.contains("1. Yes, forget it"), "{out}");
    assert!(!out.contains("(default)"), "{out}");
    assert!(has_bytes(&dir.join("data.sqlite"), "notes"));
    let out = ok(p, &["questions"]);
    assert!(out.contains("q1:"), "{out}");

    // A new prompt or change set is refused while the question waits.
    let o = run(p, &["changes", &cs("04-drop-notes.json")]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("waiting"));
    let o = run(p, &["prompt", "keep track of pets"]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("nocodo answer"));

    // Bad arguments are refused naming the argument; --defaults skips blocking questions.
    let o = run(p, &["answer", "q1:1"]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("q1:1"));
    let o = run(p, &["answer", "q1=0"]);
    assert!(!o.status.success());
    let out = ok(p, &["answer", "--defaults"]);
    assert!(out.contains("Still waiting for 1"), "{out}");
    assert!(ok(p, &["questions"]).contains("q1:"));

    let out = ok(p, &["answer", "q1=1"]);
    assert!(out.contains("Accepted"), "{out}");
    assert!(ok(p, &["questions"]).contains("No questions waiting."));
    assert!(ok(p, &["continue"]).contains("Nothing to continue."));

    let inspect = ok(p, &["inspect"]);
    assert_eq!(table_rows(&inspect), before, "{inspect}");
    assert!(!has_bytes(&dir.join("data.sqlite"), "notes"));
    let backups: Vec<_> = fs::read_dir(dir.join("backups"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|b| b.file_name().unwrap().to_string_lossy().contains("0002"))
        .collect();
    assert_eq!(backups.len(), 1, "{backups:?}");
    assert!(has_bytes(&backups[0], "notes"));
    let _ = fs::remove_dir_all(&dir);
}

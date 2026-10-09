//! Dry run over the real finance fixtures, in order, ending with the
//! destructive one.

use std::fs;
use std::path::PathBuf;

use nocodo_checker::{NamedMigration, check_change_set, dry_run};
use nocodo_sqlgen::{METADATA_TOML, apply, generate};
use nocodo_types::change::ChangeSet;
use nocodo_types::schema::{SPEC_VERSION, SchemaSpec};

fn fixture(name: &str) -> ChangeSet {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/change-sets/finance")
        .join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn dry_run_passes_for_finance_fixtures() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("migrations");
    fs::create_dir_all(&dir).unwrap();
    let mut spec = SchemaSpec {
        spec_version: SPEC_VERSION,
        tables: vec![],
    };
    for (i, f) in [
        "01-initial.json",
        "02-family-member.json",
        "03-rebuild.json",
        "04-drop-notes.json",
    ]
    .iter()
    .enumerate()
    {
        let c = fixture(f);
        let findings = check_change_set(&spec, &c, true);
        assert!(findings.is_empty(), "{f}: static {findings:?}");
        let after = apply(&spec, &c.changes).unwrap();
        let m = generate(&spec, &c).unwrap();
        let named = NamedMigration {
            name: format!("{:04}_{}", i + 1, c.name),
            up: m.up,
            down: m.down,
        };
        let findings = dry_run(None, &dir, &named, &spec, &after).unwrap();
        assert!(findings.is_empty(), "{f}: dry run {findings:?}");
        let d = dir.join(&named.name);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("up.sql"), &named.up).unwrap();
        fs::write(d.join("down.sql"), &named.down).unwrap();
        fs::write(d.join("metadata.toml"), METADATA_TOML).unwrap();
        spec = after;
    }
}

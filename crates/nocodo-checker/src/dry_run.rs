//! Dry run: replay migrations on a throwaway copy of the DB.

use std::fs;
use std::io;
use std::path::Path;

use diesel::prelude::*;
use diesel::sql_types::{BigInt, Text};
use diesel_migrations::{FileBasedMigrations, MigrationHarness};
use nocodo_sqlgen::METADATA_TOML;
use nocodo_types::change::SchemaChange;
use nocodo_types::event::{CheckStep, Finding};
use nocodo_types::schema::SchemaSpec;
use std::collections::BTreeMap;

use crate::introspect::{Shape, diff, diff_shapes, expected_shape, introspect};

/// A migration that has not been written to the project yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedMigration {
    /// Folder name, e.g. `0002_add_family_member`.
    pub name: String,
    pub up: String,
    pub down: String,
    /// Tables the change set renames, as `(from, to)` in order. Used to
    /// compare row counts under the new name. See [`table_renames`].
    pub table_renames: Vec<(String, String)>,
}

/// The `(from, to)` table renames in a list of changes, in order.
pub fn table_renames(changes: &[SchemaChange]) -> Vec<(String, String)> {
    changes
        .iter()
        .filter_map(|c| match c {
            SchemaChange::RenameTable { from, to } => Some((from.clone(), to.clone())),
            _ => None,
        })
        .collect()
}

/// The dry run could not be carried out (not a finding about the migration).
#[derive(Debug)]
pub enum CheckerError {
    Io(io::Error),
}

impl std::fmt::Display for CheckerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "dry run could not prepare its temporary files: {e}"),
        }
    }
}

impl std::error::Error for CheckerError {}

impl From<io::Error> for CheckerError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

fn finding(code: &str, message: String) -> Vec<Finding> {
    vec![Finding {
        code: code.to_string(),
        message,
    }]
}

fn sql_error(step: &str, e: impl std::fmt::Display) -> Vec<Finding> {
    finding("sql_error", format!("SQLite error while {step}: {e}"))
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    if !from.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn mismatch(code: &str, what: &str, lines: Vec<String>) -> Vec<Finding> {
    finding(code, format!("{what}: {}", lines.join("; ")))
}

#[derive(QueryableByName)]
struct FkViolation {
    #[diesel(sql_type = Text)]
    table: String,
}

/// What a dry run found and which steps it ran.
///
/// `findings` is empty when the migration passed. `steps` lists every step
/// run, in order; on failure the last one is the step that failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DryRunReport {
    pub findings: Vec<Finding>,
    pub steps: Vec<CheckStep>,
}

/// Step for the static rules, from their findings (an empty list passed).
pub fn static_rules_step(findings: &[Finding]) -> CheckStep {
    if findings.is_empty() {
        CheckStep {
            check: "static_rules".into(),
            passed: true,
            detail: "The change set follows the naming and structure rules.".into(),
        }
    } else {
        CheckStep {
            check: "static_rules".into(),
            passed: false,
            detail: format!(
                "The change set broke {} naming or structure rule(s): {}.",
                findings.len(),
                findings
                    .iter()
                    .map(|f| f.code.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

struct Run {
    steps: Vec<CheckStep>,
}

impl Run {
    fn pass(&mut self, check: &str, detail: impl Into<String>) {
        self.steps.push(CheckStep {
            check: check.into(),
            passed: true,
            detail: detail.into(),
        });
    }

    /// Record a failed step and end the run.
    fn fail(self, check: &str, findings: Vec<Finding>) -> DryRunReport {
        let mut steps = self.steps;
        let detail = findings
            .iter()
            .map(|f| f.message.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        steps.push(CheckStep {
            check: check.into(),
            passed: false,
            detail,
        });
        DryRunReport { findings, steps }
    }

    fn done(self) -> DryRunReport {
        DryRunReport {
            findings: Vec::new(),
            steps: self.steps,
        }
    }
}

fn count_migrations(dir: &Path) -> usize {
    fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .count()
        })
        .unwrap_or(0)
}

fn plural(n: usize) -> String {
    if n == 1 {
        "1 earlier migration".into()
    } else {
        format!("{n} earlier migrations")
    }
}

#[derive(QueryableByName)]
struct TableName {
    #[diesel(sql_type = Text)]
    name: String,
}

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = BigInt)]
    n: i64,
}

/// Row count of every user table (not `sqlite_*`, not the Diesel bookkeeping table).
fn row_counts(conn: &mut SqliteConnection) -> QueryResult<BTreeMap<String, i64>> {
    let names = diesel::sql_query(
        "SELECT name FROM sqlite_master WHERE type = 'table' \
         AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' AND name != '__diesel_schema_migrations' \
         ORDER BY name",
    )
    .load::<TableName>(conn)?;
    let mut out = BTreeMap::new();
    for t in names {
        let sql = format!(
            "SELECT COUNT(*) AS n FROM \"{}\"",
            t.name.replace('"', "\"\"")
        );
        let c = diesel::sql_query(sql).get_result::<Count>(conn)?;
        out.insert(t.name, c.n);
    }
    Ok(out)
}

fn rows_word(n: i64) -> String {
    if n == 1 {
        "1 row".into()
    } else {
        format!("{n} rows")
    }
}

/// Tables in both schemas must hold the same number of rows after `up`.
/// Returns the findings message lines (empty when all rows were kept) and the
/// number of tables compared.
fn rows_lost(
    before: &BTreeMap<String, i64>,
    after: &BTreeMap<String, i64>,
    new_spec: &SchemaSpec,
    renames: &[(String, String)],
) -> (Vec<String>, usize) {
    // Current name -> name before the change set (follows chained renames).
    let mut origin: BTreeMap<String, String> = BTreeMap::new();
    for t in before.keys() {
        origin.insert(t.clone(), t.clone());
    }
    for (from, to) in renames {
        if let Some(o) = origin.remove(from) {
            origin.insert(to.clone(), o);
        }
    }
    let mut lost = Vec::new();
    let mut compared = 0;
    for t in &new_spec.tables {
        let (Some(orig), Some(now)) = (origin.get(&t.name), after.get(&t.name)) else {
            continue;
        };
        let Some(was) = before.get(orig) else {
            continue;
        };
        compared += 1;
        if was != now {
            lost.push(format!(
                "{} had {} before and has {} after",
                t.name,
                rows_word(*was),
                rows_word(*now)
            ));
        }
    }
    (lost, compared)
}

/// Run all earlier migrations plus `new` on a copy of `db`.
///
/// `before` is the spec the DB should match before `new`; `after` the spec it
/// should match afterwards. An empty `findings` means the migration passed.
/// The real `db` and `migrations_dir` are never modified.
pub fn dry_run(
    db: Option<&Path>,
    migrations_dir: &Path,
    new: &NamedMigration,
    before: &SchemaSpec,
    after: &SchemaSpec,
) -> Result<DryRunReport, CheckerError> {
    let mut run = Run { steps: Vec::new() };
    let tmp = tempfile::tempdir()?;
    let db_path = tmp.path().join("data.sqlite");
    let has_db = match db {
        Some(p) if p.exists() => {
            fs::copy(p, &db_path)?;
            true
        }
        _ => false,
    };
    let mig_dir = tmp.path().join("migrations");
    copy_dir(migrations_dir, &mig_dir)?;
    let earlier_count = count_migrations(&mig_dir);

    let mut conn = match SqliteConnection::establish(&db_path.to_string_lossy()) {
        Ok(c) => c,
        Err(e) => {
            return Ok(run.fail(
                "earlier_migrations_applied",
                sql_error("opening the scratch database", e),
            ));
        }
    };

    // 2. Earlier migrations, stopping before the new one.
    let earlier = match FileBasedMigrations::from_path(&mig_dir) {
        Ok(m) => m,
        Err(e) => {
            return Ok(run.fail(
                "earlier_migrations_applied",
                sql_error("reading earlier migrations", e),
            ));
        }
    };
    let pending = if has_db {
        match conn.pending_migrations(earlier.clone()) {
            Ok(p) => p.len(),
            Err(e) => {
                return Ok(run.fail(
                    "earlier_migrations_applied",
                    sql_error("reading the recorded migrations", e),
                ));
            }
        }
    } else {
        earlier_count
    };
    if let Err(e) = conn.run_pending_migrations(earlier) {
        return Ok(run.fail(
            "earlier_migrations_applied",
            sql_error("running earlier migrations", e),
        ));
    }
    let detail = if has_db {
        let counts = match row_counts(&mut conn) {
            Ok(c) => c,
            Err(e) => {
                return Ok(run.fail(
                    "earlier_migrations_applied",
                    sql_error("counting rows in the copy", e),
                ));
            }
        };
        let total: i64 = counts.values().sum();
        let already = earlier_count.saturating_sub(pending);
        format!(
            "On a copy of data.sqlite ({} in {}): {} already applied; ran {} pending.",
            rows_word(total),
            if counts.len() == 1 {
                "1 list".to_string()
            } else {
                format!("{} lists", counts.len())
            },
            plural(already),
            pending
        )
    } else {
        format!(
            "Ran {} on an empty database (no data.sqlite yet).",
            plural(earlier_count)
        )
    };
    run.pass("earlier_migrations_applied", detail);

    // 3. The DB must match the spec before the new migration.
    let start = match introspect(&mut conn) {
        Ok(s) => s,
        Err(e) => {
            return Ok(run.fail(
                "db_matches_spec_before",
                sql_error("reading the database structure", e),
            ));
        }
    };
    let problems = diff(&expected_shape(before), &start);
    if !problems.is_empty() {
        return Ok(run.fail(
            "db_matches_spec_before",
            mismatch(
                "db_does_not_match_spec",
                "The database after earlier migrations does not match the current schema",
                problems,
            ),
        ));
    }
    run.pass(
        "db_matches_spec_before",
        "The database structure matched the current schema before the new migration.",
    );

    // 4. Add and run the new migration.
    let new_dir = mig_dir.join(&new.name);
    fs::create_dir_all(&new_dir)?;
    fs::write(new_dir.join("up.sql"), &new.up)?;
    fs::write(new_dir.join("down.sql"), &new.down)?;
    fs::write(new_dir.join("metadata.toml"), METADATA_TOML)?;
    let all = match FileBasedMigrations::from_path(&mig_dir) {
        Ok(m) => m,
        Err(e) => return Ok(run.fail("up", sql_error("reading the new migration", e))),
    };

    let rows_before = match row_counts(&mut conn) {
        Ok(c) => c,
        Err(e) => return Ok(run.fail("up", sql_error("counting rows before the migration", e))),
    };
    if let Err(e) = conn.run_pending_migrations(all.clone()) {
        return Ok(run.fail("up", sql_error("running the new migration", e)));
    }
    run.pass("up", "The new migration ran without a SQLite error.");
    let rows_after = match row_counts(&mut conn) {
        Ok(c) => c,
        Err(e) => {
            return Ok(run.fail(
                "rows_kept",
                sql_error("counting rows after the migration", e),
            ));
        }
    };
    let (lost, compared) = rows_lost(&rows_before, &rows_after, after, &new.table_renames);
    if !lost.is_empty() {
        return Ok(run.fail(
            "rows_kept",
            finding(
                "rows_lost",
                format!(
                    "The new migration changed the number of stored rows: {}",
                    lost.join("; ")
                ),
            ),
        ));
    }
    run.pass(
        "rows_kept",
        if compared == 0 {
            "No existing lists to compare; no rows were at risk.".to_string()
        } else {
            let total: i64 = rows_before.values().sum();
            format!(
                "Every existing list kept all its rows ({} across {} lists compared, {} in the copy).",
                total, compared, rows_word(total)
            )
        },
    );
    let after_step = "running the new migration";
    if let Some(r) = check_after(
        &mut conn,
        after,
        after_step,
        &mut run,
        "db_matches_spec_after",
        true,
    ) {
        return Ok(r);
    }

    if let Err(e) = conn.revert_last_migration(all.clone()) {
        return Ok(run.fail("down", sql_error("reverting the new migration", e)));
    }
    run.pass(
        "down",
        "The new migration was reverted without a SQLite error.",
    );
    let reverted = match introspect(&mut conn) {
        Ok(s) => s,
        Err(e) => {
            return Ok(run.fail(
                "db_matches_spec_after_down",
                sql_error("reading the database structure", e),
            ));
        }
    };
    let problems = diff_shapes(&start, &reverted);
    if !problems.is_empty() {
        return Ok(run.fail(
            "db_matches_spec_after_down",
            mismatch(
                "not_reversible",
                "After reverting, the structure is not what it was before",
                problems,
            ),
        ));
    }
    run.pass(
        "db_matches_spec_after_down",
        "After reverting, the database structure was exactly what it was before.",
    );

    if let Err(e) = conn.run_pending_migrations(all) {
        return Ok(run.fail(
            "up_again",
            sql_error("running the new migration a second time", e),
        ));
    }
    run.pass(
        "up_again",
        "The new migration ran again without a SQLite error.",
    );
    if let Some(r) = check_after(
        &mut conn,
        after,
        "running the new migration a second time",
        &mut run,
        "db_matches_spec_after_up_again",
        false,
    ) {
        return Ok(r);
    }
    Ok(run.done())
}

/// Compare the DB with `after`, then check foreign keys. Returns the finished
/// report on failure, `None` when both passed (steps recorded in `run`).
fn check_after(
    conn: &mut SqliteConnection,
    after: &SchemaSpec,
    step: &str,
    run: &mut Run,
    spec_check: &str,
    record_fk: bool,
) -> Option<DryRunReport> {
    let take = |run: &mut Run| Run {
        steps: std::mem::take(&mut run.steps),
    };
    let shape: Shape = match introspect(conn) {
        Ok(s) => s,
        Err(e) => {
            return Some(
                take(run).fail(spec_check, sql_error("reading the database structure", e)),
            );
        }
    };
    let problems = diff(&expected_shape(after), &shape);
    if !problems.is_empty() {
        return Some(take(run).fail(
            spec_check,
            mismatch(
                "db_does_not_match_spec",
                &format!("After {step}, the database does not match the new schema"),
                problems,
            ),
        ));
    }
    run.pass(spec_check, "The database structure matched the new schema.");
    match diesel::sql_query("PRAGMA foreign_key_check").load::<FkViolation>(conn) {
        Ok(rows) if !rows.is_empty() => {
            let mut tables: Vec<String> = rows.into_iter().map(|r| r.table).collect();
            tables.sort();
            tables.dedup();
            Some(take(run).fail(
                "foreign_key_check",
                finding(
                    "foreign_key_violation",
                    format!(
                        "After {step}, rows in {} point at rows that do not exist",
                        tables.join(", ")
                    ),
                ),
            ))
        }
        Ok(_) => {
            if record_fk {
                run.pass(
                    "foreign_key_check",
                    "Every row that points at another row points at one that exists.",
                );
            }
            None
        }
        Err(e) => Some(take(run).fail("foreign_key_check", sql_error("checking foreign keys", e))),
    }
}

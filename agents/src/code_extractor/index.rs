use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

use super::extractor::{self, CodeBlock};

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS code_index_structs (
    name TEXT PRIMARY KEY,
    file TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    source TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS code_index_free_fns (
    name TEXT PRIMARY KEY,
    file TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    source TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS code_index_impl_fns (
    struct_name TEXT NOT NULL,
    fn_name TEXT NOT NULL,
    file TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    source TEXT NOT NULL,
    PRIMARY KEY (struct_name, fn_name)
);
";

// ---------------------------------------------------------------------------
// CodeIndex — SQLite-backed persistence
// ---------------------------------------------------------------------------

pub struct CodeIndex {
    conn: Connection,
}

impl CodeIndex {
    /// Open (or create) a code index backed by a SQLite file.
    pub fn open(path: &str) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
        Ok(Self { conn })
    }

    /// Open an in-memory index (useful for testing).
    pub fn memory() -> Result<Self, String> {
        Self::open(":memory:")
    }

    // -----------------------------------------------------------------------
    // Build / Rebuild
    // -----------------------------------------------------------------------

    /// Scan `root` and populate the index tables. Existing entries are cleared first.
    pub fn build(&mut self, root: &Path) -> Result<BuildStats, String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;

        tx.execute("DELETE FROM code_index_structs", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM code_index_free_fns", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM code_index_impl_fns", [])
            .map_err(|e| e.to_string())?;

        let mut stats = BuildStats::default();

        for path in extractor::rust_sources(root) {
            let file_stats = scan_file(&path, root, &tx)?;
            stats.structs += file_stats.structs;
            stats.free_fns += file_stats.free_fns;
            stats.impl_fns += file_stats.impl_fns;
        }

        tx.commit().map_err(|e| e.to_string())?;
        Ok(stats)
    }

    /// Re-index a single file (delete old entries, re-scan).
    pub fn reindex_file(&mut self, root: &Path, file: &Path) -> Result<BuildStats, String> {
        let rel = file.strip_prefix(root).unwrap_or(file);
        let rel_str = rel.to_string_lossy();

        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM code_index_structs WHERE file = ?1",
            params![rel_str],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM code_index_free_fns WHERE file = ?1",
            params![rel_str],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM code_index_impl_fns WHERE file = ?1",
            params![rel_str],
        )
        .map_err(|e| e.to_string())?;

        let file_stats = scan_file(file, root, &tx)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(file_stats)
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    pub fn get_struct(&self, name: &str) -> Result<Option<CodeBlock>, String> {
        self.conn
            .query_row(
                "SELECT file, start_line, end_line, source FROM code_index_structs WHERE name = ?1",
                params![name],
                |row| row_to_block(row),
            )
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn get_free_fn(&self, name: &str) -> Result<Option<CodeBlock>, String> {
        self.conn
            .query_row(
                "SELECT file, start_line, end_line, source FROM code_index_free_fns WHERE name = ?1",
                params![name],
                |row| row_to_block(row),
            )
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn get_impl_fn(&self, struct_name: &str, fn_name: &str) -> Result<Option<CodeBlock>, String> {
        self.conn
            .query_row(
                "SELECT file, start_line, end_line, source FROM code_index_impl_fns WHERE struct_name = ?1 AND fn_name = ?2",
                params![struct_name, fn_name],
                |row| row_to_block(row),
            )
            .optional()
            .map_err(|e| e.to_string())
    }

    /// List all impl method names for a struct.
    pub fn list_impl_fns(&self, struct_name: &str) -> Result<Vec<String>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT fn_name FROM code_index_impl_fns WHERE struct_name = ?1 ORDER BY fn_name",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![struct_name], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// List all indexed struct names.
    pub fn list_structs(&self) -> Result<Vec<String>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM code_index_structs ORDER BY name")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// List all indexed free function names.
    pub fn list_free_fns(&self) -> Result<Vec<String>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM code_index_free_fns ORDER BY name")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

fn row_to_block(row: &rusqlite::Row<'_>) -> rusqlite::Result<CodeBlock> {
    let file: String = row.get(0)?;
    Ok(CodeBlock {
        file: PathBuf::from(file),
        start_line: row.get(1)?,
        end_line: row.get(2)?,
        source: row.get(3)?,
    })
}

#[derive(Debug, Default)]
pub struct BuildStats {
    pub structs: usize,
    pub free_fns: usize,
    pub impl_fns: usize,
}

// ---------------------------------------------------------------------------
// File scanning
// ---------------------------------------------------------------------------

fn scan_file(
    path: &Path,
    root: &Path,
    tx: &rusqlite::Transaction<'_>,
) -> Result<BuildStats, String> {
    use streaming_iterator::StreamingIterator;
    use tree_sitter::{Language, Node, Parser, Query, QueryCursor};

    let src = std::fs::read_to_string(path).map_err(|e| format!("read {path:?}: {e}"))?;
    let rel = path.strip_prefix(root).unwrap_or(path);
    let rel_str = rel.to_string_lossy().to_string();

    let language: Language = tree_sitter_rust::LANGUAGE.into();
    let mut parser = Parser::new();
    parser.set_language(&language).map_err(|e| e.to_string())?;
    let tree = parser
        .parse(src.as_bytes(), None)
        .ok_or_else(|| format!("parse failed for {path:?}"))?;

    let mut stats = BuildStats::default();

    fn find_cap<'a>(caps: &[tree_sitter::QueryCapture<'a>], q: &Query, name: &str) -> Option<Node<'a>> {
        let idx = q.capture_names().iter().position(|n| *n == name).map(|i| i as u32)?;
        caps.iter().find(|c| c.index == idx).map(|c| c.node)
    }

    // --- Structs ---
    let q_struct = Query::new(&language, super::queries::STRUCT_DEF).map_err(|e| e.to_string())?;
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&q_struct, tree.root_node(), src.as_bytes());
    while let Some(m) = matches.next() {
        if let (Some(nn), Some(it)) = (find_cap(m.captures, &q_struct, "name"), find_cap(m.captures, &q_struct, "item")) {
            let name = &src[nn.byte_range()];
            let start = it.start_position().row as u32 + 1;
            let end = it.end_position().row as u32 + 1;
            let source = &src[it.byte_range()];
            tx.execute(
                "INSERT OR REPLACE INTO code_index_structs (name, file, start_line, end_line, source) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![name, rel_str, start, end, source],
            ).map_err(|e| e.to_string())?;
            stats.structs += 1;
        }
    }

    // --- Free functions ---
    let q_fn = Query::new(&language, super::queries::ANY_FN).map_err(|e| e.to_string())?;
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&q_fn, tree.root_node(), src.as_bytes());
    while let Some(m) = matches.next() {
        if let (Some(nn), Some(it)) = (find_cap(m.captures, &q_fn, "fn_name"), find_cap(m.captures, &q_fn, "item")) {
            if it.parent().map_or(false, |p| p.kind() == "source_file") {
                let name = &src[nn.byte_range()];
                let start = it.start_position().row as u32 + 1;
                let end = it.end_position().row as u32 + 1;
                let source = &src[it.byte_range()];
                tx.execute(
                    "INSERT OR REPLACE INTO code_index_free_fns (name, file, start_line, end_line, source) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![name, rel_str, start, end, source],
                ).map_err(|e| e.to_string())?;
                stats.free_fns += 1;
            }
        }
    }

    // --- Impl methods ---
    let q_impl = Query::new(&language, super::queries::IMPL_BLOCK).map_err(|e| e.to_string())?;
    let q_impl_fn = Query::new(&language, super::queries::IMPL_FN).map_err(|e| e.to_string())?;
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&q_impl, tree.root_node(), src.as_bytes());
    while let Some(m) = matches.next() {
        if let (Some(sn), Some(impl_it)) = (find_cap(m.captures, &q_impl, "struct_name"), find_cap(m.captures, &q_impl, "item")) {
            let struct_name = &src[sn.byte_range()];

            // Skip trait impls (impl Trait for Struct) — only inherent impls
            if impl_it.child_by_field_name("trait").is_some() {
                continue;
            }

            let mut fn_cursor = QueryCursor::new();
            let mut fn_matches = fn_cursor.matches(&q_impl_fn, impl_it, src.as_bytes());
            while let Some(fm) = fn_matches.next() {
                if let (Some(fnn), Some(fn_it)) = (find_cap(fm.captures, &q_impl_fn, "fn_name"), find_cap(fm.captures, &q_impl_fn, "item")) {
                    let fn_name = &src[fnn.byte_range()];
                    let start = fn_it.start_position().row as u32 + 1;
                    let end = fn_it.end_position().row as u32 + 1;
                    let source = &src[fn_it.byte_range()];
                    tx.execute(
                        "INSERT OR REPLACE INTO code_index_impl_fns (struct_name, fn_name, file, start_line, end_line, source) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![struct_name, fn_name, rel_str, start, end, source],
                    ).map_err(|e| e.to_string())?;
                    stats.impl_fns += 1;
                }
            }
        }
    }

    Ok(stats)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn rustysolid_backend() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("rustysolid")
            .join("backend")
            .join("src")
    }

    #[test]
    fn test_build_and_query_struct() {
        let src = rustysolid_backend();
        let mut idx = CodeIndex::memory().expect("open memory");
        let stats = idx.build(&src).expect("build ok");
        assert!(stats.structs > 0, "should find structs");

        let block = idx.get_struct("ContactRecord").expect("query ok");
        let block = block.expect("struct found");
        assert!(block.source.contains("pub struct ContactRecord"));
        assert!(block.source.contains("pub id: i64"));
    }

    #[test]
    fn test_build_and_query_free_fn() {
        let src = rustysolid_backend();
        let mut idx = CodeIndex::memory().expect("open memory");
        idx.build(&src).expect("build ok");

        let block = idx.get_free_fn("register_user").expect("query ok");
        let block = block.expect("fn found");
        assert!(block.source.contains("fn register_user"));
    }

    #[test]
    fn test_build_and_query_impl_fn() {
        let src = rustysolid_backend();
        let mut idx = CodeIndex::memory().expect("open memory");
        idx.build(&src).expect("build ok");

        let block = idx.get_impl_fn("ContactRecord", "find_by_email").expect("query ok");
        let block = block.expect("fn found");
        assert!(block.source.contains("fn find_by_email"));
    }

    #[test]
    fn test_list_structs() {
        let src = rustysolid_backend();
        let mut idx = CodeIndex::memory().expect("open memory");
        idx.build(&src).expect("build ok");

        let structs = idx.list_structs().expect("list ok");
        assert!(structs.contains(&"ContactRecord".to_string()));
    }

    #[test]
    fn test_list_impl_fns() {
        let src = rustysolid_backend();
        let mut idx = CodeIndex::memory().expect("open memory");
        idx.build(&src).expect("build ok");

        let fns = idx.list_impl_fns("ContactRecord").expect("list ok");
        assert!(fns.contains(&"find_by_email".to_string()));
        assert!(fns.contains(&"verify".to_string()));
    }

    #[test]
    fn test_reindex_file() {
        let src = rustysolid_backend();
        let mut idx = CodeIndex::memory().expect("open memory");
        idx.build(&src).expect("build ok");

        let contact_path = src.join("models").join("contact.rs");
        let stats = idx.reindex_file(&src, &contact_path).expect("reindex ok");
        assert!(stats.structs > 0);

        // Should still be queryable
        let block = idx.get_struct("ContactRecord").expect("query ok");
        assert!(block.is_some());
    }

    #[test]
    fn test_not_found_returns_none() {
        let src = rustysolid_backend();
        let mut idx = CodeIndex::memory().expect("open memory");
        idx.build(&src).expect("build ok");

        assert!(idx.get_struct("Nonexistent").expect("query ok").is_none());
        assert!(idx.get_free_fn("nonexistent_fn").expect("query ok").is_none());
        assert!(idx.get_impl_fn("ContactRecord", "nonexistent").expect("query ok").is_none());
    }
}

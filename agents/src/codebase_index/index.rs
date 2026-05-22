use std::collections::HashMap;
use std::path::{Path, PathBuf};

use regex::Regex;
use walkdir::WalkDir;

pub struct CodebaseIndex {
    /// struct_name → table_name, derived from `#[diesel(table_name = X)]` attributes.
    pub struct_to_table: HashMap<String, String>,
}

pub fn build(root: &Path) -> Result<CodebaseIndex, Box<dyn std::error::Error + Send + Sync>> {
    let mut struct_to_table = HashMap::new();
    for path in rust_sources(root) {
        let src = std::fs::read_to_string(&path)?;
        parse_struct_mappings(&src, &mut struct_to_table);
    }
    Ok(CodebaseIndex { struct_to_table })
}

/// Extracts `#[diesel(table_name = foo)]` → `struct Bar` pairs from source text.
/// Assumes the struct declaration immediately follows the diesel attribute (possibly with
/// other attributes in between). This holds for all well-structured Diesel models.
fn parse_struct_mappings(src: &str, out: &mut HashMap<String, String>) {
    let attr_re = Regex::new(r"#\[diesel\(table_name\s*=\s*(\w+)\)\]").unwrap();
    let struct_re = Regex::new(r"(?:pub\s+)?struct\s+(\w+)").unwrap();

    for cap in attr_re.captures_iter(src) {
        let table_name = cap[1].to_string();
        let after_attr = &src[cap.get(0).unwrap().end()..];
        if let Some(sc) = struct_re.captures(after_attr) {
            out.insert(sc[1].to_string(), table_name);
        }
    }
}

/// Walks `root` recursively, returning paths to all `.rs` files.
/// Skips `target/`, `.git/`, and `node_modules/`.
pub fn rust_sources(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            !matches!(
                e.file_name().to_str().unwrap_or(""),
                "target" | ".git" | "node_modules"
            )
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "rs"))
        .map(|e| e.into_path())
        .collect()
}

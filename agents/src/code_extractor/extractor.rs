use std::path::{Path, PathBuf};

use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Node, Parser, Query, QueryCursor};

use super::queries;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CodeBlock {
    pub file: PathBuf,
    pub start_line: u32,
    pub end_line: u32,
    pub source: String,
}

// ---------------------------------------------------------------------------
// Parser setup
// ---------------------------------------------------------------------------

fn make_parser() -> Result<Parser, String> {
    let language: Language = tree_sitter_rust::LANGUAGE.into();
    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .map_err(|e| format!("tree-sitter: {e}"))?;
    Ok(parser)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn node_source(node: Node, src: &str) -> String {
    src[node.byte_range()].to_string()
}

fn line_range(node: Node) -> (u32, u32) {
    let start = node.start_position().row as u32 + 1;
    let end = node.end_position().row as u32 + 1;
    (start, end)
}

fn run_query<'a>(
    query: &Query,
    root: Node<'a>,
    bytes: &'a [u8],
) -> impl Iterator<Item = Vec<tree_sitter::QueryCapture<'a>>> + 'a {
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, root, bytes);
    let mut captures: Vec<Vec<tree_sitter::QueryCapture<'a>>> = Vec::new();
    while let Some(m) = matches.next() {
        captures.push(m.captures.to_vec());
    }
    captures.into_iter()
}

fn find_capture<'a>(
    caps: &[tree_sitter::QueryCapture<'a>],
    query: &Query,
    name: &str,
) -> Option<Node<'a>> {
    let idx = query
        .capture_names()
        .iter()
        .position(|n| *n == name)
        .map(|i| i as u32)?;
    caps.iter().find(|c| c.index == idx).map(|c| c.node)
}

/// Returns true if `node` is a direct child of `source_file`.
fn is_top_level(node: Node) -> bool {
    node.parent().map_or(false, |p| p.kind() == "source_file")
}

/// Collect all .rs files under `root`, skipping target/.git/node_modules.
pub fn rust_sources(root: &Path) -> Vec<PathBuf> {
    use walkdir::WalkDir;
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

// ---------------------------------------------------------------------------
// Extraction: struct definition
// ---------------------------------------------------------------------------

pub fn extract_struct(path: &Path, name: &str) -> Result<Option<CodeBlock>, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("read {path:?}: {e}"))?;
    let mut parser = make_parser()?;
    let tree = parser
        .parse(src.as_bytes(), None)
        .ok_or_else(|| format!("parse failed for {path:?}"))?;

    let query = Query::new(&tree_sitter_rust::LANGUAGE.into(), queries::STRUCT_DEF)
        .map_err(|e| format!("query compile: {e}"))?;

    for caps in run_query(&query, tree.root_node(), src.as_bytes()) {
        let name_node = find_capture(&caps, &query, "name");
        let item_node = find_capture(&caps, &query, "item");
        if let (Some(nn), Some(it)) = (name_node, item_node) {
            if &src[nn.byte_range()] == name {
                let (start_line, end_line) = line_range(it);
                return Ok(Some(CodeBlock {
                    file: path.to_path_buf(),
                    start_line,
                    end_line,
                    source: node_source(it, &src),
                }));
            }
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// Extraction: enum definition
// ---------------------------------------------------------------------------

pub fn extract_enum(path: &Path, name: &str) -> Result<Option<CodeBlock>, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("read {path:?}: {e}"))?;
    let mut parser = make_parser()?;
    let tree = parser
        .parse(src.as_bytes(), None)
        .ok_or_else(|| format!("parse failed for {path:?}"))?;

    let query = Query::new(&tree_sitter_rust::LANGUAGE.into(), queries::ENUM_DEF)
        .map_err(|e| format!("query compile: {e}"))?;

    for caps in run_query(&query, tree.root_node(), src.as_bytes()) {
        let name_node = find_capture(&caps, &query, "name");
        let item_node = find_capture(&caps, &query, "item");
        if let (Some(nn), Some(it)) = (name_node, item_node) {
            if &src[nn.byte_range()] == name {
                let (start_line, end_line) = line_range(it);
                return Ok(Some(CodeBlock {
                    file: path.to_path_buf(),
                    start_line,
                    end_line,
                    source: node_source(it, &src),
                }));
            }
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// Find dependent types referenced in a struct's fields
// ---------------------------------------------------------------------------

const KNOWN_TYPES: &[&str] = &[
    "i8",
    "i16",
    "i32",
    "i64",
    "i128",
    "isize",
    "u8",
    "u16",
    "u32",
    "u64",
    "u128",
    "usize",
    "f32",
    "f64",
    "bool",
    "char",
    "String",
    "str",
    "NaiveDateTime",
    "NaiveDate",
    "NaiveTime",
    "Uuid",
];

/// Parse the struct source for field types that are NOT standard Rust/chrono types.
/// Returns unique type names that need to be resolved (e.g. "ContactType").
fn collect_custom_type_names(struct_code: &str) -> Vec<String> {
    let mut types = Vec::new();
    let mut parser = make_parser().ok();
    let Some(ref mut parser) = parser else {
        return types;
    };
    let Some(tree) = parser.parse(struct_code.as_bytes(), None) else {
        return types;
    };

    let lang = tree_sitter_rust::LANGUAGE.into();
    // Match any type_identifier that appears as a field type in a struct.
    let type_query = Query::new(
        &lang,
        "(field_declaration type: (type_identifier) @type_name)",
    )
    .ok();
    let Some(type_query) = type_query else {
        return types;
    };

    for caps in run_query(&type_query, tree.root_node(), struct_code.as_bytes()) {
        let type_node = find_capture(&caps, &type_query, "type_name");
        if let Some(tn) = type_node {
            let name = &struct_code[tn.byte_range()];
            if !KNOWN_TYPES.contains(&name) && !types.iter().any(|t| t == name) {
                types.push(name.to_string());
            }
        }
    }

    types
}

/// For each custom type referenced in the struct, try to find and extract its
/// definition. Searches the same file first, then all Rust files under `root`.
pub fn find_dependent_types(
    root: &Path,
    struct_file: &Path,
    struct_code: &str,
) -> Result<Vec<CodeBlock>, String> {
    let type_names = collect_custom_type_names(struct_code);
    let mut result = Vec::new();

    for type_name in type_names {
        // Try the struct's own file first (enums are often co-located).
        if let Some(block) = extract_enum(struct_file, &type_name)? {
            result.push(block);
            continue;
        }

        // Search all files.
        for path in rust_sources(root) {
            if path == struct_file {
                continue;
            }
            if let Some(block) = extract_enum(&path, &type_name)? {
                result.push(block);
                break;
            }
        }
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// Extraction: free function (top-level, not inside impl/trait)
// ---------------------------------------------------------------------------

pub fn extract_free_fn(path: &Path, name: &str) -> Result<Option<CodeBlock>, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("read {path:?}: {e}"))?;
    let mut parser = make_parser()?;
    let tree = parser
        .parse(src.as_bytes(), None)
        .ok_or_else(|| format!("parse failed for {path:?}"))?;

    let query = Query::new(&tree_sitter_rust::LANGUAGE.into(), queries::ANY_FN)
        .map_err(|e| format!("query compile: {e}"))?;

    for caps in run_query(&query, tree.root_node(), src.as_bytes()) {
        let name_node = find_capture(&caps, &query, "fn_name");
        let item_node = find_capture(&caps, &query, "item");
        if let (Some(nn), Some(it)) = (name_node, item_node) {
            if &src[nn.byte_range()] == name && is_top_level(it) {
                let (start_line, end_line) = line_range(it);
                return Ok(Some(CodeBlock {
                    file: path.to_path_buf(),
                    start_line,
                    end_line,
                    source: node_source(it, &src),
                }));
            }
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// Extraction: impl method (StructName::method_name)
// ---------------------------------------------------------------------------

pub fn extract_impl_fn(
    path: &Path,
    struct_name: &str,
    fn_name: &str,
) -> Result<Option<CodeBlock>, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("read {path:?}: {e}"))?;
    let mut parser = make_parser()?;
    let tree = parser
        .parse(src.as_bytes(), None)
        .ok_or_else(|| format!("parse failed for {path:?}"))?;

    let lang = tree_sitter_rust::LANGUAGE.into();
    let impl_query =
        Query::new(&lang, queries::IMPL_BLOCK).map_err(|e| format!("query compile: {e}"))?;
    let fn_query =
        Query::new(&lang, queries::IMPL_FN).map_err(|e| format!("query compile: {e}"))?;

    for caps in run_query(&impl_query, tree.root_node(), src.as_bytes()) {
        let struct_node = find_capture(&caps, &impl_query, "struct_name");
        let impl_node = find_capture(&caps, &impl_query, "item");
        if let (Some(sn), Some(impl_it)) = (struct_node, impl_node) {
            if &src[sn.byte_range()] != struct_name {
                continue;
            }

            // Search inside this impl block's declaration_list for the method.
            for fn_caps in run_query(&fn_query, impl_it, src.as_bytes()) {
                let fn_name_node = find_capture(&fn_caps, &fn_query, "fn_name");
                let fn_item_node = find_capture(&fn_caps, &fn_query, "item");
                if let (Some(fnn), Some(fn_it)) = (fn_name_node, fn_item_node) {
                    if &src[fnn.byte_range()] == fn_name {
                        let (start_line, end_line) = line_range(fn_it);
                        return Ok(Some(CodeBlock {
                            file: path.to_path_buf(),
                            start_line,
                            end_line,
                            source: node_source(fn_it, &src),
                        }));
                    }
                }
            }
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// Scan all files for a struct (returns the file where it's defined)
// ---------------------------------------------------------------------------

pub fn find_struct_file(root: &Path, name: &str) -> Result<Option<PathBuf>, String> {
    for path in rust_sources(root) {
        if extract_struct(&path, name)?.is_some() {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

pub fn find_free_fn_file(root: &Path, name: &str) -> Result<Option<PathBuf>, String> {
    for path in rust_sources(root) {
        if extract_free_fn(&path, name)?.is_some() {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

pub fn find_impl_fn_file(
    root: &Path,
    struct_name: &str,
    fn_name: &str,
) -> Result<Option<PathBuf>, String> {
    for path in rust_sources(root) {
        if extract_impl_fn(&path, struct_name, fn_name)?.is_some() {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// Extraction: all impl methods for a struct (for use as examples)
// ---------------------------------------------------------------------------

pub fn list_impl_fns(path: &Path, struct_name: &str) -> Result<Vec<CodeBlock>, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("read {path:?}: {e}"))?;
    let mut parser = make_parser()?;
    let tree = parser
        .parse(src.as_bytes(), None)
        .ok_or_else(|| format!("parse failed for {path:?}"))?;

    let lang = tree_sitter_rust::LANGUAGE.into();
    let impl_query =
        Query::new(&lang, queries::IMPL_BLOCK).map_err(|e| format!("query compile: {e}"))?;
    let fn_query =
        Query::new(&lang, queries::IMPL_FN).map_err(|e| format!("query compile: {e}"))?;

    let mut result = Vec::new();

    for caps in run_query(&impl_query, tree.root_node(), src.as_bytes()) {
        let struct_node = find_capture(&caps, &impl_query, "struct_name");
        let impl_node = find_capture(&caps, &impl_query, "item");
        if let (Some(sn), Some(impl_it)) = (struct_node, impl_node) {
            if &src[sn.byte_range()] != struct_name {
                continue;
            }
            for fn_caps in run_query(&fn_query, impl_it, src.as_bytes()) {
                let fn_item_node = find_capture(&fn_caps, &fn_query, "item");
                if let Some(fn_it) = fn_item_node {
                    let (start_line, end_line) = line_range(fn_it);
                    result.push(CodeBlock {
                        file: path.to_path_buf(),
                        start_line,
                        end_line,
                        source: node_source(fn_it, &src),
                    });
                }
            }
        }
    }
    Ok(result)
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
    fn test_extract_struct_contact_record() {
        let src = rustysolid_backend();
        let path = src.join("models").join("contact.rs");
        let block = extract_struct(&path, "ContactRecord").expect("parse ok");
        let block = block.expect("struct found");
        assert!(block.source.contains("pub struct ContactRecord"));
        assert!(block.source.contains("pub id: i64"));
        assert!(block.source.contains("pub contact_type: ContactType"));
    }

    #[test]
    fn test_extract_enum_contact_type() {
        let src = rustysolid_backend();
        let path = src.join("models").join("contact.rs");
        let block = extract_enum(&path, "ContactType").expect("parse ok");
        let block = block.expect("enum found");
        assert!(block.source.contains("pub enum ContactType"));
        assert!(block.source.contains("Email"));
        assert!(block.source.contains("Phone"));
    }

    #[test]
    fn test_find_dependent_types_contact_record() {
        let src = rustysolid_backend();
        let path = src.join("models").join("contact.rs");
        let struct_block = extract_struct(&path, "ContactRecord")
            .expect("parse ok")
            .expect("struct found");
        let deps = find_dependent_types(&src, &path, &struct_block.source).expect("scan ok");
        assert_eq!(deps.len(), 1);
        assert!(deps[0].source.contains("pub enum ContactType"));
    }

    #[test]
    fn test_extract_struct_not_found() {
        let src = rustysolid_backend();
        let path = src.join("models").join("contact.rs");
        let block = extract_struct(&path, "Nonexistent").expect("parse ok");
        assert!(block.is_none());
    }

    #[test]
    fn test_extract_free_fn_register_user() {
        let src = rustysolid_backend();
        let path = src.join("auth").join("db.rs");
        let block = extract_free_fn(&path, "register_user").expect("parse ok");
        let block = block.expect("fn found");
        assert!(block.source.contains("pub fn register_user"));
        assert!(block.source.contains("fn register_user("));
    }

    #[test]
    fn test_extract_free_fn_not_found() {
        let src = rustysolid_backend();
        let path = src.join("auth").join("db.rs");
        let block = extract_free_fn(&path, "nonexistent_fn").expect("parse ok");
        assert!(block.is_none());
    }

    #[test]
    fn test_extract_impl_fn_find_by_email() {
        let src = rustysolid_backend();
        let path = src.join("models").join("contact.rs");
        let block = extract_impl_fn(&path, "ContactRecord", "find_by_email").expect("parse ok");
        let block = block.expect("fn found");
        assert!(block.source.contains("fn find_by_email"));
        assert!(block.source.contains("user_contacts::table"));
    }

    #[test]
    fn test_extract_impl_fn_verify() {
        let src = rustysolid_backend();
        let path = src.join("models").join("contact.rs");
        let block = extract_impl_fn(&path, "ContactRecord", "verify").expect("parse ok");
        let block = block.expect("fn found");
        assert!(block.source.contains("fn verify"));
    }

    #[test]
    fn test_extract_impl_fn_not_found() {
        let src = rustysolid_backend();
        let path = src.join("models").join("contact.rs");
        let block = extract_impl_fn(&path, "ContactRecord", "nonexistent").expect("parse ok");
        assert!(block.is_none());
    }

    #[test]
    fn test_find_struct_file() {
        let src = rustysolid_backend();
        let path = find_struct_file(&src, "ContactRecord").expect("scan ok");
        let path = path.expect("struct found");
        assert!(path.to_string_lossy().contains("contact.rs"));
    }

    #[test]
    fn test_find_free_fn_file() {
        let src = rustysolid_backend();
        let path = find_free_fn_file(&src, "register_user").expect("scan ok");
        let path = path.expect("fn found");
        assert!(path.to_string_lossy().contains("db.rs"));
    }

    #[test]
    fn test_find_impl_fn_file() {
        let src = rustysolid_backend();
        let path = find_impl_fn_file(&src, "ContactRecord", "find_by_email").expect("scan ok");
        let path = path.expect("fn found");
        assert!(path.to_string_lossy().contains("contact.rs"));
    }
}

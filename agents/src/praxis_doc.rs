//! Extracts doc comments and type signatures from `nocodo_praxis` source files
//! and formats them as prompt-safe reference text for LLM consumption.
//!
//! # Usage
//!
//! ```ignore
//! use agents::praxis_doc;
//!
//! let ref_text = praxis_doc::all_praxis_types_reference(None);
//! // Or per-module:
//! let auth_ref = praxis_doc::auth_types_reference(None);
//! ```

use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

/// Resolves the path to `nocodo_praxis/src/`, preferring the provided
/// workspace root, falling back to `CARGO_MANIFEST_DIR`.
pub fn resolve_praxis_src(workspace_root: Option<&Path>) -> PathBuf {
    if let Some(root) = workspace_root {
        root.join("nocodo_praxis").join("src")
    } else if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        Path::new(&dir).join("..").join("nocodo_praxis").join("src")
    } else {
        Path::new("../nocodo_praxis/src").to_path_buf()
    }
}

// ── Parsing helpers ─────────────────────────────────────────────────────────

/// A single documented item: its doc comment text, type definition source, and
/// any impl block methods.
#[derive(Debug, Clone)]
pub struct DocItem {
    pub doc: String,
    pub definition: String,
    pub impl_methods: Vec<String>,
}

/// Parse a single `.rs` source file into a list of documented items.
///
/// Recognises:
/// - `///` and `//!` doc comment lines (collected and associated with the
///   next struct/enum definition or impl block).
/// - Struct and enum definitions (the full block from `pub struct` / `pub enum`
///   to the closing `}`).
/// - `impl` blocks (methods with their own doc comments).
fn parse_source(source: &str) -> Vec<DocItem> {
    let mut items: Vec<DocItem> = Vec::new();
    let mut current_doc: Vec<&str> = Vec::new();
    let mut in_inner_doc = false; // inside a //! block

    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0usize;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Collect doc comments — both /// and //!
        if trimmed.starts_with("///") || trimmed.starts_with("//!") {
            // //! lines are module-level, collect them separately
            if trimmed.starts_with("//!") {
                if !in_inner_doc {
                    current_doc.clear();
                    in_inner_doc = true;
                }
                let doc_text = trimmed
                    .strip_prefix("//!").unwrap()
                    .strip_prefix(' ').unwrap_or("");
                current_doc.push(doc_text);
            } else {
                in_inner_doc = false;
                let doc_text = trimmed
                    .strip_prefix("///").unwrap()
                    .strip_prefix(' ').unwrap_or("");
                current_doc.push(doc_text);
            }
            i += 1;
            continue;
        }

        // Doc comment ended — check if next line is a struct/enum definition
        if !current_doc.is_empty() && (trimmed.starts_with("pub struct ") || trimmed.starts_with("pub enum ")) {
            let def = extract_block(&lines, &mut i);
            let methods = Vec::new(); // impl methods collected separately
            items.push(DocItem {
                doc: current_doc.join("\n"),
                definition: def,
                impl_methods: methods,
            });
            current_doc.clear();
            in_inner_doc = false;
            continue;
        }

        // An impl block — collect doc comments for methods inside it
        if trimmed.starts_with("impl ") && !trimmed.starts_with("impl<") {
            // Find the matching }
            let block = extract_block(&lines, &mut i);
            // Parse out individual methods with their doc comments
            let methods = parse_impl_methods(&block);
            // Find the most recent item and attach methods
            if let Some(last) = items.last_mut() {
                last.impl_methods.extend(methods);
            }
            continue;
        }

        // Standalone functions (pub fn) — treat like items
        if !current_doc.is_empty() && trimmed.starts_with("pub fn ") {
            let def = extract_block(&lines, &mut i);
            items.push(DocItem {
                doc: current_doc.join("\n"),
                definition: def,
                impl_methods: Vec::new(),
            });
            current_doc.clear();
            in_inner_doc = false;
            continue;
        }

        in_inner_doc = false;
        i += 1;
    }

    items
}

/// Extract a braced block starting at `lines[i]`, advancing `i` past the
/// closing `}`. Returns the full source of the block including the opening line.
fn extract_block(lines: &[&str], i: &mut usize) -> String {
    let start = *i;
    let mut depth = 0u32;
    let mut started = false;

    while *i < lines.len() {
        let line = lines[*i];
        let trimmed = line.trim();
        if trimmed.contains('{') {
            depth += trimmed.matches('{').count() as u32;
            started = true;
        }
        if trimmed.contains('}') {
            depth = depth.saturating_sub(trimmed.matches('}').count() as u32);
        }
        *i += 1;
        if started && depth == 0 {
            break;
        }
    }

    lines[start..*i].join("\n")
}

/// Parse methods (fn definitions with their doc comments) from an impl block.
fn parse_impl_methods(impl_block: &str) -> Vec<String> {
    let mut methods: Vec<String> = Vec::new();
    let mut current_doc: Vec<&str> = Vec::new();

    for line in impl_block.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("///") {
            let doc_text = trimmed
                .strip_prefix("///").unwrap()
                .strip_prefix(' ').unwrap_or("");
            current_doc.push(doc_text);
            continue;
        }

        if trimmed.starts_with("pub fn ") {
            let mut method = String::new();
            if !current_doc.is_empty() {
                write!(method, "{}\n", current_doc.join("\n")).unwrap();
            }
            write!(method, "{}", trimmed.trim_end_matches(|c| c == '{' || c == ' ')).unwrap();
            if trimmed.contains('{') {
                method.push_str(" { ... }");
            }
            methods.push(method);
            current_doc.clear();
        }
    }

    methods
}

// ── Formatting ──────────────────────────────────────────────────────────────

/// Format a list of `DocItem`s as prompt-safe text.
///
/// Output: doc comment text, then the type definition, then impl methods
/// (one per line with `    ` prefix). Items separated by blank lines.
pub fn format_reference(items: &[DocItem]) -> String {
    let mut out = String::new();

    for item in items {
        if !item.doc.is_empty() {
            out.push_str(&item.doc);
            out.push('\n');
            out.push('\n');
        }
        out.push_str(&item.definition);
        out.push('\n');
        for method in &item.impl_methods {
            write!(out, "    {method}\n").unwrap();
        }
        out.push('\n');
    }

    out
}

/// Read and parse a source file. Returns formatted reference text.
pub fn read_module_ref(src_dir: &Path, module_name: &str) -> String {
    let file_path = src_dir.join(format!("{module_name}.rs"));
    match fs::read_to_string(&file_path) {
        Ok(source) => {
            let items = parse_source(&source);
            format_reference(&items)
        }
        Err(e) => {
            format!("(Could not read {}: {e})\n", file_path.display())
        }
    }
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Reference text for `nocodo_praxis::primitives` (AtLeastOne, Unresolved).
pub fn primitives_reference(workspace_root: Option<&Path>) -> String {
    let src_dir = resolve_praxis_src(workspace_root);
    read_module_ref(&src_dir, "primitives")
}

/// Reference text for `nocodo_praxis::provenance` (Provenance, PrdValue).
pub fn provenance_reference(workspace_root: Option<&Path>) -> String {
    let src_dir = resolve_praxis_src(workspace_root);
    read_module_ref(&src_dir, "provenance")
}

/// Reference text for `nocodo_praxis::auth` (Role, Permission, UserPersona, etc.).
pub fn auth_types_reference(workspace_root: Option<&Path>) -> String {
    let src_dir = resolve_praxis_src(workspace_root);
    let mut out = read_module_ref(&src_dir, "auth");
    // Also include the lib.rs crate-level docs
    let lib_ref = read_module_ref(&src_dir, "lib");
    out.push_str(&lib_ref);
    out
}

/// Reference text for `nocodo_praxis::statemachine` (State, Transition, etc.).
pub fn statemachine_types_reference(workspace_root: Option<&Path>) -> String {
    let src_dir = resolve_praxis_src(workspace_root);
    read_module_ref(&src_dir, "statemachine")
}

/// Reference text for `nocodo_praxis::entity` (Entity, Field, etc.).
pub fn entity_types_reference(workspace_root: Option<&Path>) -> String {
    let src_dir = resolve_praxis_src(workspace_root);
    read_module_ref(&src_dir, "entity")
}

/// Full reference: all `nocodo_praxis` types across all modules.
pub fn all_praxis_types_reference(workspace_root: Option<&Path>) -> String {
    let src_dir = resolve_praxis_src(workspace_root);
    let mut out = String::new();

    out.push_str(&read_module_ref(&src_dir, "lib"));
    out.push_str("---\n\n");
    out.push_str(&read_module_ref(&src_dir, "primitives"));
    out.push_str("---\n\n");
    out.push_str(&read_module_ref(&src_dir, "provenance"));
    out.push_str("---\n\n");
    out.push_str(&read_module_ref(&src_dir, "auth"));
    out.push_str("---\n\n");
    out.push_str(&read_module_ref(&src_dir, "statemachine"));
    out.push_str("---\n\n");
    out.push_str(&read_module_ref(&src_dir, "entity"));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_uses_cargo_manifest_dir() {
        let path = resolve_praxis_src(None);
        assert!(path.ends_with("nocodo_praxis/src"), "Got: {path:?}");
    }

    #[test]
    fn extract_auth_types() {
        let ref_text = auth_types_reference(None);
        assert!(ref_text.contains("RoleId"), "Missing RoleId in auth ref");
        assert!(ref_text.contains("UserPersona"), "Missing UserPersona");
        assert!(ref_text.contains("Permission"), "Missing Permission");
        assert!(ref_text.contains("RoleSemantics"), "Missing RoleSemantics");
        assert!(!ref_text.contains("(Could not read"), "File read failed");
    }

    #[test]
    fn extract_primitives() {
        let ref_text = primitives_reference(None);
        assert!(ref_text.contains("AtLeastOne"), "Missing AtLeastOne");
        assert!(ref_text.contains("Unresolved"), "Missing Unresolved");
    }

    #[test]
    fn extract_provenance() {
        let ref_text = provenance_reference(None);
        assert!(ref_text.contains("Provenance"), "Missing Provenance");
        assert!(ref_text.contains("PrdValue"), "Missing PrdValue");
    }

    #[test]
    fn extract_statemachine() {
        let ref_text = statemachine_types_reference(None);
        assert!(ref_text.contains("StateId"), "Missing StateId");
        assert!(ref_text.contains("TransitionCondition"), "Missing TransitionCondition");
        assert!(ref_text.contains("Transitions"), "Missing Transitions");
    }

    #[test]
    fn extract_entity() {
        let ref_text = entity_types_reference(None);
        assert!(ref_text.contains("EntityId"), "Missing EntityId");
        assert!(ref_text.contains("Assignee"), "Missing Assignee");
        assert!(ref_text.contains("has_pending_invariants"), "Missing method");
    }

    #[test]
    fn all_types_contains_every_module() {
        let ref_text = all_praxis_types_reference(None);
        assert!(ref_text.contains("AtLeastOne"));
        assert!(ref_text.contains("Provenance"));
        assert!(ref_text.contains("Role"));
        assert!(ref_text.contains("State"));
        assert!(ref_text.contains("Entity"));
    }

    #[test]
    fn doc_comments_present() {
        let ref_text = all_praxis_types_reference(None);
        // Inferred variant docs
        assert!(
            ref_text.to_lowercase().contains("inferred") || ref_text.contains("LLM"),
            "Should contain Inferred provenance docs"
        );
    }
}

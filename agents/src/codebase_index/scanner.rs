use std::collections::HashSet;
use std::path::{Path, PathBuf};

use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Node, Parser, Query, QueryCapture, QueryCursor};

use super::index::rust_sources;

// ---------------------------------------------------------------------------
// Public output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceKind {
    /// `pub id: i64` inside the `UserRecord` struct body.
    StructDefinition,
    /// `users::id` inside a Diesel query expression.
    SchemaColumn,
    /// `user.id` on a variable with an explicit `UserRecord` type annotation.
    FieldAccess,
    /// Function that accepts or returns `UserRecord`.
    FunctionSignature,
    /// `UserRecord::as_select()` or `.first::<UserRecord>()` type argument.
    TypeArgument,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FieldReference {
    pub file: PathBuf,
    /// 1-indexed line number.
    pub line: u32,
    /// 1-indexed column number.
    pub column: u32,
    pub kind: ReferenceKind,
    /// Full source line for context.
    pub snippet: String,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn scan(
    root: &Path,
    struct_name: &str,
    field_name: &str,
    table_name: &str,
) -> Result<Vec<FieldReference>, Box<dyn std::error::Error + Send + Sync>> {
    let language: Language = tree_sitter_rust::LANGUAGE.into();
    let mut parser = Parser::new();
    parser.set_language(&language)?;

    // Compile all queries once; reuse across files.
    let q_struct_def = Query::new(&language, &fmt_struct_def(struct_name, field_name))?;
    let q_schema_col = Query::new(&language, &fmt_schema_col(table_name, field_name))?;
    let q_typed_vars = Query::new(&language, TYPED_VARS_QUERY)?;
    let q_field_access = Query::new(&language, &fmt_field_access(field_name))?;
    let q_fn_ret = Query::new(&language, FN_RET_QUERY)?;
    let q_fn_param = Query::new(&language, &fmt_fn_param(struct_name))?;
    let q_type_args = Query::new(&language, TYPE_ARGS_QUERY)?;
    let q_scoped_path = Query::new(&language, SCOPED_PATH_QUERY)?;

    let mut refs: Vec<FieldReference> = Vec::new();

    for path in rust_sources(root) {
        let src = std::fs::read_to_string(&path)?;
        let tree = parser
            .parse(src.as_bytes(), None)
            .ok_or_else(|| format!("tree-sitter: parse failed for {}", path.display()))?;
        let lines: Vec<&str> = src.lines().collect();
        let bytes = src.as_bytes();
        let root_node = tree.root_node();

        // 1. Struct field definition: `pub id: i64` inside `UserRecord { ... }`
        each_match(&q_struct_def, root_node, bytes, |caps| {
            if let Some(n) = find_capture(caps, &q_struct_def, "field") {
                refs.push(make_ref(&path, n, ReferenceKind::StructDefinition, &lines));
            }
        });

        // 2. Schema column: `users::id` in Diesel query builder expressions.
        each_match(&q_schema_col, root_node, bytes, |caps| {
            if let Some(n) = find_capture(caps, &q_schema_col, "col") {
                refs.push(make_ref(&path, n, ReferenceKind::SchemaColumn, &lines));
            }
        });

        // 3. Field access on explicitly typed locals (two-pass).
        //
        //    Pass 1: collect all variable names declared with type `struct_name`
        //            (direct or wrapped in Option<>/Vec<>).
        //    Pass 2: find `.field_name` accesses where the receiver is in that set.
        let typed_vars = collect_typed_vars(&q_typed_vars, root_node, &src, bytes, struct_name);
        if !typed_vars.is_empty() {
            each_match(&q_field_access, root_node, bytes, |caps| {
                let var_node = find_capture(caps, &q_field_access, "var");
                let fld_node = find_capture(caps, &q_field_access, "field");
                if let (Some(var), Some(fld)) = (var_node, fld_node) {
                    if typed_vars.contains(&src[var.byte_range()]) {
                        refs.push(make_ref(&path, fld, ReferenceKind::FieldAccess, &lines));
                    }
                }
            });
        }

        // 4a. Function return types containing `struct_name`.
        each_match(&q_fn_ret, root_node, bytes, |caps| {
            if let Some(ret) = find_capture(caps, &q_fn_ret, "return_type") {
                if src[ret.byte_range()].contains(struct_name) {
                    refs.push(make_ref(&path, ret, ReferenceKind::FunctionSignature, &lines));
                }
            }
        });

        // 4b. Function parameters explicitly typed as `struct_name`.
        each_match(&q_fn_param, root_node, bytes, |caps| {
            if let Some(n) = find_capture(caps, &q_fn_param, "type_name") {
                refs.push(make_ref(&path, n, ReferenceKind::FunctionSignature, &lines));
            }
        });

        // 5. Type arguments: `.first::<UserRecord>()`, `.load::<UserRecord>()`.
        each_match(&q_type_args, root_node, bytes, |caps| {
            if let Some(n) = find_capture(caps, &q_type_args, "type_name") {
                if &src[n.byte_range()] == struct_name {
                    refs.push(make_ref(&path, n, ReferenceKind::TypeArgument, &lines));
                }
            }
        });

        // 6. Scoped struct method calls: `UserRecord::as_select()`, `UserRecord::as_returning()`.
        each_match(&q_scoped_path, root_node, bytes, |caps| {
            if let Some(n) = find_capture(caps, &q_scoped_path, "path") {
                if &src[n.byte_range()] == struct_name {
                    refs.push(make_ref(&path, n, ReferenceKind::TypeArgument, &lines));
                }
            }
        });
    }

    Ok(refs)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Runs `query` over `root` and calls `f` with the captures of each match.
/// Uses `StreamingIterator` (required by tree-sitter 0.24+).
fn each_match<F>(query: &Query, root: Node, bytes: &[u8], mut f: F)
where
    F: FnMut(&[QueryCapture]),
{
    let mut cursor = QueryCursor::new();
    let mut ms = cursor.matches(query, root, bytes);
    while let Some(mat) = ms.next() {
        f(mat.captures);
    }
}

/// Pass 1 of the field-access search: collect all variable names in this file
/// that are explicitly annotated with `struct_name` (or `Option<struct_name>`, etc.).
fn collect_typed_vars(
    query: &Query,
    root: Node,
    src: &str,
    bytes: &[u8],
    struct_name: &str,
) -> HashSet<String> {
    let var_idx = match capture_index(query, "var") {
        Some(i) => i,
        None => return HashSet::new(),
    };
    let type_idx = match capture_index(query, "type") {
        Some(i) => i,
        None => return HashSet::new(),
    };

    let mut vars = HashSet::new();
    let mut cursor = QueryCursor::new();
    let mut ms = cursor.matches(query, root, bytes);
    while let Some(mat) = ms.next() {
        let var_cap = mat.captures.iter().find(|c| c.index == var_idx);
        let type_cap = mat.captures.iter().find(|c| c.index == type_idx);
        if let (Some(var), Some(ty)) = (var_cap, type_cap) {
            if &src[ty.node.byte_range()] == struct_name {
                vars.insert(src[var.node.byte_range()].to_string());
            }
        }
    }
    vars
}

fn find_capture<'a>(caps: &[QueryCapture<'a>], query: &Query, name: &str) -> Option<Node<'a>> {
    let idx = capture_index(query, name)?;
    caps.iter().find(|c| c.index == idx).map(|c| c.node)
}

fn capture_index(query: &Query, name: &str) -> Option<u32> {
    query
        .capture_names()
        .iter()
        .position(|n| *n == name)
        .map(|i| i as u32)
}

fn make_ref(path: &Path, node: Node, kind: ReferenceKind, lines: &[&str]) -> FieldReference {
    let row = node.start_position().row;
    let col = node.start_position().column;
    FieldReference {
        file: path.to_path_buf(),
        line: (row + 1) as u32,
        column: (col + 1) as u32,
        kind,
        snippet: lines.get(row).unwrap_or(&"").to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tree-sitter query strings
// ---------------------------------------------------------------------------

/// Matches the field declaration inside the target struct body.
///
/// Example: `pub id: i64` inside `struct UserRecord { ... }`
fn fmt_struct_def(struct_name: &str, field_name: &str) -> String {
    format!(
        r#"(struct_item
             name: (type_identifier) @struct_name
             body: (field_declaration_list
               (field_declaration
                 name: (field_identifier) @field))
             (#eq? @struct_name "{struct_name}")
             (#eq? @field "{field_name}"))"#
    )
}

/// Matches `table::column` scoped identifiers in Diesel query builder expressions.
///
/// Example: `users::id` in `.filter(users::id.eq(x))` or `.returning(users::id)`.
fn fmt_schema_col(table_name: &str, field_name: &str) -> String {
    format!(
        r#"(scoped_identifier
             path: (identifier) @table
             name: (identifier) @col
             (#eq? @table "{table_name}")
             (#eq? @col "{field_name}"))"#
    )
}

/// Matches `receiver.field` field access expressions.
/// Receiver name is filtered in Rust against the set of typed variables.
///
/// Example: `user.id`, `friend.id`
fn fmt_field_access(field_name: &str) -> String {
    format!(
        r#"(field_expression
             value: (identifier) @var
             field: (field_identifier) @field
             (#eq? @field "{field_name}"))"#
    )
}

/// Matches function parameters explicitly typed as `struct_name`.
///
/// Covers direct (`user: UserRecord`) and generic-wrapped (`user: Option<UserRecord>`).
fn fmt_fn_param(struct_name: &str) -> String {
    format!(
        r#"(function_item
             parameters: (parameters
               (parameter
                 pattern: (identifier) @param_name
                 type: (type_identifier) @type_name))
             (#eq? @type_name "{struct_name}"))

           (function_item
             parameters: (parameters
               (parameter
                 pattern: (identifier) @param_name
                 type: (generic_type
                   type_arguments: (type_arguments
                     (type_identifier) @type_name))))
             (#eq? @type_name "{struct_name}"))"#
    )
}

/// Collects all explicitly typed variable and parameter declarations.
/// Both the variable name (`@var`) and its type (`@type`) are captured per match.
/// Filtered in Rust: keep only where `@type == struct_name`.
///
/// Handles: direct types and one level of generic wrapping (Option<T>, Vec<T>).
const TYPED_VARS_QUERY: &str = r#"
    (parameter
      pattern: (identifier) @var
      type: (type_identifier) @type)
    (let_declaration
      pattern: (identifier) @var
      type: (type_identifier) @type)
    (parameter
      pattern: (identifier) @var
      type: (generic_type
        type_arguments: (type_arguments
          (type_identifier) @type)))
    (let_declaration
      pattern: (identifier) @var
      type: (generic_type
        type_arguments: (type_arguments
          (type_identifier) @type)))
"#;

/// Captures the return type of every function that has one.
/// Filtered in Rust: keep only where the return type text contains `struct_name`.
///
/// Example: `-> Result<Option<UserRecord>, Error>`
const FN_RET_QUERY: &str = r#"
    (function_item
      return_type: (_) @return_type)
"#;

/// Captures every `TypeIdentifier` inside a type argument list.
/// Filtered in Rust: keep only where the text equals `struct_name`.
///
/// Example: `.first::<UserRecord>()`, `.load::<UserRecord>()`
const TYPE_ARGS_QUERY: &str = r#"
    (type_arguments
      (type_identifier) @type_name)
"#;

/// Captures the path part of scoped identifiers in expression position.
/// Filtered in Rust: keep only where the path equals `struct_name`.
///
/// Example: `UserRecord::as_select()`, `UserRecord::as_returning()`
const SCOPED_PATH_QUERY: &str = r#"
    (scoped_identifier
      path: (identifier) @path
      name: (identifier) @method)
"#;

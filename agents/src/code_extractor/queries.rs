// Tree-sitter S-expression queries for Rust code extraction.
//
// Each query captures named nodes that the extractor uses to locate
// struct definitions, free functions, and impl methods.

// ---------------------------------------------------------------------------
// Struct definitions
// ---------------------------------------------------------------------------

/// Matches `struct Foo { ... }` or `pub struct Foo { ... }`.
/// Captures: @name (type_identifier), @item (the full struct_item node).
pub const STRUCT_DEF: &str = r#"
    (struct_item
      name: (type_identifier) @name) @item
"#;

// ---------------------------------------------------------------------------
// Impl blocks with methods
// ---------------------------------------------------------------------------

/// Matches `impl Foo { fn bar() { ... } }` and `impl Foo { ... }`.
/// Captures: @struct_name (type_identifier), @item (the full impl_item).
pub const IMPL_BLOCK: &str = r#"
    (impl_item
      type: (type_identifier) @struct_name) @item
"#;

/// Matches a function inside an impl block.
/// Captures: @fn_name (identifier), @item (the full function_item node).
/// Used as a secondary pass inside an impl_item's declaration_list.
pub const IMPL_FN: &str = r#"
    (function_item
      name: (identifier) @fn_name) @item
"#;

// ---------------------------------------------------------------------------
// Free functions (top-level, not inside impl/trait/mod)
// ---------------------------------------------------------------------------

/// Matches any function_item. The extractor filters to keep only those
/// whose parent is `source_file` (i.e. not nested in impl/trait/mod).
pub const ANY_FN: &str = r#"
    (function_item
      name: (identifier) @fn_name) @item
"#;

// ---------------------------------------------------------------------------
// Enum definitions
// ---------------------------------------------------------------------------

/// Matches `enum Foo { ... }` or `pub enum Foo { ... }`.
/// Captures: @name (type_identifier), @item (the full enum_item node).
pub const ENUM_DEF: &str = r#"
    (enum_item
      name: (type_identifier) @name) @item
"#;

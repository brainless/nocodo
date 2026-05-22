mod index;
mod scanner;

use std::path::Path;

pub use scanner::{FieldReference, ReferenceKind};

/// Find all references to `struct_name.field_name` across the Rust codebase at `root`.
///
/// Requires:
/// - `struct_name` has a `#[diesel(table_name = ...)]` attribute mapping it to a DB table.
/// - All variables holding `struct_name` values carry explicit type annotations.
pub fn find_field_references(
    root: &Path,
    struct_name: &str,
    field_name: &str,
) -> Result<Vec<FieldReference>, Box<dyn std::error::Error + Send + Sync>> {
    let idx = index::build(root)?;
    let table_name = idx
        .struct_to_table
        .get(struct_name)
        .ok_or_else(|| format!("no #[diesel(table_name = ...)] found for '{struct_name}'"))?;
    scanner::scan(root, struct_name, field_name, table_name)
}

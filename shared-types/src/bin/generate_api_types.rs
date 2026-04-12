use shared_types::{
    ColumnType, GetSheetResponse, GetSheetTabDataRequest, GetSheetTabDataResponse,
    GetSheetTabSchemaRequest, GetSheetTabSchemaResponse, HeartbeatResponse, ListSheetsRequest,
    ListSheetsResponse, Sheet, SheetTab, SheetTabColumn, SheetTabRow,
};
use std::fs;
use std::path::{Path, PathBuf};
use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut types = Vec::new();

    // Helper macro to export types with error handling
    macro_rules! export_type {
        ($t:ty) => {
            match <$t>::export_to_string() {
                Ok(s) => types.push(clean_type(s)),
                Err(e) => eprintln!("Warning: Failed to export {}: {}", stringify!($t), e),
            }
        };
    }

    // Core types
    export_type!(Sheet);
    export_type!(SheetTab);
    export_type!(SheetTabColumn);
    export_type!(SheetTabRow);
    export_type!(ColumnType);

    // API request/response types
    export_type!(ListSheetsRequest);
    export_type!(ListSheetsResponse);
    export_type!(GetSheetResponse);
    export_type!(GetSheetTabSchemaRequest);
    export_type!(GetSheetTabSchemaResponse);
    export_type!(GetSheetTabDataRequest);
    export_type!(GetSheetTabDataResponse);

    // Legacy types
    export_type!(HeartbeatResponse);

    let output_content = types.join("\n\n");

    // Get the workspace root (parent of shared-types directory)
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap());
    let workspace_root = manifest_dir.parent().unwrap_or(&manifest_dir);

    // Generate for gui
    let output_dir = workspace_root.join("gui/src/types");
    fs::create_dir_all(&output_dir)?;
    let output_path = output_dir.join("api.ts");
    fs::write(&output_path, &output_content)?;
    println!("Generated TypeScript types in {}", output_path.display());

    // Generate for admin-gui
    let admin_output_dir = workspace_root.join("admin-gui/src/types");
    fs::create_dir_all(&admin_output_dir)?;
    let admin_output_path = admin_output_dir.join("api.ts");
    fs::write(&admin_output_path, &output_content)?;
    println!(
        "Generated TypeScript types in {}",
        admin_output_path.display()
    );

    Ok(())
}

fn clean_type(mut type_def: String) -> String {
    type_def.retain(|c| c != '\r');
    let lines: Vec<&str> = type_def.lines().collect();

    let filtered: Vec<&str> = lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("import type")
                && !trimmed.starts_with("// This file was generated")
                && !trimmed.starts_with("/* This file was generated")
        })
        .copied()
        .collect();

    let result = filtered.join("\n").trim().to_string();
    if result.is_empty() {
        result
    } else {
        format!("{}\n", result)
    }
}

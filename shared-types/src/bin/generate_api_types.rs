use shared_types::{
    Column, ColumnDef, ColumnDisplay, CreateProjectRequest, CreateProjectResponse, DataType,
    EpicItem, ForeignKey, ForeignKeyDef, GetSchemaResponse, GetTableColumnsResponse,
    GetTableDataResponse, HeartbeatResponse, ListEpicsResponse, ListProjectsResponse,
    ListSchemasResponse, ListTasksResponse, PaginationInfo, Project, Schema, SchemaDef, Table,
    TableDataResult, TableDef, TaskItem,
};
use std::fs;
use std::path::PathBuf;
use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut types = Vec::new();

    macro_rules! export_type {
        ($t:ty) => {
            match <$t>::export_to_string() {
                Ok(s) => types.push(clean_type(s)),
                Err(e) => eprintln!("Warning: Failed to export {}: {}", stringify!($t), e),
            }
        };
    }

    // Core project type
    export_type!(Project);

    // Core relational types (persisted)
    export_type!(Schema);
    export_type!(Table);
    export_type!(DataType);
    export_type!(Column);
    export_type!(ForeignKey);
    export_type!(ColumnDisplay);

    // Agent definition types
    export_type!(ForeignKeyDef);
    export_type!(ColumnDef);
    export_type!(TableDef);
    export_type!(SchemaDef);

    // Schema API response types
    export_type!(ListSchemasResponse);
    export_type!(GetSchemaResponse);
    export_type!(GetTableColumnsResponse);
    export_type!(PaginationInfo);
    export_type!(TableDataResult);
    export_type!(GetTableDataResponse);

    // Project API types
    export_type!(CreateProjectRequest);
    export_type!(CreateProjectResponse);
    export_type!(ListProjectsResponse);

    // Misc
    export_type!(HeartbeatResponse);

    // Agent Task/Epic API types
    export_type!(TaskItem);
    export_type!(ListTasksResponse);
    export_type!(EpicItem);
    export_type!(ListEpicsResponse);

    let output_content = types.join("\n\n");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap());
    let workspace_root = manifest_dir.parent().unwrap_or(&manifest_dir);

    // gui
    let output_dir = workspace_root.join("gui/src/types");
    fs::create_dir_all(&output_dir)?;
    let output_path = output_dir.join("api.ts");
    fs::write(&output_path, &output_content)?;
    println!("Generated TypeScript types in {}", output_path.display());

    // admin-gui
    let admin_output_dir = workspace_root.join("admin-gui/src/types");
    fs::create_dir_all(&admin_output_dir)?;
    let admin_output_path = admin_output_dir.join("api.ts");
    fs::write(&admin_output_path, &output_content)?;
    println!("Generated TypeScript types in {}", admin_output_path.display());

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

use shared_types::HeartbeatResponse;
use std::fs;
use std::path::Path;
use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut types = Vec::new();
    types.push(clean_type(HeartbeatResponse::export_to_string()?));

    let output_dir = Path::new("../gui/src/types");
    fs::create_dir_all(output_dir)?;

    let output_path = output_dir.join("api.ts");
    fs::write(&output_path, types.join("\n\n"))?;

    println!("Generated TypeScript types in {}", output_path.display());
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

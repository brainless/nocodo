use shared_types::*;
use std::fs;
use std::path::Path;
use tempfile::NamedTempFile;
use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate TypeScript definitions for API types
    let mut types = Vec::new();

    let temp_file = NamedTempFile::new()?;
    let path = temp_file.path();
    AgentInfo::export_to(path)?;
    types.push(fs::read_to_string(path)?);

    let temp_file = NamedTempFile::new()?;
    let path = temp_file.path();
    SqliteAgentConfig::export_to(path)?;
    types.push(fs::read_to_string(path)?);

    let temp_file = NamedTempFile::new()?;
    let path = temp_file.path();
    CodebaseAnalysisAgentConfig::export_to(path)?;
    types.push(fs::read_to_string(path)?);

    let temp_file = NamedTempFile::new()?;
    let path = temp_file.path();
    AgentConfig::export_to(path)?;
    types.push(fs::read_to_string(path)?);

    let temp_file = NamedTempFile::new()?;
    let path = temp_file.path();
    AgentExecutionRequest::export_to(path)?;
    types.push(fs::read_to_string(path)?);

    let temp_file = NamedTempFile::new()?;
    let path = temp_file.path();
    AgentsResponse::export_to(path)?;
    types.push(fs::read_to_string(path)?);

    let temp_file = NamedTempFile::new()?;
    let path = temp_file.path();
    AgentExecutionResponse::export_to(path)?;
    types.push(fs::read_to_string(path)?);

    let temp_file = NamedTempFile::new()?;
    let path = temp_file.path();
    ErrorResponse::export_to(path)?;
    types.push(fs::read_to_string(path)?);

    let output_dir = Path::new("../gui/api-types");
    fs::create_dir_all(output_dir)?;

    let output_path = output_dir.join("types.ts");
    let output = types.join("\n\n");

    fs::write(&output_path, output)?;
    println!("Generated TypeScript types in {}", output_path.display());

    Ok(())
}

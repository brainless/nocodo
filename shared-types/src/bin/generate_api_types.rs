use shared_types::*;
use std::fs;
use std::path::Path;
use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate TypeScript definitions for API types
    let mut types = Vec::new();

    types.push(clean_type(AgentInfo::export_to_string()?));
    types.push(clean_type(SqliteAgentConfig::export_to_string()?));
    types.push(clean_type(CodebaseAnalysisAgentConfig::export_to_string()?));
    types.push(clean_type(TesseractAgentConfig::export_to_string()?));
    types.push(clean_type(StructuredJsonAgentConfig::export_to_string()?));
    types.push(clean_type(AgentConfig::export_to_string()?));
    types.push(clean_type(AgentExecutionRequest::export_to_string()?));
    types.push(clean_type(AgentsResponse::export_to_string()?));
    types.push(clean_type(AgentExecutionResponse::export_to_string()?));
    types.push(clean_type(SessionMessage::export_to_string()?));
    types.push(clean_type(SessionToolCall::export_to_string()?));
    types.push(clean_type(SessionResponse::export_to_string()?));
    types.push(clean_type(SessionListItem::export_to_string()?));
    types.push(clean_type(SessionListResponse::export_to_string()?));
    types.push(clean_type(ApiKeyConfig::export_to_string()?));
    types.push(clean_type(SettingsResponse::export_to_string()?));
    types.push(clean_type(UpdateApiKeysRequest::export_to_string()?));
    types.push(clean_type(PMProject::export_to_string()?));
    types.push(clean_type(Workflow::export_to_string()?));
    types.push(clean_type(WorkflowStep::export_to_string()?));
    types.push(clean_type(WorkflowWithSteps::export_to_string()?));
    types.push(clean_type(SaveWorkflowRequest::export_to_string()?));
    types.push(clean_type(WorkflowStepData::export_to_string()?));

    let output_dir = Path::new("../gui/api-types");
    fs::create_dir_all(output_dir)?;

    let output_path = output_dir.join("types.ts");
    let output = types.join("\n\n");

    fs::write(&output_path, output)?;
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
        })
        .cloned()
        .collect();

    let result = filtered.join("\n").trim().to_string();
    if result.is_empty() {
        result
    } else {
        format!("{}\n", result)
    }
}

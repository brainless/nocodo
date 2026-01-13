pub fn generate_typescript_definitions(
    type_names: &[&str],
) -> Result<String, Box<dyn std::error::Error>> {
    if type_names.is_empty() {
        return Err("No type names provided".into());
    }

    let mut definitions = Vec::new();

    for name in type_names {
        let type_def = export_type(name)?;
        let cleaned = clean_type(type_def);

        if !cleaned.trim().is_empty() {
            definitions.push(cleaned);
        }
    }

    Ok(definitions.join("\n\n"))
}

fn export_type(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    use crate::*;

    let result = match name {
        "AgentInfo" => AgentInfo::export_to_string()?,
        "AgentConfig" => AgentConfig::export_to_string()?,
        "SqliteAgentConfig" => SqliteAgentConfig::export_to_string()?,
        "CodebaseAnalysisAgentConfig" => CodebaseAnalysisAgentConfig::export_to_string()?,
        "TesseractAgentConfig" => TesseractAgentConfig::export_to_string()?,
        "StructuredJsonAgentConfig" => StructuredJsonAgentConfig::export_to_string()?,
        "AgentExecutionRequest" => AgentExecutionRequest::export_to_string()?,
        "AgentsResponse" => AgentsResponse::export_to_string()?,

        "SessionMessage" => SessionMessage::export_to_string()?,
        "SessionToolCall" => SessionToolCall::export_to_string()?,
        "SessionResponse" => SessionResponse::export_to_string()?,
        "SessionListItem" => SessionListItem::export_to_string()?,
        "SessionListResponse" => SessionListResponse::export_to_string()?,
        "AgentExecutionResponse" => AgentExecutionResponse::export_to_string()?,

        "PMProject" | "Project" => {
            let mut result = PMProject::export_to_string()?;
            result = result.replace("export type Project", "export type PMProject");
            result
        }
        "Workflow" => Workflow::export_to_string()?,
        "WorkflowStep" => WorkflowStep::export_to_string()?,
        "WorkflowWithSteps" => WorkflowWithSteps::export_to_string()?,
        "SaveWorkflowRequest" => SaveWorkflowRequest::export_to_string()?,
        "WorkflowStepData" => WorkflowStepData::export_to_string()?,

        "ErrorResponse" => ErrorResponse::export_to_string()?,

        "ApiKeyConfig" => ApiKeyConfig::export_to_string()?,
        "SettingsResponse" => SettingsResponse::export_to_string()?,
        "UpdateApiKeysRequest" => UpdateApiKeysRequest::export_to_string()?,

        _ => {
            return Err(format!(
                "Unknown type: '{}'. Available types can be found in shared-types/src/",
                name
            )
            .into());
        }
    };

    Ok(result)
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

    filtered.join("\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_single_type() {
        let result = generate_typescript_definitions(&["PMProject"]).unwrap();
        assert!(result.contains("PMProject"));
        assert!(result.contains("id: number"));
    }

    #[test]
    fn test_generate_multiple_types() {
        let result = generate_typescript_definitions(&["PMProject", "Workflow"]).unwrap();
        assert!(result.contains("PMProject"));
        assert!(result.contains("Workflow"));
    }

    #[test]
    fn test_unknown_type_error() {
        let result = generate_typescript_definitions(&["NonExistentType"]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown type"));
    }

    #[test]
    fn test_empty_type_names() {
        let result = generate_typescript_definitions(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cleaned_output() {
        let result = generate_typescript_definitions(&["PMProject"]).unwrap();
        assert!(!result.contains("import type"));
        assert!(!result.contains("This file was generated"));
    }
}

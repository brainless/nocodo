use nocodo_llm_sdk::{
    claude::client::ClaudeClient,
    glm::{zai::ZaiGlmClient, zen::ZenGlmClient},
    grok::zen::ZenGrokClient,
};

/// Get LLM response from the selected provider
///
/// This function handles provider selection and API calls for all supported providers.
/// The provider is determined by the WORKFLOW_PROVIDER environment variable.
pub async fn get_llm_response(system_prompt: &str, user_prompt: &str) -> Result<String, String> {
    let provider = std::env::var("WORKFLOW_PROVIDER").unwrap_or_else(|_| "zen_glm".to_string());

    match provider.as_str() {
        "zai_glm" => get_zai_glm_response(system_prompt, user_prompt).await,
        "zen_glm" => get_zen_glm_response(system_prompt, user_prompt).await,
        "zen_grok" => get_zen_grok_response(system_prompt, user_prompt).await,
        "anthropic_claude" => get_anthropic_response(system_prompt, user_prompt).await,
        _ => Err(format!("Unknown provider: {}", provider)),
    }
}

/// Get response from z.ai GLM provider
async fn get_zai_glm_response(system_prompt: &str, user_prompt: &str) -> Result<String, String> {
    let api_key = std::env::var("ZAI_API_KEY")
        .map_err(|_| "ZAI_API_KEY must be set for zai_glm provider".to_string())?;

    // Check if coding plan is enabled
    let has_coding_plan = std::env::var("ZAI_CODING_PLAN")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    let client = ZaiGlmClient::with_coding_plan(api_key, has_coding_plan)
        .map_err(|e| format!("Failed to create z.ai GLM client: {}", e))?;

    let response = client
        .message_builder()
        .model("glm-4.6")
        .max_tokens(2000)
        .system_message(system_prompt)
        .user_message(user_prompt)
        .temperature(0.7)
        .send()
        .await
        .map_err(|e| format!("Failed to get response from z.ai GLM: {}", e))?;

    if response.choices.is_empty() {
        return Err("Response has no choices".to_string());
    }

    Ok(response.choices[0].message.get_text())
}

/// Get response from Zen GLM provider (free)
async fn get_zen_glm_response(system_prompt: &str, user_prompt: &str) -> Result<String, String> {
    let client =
        ZenGlmClient::new().map_err(|e| format!("Failed to create Zen GLM client: {}", e))?;

    let response = client
        .message_builder()
        .model("big-pickle")
        .max_tokens(2000)
        .system_message(system_prompt)
        .user_message(user_prompt)
        .temperature(0.7)
        .send()
        .await
        .map_err(|e| format!("Failed to get response from Zen GLM: {}", e))?;

    if response.choices.is_empty() {
        return Err("Response has no choices".to_string());
    }

    Ok(response.choices[0].message.get_text())
}

/// Get response from Zen Grok provider (free)
async fn get_zen_grok_response(system_prompt: &str, user_prompt: &str) -> Result<String, String> {
    let client =
        ZenGrokClient::new().map_err(|e| format!("Failed to create Zen Grok client: {}", e))?;

    let response = client
        .message_builder()
        .model("grok-code")
        .max_tokens(2000)
        .system_message(system_prompt)
        .user_message(user_prompt)
        .temperature(0.7)
        .send()
        .await
        .map_err(|e| format!("Failed to get response from Zen Grok: {}", e))?;

    if response.choices.is_empty() {
        return Err("Response has no choices".to_string());
    }

    Ok(response.choices[0].message.content.clone())
}

/// Get response from Anthropic Claude provider
async fn get_anthropic_response(system_prompt: &str, user_prompt: &str) -> Result<String, String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| "ANTHROPIC_API_KEY must be set for anthropic_claude provider".to_string())?;

    let client =
        ClaudeClient::new(api_key).map_err(|e| format!("Failed to create Claude client: {}", e))?;

    let response = client
        .message_builder()
        .model("claude-sonnet-4-5-20250929")
        .max_tokens(2000)
        .system(system_prompt)
        .user_message(user_prompt)
        .send()
        .await
        .map_err(|e| format!("Failed to get response from Claude: {}", e))?;

    if response.content.is_empty() {
        return Err("Response has no content".to_string());
    }

    match &response.content[0] {
        nocodo_llm_sdk::claude::types::ClaudeContentBlock::Text { text } => Ok(text.clone()),
        nocodo_llm_sdk::claude::types::ClaudeContentBlock::ToolUse { .. } => {
            Err("Unexpected tool use in response".to_string())
        }
    }
}

/// Extract JSON from response text, handling markdown code blocks
///
/// This function handles responses that may be wrapped in markdown code blocks:
/// - ```json ... ```
/// - ``` ... ```
/// - Plain JSON
pub fn extract_json(response_text: &str) -> &str {
    if response_text.contains("```json") {
        // Extract JSON from markdown code block with language specifier
        if let Some(start_idx) = response_text.find("```json") {
            let start = start_idx + 7; // "```json".len()
            if let Some(end_idx) = response_text[start..].find("```") {
                return response_text[start..start + end_idx].trim();
            }
        }
    } else if response_text.contains("```") {
        // Extract from generic code block
        if let Some(start_idx) = response_text.find("```") {
            let start = start_idx + 3;
            if let Some(end_idx) = response_text[start..].find("```") {
                return response_text[start..start + end_idx].trim();
            }
        }
    }

    response_text.trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_markdown_with_language() {
        let response = r#"Here's the response:
```json
{"questions": ["What do you need?"]}
```
"#;
        let json = extract_json(response);
        assert_eq!(json, r#"{"questions": ["What do you need?"]}"#);
    }

    #[test]
    fn test_extract_json_from_markdown_without_language() {
        let response = r#"```
{"questions": ["What do you need?"]}
```"#;
        let json = extract_json(response);
        assert_eq!(json, r#"{"questions": ["What do you need?"]}"#);
    }

    #[test]
    fn test_extract_json_plain() {
        let response = r#"{"questions": ["What do you need?"]}"#;
        let json = extract_json(response);
        assert_eq!(json, r#"{"questions": ["What do you need?"]}"#);
    }

    #[test]
    fn test_extract_json_with_whitespace() {
        let response = r#"
        {"questions": ["What do you need?"]}
        "#;
        let json = extract_json(response);
        assert_eq!(json, r#"{"questions": ["What do you need?"]}"#);
    }

    #[test]
    fn test_extract_json_markdown_multiline() {
        let response = r#"```json
{
  "questions": ["What do you need?"],
  "inputs": []
}
```"#;
        let json = extract_json(response);
        let expected = r#"{
  "questions": ["What do you need?"],
  "inputs": []
}"#;
        assert_eq!(json, expected);
    }
}

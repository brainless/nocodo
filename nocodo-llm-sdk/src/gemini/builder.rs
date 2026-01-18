use super::client::GeminiClient;
use super::types::*;
use crate::error::LlmError;

pub struct MessageBuilder<'a> {
    client: &'a GeminiClient,
    model: Option<String>,
    contents: Vec<GeminiContent>,
    system_instruction: Option<String>,
    tools: Vec<GeminiTool>,
    tool_config: Option<GeminiToolConfig>,
    generation_config: GenerationConfig,
}

impl<'a> MessageBuilder<'a> {
    pub fn new(client: &'a GeminiClient) -> Self {
        Self {
            client,
            model: None,
            contents: Vec::new(),
            system_instruction: None,
            tools: Vec::new(),
            tool_config: None,
            generation_config: GenerationConfig::default(),
        }
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn user_message(mut self, text: impl Into<String>) -> Self {
        self.contents.push(GeminiContent {
            role: GeminiRole::User,
            parts: vec![GeminiPart {
                text: Some(text.into()),
                ..Default::default()
            }],
        });
        self
    }

    pub fn model_message(mut self, text: impl Into<String>) -> Self {
        self.contents.push(GeminiContent {
            role: GeminiRole::Model,
            parts: vec![GeminiPart {
                text: Some(text.into()),
                ..Default::default()
            }],
        });
        self
    }

    pub fn content(mut self, content: GeminiContent) -> Self {
        self.contents.push(content);
        self
    }

    pub fn system(mut self, text: impl Into<String>) -> Self {
        self.system_instruction = Some(text.into());
        self
    }

    pub fn thinking_level(mut self, level: impl Into<String>) -> Self {
        self.generation_config.thinking_config = Some(ThinkingConfig {
            thinking_level: level.into(),
        });
        self
    }

    pub fn temperature(mut self, temp: f32) -> Self {
        self.generation_config.temperature = Some(temp);
        self
    }

    pub fn max_output_tokens(mut self, tokens: u32) -> Self {
        self.generation_config.max_output_tokens = Some(tokens);
        self
    }

    pub fn top_p(mut self, top_p: f32) -> Self {
        self.generation_config.top_p = Some(top_p);
        self
    }

    pub fn top_k(mut self, top_k: u32) -> Self {
        self.generation_config.top_k = Some(top_k);
        self
    }

    pub fn tool(mut self, tool: GeminiTool) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn tool_config(mut self, config: GeminiToolConfig) -> Self {
        self.tool_config = Some(config);
        self
    }

    pub async fn send(self) -> Result<GeminiGenerateContentResponse, LlmError> {
        let model = self
            .model
            .ok_or_else(|| LlmError::invalid_request("Model is required"))?;

        if self.contents.is_empty() {
            return Err(LlmError::invalid_request(
                "At least one message is required",
            ));
        }

        let request = GeminiGenerateContentRequest {
            contents: self.contents,
            system_instruction: self.system_instruction.map(|text| GeminiContent {
                role: GeminiRole::User,
                parts: vec![GeminiPart {
                    text: Some(text),
                    ..Default::default()
                }],
            }),
            tools: if self.tools.is_empty() {
                None
            } else {
                Some(self.tools)
            },
            tool_config: self.tool_config,
            generation_config: Some(self.generation_config),
        };

        self.client.generate_content(model, request).await
    }
}

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Ask the user a structured question with predefined choices.
/// Prefer this over writing options inline in prose so the UI can render
/// radio buttons or checkboxes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RequestUserInputParams {
    /// The question to ask.
    pub question: String,
    /// "single_choice" for one answer (radio), "multiple_choice" for many (checkboxes).
    pub input_type: InputType,
    /// 2–6 concise options for the user to pick from.
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InputType {
    SingleChoice,
    MultipleChoice,
}

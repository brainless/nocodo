use serde::{Deserialize, Serialize};

/// A question the PM/PO poses to the user with predefined answer choices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredQuestion {
    pub question: String,
    pub kind: QuestionKind,
}

/// The user's answer to a StructuredQuestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredResponse {
    /// ID of the user_chat_message that holds the matching StructuredQuestion.
    pub question_message_id: i64,
    pub selected: Vec<String>,
}

/// Discriminates how the user picks an answer. New variants (Rating, Scale, …) go here.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QuestionKind {
    SingleChoice { options: Vec<String> },
    MultipleChoice { options: Vec<String> },
}

/// Typed representation of a `user_chat_message` row's `(content_type, content)` pair.
/// `Text` is stored as a plain string; structured variants are stored as JSON.
pub enum MessageContent {
    Text(String),
    StructuredQuestion(StructuredQuestion),
    StructuredResponse(StructuredResponse),
}

impl MessageContent {
    pub fn content_type_str(&self) -> &'static str {
        match self {
            MessageContent::Text(_) => "text",
            MessageContent::StructuredQuestion(_) => "structured_question",
            MessageContent::StructuredResponse(_) => "structured_response",
        }
    }

    /// Returns the raw string written to the `content` column.
    pub fn to_storage_content(&self) -> String {
        match self {
            MessageContent::Text(s) => s.clone(),
            MessageContent::StructuredQuestion(q) => {
                serde_json::to_string(q).unwrap_or_default()
            }
            MessageContent::StructuredResponse(r) => {
                serde_json::to_string(r).unwrap_or_default()
            }
        }
    }

    /// Compact plain-text form sent to the LLM in chat history.
    pub fn to_llm_text(&self) -> String {
        match self {
            MessageContent::Text(s) => s.clone(),
            MessageContent::StructuredQuestion(q) => {
                let options = match &q.kind {
                    QuestionKind::SingleChoice { options }
                    | QuestionKind::MultipleChoice { options } => options.join(", "),
                };
                let hint = match &q.kind {
                    QuestionKind::SingleChoice { .. } => "pick one",
                    QuestionKind::MultipleChoice { .. } => "pick all that apply",
                };
                format!("{} ({}): {}", q.question, hint, options)
            }
            MessageContent::StructuredResponse(r) => {
                format!("Selected: {}", r.selected.join(", "))
            }
        }
    }

    /// Reconstruct from the two columns stored in the DB.
    pub fn from_row(content_type: &str, content: &str) -> Self {
        match content_type {
            "structured_question" => serde_json::from_str::<StructuredQuestion>(content)
                .map(MessageContent::StructuredQuestion)
                .unwrap_or_else(|_| MessageContent::Text(content.to_string())),
            "structured_response" => serde_json::from_str::<StructuredResponse>(content)
                .map(MessageContent::StructuredResponse)
                .unwrap_or_else(|_| MessageContent::Text(content.to_string())),
            _ => MessageContent::Text(content.to_string()),
        }
    }
}

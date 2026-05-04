pub mod agent;
pub mod prompts;
pub mod tools;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Top-level form definition for one entity.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FormLayout {
    /// The entity (table) name this form is for.
    pub entity: String,
    /// Human-readable title shown at the top of the form.
    pub title: String,
    /// Ordered list of rows. Each row renders as a flex row in the canvas.
    pub rows: Vec<FormRow>,
}

/// One horizontal band in the form. A row with one field is full-width.
/// A row with two fields renders them side-by-side.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FormRow {
    pub fields: Vec<FormField>,
}

/// A single input within a row.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FormField {
    /// Column name (snake_case, matches the DB column).
    pub name: String,
    /// Human-readable label shown above the input.
    pub label: String,
    pub field_type: FormFieldType,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FormFieldType {
    Text,
    Number,
    Boolean,
    Date,
    Select,
    Textarea,
}

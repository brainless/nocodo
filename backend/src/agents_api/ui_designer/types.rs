use nocodo_agents::{FormField, FormFieldType, FormLayout, FormRow};
use serde::{Deserialize, Serialize};

/// POST /api/agents/ui-designer/form
#[derive(Debug, Deserialize)]
pub struct GenerateFormRequest {
    pub project_id: i64,
    pub entity_name: String,
}

/// Returned when a form layout is available immediately (cache hit) or after generation.
#[derive(Debug, Serialize)]
pub struct FormLayoutResponse {
    pub entity_name: String,
    pub layout: FormLayoutJson,
}

/// Wire-format mirror of FormLayout for the frontend.
#[derive(Debug, Serialize)]
pub struct FormLayoutJson {
    pub entity: String,
    pub title: String,
    pub rows: Vec<FormRowJson>,
}

#[derive(Debug, Serialize)]
pub struct FormRowJson {
    pub fields: Vec<FormFieldJson>,
}

#[derive(Debug, Serialize)]
pub struct FormFieldJson {
    pub name: String,
    pub label: String,
    pub field_type: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
}

impl From<FormLayout> for FormLayoutJson {
    fn from(f: FormLayout) -> Self {
        Self {
            entity: f.entity,
            title: f.title,
            rows: f.rows.into_iter().map(FormRowJson::from).collect(),
        }
    }
}

impl From<FormRow> for FormRowJson {
    fn from(r: FormRow) -> Self {
        Self {
            fields: r.fields.into_iter().map(FormFieldJson::from).collect(),
        }
    }
}

impl From<FormField> for FormFieldJson {
    fn from(f: FormField) -> Self {
        Self {
            name: f.name,
            label: f.label,
            field_type: field_type_str(&f.field_type).to_string(),
            required: f.required,
            placeholder: f.placeholder,
        }
    }
}

fn field_type_str(ft: &FormFieldType) -> &'static str {
    match ft {
        FormFieldType::Text => "text",
        FormFieldType::Number => "number",
        FormFieldType::Boolean => "boolean",
        FormFieldType::Date => "date",
        FormFieldType::Select => "select",
        FormFieldType::Textarea => "textarea",
    }
}

/// Returned when the form doesn't exist yet — client should poll.
#[derive(Debug, Serialize)]
pub struct GenerateFormQueued {
    pub task_id: i64,
}

/// GET /api/agents/ui-designer/forms/{project_id}
#[derive(Debug, Serialize)]
pub struct ListFormsResponse {
    pub forms: Vec<FormLayoutResponse>,
}

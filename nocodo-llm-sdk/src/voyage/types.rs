use serde::{Deserialize, Serialize};

/// Input type for embedding requests
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VoyageInputType {
    /// Query input - prepends "Represent the query for retrieving supporting documents: "
    Query,
    /// Document input - prepends "Represent the document for retrieval: "
    Document,
}

/// Output data type for embeddings
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VoyageOutputDtype {
    /// 32-bit floating-point numbers (default, highest precision)
    Float,
    /// 8-bit signed integers (-128 to 127)
    Int8,
    /// 8-bit unsigned integers (0 to 255)
    Uint8,
    /// Bit-packed binary embedding with offset binary (int8)
    Binary,
    /// Bit-packed binary embedding (uint8)
    Ubinary,
}

impl Default for VoyageOutputDtype {
    fn default() -> Self {
        Self::Float
    }
}

/// Encoding format for embeddings
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VoyageEncodingFormat {
    /// Base64-encoded NumPy array
    Base64,
}

/// Input for embedding request - can be a single string or array of strings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VoyageInput {
    /// Single text string
    Single(String),
    /// Multiple text strings
    Multiple(Vec<String>),
}

impl From<String> for VoyageInput {
    fn from(s: String) -> Self {
        VoyageInput::Single(s)
    }
}

impl From<&str> for VoyageInput {
    fn from(s: &str) -> Self {
        VoyageInput::Single(s.to_string())
    }
}

impl From<Vec<String>> for VoyageInput {
    fn from(v: Vec<String>) -> Self {
        VoyageInput::Multiple(v)
    }
}

impl From<Vec<&str>> for VoyageInput {
    fn from(v: Vec<&str>) -> Self {
        VoyageInput::Multiple(v.iter().map(|s| s.to_string()).collect())
    }
}

/// Request for creating embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoyageEmbeddingRequest {
    /// A single text string or a list of texts
    /// Max 1000 items, max 1M tokens for voyage-4-lite/3.5-lite, 320K for voyage-4/3.5/2, 120K for others
    pub input: VoyageInput,

    /// Name of the model (e.g., "voyage-4-lite")
    pub model: String,

    /// Type of input text (query, document, or null for direct conversion)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_type: Option<VoyageInputType>,

    /// Whether to truncate input texts to fit context length (default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation: Option<bool>,

    /// Output embedding dimensions (256, 512, 1024 (default), or 2048)
    /// Only supported by voyage-4-large, voyage-4, voyage-4-lite, voyage-3-large, voyage-3.5, voyage-3.5-lite, voyage-code-3
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dimension: Option<u32>,

    /// Output data type (float, int8, uint8, binary, ubinary)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dtype: Option<VoyageOutputDtype>,

    /// Encoding format (null or base64)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<VoyageEncodingFormat>,
}

/// Single embedding object in the response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoyageEmbedding {
    /// Always "embedding"
    pub object: String,

    /// The embedding vector (array of numbers)
    pub embedding: Vec<f32>,

    /// Index of this embedding in the list
    pub index: usize,
}

/// Token usage information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoyageUsage {
    /// Total number of tokens used for computing embeddings
    pub total_tokens: u32,
}

/// Response from the embeddings API
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoyageEmbeddingResponse {
    /// Always "list"
    pub object: String,

    /// Array of embedding objects
    pub data: Vec<VoyageEmbedding>,

    /// Name of the model used
    pub model: String,

    /// Token usage information
    pub usage: VoyageUsage,
}

/// Error response from Voyage API
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoyageErrorResponse {
    /// Error message
    pub detail: String,
}

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Request to extract text from a PDF file using pdftotext
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PdfToTextRequest {
    /// Path to the PDF file to extract text from
    pub file_path: String,

    /// Optional output file path. If not specified, output is returned in response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,

    /// Preserve original physical layout (default: true)
    /// Uses pdftotext -layout flag
    #[serde(default = "default_true")]
    pub preserve_layout: bool,

    /// First page to convert (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_page: Option<u32>,

    /// Last page to convert (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_page: Option<u32>,

    /// Output text encoding (default: UTF-8)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,

    /// Don't insert page breaks between pages
    #[serde(default)]
    pub no_page_breaks: bool,
}

fn default_true() -> bool {
    true
}

/// Response from PDF to text extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfToTextResponse {
    /// Extracted text content (if output_path was not specified)
    pub content: Option<String>,

    /// Path to the output file (if output_path was specified)
    pub output_path: Option<String>,

    /// Number of bytes written (if output_path was specified)
    pub bytes_written: Option<usize>,

    /// Success status
    pub success: bool,

    /// Any error or informational message
    pub message: String,
}

/// Request to confirm that PDF text extraction looks correct
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConfirmExtractionRequest {}

/// Response confirming PDF text extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmExtractionResponse {
    /// Success status
    pub success: bool,

    /// Confirmation message
    pub message: String,
}

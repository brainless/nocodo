use crate::{
    error::LlmError,
    voyage::{
        client::VoyageClient,
        types::{
            VoyageEmbeddingRequest, VoyageEmbeddingResponse, VoyageEncodingFormat, VoyageInput,
            VoyageInputType, VoyageOutputDtype,
        },
    },
};

/// Builder for creating Voyage AI embedding requests
pub struct VoyageEmbeddingBuilder<'a> {
    client: &'a VoyageClient,
    model: Option<String>,
    input: Option<VoyageInput>,
    input_type: Option<VoyageInputType>,
    truncation: Option<bool>,
    output_dimension: Option<u32>,
    output_dtype: Option<VoyageOutputDtype>,
    encoding_format: Option<VoyageEncodingFormat>,
}

impl<'a> VoyageEmbeddingBuilder<'a> {
    /// Create a new embedding builder
    pub fn new(client: &'a VoyageClient) -> Self {
        Self {
            client,
            model: None,
            input: None,
            input_type: None,
            truncation: None,
            output_dimension: None,
            output_dtype: None,
            encoding_format: None,
        }
    }

    /// Set the model to use (e.g., "voyage-4-lite")
    ///
    /// It's recommended to use constants from `crate::models::voyage` module:
    /// - `VOYAGE_4_LARGE` - Highest accuracy
    /// - `VOYAGE_4` - Balanced performance
    /// - `VOYAGE_4_LITE` - Fast and cost-effective (1M tokens/batch)
    /// - `VOYAGE_CODE_3` - Specialized for code
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the input text(s) to embed
    ///
    /// Can be a single string or multiple strings:
    /// ```rust,ignore
    /// builder.input("Single text")
    /// builder.input(vec!["Text 1", "Text 2", "Text 3"])
    /// ```
    pub fn input(mut self, input: impl Into<VoyageInput>) -> Self {
        self.input = Some(input.into());
        self
    }

    /// Set the input type for retrieval optimization
    ///
    /// - `VoyageInputType::Query` - For search queries (prepends query prompt)
    /// - `VoyageInputType::Document` - For documents to be searched (prepends document prompt)
    /// - `None` - Direct embedding without prompt (default)
    pub fn input_type(mut self, input_type: VoyageInputType) -> Self {
        self.input_type = Some(input_type);
        self
    }

    /// Set whether to truncate inputs that exceed context length
    ///
    /// - `true` - Truncate over-length inputs (default)
    /// - `false` - Return error for over-length inputs
    pub fn truncation(mut self, truncation: bool) -> Self {
        self.truncation = Some(truncation);
        self
    }

    /// Set the output embedding dimension
    ///
    /// Supported values: 256, 512, 1024 (default), 2048
    ///
    /// Only supported by: voyage-4-large, voyage-4, voyage-4-lite,
    /// voyage-3-large, voyage-3.5, voyage-3.5-lite, voyage-code-3
    pub fn output_dimension(mut self, dimension: u32) -> Self {
        self.output_dimension = Some(dimension);
        self
    }

    /// Set the output data type
    ///
    /// - `VoyageOutputDtype::Float` - 32-bit floats (default, highest precision)
    /// - `VoyageOutputDtype::Int8` - 8-bit signed integers
    /// - `VoyageOutputDtype::Uint8` - 8-bit unsigned integers
    /// - `VoyageOutputDtype::Binary` - Bit-packed binary (int8)
    /// - `VoyageOutputDtype::Ubinary` - Bit-packed binary (uint8)
    pub fn output_dtype(mut self, dtype: VoyageOutputDtype) -> Self {
        self.output_dtype = Some(dtype);
        self
    }

    /// Set the encoding format
    ///
    /// - `None` - Return embeddings as arrays (default)
    /// - `Some(VoyageEncodingFormat::Base64)` - Return as base64-encoded NumPy arrays
    pub fn encoding_format(mut self, format: VoyageEncodingFormat) -> Self {
        self.encoding_format = Some(format);
        self
    }

    /// Send the embedding request
    pub async fn send(self) -> Result<VoyageEmbeddingResponse, LlmError> {
        let model = self
            .model
            .ok_or_else(|| LlmError::invalid_request("Model is required"))?;
        let input = self
            .input
            .ok_or_else(|| LlmError::invalid_request("Input is required"))?;

        let request = VoyageEmbeddingRequest {
            model,
            input,
            input_type: self.input_type,
            truncation: self.truncation,
            output_dimension: self.output_dimension,
            output_dtype: self.output_dtype,
            encoding_format: self.encoding_format,
        };

        self.client.create_embedding(request).await
    }
}

impl VoyageClient {
    /// Create a new embedding builder
    ///
    /// # Example
    /// ```rust,ignore
    /// use nocodo_llm_sdk::voyage::VoyageClient;
    /// use nocodo_llm_sdk::models::voyage::VOYAGE_4_LITE;
    ///
    /// let client = VoyageClient::new("your-api-key")?;
    /// let response = client
    ///     .embedding_builder()
    ///     .model(VOYAGE_4_LITE)
    ///     .input("Hello, world!")
    ///     .send()
    ///     .await?;
    /// ```
    pub fn embedding_builder(&self) -> VoyageEmbeddingBuilder<'_> {
        VoyageEmbeddingBuilder::new(self)
    }
}

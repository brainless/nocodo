use nocodo_llm_sdk::voyage::{VoyageClient, VoyageInputType, VoyageOutputDtype};
use nocodo_llm_sdk::models::voyage::{VOYAGE_4_LITE, VOYAGE_4, VOYAGE_CODE_3};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment variable
    let api_key = env::var("VOYAGE_API_KEY")
        .expect("VOYAGE_API_KEY environment variable not set");

    let client = VoyageClient::new(api_key)?;

    println!("=== Voyage AI Embeddings Examples ===\n");

    // Example 1: Single text embedding with voyage-4-lite
    println!("1. Single text embedding (voyage-4-lite):");
    let response = client
        .embedding_builder()
        .model(VOYAGE_4_LITE)
        .input("Hello, world! This is a test of text embeddings.")
        .send()
        .await?;

    println!("   Model: {}", response.model);
    println!("   Total tokens: {}", response.usage.total_tokens);
    for embedding in &response.data {
        println!("   Embedding {}: {} dimensions", embedding.index, embedding.embedding.len());
        println!("   First 5 values: {:?}", &embedding.embedding[..5.min(embedding.embedding.len())]);
    }
    println!();

    // Example 2: Multiple texts with query input type
    println!("2. Multiple texts with query input type:");
    let response = client
        .embedding_builder()
        .model(VOYAGE_4_LITE)
        .input(vec![
            "What is machine learning?",
            "How does neural network training work?",
            "Explain gradient descent algorithm"
        ])
        .input_type(VoyageInputType::Query)
        .send()
        .await?;

    println!("   Model: {}", response.model);
    println!("   Total tokens: {}", response.usage.total_tokens);
    println!("   Number of embeddings: {}", response.data.len());
    for embedding in &response.data {
        println!("   Embedding {}: {} dimensions", embedding.index, embedding.embedding.len());
    }
    println!();

    // Example 3: Document embeddings with custom dimension
    println!("3. Document embeddings with 512 dimensions:");
    let response = client
        .embedding_builder()
        .model(VOYAGE_4)
        .input(vec![
            "Machine learning is a subset of artificial intelligence.",
            "Deep learning uses neural networks with multiple layers.",
            "Natural language processing enables computers to understand text."
        ])
        .input_type(VoyageInputType::Document)
        .output_dimension(512)
        .send()
        .await?;

    println!("   Model: {}", response.model);
    println!("   Total tokens: {}", response.usage.total_tokens);
    for embedding in &response.data {
        println!("   Embedding {}: {} dimensions", embedding.index, embedding.embedding.len());
    }
    println!();

    // Example 4: Code embeddings with voyage-code-3
    println!("4. Code embeddings (voyage-code-3):");
    let response = client
        .embedding_builder()
        .model(VOYAGE_CODE_3)
        .input(vec![
            "fn add(a: i32, b: i32) -> i32 { a + b }",
            "function multiply(x, y) { return x * y; }",
            "def subtract(a, b): return a - b"
        ])
        .input_type(VoyageInputType::Document)
        .send()
        .await?;

    println!("   Model: {}", response.model);
    println!("   Total tokens: {}", response.usage.total_tokens);
    for embedding in &response.data {
        println!("   Embedding {}: {} dimensions", embedding.index, embedding.embedding.len());
    }
    println!();

    // Example 5: High-dimensional embeddings (2048)
    println!("5. High-dimensional embeddings (2048 dimensions):");
    let response = client
        .embedding_builder()
        .model(VOYAGE_4_LITE)
        .input("This text will be embedded in high-dimensional space.")
        .output_dimension(2048)
        .send()
        .await?;

    println!("   Model: {}", response.model);
    println!("   Total tokens: {}", response.usage.total_tokens);
    for embedding in &response.data {
        println!("   Embedding {}: {} dimensions", embedding.index, embedding.embedding.len());
    }
    println!();

    // Example 6: Low-dimensional embeddings (256) for efficiency
    println!("6. Low-dimensional embeddings (256 dimensions):");
    let response = client
        .embedding_builder()
        .model(VOYAGE_4_LITE)
        .input(vec![
            "Compact representation for fast similarity search",
            "Lower dimensional embeddings use less memory",
            "Trade-off between dimension and accuracy"
        ])
        .output_dimension(256)
        .truncation(true)
        .send()
        .await?;

    println!("   Model: {}", response.model);
    println!("   Total tokens: {}", response.usage.total_tokens);
    for embedding in &response.data {
        println!("   Embedding {}: {} dimensions", embedding.index, embedding.embedding.len());
    }
    println!();

    println!("=== All examples completed successfully! ===");

    Ok(())
}

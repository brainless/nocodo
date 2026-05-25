use crate::code_extractor::CodeBlock;

/// Build a single-shot prompt for generating a Diesel model impl function.
///
/// Keeps context minimal: struct definition + up to 3 existing impl functions
/// as style examples. No system prompt — the user message carries everything.
pub fn build_prompt(struct_code: &str, example_fns: &[CodeBlock], fn_name: &str) -> String {
    let mut prompt = format!(
        "Write the Rust function `{}` for this Diesel model.\n\nModel:\n```rust\n{}\n```\n\n",
        fn_name, struct_code
    );

    let examples: Vec<&CodeBlock> = example_fns.iter().take(3).collect();
    if !examples.is_empty() {
        prompt.push_str("Existing functions (for reference):\n```rust\n");
        for f in &examples {
            prompt.push_str(&f.source);
            prompt.push('\n');
        }
        prompt.push_str("```\n\n");
    }

    prompt.push_str("Return only the function definition, no explanation.");
    prompt
}

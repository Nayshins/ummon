use anyhow::Result;

use crate::graph::knowledge_graph::KnowledgeGraph;
use crate::prompt::context_builder::build_context;
use crate::prompt::llm_integration::{query_llm, LlmConfig, LlmProvider};
use std::str::FromStr;

pub async fn run(
    instruction: &str,
    llm_provider: Option<&str>,
    llm_model: Option<&str>,
) -> Result<()> {
    println!("AI Assist: {}", instruction);

    // Load the KG
    let kg = KnowledgeGraph::load_from_file("knowledge_graph.json")?;

    // Build relevant context
    let context = build_context(&kg, instruction);

    // Create LLM config
    let provider_str = llm_provider
        .map(|s| s.to_string())
        .or_else(|| std::env::var("LLM_PROVIDER").ok())
        .unwrap_or_else(|| "openrouter".to_string());

    let provider = LlmProvider::from_str(&provider_str).unwrap_or(LlmProvider::OpenRouter);

    // Get the appropriate API key based on the provider
    let api_key = match provider {
        LlmProvider::OpenRouter => std::env::var("OPENROUTER_API_KEY").unwrap_or_default(),
        LlmProvider::OpenAI => std::env::var("OPENAI_API_KEY").unwrap_or_default(),
        LlmProvider::Anthropic => std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
        LlmProvider::GoogleVertexAI => std::env::var("GOOGLE_API_KEY").unwrap_or_default(),
        LlmProvider::Ollama => String::new(), // Ollama doesn't need an API key
        LlmProvider::Mock => String::new(),
    };

    // Determine the model to use (CLI option, then env var, then default)
    let model = llm_model
        .map(|s| s.to_string())
        .or_else(|| std::env::var("LLM_MODEL").ok())
        .unwrap_or_else(|| {
            // Default models based on provider
            match provider {
                LlmProvider::OpenRouter => "anthropic/claude-3-5-haiku-20241022".to_string(),
                LlmProvider::OpenAI => "gpt-4-turbo".to_string(),
                LlmProvider::Anthropic => "claude-3-5-haiku-20241022".to_string(),
                LlmProvider::GoogleVertexAI => "gemini-1.5-pro".to_string(),
                LlmProvider::Ollama => "llama3".to_string(),
                LlmProvider::Mock => "mock".to_string(),
            }
        });

    // Configure LLM request
    let config = LlmConfig {
        provider,
        api_key,
        model,
        temperature: 0.2,
        max_tokens: 1500,
        endpoint_url: std::env::var("LLM_ENDPOINT").ok(),
    };

    // We're already in an async context (from tokio::main), so we can just await
    let response = query_llm(&context, &config).await?;

    println!("LLM suggests:\n{}", response);

    // For advanced usage, parse diffs from response & apply them.

    Ok(())
}

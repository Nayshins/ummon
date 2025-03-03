use anyhow::Result;

use crate::graph::knowledge_graph::KnowledgeGraph;
use crate::prompt::context_builder::build_context;
use crate::prompt::llm_integration::query_llm;

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
    let config = crate::prompt::llm_integration::get_llm_config(llm_provider, llm_model);

    // We're already in an async context (from tokio::main), so we can just await
    let response = query_llm(&context, &config).await?;

    println!("LLM suggests:\n{}", response);

    // For advanced usage, parse diffs from response & apply them.

    Ok(())
}

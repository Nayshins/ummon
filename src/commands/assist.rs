use anyhow::Result;

use crate::graph::knowledge_graph::KnowledgeGraph;
use crate::prompt::context_builder::build_context;
use crate::prompt::llm_integration::query_llm;

pub fn run(instruction: &str) -> Result<()> {
    println!("AI Assist: {}", instruction);

    // Load the KG
    let kg = KnowledgeGraph::load_from_file("knowledge_graph.json")?;

    // Build relevant context
    let context = build_context(&kg, instruction);

    // Call the LLM
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    let response = query_llm(&context, &api_key);

    println!("LLM suggests:\n{:?}", response);

    // For advanced usage, parse diffs from response & apply them.

    Ok(())
}

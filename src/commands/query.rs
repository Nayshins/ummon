use crate::graph::KnowledgeGraph;
use crate::prompt::context_builder::build_context;
use crate::prompt::llm_integration::query_llm;
use anyhow::Result;

pub async fn run(instruction: &str, format: &str) -> Result<()> {
    println!("Querying knowledge graph: {}", instruction);

    // Load the KG
    let kg = KnowledgeGraph::load_from_file("knowledge_graph.json")?;

    // Build relevant context
    let context = build_context(&kg, instruction);

    // Call the LLM (async)
    let api_key = std::env::var("OPENROUTER_API_KEY").unwrap_or_default();
    let response = query_llm(&context, &api_key).await?;

    // Format the output according to the requested format
    match format {
        "json" => {
            // Try to parse the response as JSON for structured output
            match serde_json::from_str::<serde_json::Value>(&response) {
                Ok(json) => println!("{}", serde_json::to_string_pretty(&json)?),
                Err(_) => println!("{}", response), // Fallback to plain text if not valid JSON
            }
        }
        _ => println!("{}", response), // Default to plain text
    }

    Ok(())
}

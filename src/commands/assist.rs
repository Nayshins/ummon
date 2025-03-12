use anyhow::Result;
use tracing;

use crate::graph::knowledge_graph::KnowledgeGraph;
use crate::prompt::context_builder::build_context;
use crate::prompt::llm_integration::query_llm;

pub async fn run(
    instruction: &str,
    llm_provider: Option<&str>,
    llm_model: Option<&str>,
) -> Result<()> {
    println!("AI Assist: {}", instruction);

    // Load the knowledge graph from database
    let db = crate::db::get_database("ummon.db")?;
    let mut kg = KnowledgeGraph::new();

    // Load entities from database
    let entities = db.load_entities()?;
    for entity in entities {
        if let Err(e) = kg.add_boxed_entity(entity) {
            tracing::warn!("Failed to add entity to knowledge graph: {}", e);
        }
    }

    // Load relationships from database
    let relationships = db.load_relationships()?;
    for relationship in relationships {
        kg.add_relationship(relationship);
    }

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

use anyhow::Result;
use colored::Colorize;
use tracing;

use crate::graph::knowledge_graph::KnowledgeGraph;
use crate::prompt::context_builder::build_context;
use crate::prompt::llm_integration::{get_llm_config, query_llm};
use ummon::agent::relevance_agent::RelevantFile;

pub async fn run(
    instruction: &str,
    llm_provider: Option<&str>,
    llm_model: Option<&str>,
) -> Result<()> {
    println!("{} {}", "AI Assist:".bold().green(), instruction);

    let db = crate::db::get_database("ummon.db")?;

    println!("{}", "Finding relevant files...".italic());
    
    async fn get_relevant_files(
        query: &str,
        _db: &crate::db::Database,
    ) -> Result<Vec<RelevantFile>> {
        let db_path = "ummon.db";
        let lib_db = ummon::db::get_database(db_path)?;
        let files = ummon::agent::relevance_agent::suggest_relevant_files(query, &lib_db).await?;
        Ok(files)
    }

    let relevant_files = get_relevant_files(instruction, &db).await?;

    if !relevant_files.is_empty() {
        println!("\n{}", "Suggested files:".bold().underline());
        relevant_files.iter().enumerate().for_each(|(i, file)| {
            println!(
                "{}. {} (score: {:.2}, entities: {})",
                i + 1,
                file.path.bold(),
                file.relevance_score,
                file.contributing_entity_ids.len()
            );
        });
        println!();
    }

    println!("{}", "Building knowledge graph context...".italic());
    let mut kg = KnowledgeGraph::new();

    let entities = db.load_entities()?;
    entities.into_iter().for_each(|entity| {
        if let Err(e) = kg.add_boxed_entity(entity) {
            tracing::warn!("Failed to add entity to knowledge graph: {}", e);
        }
    });

    db.load_relationships()?
        .into_iter()
        .for_each(|relationship| {
            kg.add_relationship(relationship);
        });

    let file_context = relevant_files
        .iter()
        .map(|file| format!("- {}", file.path))
        .collect::<Vec<_>>()
        .join("\n");

    let context = if file_context.is_empty() {
        build_context(&kg, instruction)
    } else {
        format!(
            "{}\n\nRelevant files:\n{}",
            build_context(&kg, instruction),
            file_context
        )
    };

    println!("{}", "Consulting LLM for guidance...".italic());
    let response = query_llm(&context, &get_llm_config(llm_provider, llm_model)).await?;

    println!("\n{}\n{}", "LLM suggests:".bold().blue(), response);

    Ok(())
}

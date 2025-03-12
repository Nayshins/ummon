mod executor;
mod formatter;
mod nl_translator;
mod parser;

pub use executor::QueryExecutor;
pub use formatter::{OutputFormat, ResultFormatter};
pub use nl_translator::NaturalLanguageTranslator;
pub use parser::parse_query;

// Only re-export QueryType for test modules
#[cfg(test)]
pub use parser::QueryType;

use anyhow::Result;
use crate::{graph::knowledge_graph::KnowledgeGraph, prompt::llm_integration::get_llm_config};

/// Process a query string and return formatted results
pub async fn process_query(
    kg: &KnowledgeGraph,
    query_str: &str,
    format_str: &str,
    natural: bool,
    llm_provider: Option<&str>,
    llm_model: Option<&str>,
) -> Result<String> {
    // Set up the formatter
    let format = format_str.parse().unwrap_or(OutputFormat::Text);
    let formatter = ResultFormatter::new(kg, format);

    // If natural language is enabled, translate query first
    let query_to_execute = if natural {
        let config = get_llm_config(llm_provider, llm_model);
        let translator = NaturalLanguageTranslator::new(config);
        let (translated, confidence) = translator.translate(query_str).await?;

        // Print the translation information
        eprintln!("Translated query: {}", translated);
        eprintln!("Translation confidence: {:.2}", confidence);

        // Return the translated query
        translated
    } else {
        query_str.to_string()
    };

    // Parse the query
    let parsed_query = parse_query(&query_to_execute)?;

    // Execute the query
    let executor = QueryExecutor::new(kg);
    let results = executor.execute(parsed_query)?;

    // Format and return the results
    formatter.format(results)
}

/// Options for refining query execution and output
pub struct QueryOptions {
    pub format: String,
    pub natural: bool,
    pub llm_provider: Option<String>,
    pub llm_model: Option<String>,
    pub limit: usize,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            format: "text".to_string(),
            natural: false,
            llm_provider: None,
            llm_model: None,
            limit: 100,
        }
    }
}

/// Convenience function to execute a query with options
pub async fn execute_query(query_str: &str, options: QueryOptions) -> Result<String> {
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

    // Process the query
    let result = process_query(
        &kg,
        query_str,
        &options.format,
        options.natural,
        options.llm_provider.as_deref(),
        options.llm_model.as_deref(),
    )
    .await?;

    // Apply limit if needed
    if options.limit > 0 && options.format == "text" {
        // Only apply limit to text format to avoid breaking JSON/CSV structure
        let lines: Vec<&str> = result.lines().collect();
        let total_count = lines.len();

        if total_count > options.limit {
            let limited = lines.into_iter().take(options.limit).collect::<Vec<_>>();
            return Ok(format!(
                "{}\n(Limited to {} results, total: {})",
                limited.join("\n"),
                options.limit,
                total_count
            ));
        }
    }

    Ok(result)
}

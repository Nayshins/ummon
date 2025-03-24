mod db_executor;
mod executor;
mod formatter;
mod nl_translator;
mod parser;

pub use db_executor::DbQueryExecutor;
pub use executor::QueryExecutor;
pub use formatter::{OutputFormat, ResultFormatter};
pub use nl_translator::NaturalLanguageTranslator;
pub use parser::parse_query;

use crate::{graph::knowledge_graph::KnowledgeGraph, prompt::llm_integration::get_llm_config};
use anyhow::Result;

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

/// Process a query directly using the database without loading everything into memory
pub async fn process_query_with_db(
    db: &crate::db::Database,
    query_str: &str,
    format_str: &str,
    natural: bool,
    llm_provider: Option<&str>,
    llm_model: Option<&str>,
) -> Result<String> {
    // Set up the formatter with SQLite mode (modified formatter that handles boxed entities)
    let format = format_str.parse().unwrap_or(OutputFormat::Text);
    let formatter = ResultFormatter::new_for_boxed_entities(format);

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

    // Execute the query directly with the database
    let executor = DbQueryExecutor::new(db);
    let results = executor.execute(parsed_query)?;

    // Format and return the results
    formatter.format_boxed_entities(&results)
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
    // Connect to the database
    let db = crate::db::get_database("ummon.db")?;

    // Use the direct database query approach
    let result = process_query_with_db(
        &db,
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

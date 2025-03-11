use crate::query::{self, QueryOptions};
use anyhow::Result;

/// Runs the query command with the provided arguments
pub async fn run(
    query_str: &str,
    format: &str,
    natural: bool,
    limit: usize,
    llm_provider: Option<&str>,
    llm_model: Option<&str>,
) -> Result<()> {
    tracing::info!("Querying knowledge graph: {}", query_str);

    // Set up query options
    let options = QueryOptions {
        format: format.to_string(),
        natural,
        llm_provider: llm_provider.map(|s| s.to_string()),
        llm_model: llm_model.map(|s| s.to_string()),
        limit,
    };

    // Show what mode we're using
    if natural {
        eprintln!("Using natural language translation");
    } else {
        eprintln!("Using direct query syntax");
    }

    // Execute the query
    let result = query::execute_query(query_str, options).await?;

    // Print the result
    println!("{}", result);

    // Add help text for first-time users
    if result.is_empty() || result.trim() == "[]" || result.trim() == "No results found." {
        eprintln!("\nNo results found. Here are some tips:");
        eprintln!(" - Check if your query syntax is correct");
        eprintln!(" - Try using more general terms or wildcards like '%'");
        eprintln!(" - Make sure the entity types you're looking for exist in the codebase");
        eprintln!(" - Run with --no-llm flag if you want to use the query language directly");
        eprintln!(" - See documentation: docs/query_system.md for more examples");
    }

    Ok(())
}

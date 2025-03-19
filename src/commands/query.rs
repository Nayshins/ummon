use crate::db::get_database;
use crate::query::{self, nl_translator, parser, QueryExecutor};
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

    // Show what mode we're using
    if natural {
        eprintln!("Using natural language translation");
    } else {
        eprintln!("Using direct query syntax");
    }

    // Get a connection to the database
    let db = get_database("ummon.db")?;
    let executor = QueryExecutor::new(&db);

    // Parse or translate the query
    let query = if natural {
        // Translate natural language to query language
        nl_translator::translate(query_str, llm_provider, llm_model).await?
    } else {
        // Parse directly
        parser::parse_query(query_str)?
    };

    // Execute the query using SQLite
    let result = executor.execute(query)?;

    // Print the result
    println!("{}", result);

    // Add help text for first-time users
    if result.is_empty()
        || result.trim() == "[]"
        || result.trim() == "No results found."
        || result.contains("Found 0")
    {
        eprintln!("\nNo results found. Here are some tips:");
        eprintln!(" - Check if your query syntax is correct");
        eprintln!(" - Try using more general terms or wildcards like '%'");
        eprintln!(" - Make sure the entity types you're looking for exist in the codebase");
        eprintln!(" - Run with --no-llm flag if you want to use the query language directly");
        eprintln!(" - See documentation: docs/query_system.md for more examples");
    }

    Ok(())
}

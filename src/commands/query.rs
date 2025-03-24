use crate::db;
use crate::query::{self, DbQueryExecutor, QueryOptions};
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

    // Connect to the database
    let db = db::get_database("ummon.db")?;

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

    // Execute the query directly with the database
    let result = query::process_query_with_db(
        &db,
        query_str,
        &options.format,
        options.natural,
        options.llm_provider.as_deref(),
        options.llm_model.as_deref(),
    )
    .await?;

    // Apply limit if needed
    let output = if limit > 0 && format == "text" {
        // Only apply limit to text format to avoid breaking JSON/CSV structure
        let lines: Vec<&str> = result.lines().collect();
        let total_count = lines.len();

        if total_count > limit {
            let limited = lines.into_iter().take(limit).collect::<Vec<_>>();
            format!(
                "{}\n(Limited to {} results, total: {})",
                limited.join("\n"),
                limit,
                total_count
            )
        } else {
            result
        }
    } else {
        result
    };

    // Print the result
    println!("{}", output);

    // Add help text for first-time users
    if output.is_empty()
        || output.trim() == "[]"
        || output.trim() == "No results found."
        || output.trim() == "No entities found"
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

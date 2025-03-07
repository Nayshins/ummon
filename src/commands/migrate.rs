use anyhow::Result;
use std::path::Path;
use std::time::Instant;

use ummon::Database;
use ummon::KnowledgeGraph;

/// Migrate knowledge graph from JSON to SQLite
pub async fn run(json_path: &str, db_path: &str) -> Result<()> {
    tracing::info!(
        "Migrating knowledge graph from {} to {}",
        json_path,
        db_path
    );
    let start_time = Instant::now();

    // Check if the JSON file exists
    if !Path::new(json_path).exists() {
        return Err(anyhow::anyhow!("JSON file {} not found", json_path));
    }

    // Load the knowledge graph from JSON
    tracing::info!("Loading knowledge graph from JSON...");
    let kg = KnowledgeGraph::load_from_file(json_path)?;

    let entity_count = kg.get_all_entities().len();
    let relationship_count = kg.get_relationship_count();

    tracing::info!(
        "Loaded {} entities and {} relationships from JSON",
        entity_count,
        relationship_count
    );

    // Create a new database
    tracing::info!("Creating SQLite database...");
    let db = Database::new(db_path)?;

    // Save to the database
    tracing::info!("Saving entities and relationships to SQLite...");
    // Wrap the database in an Arc
    use std::sync::Arc;
    kg.save_to_database(Arc::new(db))?;

    let duration = start_time.elapsed();
    tracing::info!("Migration complete in {:.2?}", duration);
    tracing::info!("Data migrated to: {}", db_path);

    // Verify the migration by loading from the database
    tracing::info!("Verifying migration...");
    let mut new_kg = KnowledgeGraph::with_database(db_path)?;
    new_kg.load_from_database()?;

    let new_entity_count = new_kg.get_all_entities().len();
    let new_relationship_count = new_kg.get_relationship_count();

    tracing::info!(
        "Verified: {} entities and {} relationships in SQLite database",
        new_entity_count,
        new_relationship_count
    );

    if entity_count == new_entity_count && relationship_count == new_relationship_count {
        tracing::info!("Migration successful! SQLite database matches JSON source.");
    } else {
        tracing::warn!(
            "Migration may be incomplete. Counts don't match between source and destination."
        );
        tracing::warn!(
            "  JSON: {} entities, {} relationships",
            entity_count,
            relationship_count
        );
        tracing::warn!(
            "  SQLite: {} entities, {} relationships",
            new_entity_count,
            new_relationship_count
        );
    }

    Ok(())
}

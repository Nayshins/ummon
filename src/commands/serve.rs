use anyhow::Result;
use std::sync::Arc;
use tokio::io::stdin;
use tokio::io::stdout;
use tracing::{error, info};

use crate::graph::knowledge_graph::KnowledgeGraph;
use crate::mcp_server::{ByteTransport, Server, UmmonRouter};

/// Run the MCP server with stdin/stdout transport
pub async fn run() -> Result<()> {
    info!("Starting Ummon MCP server");

    // Try to load the knowledge graph from the database
    let db = match crate::db::get_database("ummon.db") {
        Ok(db) => {
            info!("Connected to knowledge graph database");
            db
        }
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            error!("Please run `ummon index <directory>` first to create a knowledge graph");
            return Err(anyhow::anyhow!(
                "Database connection failed, please run `ummon index <directory>` first"
            ));
        }
    };

    // Create a new knowledge graph and load entities and relationships
    let mut knowledge_graph = KnowledgeGraph::new();

    // Load entities from database
    match db.load_entities() {
        Ok(entities) => {
            for entity in entities {
                if let Err(e) = knowledge_graph.add_boxed_entity(entity) {
                    error!("Failed to add entity to knowledge graph: {}", e);
                }
            }
        }
        Err(e) => {
            error!("Failed to load entities from database: {}", e);
            return Err(anyhow::anyhow!(
                "Failed to load entities from database: {}",
                e
            ));
        }
    }

    // Load relationships from database
    match db.load_relationships() {
        Ok(relationships) => {
            for relationship in relationships {
                knowledge_graph.add_relationship(relationship);
            }
        }
        Err(e) => {
            error!("Failed to load relationships from database: {}", e);
            return Err(anyhow::anyhow!(
                "Failed to load relationships from database: {}",
                e
            ));
        }
    }

    // Verify that there's something in the knowledge graph
    let entity_count = knowledge_graph.get_all_entities().len();
    if entity_count == 0 {
        error!("Knowledge graph was loaded but contains 0 entities");
        error!("Please run `ummon index <directory>` to populate the knowledge graph");
        return Err(anyhow::anyhow!(
            "Empty knowledge graph, please run `ummon index <directory>` to populate it"
        ));
    }

    info!(
        "Successfully loaded knowledge graph with {} entities and {} relationships",
        entity_count,
        knowledge_graph.get_relationship_count()
    );

    let knowledge_graph = Arc::new(knowledge_graph);
    let router = UmmonRouter::new(knowledge_graph);
    let server = Server::new(router);

    info!("Using stdin/stdout transport");
    info!("Server is ready to receive JSON-RPC requests - connect a compatible client");
    info!("Available tools: search_code, get_entity, debug_graph");
    let transport = ByteTransport::new(stdin(), stdout());
    server.run(transport).await?;

    Ok(())
}

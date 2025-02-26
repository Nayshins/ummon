use std::sync::Arc;
use anyhow::Result;
use tokio::io::stdin;
use tokio::io::stdout;
use tracing::{info, error};

use crate::cli::TransportType;
use crate::graph::knowledge_graph::KnowledgeGraph;
use crate::mcp_server::{
    ByteTransport, 
    UmmonRouter, 
    Server
};

/// Run the MCP server with the specified transport
pub async fn run(transport_type: &TransportType) -> Result<()> {
    info!("Starting Ummon MCP server");

    // Try to load the knowledge graph
    let knowledge_graph = match KnowledgeGraph::load() {
        Ok(kg) => {
            // Verify that there's something in the knowledge graph
            let entity_count = kg.get_all_entities().len();
            if entity_count == 0 {
                error!("Knowledge graph was loaded but contains 0 entities");
                error!("Please run `ummon index <directory>` to populate the knowledge graph");
                return Err(anyhow::anyhow!("Empty knowledge graph, please run `ummon index <directory>` to populate it"));
            }
            
            info!("Successfully loaded knowledge graph with {} entities and {} relationships", 
                  entity_count, kg.get_relationship_count());
            kg
        },
        Err(e) => {
            error!("Failed to load knowledge graph: {}", e);
            error!("Please run `ummon index <directory>` first to create a knowledge graph");
            return Err(anyhow::anyhow!("Knowledge graph not found, please run `ummon index <directory>` first"));
        }
    };

    let knowledge_graph = Arc::new(knowledge_graph);
    let router = UmmonRouter::new(knowledge_graph);
    let server = Server::new(router);

    match transport_type {
        TransportType::StdinStdout => {
            info!("Using stdin/stdout transport");
            info!("Server is ready to receive JSON-RPC requests - connect a compatible client");
            info!("Available tools: search_code, get_entity, debug_graph");
            // Use only stderr for logging when using stdin/stdout for the protocol
            let transport = ByteTransport::new(stdin(), stdout());
            server.run(transport).await?;
        }
        TransportType::Http => {
            #[cfg(feature = "http")]
            {
                info!("Using HTTP transport");
                // HTTP transport implementation would go here
                unimplemented!("HTTP transport is not yet implemented");
            }
            #[cfg(not(feature = "http"))]
            {
                error!("HTTP transport is not available in this build");
                return Err(anyhow::anyhow!("HTTP transport not available"));
            }
        }
    }

    Ok(())
}
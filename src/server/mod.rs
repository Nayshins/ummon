pub mod analysis;
pub mod domain;
pub mod handlers;
pub mod models;
pub mod routes;
pub mod utils;

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use axum::{
    Router,
    extract::Extension,
    http::Method,
    routing::get,
};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, error};

use crate::graph::KnowledgeGraph;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// The knowledge graph used for analysis
    pub knowledge_graph: Arc<RwLock<KnowledgeGraph>>,
}

/// Run the MCP server on the specified host and port
pub async fn run_server(host: &str, port: u16) -> Result<()> {
    // Check if knowledge graph exists, if not, suggest indexing first
    if !Path::new("knowledge_graph.json").exists() {
        error!("Knowledge graph not found. Please run 'ummon index' first.");
        return Err(anyhow!("Knowledge graph not found"));
    }

    info!("Loading knowledge graph...");
    let kg = KnowledgeGraph::load_from_file("knowledge_graph.json")?;
    
    // Print some stats about the loaded graph
    let entity_count = kg.get_all_entities().len();
    let relationship_count = kg.get_relationship_count();
    let domain_concept_count = kg.get_domain_concepts().len();
    
    info!("Knowledge Graph loaded:");
    info!("  - {} entities", entity_count);
    info!("  - {} relationships", relationship_count);
    info!("  - {} domain concepts", domain_concept_count);
    
    // Create shared state
    let app_state = AppState {
        knowledge_graph: Arc::new(RwLock::new(kg)),
    };

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any)
        .allow_origin(Any);

    // Build the API router
    let app = Router::new()
        .nest("/api", routes::api_router())
        .nest("/mcp", routes::mcp_router())
        .route("/health", get(handlers::health_check))
        .layer(Extension(app_state))
        .layer(cors);

    // Create socket address
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    
    info!("MCP server starting on http://{}", addr);
    
    // Run the server
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}
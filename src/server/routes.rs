use axum::{
    Router,
    routing::post,
};

use crate::server::handlers;

/// Create the API router
pub fn api_router() -> Router {
    Router::new()
        .route("/analyze-impact", post(handlers::analyze_impact))
        .route("/map-domain-to-code", post(handlers::map_domain_to_code))
        .route("/map-code-to-domain", post(handlers::map_code_to_domain))
        .route("/get-entity-details", post(handlers::get_entity_details))
}

/// Create the MCP router
pub fn mcp_router() -> Router {
    Router::new()
        .route("/initialize", post(handlers::initialize))
        .route("/prompts/list", post(handlers::list_prompts))
        .route("/prompts/get", post(handlers::get_prompt))
}
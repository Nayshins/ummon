use axum::{
    extract::{Extension, Json},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use tracing::{error, info};
use std::collections::HashMap;

use crate::graph::entity::{Entity, EntityId};
use crate::graph::relationship::RelationshipType;
use crate::server::AppState;
use crate::server::models::*;

/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

/// MCP initialize endpoint
pub async fn initialize(
    Extension(_state): Extension<AppState>,
    Json(req): Json<JsonRpcRequest<InitializeParams>>,
) -> impl IntoResponse {
    info!("Received initialize request from {}", req.params.as_ref().map_or("unknown client", |p| &p.client_info.name));
    
    let result = InitializeResult {
        server_info: ServerInfo {
            name: "Ummon MCP Server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        capabilities: ServerCapabilities {
            prompts: Some(PromptOptions {
                supports_arguments: true,
            }),
            resources: Some(ResourceOptions {
                supports_binary: false,
            }),
        },
    };
    
    Json(JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: req.id,
        result,
    })
}

/// MCP list prompts endpoint
pub async fn list_prompts(
    Extension(_state): Extension<AppState>,
    Json(req): Json<JsonRpcRequest<ListPromptsRequest>>,
) -> impl IntoResponse {
    info!("Received prompts/list request");
    
    // Define available prompts
    let prompts = vec![
        Prompt {
            name: "analyze-change-impact".to_string(),
            description: Some("Analyze the impact of changing a file or component".to_string()),
            arguments: Some(vec![
                PromptArgument {
                    name: "target".to_string(),
                    description: Some("Target file or component to analyze".to_string()),
                    required: true,
                },
                PromptArgument {
                    name: "depth".to_string(),
                    description: Some("Maximum depth for impact analysis (default: 3)".to_string()),
                    required: false,
                },
            ]),
        },
        Prompt {
            name: "map-domain-to-code".to_string(),
            description: Some("Map a domain concept to code implementations".to_string()),
            arguments: Some(vec![
                PromptArgument {
                    name: "concept".to_string(),
                    description: Some("Domain concept to map".to_string()),
                    required: true,
                },
            ]),
        },
        Prompt {
            name: "map-code-to-domain".to_string(),
            description: Some("Map code to domain concepts".to_string()),
            arguments: Some(vec![
                PromptArgument {
                    name: "path".to_string(),
                    description: Some("Path to the code file to map".to_string()),
                    required: true,
                },
            ]),
        },
        Prompt {
            name: "get-entity-details".to_string(),
            description: Some("Get detailed information about an entity".to_string()),
            arguments: Some(vec![
                PromptArgument {
                    name: "id".to_string(),
                    description: Some("Entity ID to get details for".to_string()),
                    required: true,
                },
            ]),
        },
    ];
    
    Json(JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: req.id,
        result: ListPromptsResponse { prompts },
    })
}

/// MCP get prompt endpoint
pub async fn get_prompt(
    Extension(_state): Extension<AppState>,
    Json(req): Json<JsonRpcRequest<GetPromptRequest>>,
) -> impl IntoResponse {
    if let Some(params) = &req.params {
        info!("Received prompts/get request for prompt: {}", params.name);
        
        let response = match params.name.as_str() {
            "analyze-change-impact" => {
                let target = params.arguments.as_ref().and_then(|args| args.get("target")).cloned().unwrap_or_default();
                let _depth = params.arguments.as_ref()
                    .and_then(|args| args.get("depth"))
                    .and_then(|d| d.parse::<usize>().ok())
                    .unwrap_or(3);
                
                GetPromptResponse {
                    messages: vec![
                        PromptMessage {
                            role: "user".to_string(),
                            content: PromptContent::Text {
                                text: format!("Analyze the impact of changing the file or component '{}'. Please identify all affected files and components that might need modification as a result of this change. Consider both direct dependencies and domain relationships.", target),
                            },
                        },
                    ],
                }
            },
            "map-domain-to-code" => {
                let concept = params.arguments.as_ref().and_then(|args| args.get("concept")).cloned().unwrap_or_default();
                
                GetPromptResponse {
                    messages: vec![
                        PromptMessage {
                            role: "user".to_string(),
                            content: PromptContent::Text {
                                text: format!("Map the domain concept '{}' to its code implementations. Identify all components, files, classes, and functions that implement this concept.", concept),
                            },
                        },
                    ],
                }
            },
            "map-code-to-domain" => {
                let path = params.arguments.as_ref().and_then(|args| args.get("path")).cloned().unwrap_or_default();
                
                GetPromptResponse {
                    messages: vec![
                        PromptMessage {
                            role: "user".to_string(),
                            content: PromptContent::Text {
                                text: format!("Identify the domain concepts implemented by the code at '{}'. What business or domain concepts does this code represent?", path),
                            },
                        },
                    ],
                }
            },
            "get-entity-details" => {
                let id = params.arguments.as_ref().and_then(|args| args.get("id")).cloned().unwrap_or_default();
                
                GetPromptResponse {
                    messages: vec![
                        PromptMessage {
                            role: "user".to_string(),
                            content: PromptContent::Text {
                                text: format!("Provide detailed information about the entity with ID '{}', including its relationships to other entities and any domain concepts it implements.", id),
                            },
                        },
                    ],
                }
            },
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(JsonRpcErrorResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        error: JsonRpcError {
                            code: -32601,
                            message: format!("Prompt not found: {}", params.name),
                            data: None,
                        },
                    }),
                ).into_response();
            }
        };
        
        return Json(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id,
            result: response,
        }).into_response();
    }
    
    (
        StatusCode::BAD_REQUEST,
        Json(JsonRpcErrorResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id,
            error: JsonRpcError {
                code: -32602,
                message: "Missing parameters".to_string(),
                data: None,
            },
        }),
    ).into_response()
}

/// Analyze impact endpoint
pub async fn analyze_impact(
    Extension(state): Extension<AppState>,
    Json(request): Json<AnalyzeImpactRequest>,
) -> Result<Json<AnalyzeImpactResponse>, StatusCode> {
    info!("Analyzing impact for target: {}", request.target);
    
    let kg = state.knowledge_graph.read().await;
    let depth = request.depth.unwrap_or(3);
    
    // Find the target entity
    let target_id = EntityId::new(&request.target);
    let entity = kg.get_entity(&target_id).ok_or_else(|| {
        error!("Entity not found: {}", request.target);
        StatusCode::NOT_FOUND
    })?;
    
    // Calculate impact
    let impact_map = kg.calculate_impact(&target_id, depth);
    
    // Convert to response format
    let mut affected_files = Vec::new();
    let mut domain_impacts = HashMap::new();
    
    for (entity_id, impact_score) in impact_map {
        if let Some(impacted_entity) = kg.get_entity(&entity_id) {
            // Add to affected files
            if let Some(file_path) = impacted_entity.metadata().get("file_path") {
                affected_files.push(AffectedFile {
                    path: file_path.clone(),
                    impact_score,
                    entity_type: format!("{:?}", impacted_entity.entity_type()),
                    reason: format!("Depends on {}", entity.name()),
                });
            }
            
            // Check if this entity is related to any domain concepts
            let domain_concepts = kg.get_domain_concepts_for_entity(&entity_id);
            for concept in domain_concepts {
                let entry = domain_impacts.entry(concept.name().to_string())
                    .or_insert_with(|| DomainImpact {
                        concept: concept.name().to_string(),
                        impact_score: 0.0,
                        affected_components: Vec::new(),
                        description: concept.base.documentation.clone(),
                    });
                
                entry.impact_score = entry.impact_score.max(impact_score);
                if let Some(component) = impacted_entity.metadata().get("component") {
                    if !entry.affected_components.contains(component) {
                        entry.affected_components.push(component.clone());
                    }
                }
            }
        }
    }
    
    Ok(Json(AnalyzeImpactResponse {
        affected_files,
        domain_impacts: domain_impacts.into_values().collect(),
    }))
}

/// Map domain to code endpoint
pub async fn map_domain_to_code(
    Extension(state): Extension<AppState>,
    Json(request): Json<MapDomainToCodeRequest>,
) -> Result<Json<MapDomainToCodeResponse>, StatusCode> {
    info!("Mapping domain concept to code: {}", request.concept);
    
    let kg = state.knowledge_graph.read().await;
    
    // Look for domain concept matching the name
    let domain_concepts = kg.get_domain_concepts();
    let concept = domain_concepts.into_iter()
        .find(|c| c.name().to_lowercase() == request.concept.to_lowercase())
        .ok_or_else(|| {
            error!("Domain concept not found: {}", request.concept);
            StatusCode::NOT_FOUND
        })?;
    
    // Get entities related to this domain concept
    let concept_id = concept.id();
    let related_entities = kg.get_related_entities(concept_id, Some(&RelationshipType::RepresentedBy));
    
    // Convert to response format
    let mut implementations = Vec::new();
    for entity in related_entities {
        let path = entity.metadata().get("file_path").cloned().unwrap_or_default();
        
        implementations.push(CodeImplementation {
            path,
            entity_type: format!("{:?}", entity.entity_type()),
            name: entity.name().to_string(),
            relevance: 1.0, // Can be refined based on relationship weight
            location: entity.location().cloned(),
        });
    }
    
    Ok(Json(MapDomainToCodeResponse {
        concept: concept.name().to_string(),
        description: concept.base.documentation.clone(),
        implementations,
    }))
}

/// Map code to domain endpoint
pub async fn map_code_to_domain(
    Extension(state): Extension<AppState>,
    Json(request): Json<MapCodeToDomainRequest>,
) -> Result<Json<MapCodeToDomainResponse>, StatusCode> {
    info!("Mapping code to domain concepts: {}", request.path);
    
    let kg = state.knowledge_graph.read().await;
    
    // Find entities for this file path
    let entities = kg.get_all_entities();
    let file_entities: Vec<_> = entities.into_iter()
        .filter(|e| e.metadata().get("file_path").map_or(false, |p| p == &request.path))
        .collect();
    
    if file_entities.is_empty() {
        error!("No entities found for path: {}", request.path);
        return Err(StatusCode::NOT_FOUND);
    }
    
    // Collect domain concepts for each entity
    let mut concept_map = HashMap::new();
    
    for entity in file_entities {
        let domain_concepts = kg.get_domain_concepts_for_entity(entity.id());
        
        for concept in domain_concepts {
            let entry = concept_map.entry(concept.name().to_string())
                .or_insert_with(|| DomainConceptInfo {
                    name: concept.name().to_string(),
                    description: concept.base.documentation.clone(),
                    confidence: concept.confidence,
                    related_concepts: Vec::new(),
                });
            
            // Find related concepts
            let related_concept_entities = kg.get_related_entities(
                concept.id(),
                Some(&RelationshipType::RelatesTo),
            );
            
            for related in related_concept_entities {
                let related_name = related.name().to_string();
                if !entry.related_concepts.contains(&related_name) {
                    entry.related_concepts.push(related_name);
                }
            }
        }
    }
    
    Ok(Json(MapCodeToDomainResponse {
        path: request.path,
        domain_concepts: concept_map.into_values().collect(),
    }))
}

/// Get entity details endpoint
pub async fn get_entity_details(
    Extension(state): Extension<AppState>,
    Json(request): Json<GetEntityDetailsRequest>,
) -> Result<Json<GetEntityDetailsResponse>, StatusCode> {
    info!("Getting entity details for: {}", request.id);
    
    let kg = state.knowledge_graph.read().await;
    
    // Find the entity
    let entity_id = EntityId::new(&request.id);
    let entity = kg.get_entity(&entity_id).ok_or_else(|| {
        error!("Entity not found: {}", request.id);
        StatusCode::NOT_FOUND
    })?;
    
    // Get relationships
    let outgoing = kg.get_outgoing_relationships(&entity_id);
    let incoming = kg.get_incoming_relationships(&entity_id);
    
    let mut relationships = Vec::new();
    
    // Process outgoing relationships
    for rel in outgoing {
        if let Some(target) = kg.get_entity(&rel.target_id) {
            relationships.push(RelationshipInfo {
                relationship_type: format!("{:?}", rel.relationship_type),
                target_id: rel.target_id.as_str().to_string(),
                target_name: target.name().to_string(),
                target_type: format!("{:?}", target.entity_type()),
            });
        }
    }
    
    // Process incoming relationships
    for rel in incoming {
        if let Some(source) = kg.get_entity(&rel.source_id) {
            relationships.push(RelationshipInfo {
                relationship_type: format!("Incoming{:?}", rel.relationship_type),
                target_id: rel.source_id.as_str().to_string(),
                target_name: source.name().to_string(),
                target_type: format!("{:?}", source.entity_type()),
            });
        }
    }
    
    Ok(Json(GetEntityDetailsResponse {
        id: entity_id.as_str().to_string(),
        name: entity.name().to_string(),
        entity_type: format!("{:?}", entity.entity_type()),
        file_path: entity.metadata().get("file_path").cloned(),
        location: entity.location().cloned(),
        documentation: entity.metadata().get("documentation").cloned(),
        relationships,
        metadata: entity.metadata().clone(),
    }))
}
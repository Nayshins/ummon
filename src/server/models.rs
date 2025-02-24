use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::graph::entity::Location;

/// Base JSON-RPC 2.0 request
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest<T> {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    pub params: Option<T>,
}

/// Base JSON-RPC 2.0 response
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub result: T,
}

/// Base JSON-RPC 2.0 error response
#[derive(Debug, Serialize)]
pub struct JsonRpcErrorResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub error: JsonRpcError,
}

/// JSON-RPC 2.0 error object
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// MCP initialization params
#[derive(Debug, Deserialize)]
pub struct InitializeParams {
    pub client_info: ClientInfo,
    pub capabilities: ClientCapabilities,
}

/// MCP client info
#[derive(Debug, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: Option<String>,
}

/// MCP client capabilities
#[derive(Debug, Deserialize)]
pub struct ClientCapabilities {
    pub prompts: Option<PromptClientCapabilities>,
    pub resources: Option<ResourceClientCapabilities>,
}

/// MCP prompt client capabilities
#[derive(Debug, Deserialize)]
pub struct PromptClientCapabilities {
    #[serde(default)]
    pub dynamicRegistration: bool,
}

/// MCP resource client capabilities
#[derive(Debug, Deserialize)]
pub struct ResourceClientCapabilities {
    #[serde(default)]
    pub dynamicRegistration: bool,
}

/// MCP initialization result
#[derive(Debug, Serialize)]
pub struct InitializeResult {
    pub server_info: ServerInfo,
    pub capabilities: ServerCapabilities,
}

/// MCP server info
#[derive(Debug, Serialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// MCP server capabilities
#[derive(Debug, Serialize)]
pub struct ServerCapabilities {
    pub prompts: Option<PromptOptions>,
    pub resources: Option<ResourceOptions>,
}

/// MCP prompt options
#[derive(Debug, Serialize)]
pub struct PromptOptions {
    #[serde(default)]
    pub supports_arguments: bool,
}

/// MCP resource options
#[derive(Debug, Serialize)]
pub struct ResourceOptions {
    #[serde(default)]
    pub supports_binary: bool,
}

/// MCP prompt model
#[derive(Debug, Serialize, Deserialize)]
pub struct Prompt {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Option<Vec<PromptArgument>>,
}

/// MCP prompt argument
#[derive(Debug, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
}

/// MCP prompt list request
#[derive(Debug, Deserialize)]
pub struct ListPromptsRequest {}

/// MCP prompt list response
#[derive(Debug, Serialize)]
pub struct ListPromptsResponse {
    pub prompts: Vec<Prompt>,
}

/// MCP get prompt request
#[derive(Debug, Deserialize)]
pub struct GetPromptRequest {
    pub name: String,
    pub arguments: Option<HashMap<String, String>>,
}

/// MCP get prompt response
#[derive(Debug, Serialize)]
pub struct GetPromptResponse {
    pub messages: Vec<PromptMessage>,
}

/// MCP prompt message
#[derive(Debug, Serialize)]
pub struct PromptMessage {
    pub role: String,
    pub content: PromptContent,
}

/// MCP prompt content
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum PromptContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "resource")]
    Resource { resource: ResourceContent },
}

/// MCP resource content
#[derive(Debug, Serialize)]
pub struct ResourceContent {
    pub uri: String,
    pub text: String,
    pub mime_type: String,
}

/// API models for our custom endpoints

/// Impact analysis request
#[derive(Debug, Deserialize)]
pub struct AnalyzeImpactRequest {
    pub target: String,
    pub depth: Option<usize>,
}

/// Impact analysis response
#[derive(Debug, Serialize)]
pub struct AnalyzeImpactResponse {
    pub affected_files: Vec<AffectedFile>,
    pub domain_impacts: Vec<DomainImpact>,
}

/// Affected file information
#[derive(Debug, Serialize)]
pub struct AffectedFile {
    pub path: String,
    pub impact_score: f32,
    pub entity_type: String,
    pub reason: String,
}

/// Domain concept impact information
#[derive(Debug, Serialize)]
pub struct DomainImpact {
    pub concept: String,
    pub impact_score: f32,
    pub affected_components: Vec<String>,
    pub description: Option<String>,
}

/// Domain to code mapping request
#[derive(Debug, Deserialize)]
pub struct MapDomainToCodeRequest {
    pub concept: String,
}

/// Domain to code mapping response
#[derive(Debug, Serialize)]
pub struct MapDomainToCodeResponse {
    pub concept: String,
    pub description: Option<String>,
    pub implementations: Vec<CodeImplementation>,
}

/// Code implementation information
#[derive(Debug, Serialize)]
pub struct CodeImplementation {
    pub path: String,
    pub entity_type: String,
    pub name: String,
    pub relevance: f32, 
    pub location: Option<Location>,
}

/// Code to domain mapping request  
#[derive(Debug, Deserialize)]
pub struct MapCodeToDomainRequest {
    pub path: String,
}

/// Code to domain mapping response
#[derive(Debug, Serialize)]
pub struct MapCodeToDomainResponse {
    pub path: String,
    pub domain_concepts: Vec<DomainConceptInfo>,
}

/// Domain concept information
#[derive(Debug, Serialize)]
pub struct DomainConceptInfo {
    pub name: String,
    pub description: Option<String>,
    pub confidence: f32,
    pub related_concepts: Vec<String>,
}

/// Entity details request
#[derive(Debug, Deserialize)]
pub struct GetEntityDetailsRequest {
    pub id: String,
}

/// Entity details response
#[derive(Debug, Serialize)]
pub struct GetEntityDetailsResponse {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub file_path: Option<String>,
    pub location: Option<Location>,
    pub documentation: Option<String>,
    pub relationships: Vec<RelationshipInfo>,
    pub metadata: HashMap<String, String>,
}

/// Relationship information
#[derive(Debug, Serialize)]
pub struct RelationshipInfo {
    pub relationship_type: String,
    pub target_id: String,
    pub target_name: String,
    pub target_type: String,
}
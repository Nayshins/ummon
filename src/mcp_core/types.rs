use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub tools: bool,
    pub resources: CapabilityLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityLevel {
    pub read: bool,
    pub write: bool,
}

pub struct CapabilitiesBuilder {
    tools: bool,
    resources_read: bool,
    resources_write: bool,
}

impl Default for CapabilitiesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilitiesBuilder {
    pub fn new() -> Self {
        Self {
            tools: false,
            resources_read: false,
            resources_write: false,
        }
    }

    pub fn with_tools(mut self, tools: bool) -> Self {
        self.tools = tools;
        self
    }

    pub fn build(self) -> ServerCapabilities {
        ServerCapabilities {
            tools: self.tools,
            resources: CapabilityLevel {
                read: self.resources_read,
                write: self.resources_write,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub schema: Value,
}

impl Tool {
    pub fn new(name: String, description: String, schema: Value) -> Self {
        Self {
            name,
            description,
            schema,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub writeable: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum Content {
    #[serde(rename = "text")]
    Text(String),
    #[serde(rename = "image")]
    Image {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        alt: Option<String>,
    },
    #[serde(rename = "json")]
    Json(Value),
}

impl Content {
    pub fn text(content: impl Into<String>) -> Self {
        Content::Text(content.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    pub name: String,
    pub instructions: String,
    pub capabilities: ServerCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallParams {
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub content: Vec<Content>,
}

// JSON-RPC Error Codes
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;

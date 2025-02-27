use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::mcp_core::{
    InitializeParams, InitializeResult, JsonRpcError, JsonRpcRequest, JsonRpcResponse,
    ResourceError, Router, ServerError, ToolCallParams, ToolCallResult, ToolError, INTERNAL_ERROR,
    INVALID_PARAMS, METHOD_NOT_FOUND,
};
use crate::mcp_server::Transport;

/// MCP server that handles incoming JSON-RPC requests
pub struct Server<R: Router> {
    router: Arc<R>,
}

impl<R: Router> Server<R> {
    pub fn new(router: R) -> Self {
        Self {
            router: Arc::new(router),
        }
    }

    /// Run the server with the given transport
    pub async fn run<T: Transport>(&self, mut transport: T) -> Result<()> {
        info!("Starting MCP server");

        loop {
            let request = match transport.read_request().await {
                Ok(req) => req,
                Err(e) => {
                    error!("Error reading request: {:?}", e);
                    break;
                }
            };

            debug!("Received request: {:?}", request.method);

            let response = self.handle_request(request).await;

            if let Err(e) = transport.send_response(response).await {
                error!("Error sending response: {:?}", e);
                break;
            }
        }

        info!("MCP server stopped");
        Ok(())
    }

    /// Handle a JSON-RPC request and produce a response
    async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();

        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(&request.params).await,
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tools_call(&request.params).await,
            "resources/list" => self.handle_resources_list().await,
            "resources/read" => self.handle_resources_read(&request.params).await,
            "resources/write" => self.handle_resources_write(&request.params).await,
            _ => Err(ServerError::MethodNotFound(request.method)),
        };

        match result {
            Ok(value) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(value),
                error: None,
            },
            Err(e) => {
                let (code, message) = match e {
                    ServerError::MethodNotFound(method) => {
                        (METHOD_NOT_FOUND, format!("Method not found: {}", method))
                    }
                    ServerError::InvalidParams(reason) => {
                        (INVALID_PARAMS, format!("Invalid params: {}", reason))
                    }
                    ServerError::Router(reason) => {
                        (INTERNAL_ERROR, format!("Router error: {}", reason))
                    }
                    ServerError::Transport(err) => {
                        (INTERNAL_ERROR, format!("Transport error: {}", err))
                    }
                };

                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code,
                        message,
                        data: None,
                    }),
                }
            }
        }
    }

    async fn handle_initialize(&self, params: &Value) -> Result<Value, ServerError> {
        let _params: InitializeParams = serde_json::from_value(params.clone())
            .map_err(|e| ServerError::InvalidParams(e.to_string()))?;

        let result = InitializeResult {
            name: self.router.name(),
            instructions: self.router.instructions(),
            capabilities: self.router.capabilities(),
        };

        serde_json::to_value(result).map_err(|e| {
            ServerError::Router(format!("Failed to serialize initialize result: {}", e))
        })
    }

    async fn handle_tools_list(&self) -> Result<Value, ServerError> {
        let tools = self.router.list_tools();
        serde_json::to_value(tools)
            .map_err(|e| ServerError::Router(format!("Failed to serialize tools: {}", e)))
    }

    async fn handle_tools_call(&self, params: &Value) -> Result<Value, ServerError> {
        let params: ToolCallParams = serde_json::from_value(params.clone())
            .map_err(|e| ServerError::InvalidParams(e.to_string()))?;

        let content = self
            .router
            .call_tool(&params.name, params.arguments)
            .await
            .map_err(|e| match e {
                ToolError::NotFound(name) => {
                    ServerError::MethodNotFound(format!("Tool not found: {}", name))
                }
                ToolError::InvalidParams(msg) => ServerError::InvalidParams(msg),
                ToolError::ExecutionFailed(msg) | ToolError::Internal(msg) => {
                    ServerError::Router(msg)
                }
            })?;

        let result = ToolCallResult { content };
        serde_json::to_value(result)
            .map_err(|e| ServerError::Router(format!("Failed to serialize tool result: {}", e)))
    }

    async fn handle_resources_list(&self) -> Result<Value, ServerError> {
        if !self.router.capabilities().resources.read {
            return Err(ServerError::MethodNotFound(
                "Resources not supported".to_string(),
            ));
        }

        let resources = self.router.list_resources();
        serde_json::to_value(resources)
            .map_err(|e| ServerError::Router(format!("Failed to serialize resources: {}", e)))
    }

    async fn handle_resources_read(&self, params: &Value) -> Result<Value, ServerError> {
        if !self.router.capabilities().resources.read {
            return Err(ServerError::MethodNotFound(
                "Resource reading not supported".to_string(),
            ));
        }

        let uri = params
            .get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidParams("Missing 'uri' parameter".to_string()))?;

        let content = self.router.read_resource(uri).await.map_err(|e| match e {
            ResourceError::NotFound(uri) => {
                ServerError::MethodNotFound(format!("Resource not found: {}", uri))
            }
            ResourceError::PermissionDenied(msg)
            | ResourceError::InvalidResource(msg)
            | ResourceError::Internal(msg) => ServerError::Router(msg),
        })?;

        serde_json::to_value(content).map_err(|e| {
            ServerError::Router(format!("Failed to serialize resource content: {}", e))
        })
    }

    async fn handle_resources_write(&self, params: &Value) -> Result<Value, ServerError> {
        if !self.router.capabilities().resources.write {
            return Err(ServerError::MethodNotFound(
                "Resource writing not supported".to_string(),
            ));
        }

        let uri = params
            .get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidParams("Missing 'uri' parameter".to_string()))?;

        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidParams("Missing 'content' parameter".to_string()))?;

        self.router
            .write_resource(uri, content.to_string())
            .await
            .map_err(|e| match e {
                ResourceError::NotFound(uri) => {
                    ServerError::MethodNotFound(format!("Resource not found: {}", uri))
                }
                ResourceError::PermissionDenied(msg)
                | ResourceError::InvalidResource(msg)
                | ResourceError::Internal(msg) => ServerError::Router(msg),
            })?;

        Ok(Value::Bool(true))
    }
}

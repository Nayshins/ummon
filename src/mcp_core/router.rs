use async_trait::async_trait;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

use crate::mcp_core::{
    Content, Resource, ResourceError, ServerCapabilities, Tool, ToolError,
};

/// Router trait defines the interface for handling MCP requests
#[async_trait]
pub trait Router: Send + Sync + 'static {
    /// Returns the name of the server
    fn name(&self) -> String;

    /// Returns instructions for the AI agent
    fn instructions(&self) -> String;

    /// Returns the capabilities of the server
    fn capabilities(&self) -> ServerCapabilities;

    /// Lists available tools
    fn list_tools(&self) -> Vec<Tool>;

    /// Calls a tool with the given name and arguments
    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>>;

    /// Lists available resources (if supported)
    fn list_resources(&self) -> Vec<Resource> {
        Vec::new()
    }

    /// Reads a resource with the given URI (if supported)
    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        let err = ResourceError::NotFound(format!("Resource '{}' not found", uri));
        Box::pin(async move { Err(err) })
    }

    /// Writes a resource with the given URI (if supported)
    fn write_resource(
        &self,
        uri: &str,
        _content: String,
    ) -> Pin<Box<dyn Future<Output = Result<(), ResourceError>> + Send + 'static>> {
        let err = ResourceError::PermissionDenied(format!("Cannot write to resource '{}'", uri));
        Box::pin(async move { Err(err) })
    }
}
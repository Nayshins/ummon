use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

#[derive(Debug, Error)]
pub enum ResourceError {
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Internal error: {0}")]
    // Used in server.rs for error handling
    #[allow(dead_code)]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid JSON-RPC: {0}")]
    InvalidJsonRpc(String),
}

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),

    #[error("Router error: {0}")]
    Router(String),

    #[error("Method not found: {0}")]
    MethodNotFound(String),

    #[error("Invalid params: {0}")]
    InvalidParams(String),
}

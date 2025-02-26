use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, AsyncBufReadExt};
use tokio::io::BufReader;
use tokio::sync::Mutex;
use std::sync::Arc;

use crate::mcp_core::{
    TransportError, JsonRpcRequest, JsonRpcResponse,
};

/// Transport trait for handling JSON-RPC communication
#[async_trait]
pub trait Transport: Send + Sync + 'static {
    /// Read the next JSON-RPC request
    async fn read_request(&mut self) -> Result<JsonRpcRequest, TransportError>;

    /// Send a JSON-RPC response
    async fn send_response(&mut self, response: JsonRpcResponse) -> Result<(), TransportError>;
}

/// ByteTransport implements Transport over AsyncRead and AsyncWrite
pub struct ByteTransport<R, W> {
    reader: BufReader<R>,
    writer: Arc<Mutex<W>>,
}

impl<R, W> ByteTransport<R, W>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: BufReader::new(reader),
            writer: Arc::new(Mutex::new(writer)),
        }
    }
}

#[async_trait]
impl<R, W> Transport for ByteTransport<R, W>
where
    R: AsyncRead + Unpin + Send + Sync + 'static,
    W: AsyncWrite + Unpin + Send + Sync + 'static,
{
    async fn read_request(&mut self) -> Result<JsonRpcRequest, TransportError> {
        let mut line = String::new();
        self.reader.read_line(&mut line).await.map_err(|e| {
            TransportError::IoError(e)
        })?;

        if line.is_empty() {
            return Err(TransportError::IoError(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Reached end of input",
            )));
        }

        let request: JsonRpcRequest = serde_json::from_str(&line).map_err(|e| {
            TransportError::ParseError(e.to_string())
        })?;

        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            return Err(TransportError::InvalidJsonRpc(
                "Invalid JSON-RPC version, expected 2.0".to_string(),
            ));
        }

        Ok(request)
    }

    async fn send_response(&mut self, response: JsonRpcResponse) -> Result<(), TransportError> {
        let json = serde_json::to_string(&response).map_err(|e| {
            TransportError::ParseError(format!("Failed to serialize response: {}", e))
        })?;

        let mut writer = self.writer.lock().await;
        writer.write_all(json.as_bytes()).await.map_err(|e| {
            TransportError::IoError(e)
        })?;
        writer.write_all(b"\n").await.map_err(|e| {
            TransportError::IoError(e)
        })?;
        writer.flush().await.map_err(|e| {
            TransportError::IoError(e)
        })?;

        Ok(())
    }
}

/// HTTP transport for JSON-RPC over HTTP
#[cfg(feature = "http")]
pub struct HttpTransport {
    // Implementation for HTTP transport
}

#[cfg(feature = "http")]
#[async_trait]
impl Transport for HttpTransport {
    // Implementation for HTTP transport
}
use serde_json::Value;
use crate::transport::McpTransportConfig;
use sentinel_protocol::ToolDef;

pub struct McpClient {
    id: String,
    transport: McpTransportConfig,
}

impl McpClient {
    pub fn new(id: impl Into<String>, transport: McpTransportConfig) -> Self {
        Self { id: id.into(), transport }
    }

    pub fn id(&self) -> &str { &self.id }

    pub async fn list_tools(&self) -> Result<Vec<ToolDef>, McpError> {
        match &self.transport {
            McpTransportConfig::Stdio { command, args, .. } => {
                let mut child = tokio::process::Command::new(command)
                    .args(args)
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .spawn()
                    .map_err(|e| McpError::SpawnError(e.to_string()))?;

                let request = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "tools/list",
                    "params": {}
                });

                if let Some(stdin) = child.stdin.as_mut() {
                    use tokio::io::AsyncWriteExt;
                    stdin.write_all(serde_json::to_string(&request).unwrap().as_bytes()).await
                        .map_err(|e| McpError::WriteError(e.to_string()))?;
                }

                let output = child.wait_with_output().await
                    .map_err(|e| McpError::ReadError(e.to_string()))?;

                let stdout = String::from_utf8_lossy(&output.stdout);
                let tools = parse_mcp_tools_response(&stdout)?;
                Ok(tools)
            }
            McpTransportConfig::Http { url, .. } => {
                let client = reqwest::Client::new();
                let resp = client.post(url)
                    .json(&serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "tools/list",
                        "params": {}
                    }))
                    .send()
                    .await
                    .map_err(|e| McpError::HttpError(e.to_string()))?;

                let data: Value = resp.json().await
                    .map_err(|e| McpError::ParseError(e.to_string()))?;

                let tools = parse_mcp_tools_value(&data)?;
                Ok(tools)
            }
            McpTransportConfig::WebSocket { .. } => {
                Err(McpError::NotImplemented("WebSocket transport"))
            }
        }
    }

    pub async fn call_tool(&self, name: &str, args: Value) -> Result<String, McpError> {
        match &self.transport {
            McpTransportConfig::Stdio { command, args: cmd_args, .. } => {
                let mut child = tokio::process::Command::new(command)
                    .args(cmd_args)
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .spawn()
                    .map_err(|e| McpError::SpawnError(e.to_string()))?;

                let request = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "tools/call",
                    "params": { "name": name, "arguments": args }
                });

                if let Some(stdin) = child.stdin.as_mut() {
                    use tokio::io::AsyncWriteExt;
                    stdin.write_all(serde_json::to_string(&request).unwrap().as_bytes()).await
                        .map_err(|e| McpError::WriteError(e.to_string()))?;
                }

                let output = child.wait_with_output().await
                    .map_err(|e| McpError::ReadError(e.to_string()))?;

                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            }
            McpTransportConfig::Http { url, .. } => {
                let client = reqwest::Client::new();
                let resp = client.post(url)
                    .json(&serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": 2,
                        "method": "tools/call",
                        "params": { "name": name, "arguments": args }
                    }))
                    .send()
                    .await
                    .map_err(|e| McpError::HttpError(e.to_string()))?;

                let data: Value = resp.json().await
                    .map_err(|e| McpError::ParseError(e.to_string()))?;

                let result = data["result"]["content"][0]["text"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                Ok(result)
            }
            McpTransportConfig::WebSocket { .. } => {
                Err(McpError::NotImplemented("WebSocket transport"))
            }
        }
    }
}

fn parse_mcp_tools_response(response: &str) -> Result<Vec<ToolDef>, McpError> {
    let data: Value = serde_json::from_str(response)
        .map_err(|e| McpError::ParseError(e.to_string()))?;
    parse_mcp_tools_value(&data)
}

fn parse_mcp_tools_value(data: &Value) -> Result<Vec<ToolDef>, McpError> {
    let tools = data["result"]["tools"].as_array()
        .ok_or(McpError::ParseError("No tools array in response".into()))?;

    tools.iter().map(|t| {
        Ok(ToolDef {
            name: t["name"].as_str().unwrap_or("unknown").to_string(),
            description: t["description"].as_str().unwrap_or("").to_string(),
            input_schema: t["inputSchema"].clone(),
        })
    }).collect()
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("Failed to spawn MCP server: {0}")]
    SpawnError(String),
    #[error("Failed to write to MCP server stdin: {0}")]
    WriteError(String),
    #[error("Failed to read MCP server output: {0}")]
    ReadError(String),
    #[error("HTTP request failed: {0}")]
    HttpError(String),
    #[error("Failed to parse MCP response: {0}")]
    ParseError(String),
    #[error("Not implemented: {0}")]
    NotImplemented(&'static str),
}

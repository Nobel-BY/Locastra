use crate::models::{McpServerConfig, McpToolInfo};
use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub async fn list_tools(config: &McpServerConfig) -> Result<Vec<McpToolInfo>, String> {
    let result = request(config, "tools/list", json!({})).await?;
    Ok(result
        .get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|tool| {
            Some(McpToolInfo {
                server_id: config.id.clone(),
                name: tool.get("name")?.as_str()?.to_string(),
                description: tool
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                input_schema: tool
                    .get("inputSchema")
                    .cloned()
                    .unwrap_or_else(|| json!({"type":"object"})),
            })
        })
        .collect())
}

pub async fn call_tool(
    config: &McpServerConfig,
    name: &str,
    arguments: Value,
) -> Result<Value, String> {
    request(
        config,
        "tools/call",
        json!({"name":name,"arguments":arguments}),
    )
    .await
}

async fn request(config: &McpServerConfig, method: &str, params: Value) -> Result<Value, String> {
    if !config.enabled {
        return Err("MCP 服务器未启用".into());
    }
    if config.command.trim().is_empty() {
        return Err("MCP 启动命令不能为空".into());
    }
    let mut command = tokio::process::Command::new(&config.command);
    command
        .args(&config.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    if let Some(directory) = config
        .working_directory
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        if !std::path::Path::new(directory).is_dir() {
            return Err("MCP 工作目录不存在".into());
        }
        command.current_dir(directory);
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.as_std_mut().creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("无法启动 MCP 服务器：{e}"))?;
    let mut stdin = child.stdin.take().ok_or("无法连接 MCP 标准输入")?;
    let stdout = child.stdout.take().ok_or("无法连接 MCP 标准输出")?;
    let mut reader = BufReader::new(stdout);
    send(&mut stdin, json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"LocalDeploy","version":"0.1.0"}}})).await?;
    let initialized = read_response(&mut reader, 1).await?;
    if initialized.get("error").is_some() {
        let _ = child.kill().await;
        return Err(format!("MCP 初始化失败：{}", initialized["error"]));
    }
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","method":"notifications/initialized","params":{}}),
    )
    .await?;
    send(
        &mut stdin,
        json!({"jsonrpc":"2.0","id":2,"method":method,"params":params}),
    )
    .await?;
    let response = read_response(&mut reader, 2).await?;
    let _ = child.kill().await;
    if let Some(error) = response.get("error") {
        return Err(format!("MCP 调用失败：{error}"));
    }
    response
        .get("result")
        .cloned()
        .ok_or("MCP 响应缺少 result".into())
}

async fn send(stdin: &mut tokio::process::ChildStdin, value: Value) -> Result<(), String> {
    let mut data = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    data.push(b'\n');
    stdin.write_all(&data).await.map_err(|e| e.to_string())?;
    stdin.flush().await.map_err(|e| e.to_string())
}

async fn read_response(
    reader: &mut BufReader<tokio::process::ChildStdout>,
    id: u64,
) -> Result<Value, String> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    let mut line = String::new();
    loop {
        line.clear();
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("MCP 服务器响应超时".into());
        }
        let read = tokio::time::timeout(remaining, reader.read_line(&mut line))
            .await
            .map_err(|_| "MCP 服务器响应超时".to_string())?
            .map_err(|e| e.to_string())?;
        if read == 0 {
            return Err("MCP 服务器提前退出".into());
        }
        if let Ok(value) = serde_json::from_str::<Value>(line.trim()) {
            if value.get("id").and_then(Value::as_u64) == Some(id) {
                return Ok(value);
            }
        }
    }
}

use crate::models::{ChatMessage, ChatParams, InstalledModel, RuntimeComponent, RuntimeState};
use base64::Engine;
use futures_util::StreamExt;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap},
    net::TcpListener,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;

#[cfg(target_os = "windows")]
struct KillOnCloseJob(usize);

#[cfg(target_os = "windows")]
impl KillOnCloseJob {
    fn attach(child: &Child) -> Result<Self, String> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::{
            Foundation::{CloseHandle, HANDLE},
            System::JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
                SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            },
        };

        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(format!(
                    "无法创建进程清理任务：{}",
                    std::io::Error::last_os_error()
                ));
            }

            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let configured = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            let assigned = configured != 0
                && AssignProcessToJobObject(handle, child.as_raw_handle() as HANDLE) != 0;
            if !assigned {
                let error = std::io::Error::last_os_error();
                CloseHandle(handle);
                return Err(format!("无法接管推理进程：{error}"));
            }
            Ok(Self(handle as usize))
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for KillOnCloseJob {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
        unsafe {
            CloseHandle(self.0 as HANDLE);
        }
    }
}

pub struct RuntimeManager {
    child: Mutex<Option<Child>>,
    #[cfg(target_os = "windows")]
    job: Mutex<Option<KillOnCloseJob>>,
    state: Mutex<RuntimeState>,
    generations: Mutex<HashMap<String, CancellationToken>>,
    unload_timer: Mutex<Option<CancellationToken>>,
    last_activity: Mutex<Instant>,
    client: reqwest::Client,
    api_key: Mutex<String>,
}

impl RuntimeManager {
    pub fn new() -> Self {
        Self {
            child: Mutex::new(None),
            #[cfg(target_os = "windows")]
            job: Mutex::new(None),
            state: Mutex::new(RuntimeState::default()),
            generations: Mutex::new(HashMap::new()),
            unload_timer: Mutex::new(None),
            last_activity: Mutex::new(Instant::now()),
            api_key: Mutex::new(String::new()),
            // Runtime traffic is always loopback and must never be captured by
            // a VPN/system HTTP proxy.  A short connect timeout still leaves
            // streamed generation itself unlimited.
            client: reqwest::Client::builder()
                .no_proxy()
                .connect_timeout(Duration::from_secs(5))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
    pub fn state(&self) -> RuntimeState {
        let exited = self
            .child
            .lock()
            .as_mut()
            .is_some_and(|child| child.try_wait().ok().flatten().is_some());
        let mut state = self.state.lock();
        if exited && matches!(state.status.as_str(), "starting" | "running") {
            state.status = "error".into();
            state.port = None;
            state.error = Some("本地推理进程已退出，请重新加载模型".into());
        }
        state.clone()
    }
    fn post(&self, url: String) -> reqwest::RequestBuilder {
        let request = self.client.post(url);
        let key = self.api_key.lock().clone();
        if key.is_empty() {
            request
        } else {
            request.bearer_auth(key)
        }
    }
    pub fn unload(&self) -> RuntimeState {
        if let Some(token) = self.unload_timer.lock().take() {
            token.cancel();
        }
        if let Some(mut child) = self.child.lock().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        #[cfg(target_os = "windows")]
        self.job.lock().take();
        for token in self.generations.lock().drain().map(|(_, v)| v) {
            token.cancel();
        }
        let next = RuntimeState::default();
        *self.state.lock() = next.clone();
        next
    }

    pub async fn load(
        self: &Arc<Self>,
        app: &AppHandle,
        model: &InstalledModel,
        draft_model: Option<&InstalledModel>,
        context_size: u32,
        auto_unload_minutes: u32,
        runtime_directory: &str,
        fixed_port: Option<u16>,
        api_key: &str,
    ) -> Result<RuntimeState, String> {
        self.unload();
        let _ = app.emit("runtime-progress", json!({"modelId":model.id,"stage":"checking","percent":8,"message":"正在检查模型文件和运行组件"}));
        if !std::path::Path::new(&model.file_path).is_file() {
            return Err("模型文件不存在，请重新定位或删除记录".into());
        }
        let executable = find_server_in(Some(runtime_directory))?;
        let port = if let Some(port) = fixed_port {
            if TcpListener::bind(("127.0.0.1", port)).is_err() {
                return Err(format!(
                    "本地 API 固定端口 {port} 已被占用，请在设置中更换端口"
                ));
            }
            port
        } else {
            free_port()?
        };
        *self.api_key.lock() = api_key.to_string();
        let threads = if model.config.threads > 0 {
            model.config.threads as usize
        } else {
            std::thread::available_parallelism()
                // llama.cpp generally benefits from physical cores for memory-bound
                // CPU MoE work.  Treat two logical processors as one physical core.
                .map(|n| (n.get() / 2).max(2))
                .unwrap_or(4)
        };
        let gpu_layers = if model.config.gpu_layers >= 999 {
            "auto".to_string()
        } else {
            model.config.gpu_layers.to_string()
        };
        *self.state.lock() = RuntimeState {
            status: "starting".into(),
            model_id: Some(model.id.clone()),
            model_name: Some(model.display_name.clone()),
            port: Some(port),
            error: None,
        };
        let _ = app.emit("runtime-progress", json!({"modelId":model.id,"stage":"starting","percent":22,"message":"正在启动本地推理服务"}));
        let log_directory = dirs::data_local_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalDeploy")
            .join("logs");
        std::fs::create_dir_all(&log_directory)
            .map_err(|e| format!("无法创建运行日志目录：{e}"))?;
        let log_path = log_directory.join("llama-server.log");
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_path)
            .map_err(|e| format!("无法创建运行日志：{e}"))?;
        let mut command = Command::new(&executable);
        command
            .args([
                "-m",
                &model.file_path,
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "-c",
                &context_size.to_string(),
                "--threads",
                &threads.to_string(),
                "-ngl",
                &gpu_layers,
                "--parallel",
                "1",
                "--no-webui",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(log_file));
        if !api_key.trim().is_empty() {
            command.args(["--api-key", api_key]);
        }
        command.args([
            "--flash-attn",
            if model.config.flash_attention {
                "on"
            } else {
                "off"
            },
        ]);
        if model.config.gpu_layers >= 999 && !model.config.oversized_mode {
            // Let llama.cpp measure free VRAM and choose a safe offload plan.
            // This avoids the common failure where "all layers" fits on paper
            // but KV/cache/driver allocations push the process over the limit.
            command.args(["--fit", "on", "--fit-target", "1024"]);
        }
        if model.config.oversized_mode {
            if model
                .file_path
                .to_ascii_lowercase()
                .contains("deepseek-v4-flash")
            {
                // The bundled parameter fitter measured this quant at about
                // 27.5 GiB total device use with 39 CPU MoE layers, leaving a
                // useful safety margin on a 32 GiB RTX 5090.
                command.args(["--n-cpu-moe", "39"]);
            } else {
                command.arg("--cpu-moe");
            }
            command.args([
                "--load-mode",
                "mmap",
                "--no-repack",
                "--fit",
                "on",
                "--fit-target",
                "1024",
                "--batch-size",
                "128",
                "--ubatch-size",
                "64",
                "--cache-type-k",
                "f16",
                "--cache-type-v",
                "f16",
                "--slots",
                "--metrics",
            ]);
        } else if model.config.kv_cache_type != "auto" {
            command.args([
                "--cache-type-k",
                &model.config.kv_cache_type,
                "--cache-type-v",
                &model.config.kv_cache_type,
            ]);
        }
        if !model.config.chat_template.trim().is_empty() {
            command.args(["--chat-template", model.config.chat_template.trim()]);
        }
        if let Some(draft) = draft_model {
            let draft_tokens = model.config.draft_tokens.clamp(1, 32).to_string();
            command.args([
                "--spec-type",
                "draft-simple",
                "--spec-draft-model",
                &draft.file_path,
                "--spec-draft-n-max",
                &draft_tokens,
            ]);
        } else if supports_builtin_mtp(&model.display_name, &model.file_path) {
            // Qwen3.8 GGUFs include an in-model MTP/next-token prediction head.
            // Four draft tokens is a conservative default that improves decode
            // throughput without requiring a second model or much extra VRAM.
            command.args(["--spec-type", "draft-mtp", "--spec-draft-n-max", "4"]);
        }
        if let Some(mmproj) = model
            .config
            .mmproj_path
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            if !std::path::Path::new(mmproj).is_file() {
                return Err("视觉投影文件 mmproj 不存在，请在模型设置中重新选择".into());
            }
            command.args(["--mmproj", mmproj]);
        }
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("无法启动 llama-server：{e}"))?;
        #[cfg(target_os = "windows")]
        {
            let job = KillOnCloseJob::attach(&child).map_err(|error| {
                let _ = child.kill();
                let _ = child.wait();
                error
            })?;
            *self.job.lock() = Some(job);
        }
        *self.child.lock() = Some(child);
        let health = format!("http://127.0.0.1:{port}/health");
        let mut ready = false;
        for attempt in 0..600 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if attempt % 10 == 0 {
                let percent = 30 + (attempt * 60 / 600);
                let _=app.emit("runtime-progress",json!({"modelId":model.id,"stage":"loading","percent":percent,"message":"正在加载权重并分配内存/显存"}));
            }
            if self
                .child
                .lock()
                .as_mut()
                .is_some_and(|c| c.try_wait().ok().flatten().is_some())
            {
                break;
            }
            if let Ok(response) = self.client.get(&health).send().await {
                if response.status().is_success() {
                    ready = true;
                    break;
                }
            }
        }
        if !ready {
            self.unload();
            let details = std::fs::read_to_string(&log_path).unwrap_or_default();
            let tail = details
                .lines()
                .rev()
                .take(8)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n");
            let message = if details.trim().is_empty() {
                "模型启动超时或提前退出。可能是内存不足，或 llama.cpp 不支持该模型。".to_string()
            } else {
                friendly_load_error(&details, &tail)
            };
            let _ = app.emit(
                "runtime-progress",
                json!({"modelId":model.id,"stage":"error","percent":0,"message":message}),
            );
            return Err(message);
        }
        let next = RuntimeState {
            status: "running".into(),
            model_id: Some(model.id.clone()),
            model_name: Some(model.display_name.clone()),
            port: Some(port),
            error: None,
        };
        *self.state.lock() = next.clone();
        let _ = app.emit(
            "runtime-progress",
            json!({"modelId":model.id,"stage":"ready","percent":100,"message":"模型已就绪"}),
        );
        *self.last_activity.lock() = Instant::now();
        if auto_unload_minutes > 0 {
            let token = CancellationToken::new();
            *self.unload_timer.lock() = Some(token.clone());
            let manager = Arc::downgrade(self);
            tauri::async_runtime::spawn(async move {
                let ttl = Duration::from_secs(auto_unload_minutes as u64 * 60);
                loop {
                    tokio::select! {
                        _ = token.cancelled() => return,
                        _ = tokio::time::sleep(Duration::from_secs(15)) => {}
                    }
                    let Some(manager) = manager.upgrade() else {
                        return;
                    };
                    if manager.state().status == "running"
                        && manager.last_activity.lock().elapsed() >= ttl
                    {
                        manager.unload();
                        return;
                    }
                }
            });
        }
        Ok(next)
    }

    pub fn stop(&self, id: &str) {
        if let Some(token) = self.generations.lock().remove(id) {
            token.cancel();
        }
    }

    pub async fn count_tokens(&self, content: &str) -> Result<u64, String> {
        let state = self.state();
        if state.status != "running" {
            return Err("请先加载模型，才能使用模型的真实分词器".into());
        }
        let port = state.port.ok_or("推理端口不可用")?;
        let response = self
            .post(format!("http://127.0.0.1:{port}/tokenize"))
            .json(&json!({"content": content, "add_special": false}))
            .send()
            .await
            .map_err(|e| format!("无法连接模型分词器：{e}"))?;
        if !response.status().is_success() {
            return Err(format!("模型分词器返回 HTTP {}", response.status()));
        }
        let value: Value = response.json().await.map_err(|e| e.to_string())?;
        value
            .get("tokens")
            .and_then(Value::as_array)
            .map(|tokens| tokens.len() as u64)
            .or_else(|| value.get("count").and_then(Value::as_u64))
            .ok_or_else(|| "模型分词器没有返回 Token 数".into())
    }

    pub async fn summarize_context(
        &self,
        transcript: &str,
        previous_summary: Option<&str>,
    ) -> Result<String, String> {
        let state = self.state();
        if state.status != "running" {
            return Err("请先加载模型，才能自动压缩上下文".into());
        }
        let port = state.port.ok_or("推理端口不可用")?;
        let previous = previous_summary.unwrap_or("").trim();
        let prompt = format!(
            "你是 Locastra 的本地上下文整理器。把较早对话压缩成事实密集、可继续对话的中文摘要。\n\
             必须保留：用户目标、明确偏好、关键事实、文件名/路径、代码约束、已作决定、未完成事项和重要引用。\n\
             删除寒暄、重复内容和无用推理；不要新增事实；使用简短分点，最多 900 中文字。\n\n\
             既有摘要：\n{}\n\n本次要合并的较早对话：\n{}",
            if previous.is_empty() { "（无）" } else { previous },
            transcript
        );
        let response = self
            .post(format!("http://127.0.0.1:{port}/v1/chat/completions"))
            .json(&json!({
                "messages": [
                    {"role":"system","content":"只输出压缩摘要，不要解释过程。"},
                    {"role":"user","content":prompt}
                ],
                "stream": false,
                "temperature": 0.15,
                "top_p": 0.9,
                "max_tokens": 1200,
                "chat_template_kwargs": {"enable_thinking": false},
                "reasoning_effort": "none"
            }))
            .send()
            .await
            .map_err(|e| format!("自动压缩上下文失败：{e}"))?;
        let status = response.status();
        let value: Value = response.json().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("自动压缩上下文失败（HTTP {status}）：{value}"));
        }
        value
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|summary| !summary.is_empty())
            .map(str::to_string)
            .ok_or_else(|| "模型没有生成可用的上下文摘要".into())
    }

    pub fn spawn_chat(
        self: Arc<Self>,
        app: AppHandle,
        request_id: String,
        messages: Vec<ChatMessage>,
        params: ChatParams,
        tools: Option<Vec<Value>>,
    ) {
        *self.last_activity.lock() = Instant::now();
        let token = CancellationToken::new();
        self.generations
            .lock()
            .insert(request_id.clone(), token.clone());
        tauri::async_runtime::spawn(async move {
            let result = self
                .chat_inner(&app, &request_id, messages, params, tools, &token)
                .await;
            if let Err(error) = result {
                let _ = app.emit(
                    "chat-token",
                    json!({"requestId":request_id,"token":"","done":true,"error":error}),
                );
            }
            self.generations.lock().remove(&request_id);
        });
    }

    async fn chat_inner(
        &self,
        app: &AppHandle,
        request_id: &str,
        messages: Vec<ChatMessage>,
        params: ChatParams,
        tools: Option<Vec<Value>>,
        token: &CancellationToken,
    ) -> Result<(), String> {
        let state = self.state();
        if state.status != "running" {
            return Err("请先加载模型".into());
        }
        let port = state.port.ok_or("推理端口不可用")?;
        let mut wire = Vec::new();
        if !params.system_prompt.trim().is_empty() && !messages.iter().any(|m| m.role == "system") {
            wire.push(json!({"role":"system","content":params.system_prompt}));
        }
        for message in messages {
            if message.attachments.is_empty() {
                wire.push(json!({"role":message.role,"content":message.content}));
                continue;
            }
            let mut content = vec![json!({"type":"text","text":message.content})];
            for attachment in message.attachments {
                if attachment.file_size > 20 * 1024 * 1024 {
                    return Err("图片附件超过 20 MB".into());
                }
                let bytes = std::fs::read(&attachment.file_path)
                    .map_err(|e| format!("无法读取图片附件 {}：{e}", attachment.name))?;
                let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
                content.push(json!({"type":"image_url","image_url":{"url":format!("data:{};base64,{}",attachment.mime_type,encoded)}}));
            }
            wire.push(json!({"role":message.role,"content":content}));
        }
        let mut request = json!({"messages":wire,"stream":true,"stream_options":{"include_usage":true},"temperature":params.temperature,"top_p":params.top_p,"top_k":params.top_k,"min_p":params.min_p,"repeat_penalty":params.repeat_penalty,"max_tokens":params.max_tokens,"chat_template_kwargs":{"enable_thinking":params.enable_thinking},"reasoning_effort":if params.enable_thinking { "medium" } else { "none" }});
        if let Some(tools) = tools.filter(|items| !items.is_empty()) {
            request["tools"] = json!(tools);
            request["tool_choice"] = json!("auto");
        }
        match params.response_mode.as_str() {
            "json" => request["response_format"] = json!({"type":"json_object"}),
            "schema" => {
                let schema: serde_json::Value = serde_json::from_str(&params.json_schema)
                    .map_err(|e| format!("JSON Schema 格式无效：{e}"))?;
                request["response_format"] = json!({"type":"json_schema","schema":schema});
            }
            "grammar" => {
                if params.grammar.trim().is_empty() {
                    return Err("GBNF 语法不能为空".into());
                }
                request["grammar"] = json!(params.grammar);
            }
            _ => {}
        }
        let response = self
            .post(format!("http://127.0.0.1:{port}/v1/chat/completions"))
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("无法连接本地模型：{e}"))?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let log_path = dirs::data_local_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join("LocalDeploy")
                .join("logs")
                .join("llama-server.log");
            let details = std::fs::read_to_string(log_path).unwrap_or_default();
            let message = friendly_chat_error(status.as_u16(), &body, &details);
            if status == reqwest::StatusCode::BAD_GATEWAY
                || self
                    .child
                    .lock()
                    .as_mut()
                    .is_some_and(|child| child.try_wait().ok().flatten().is_some())
            {
                self.unload();
            }
            return Err(message);
        }
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let started = Instant::now();
        let mut progress_tick = tokio::time::interval(Duration::from_secs(5));
        progress_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        progress_tick.tick().await;
        let mut first_token_ms = None;
        let mut input_tokens = None;
        let mut output_tokens = None;
        let mut prompt_tokens_per_second = None;
        let mut tokens_per_second = None;
        let mut finish_reason: Option<String> = None;
        let mut tool_calls: BTreeMap<u64, (String, String, String)> = BTreeMap::new();
        loop {
            let next = tokio::select! {
                _ = token.cancelled() => {
                    let _ = app.emit("chat-token", json!({"requestId":request_id,"token":"","done":true,"finishReason":"cancelled","stats":{"timeToFirstTokenMs":first_token_ms,"totalTimeMs":started.elapsed().as_millis() as u64}}));
                    return Ok(());
                },
                _ = progress_tick.tick() => {
                    let stage = if first_token_ms.is_some() { "generating" } else { "processing_prompt" };
                    let _ = app.emit("chat-progress", json!({
                        "requestId": request_id,
                        "stage": stage,
                        "elapsedMs": started.elapsed().as_millis() as u64,
                    }));
                    continue;
                },
                value = stream.next() => value,
            };
            let Some(chunk) = next else {
                break;
            };
            let chunk = chunk.map_err(|e| e.to_string())?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(pos) = buffer.find("\n\n") {
                let event = buffer[..pos].to_string();
                buffer.drain(..pos + 2);
                for line in event.lines() {
                    let Some(data) = line.strip_prefix("data: ") else {
                        continue;
                    };
                    if data.trim() == "[DONE]" {
                        let completed_tool_calls = tool_calls
                            .values()
                            .map(|(id, name, arguments)| json!({"id":id,"name":name,"arguments":arguments}))
                            .collect::<Vec<_>>();
                        let _ = app.emit(
                            "chat-token",
                            json!({"requestId":request_id,"token":"","done":true,"finishReason":finish_reason,"toolCalls":completed_tool_calls,"stats":{"inputTokens":input_tokens,"outputTokens":output_tokens,"promptTokensPerSecond":prompt_tokens_per_second,"tokensPerSecond":tokens_per_second,"timeToFirstTokenMs":first_token_ms,"totalTimeMs":started.elapsed().as_millis() as u64}}),
                        );
                        return Ok(());
                    }
                    if let Ok(value) = serde_json::from_str::<Value>(data) {
                        finish_reason = value
                            .pointer("/choices/0/finish_reason")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                            .or(finish_reason);
                        input_tokens = value
                            .pointer("/usage/prompt_tokens")
                            .and_then(Value::as_u64)
                            .or(input_tokens)
                            .or_else(|| value.pointer("/timings/prompt_n").and_then(Value::as_u64));
                        output_tokens = value
                            .pointer("/usage/completion_tokens")
                            .and_then(Value::as_u64)
                            .or(output_tokens)
                            .or_else(|| {
                                value
                                    .pointer("/timings/predicted_n")
                                    .and_then(Value::as_u64)
                            });
                        prompt_tokens_per_second = value
                            .pointer("/timings/prompt_per_second")
                            .and_then(Value::as_f64)
                            .or(prompt_tokens_per_second);
                        tokens_per_second = value
                            .pointer("/timings/predicted_per_second")
                            .and_then(Value::as_f64)
                            .or(tokens_per_second);
                        if let Some(calls) = value
                            .pointer("/choices/0/delta/tool_calls")
                            .and_then(Value::as_array)
                        {
                            for call in calls {
                                let index = call.get("index").and_then(Value::as_u64).unwrap_or(0);
                                let entry = tool_calls.entry(index).or_insert_with(|| {
                                    (String::new(), String::new(), String::new())
                                });
                                if let Some(id) = call.get("id").and_then(Value::as_str) {
                                    entry.0.push_str(id);
                                }
                                if let Some(name) =
                                    call.pointer("/function/name").and_then(Value::as_str)
                                {
                                    entry.1.push_str(name);
                                }
                                if let Some(arguments) =
                                    call.pointer("/function/arguments").and_then(Value::as_str)
                                {
                                    entry.2.push_str(arguments);
                                }
                            }
                        }
                        if let Some(reasoning) = value
                            .pointer("/choices/0/delta/reasoning_content")
                            .and_then(Value::as_str)
                        {
                            if !reasoning.is_empty() && first_token_ms.is_none() {
                                first_token_ms = Some(started.elapsed().as_millis() as u64);
                            }
                            if !reasoning.is_empty() {
                                let _ = app.emit(
                                    "chat-token",
                                    json!({"requestId":request_id,"token":"","reasoning":reasoning,"done":false}),
                                );
                            }
                        }
                        if let Some(content) = value
                            .pointer("/choices/0/delta/content")
                            .and_then(Value::as_str)
                        {
                            if !content.is_empty() && first_token_ms.is_none() {
                                first_token_ms = Some(started.elapsed().as_millis() as u64);
                            }
                            let _ = app.emit(
                                "chat-token",
                                json!({"requestId":request_id,"token":content,"done":false}),
                            );
                        }
                    }
                }
            }
        }
        let completed_tool_calls = tool_calls
            .values()
            .map(|(id, name, arguments)| json!({"id":id,"name":name,"arguments":arguments}))
            .collect::<Vec<_>>();
        let _ = app.emit(
            "chat-token",
            json!({"requestId":request_id,"token":"","done":true,"finishReason":finish_reason.unwrap_or_else(|| "connection_closed".into()),"toolCalls":completed_tool_calls,"stats":{"inputTokens":input_tokens,"outputTokens":output_tokens,"promptTokensPerSecond":prompt_tokens_per_second,"tokensPerSecond":tokens_per_second,"timeToFirstTokenMs":first_token_ms,"totalTimeMs":started.elapsed().as_millis() as u64}}),
        );
        Ok(())
    }
}

impl Drop for RuntimeManager {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.get_mut().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        #[cfg(target_os = "windows")]
        self.job.get_mut().take();
    }
}

fn strip_ansi(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for code in chars.by_ref() {
                if code.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        output.push(character);
    }
    output
}

fn compact_log_tail(details: &str, lines: usize) -> String {
    let cleaned = strip_ansi(details);
    cleaned
        .lines()
        .filter(|line| !line.trim().is_empty())
        .rev()
        .take(lines)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}

fn friendly_load_error(details: &str, fallback_tail: &str) -> String {
    let lower = details.to_ascii_lowercase();
    if lower.contains("outofdevicememory")
        || lower.contains("failed to allocate vulkan")
        || lower.contains("device memory allocation")
    {
        return "模型加载失败：显存不足。应用已让 llama.cpp 自动调整 GPU 卸载层数；如果仍然失败，说明这个量化版本超出了显存与可用内存的组合容量。请改用同模型的 IQ1、IQ2、Q2 版本，或选择参数量更小的模型。".into();
    }
    if lower.contains("bad_alloc") || lower.contains("cannot allocate memory") {
        return "模型加载失败：可用内存不足。请关闭占用内存较高的程序，降低上下文长度，或改用更低量化/更小的模型。".into();
    }
    if lower.contains("unknown model architecture") || lower.contains("unsupported model") {
        return "模型加载失败：当前 llama.cpp 运行组件尚不支持这个模型架构。可在设置中安装较新的 CUDA 组件，或换用已支持的 GGUF 模型。".into();
    }
    let tail = compact_log_tail(details, 8);
    if tail.is_empty() {
        format!("模型加载失败：\n{}", strip_ansi(fallback_tail))
    } else {
        format!("模型加载失败：\n{tail}")
    }
}

fn response_error_detail(body: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(body).ok()?;
    value
        .pointer("/error/message")
        .and_then(Value::as_str)
        .or_else(|| value.get("error").and_then(Value::as_str))
        .or_else(|| value.get("message").and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn friendly_chat_error(status: u16, body: &str, details: &str) -> String {
    let combined = format!("{body}\n{details}");
    let lower = combined.to_ascii_lowercase();
    if lower.contains("outofdevicememory")
        || lower.contains("failed to allocate vulkan")
        || lower.contains("device memory allocation")
    {
        return "生成失败：模型没有真正加载成功，显存不足。请重新加载模型；新版会自动调整 GPU 卸载。如果仍失败，请改用 IQ1/IQ2/Q2 量化或更小模型。".into();
    }
    if lower.contains("context")
        && (lower.contains("exceed")
            || lower.contains("too long")
            || lower.contains("too large")
            || lower.contains("overflow"))
    {
        return "生成失败：当前对话超过模型上下文容量。请删除较早消息、降低最大输出长度，或在对话设置中增大上下文后重新加载模型。".into();
    }
    if lower.contains("chat template") || lower.contains("chat_template") {
        return "生成失败：模型聊天模板不兼容。请到“我的模型 → 模型设置”清空自定义聊天模板后重新加载，或选择模型要求的模板。".into();
    }
    if let Some(message) = response_error_detail(body) {
        return format!("本地模型生成失败（HTTP {status}）：{message}");
    }
    let tail = compact_log_tail(details, 4);
    if !tail.is_empty() {
        format!("本地模型生成失败（HTTP {status}）。推理日志：\n{tail}")
    } else {
        format!("本地模型生成失败（HTTP {status}）。请重新加载模型后再试。")
    }
}

pub(crate) fn find_server_in(runtime_directory: Option<&str>) -> Result<PathBuf, String> {
    const BUNDLED_RUNTIME: &str = "llama-b10333-vulkan";
    const BUNDLED_CUDA_RUNTIME: &str = "llama-b10357-cuda13.3";
    if let Ok(path) = std::env::var("LOCALDEPLOY_LLAMA_SERVER") {
        let p = PathBuf::from(path);
        if p.is_file() {
            return Ok(p);
        }
    }
    let mut candidates = Vec::new();
    if let Some(directory) = runtime_directory.filter(|value| !value.trim().is_empty()) {
        candidates.push(PathBuf::from(directory).join("llama-server.exe"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("llama-server.exe"));
            candidates.push(dir.join("binaries").join("llama-server.exe"));
            if nvidia_driver_available() {
                candidates.push(
                    dir.join("binaries")
                        .join(BUNDLED_CUDA_RUNTIME)
                        .join("llama-server.exe"),
                );
            }
            candidates.push(
                dir.join("binaries")
                    .join(BUNDLED_RUNTIME)
                    .join("llama-server.exe"),
            );
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        if nvidia_driver_available() {
            candidates.push(
                cwd.join("src-tauri")
                    .join("binaries")
                    .join(BUNDLED_CUDA_RUNTIME)
                    .join("llama-server.exe"),
            );
        }
        candidates.push(
            cwd.join("src-tauri")
                .join("binaries")
                .join("llama-server.exe"),
        );
        candidates.push(
            cwd.join("src-tauri")
                .join("binaries")
                .join(BUNDLED_RUNTIME)
                .join("llama-server.exe"),
        );
        candidates.push(cwd.join("binaries").join("llama-server.exe"));
        candidates.push(
            cwd.join("binaries")
                .join(BUNDLED_RUNTIME)
                .join("llama-server.exe"),
        );
    }
    candidates.into_iter().find(|p|p.is_file()).ok_or_else(||"未找到 llama-server.exe。请保留随应用提供的 binaries 目录，或设置 LOCALDEPLOY_LLAMA_SERVER。".into())
}

pub(crate) fn list_runtime_components(active_directory: &str) -> Vec<RuntimeComponent> {
    use std::collections::HashSet;
    const BUNDLED: [(&str, &str, &str); 2] = [
        ("llama-b10357-cuda13.3", "CUDA 13.3", "cuda"),
        ("llama-b10333-vulkan", "Vulkan", "vulkan"),
    ];
    let mut directories: Vec<(PathBuf, String, String, String)> = Vec::new();
    if !active_directory.trim().is_empty() {
        directories.push((
            PathBuf::from(active_directory),
            "自定义/已安装运行组件".into(),
            "custom".into(),
            "custom".into(),
        ));
    }
    if let Some(root) = dirs::data_local_dir() {
        let managed = root.join("LocalDeploy").join("runtimes");
        if let Ok(entries) = std::fs::read_dir(managed) {
            for entry in entries.flatten().filter(|entry| entry.path().is_dir()) {
                let name = entry.file_name().to_string_lossy().into_owned();
                let backend = if name.to_ascii_lowercase().contains("cuda") {
                    "cuda"
                } else if name.to_ascii_lowercase().contains("vulkan") {
                    "vulkan"
                } else {
                    "custom"
                };
                directories.push((entry.path(), name, backend.into(), "managed".into()));
            }
        }
    }
    let mut bases = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            bases.push(parent.join("binaries"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        bases.push(cwd.join("src-tauri").join("binaries"));
        bases.push(cwd.join("binaries"));
    }
    for base in bases {
        for (folder, label, backend) in BUNDLED {
            directories.push((
                base.join(folder),
                format!("内置 {label}"),
                backend.into(),
                "bundled".into(),
            ));
        }
    }
    let normalized_active = std::fs::canonicalize(active_directory).ok();
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for (directory, name, backend_hint, source) in directories {
        let key = std::fs::canonicalize(&directory).unwrap_or_else(|_| directory.clone());
        if !seen.insert(key.clone()) {
            continue;
        }
        let server = directory.join("llama-server.exe");
        if !server.is_file() && source != "custom" {
            continue;
        }
        let lower = directory.to_string_lossy().to_ascii_lowercase();
        let backend = if lower.contains("cuda") || directory.join("ggml-cuda.dll").is_file() {
            "cuda"
        } else if lower.contains("vulkan") || directory.join("ggml-vulkan.dll").is_file() {
            "vulkan"
        } else if backend_hint == "custom" {
            "custom"
        } else {
            "cpu"
        };
        let version = directory
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown")
            .split('-')
            .find(|part| part.starts_with('b') && part[1..].chars().all(|c| c.is_ascii_digit()))
            .unwrap_or("unknown")
            .to_string();
        let active = normalized_active
            .as_ref()
            .is_some_and(|active| active == &key);
        let valid = server.is_file()
            && directory.join("llama.dll").is_file()
            && directory.join("ggml.dll").is_file();
        let mut capabilities = vec!["chat".into(), "tokenize".into(), "single-slot".into()];
        if directory.join("mtmd.dll").is_file() {
            capabilities.push("vision".into());
        }
        if version != "unknown" {
            capabilities.extend(["mtp".into(), "tools".into()]);
        }
        result.push(RuntimeComponent {
            id: format!("{}:{}", source, directory.to_string_lossy()),
            name,
            version,
            backend: backend.into(),
            path: directory.to_string_lossy().into_owned(),
            source,
            active,
            valid,
            capabilities,
            diagnostic: (!valid).then(|| "缺少 llama-server.exe 或核心 DLL".into()),
        });
    }
    result.sort_by_key(|item| (!item.active, !item.valid, item.name.clone()));
    result
}

fn nvidia_driver_available() -> bool {
    #[cfg(target_os = "windows")]
    {
        std::path::Path::new(r"C:\Windows\System32\nvcuda.dll").is_file()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}
fn free_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    listener
        .local_addr()
        .map(|a| a.port())
        .map_err(|e| e.to_string())
}

fn supports_builtin_mtp(display_name: &str, file_path: &str) -> bool {
    [display_name, file_path].iter().any(|value| {
        let normalized = value.to_ascii_lowercase().replace(['-', '_'], "");
        normalized.contains("qwen3.8") || normalized.contains("qwen38")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_and_generation_errors_are_translated() {
        let oom = "ggml_vulkan: vk::Device::allocateMemory: ErrorOutOfDeviceMemory";
        assert!(friendly_load_error(oom, "").contains("显存不足"));
        assert!(friendly_chat_error(502, "", oom).contains("没有真正加载成功"));
        assert!(friendly_chat_error(
            400,
            r#"{"error":{"message":"context is too large and exceeds the limit"}}"#,
            ""
        )
        .contains("上下文容量"));
    }

    #[test]
    fn ansi_sequences_are_removed_from_diagnostics() {
        assert_eq!(strip_ansi("\u{1b}[31merror\u{1b}[0m"), "error");
    }

    #[test]
    fn qwen38_models_enable_builtin_mtp_only_for_matching_names_or_paths() {
        assert!(supports_builtin_mtp(
            "Qwen3.8-27B-UD-Q8_K_XL",
            r"D:\models\qwen.gguf"
        ));
        assert!(supports_builtin_mtp(
            "自定义模型",
            r"D:\models\Qwen3_8-27B-GGUF\model.gguf"
        ));
        assert!(!supports_builtin_mtp(
            "Qwen3.6-27B-UD-Q8_K_XL",
            r"D:\models\qwen3.6.gguf"
        ));
    }
}

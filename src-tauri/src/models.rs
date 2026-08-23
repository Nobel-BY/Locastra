use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub dedicated_memory_bytes: u64,
    pub driver_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareReport {
    pub os: String,
    pub arch: String,
    pub cpu_name: String,
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub instruction_sets: Vec<String>,
    pub total_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub gpus: Vec<GpuInfo>,
    pub free_disk_bytes: u64,
    pub model_directory: String,
    pub acceleration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityRating {
    Smooth,
    Runnable,
    NotRecommended,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSearchResult {
    pub id: String,
    pub repo_id: String,
    pub name: String,
    pub author: String,
    pub source: String,
    pub description: Option<String>,
    pub license: Option<String>,
    pub parameter_count: Option<u64>,
    pub downloads: Option<u64>,
    pub likes: Option<u64>,
    pub tags: Vec<String>,
    pub compatible: bool,
    pub incompatible_reason: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WebSearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub engine: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelFilePart {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelFile {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub sha256: Option<String>,
    pub quantization: Option<String>,
    pub is_gguf: bool,
    pub is_mmproj: bool,
    pub shard_group: Option<String>,
    #[serde(default)]
    pub parts: Vec<ModelFilePart>,
    pub recommended: bool,
    pub compatibility: CompatibilityRating,
    pub compatibility_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub repo_id: String,
    pub file: ModelFile,
    pub source: String,
    pub destination_directory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTask {
    pub id: String,
    pub repo_id: String,
    pub file_name: String,
    pub destination: String,
    pub source: String,
    pub status: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub bytes_per_second: u64,
    pub error: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub parts: Vec<ModelFilePart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledModel {
    pub id: String,
    pub display_name: String,
    pub repo_id: Option<String>,
    pub file_path: String,
    pub file_size: u64,
    pub quantization: Option<String>,
    pub source: String,
    pub installed_at: String,
    pub valid: bool,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub last_used_at: Option<String>,
    #[serde(default)]
    pub use_count: u64,
    #[serde(default)]
    pub config: ModelRuntimeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRuntimeConfig {
    pub context_size: u32,
    pub gpu_layers: i32,
    pub threads: u32,
    pub flash_attention: bool,
    pub kv_cache_type: String,
    pub chat_template: String,
    #[serde(default)]
    pub draft_model_id: Option<String>,
    #[serde(default = "default_draft_tokens")]
    pub draft_tokens: u32,
    #[serde(default)]
    pub mmproj_path: Option<String>,
    #[serde(default)]
    pub oversized_mode: bool,
}

fn default_draft_tokens() -> u32 {
    5
}

impl Default for ModelRuntimeConfig {
    fn default() -> Self {
        Self {
            context_size: 4096,
            gpu_layers: 999,
            threads: 0,
            flash_attention: true,
            kv_cache_type: "auto".into(),
            chat_template: String::new(),
            draft_model_id: None,
            draft_tokens: 5,
            mmproj_path: None,
            oversized_mode: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelBenchmark {
    pub model_id: String,
    pub load_time_ms: u64,
    pub prompt_tokens_per_second: Option<f64>,
    pub generation_tokens_per_second: Option<f64>,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeState {
    pub status: String,
    pub model_id: Option<String>,
    pub model_name: Option<String>,
    pub port: Option<u16>,
    pub error: Option<String>,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            status: "stopped".into(),
            model_id: None,
            model_name: None,
            port: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeComponent {
    pub id: String,
    pub name: String,
    pub version: String,
    pub backend: String,
    pub path: String,
    pub source: String,
    pub active: bool,
    pub valid: bool,
    pub capabilities: Vec<String>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub notes: String,
    pub published_at: Option<String>,
    pub download_url: Option<String>,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    #[serde(default)]
    pub stats: Option<GenerationStats>,
    #[serde(default)]
    pub attachments: Vec<ChatAttachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatAttachment {
    pub id: String,
    pub name: String,
    pub file_path: String,
    pub mime_type: String,
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerationStats {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub prompt_tokens_per_second: Option<f64>,
    pub tokens_per_second: Option<f64>,
    pub time_to_first_token_ms: Option<u64>,
    pub total_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub model_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub messages: Option<Vec<ChatMessage>>,
    #[serde(default)]
    pub folder_id: Option<String>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub params: Option<ChatParams>,
    #[serde(default)]
    pub knowledge_document_ids: Vec<String>,
    #[serde(default)]
    pub context_state: Option<ContextCompressionState>,
    #[serde(default)]
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextCompressionState {
    pub summary: String,
    pub summarized_message_ids: Vec<String>,
    pub original_token_count: u64,
    pub summary_token_count: u64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub model_id: Option<String>,
    pub params: ChatParams,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeDocument {
    pub id: String,
    pub name: String,
    pub file_path: String,
    pub file_type: String,
    pub file_size: u64,
    pub character_count: u64,
    pub created_at: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default = "default_index_status")]
    pub index_status: String,
    #[serde(default)]
    pub index_error: Option<String>,
    #[serde(default)]
    pub chunk_count: u32,
    #[serde(default)]
    pub indexed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSnippet {
    pub chunk_id: String,
    pub document_id: String,
    pub document_name: String,
    pub source_path: String,
    pub chunk_index: u32,
    pub page: Option<u32>,
    pub content: String,
    pub score: f64,
    pub vector_score: f64,
    pub bm25_score: f64,
}

fn default_index_status() -> String {
    "pending".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub model_id: Option<String>,
    pub params: ChatParams,
    #[serde(default)]
    pub knowledge_document_ids: Vec<String>,
    #[serde(default)]
    pub mcp_server_ids: Vec<String>,
    #[serde(default = "default_tool_policy")]
    pub tool_policy: String,
    pub created_at: String,
    pub updated_at: String,
}

fn default_tool_policy() -> String {
    "ask".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectArtifact {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub relative_path: String,
    pub language: String,
    pub content: String,
    pub version: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallLog {
    pub id: String,
    pub project_id: Option<String>,
    pub conversation_id: Option<String>,
    pub server_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_directory: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolInfo {
    pub server_id: String,
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationFolder {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub params: ChatParams,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatParams {
    pub temperature: f64,
    pub context_size: u32,
    pub max_tokens: i32,
    #[serde(default)]
    pub enable_thinking: bool,
    pub system_prompt: String,
    #[serde(default = "default_top_p")]
    pub top_p: f64,
    #[serde(default = "default_top_k")]
    pub top_k: u32,
    #[serde(default)]
    pub min_p: f64,
    #[serde(default = "default_repeat_penalty")]
    pub repeat_penalty: f64,
    #[serde(default = "default_context_policy")]
    pub context_policy: String,
    #[serde(default = "default_response_mode")]
    pub response_mode: String,
    #[serde(default)]
    pub json_schema: String,
    #[serde(default)]
    pub grammar: String,
    #[serde(default = "default_validation_retries")]
    pub validation_retries: u8,
}

fn default_response_mode() -> String {
    "text".into()
}
fn default_validation_retries() -> u8 {
    1
}

fn default_context_policy() -> String {
    "auto".into()
}

fn default_top_p() -> f64 {
    0.95
}
fn default_top_k() -> u32 {
    40
}
fn default_repeat_penalty() -> f64 {
    1.1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub model_directory: String,
    pub preferred_source: String,
    pub custom_mirror: String,
    #[serde(default = "default_true")]
    pub download_bypass_proxy: bool,
    pub download_concurrency: u8,
    pub onboarding_complete: bool,
    pub chat: ChatParams,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_font_size")]
    pub font_size: u8,
    #[serde(default)]
    pub compact_mode: bool,
    #[serde(default)]
    pub minimize_to_tray: bool,
    #[serde(default = "default_auto_unload_minutes")]
    pub auto_unload_minutes: u32,
    #[serde(default)]
    pub runtime_directory: String,
    #[serde(default)]
    pub previous_runtime_directory: String,
    #[serde(default)]
    pub developer_service: DeveloperServiceSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperServiceSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_service_port")]
    pub port: u16,
    #[serde(default)]
    pub api_token: String,
    #[serde(default)]
    pub request_logging: bool,
}

impl Default for DeveloperServiceSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 12345,
            api_token: String::new(),
            request_logging: false,
        }
    }
}
fn default_service_port() -> u16 {
    12345
}

fn default_true() -> bool {
    true
}
fn default_theme() -> String {
    "dark".into()
}
fn default_font_size() -> u8 {
    14
}
fn default_auto_unload_minutes() -> u32 {
    60
}

pub fn quantization_from_name(name: &str) -> Option<String> {
    let upper = name.to_ascii_uppercase();
    let re = regex::Regex::new(r"(IQ[1-4]_[A-Z0-9_]+|Q[2-8]_[A-Z0-9_]+|F16|BF16|FP16)").ok()?;
    re.captures(&upper)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

pub fn shard_group(name: &str) -> Option<String> {
    let re = regex::Regex::new(r"(?i)(.+)-\d{5}-of-\d{5}\.gguf$").ok()?;
    re.captures(name)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

pub fn shard_sequence(name: &str) -> Option<(usize, usize)> {
    let re = regex::Regex::new(r"(?i)-([0-9]{5})-of-([0-9]{5})\.gguf$").ok()?;
    let captures = re.captures(name)?;
    Some((
        captures.get(1)?.as_str().parse().ok()?,
        captures.get(2)?.as_str().parse().ok()?,
    ))
}

pub fn belongs_to_same_shard_set(anchor: &str, candidate: &str) -> bool {
    let Some(anchor_group) = shard_group(anchor) else {
        return false;
    };
    let Some(candidate_group) = shard_group(candidate) else {
        return false;
    };
    let Some((_, anchor_total)) = shard_sequence(anchor) else {
        return false;
    };
    let Some((candidate_index, candidate_total)) = shard_sequence(candidate) else {
        return false;
    };
    anchor_group.eq_ignore_ascii_case(&candidate_group)
        && anchor_total == candidate_total
        && candidate_index > 0
        && candidate_index <= candidate_total
        && candidate_total <= 999
}

pub fn rate_file(
    size: u64,
    available_memory: u64,
    dedicated_gpu_memory: u64,
) -> (CompatibilityRating, String) {
    if size == 0 {
        return (CompatibilityRating::Unknown, "文件大小未知".into());
    }
    let required = (size as f64 * 1.08) as u64 + 1024 * 1024 * 1024;
    let gib = |bytes: u64| bytes as f64 / 1024f64.powi(3);
    if dedicated_gpu_memory > 0
        && required <= dedicated_gpu_memory.saturating_mul(9).saturating_div(10)
    {
        (
            CompatibilityRating::Smooth,
            format!("预计约 {:.1} GB，可主要放入显存，速度最佳", gib(required)),
        )
    } else {
        let combined = available_memory
            .saturating_add(dedicated_gpu_memory.saturating_mul(95).saturating_div(100));
        if required <= combined.saturating_mul(78).saturating_div(100) {
            (
                CompatibilityRating::Smooth,
                format!("预计约 {:.1} GB，可用显存与内存混合加速", gib(required)),
            )
        } else if required <= combined.saturating_mul(95).saturating_div(100) {
            (
                CompatibilityRating::Runnable,
                format!(
                    "预计约 {:.1} GB，接近本机可用容量，建议关闭其他程序",
                    gib(required)
                ),
            )
        } else {
            (
                CompatibilityRating::NotRecommended,
                format!(
                    "预计需要 {:.1} GB；当前可用内存与显存合计约 {:.1} GB",
                    gib(required),
                    gib(combined)
                ),
            )
        }
    }
}

pub fn model_capacity_error(
    model_size: u64,
    available_memory: u64,
    dedicated_gpu_memory: u64,
    context_size: u32,
) -> Option<String> {
    if model_size == 0 {
        return None;
    }
    let context_reserve =
        u64::from(context_size.max(4096).div_ceil(4096)).saturating_mul(512 * 1024 * 1024);
    let estimated_required = model_size
        .saturating_mul(106)
        .saturating_div(100)
        .saturating_add(context_reserve);
    let usable_capacity = available_memory
        .saturating_add(dedicated_gpu_memory.saturating_mul(95).saturating_div(100));
    if usable_capacity == 0
        || estimated_required <= usable_capacity.saturating_mul(120).saturating_div(100)
    {
        return None;
    }
    let gib = |bytes: u64| bytes as f64 / 1024f64.powi(3);
    Some(format!(
        "无法加载：该模型文件共 {:.1} GB，预计至少需要约 {:.1} GB 的内存与显存，而当前可用内存约 {:.1} GB、显存约 {:.1} GB。请改用同模型的 IQ1/IQ2/Q2 量化，或选择参数量更小的模型。",
        gib(model_size),
        gib(estimated_required),
        gib(available_memory),
        gib(dedicated_gpu_memory)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_quantization() {
        assert_eq!(
            quantization_from_name("Qwen-4B-Q4_K_M.gguf").as_deref(),
            Some("Q4_K_M")
        );
    }
    #[test]
    fn parses_shard() {
        assert_eq!(
            shard_group("model-Q4_K_M-00001-of-00003.gguf").as_deref(),
            Some("model-Q4_K_M")
        );
        assert_eq!(
            shard_sequence("model-Q4_K_M-00002-of-00003.gguf"),
            Some((2, 3))
        );
        assert!(belongs_to_same_shard_set(
            "model-Q4_K_M-00001-of-00003.gguf",
            "model-Q4_K_M-00003-of-00003.gguf"
        ));
        assert!(!belongs_to_same_shard_set(
            "model-Q4_K_M-00001-of-00003.gguf",
            "model-Q8_0-00001-of-00003.gguf"
        ));
        assert!(!belongs_to_same_shard_set(
            "model-Q4_K_M-00001-of-00003.gguf",
            "model-Q4_K_M-00001-of-00005.gguf"
        ));
    }
    #[test]
    fn rates_memory() {
        assert_eq!(
            rate_file(2 * 1024u64.pow(3), 16 * 1024u64.pow(3), 0).0,
            CompatibilityRating::Smooth
        );
    }

    #[test]
    fn rates_model_that_fits_in_vram_as_smooth() {
        let (rating, reason) =
            rate_file(12 * 1024u64.pow(3), 4 * 1024u64.pow(3), 24 * 1024u64.pow(3));
        assert_eq!(rating, CompatibilityRating::Smooth);
        assert!(reason.contains("显存"));
    }

    #[test]
    fn old_settings_default_to_direct_downloads() {
        let settings: AppSettings = serde_json::from_value(serde_json::json!({
            "modelDirectory": "D:/models",
            "preferredSource": "modelscope",
            "customMirror": "",
            "downloadConcurrency": 4,
            "onboardingComplete": true,
            "chat": {
                "temperature": 0.7,
                "contextSize": 4096,
                "maxTokens": 2048,
                "systemPrompt": "test"
            }
        }))
        .expect("legacy settings should deserialize");
        assert!(settings.download_bypass_proxy);
    }

    #[test]
    fn accepts_llama_cpp_unlimited_output_sentinel() {
        let params: ChatParams = serde_json::from_value(serde_json::json!({
            "temperature": 0.7,
            "contextSize": 16384,
            "maxTokens": -1,
            "enableThinking": false,
            "systemPrompt": "test"
        }))
        .expect("llama.cpp unlimited output should deserialize");
        assert_eq!(params.max_tokens, -1);
    }

    #[test]
    fn blocks_models_far_beyond_combined_capacity() {
        let error = model_capacity_error(
            150 * 1024u64.pow(3),
            22 * 1024u64.pow(3),
            32 * 1024u64.pow(3),
            16384,
        )
        .expect("oversized model should be blocked");
        assert!(error.contains("150.0 GB"));
        assert!(model_capacity_error(
            20 * 1024u64.pow(3),
            22 * 1024u64.pow(3),
            32 * 1024u64.pow(3),
            4096,
        )
        .is_none());
    }
}

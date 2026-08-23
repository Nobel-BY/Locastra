use crate::models::{
    quantization_from_name, rate_file, shard_group, shard_sequence, CompatibilityRating, ModelFile,
    ModelFilePart, ModelSearchResult,
};
use reqwest::Client;
use serde_json::Value;
use std::collections::BTreeMap;

const HF_MIRROR_BASE: &str = "https://hf-mirror.com";

pub struct ModelSources {
    client: Client,
    download_client: Client,
    direct_download_client: Client,
}

impl ModelSources {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .user_agent("Locastra/0.5 (+local Windows app)")
            .connect_timeout(std::time::Duration::from_secs(12))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;
        let download_client = Client::builder()
            .user_agent("Locastra/0.5 (+local Windows app)")
            .connect_timeout(std::time::Duration::from_secs(20))
            .read_timeout(std::time::Duration::from_secs(120))
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .tcp_nodelay(true)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(8)
            .http2_keep_alive_interval(std::time::Duration::from_secs(30))
            .http2_keep_alive_timeout(std::time::Duration::from_secs(20))
            .http2_keep_alive_while_idle(true)
            .build()
            .map_err(|e| e.to_string())?;
        let direct_download_client = Client::builder()
            .user_agent("Locastra/0.5 (+local Windows app)")
            .connect_timeout(std::time::Duration::from_secs(20))
            .read_timeout(std::time::Duration::from_secs(120))
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .tcp_nodelay(true)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(8)
            .http2_keep_alive_interval(std::time::Duration::from_secs(30))
            .http2_keep_alive_timeout(std::time::Duration::from_secs(20))
            .http2_keep_alive_while_idle(true)
            .no_proxy()
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            client,
            download_client,
            direct_download_client,
        })
    }

    pub fn download_client(&self, bypass_proxy: bool) -> Client {
        if bypass_proxy {
            self.direct_download_client.clone()
        } else {
            self.download_client.clone()
        }
    }

    pub async fn search(
        &self,
        query: &str,
        source: &str,
    ) -> Result<Vec<ModelSearchResult>, String> {
        match source {
            "modelscope" => self.search_modelscope(query).await,
            "huggingface" => self.search_huggingface(query).await,
            "all" => {
                let (modelscope, huggingface) = tokio::join!(
                    self.search_modelscope(query),
                    self.search_huggingface(query)
                );
                match (modelscope, huggingface) {
                    (Ok(modelscope), Ok(huggingface)) => {
                        Ok(merge_search_results(modelscope, huggingface))
                    }
                    (Ok(items), Err(_)) | (Err(_), Ok(items)) => Ok(items),
                    (Err(modelscope_error), Err(huggingface_error)) => Err(format!(
                        "两个模型站点均不可用。魔搭：{modelscope_error}；Hugging Face：{huggingface_error}"
                    )),
                }
            }
            _ => Err("未知模型来源".into()),
        }
    }

    async fn search_huggingface(&self, query: &str) -> Result<Vec<ModelSearchResult>, String> {
        let url = format!("https://huggingface.co/api/models?search={}&filter=gguf&sort=downloads&direction=-1&limit=30&full=true", urlencoding::encode(query));
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(network_error)?
            .error_for_status()
            .map_err(network_error)?;
        let values: Vec<Value> = response.json().await.map_err(network_error)?;
        Ok(values
            .into_iter()
            .filter_map(|v| {
                let repo_id = text(&v, &["id", "modelId"])?;
                if v.get("private").and_then(Value::as_bool).unwrap_or(false)
                    || v.get("gated")
                        .is_some_and(|g| g != &Value::Bool(false) && !g.is_null())
                {
                    return None;
                }
                let tags = strings(v.get("tags"));
                let is_gguf = tags.iter().any(|t| t.eq_ignore_ascii_case("gguf"))
                    || repo_id.to_ascii_lowercase().contains("gguf");
                let text_candidate = is_text_model_candidate(&repo_id, &tags);
                let author = text(&v, &["author"])
                    .unwrap_or_else(|| repo_id.split('/').next().unwrap_or("community").into());
                let name = repo_id
                    .split('/')
                    .next_back()
                    .unwrap_or(&repo_id)
                    .replace("-GGUF", "")
                    .replace('-', " ");
                Some(ModelSearchResult {
                    id: format!("huggingface:{repo_id}"),
                    repo_id,
                    name,
                    author,
                    source: "huggingface".into(),
                    description: v
                        .pointer("/cardData/model_description")
                        .and_then(Value::as_str)
                        .map(truncate),
                    license: text(&v, &["license"]).or_else(|| {
                        v.pointer("/cardData/license")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    }),
                    parameter_count: v
                        .pointer("/gguf/total")
                        .and_then(Value::as_u64)
                        .or_else(|| v.pointer("/safetensors/total").and_then(Value::as_u64)),
                    downloads: number(&v, &["downloads", "downloadsAllTime"]),
                    likes: number(&v, &["likes"]),
                    tags: tags.into_iter().take(8).collect(),
                    compatible: is_gguf && text_candidate,
                    incompatible_reason: if !is_gguf {
                        Some("仓库未标记为 GGUF".into())
                    } else if !text_candidate {
                        Some("首版仅支持文本聊天模型，不支持视觉、多模态或 LoRA".into())
                    } else {
                        None
                    },
                    updated_at: text(&v, &["lastModified", "last_modified"]),
                })
            })
            .collect())
    }

    async fn search_modelscope(&self, query: &str) -> Result<Vec<ModelSearchResult>, String> {
        let search_term = if query.to_ascii_lowercase().contains("gguf") {
            query.trim().to_string()
        } else {
            format!("{} GGUF", query.trim())
        };
        // ModelScope's documented model-list OpenAPI currently ignores its `search`
        // query parameter. The public model centre itself uses this endpoint and body.
        // Keeping the request identical to the website prevents unrelated global-list
        // results from masquerading as search matches.
        let urls = [
            "https://www.modelscope.cn/api/v1/dolphin/models",
            "https://modelscope.cn/api/v1/dolphin/models",
        ];
        let request_body = serde_json::json!({
            "PageSize": 30,
            "PageNumber": 1,
            "SortBy": "Default",
            "Target": "",
            "SingleCriterion": [],
            "Name": search_term,
            "Criterion": []
        });
        let mut last_error = String::new();
        for url in urls {
            match self
                .client
                .put(url)
                .header("x-modelscope-accept-language", "zh_CN")
                .json(&request_body)
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    let value: Value = response.json().await.map_err(network_error)?;
                    let entries = find_model_array(&value);
                    let items: Vec<_> = entries
                        .into_iter()
                        .filter_map(|v| model_scope_result(&v))
                        // Keep unsupported visual GGUF repositories visible so the UI
                        // can explain why they are disabled, but omit unrelated formats.
                        .filter(|model| {
                            model.repo_id.to_ascii_lowercase().contains("gguf")
                                || model
                                    .tags
                                    .iter()
                                    .any(|tag| tag.eq_ignore_ascii_case("gguf"))
                        })
                        .collect();
                    if !items.is_empty() {
                        return Ok(items);
                    }
                    last_error = "魔搭没有返回匹配的 GGUF 仓库".into();
                }
                Ok(response) => last_error = format!("HTTP {}", response.status()),
                Err(e) => last_error = e.to_string(),
            }
        }
        Err(if last_error.is_empty() {
            "魔搭接口没有返回可解析的模型".into()
        } else {
            format!("无法连接魔搭社区：{last_error}")
        })
    }

    pub async fn files(
        &self,
        repo_id: &str,
        source: &str,
        available_memory: u64,
        dedicated_gpu_memory: u64,
    ) -> Result<Vec<ModelFile>, String> {
        let raw = if source == "huggingface" {
            self.hf_files(repo_id).await?
        } else {
            self.ms_files(repo_id).await?
        };
        let files = group_model_files(raw, available_memory, dedicated_gpu_memory);
        if files.is_empty() {
            return Err("该仓库没有可用的文本 GGUF 文件（视觉投影文件不会显示）".into());
        }
        Ok(files)
    }

    async fn hf_files(&self, repo_id: &str) -> Result<Vec<(String, u64, Option<String>)>, String> {
        let url = format!("https://huggingface.co/api/models/{repo_id}/tree/main?recursive=true");
        let value: Value = self
            .client
            .get(url)
            .send()
            .await
            .map_err(network_error)?
            .error_for_status()
            .map_err(network_error)?
            .json()
            .await
            .map_err(network_error)?;
        let entries = value.as_array().ok_or("Hugging Face 未返回完整文件树")?;
        Ok(entries
            .iter()
            .filter_map(|v| {
                let path = text(v, &["path", "rfilename"])?;
                let size = number(v, &["size"])
                    .or_else(|| v.pointer("/lfs/size").and_then(Value::as_u64))
                    .unwrap_or(0);
                let sha = text(v, &["sha256"])
                    .or_else(|| {
                        v.pointer("/lfs/sha256")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
                    .or_else(|| {
                        v.pointer("/lfs/oid")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    });
                Some((path, size, sha))
            })
            .collect())
    }

    async fn ms_files(&self, repo_id: &str) -> Result<Vec<(String, u64, Option<String>)>, String> {
        let url=format!("https://www.modelscope.cn/api/v1/models/{repo_id}/repo/files?Revision=master&Recursive=true");
        let value: Value = self
            .client
            .get(url)
            .send()
            .await
            .map_err(network_error)?
            .error_for_status()
            .map_err(network_error)?
            .json()
            .await
            .map_err(network_error)?;
        let entries = find_file_array(&value);
        Ok(entries
            .into_iter()
            .filter_map(|v| {
                let path = text(&v, &["Path", "path", "Name", "name"])?;
                let size = number(&v, &["Size", "size", "FileSize"]).unwrap_or(0);
                let sha = text(&v, &["Sha256", "sha256", "Oid", "oid"]);
                Some((path, size, sha))
            })
            .collect())
    }

    pub fn download_urls(
        repo_id: &str,
        path: &str,
        source: &str,
        mirror: &str,
    ) -> Vec<(String, String)> {
        let encoded_path = path
            .split('/')
            .map(|p| urlencoding::encode(p).to_string())
            .collect::<Vec<_>>()
            .join("/");
        let mut urls = Vec::new();
        if source == "modelscope" {
            urls.push((
                "modelscope".into(),
                format!("https://modelscope.cn/models/{repo_id}/resolve/master/{encoded_path}"),
            ));
            return urls;
        }
        let custom_mirror = mirror.trim().trim_end_matches('/');
        if !custom_mirror.is_empty() && custom_mirror.starts_with("https://") {
            urls.push((
                "custom".into(),
                format!("{custom_mirror}/{repo_id}/resolve/main/{encoded_path}"),
            ));
        }
        if !custom_mirror.eq_ignore_ascii_case(HF_MIRROR_BASE) {
            urls.push((
                "hf-mirror".into(),
                format!("{HF_MIRROR_BASE}/{repo_id}/resolve/main/{encoded_path}"),
            ));
        }
        urls.push((
            "huggingface".into(),
            format!("https://huggingface.co/{repo_id}/resolve/main/{encoded_path}?download=true"),
        ));
        urls
    }
}

fn group_model_files(
    raw: Vec<(String, u64, Option<String>)>,
    available_memory: u64,
    dedicated_gpu_memory: u64,
) -> Vec<ModelFile> {
    let mut grouped: BTreeMap<String, (Option<String>, Vec<ModelFilePart>)> = BTreeMap::new();
    for (path, size_bytes, sha256) in raw {
        let Some(name) = path.split('/').next_back().map(str::to_string) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        if !lower.ends_with(".gguf") || lower.contains("mmproj") || lower.starts_with("dspark-") {
            continue;
        }
        let group = shard_group(&name);
        let parent = path.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
        let key = group
            .as_ref()
            .map(|value| format!("{parent}\0{value}"))
            .unwrap_or_else(|| path.clone());
        grouped
            .entry(key)
            .or_insert_with(|| (group, Vec::new()))
            .1
            .push(ModelFilePart {
                path,
                name,
                size_bytes,
                sha256,
            });
    }

    let mut files = Vec::new();
    for (_, (group, mut parts)) in grouped {
        parts.sort_by_key(|part| shard_sequence(&part.name).map(|v| v.0).unwrap_or(1));
        if let Some((_, expected)) = parts.first().and_then(|part| shard_sequence(&part.name)) {
            if parts.len() != expected {
                continue;
            }
        }
        let Some(first) = parts.first().cloned() else {
            continue;
        };
        let size_known = parts.iter().all(|part| part.size_bytes > 0);
        let size_bytes = if size_known {
            parts.iter().map(|part| part.size_bytes).sum()
        } else {
            0
        };
        let quantization = quantization_from_name(group.as_deref().unwrap_or(&first.name));
        let (compatibility, compatibility_reason) =
            rate_file(size_bytes, available_memory, dedicated_gpu_memory);
        files.push(ModelFile {
            path: first.path.clone(),
            name: first.name.clone(),
            size_bytes,
            sha256: (parts.len() == 1).then(|| first.sha256.clone()).flatten(),
            quantization,
            is_gguf: true,
            is_mmproj: false,
            shard_group: group,
            parts,
            recommended: false,
            compatibility,
            compatibility_reason,
        });
    }
    if let Some(index) = balanced_recommendation(&files) {
        files[index].recommended = true;
    }
    files.sort_by_key(|file| (!file.recommended, file.size_bytes == 0, file.size_bytes));
    files
}

fn merge_search_results(
    modelscope: Vec<ModelSearchResult>,
    huggingface: Vec<ModelSearchResult>,
) -> Vec<ModelSearchResult> {
    let mut merged = BTreeMap::<String, ModelSearchResult>::new();
    for item in modelscope.into_iter().chain(huggingface) {
        let key = item.repo_id.to_ascii_lowercase();
        match merged.get(&key) {
            Some(existing)
                if existing.source == "modelscope"
                    || existing.downloads.unwrap_or(0) >= item.downloads.unwrap_or(0) => {}
            _ => {
                merged.insert(key, item);
            }
        }
    }
    let mut items = merged.into_values().collect::<Vec<_>>();
    items.sort_by_key(|item| std::cmp::Reverse(item.downloads.unwrap_or(0)));
    items
}

fn balanced_recommendation(files: &[ModelFile]) -> Option<usize> {
    files
        .iter()
        .enumerate()
        .filter(|(_, file)| {
            file.size_bytes > 0
                && !matches!(file.compatibility, CompatibilityRating::NotRecommended)
        })
        .min_by_key(|(_, file)| {
            let quality = quantization_quality(file.quantization.as_deref());
            let target_distance = quality.abs_diff(50);
            let fit_penalty = match file.compatibility {
                CompatibilityRating::Smooth => 0,
                CompatibilityRating::Runnable => 18,
                CompatibilityRating::Unknown => 50,
                CompatibilityRating::NotRecommended => 100,
            };
            target_distance + fit_penalty
        })
        .map(|(index, _)| index)
}

fn quantization_quality(value: Option<&str>) -> u32 {
    let value = value.unwrap_or_default().to_ascii_uppercase();
    if value.contains("BF16") || value.contains("F16") {
        100
    } else if value.starts_with("Q8") {
        82
    } else if value.starts_with("Q6") {
        72
    } else if value.starts_with("Q5") {
        62
    } else if value.starts_with("Q4") {
        50
    } else if value.starts_with("Q3") || value.starts_with("IQ3") {
        38
    } else if value.starts_with("Q2") || value.starts_with("IQ2") {
        27
    } else if value.starts_with("IQ1") {
        12
    } else {
        45
    }
}

fn network_error(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "网络请求超时，请检查连接或切换下载源".into()
    } else {
        error.to_string()
    }
}
fn text(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|k| value.get(*k).and_then(Value::as_str).map(str::to_string))
}
fn non_empty_text(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}
fn number(value: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|k| {
        value.get(*k).and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_i64().map(|n| n.max(0) as u64))
                .or_else(|| v.as_str()?.parse().ok())
        })
    })
}
fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}
fn truncate(s: &str) -> String {
    let mut chars = s.chars();
    let value: String = chars.by_ref().take(180).collect();
    if chars.next().is_some() {
        format!("{value}…")
    } else {
        value
    }
}

fn is_text_model_candidate(repo_id: &str, tags: &[String]) -> bool {
    let haystack = format!("{} {}", repo_id, tags.join(" ")).to_ascii_lowercase();
    let unsupported_markers = [
        "image-text-to-text",
        "vision-language",
        "multimodal",
        "visual-question-answering",
        "text-to-image",
        "llava",
        "mmproj",
        "qwen2-vl",
        "qwen2.5-vl",
        "qwen3-vl",
        "minicpm-v",
        "adapter",
        "lora",
    ];
    !unsupported_markers
        .iter()
        .any(|marker| haystack.contains(marker))
}

fn find_model_array(value: &Value) -> Vec<Value> {
    for pointer in [
        "/Data/Model/Models",
        "/Data/Models",
        "/Data/data",
        "/data/Models",
        "/data/models",
        "/models",
        "/Data",
    ] {
        if let Some(a) = value.pointer(pointer).and_then(Value::as_array) {
            return a.clone();
        }
    }
    value.as_array().cloned().unwrap_or_default()
}
fn find_file_array(value: &Value) -> Vec<Value> {
    for pointer in [
        "/Data/Files",
        "/Data/files",
        "/data/Files",
        "/data/files",
        "/Files",
        "/files",
    ] {
        if let Some(a) = value.pointer(pointer).and_then(Value::as_array) {
            return a.clone();
        }
    }
    Vec::new()
}
fn model_scope_result(v: &Value) -> Option<ModelSearchResult> {
    if v.get("private").and_then(Value::as_bool).unwrap_or(false)
        || v.get("gated").and_then(Value::as_bool).unwrap_or(false)
    {
        return None;
    }
    let repo_id = match (
        non_empty_text(v, &["Path", "Owner", "author"]),
        non_empty_text(v, &["Name"]),
    ) {
        (Some(path), Some(name)) if !path.contains('/') => format!("{path}/{name}"),
        _ => non_empty_text(v, &["ModelId", "modelId", "id", "Path", "Name"])?.to_string(),
    };
    let tags = strings(v.get("Tags").or_else(|| v.get("tags")));
    let is_gguf = tags.iter().any(|tag| {
        let tag = tag.to_ascii_lowercase();
        tag == "gguf" || tag.ends_with(":gguf")
    }) || repo_id.to_ascii_lowercase().contains("gguf");
    let text_candidate = is_text_model_candidate(&repo_id, &tags);
    let author = non_empty_text(v, &["Path", "Owner", "author", "CreatedBy"])
        .unwrap_or_else(|| repo_id.split('/').next().unwrap_or("community").into());
    let name = non_empty_text(v, &["ChineseName", "DisplayName", "display_name", "Name"])
        .unwrap_or_else(|| {
            repo_id
                .split('/')
                .next_back()
                .unwrap_or(&repo_id)
                .replace("-GGUF", "")
                .replace('-', " ")
        });
    Some(ModelSearchResult {
        id: format!("modelscope:{repo_id}"),
        repo_id,
        name,
        author,
        source: "modelscope".into(),
        description: text(v, &["Description", "description"]).map(|s| truncate(&s)),
        license: text(v, &["License", "license"]),
        parameter_count: number(v, &["ParameterCount", "parameterCount", "params"]),
        downloads: number(v, &["Downloads", "downloads", "DownloadCount"]),
        likes: number(v, &["Likes", "likes", "Stars"]),
        tags: tags.into_iter().take(8).collect(),
        compatible: is_gguf && text_candidate,
        incompatible_reason: if !is_gguf {
            Some("仓库未标记为 GGUF".into())
        } else if !text_candidate {
            Some("首版仅支持文本聊天模型，不支持视觉、多模态或 LoRA".into())
        } else {
            None
        },
        updated_at: text(
            v,
            &[
                "LastUpdatedTime",
                "lastModified",
                "UpdatedAt",
                "last_modified",
                "created_at",
            ],
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn orders_primary_source() {
        let u = ModelSources::download_urls("a/b", "x.gguf", "modelscope", "");
        assert_eq!(u[0].0, "modelscope");
        assert_eq!(u.len(), 1);
    }
    #[test]
    fn rejects_http_mirror() {
        let u = ModelSources::download_urls("a/b", "x.gguf", "huggingface", "http://bad");
        assert!(!u.iter().any(|x| x.0 == "custom"));
        assert_eq!(u[0].0, "hf-mirror");
    }
    #[test]
    fn prioritizes_custom_then_builtin_hf_mirror() {
        let u = ModelSources::download_urls(
            "a/b",
            "folder/x y.gguf",
            "huggingface",
            "https://custom.example/",
        );
        assert_eq!(u[0].0, "custom");
        assert_eq!(u[1].0, "hf-mirror");
        assert!(u[1]
            .1
            .contains("hf-mirror.com/a/b/resolve/main/folder/x%20y.gguf"));
        assert_eq!(u[2].0, "huggingface");
    }
    #[test]
    fn parses_modelscope_openapi_model() {
        let value = serde_json::json!({
            "id": "unsloth/DeepSeek-R1-Distill-Qwen-7B-GGUF",
            "display_name": "DeepSeek R1 7B GGUF",
            "downloads": 643087,
            "likes": 137,
            "license": "apache-2.0",
            "params": 7615616512u64,
            "tags": ["library:gguf", "task:text-generation"],
            "private": false,
            "gated": false,
            "last_modified": "2025-01-25T12:55:14Z"
        });
        let model = model_scope_result(&value).expect("public GGUF model");
        assert!(model.compatible);
        assert_eq!(model.parameter_count, Some(7615616512));
        assert_eq!(model.updated_at.as_deref(), Some("2025-01-25T12:55:14Z"));
    }

    #[test]
    fn parses_modelscope_model_centre_response() {
        let response = serde_json::json!({
            "Code": 200,
            "Data": {
                "Model": {
                    "Models": [{
                        "Path": "unsloth",
                        "Name": "Qwen3.8-27B-GGUF",
                        "ChineseName": "",
                        "Downloads": 80163,
                        "Stars": 103,
                        "License": "apache-2.0",
                        "Libraries": ["gguf", "pytorch"],
                        "Tags": ["unsloth", "gguf"],
                        "Tasks": [{"Name": "text-generation"}]
                    }],
                    "TotalCount": 348
                }
            }
        });
        let entries = find_model_array(&response);
        assert_eq!(entries.len(), 1);
        let model = model_scope_result(&entries[0]).expect("public GGUF model");
        assert_eq!(model.repo_id, "unsloth/Qwen3.8-27B-GGUF");
        assert_eq!(model.author, "unsloth");
        assert_eq!(model.name, "Qwen3.8-27B-GGUF");
        assert_eq!(model.downloads, Some(80163));
        assert!(model.compatible);
    }

    #[test]
    fn rejects_visual_and_lora_repositories() {
        assert!(!is_text_model_candidate(
            "owner/Qwen3-VL-GGUF",
            &["gguf".into(), "image-text-to-text".into()]
        ));
        assert!(!is_text_model_candidate(
            "owner/model-LoRA-GGUF",
            &["gguf".into()]
        ));
        assert!(is_text_model_candidate(
            "owner/Qwen3-8B-GGUF",
            &["gguf".into(), "text-generation".into()]
        ));
    }

    #[test]
    fn merged_search_prefers_modelscope_for_same_repository() {
        let base = ModelSearchResult {
            id: "huggingface:owner/model".into(),
            repo_id: "owner/model".into(),
            name: "model".into(),
            author: "owner".into(),
            source: "huggingface".into(),
            description: None,
            license: None,
            parameter_count: None,
            downloads: Some(100),
            likes: None,
            tags: vec!["gguf".into()],
            compatible: true,
            incompatible_reason: None,
            updated_at: None,
        };
        let mut domestic = base.clone();
        domestic.id = "modelscope:owner/model".into();
        domestic.source = "modelscope".into();
        domestic.downloads = Some(1);
        let merged = merge_search_results(vec![domestic], vec![base]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].source, "modelscope");
    }

    #[test]
    fn balanced_recommendation_prefers_q4_over_larger_quantization() {
        let files = group_model_files(
            vec![
                ("model-Q4_K_M.gguf".into(), 4 * 1024u64.pow(3), None),
                ("model-Q8_0.gguf".into(), 8 * 1024u64.pow(3), None),
            ],
            24 * 1024u64.pow(3),
            0,
        );
        assert_eq!(
            files
                .iter()
                .find(|file| file.recommended)
                .and_then(|file| file.quantization.as_deref()),
            Some("Q4_K_M")
        );
    }

    #[test]
    fn groups_shards_and_rates_their_total_size() {
        let files = group_model_files(
            vec![
                ("Q4/model-Q4_K_M-00003-of-00003.gguf".into(), 30, None),
                ("Q4/model-Q4_K_M-00001-of-00003.gguf".into(), 10, None),
                ("Q4/model-Q4_K_M-00002-of-00003.gguf".into(), 20, None),
            ],
            16 * 1024u64.pow(3),
            0,
        );
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].parts.len(), 3);
        assert_eq!(files[0].parts[0].name, "model-Q4_K_M-00001-of-00003.gguf");
        assert_eq!(files[0].size_bytes, 60);
        assert_eq!(files[0].quantization.as_deref(), Some("Q4_K_M"));
        assert!(files[0].recommended);
    }

    #[test]
    fn hides_incomplete_shard_sets() {
        let files = group_model_files(
            vec![
                ("model-Q8_0-00001-of-00003.gguf".into(), 10, None),
                ("model-Q8_0-00003-of-00003.gguf".into(), 30, None),
            ],
            16 * 1024u64.pow(3),
            0,
        );
        assert!(files.is_empty());
    }

    #[test]
    fn hides_dspark_speculative_decoding_modules() {
        let files = group_model_files(
            vec![
                ("dspark-DeepSeek-V4-Flash-Q8_0.gguf".into(), 10, None),
                (
                    "DeepSeek-V4-Flash-UD-IQ1_S-00001-of-00001.gguf".into(),
                    80,
                    None,
                ),
            ],
            16 * 1024u64.pow(3),
            0,
        );
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].quantization.as_deref(), Some("IQ1_S"));
    }
}

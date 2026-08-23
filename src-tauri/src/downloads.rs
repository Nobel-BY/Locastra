use crate::{
    db::Database,
    models::{
        DownloadRequest, DownloadTask, InstalledModel, ModelFile, ModelFilePart, ModelRuntimeConfig,
    },
    sources::ModelSources,
};
use futures_util::StreamExt;
use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};
use tokio::{
    fs::{self, OpenOptions},
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufWriter},
};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
struct SavedRequest {
    request: DownloadRequest,
    mirror: String,
    bypass_proxy: bool,
}

pub struct DownloadManager {
    tasks: Mutex<HashMap<String, DownloadTask>>,
    requests: Mutex<HashMap<String, SavedRequest>>,
    controls: Mutex<HashMap<String, CancellationToken>>,
}

impl DownloadManager {
    pub fn new(previous: Vec<DownloadTask>) -> Self {
        let tasks = previous
            .into_iter()
            .map(|mut t| {
                if matches!(t.status.as_str(), "downloading" | "verifying" | "queued") {
                    t.status = "paused".into();
                }
                (t.id.clone(), t)
            })
            .collect();
        Self {
            tasks: Mutex::new(tasks),
            requests: Mutex::new(HashMap::new()),
            controls: Mutex::new(HashMap::new()),
        }
    }
    pub fn list(&self) -> Vec<DownloadTask> {
        let mut v: Vec<_> = self.tasks.lock().values().cloned().collect();
        v.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        v
    }
    pub fn get(&self, id: &str) -> Option<DownloadTask> {
        self.tasks.lock().get(id).cloned()
    }
    pub fn pause(&self, id: &str) -> Result<(), String> {
        if let Some(token) = self.controls.lock().remove(id) {
            token.cancel();
        }
        let mut tasks = self.tasks.lock();
        let task = tasks.get_mut(id).ok_or("下载任务不存在")?;
        task.status = "paused".into();
        task.bytes_per_second = 0;
        Ok(())
    }
    pub fn cancel(&self, id: &str) -> Result<Vec<PathBuf>, String> {
        if let Some(token) = self.controls.lock().remove(id) {
            token.cancel();
        }
        let mut tasks = self.tasks.lock();
        let task = tasks.get_mut(id).ok_or("下载任务不存在")?;
        task.status = "cancelled".into();
        task.bytes_per_second = 0;
        let destination = PathBuf::from(&task.destination);
        let parent = destination.parent().unwrap_or_else(|| Path::new(""));
        let paths = if task.parts.is_empty() {
            vec![part_path(&destination)]
        } else {
            task.parts
                .iter()
                .map(|part| part_path(&parent.join(&part.name)))
                .collect()
        };
        Ok(paths)
    }
    pub fn saved(&self, id: &str) -> Option<(DownloadRequest, String, bool)> {
        self.requests
            .lock()
            .get(id)
            .map(|r| (r.request.clone(), r.mirror.clone(), r.bypass_proxy))
    }
    pub fn insert(
        &self,
        task: DownloadTask,
        request: DownloadRequest,
        mirror: String,
        bypass_proxy: bool,
    ) {
        self.requests.lock().insert(
            task.id.clone(),
            SavedRequest {
                request,
                mirror,
                bypass_proxy,
            },
        );
        self.tasks.lock().insert(task.id.clone(), task);
    }
    fn set_token(&self, id: &str, token: CancellationToken) {
        self.controls.lock().insert(id.into(), token);
    }
    fn update<F: FnOnce(&mut DownloadTask)>(&self, id: &str, f: F) -> Option<DownloadTask> {
        let mut tasks = self.tasks.lock();
        let task = tasks.get_mut(id)?;
        f(task);
        Some(task.clone())
    }
}

pub fn create_task(request: &DownloadRequest, model_dir: &Path) -> DownloadTask {
    let safe_repo = request.repo_id.replace(['/', '\\'], "--");
    let dir = request
        .destination_directory
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| model_dir.join(safe_repo));
    let parts = normalized_parts(&request.file);
    let destination = dir.join(&parts[0].name);
    let file_name = if parts.len() > 1 {
        format!(
            "{}（{} 个分片）",
            request
                .file
                .quantization
                .as_deref()
                .or(request.file.shard_group.as_deref())
                .unwrap_or(&request.file.name),
            parts.len()
        )
    } else {
        request.file.name.clone()
    };
    DownloadTask {
        id: uuid::Uuid::new_v4().to_string(),
        repo_id: request.repo_id.clone(),
        file_name,
        destination: destination.to_string_lossy().into(),
        source: request.source.clone(),
        status: "queued".into(),
        downloaded_bytes: 0,
        total_bytes: request.file.size_bytes,
        bytes_per_second: 0,
        error: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        parts,
    }
}

fn normalized_parts(file: &ModelFile) -> Vec<ModelFilePart> {
    if file.parts.is_empty() {
        vec![ModelFilePart {
            path: file.path.clone(),
            name: file.name.clone(),
            size_bytes: file.size_bytes,
            sha256: file.sha256.clone(),
        }]
    } else {
        file.parts.clone()
    }
}

pub fn spawn_download(
    app: AppHandle,
    manager: Arc<DownloadManager>,
    db: Arc<Database>,
    sources: Arc<ModelSources>,
    task_id: String,
    request: DownloadRequest,
    mirror: String,
    bypass_proxy: bool,
) {
    let token = CancellationToken::new();
    manager.set_token(&task_id, token.clone());
    tauri::async_runtime::spawn(async move {
        let result = run_download(
            &app,
            &manager,
            &db,
            &sources,
            &task_id,
            &request,
            &mirror,
            bypass_proxy,
            &token,
        )
        .await;
        if let Err(error) = result {
            if !token.is_cancelled() {
                if let Some(task) = manager.update(&task_id, |t| {
                    t.status = "failed".into();
                    t.error = Some(error);
                    t.bytes_per_second = 0;
                }) {
                    let _ = db.save_download(&task);
                    let _ = app.emit("download-progress", task);
                }
            }
        }
    });
}

async fn run_download(
    app: &AppHandle,
    manager: &DownloadManager,
    db: &Database,
    sources: &ModelSources,
    id: &str,
    request: &DownloadRequest,
    mirror: &str,
    bypass_proxy: bool,
    token: &CancellationToken,
) -> Result<(), String> {
    let destination = PathBuf::from(&manager.get(id).ok_or("任务不存在")?.destination);
    let parent = destination.parent().ok_or("模型保存目录无效")?;
    fs::create_dir_all(parent)
        .await
        .map_err(|e| format!("无法创建模型目录：{e}"))?;
    let parts = normalized_parts(&request.file);
    for part in &parts {
        let file_name = Path::new(&part.name);
        if file_name.file_name().and_then(|v| v.to_str()) != Some(part.name.as_str())
            || matches!(part.name.as_str(), "." | "..")
        {
            return Err(format!("不安全的模型分片路径：{}", part.name));
        }
    }
    let expected_total = if request.file.size_bytes > 0 {
        request.file.size_bytes
    } else if parts.iter().all(|part| part.size_bytes > 0) {
        parts.iter().map(|part| part.size_bytes).sum()
    } else {
        0
    };
    let existing: u64 = parts
        .iter()
        .map(|part| {
            part_path(&parent.join(&part.name))
                .metadata()
                .map(|metadata| metadata.len().min(part.size_bytes))
                .unwrap_or(0)
        })
        .sum();
    let free = fs2::available_space(parent).unwrap_or(u64::MAX);
    if expected_total > existing && free < expected_total - existing + 256 * 1024 * 1024 {
        return Err(format!(
            "磁盘空间不足，还需要约 {:.1} GB",
            (expected_total - existing) as f64 / 1024f64.powi(3)
        ));
    }
    emit_update(app, manager, db, id, |t| {
        t.status = "downloading".into();
        t.error = None;
        t.total_bytes = expected_total;
    })?;
    let mut completed_before = 0;
    for model_part in &parts {
        if token.is_cancelled() {
            return Ok(());
        }
        let part_destination = parent.join(&model_part.name);
        let urls = ModelSources::download_urls(
            &request.repo_id,
            &model_part.path,
            &request.source,
            mirror,
        );
        let mut errors = Vec::new();
        let mut downloaded = None;
        for (source, url) in urls {
            if token.is_cancelled() {
                return Ok(());
            }
            match download_with_retries(
                app,
                manager,
                db,
                sources,
                id,
                &part_destination,
                &url,
                &source,
                completed_before,
                expected_total,
                model_part.size_bytes,
                bypass_proxy,
                token,
            )
            .await
            {
                Ok(size) => {
                    downloaded = Some(size);
                    break;
                }
                Err(error) => errors.push(format!("{source}: {error}")),
            }
        }
        let size = downloaded.ok_or_else(|| {
            format!(
                "分片 {} 的所有下载源均失败：{}",
                model_part.name,
                errors.join("；")
            )
        })?;
        completed_before += size;
    }
    if token.is_cancelled() {
        return Ok(());
    }
    emit_update(app, manager, db, id, |t| {
        t.status = "verifying".into();
        t.bytes_per_second = 0;
    })?;
    let mut verified_total = 0;
    for model_part in &parts {
        let final_path = parent.join(&model_part.name);
        let temporary_path = part_path(&final_path);
        let size = fs::metadata(&temporary_path)
            .await
            .map_err(|e| format!("无法读取分片 {}：{e}", model_part.name))?
            .len();
        if model_part.size_bytes > 0 && size != model_part.size_bytes {
            return Err(format!(
                "分片 {} 大小校验失败：预期 {}，实际 {}",
                model_part.name, model_part.size_bytes, size
            ));
        }
        if let Some(expected) = model_part
            .sha256
            .as_deref()
            .filter(|value| value.len() == 64)
        {
            let actual = sha256_file(&temporary_path).await?;
            if !actual.eq_ignore_ascii_case(expected) {
                return Err(format!("分片 {} 的 SHA-256 校验失败", model_part.name));
            }
        }
        validate_gguf(&temporary_path).await?;
        verified_total += size;
    }
    for model_part in &parts {
        let final_path = parent.join(&model_part.name);
        fs::rename(part_path(&final_path), &final_path)
            .await
            .map_err(|e| format!("无法保存分片 {}：{e}", model_part.name))?;
    }
    let first_path = parent.join(&parts[0].name);
    let model = InstalledModel {
        id: uuid::Uuid::new_v4().to_string(),
        display_name: request
            .file
            .shard_group
            .as_deref()
            .unwrap_or(&request.file.name)
            .trim_end_matches(".gguf")
            .into(),
        repo_id: Some(request.repo_id.clone()),
        file_path: first_path.to_string_lossy().into(),
        file_size: verified_total,
        quantization: request.file.quantization.clone(),
        source: request.source.clone(),
        installed_at: chrono::Utc::now().to_rfc3339(),
        valid: true,
        favorite: false,
        note: String::new(),
        last_used_at: None,
        use_count: 0,
        config: ModelRuntimeConfig::default(),
    };
    db.add_model(&model)?;
    emit_update(app, manager, db, id, |t| {
        t.status = "completed".into();
        t.downloaded_bytes = verified_total;
        t.total_bytes = verified_total;
        t.bytes_per_second = 0;
    })?;
    Ok(())
}

const SOURCE_RETRY_LIMIT: u32 = 12;

async fn download_with_retries(
    app: &AppHandle,
    manager: &DownloadManager,
    db: &Database,
    sources: &ModelSources,
    id: &str,
    destination: &Path,
    url: &str,
    source: &str,
    completed_before: u64,
    overall_total: u64,
    expected_size: u64,
    bypass_proxy: bool,
    token: &CancellationToken,
) -> Result<u64, String> {
    let mut last_error = String::new();
    let mut consecutive_no_progress = 0;
    for attempt in 1..=SOURCE_RETRY_LIMIT {
        let before = fs::metadata(part_path(destination))
            .await
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        match download_one(
            app,
            manager,
            db,
            sources,
            id,
            destination,
            url,
            source,
            completed_before,
            overall_total,
            expected_size,
            bypass_proxy,
            token,
        )
        .await
        {
            Ok(size) => {
                emit_update(app, manager, db, id, |task| task.error = None)?;
                return Ok(size);
            }
            Err(error) => {
                last_error = error;
                let after = fs::metadata(part_path(destination))
                    .await
                    .map(|metadata| metadata.len())
                    .unwrap_or(0);
                if after > before {
                    consecutive_no_progress = 0;
                } else {
                    consecutive_no_progress += 1;
                }
                if token.is_cancelled()
                    || attempt == SOURCE_RETRY_LIMIT
                    || consecutive_no_progress >= 3
                    || !retryable_download_error(&last_error)
                {
                    break;
                }
                let wait_seconds = u64::from((attempt * 2).min(20));
                emit_update(app, manager, db, id, |task| {
                    task.bytes_per_second = 0;
                    task.error = Some(format!(
                        "{source} 连接中断，已保留进度；{wait_seconds} 秒后自动断点重试（{attempt}/{SOURCE_RETRY_LIMIT}）"
                    ));
                })?;
                tokio::select! {
                    _ = token.cancelled() => return Err("下载已暂停或取消".into()),
                    _ = tokio::time::sleep(Duration::from_secs(wait_seconds)) => {}
                }
            }
        }
    }
    Err(last_error)
}

fn retryable_download_error(error: &str) -> bool {
    ![
        "HTTP 400", "HTTP 401", "HTTP 403", "HTTP 404", "HTTP 405", "HTTP 416",
    ]
    .iter()
    .any(|status| error.starts_with(status))
}

async fn download_one(
    app: &AppHandle,
    manager: &DownloadManager,
    db: &Database,
    sources: &ModelSources,
    id: &str,
    destination: &Path,
    url: &str,
    source: &str,
    completed_before: u64,
    overall_total: u64,
    expected_size: u64,
    bypass_proxy: bool,
    token: &CancellationToken,
) -> Result<u64, String> {
    let part = part_path(destination);
    let mut offset = fs::metadata(&part).await.map(|m| m.len()).unwrap_or(0);
    if expected_size > 0 && offset == expected_size {
        emit_update(app, manager, db, id, |t| {
            t.downloaded_bytes = completed_before + offset;
            t.total_bytes = overall_total;
        })?;
        return Ok(offset);
    }
    if expected_size > 0 && offset > expected_size {
        offset = 0;
    }
    let mut request = sources
        .download_client(bypass_proxy)
        .get(url)
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .header(reqwest::header::CACHE_CONTROL, "no-transform");
    if offset > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={offset}-"));
    }
    let response = request.send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() && response.status() != reqwest::StatusCode::PARTIAL_CONTENT
    {
        return Err(format!("HTTP {}", response.status()));
    }
    let partial = response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    if offset > 0 && !partial {
        offset = 0;
    }
    let content_length = response.content_length().unwrap_or(0);
    let part_total = if partial {
        offset + content_length
    } else {
        content_length
    };
    let mut options = OpenOptions::new();
    options.create(true).write(true);
    if offset == 0 {
        options.truncate(true);
    } else {
        options.append(true);
    }
    let mut file = options.open(&part).await.map_err(|e| e.to_string())?;
    if offset > 0 {
        file.seek(std::io::SeekFrom::End(0))
            .await
            .map_err(|e| e.to_string())?;
    }
    let mut file = BufWriter::with_capacity(4 * 1024 * 1024, file);
    emit_update(app, manager, db, id, |t| {
        t.source = source.into();
        t.downloaded_bytes = completed_before + offset;
        if overall_total > 0 {
            t.total_bytes = overall_total;
        } else if part_total > 0 {
            t.total_bytes = completed_before + part_total;
        }
    })?;
    let mut stream = response.bytes_stream();
    let started = Instant::now();
    let mut last_emit = Instant::now();
    let start_offset = offset;
    while let Some(chunk) =
        tokio::select! {_=token.cancelled()=>return Ok(offset),value=stream.next()=>value}
    {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        offset += chunk.len() as u64;
        if last_emit.elapsed() >= Duration::from_millis(750) {
            file.flush().await.map_err(|e| e.to_string())?;
            let elapsed = started.elapsed().as_secs_f64().max(0.1);
            let speed = ((offset - start_offset) as f64 / elapsed) as u64;
            emit_update(app, manager, db, id, |t| {
                t.downloaded_bytes = completed_before + offset;
                t.bytes_per_second = speed;
            })?;
            last_emit = Instant::now();
        }
    }
    file.flush().await.map_err(|e| e.to_string())?;
    if expected_size > 0 && offset != expected_size {
        return Err(format!(
            "连接提前结束：当前 {} 字节，应为 {} 字节",
            offset, expected_size
        ));
    }
    Ok(offset)
}

fn emit_update<F: FnOnce(&mut DownloadTask)>(
    app: &AppHandle,
    manager: &DownloadManager,
    db: &Database,
    id: &str,
    f: F,
) -> Result<(), String> {
    let task = manager.update(id, f).ok_or("任务不存在")?;
    db.save_download(&task)?;
    app.emit("download-progress", task)
        .map_err(|e| e.to_string())
}
fn part_path(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.part", path.to_string_lossy()))
}
async fn validate_gguf(path: &Path) -> Result<(), String> {
    let mut file = fs::File::open(path).await.map_err(|e| e.to_string())?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .await
        .map_err(|_| "文件过短，不是有效的 GGUF 模型".to_string())?;
    if &magic != b"GGUF" {
        return Err("文件头不是 GGUF，已拒绝导入".into());
    }
    Ok(())
}
async fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).await.map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0u8; 4 * 1024 * 1024];
    loop {
        let n = file.read(&mut buffer).await.map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn destination_is_scoped() {
        let req = DownloadRequest {
            repo_id: "a/b".into(),
            source: "modelscope".into(),
            destination_directory: None,
            file: crate::models::ModelFile {
                path: "x.gguf".into(),
                name: "x.gguf".into(),
                size_bytes: 1,
                sha256: None,
                quantization: None,
                is_gguf: true,
                is_mmproj: false,
                shard_group: None,
                parts: vec![],
                recommended: true,
                compatibility: crate::models::CompatibilityRating::Smooth,
                compatibility_reason: "ok".into(),
            },
        };
        let t = create_task(&req, Path::new("D:/models"));
        assert!(t.destination.contains("a--b"));
        assert_eq!(t.parts.len(), 1);
    }

    #[test]
    fn retries_transient_errors_but_not_missing_files() {
        assert!(retryable_download_error("error decoding response body"));
        assert!(retryable_download_error("HTTP 429 Too Many Requests"));
        assert!(retryable_download_error("HTTP 500 Internal Server Error"));
        assert!(!retryable_download_error("HTTP 404 Not Found"));
        assert!(!retryable_download_error("HTTP 403 Forbidden"));
    }
}

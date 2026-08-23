mod models;
mod rag;
mod sources;
mod web_search;

#[cfg(any(feature = "desktop", test))]
mod db;
#[cfg(feature = "desktop")]
mod downloads;
#[cfg(feature = "desktop")]
mod hardware;
#[cfg(feature = "desktop")]
mod mcp;
#[cfg(feature = "desktop")]
mod runtime;

#[cfg(feature = "desktop")]
mod desktop {

    use crate::{
        db::Database,
        downloads::{create_task, spawn_download, DownloadManager},
        hardware,
        models::*,
        runtime::{find_server_in, list_runtime_components, RuntimeManager},
        sources::ModelSources,
    };
    use parking_lot::Mutex;
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::Arc,
    };
    use tauri::{
        menu::{Menu, MenuItem},
        tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
        Emitter, Manager, State,
    };

    pub struct AppState {
        db: Arc<Database>,
        sources: Arc<ModelSources>,
        downloads: Arc<DownloadManager>,
        runtime: Arc<RuntimeManager>,
        speech: Mutex<Option<std::process::Child>>,
        default_model_dir: PathBuf,
    }

    fn normalized_model_path(path: &str) -> String {
        let replaced = path.replace('/', "\\");
        let without_prefix = if let Some(rest) = replaced.strip_prefix(r"\\?\UNC\") {
            format!(r"\\{rest}")
        } else {
            replaced
                .strip_prefix(r"\\?\")
                .unwrap_or(&replaced)
                .to_string()
        };
        without_prefix.to_lowercase()
    }

    fn display_model_path(path: &Path) -> String {
        let value = path.to_string_lossy();
        if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
            format!(r"\\{rest}")
        } else {
            value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
        }
    }

    #[tauri::command]
    fn get_hardware_report(state: State<'_, Arc<AppState>>) -> Result<HardwareReport, String> {
        let settings = state
            .db
            .settings(state.default_model_dir.to_string_lossy().into())?;
        let dir = PathBuf::from(settings.model_directory);
        let _ = std::fs::create_dir_all(&dir);
        Ok(hardware::detect(&dir))
    }

    #[tauri::command]
    fn get_settings(state: State<'_, Arc<AppState>>) -> Result<AppSettings, String> {
        state
            .db
            .settings(state.default_model_dir.to_string_lossy().into())
    }
    #[tauri::command]
    fn save_settings(
        settings: AppSettings,
        state: State<'_, Arc<AppState>>,
    ) -> Result<AppSettings, String> {
        if !settings.custom_mirror.is_empty() && !settings.custom_mirror.starts_with("https://") {
            return Err("自定义镜像必须使用 HTTPS".into());
        }
        let dir = PathBuf::from(&settings.model_directory);
        std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建模型目录：{e}"))?;
        state.db.save_settings(&settings)?;
        Ok(settings)
    }

    #[tauri::command]
    async fn search_models(
        query: String,
        source: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<ModelSearchResult>, String> {
        let term = if query.trim().is_empty() {
            "Qwen GGUF"
        } else {
            query.trim()
        };
        state.sources.search(term, &source).await
    }
    #[tauri::command]
    async fn get_model_files(
        repo_id: String,
        source: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<ModelFile>, String> {
        let report = get_hardware_report(state.clone())?;
        state
            .sources
            .files(
                &repo_id,
                &source,
                report.available_memory_bytes,
                report
                    .gpus
                    .iter()
                    .map(|gpu| gpu.dedicated_memory_bytes)
                    .max()
                    .unwrap_or(0),
            )
            .await
    }

    #[tauri::command]
    fn start_download(
        request: DownloadRequest,
        app: tauri::AppHandle,
        state: State<'_, Arc<AppState>>,
    ) -> Result<DownloadTask, String> {
        if !request.file.is_gguf || request.file.is_mmproj {
            return Err("只允许下载文本 GGUF 模型".into());
        }
        let settings = state
            .db
            .settings(state.default_model_dir.to_string_lossy().into())?;
        let model_dir = PathBuf::from(&settings.model_directory);
        let task = create_task(&request, &model_dir);
        state.downloads.insert(
            task.clone(),
            request.clone(),
            settings.custom_mirror.clone(),
            settings.download_bypass_proxy,
        );
        state.db.save_download(&task)?;
        let _ = app.emit("download-progress", task.clone());
        spawn_download(
            app,
            state.downloads.clone(),
            state.db.clone(),
            state.sources.clone(),
            task.id.clone(),
            request,
            settings.custom_mirror,
            settings.download_bypass_proxy,
        );
        Ok(task)
    }
    #[tauri::command]
    fn list_downloads(state: State<'_, Arc<AppState>>) -> Vec<DownloadTask> {
        state.downloads.list()
    }
    #[tauri::command]
    fn pause_download(
        id: String,
        app: tauri::AppHandle,
        state: State<'_, Arc<AppState>>,
    ) -> Result<(), String> {
        state.downloads.pause(&id)?;
        if let Some(t) = state.downloads.get(&id) {
            state.db.save_download(&t)?;
            let _ = app.emit("download-progress", t);
        }
        Ok(())
    }
    #[tauri::command]
    fn cancel_download(
        id: String,
        app: tauri::AppHandle,
        state: State<'_, Arc<AppState>>,
    ) -> Result<(), String> {
        let parts = state.downloads.cancel(&id)?;
        for part in parts {
            if part.is_file() {
                std::fs::remove_file(&part).map_err(|e| format!("无法删除临时文件：{e}"))?;
            }
        }
        if let Some(t) = state.downloads.get(&id) {
            state.db.save_download(&t)?;
            let _ = app.emit("download-progress", t);
        }
        Ok(())
    }
    #[tauri::command]
    fn resume_download(
        id: String,
        app: tauri::AppHandle,
        state: State<'_, Arc<AppState>>,
    ) -> Result<(), String> {
        let (request, _, _) = if let Some(saved) = state.downloads.saved(&id) {
            saved
        } else {
            let t = state.downloads.get(&id).ok_or("任务不存在")?;
            let destination = PathBuf::from(&t.destination);
            let parts = if t.parts.is_empty() {
                vec![crate::models::ModelFilePart {
                    path: destination
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(&t.file_name)
                        .to_string(),
                    name: destination
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(&t.file_name)
                        .to_string(),
                    size_bytes: t.total_bytes,
                    sha256: None,
                }]
            } else {
                t.parts.clone()
            };
            let first = parts.first().ok_or("下载任务缺少模型文件")?;
            let file = ModelFile {
                path: first.path.clone(),
                name: first.name.clone(),
                size_bytes: t.total_bytes,
                sha256: None,
                quantization: quantization_from_name(&first.name),
                is_gguf: true,
                is_mmproj: false,
                shard_group: shard_group(&first.name),
                parts,
                recommended: false,
                compatibility: CompatibilityRating::Unknown,
                compatibility_reason: "恢复任务".into(),
            };
            let req = DownloadRequest {
                repo_id: t.repo_id.clone(),
                file,
                source: t.source.clone(),
                destination_directory: destination.parent().map(|p| p.to_string_lossy().into()),
            };
            let settings = state
                .db
                .settings(state.default_model_dir.to_string_lossy().into())?;
            state.downloads.insert(
                t,
                req.clone(),
                settings.custom_mirror.clone(),
                settings.download_bypass_proxy,
            );
            (req, settings.custom_mirror, settings.download_bypass_proxy)
        };
        let current_settings = state
            .db
            .settings(state.default_model_dir.to_string_lossy().into())?;
        if let Some(task) = state.downloads.get(&id) {
            state.downloads.insert(
                task,
                request.clone(),
                current_settings.custom_mirror.clone(),
                current_settings.download_bypass_proxy,
            );
        }
        spawn_download(
            app,
            state.downloads.clone(),
            state.db.clone(),
            state.sources.clone(),
            id,
            request,
            current_settings.custom_mirror,
            current_settings.download_bypass_proxy,
        );
        Ok(())
    }

    #[tauri::command]
    fn import_model(
        path: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<InstalledModel, String> {
        let canonical =
            std::fs::canonicalize(&path).map_err(|e| format!("无法读取模型文件：{e}"))?;
        if canonical
            .extension()
            .and_then(|e| e.to_str())
            .is_none_or(|e| !e.eq_ignore_ascii_case("gguf"))
        {
            return Err("只支持 .gguf 模型文件".into());
        }
        validate_local_gguf(&canonical)?;
        let meta = std::fs::metadata(&canonical).map_err(|e| e.to_string())?;
        let file_name = canonical
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("文件名无效")?;
        let model = InstalledModel {
            id: uuid::Uuid::new_v4().to_string(),
            display_name: file_name.trim_end_matches(".gguf").into(),
            repo_id: None,
            file_path: display_model_path(&canonical),
            file_size: meta.len(),
            quantization: quantization_from_name(file_name),
            source: "local".into(),
            installed_at: chrono::Utc::now().to_rfc3339(),
            valid: true,
            favorite: false,
            note: String::new(),
            last_used_at: None,
            use_count: 0,
            config: ModelRuntimeConfig::default(),
        };
        state.db.add_model(&model)?;
        Ok(model)
    }
    #[tauri::command]
    fn list_installed_models(
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<InstalledModel>, String> {
        let models = state.db.models()?;
        let active_id = state.runtime.state().model_id;
        let mut unique: HashMap<String, InstalledModel> = HashMap::new();
        for model in models {
            let key = normalized_model_path(&model.file_path);
            if let Some(existing) = unique.get(&key).cloned() {
                let model_score = (
                    active_id.as_deref() == Some(model.id.as_str()),
                    model.repo_id.is_some(),
                    model.file_size,
                );
                let existing_score = (
                    active_id.as_deref() == Some(existing.id.as_str()),
                    existing.repo_id.is_some(),
                    existing.file_size,
                );
                let (mut keeper, discarded) = if model_score > existing_score {
                    (model, existing)
                } else {
                    (existing, model)
                };
                keeper.favorite |= discarded.favorite;
                if keeper.note.trim().is_empty() {
                    keeper.note = discarded.note;
                }
                state.db.update_model(&keeper)?;
                state.db.delete_model(&discarded.id)?;
                unique.insert(key, keeper);
            } else {
                unique.insert(key, model);
            }
        }
        let mut result = unique.into_values().collect::<Vec<_>>();
        result.sort_by(|a, b| b.installed_at.cmp(&a.installed_at));
        Ok(result)
    }
    #[tauri::command]
    fn delete_model(
        id: String,
        delete_file: bool,
        state: State<'_, Arc<AppState>>,
    ) -> Result<(), String> {
        let model = state.db.model(&id)?.ok_or("模型不存在")?;
        if state.runtime.state().model_id.as_deref() == Some(&id) {
            state.runtime.unload();
        }
        if delete_file {
            let primary = Path::new(&model.file_path);
            let primary_name = primary.file_name().and_then(|value| value.to_str());
            let mut files = Vec::new();
            if let (Some(parent), Some(primary_name)) = (primary.parent(), primary_name) {
                if shard_sequence(primary_name).is_some() {
                    let entries = std::fs::read_dir(parent)
                        .map_err(|e| format!("无法读取模型分片目录：{e}"))?;
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let is_regular_file = entry
                            .file_type()
                            .map(|kind| kind.is_file())
                            .unwrap_or(false);
                        let candidate = entry.file_name();
                        if is_regular_file
                            && candidate
                                .to_str()
                                .is_some_and(|name| belongs_to_same_shard_set(primary_name, name))
                        {
                            files.push(path);
                        }
                    }
                    if files.is_empty() && primary.is_file() {
                        files.push(primary.to_path_buf());
                    }
                } else if primary.is_file() {
                    files.push(primary.to_path_buf());
                }
            } else if primary.is_file() {
                files.push(primary.to_path_buf());
            }
            files.sort();
            for path in files {
                std::fs::remove_file(&path)
                    .map_err(|e| format!("模型文件删除失败（{}）：{e}", path.display()))?;
            }
        }
        state.db.delete_model(&id)
    }

    #[tauri::command]
    async fn load_model(
        id: String,
        context_size: u32,
        app: tauri::AppHandle,
        state: State<'_, Arc<AppState>>,
    ) -> Result<RuntimeState, String> {
        let model = state.db.model(&id)?.ok_or("模型不存在")?;
        let draft_model = if let Some(draft_id) = model.config.draft_model_id.as_deref() {
            let draft = state
                .db
                .model(draft_id)?
                .ok_or("草稿模型不存在，请在模型设置中重新选择")?;
            if !draft.valid {
                return Err("草稿模型文件不存在".into());
            }
            Some(draft)
        } else {
            None
        };
        let effective_context = if model.config.context_size > 0 {
            model.config.context_size
        } else {
            context_size
        };
        state.runtime.unload();
        let hardware = hardware::detect(&state.default_model_dir);
        let dedicated_gpu_memory = hardware
            .gpus
            .iter()
            .map(|gpu| gpu.dedicated_memory_bytes)
            .max()
            .unwrap_or(0);
        if !model.config.oversized_mode {
            if let Some(error) = model_capacity_error(
                model.file_size,
                hardware.available_memory_bytes,
                dedicated_gpu_memory,
                effective_context,
            ) {
                return Err(format!(
                    "{error} 如果你了解速度和稳定性风险，可在“模型设置”中启用“超大模型实验模式”后继续尝试。"
                ));
            }
        }
        let settings = state
            .db
            .settings(state.default_model_dir.to_string_lossy().into())?;
        let first_result = state
            .runtime
            .load(
                &app,
                &model,
                draft_model.as_ref(),
                if model.config.oversized_mode {
                    effective_context.clamp(512, 2048)
                } else {
                    effective_context.clamp(512, 131072)
                },
                settings.auto_unload_minutes,
                &settings.runtime_directory,
                settings
                    .developer_service
                    .enabled
                    .then_some(settings.developer_service.port),
                if settings.developer_service.enabled {
                    &settings.developer_service.api_token
                } else {
                    ""
                },
            )
            .await;
        let result = match first_result {
            Ok(result) => result,
            Err(first_error)
                if model.config.gpu_layers != 0
                    && model.file_size
                        <= hardware
                            .total_memory_bytes
                            .saturating_mul(3)
                            .saturating_div(4) =>
            {
                let _ = app.emit(
                    "runtime-progress",
                    serde_json::json!({
                        "modelId": model.id,
                        "stage": "fallback",
                        "percent": 10,
                        "message": "GPU 加载失败，正在自动回退到 CPU 安全配置"
                    }),
                );
                let mut cpu_model = model.clone();
                cpu_model.config.gpu_layers = 0;
                cpu_model.config.flash_attention = false;
                cpu_model.config.draft_model_id = None;
                match state
                    .runtime
                    .load(
                        &app,
                        &cpu_model,
                        None,
                        effective_context.clamp(512, 8192),
                        settings.auto_unload_minutes,
                        &settings.runtime_directory,
                        settings
                            .developer_service
                            .enabled
                            .then_some(settings.developer_service.port),
                        if settings.developer_service.enabled {
                            &settings.developer_service.api_token
                        } else {
                            ""
                        },
                    )
                    .await
                {
                    Ok(result) => result,
                    Err(cpu_error) => {
                        return Err(format!(
                            "GPU 自动适配失败：{first_error}\n\nCPU 回退也失败：{cpu_error}"
                        ))
                    }
                }
            }
            Err(error) => return Err(error),
        };
        state.db.mark_model_used(&id)?;
        Ok(result)
    }

    #[tauri::command]
    fn update_installed_model(
        model: InstalledModel,
        state: State<'_, Arc<AppState>>,
    ) -> Result<InstalledModel, String> {
        let existing = state.db.model(&model.id)?.ok_or("模型不存在")?;
        if model.file_path != existing.file_path {
            validate_local_gguf(Path::new(&model.file_path))?;
        }
        if model.config.draft_model_id.as_deref() == Some(&model.id) {
            return Err("模型不能把自己设为草稿模型".into());
        }
        if let Some(mmproj) = model
            .config
            .mmproj_path
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            validate_local_gguf(Path::new(mmproj))
                .map_err(|error| format!("mmproj 无效：{error}"))?;
        }
        state.db.update_model(&model)?;
        state.db.model(&model.id)?.ok_or("模型不存在".into())
    }
    #[tauri::command]
    fn reveal_model(id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
        let model = state.db.model(&id)?.ok_or("模型不存在")?;
        std::process::Command::new("explorer.exe")
            .arg(format!("/select,{}", model.file_path))
            .spawn()
            .map_err(|e| format!("无法打开资源管理器：{e}"))?;
        Ok(())
    }
    #[tauri::command]
    fn relocate_model(
        id: String,
        path: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<InstalledModel, String> {
        let target = PathBuf::from(&path);
        validate_local_gguf(&target)?;
        let mut model = state.db.model(&id)?.ok_or("模型不存在")?;
        model.file_path = target.to_string_lossy().into();
        model.file_size = std::fs::metadata(&target).map_err(|e| e.to_string())?.len();
        state.db.update_model(&model)?;
        state.db.model(&id)?.ok_or("模型不存在".into())
    }
    #[tauri::command]
    fn find_duplicate_models(
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<Vec<InstalledModel>>, String> {
        use std::collections::HashMap;
        let mut groups: HashMap<(u64, String), Vec<InstalledModel>> = HashMap::new();
        for model in state.db.models()? {
            groups
                .entry((model.file_size, model.display_name.to_ascii_lowercase()))
                .or_default()
                .push(model);
        }
        Ok(groups
            .into_values()
            .filter(|items| items.len() > 1)
            .collect())
    }
    #[tauri::command]
    fn scan_model_directory(
        path: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<InstalledModel>, String> {
        use std::collections::HashSet;
        let root = std::fs::canonicalize(&path).map_err(|e| format!("无法读取目录：{e}"))?;
        if !root.is_dir() {
            return Err("请选择文件夹".into());
        }
        let known: HashSet<String> = state
            .db
            .models()?
            .into_iter()
            .map(|model| model.file_path.to_ascii_lowercase())
            .collect();
        let shard =
            regex::Regex::new(r"(?i)-(\d{5})-of-(\d{5})\.gguf$").map_err(|e| e.to_string())?;
        let mut pending = vec![root];
        let mut found = Vec::new();
        let mut inspected = 0usize;
        while let Some(directory) = pending.pop() {
            for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let file_type = entry.file_type().map_err(|e| e.to_string())?;
                if file_type.is_symlink() {
                    continue;
                }
                if file_type.is_dir() {
                    pending.push(entry.path());
                    continue;
                }
                inspected += 1;
                if inspected > 20_000 {
                    return Err("目录中文件过多，已在检查 20000 个文件后停止".into());
                }
                let candidate = entry.path();
                if candidate
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_none_or(|value| !value.eq_ignore_ascii_case("gguf"))
                {
                    continue;
                }
                let name = candidate
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default();
                if shard
                    .captures(name)
                    .and_then(|captures| captures.get(1))
                    .is_some_and(|part| part.as_str() != "00001")
                {
                    continue;
                }
                let canonical = std::fs::canonicalize(&candidate).map_err(|e| e.to_string())?;
                if known.contains(&canonical.to_string_lossy().to_ascii_lowercase())
                    || validate_local_gguf(&canonical).is_err()
                {
                    continue;
                }
                let model = installed_model_from_path(&canonical)?;
                state.db.add_model(&model)?;
                found.push(model);
            }
        }
        Ok(found)
    }
    #[tauri::command]
    fn move_model(
        id: String,
        destination_directory: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<InstalledModel, String> {
        let mut model = state.db.model(&id)?.ok_or("模型不存在")?;
        if state.runtime.state().model_id.as_deref() == Some(&id) {
            return Err("请先卸载模型再移动文件".into());
        }
        let destination = std::fs::canonicalize(&destination_directory)
            .map_err(|e| format!("无法读取目标目录：{e}"))?;
        if !destination.is_dir() {
            return Err("目标位置不是文件夹".into());
        }
        let source =
            std::fs::canonicalize(&model.file_path).map_err(|e| format!("模型文件不存在：{e}"))?;
        if source.parent() == Some(destination.as_path()) {
            return Ok(model);
        }
        let file_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or("模型文件名无效")?;
        let shard = regex::Regex::new(r"(?i)^(.*?)-(\d{5})-of-(\d{5})\.gguf$")
            .map_err(|e| e.to_string())?;
        let mut sources = vec![source.clone()];
        if let Some(captures) = shard.captures(file_name) {
            let prefix = captures.get(1).unwrap().as_str();
            let total: usize = captures
                .get(3)
                .unwrap()
                .as_str()
                .parse()
                .map_err(|_| "分片数量无效")?;
            sources = (1..=total)
                .map(|part| {
                    source
                        .parent()
                        .unwrap()
                        .join(format!("{prefix}-{part:05}-of-{total:05}.gguf"))
                })
                .collect();
        }
        for item in &sources {
            if !item.is_file() {
                return Err(format!("缺少模型分片：{}", item.display()));
            }
            let target = destination.join(item.file_name().ok_or("模型文件名无效")?);
            if target.exists() {
                return Err(format!("目标文件已存在：{}", target.display()));
            }
        }
        let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
        for item in &sources {
            let target = destination.join(item.file_name().ok_or("模型文件名无效")?);
            if let Err(error) = std::fs::rename(item, &target) {
                for (original, placed) in moved.iter().rev() {
                    let _ = std::fs::rename(placed, original);
                }
                return Err(format!("移动模型失败（仅支持同一磁盘内移动）：{error}"));
            }
            moved.push((item.clone(), target));
        }
        let primary_index = sources
            .iter()
            .position(|value| value == &source)
            .unwrap_or(0);
        model.file_path = moved[primary_index].1.to_string_lossy().into();
        state.db.update_model(&model)?;
        state.db.model(&id)?.ok_or("模型不存在".into())
    }
    #[tauri::command]
    async fn benchmark_model(
        id: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<ModelBenchmark, String> {
        if state.runtime.state().status != "stopped" {
            return Err("基准测试前请先卸载当前模型，避免显存不足".into());
        }
        let model = state.db.model(&id)?.ok_or("模型不存在")?;
        validate_local_gguf(Path::new(&model.file_path))?;
        if model.config.oversized_mode {
            return Err("超大模型实验模式不运行常规基准测试，因为 llama-bench 不支持服务器的自动显存适配。请直接加载模型，并在对话回复下方查看实际 Token/s。".into());
        }
        let hardware = hardware::detect(&state.default_model_dir);
        let dedicated_gpu_memory = hardware
            .gpus
            .iter()
            .map(|gpu| gpu.dedicated_memory_bytes)
            .max()
            .unwrap_or(0);
        if let Some(error) = model_capacity_error(
            model.file_size,
            hardware.available_memory_bytes,
            dedicated_gpu_memory,
            model.config.context_size,
        ) {
            return Err(format!("基准测试已跳过：{error}"));
        }
        let settings = state
            .db
            .settings(state.default_model_dir.to_string_lossy().into())?;
        let benchmark =
            find_server_in(Some(&settings.runtime_directory))?.with_file_name("llama-bench.exe");
        if !benchmark.is_file() {
            return Err("随应用提供的 llama-bench.exe 不存在".into());
        }
        let started = std::time::Instant::now();
        let mut command = tokio::process::Command::new(&benchmark);
        let gpu_layers = if model.config.gpu_layers >= 999 {
            "0".to_string()
        } else {
            model.config.gpu_layers.to_string()
        };
        let threads = model.config.threads.to_string();
        command
            .current_dir(benchmark.parent().ok_or("基准工具目录无效")?)
            .args([
                "-m",
                &model.file_path,
                "-p",
                "128",
                "-n",
                "32",
                "-r",
                "1",
                "-o",
                "json",
                "-ngl",
                &gpu_layers,
            ]);
        if model.config.threads > 0 {
            command.args(["-t", &threads]);
        }
        let output = tokio::time::timeout(std::time::Duration::from_secs(600), command.output())
            .await
            .map_err(|_| "基准测试超过 10 分钟，已停止等待".to_string())?
            .map_err(|e| format!("无法启动基准工具：{e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if !output.status.success() {
            return Err(format!(
                "基准测试失败：{}",
                if stderr.trim().is_empty() {
                    stdout.trim()
                } else {
                    stderr.trim()
                }
            ));
        }
        let entries: Vec<serde_json::Value> =
            serde_json::from_str(stdout.trim()).unwrap_or_default();
        let prompt_tokens_per_second = entries
            .iter()
            .find(|value| value.get("n_prompt").and_then(|v| v.as_u64()).unwrap_or(0) > 0)
            .and_then(|value| value.get("avg_ts").and_then(|v| v.as_f64()));
        let generation_tokens_per_second = entries
            .iter()
            .find(|value| value.get("n_gen").and_then(|v| v.as_u64()).unwrap_or(0) > 0)
            .and_then(|value| value.get("avg_ts").and_then(|v| v.as_f64()));
        Ok(ModelBenchmark {
            model_id: id,
            load_time_ms: started.elapsed().as_millis() as u64,
            prompt_tokens_per_second,
            generation_tokens_per_second,
            output: stdout,
        })
    }
    #[tauri::command]
    async fn install_cuda_runtime(
        cuda_version: Option<String>,
        app: tauri::AppHandle,
        state: State<'_, Arc<AppState>>,
    ) -> Result<AppSettings, String> {
        use futures_util::StreamExt;
        use sha2::Digest;
        use tokio::io::AsyncWriteExt;
        let version = cuda_version.unwrap_or_else(|| "12.4".into());
        if !matches!(version.as_str(), "12.4" | "13.3") {
            return Err("当前支持 CUDA 12.4 或 13.3 官方运行包".into());
        }
        let mut settings = state
            .db
            .settings(state.default_model_dir.to_string_lossy().into())?;
        // Runtime upgrades are deliberately pinned. A moving `latest` target can
        // change CLI flags or model support underneath an otherwise unchanged
        // Locastra release, making failures impossible to reproduce or roll back.
        const PINNED_RUNTIME_TAG: &str = "b10357";
        let mut builder = reqwest::Client::builder()
            .user_agent(format!("Locastra/{}", env!("CARGO_PKG_VERSION")))
            .connect_timeout(std::time::Duration::from_secs(20))
            .timeout(std::time::Duration::from_secs(1800));
        if settings.download_bypass_proxy {
            builder = builder.no_proxy();
        }
        let client = builder.build().map_err(|e| e.to_string())?;
        let _ = app.emit(
            "runtime-install-progress",
            serde_json::json!({"percent":2,"message":"正在查询 llama.cpp 官方 CUDA 运行包"}),
        );
        let release: serde_json::Value = client
            .get(format!(
                "https://api.github.com/repos/ggml-org/llama.cpp/releases/tags/{PINNED_RUNTIME_TAG}"
            ))
            .send()
            .await
            .map_err(|e| format!("无法访问 GitHub 发布接口：{e}"))?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        let tag = release
            .get("tag_name")
            .and_then(|v| v.as_str())
            .ok_or("发布信息缺少版本号")?;
        let assets = release
            .get("assets")
            .and_then(|v| v.as_array())
            .ok_or("发布信息缺少下载文件")?;
        let suffix = format!("cuda-{version}-x64.zip");
        let selected: Vec<&serde_json::Value> = assets
            .iter()
            .filter(|asset| {
                asset
                    .get("name")
                    .and_then(|v| v.as_str())
                    .is_some_and(|name| {
                        (name.starts_with("llama-") || name.starts_with("cudart-llama-"))
                            && name.ends_with(&suffix)
                    })
            })
            .collect();
        if selected.len() < 2 {
            return Err(format!(
                "官方发布 {tag} 中没有找到完整的 CUDA {version} Windows x64 运行包"
            ));
        }
        let base = dirs::data_local_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalDeploy")
            .join("runtimes")
            .join(format!("{tag}-cuda-{version}"));
        std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
        let total: u64 = selected
            .iter()
            .filter_map(|asset| asset.get("size").and_then(|v| v.as_u64()))
            .sum();
        let mut downloaded = 0u64;
        for asset in selected {
            let name = asset
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or("运行包名称无效")?;
            let url = asset
                .get("browser_download_url")
                .and_then(|v| v.as_str())
                .ok_or("运行包地址无效")?;
            let archive = base.join(name);
            let mut file = tokio::fs::File::create(&archive)
                .await
                .map_err(|e| e.to_string())?;
            let response = client
                .get(url)
                .send()
                .await
                .map_err(|e| format!("下载 {name} 失败：{e}"))?
                .error_for_status()
                .map_err(|e| e.to_string())?;
            let mut stream = response.bytes_stream();
            let mut hasher = sha2::Sha256::new();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| format!("下载 {name} 中断：{e}"))?;
                file.write_all(&chunk).await.map_err(|e| e.to_string())?;
                hasher.update(&chunk);
                downloaded += chunk.len() as u64;
                let percent = 5 + if total > 0 {
                    downloaded * 75 / total
                } else {
                    0
                };
                let _ = app.emit(
                    "runtime-install-progress",
                    serde_json::json!({"percent":percent,"message":format!("正在下载 {name}")}),
                );
            }
            file.flush().await.map_err(|e| e.to_string())?;
            drop(file);
            if let Some(expected) = asset
                .get("digest")
                .and_then(|v| v.as_str())
                .and_then(|v| v.strip_prefix("sha256:"))
            {
                let actual = format!("{:x}", hasher.finalize());
                if !actual.eq_ignore_ascii_case(expected) {
                    return Err(format!("{name} 完整性校验失败"));
                }
            }
            extract_runtime_archive(&archive, &base)?;
            let _ = std::fs::remove_file(&archive);
        }
        if !base.join("llama-server.exe").is_file() {
            return Err("CUDA 运行包解压后缺少 llama-server.exe".into());
        }
        settings.previous_runtime_directory = settings.runtime_directory.clone();
        settings.runtime_directory = base.to_string_lossy().into();
        state.db.save_settings(&settings)?;
        let _ = app.emit(
            "runtime-install-progress",
            serde_json::json!({"percent":100,"message":format!("CUDA {version} 运行组件已安装")}),
        );
        Ok(settings)
    }
    #[tauri::command]
    fn unload_model(state: State<'_, Arc<AppState>>) -> RuntimeState {
        state.runtime.unload()
    }
    #[tauri::command]
    fn get_runtime_state(state: State<'_, Arc<AppState>>) -> RuntimeState {
        state.runtime.state()
    }

    #[tauri::command]
    fn list_runtime_components_command(
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<RuntimeComponent>, String> {
        let settings = state
            .db
            .settings(state.default_model_dir.to_string_lossy().into())?;
        Ok(list_runtime_components(&settings.runtime_directory))
    }

    #[tauri::command]
    fn activate_runtime_component(
        path: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<AppSettings, String> {
        let directory = PathBuf::from(path.trim());
        if !directory.join("llama-server.exe").is_file()
            || !directory.join("llama.dll").is_file()
            || !directory.join("ggml.dll").is_file()
        {
            return Err("所选目录不是完整的 llama.cpp Windows 运行组件".into());
        }
        let mut settings = state
            .db
            .settings(state.default_model_dir.to_string_lossy().into())?;
        if settings.runtime_directory != directory.to_string_lossy() {
            settings.previous_runtime_directory = settings.runtime_directory.clone();
            settings.runtime_directory = directory.to_string_lossy().into_owned();
            state.runtime.unload();
            state.db.save_settings(&settings)?;
        }
        Ok(settings)
    }

    #[tauri::command]
    fn rollback_runtime_component(state: State<'_, Arc<AppState>>) -> Result<AppSettings, String> {
        let mut settings = state
            .db
            .settings(state.default_model_dir.to_string_lossy().into())?;
        if settings.previous_runtime_directory.trim().is_empty() {
            return Err("没有可回退的上一版运行组件".into());
        }
        let previous = PathBuf::from(&settings.previous_runtime_directory);
        if !previous.join("llama-server.exe").is_file() {
            return Err("上一版运行组件已被移动或删除".into());
        }
        std::mem::swap(
            &mut settings.runtime_directory,
            &mut settings.previous_runtime_directory,
        );
        state.runtime.unload();
        state.db.save_settings(&settings)?;
        Ok(settings)
    }

    #[tauri::command]
    async fn count_tokens(content: String, state: State<'_, Arc<AppState>>) -> Result<u64, String> {
        if content.len() > 8 * 1024 * 1024 {
            return Err("待分词内容过大".into());
        }
        state.runtime.count_tokens(&content).await
    }

    #[tauri::command]
    async fn summarize_context(
        transcript: String,
        previous_summary: Option<String>,
        state: State<'_, Arc<AppState>>,
    ) -> Result<String, String> {
        if transcript.len() > 4 * 1024 * 1024 {
            return Err("待压缩的对话内容过大".into());
        }
        state
            .runtime
            .summarize_context(&transcript, previous_summary.as_deref())
            .await
    }

    fn version_parts(value: &str) -> Vec<u64> {
        value
            .trim_start_matches('v')
            .split('.')
            .map(|part| {
                part.chars()
                    .take_while(|character| character.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .unwrap_or(0)
            })
            .collect()
    }

    #[tauri::command]
    async fn check_for_app_update(
        state: State<'_, Arc<AppState>>,
    ) -> Result<AppUpdateInfo, String> {
        let current = env!("CARGO_PKG_VERSION").to_string();
        let manifest_url = std::env::var("LOCASTRA_UPDATE_MANIFEST_URL")
            .ok()
            .or_else(|| option_env!("LOCASTRA_UPDATE_MANIFEST_URL").map(str::to_string));
        let Some(manifest_url) = manifest_url.filter(|url| url.starts_with("https://")) else {
            return Ok(AppUpdateInfo {
                available: false,
                current_version: current.clone(),
                latest_version: current,
                notes: "当前构建未配置正式更新源。安装正式签名版后会在这里检查更新。".into(),
                published_at: None,
                download_url: None,
                sha256: None,
            });
        };
        let settings = state
            .db
            .settings(state.default_model_dir.to_string_lossy().into())?;
        let mut builder = reqwest::Client::builder()
            .user_agent(format!("Locastra/{current}"))
            .connect_timeout(std::time::Duration::from_secs(15))
            .timeout(std::time::Duration::from_secs(30));
        if settings.download_bypass_proxy {
            builder = builder.no_proxy();
        }
        let value: serde_json::Value = builder
            .build()
            .map_err(|e| e.to_string())?
            .get(manifest_url)
            .send()
            .await
            .map_err(|e| format!("检查更新失败：{e}"))?
            .error_for_status()
            .map_err(|e| format!("更新服务器返回错误：{e}"))?
            .json()
            .await
            .map_err(|e| format!("更新清单格式无效：{e}"))?;
        let latest = value
            .get("version")
            .and_then(serde_json::Value::as_str)
            .ok_or("更新清单缺少 version")?
            .trim_start_matches('v')
            .to_string();
        let download_url = value
            .get("url")
            .and_then(serde_json::Value::as_str)
            .filter(|url| url.starts_with("https://"))
            .map(str::to_string);
        let sha256 = value
            .get("sha256")
            .and_then(serde_json::Value::as_str)
            .filter(|hash| hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()))
            .map(str::to_lowercase);
        let available = version_parts(&latest) > version_parts(&current)
            && download_url.is_some()
            && sha256.is_some();
        Ok(AppUpdateInfo {
            available,
            current_version: current,
            latest_version: latest,
            notes: value
                .get("notes")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string(),
            published_at: value
                .get("publishedAt")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            download_url,
            sha256,
        })
    }

    #[tauri::command]
    async fn install_app_update(
        update: AppUpdateInfo,
        app: tauri::AppHandle,
        state: State<'_, Arc<AppState>>,
    ) -> Result<(), String> {
        use futures_util::StreamExt;
        use sha2::Digest;
        use tokio::io::AsyncWriteExt;
        if !update.available {
            return Err("没有可安装的更新".into());
        }
        let url = update.download_url.ok_or("更新缺少下载地址")?;
        let expected = update.sha256.ok_or("更新缺少 SHA-256")?;
        if !url.starts_with("https://") {
            return Err("更新安装包必须通过 HTTPS 下载".into());
        }
        let settings = state
            .db
            .settings(state.default_model_dir.to_string_lossy().into())?;
        let mut builder = reqwest::Client::builder()
            .user_agent(format!("Locastra/{}", env!("CARGO_PKG_VERSION")))
            .connect_timeout(std::time::Duration::from_secs(20))
            .timeout(std::time::Duration::from_secs(1800));
        if settings.download_bypass_proxy {
            builder = builder.no_proxy();
        }
        let response = builder
            .build()
            .map_err(|e| e.to_string())?
            .get(url)
            .send()
            .await
            .map_err(|e| format!("下载更新失败：{e}"))?
            .error_for_status()
            .map_err(|e| format!("下载更新失败：{e}"))?;
        let directory = dirs::data_local_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalDeploy")
            .join("updates");
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let partial = directory.join(format!("Locastra-{}-setup.exe.part", update.latest_version));
        let installer = directory.join(format!("Locastra-{}-setup.exe", update.latest_version));
        let mut file = tokio::fs::File::create(&partial)
            .await
            .map_err(|e| e.to_string())?;
        let mut hasher = sha2::Sha256::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("更新下载中断：{e}"))?;
            hasher.update(&chunk);
            file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        }
        file.flush().await.map_err(|e| e.to_string())?;
        drop(file);
        let actual = format!("{:x}", hasher.finalize());
        if !actual.eq_ignore_ascii_case(&expected) {
            let _ = std::fs::remove_file(&partial);
            return Err("更新安装包 SHA-256 校验失败，现有版本未受影响".into());
        }
        std::fs::rename(&partial, &installer).map_err(|e| e.to_string())?;
        let signature = std::process::Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "(Get-AuthenticodeSignature -LiteralPath $args[0]).Status.ToString()",
                &installer.to_string_lossy(),
            ])
            .output()
            .map_err(|e| format!("无法验证更新签名：{e}"))?;
        if String::from_utf8_lossy(&signature.stdout).trim() != "Valid" {
            return Err("更新安装包没有有效的 Windows 代码签名，已拒绝安装".into());
        }
        state.runtime.unload();
        std::process::Command::new(&installer)
            .arg("/S")
            .spawn()
            .map_err(|e| format!("无法启动更新安装器：{e}"))?;
        app.exit(0);
        Ok(())
    }

    fn authenticode_identity(path: &Path) -> Result<String, String> {
        let output = std::process::Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "$s=Get-AuthenticodeSignature -LiteralPath $args[0]; if($s.Status -ne 'Valid' -or -not $s.SignerCertificate){exit 7}; $s.SignerCertificate.Thumbprint",
                &path.to_string_lossy(),
            ])
            .output()
            .map_err(|e| format!("无法验证 Windows 签名：{e}"))?;
        if !output.status.success() {
            return Err("安装包没有有效的 Windows 代码签名".into());
        }
        let thumbprint = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_ascii_uppercase();
        if thumbprint.len() < 32 || !thumbprint.chars().all(|value| value.is_ascii_hexdigit()) {
            return Err("无法读取安装包签名证书".into());
        }
        Ok(thumbprint)
    }

    #[tauri::command]
    fn rollback_app_update(
        path: String,
        app: tauri::AppHandle,
        state: State<'_, Arc<AppState>>,
    ) -> Result<(), String> {
        let installer = PathBuf::from(path);
        if !installer.is_file()
            || !installer
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("exe"))
        {
            return Err("请选择旧版 Locastra 的 .exe 安装包".into());
        }
        let current_executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let current_signer = authenticode_identity(&current_executable).map_err(|_| {
            "当前 Locastra 不是正式签名版本，无法安全确认旧安装包的发布者".to_string()
        })?;
        let candidate_signer = authenticode_identity(&installer)?;
        if current_signer != candidate_signer {
            return Err("旧安装包与当前 Locastra 的签名发布者不一致，已拒绝执行".into());
        }

        let database = state.db.checkpoint_for_backup()?;
        let backup_directory = dirs::data_local_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalDeploy")
            .join("backups");
        std::fs::create_dir_all(&backup_directory).map_err(|e| e.to_string())?;
        let backup = backup_directory.join(format!(
            "pre-rollback-{}.db",
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        ));
        std::fs::copy(database, backup).map_err(|e| format!("无法创建回退前数据备份：{e}"))?;
        state.runtime.unload();
        std::process::Command::new(&installer)
            .arg("/S")
            .spawn()
            .map_err(|e| format!("无法启动旧版安装器：{e}"))?;
        app.exit(0);
        Ok(())
    }

    #[tauri::command]
    fn list_conversations(state: State<'_, Arc<AppState>>) -> Result<Vec<Conversation>, String> {
        state.db.conversations()
    }
    #[tauri::command]
    fn search_conversations(
        query: String,
        include_archived: bool,
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<Conversation>, String> {
        state.db.search_conversations(&query, include_archived)
    }
    #[tauri::command]
    fn create_conversation(
        title: String,
        model_id: Option<String>,
        state: State<'_, Arc<AppState>>,
    ) -> Result<Conversation, String> {
        state.db.create_conversation(title, model_id)
    }
    #[tauri::command]
    fn get_conversation(
        id: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<Conversation, String> {
        state.db.conversation(&id)?.ok_or("对话不存在".into())
    }
    #[tauri::command]
    fn delete_conversation(id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
        state.db.delete_conversation(&id)
    }
    #[tauri::command]
    fn update_conversation(
        conversation: Conversation,
        state: State<'_, Arc<AppState>>,
    ) -> Result<Conversation, String> {
        state.db.update_conversation(&conversation)?;
        state
            .db
            .conversation(&conversation.id)?
            .ok_or("对话不存在".into())
    }
    #[tauri::command]
    fn duplicate_conversation(
        id: String,
        through_message_id: Option<String>,
        state: State<'_, Arc<AppState>>,
    ) -> Result<Conversation, String> {
        state
            .db
            .duplicate_conversation(&id, through_message_id.as_deref())
    }
    #[tauri::command]
    fn add_message(
        conversation_id: String,
        role: String,
        content: String,
        attachments: Option<Vec<ChatAttachment>>,
        state: State<'_, Arc<AppState>>,
    ) -> Result<ChatMessage, String> {
        if !matches!(role.as_str(), "user" | "assistant" | "system") {
            return Err("消息角色无效".into());
        }
        let attachments = attachments.unwrap_or_default();
        for attachment in &attachments {
            let path = Path::new(&attachment.file_path);
            if !path.is_file() {
                return Err(format!("附件文件不存在：{}", attachment.name));
            }
            if attachment.file_size > 20 * 1024 * 1024 {
                return Err("单个图片附件不能超过 20 MB".into());
            }
        }
        state
            .db
            .add_message_with_attachments(&conversation_id, &role, &content, &attachments)
    }
    #[tauri::command]
    fn create_image_attachment(path: String) -> Result<ChatAttachment, String> {
        let canonical = std::fs::canonicalize(&path).map_err(|e| format!("无法读取图片：{e}"))?;
        let metadata = std::fs::metadata(&canonical).map_err(|e| e.to_string())?;
        if metadata.len() > 20 * 1024 * 1024 {
            return Err("图片超过 20 MB".into());
        }
        let extension = canonical
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let mime_type = match extension.as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            _ => return Err("仅支持 PNG、JPEG、WebP、GIF 和 BMP 图片".into()),
        };
        let name = canonical
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or("图片文件名无效")?
            .to_string();
        Ok(ChatAttachment {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            file_path: canonical.to_string_lossy().into(),
            mime_type: mime_type.into(),
            file_size: metadata.len(),
        })
    }
    #[tauri::command]
    fn update_message(
        id: String,
        conversation_id: String,
        content: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<Conversation, String> {
        if content.trim().is_empty() {
            return Err("消息内容不能为空".into());
        }
        state.db.update_message(&id, content.trim())?;
        state
            .db
            .conversation(&conversation_id)?
            .ok_or("对话不存在".into())
    }
    #[tauri::command]
    fn set_message_stats(
        id: String,
        stats: GenerationStats,
        state: State<'_, Arc<AppState>>,
    ) -> Result<(), String> {
        state.db.set_message_stats(&id, &stats)
    }
    #[tauri::command]
    fn truncate_messages(
        conversation_id: String,
        message_id: String,
        include_message: bool,
        state: State<'_, Arc<AppState>>,
    ) -> Result<Conversation, String> {
        state
            .db
            .delete_message_and_after(&conversation_id, &message_id, include_message)?;
        state
            .db
            .conversation(&conversation_id)?
            .ok_or("对话不存在".into())
    }
    #[tauri::command]
    fn list_conversation_folders(
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<ConversationFolder>, String> {
        state.db.folders()
    }
    #[tauri::command]
    fn create_conversation_folder(
        name: String,
        parent_id: Option<String>,
        state: State<'_, Arc<AppState>>,
    ) -> Result<ConversationFolder, String> {
        state.db.create_folder(&name, parent_id)
    }
    #[tauri::command]
    fn delete_conversation_folder(
        id: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<(), String> {
        state.db.delete_folder(&id)
    }
    #[tauri::command]
    fn list_prompt_presets(state: State<'_, Arc<AppState>>) -> Result<Vec<PromptPreset>, String> {
        state.db.presets()
    }
    #[tauri::command]
    fn save_prompt_preset(
        preset: PromptPreset,
        state: State<'_, Arc<AppState>>,
    ) -> Result<PromptPreset, String> {
        state.db.save_preset(preset)
    }
    #[tauri::command]
    fn delete_prompt_preset(id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
        state.db.delete_preset(&id)
    }
    #[tauri::command]
    fn list_assistants(state: State<'_, Arc<AppState>>) -> Result<Vec<AssistantProfile>, String> {
        state.db.assistants()
    }
    #[tauri::command]
    fn save_assistant(
        assistant: AssistantProfile,
        state: State<'_, Arc<AppState>>,
    ) -> Result<AssistantProfile, String> {
        state.db.save_assistant(assistant)
    }
    #[tauri::command]
    fn delete_assistant(id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
        state.db.delete_assistant(&id)
    }
    #[tauri::command]
    fn list_knowledge_documents(
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<KnowledgeDocument>, String> {
        state.db.knowledge_documents()
    }
    #[tauri::command]
    fn import_knowledge_document(
        path: String,
        project_id: Option<String>,
        state: State<'_, Arc<AppState>>,
    ) -> Result<KnowledgeDocument, String> {
        let canonical =
            std::fs::canonicalize(&path).map_err(|e| format!("无法读取资料文件：{e}"))?;
        let metadata = std::fs::metadata(&canonical).map_err(|e| e.to_string())?;
        if metadata.len() > 20 * 1024 * 1024 {
            return Err("资料文件超过 20 MB；请拆分后导入".into());
        }
        let extension = canonical
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        const TEXT_TYPES: &[&str] = &[
            "txt", "md", "markdown", "json", "jsonl", "csv", "tsv", "xml", "html", "htm", "yaml",
            "yml", "toml", "rs", "py", "js", "ts", "tsx", "jsx", "java", "go", "c", "h", "cpp",
            "hpp", "cs", "sql", "log",
        ];
        let content = if TEXT_TYPES.contains(&extension.as_str()) {
            let bytes = std::fs::read(&canonical).map_err(|e| e.to_string())?;
            if bytes.iter().take(8192).any(|value| *value == 0) {
                return Err("文件看起来是二进制格式，无法作为文本资料导入".into());
            }
            String::from_utf8(bytes)
                .map_err(|_| "文件不是 UTF-8 编码；请先另存为 UTF-8".to_string())?
        } else if extension == "pdf" {
            pdf_extract::extract_text(&canonical)
                .map_err(|e| format!("PDF 解析失败：{e}；扫描版 PDF 请先进行 OCR"))?
        } else if matches!(extension.as_str(), "docx" | "pptx" | "xlsx") {
            extract_office_text(&canonical, &extension)?
        } else {
            return Err("支持 PDF、DOCX、PPTX、XLSX、文本、Markdown、表格数据和常见源代码；扫描图片请先 OCR".into());
        };
        let name = canonical
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or("文件名无效")?
            .to_string();
        let document = KnowledgeDocument {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            file_path: canonical.to_string_lossy().into(),
            file_type: extension,
            file_size: metadata.len(),
            character_count: content.chars().count() as u64,
            created_at: chrono::Utc::now().to_rfc3339(),
            project_id,
            index_status: "indexing".into(),
            index_error: None,
            chunk_count: 0,
            indexed_at: None,
        };
        state.db.add_knowledge_document(&document, &content)?;
        let chunks = crate::rag::chunk_text(&content, 1400, 160)
            .into_iter()
            .enumerate()
            .map(|(index, content)| crate::rag::IndexedChunk {
                id: uuid::Uuid::new_v4().to_string(),
                document_id: document.id.clone(),
                document_name: document.name.clone(),
                source_path: document.file_path.clone(),
                chunk_index: index,
                page: None,
                terms: crate::rag::tokenize(&content),
                embedding: crate::rag::embed(&content),
                content,
            })
            .collect::<Vec<_>>();
        state.db.replace_knowledge_chunks(&document.id, &chunks)?;
        let mut indexed = document;
        indexed.index_status = "ready".into();
        indexed.chunk_count = chunks.len() as u32;
        indexed.indexed_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(indexed)
    }
    #[tauri::command]
    fn delete_knowledge_document(
        id: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<(), String> {
        state.db.delete_knowledge_document(&id)
    }
    #[tauri::command]
    fn retrieve_knowledge(
        document_ids: Vec<String>,
        query: String,
        max_characters: usize,
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<KnowledgeSnippet>, String> {
        let mut chunks = state.db.knowledge_chunks(&document_ids)?;
        if chunks.is_empty() {
            for (document, content) in state.db.knowledge_contents(&document_ids)? {
                let indexed = crate::rag::chunk_text(&content, 1400, 160)
                    .into_iter()
                    .enumerate()
                    .map(|(index, content)| crate::rag::IndexedChunk {
                        id: uuid::Uuid::new_v4().to_string(),
                        document_id: document.id.clone(),
                        document_name: document.name.clone(),
                        source_path: document.file_path.clone(),
                        chunk_index: index,
                        page: None,
                        terms: crate::rag::tokenize(&content),
                        embedding: crate::rag::embed(&content),
                        content,
                    })
                    .collect::<Vec<_>>();
                state.db.replace_knowledge_chunks(&document.id, &indexed)?;
                chunks.extend(indexed);
            }
        }
        let mut snippets = crate::rag::search(&chunks, &query, 20);
        let mut used = 0usize;
        snippets.retain(|snippet| {
            if used >= max_characters.max(1000) {
                return false;
            }
            used += snippet.content.chars().count();
            true
        });
        Ok(snippets)
    }
    #[tauri::command]
    fn rebuild_knowledge_index(
        id: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<KnowledgeDocument, String> {
        let (mut document, content) = state
            .db
            .knowledge_contents(std::slice::from_ref(&id))?
            .into_iter()
            .next()
            .ok_or("资料不存在")?;
        let chunks = crate::rag::chunk_text(&content, 1400, 160)
            .into_iter()
            .enumerate()
            .map(|(index, content)| crate::rag::IndexedChunk {
                id: uuid::Uuid::new_v4().to_string(),
                document_id: document.id.clone(),
                document_name: document.name.clone(),
                source_path: document.file_path.clone(),
                chunk_index: index,
                page: None,
                terms: crate::rag::tokenize(&content),
                embedding: crate::rag::embed(&content),
                content,
            })
            .collect::<Vec<_>>();
        state.db.replace_knowledge_chunks(&id, &chunks)?;
        document.index_status = "ready".into();
        document.chunk_count = chunks.len() as u32;
        document.indexed_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(document)
    }
    #[tauri::command]
    fn list_projects(state: State<'_, Arc<AppState>>) -> Result<Vec<Project>, String> {
        state.db.projects()
    }
    #[tauri::command]
    fn save_project(project: Project, state: State<'_, Arc<AppState>>) -> Result<Project, String> {
        state.db.save_project(project)
    }
    #[tauri::command]
    fn delete_project(id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
        state.db.delete_project(&id)
    }
    #[tauri::command]
    fn list_project_artifacts(
        project_id: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<ProjectArtifact>, String> {
        state.db.project_artifacts(&project_id)
    }
    #[tauri::command]
    fn save_project_artifact(
        artifact: ProjectArtifact,
        state: State<'_, Arc<AppState>>,
    ) -> Result<ProjectArtifact, String> {
        state.db.save_project_artifact(artifact)
    }
    #[tauri::command]
    fn delete_project_artifact(id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
        state.db.delete_project_artifact(&id)
    }
    #[tauri::command]
    fn export_project_artifacts(
        project_id: String,
        directory: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<String, String> {
        let root = std::path::PathBuf::from(directory);
        if root.as_os_str().is_empty() {
            return Err("请选择导出目录".into());
        }
        std::fs::create_dir_all(&root).map_err(|e| format!("无法创建导出目录：{e}"))?;
        let root = root
            .canonicalize()
            .map_err(|e| format!("无法解析导出目录：{e}"))?;
        let artifacts = state.db.project_artifacts(&project_id)?;
        if artifacts.is_empty() {
            return Err("项目还没有可导出的文件".into());
        }
        for artifact in artifacts {
            let relative = std::path::Path::new(&artifact.relative_path);
            if relative.is_absolute()
                || relative.components().any(|part| {
                    matches!(
                        part,
                        std::path::Component::ParentDir
                            | std::path::Component::RootDir
                            | std::path::Component::Prefix(_)
                    )
                })
            {
                return Err(format!("项目文件路径不安全：{}", artifact.relative_path));
            }
            let target = root.join(relative);
            if let Some(parent) = target.parent() {
                let mut cursor = root.clone();
                for component in relative
                    .parent()
                    .into_iter()
                    .flat_map(|path| path.components())
                {
                    cursor.push(component.as_os_str());
                    if cursor.exists()
                        && std::fs::symlink_metadata(&cursor)
                            .map(|metadata| metadata.file_type().is_symlink())
                            .unwrap_or(false)
                    {
                        return Err(format!("导出路径包含符号链接：{}", cursor.display()));
                    }
                }
                std::fs::create_dir_all(parent).map_err(|e| format!("无法创建项目子目录：{e}"))?;
            }
            std::fs::write(&target, artifact.content)
                .map_err(|e| format!("无法写入 {}：{e}", target.display()))?;
        }
        Ok(root.to_string_lossy().into_owned())
    }
    #[tauri::command]
    fn list_mcp_servers(state: State<'_, Arc<AppState>>) -> Result<Vec<McpServerConfig>, String> {
        state.db.mcp_servers()
    }
    #[tauri::command]
    fn save_mcp_server(
        server: McpServerConfig,
        state: State<'_, Arc<AppState>>,
    ) -> Result<McpServerConfig, String> {
        state.db.save_mcp_server(server)
    }
    #[tauri::command]
    fn delete_mcp_server(id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
        state.db.delete_mcp_server(&id)
    }
    #[tauri::command]
    async fn discover_mcp_tools(
        id: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<McpToolInfo>, String> {
        let server = state.db.mcp_server(&id)?.ok_or("MCP 服务器不存在")?;
        crate::mcp::list_tools(&server).await
    }
    #[tauri::command]
    async fn call_mcp_tool(
        id: String,
        name: String,
        arguments: serde_json::Value,
        project_id: Option<String>,
        conversation_id: Option<String>,
        state: State<'_, Arc<AppState>>,
    ) -> Result<serde_json::Value, String> {
        let server = state.db.mcp_server(&id)?.ok_or("MCP 服务器不存在")?;
        let created_at = chrono::Utc::now().to_rfc3339();
        let mut log = ToolCallLog {
            id: uuid::Uuid::new_v4().to_string(),
            project_id,
            conversation_id,
            server_id: id.clone(),
            tool_name: name.clone(),
            arguments: arguments.clone(),
            result: None,
            status: "running".into(),
            created_at,
        };
        match crate::mcp::call_tool(&server, &name, arguments).await {
            Ok(result) => {
                log.status = "completed".into();
                log.result = Some(result.clone());
                state.db.add_tool_call_log(&log)?;
                Ok(result)
            }
            Err(error) => {
                log.status = "failed".into();
                log.result = Some(serde_json::json!({"error":error}));
                state.db.add_tool_call_log(&log)?;
                Err(error)
            }
        }
    }
    #[tauri::command]
    fn export_prompt_preset(
        id: String,
        path: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<(), String> {
        let preset = state
            .db
            .presets()?
            .into_iter()
            .find(|preset| preset.id == id)
            .ok_or("预设不存在")?;
        let content = serde_json::to_string_pretty(&preset).map_err(|e| e.to_string())?;
        std::fs::write(path, content).map_err(|e| format!("无法导出预设：{e}"))
    }
    #[tauri::command]
    fn import_prompt_preset(
        path: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<PromptPreset, String> {
        let metadata = std::fs::metadata(&path).map_err(|e| format!("无法读取预设：{e}"))?;
        if metadata.len() > 1024 * 1024 {
            return Err("预设文件超过 1 MB，已拒绝导入".into());
        }
        let content = std::fs::read_to_string(path).map_err(|e| format!("无法读取预设：{e}"))?;
        let mut preset: PromptPreset =
            serde_json::from_str(&content).map_err(|e| format!("不是有效的 Locastra 预设：{e}"))?;
        preset.id = String::new();
        preset.created_at = String::new();
        preset.updated_at = String::new();
        state.db.save_preset(preset)
    }
    #[tauri::command]
    fn export_conversation(
        id: String,
        format: String,
        path: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<(), String> {
        let conversation = state.db.conversation(&id)?.ok_or("对话不存在")?;
        let content = match format.as_str() {
            "json" => serde_json::to_string_pretty(&conversation).map_err(|e| e.to_string())?,
            "markdown" | "md" => conversation_to_markdown(&conversation),
            _ => return Err("仅支持 Markdown 或 JSON 导出".into()),
        };
        let target = PathBuf::from(path);
        if target.as_os_str().is_empty() {
            return Err("导出路径不能为空".into());
        }
        std::fs::write(target, content).map_err(|e| format!("无法导出对话：{e}"))
    }
    #[tauri::command]
    fn save_generated_file(path: String, content: String) -> Result<(), String> {
        if content.len() > 16 * 1024 * 1024 {
            return Err("生成内容超过 16 MB，已拒绝保存".into());
        }
        let target = PathBuf::from(path);
        if target.as_os_str().is_empty() || target.file_name().is_none() {
            return Err("保存路径无效".into());
        }
        if let Some(parent) = target.parent() {
            if !parent.is_dir() {
                return Err("目标文件夹不存在".into());
            }
        }
        std::fs::write(&target, content).map_err(|e| format!("无法保存文件：{e}"))
    }
    #[tauri::command]
    fn reveal_path(path: String) -> Result<(), String> {
        let target = std::fs::canonicalize(&path).map_err(|e| format!("无法定位文件：{e}"))?;
        if target.is_dir() {
            std::process::Command::new("explorer.exe")
                .arg(&target)
                .spawn()
                .map_err(|e| format!("无法打开资源管理器：{e}"))?;
        } else {
            std::process::Command::new("explorer.exe")
                .arg(format!("/select,{}", target.to_string_lossy()))
                .spawn()
                .map_err(|e| format!("无法打开资源管理器：{e}"))?;
        }
        Ok(())
    }
    #[tauri::command]
    async fn web_search(
        query: String,
        max_results: Option<usize>,
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<WebSearchResult>, String> {
        let query = query.trim();
        if query.is_empty() {
            return Err("搜索内容不能为空".into());
        }
        if query.chars().count() > 500 {
            return Err("搜索内容过长，请缩短到 500 字以内".into());
        }
        let settings = state
            .db
            .settings(state.default_model_dir.to_string_lossy().into())?;
        crate::web_search::search(
            query,
            max_results.unwrap_or(5),
            settings.download_bypass_proxy,
        )
        .await
    }
    #[tauri::command]
    fn open_external_url(url: String) -> Result<(), String> {
        let parsed = reqwest::Url::parse(&url).map_err(|_| "链接格式无效".to_string())?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err("只允许打开 HTTP 或 HTTPS 链接".into());
        }
        std::process::Command::new("rundll32.exe")
            .arg("url.dll,FileProtocolHandler")
            .arg(parsed.as_str())
            .spawn()
            .map_err(|e| format!("无法打开系统浏览器：{e}"))?;
        Ok(())
    }
    #[tauri::command]
    fn import_conversation(
        path: String,
        state: State<'_, Arc<AppState>>,
    ) -> Result<Conversation, String> {
        let metadata = std::fs::metadata(&path).map_err(|e| format!("无法读取对话文件：{e}"))?;
        if metadata.len() > 64 * 1024 * 1024 {
            return Err("对话文件超过 64 MB，已拒绝导入".into());
        }
        let content =
            std::fs::read_to_string(&path).map_err(|e| format!("无法读取对话文件：{e}"))?;
        let source: Conversation = serde_json::from_str(&content)
            .map_err(|e| format!("不是有效的 Locastra JSON 对话：{e}"))?;
        let mut target = state
            .db
            .create_conversation(source.title, source.model_id)?;
        target.folder_id = source.folder_id;
        target.tags = source.tags;
        target.params = source.params;
        state.db.update_conversation(&target)?;
        for message in source.messages.unwrap_or_default() {
            if matches!(message.role.as_str(), "user" | "assistant" | "system") {
                state
                    .db
                    .add_message(&target.id, &message.role, &message.content)?;
            }
        }
        state
            .db
            .conversation(&target.id)?
            .ok_or("导入对话失败".into())
    }
    #[tauri::command]
    fn chat_stream(
        request_id: String,
        _conversation_id: String,
        messages: Vec<ChatMessage>,
        params: ChatParams,
        tools: Option<Vec<serde_json::Value>>,
        app: tauri::AppHandle,
        state: State<'_, Arc<AppState>>,
    ) -> Result<(), String> {
        if state.runtime.state().status != "running" {
            return Err("请先加载模型".into());
        }
        state
            .runtime
            .clone()
            .spawn_chat(app, request_id, messages, params, tools);
        Ok(())
    }
    #[tauri::command]
    fn stop_generation(request_id: String, state: State<'_, Arc<AppState>>) {
        state.runtime.stop(&request_id)
    }

    #[tauri::command]
    fn speak_text(text: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
        let content = text.trim();
        if content.is_empty() {
            return Err("没有可朗读的内容".into());
        }
        if content.chars().count() > 20_000 {
            return Err("朗读内容过长，请分段使用".into());
        }
        if let Some(mut child) = state.speech.lock().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let script="Add-Type -AssemblyName System.Speech; [Console]::InputEncoding=[Text.Encoding]::UTF8; $s=New-Object System.Speech.Synthesis.SpeechSynthesizer; $s.Speak($args[0])";
        let mut command = std::process::Command::new("powershell.exe");
        command
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                script,
                content,
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let child = command
            .spawn()
            .map_err(|e| format!("无法启动 Windows 语音合成：{e}"))?;
        *state.speech.lock() = Some(child);
        Ok(())
    }
    #[tauri::command]
    fn stop_speech(state: State<'_, Arc<AppState>>) {
        if let Some(mut child) = state.speech.lock().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    #[tauri::command]
    async fn dictate_once(language: Option<String>) -> Result<String, String> {
        let language = language.unwrap_or_else(|| "zh-CN".into());
        let script="Add-Type -AssemblyName System.Speech; [Console]::OutputEncoding=[Text.Encoding]::UTF8; try {$c=[Globalization.CultureInfo]::GetCultureInfo($args[0]); $r=New-Object System.Speech.Recognition.SpeechRecognitionEngine($c); $r.SetInputToDefaultAudioDevice(); $r.LoadGrammar((New-Object System.Speech.Recognition.DictationGrammar)); $x=$r.Recognize([TimeSpan]::FromSeconds(45)); if($null -ne $x){Write-Output $x.Text}} catch {[Console]::Error.WriteLine($_.Exception.Message); exit 1}";
        let mut command = tokio::process::Command::new("powershell.exe");
        command
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                script,
                &language,
            ])
            .kill_on_drop(true);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.as_std_mut().creation_flags(0x08000000);
        }
        let output = tokio::time::timeout(std::time::Duration::from_secs(55), command.output())
            .await
            .map_err(|_| "听写等待超时".to_string())?
            .map_err(|e| format!("无法启动 Windows 听写：{e}"))?;
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "Windows 离线听写不可用：{}。请在系统语言设置中安装中文语音包。",
                error.trim()
            ));
        }
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() {
            return Err("没有识别到语音，请靠近麦克风后重试".into());
        }
        Ok(text)
    }

    fn validate_local_gguf(path: &Path) -> Result<(), String> {
        use std::io::Read;
        let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)
            .map_err(|_| "文件太短，不是 GGUF".to_string())?;
        if &magic != b"GGUF" {
            return Err("文件头不是 GGUF，已拒绝导入".into());
        }
        Ok(())
    }

    fn extract_runtime_archive(archive: &Path, destination: &Path) -> Result<(), String> {
        let file = std::fs::File::open(archive).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("运行包不是有效 ZIP：{e}"))?;
        if zip.len() > 5000 {
            return Err("运行包文件数量异常".into());
        }
        let mut extracted = 0u64;
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).map_err(|e| e.to_string())?;
            let Some(relative) = entry.enclosed_name().map(|value| value.to_path_buf()) else {
                return Err("运行包包含不安全路径".into());
            };
            let target = destination.join(relative);
            if entry.is_dir() {
                std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
                continue;
            }
            extracted = extracted.saturating_add(entry.size());
            if extracted > 3 * 1024 * 1024 * 1024 {
                return Err("运行包解压大小异常".into());
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut output = std::fs::File::create(&target).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut output).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn installed_model_from_path(path: &Path) -> Result<InstalledModel, String> {
        let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or("文件名无效")?;
        Ok(InstalledModel {
            id: uuid::Uuid::new_v4().to_string(),
            display_name: file_name.trim_end_matches(".gguf").into(),
            repo_id: None,
            file_path: path.to_string_lossy().into(),
            file_size: metadata.len(),
            quantization: quantization_from_name(file_name),
            source: "local".into(),
            installed_at: chrono::Utc::now().to_rfc3339(),
            valid: true,
            favorite: false,
            note: String::new(),
            last_used_at: None,
            use_count: 0,
            config: ModelRuntimeConfig::default(),
        })
    }

    fn conversation_to_markdown(conversation: &Conversation) -> String {
        let mut output = format!("# {}\n\n", conversation.title);
        for message in conversation.messages.as_deref().unwrap_or_default() {
            let role = match message.role.as_str() {
                "user" => "你",
                "assistant" => "助手",
                "system" => "系统",
                _ => "消息",
            };
            output.push_str(&format!("## {role}\n\n{}\n\n", message.content));
        }
        output
    }

    fn extract_office_text(path: &Path, extension: &str) -> Result<String, String> {
        let file = std::fs::File::open(path).map_err(|e| format!("无法打开 Office 文档：{e}"))?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("Office 文档结构无效：{e}"))?;
        let accepted: &[&str] = match extension {
            "docx" => &["word/document.xml"],
            "pptx" => &["ppt/slides/"],
            "xlsx" => &["xl/sharedStrings.xml", "xl/worksheets/"],
            _ => &[],
        };
        let tag = regex::Regex::new(r"<[^>]+>").map_err(|e| e.to_string())?;
        let entities = [
            ("&lt;", "<"),
            ("&gt;", ">"),
            ("&amp;", "&"),
            ("&quot;", "\""),
        ];
        let mut output = String::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
            let name = entry.name().to_string();
            let include = accepted.iter().any(|prefix| {
                if prefix.ends_with('/') {
                    name.starts_with(prefix) && name.ends_with(".xml")
                } else {
                    name == *prefix
                }
            });
            if !include {
                continue;
            }
            let mut xml = String::new();
            std::io::Read::read_to_string(&mut entry, &mut xml).map_err(|e| e.to_string())?;
            xml = xml
                .replace("</w:p>", "\n")
                .replace("</a:p>", "\n")
                .replace("</row>", "\n")
                .replace("</si>", "\n");
            let mut text = tag.replace_all(&xml, " ").to_string();
            for (from, to) in entities {
                text = text.replace(from, to);
            }
            output.push_str(&text);
            output.push('\n');
        }
        let compact = output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if compact.is_empty() {
            Err("文档中没有可索引的文字；若内容为扫描图片，请先 OCR".into())
        } else {
            Ok(compact)
        }
    }

    fn create_support_zip(
        destination: &Path,
        entries: Vec<(String, Vec<u8>)>,
    ) -> Result<(), String> {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let temporary = destination.with_extension("zip.part");
        let file =
            std::fs::File::create(&temporary).map_err(|e| format!("无法创建导出文件：{e}"))?;
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o600);
        for (name, content) in entries {
            archive
                .start_file(name, options)
                .map_err(|e| format!("无法写入压缩包：{e}"))?;
            std::io::Write::write_all(&mut archive, &content)
                .map_err(|e| format!("无法写入压缩包：{e}"))?;
        }
        archive
            .finish()
            .map_err(|e| format!("无法完成压缩包：{e}"))?;
        std::fs::rename(&temporary, destination).map_err(|e| format!("无法保存导出文件：{e}"))?;
        Ok(())
    }

    #[tauri::command]
    fn export_diagnostics(path: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
        let destination = PathBuf::from(path);
        if destination.extension().and_then(|v| v.to_str()) != Some("zip") {
            return Err("诊断包必须保存为 .zip 文件".into());
        }
        let settings = state
            .db
            .settings(state.default_model_dir.to_string_lossy().into())?;
        let hardware = hardware::detect(Path::new(&settings.model_directory));
        let components = list_runtime_components(&settings.runtime_directory);
        let report = serde_json::json!({
            "generatedAt": chrono::Utc::now().to_rfc3339(),
            "appVersion": env!("CARGO_PKG_VERSION"),
            "os": hardware.os,
            "arch": hardware.arch,
            "cpu": hardware.cpu_name,
            "memoryBytes": hardware.total_memory_bytes,
            "gpus": hardware.gpus,
            "runtime": state.runtime.state(),
            "runtimeComponents": components,
            "settings": {
                "preferredSource": settings.preferred_source,
                "downloadBypassProxy": settings.download_bypass_proxy,
                "downloadConcurrency": settings.download_concurrency,
                "contextSize": settings.chat.context_size,
                "contextPolicy": settings.chat.context_policy,
                "runtimeDirectory": settings.runtime_directory,
            }
        });
        let log_path = dirs::data_local_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalDeploy")
            .join("logs")
            .join("llama-server.log");
        let log = std::fs::read(&log_path).unwrap_or_default();
        create_support_zip(
            &destination,
            vec![
                (
                    "diagnostics.json".into(),
                    serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
                ),
                ("logs/llama-server.log".into(), log),
                (
                    "README.txt".into(),
                    "Locastra 诊断包不包含聊天正文、模型文件或访问令牌。\r\n"
                        .as_bytes()
                        .to_vec(),
                ),
            ],
        )
    }

    #[tauri::command]
    fn backup_user_data(path: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
        let destination = PathBuf::from(path);
        if destination.extension().and_then(|v| v.to_str()) != Some("zip") {
            return Err("备份必须保存为 .zip 文件".into());
        }
        let database = state.db.checkpoint_for_backup()?;
        let database_bytes =
            std::fs::read(&database).map_err(|e| format!("无法读取本地数据库：{e}"))?;
        let manifest = serde_json::json!({
            "format": "locastra-backup",
            "version": 1,
            "appVersion": env!("CARGO_PKG_VERSION"),
            "createdAt": chrono::Utc::now().to_rfc3339(),
            "contains": ["settings", "conversations", "assistants", "knowledge metadata", "model metadata"],
            "excludes": ["GGUF model files", "download partial files"]
        });
        create_support_zip(
            &destination,
            vec![
                ("localdeploy.db".into(), database_bytes),
                (
                    "manifest.json".into(),
                    serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
                ),
            ],
        )
    }

    #[tauri::command]
    fn stage_user_data_restore(path: String) -> Result<(), String> {
        let source = PathBuf::from(path);
        if !source.is_file() {
            return Err("备份文件不存在".into());
        }
        let file = std::fs::File::open(&source).map_err(|e| format!("无法打开备份：{e}"))?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("备份格式无效：{e}"))?;
        let mut entry = archive
            .by_name("localdeploy.db")
            .map_err(|_| "备份中缺少 localdeploy.db")?;
        if entry.size() > 1024 * 1024 * 1024 {
            return Err("备份数据库异常大，已拒绝恢复".into());
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        std::io::Read::read_to_end(&mut entry, &mut bytes)
            .map_err(|e| format!("无法读取备份数据库：{e}"))?;
        if !bytes.starts_with(b"SQLite format 3\0") {
            return Err("备份中的数据库不是有效 SQLite 文件".into());
        }
        let base = dirs::data_local_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalDeploy");
        std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
        let temporary = base.join("restore-pending.db.part");
        let pending = base.join("restore-pending.db");
        std::fs::write(&temporary, bytes).map_err(|e| format!("无法暂存备份：{e}"))?;
        std::fs::rename(temporary, pending).map_err(|e| format!("无法暂存备份：{e}"))?;
        Ok(())
    }

    fn apply_pending_restore(base: &Path) -> Result<(), String> {
        let pending = base.join("restore-pending.db");
        if !pending.is_file() {
            return Ok(());
        }
        let current = base.join("localdeploy.db");
        let rollback = base.join("localdeploy.pre-restore.db");
        if current.is_file() {
            std::fs::copy(&current, &rollback)
                .map_err(|e| format!("无法创建恢复前回滚副本：{e}"))?;
        }
        for suffix in ["-wal", "-shm"] {
            let sidecar = PathBuf::from(format!("{}{}", current.to_string_lossy(), suffix));
            if sidecar.is_file() {
                std::fs::remove_file(sidecar).map_err(|e| format!("无法整理旧数据库：{e}"))?;
            }
        }
        std::fs::rename(&pending, &current).map_err(|error| {
            if rollback.is_file() {
                let _ = std::fs::copy(&rollback, &current);
            }
            format!("无法恢复备份，已保留旧数据：{error}")
        })?;
        Ok(())
    }

    pub fn run() {
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("LocalDeploy");
        let default_model_dir = base.join("models");
        let _ = apply_pending_restore(&base);
        let _ = std::fs::create_dir_all(&default_model_dir);
        let db = Arc::new(
            Database::open(&base.join("localdeploy.db")).expect("failed to open local database"),
        );
        let sources = Arc::new(ModelSources::new().expect("failed to build HTTP client"));
        let previous = db.downloads().unwrap_or_default();
        let state = Arc::new(AppState {
            db: db.clone(),
            sources,
            downloads: Arc::new(DownloadManager::new(previous)),
            runtime: Arc::new(RuntimeManager::new()),
            speech: Mutex::new(None),
            default_model_dir,
        });
        let app = tauri::Builder::default()
            .plugin(tauri_plugin_dialog::init())
            .manage(state)
            .setup(|app| {
                #[cfg(windows)]
                if let Some(window) = app.get_webview_window("main") {
                    window.with_webview(|webview| unsafe {
                        use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
                        use windows::core::Interface;

                        if let Ok(core) = webview.controller().CoreWebView2() {
                            if let Ok(settings) = core.Settings() {
                                let _ = settings.SetAreDefaultContextMenusEnabled(false);
                                let _ = settings.SetAreDevToolsEnabled(false);
                                let _ = settings.SetIsZoomControlEnabled(false);
                                if let Ok(settings3) = settings.cast::<ICoreWebView2Settings3>() {
                                    let _ = settings3.SetAreBrowserAcceleratorKeysEnabled(false);
                                }
                            }
                        }
                    })?;
                }

                let show = MenuItem::with_id(app, "show", "显示 Locastra", true, None::<&str>)?;
                let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show, &quit])?;
                if let Some(icon) = app.default_window_icon().cloned() {
                    TrayIconBuilder::new()
                        .icon(icon)
                        .tooltip("Locastra · 本地智聊")
                        .menu(&menu)
                        .on_menu_event(|app, event| match event.id.as_ref() {
                            "show" => {
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                            "quit" => app.exit(0),
                            _ => {}
                        })
                        .on_tray_icon_event(|tray, event| {
                            if matches!(
                                event,
                                TrayIconEvent::Click {
                                    button: MouseButton::Left,
                                    button_state: MouseButtonState::Up,
                                    ..
                                }
                            ) {
                                if let Some(window) = tray.app_handle().get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        })
                        .build(app)?;
                }
                Ok(())
            })
            .on_window_event(|window, event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    let state = window.state::<Arc<AppState>>();
                    if state
                        .db
                        .settings(state.default_model_dir.to_string_lossy().into())
                        .is_ok_and(|settings| settings.minimize_to_tray)
                    {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            })
            .invoke_handler(tauri::generate_handler![
                get_hardware_report,
                get_settings,
                save_settings,
                search_models,
                get_model_files,
                start_download,
                list_downloads,
                pause_download,
                resume_download,
                cancel_download,
                import_model,
                list_installed_models,
                update_installed_model,
                reveal_model,
                relocate_model,
                find_duplicate_models,
                scan_model_directory,
                move_model,
                benchmark_model,
                install_cuda_runtime,
                delete_model,
                load_model,
                unload_model,
                get_runtime_state,
                list_runtime_components_command,
                activate_runtime_component,
                rollback_runtime_component,
                count_tokens,
                summarize_context,
                check_for_app_update,
                install_app_update,
                rollback_app_update,
                list_conversations,
                search_conversations,
                create_conversation,
                get_conversation,
                delete_conversation,
                update_conversation,
                duplicate_conversation,
                add_message,
                create_image_attachment,
                update_message,
                set_message_stats,
                truncate_messages,
                list_conversation_folders,
                create_conversation_folder,
                delete_conversation_folder,
                list_prompt_presets,
                save_prompt_preset,
                delete_prompt_preset,
                list_assistants,
                save_assistant,
                delete_assistant,
                list_knowledge_documents,
                import_knowledge_document,
                delete_knowledge_document,
                retrieve_knowledge,
                rebuild_knowledge_index,
                list_projects,
                save_project,
                delete_project,
                list_project_artifacts,
                save_project_artifact,
                delete_project_artifact,
                export_project_artifacts,
                list_mcp_servers,
                save_mcp_server,
                delete_mcp_server,
                discover_mcp_tools,
                call_mcp_tool,
                export_prompt_preset,
                import_prompt_preset,
                export_conversation,
                save_generated_file,
                reveal_path,
                web_search,
                open_external_url,
                import_conversation,
                export_diagnostics,
                backup_user_data,
                stage_user_data_restore,
                chat_stream,
                stop_generation,
                speak_text,
                stop_speech,
                dictate_once
            ])
            .build(tauri::generate_context!())
            .expect("error while building Locastra");
        app.run(|handle, event| {
            if let tauri::RunEvent::Exit = event {
                let state = handle.state::<Arc<AppState>>();
                state.runtime.unload();
                if let Some(mut child) = state.speech.lock().take() {
                    let _ = child.kill();
                    let _ = child.wait();
                };
            }
        });
    }
}

#[cfg(feature = "desktop")]
pub use desktop::run;

#[cfg(not(feature = "desktop"))]
pub fn run() {}

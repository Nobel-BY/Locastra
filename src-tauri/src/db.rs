use crate::models::{
    AppSettings, AssistantProfile, ChatAttachment, ChatMessage, ChatParams, Conversation,
    ConversationFolder, DeveloperServiceSettings, DownloadTask, GenerationStats, InstalledModel,
    KnowledgeDocument, McpServerConfig, ModelRuntimeConfig, Project, ProjectArtifact, PromptPreset,
    ToolCallLog,
};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

pub struct Database {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;
          CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS installed_models (id TEXT PRIMARY KEY, display_name TEXT NOT NULL, repo_id TEXT, file_path TEXT NOT NULL UNIQUE, file_size INTEGER NOT NULL, quantization TEXT, source TEXT NOT NULL, installed_at TEXT NOT NULL, favorite INTEGER NOT NULL DEFAULT 0, note TEXT NOT NULL DEFAULT '', last_used_at TEXT, use_count INTEGER NOT NULL DEFAULT 0, config TEXT NOT NULL DEFAULT '{}');
          CREATE TABLE IF NOT EXISTS downloads (id TEXT PRIMARY KEY, payload TEXT NOT NULL, updated_at TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS conversations (id TEXT PRIMARY KEY, title TEXT NOT NULL, model_id TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, folder_id TEXT, pinned INTEGER NOT NULL DEFAULT 0, archived INTEGER NOT NULL DEFAULT 0, tags TEXT NOT NULL DEFAULT '[]', parent_id TEXT, params TEXT);
          CREATE TABLE IF NOT EXISTS messages (id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE, role TEXT NOT NULL, content TEXT NOT NULL, created_at TEXT NOT NULL, stats TEXT);
          CREATE TABLE IF NOT EXISTS conversation_folders (id TEXT PRIMARY KEY, name TEXT NOT NULL, parent_id TEXT, created_at TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS prompt_presets (id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, description TEXT NOT NULL, params TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS assistant_profiles (id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT NOT NULL, icon TEXT NOT NULL, model_id TEXT, params TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS knowledge_documents (id TEXT PRIMARY KEY, name TEXT NOT NULL, file_path TEXT NOT NULL UNIQUE, file_type TEXT NOT NULL, file_size INTEGER NOT NULL, content TEXT NOT NULL, created_at TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS mcp_servers (id TEXT PRIMARY KEY, payload TEXT NOT NULL, updated_at TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS projects (id TEXT PRIMARY KEY, payload TEXT NOT NULL, updated_at TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS project_artifacts (id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE, name TEXT NOT NULL, relative_path TEXT NOT NULL, language TEXT NOT NULL, content TEXT NOT NULL, version INTEGER NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, UNIQUE(project_id,relative_path));
          CREATE TABLE IF NOT EXISTS artifact_revisions (id INTEGER PRIMARY KEY AUTOINCREMENT, artifact_id TEXT NOT NULL REFERENCES project_artifacts(id) ON DELETE CASCADE, version INTEGER NOT NULL, content TEXT NOT NULL, created_at TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS knowledge_chunks (id TEXT PRIMARY KEY, document_id TEXT NOT NULL REFERENCES knowledge_documents(id) ON DELETE CASCADE, chunk_index INTEGER NOT NULL, page INTEGER, content TEXT NOT NULL, terms TEXT NOT NULL, embedding TEXT NOT NULL);
          CREATE INDEX IF NOT EXISTS idx_knowledge_chunks_document ON knowledge_chunks(document_id);
          CREATE TABLE IF NOT EXISTS tool_call_logs (id TEXT PRIMARY KEY, payload TEXT NOT NULL, created_at TEXT NOT NULL);")
            .map_err(|e| e.to_string())?;
        ensure_column(&conn, "conversations", "folder_id", "TEXT")?;
        ensure_column(
            &conn,
            "conversations",
            "pinned",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        ensure_column(
            &conn,
            "conversations",
            "archived",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        ensure_column(&conn, "conversations", "tags", "TEXT NOT NULL DEFAULT '[]'")?;
        ensure_column(&conn, "conversations", "parent_id", "TEXT")?;
        ensure_column(&conn, "conversations", "params", "TEXT")?;
        ensure_column(
            &conn,
            "conversations",
            "knowledge_ids",
            "TEXT NOT NULL DEFAULT '[]'",
        )?;
        ensure_column(&conn, "conversations", "context_state", "TEXT")?;
        ensure_column(&conn, "conversations", "project_id", "TEXT")?;
        ensure_column(&conn, "knowledge_documents", "project_id", "TEXT")?;
        ensure_column(
            &conn,
            "knowledge_documents",
            "index_status",
            "TEXT NOT NULL DEFAULT 'pending'",
        )?;
        ensure_column(&conn, "knowledge_documents", "index_error", "TEXT")?;
        ensure_column(
            &conn,
            "knowledge_documents",
            "chunk_count",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        ensure_column(&conn, "knowledge_documents", "indexed_at", "TEXT")?;
        ensure_column(&conn, "messages", "stats", "TEXT")?;
        ensure_column(
            &conn,
            "messages",
            "attachments",
            "TEXT NOT NULL DEFAULT '[]'",
        )?;
        ensure_column(
            &conn,
            "installed_models",
            "favorite",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        ensure_column(
            &conn,
            "installed_models",
            "note",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        ensure_column(&conn, "installed_models", "last_used_at", "TEXT")?;
        ensure_column(
            &conn,
            "installed_models",
            "use_count",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        ensure_column(
            &conn,
            "installed_models",
            "config",
            "TEXT NOT NULL DEFAULT '{}'",
        )?;
        let database = Self {
            conn: Mutex::new(conn),
            path: path.to_path_buf(),
        };
        database.seed_default_presets()?;
        Ok(database)
    }

    pub fn settings(&self, default_model_dir: String) -> Result<AppSettings, String> {
        let value: Option<String> = self
            .conn
            .lock()
            .query_row("SELECT value FROM settings WHERE key='app'", [], |r| {
                r.get(0)
            })
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some(value) = value {
            serde_json::from_str(&value).map_err(|e| e.to_string())
        } else {
            Ok(AppSettings {
                model_directory: default_model_dir,
                preferred_source: "modelscope".into(),
                custom_mirror: String::new(),
                download_bypass_proxy: true,
                download_concurrency: 4,
                onboarding_complete: false,
                chat: ChatParams {
                    temperature: 0.7,
                    context_size: 4096,
                    max_tokens: 2048,
                    enable_thinking: false,
                    system_prompt: "你是一位可靠、友善的中文助手。".into(),
                    top_p: 0.95,
                    top_k: 40,
                    min_p: 0.0,
                    repeat_penalty: 1.1,
                    context_policy: "auto".into(),
                    response_mode: "text".into(),
                    json_schema: String::new(),
                    grammar: String::new(),
                    validation_retries: 1,
                },
                theme: "dark".into(),
                font_size: 14,
                compact_mode: false,
                minimize_to_tray: false,
                auto_unload_minutes: 60,
                runtime_directory: String::new(),
                previous_runtime_directory: String::new(),
                developer_service: DeveloperServiceSettings::default(),
            })
        }
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), String> {
        let json = serde_json::to_string(settings).map_err(|e| e.to_string())?;
        self.conn.lock().execute("INSERT INTO settings(key,value) VALUES('app',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [json]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn checkpoint_for_backup(&self) -> Result<PathBuf, String> {
        self.conn
            .lock()
            .execute_batch("PRAGMA wal_checkpoint(FULL);")
            .map_err(|e| format!("无法整理本地数据库：{e}"))?;
        Ok(self.path.clone())
    }

    pub fn add_model(&self, model: &InstalledModel) -> Result<(), String> {
        let config = serde_json::to_string(&model.config).map_err(|e| e.to_string())?;
        self.conn.lock().execute("INSERT INTO installed_models(id,display_name,repo_id,file_path,file_size,quantization,source,installed_at,favorite,note,last_used_at,use_count,config) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13) ON CONFLICT(file_path) DO UPDATE SET display_name=excluded.display_name,file_size=excluded.file_size,quantization=excluded.quantization",
          params![model.id, model.display_name, model.repo_id, model.file_path, model.file_size, model.quantization, model.source, model.installed_at,model.favorite as i64,model.note,model.last_used_at,model.use_count as i64,config]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn models(&self) -> Result<Vec<InstalledModel>, String> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT id,display_name,repo_id,file_path,file_size,quantization,source,installed_at,favorite,note,last_used_at,use_count,config FROM installed_models ORDER BY favorite DESC,COALESCE(last_used_at,installed_at) DESC").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                let path: String = r.get(3)?;
                Ok(InstalledModel {
                    id: r.get(0)?,
                    display_name: r.get(1)?,
                    repo_id: r.get(2)?,
                    file_path: path.clone(),
                    file_size: r.get::<_, i64>(4)? as u64,
                    quantization: r.get(5)?,
                    source: r.get(6)?,
                    installed_at: r.get(7)?,
                    valid: Path::new(&path).is_file(),
                    favorite: r.get::<_, i64>(8)? != 0,
                    note: r.get(9)?,
                    last_used_at: r.get(10)?,
                    use_count: r.get::<_, i64>(11)? as u64,
                    config: serde_json::from_str::<ModelRuntimeConfig>(&r.get::<_, String>(12)?)
                        .unwrap_or_default(),
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn model(&self, id: &str) -> Result<Option<InstalledModel>, String> {
        Ok(self.models()?.into_iter().find(|m| m.id == id))
    }
    pub fn delete_model(&self, id: &str) -> Result<(), String> {
        self.conn
            .lock()
            .execute("DELETE FROM installed_models WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn update_model(&self, model: &InstalledModel) -> Result<(), String> {
        let config = serde_json::to_string(&model.config).map_err(|e| e.to_string())?;
        self.conn.lock().execute("UPDATE installed_models SET display_name=?1,file_path=?2,file_size=?3,quantization=?4,favorite=?5,note=?6,config=?7 WHERE id=?8",params![model.display_name,model.file_path,model.file_size as i64,model.quantization,model.favorite as i64,model.note,config,model.id]).map_err(|e|e.to_string())?;
        Ok(())
    }

    pub fn mark_model_used(&self, id: &str) -> Result<(), String> {
        self.conn
            .lock()
            .execute(
                "UPDATE installed_models SET last_used_at=?1,use_count=use_count+1 WHERE id=?2",
                params![chrono::Utc::now().to_rfc3339(), id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn save_download(&self, task: &DownloadTask) -> Result<(), String> {
        let payload = serde_json::to_string(task).map_err(|e| e.to_string())?;
        self.conn.lock().execute("INSERT INTO downloads(id,payload,updated_at) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload,updated_at=excluded.updated_at", params![task.id,payload,chrono::Utc::now().to_rfc3339()]).map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn downloads(&self) -> Result<Vec<DownloadTask>, String> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT payload FROM downloads ORDER BY updated_at DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        Ok(rows
            .filter_map(|r| r.ok())
            .filter_map(|s| serde_json::from_str(&s).ok())
            .collect())
    }

    pub fn create_conversation(
        &self,
        title: String,
        model_id: Option<String>,
    ) -> Result<Conversation, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.lock().execute("INSERT INTO conversations(id,title,model_id,created_at,updated_at,tags,knowledge_ids) VALUES(?1,?2,?3,?4,?4,'[]','[]')",params![id,title,model_id,now]).map_err(|e|e.to_string())?;
        Ok(Conversation {
            id,
            title,
            model_id,
            created_at: now.clone(),
            updated_at: now,
            messages: Some(vec![]),
            folder_id: None,
            pinned: false,
            archived: false,
            tags: vec![],
            parent_id: None,
            params: None,
            knowledge_document_ids: vec![],
            context_state: None,
            project_id: None,
        })
    }
    pub fn conversations(&self) -> Result<Vec<Conversation>, String> {
        let conn = self.conn.lock();
        let mut stmt=conn.prepare("SELECT id,title,model_id,created_at,updated_at,folder_id,pinned,archived,tags,parent_id,params,knowledge_ids,context_state,project_id FROM conversations ORDER BY pinned DESC,updated_at DESC").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Conversation {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    model_id: r.get(2)?,
                    created_at: r.get(3)?,
                    updated_at: r.get(4)?,
                    messages: None,
                    folder_id: r.get(5)?,
                    pinned: r.get::<_, i64>(6)? != 0,
                    archived: r.get::<_, i64>(7)? != 0,
                    tags: json_vec(r.get::<_, String>(8)?),
                    parent_id: r.get(9)?,
                    params: json_option(r.get::<_, Option<String>>(10)?),
                    knowledge_document_ids: json_vec(r.get::<_, String>(11)?),
                    context_state: json_option(r.get::<_, Option<String>>(12)?),
                    project_id: r.get(13)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn search_conversations(
        &self,
        query: &str,
        include_archived: bool,
    ) -> Result<Vec<Conversation>, String> {
        let needle = format!("%{}%", query.trim());
        let conn = self.conn.lock();
        let mut stmt=conn.prepare("SELECT DISTINCT c.id,c.title,c.model_id,c.created_at,c.updated_at,c.folder_id,c.pinned,c.archived,c.tags,c.parent_id,c.params,c.knowledge_ids,c.context_state,c.project_id FROM conversations c LEFT JOIN messages m ON m.conversation_id=c.id WHERE (?1=1 OR c.archived=0) AND (?2='%%' OR c.title LIKE ?2 OR m.content LIKE ?2 OR c.tags LIKE ?2) ORDER BY c.pinned DESC,c.updated_at DESC LIMIT 200").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map(params![include_archived as i64, needle], |r| {
                Ok(Conversation {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    model_id: r.get(2)?,
                    created_at: r.get(3)?,
                    updated_at: r.get(4)?,
                    messages: None,
                    folder_id: r.get(5)?,
                    pinned: r.get::<_, i64>(6)? != 0,
                    archived: r.get::<_, i64>(7)? != 0,
                    tags: json_vec(r.get::<_, String>(8)?),
                    parent_id: r.get(9)?,
                    params: json_option(r.get::<_, Option<String>>(10)?),
                    knowledge_document_ids: json_vec(r.get::<_, String>(11)?),
                    context_state: json_option(r.get::<_, Option<String>>(12)?),
                    project_id: r.get(13)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }
    pub fn conversation(&self, id: &str) -> Result<Option<Conversation>, String> {
        let conn = self.conn.lock();
        let mut conv: Option<Conversation> = conn
            .query_row(
                "SELECT id,title,model_id,created_at,updated_at,folder_id,pinned,archived,tags,parent_id,params,knowledge_ids,context_state,project_id FROM conversations WHERE id=?1",
                [id],
                |r| {
                    Ok(Conversation {
                        id: r.get(0)?,
                        title: r.get(1)?,
                        model_id: r.get(2)?,
                        created_at: r.get(3)?,
                        updated_at: r.get(4)?,
                        messages: None,
                        folder_id: r.get(5)?,
                        pinned: r.get::<_, i64>(6)? != 0,
                        archived: r.get::<_, i64>(7)? != 0,
                        tags: json_vec(r.get::<_, String>(8)?),
                        parent_id: r.get(9)?,
                        params: json_option(r.get::<_, Option<String>>(10)?),
                        knowledge_document_ids: json_vec(r.get::<_, String>(11)?),
                        context_state: json_option(r.get::<_, Option<String>>(12)?),
                        project_id: r.get(13)?,
                    })
                },
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some(c) = conv.as_mut() {
            let mut stmt=conn.prepare("SELECT id,conversation_id,role,content,created_at,stats,attachments FROM messages WHERE conversation_id=?1 ORDER BY created_at,rowid").map_err(|e|e.to_string())?;
            let rows = stmt
                .query_map([id], |r| {
                    Ok(ChatMessage {
                        id: r.get(0)?,
                        conversation_id: r.get(1)?,
                        role: r.get(2)?,
                        content: r.get(3)?,
                        created_at: r.get(4)?,
                        stats: json_option::<GenerationStats>(r.get::<_, Option<String>>(5)?),
                        attachments: serde_json::from_str(&r.get::<_, String>(6)?)
                            .unwrap_or_default(),
                    })
                })
                .map_err(|e| e.to_string())?;
            c.messages = Some(
                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(conv)
    }
    pub fn add_message(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
    ) -> Result<ChatMessage, String> {
        self.add_message_with_attachments(conversation_id, role, content, &[])
    }

    pub fn add_message_with_attachments(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
        attachments: &[ChatAttachment],
    ) -> Result<ChatMessage, String> {
        let msg = ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conversation_id.into(),
            role: role.into(),
            content: content.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            stats: None,
            attachments: attachments.to_vec(),
        };
        let attachments_json = serde_json::to_string(attachments).map_err(|e| e.to_string())?;
        let conn = self.conn.lock();
        conn.execute("INSERT INTO messages(id,conversation_id,role,content,created_at,attachments) VALUES(?1,?2,?3,?4,?5,?6)",params![msg.id,msg.conversation_id,msg.role,msg.content,msg.created_at,attachments_json]).map_err(|e|e.to_string())?;
        conn.execute("UPDATE conversations SET updated_at=?1,title=CASE WHEN title='新对话' AND ?2='user' THEN substr(?3,1,28) ELSE title END WHERE id=?4",params![msg.created_at,role,content,conversation_id]).map_err(|e|e.to_string())?;
        Ok(msg)
    }

    pub fn update_message(&self, id: &str, content: &str) -> Result<(), String> {
        self.conn
            .lock()
            .execute(
                "UPDATE messages SET content=?1,stats=NULL WHERE id=?2",
                params![content, id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_message_stats(&self, id: &str, stats: &GenerationStats) -> Result<(), String> {
        let value = serde_json::to_string(stats).map_err(|e| e.to_string())?;
        self.conn
            .lock()
            .execute(
                "UPDATE messages SET stats=?1 WHERE id=?2",
                params![value, id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_message_and_after(
        &self,
        conversation_id: &str,
        message_id: &str,
        include_message: bool,
    ) -> Result<(), String> {
        let conn = self.conn.lock();
        let rowid: i64 = conn
            .query_row(
                "SELECT rowid FROM messages WHERE id=?1 AND conversation_id=?2",
                params![message_id, conversation_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let op = if include_message { ">=" } else { ">" };
        conn.execute(
            &format!("DELETE FROM messages WHERE conversation_id=?1 AND rowid {op} ?2"),
            params![conversation_id, rowid],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE conversations SET updated_at=?1 WHERE id=?2",
            params![chrono::Utc::now().to_rfc3339(), conversation_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn update_conversation(&self, conversation: &Conversation) -> Result<(), String> {
        let tags = serde_json::to_string(&conversation.tags).map_err(|e| e.to_string())?;
        let chat_params = conversation
            .params
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| e.to_string())?;
        let knowledge_ids = serde_json::to_string(&conversation.knowledge_document_ids)
            .map_err(|e| e.to_string())?;
        let context_state = conversation
            .context_state
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| e.to_string())?;
        self.conn.lock().execute(
            "UPDATE conversations SET title=?1,model_id=?2,folder_id=?3,pinned=?4,archived=?5,tags=?6,params=?7,knowledge_ids=?8,context_state=?9,updated_at=?10,project_id=?11,parent_id=?12 WHERE id=?13",
            params![conversation.title,conversation.model_id,conversation.folder_id,conversation.pinned as i64,conversation.archived as i64,tags,chat_params,knowledge_ids,context_state,chrono::Utc::now().to_rfc3339(),conversation.project_id,conversation.parent_id,conversation.id],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn duplicate_conversation(
        &self,
        id: &str,
        through_message_id: Option<&str>,
    ) -> Result<Conversation, String> {
        let source = self.conversation(id)?.ok_or("对话不存在")?;
        let mut target =
            self.create_conversation(format!("{}（副本）", source.title), source.model_id.clone())?;
        target.folder_id = source.folder_id.clone();
        target.tags = source.tags.clone();
        target.parent_id = Some(source.id.clone());
        target.params = source.params.clone();
        target.knowledge_document_ids = source.knowledge_document_ids.clone();
        target.context_state = source.context_state.clone();
        target.project_id = source.project_id.clone();
        self.update_conversation(&target)?;
        for message in source.messages.unwrap_or_default() {
            self.add_message_with_attachments(
                &target.id,
                &message.role,
                &message.content,
                &message.attachments,
            )?;
            if through_message_id.is_some_and(|message_id| message.id == message_id) {
                break;
            }
        }
        self.conversation(&target.id)?
            .ok_or("创建对话副本失败".into())
    }

    pub fn folders(&self) -> Result<Vec<ConversationFolder>, String> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT id,name,parent_id,created_at FROM conversation_folders ORDER BY name COLLATE NOCASE").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(ConversationFolder {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    parent_id: r.get(2)?,
                    created_at: r.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn create_folder(
        &self,
        name: &str,
        parent_id: Option<String>,
    ) -> Result<ConversationFolder, String> {
        let folder = ConversationFolder {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.trim().to_string(),
            parent_id,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        if folder.name.is_empty() {
            return Err("文件夹名称不能为空".into());
        }
        self.conn.lock().execute("INSERT INTO conversation_folders(id,name,parent_id,created_at) VALUES(?1,?2,?3,?4)",params![folder.id,folder.name,folder.parent_id,folder.created_at]).map_err(|e|e.to_string())?;
        Ok(folder)
    }

    pub fn delete_folder(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE conversations SET folder_id=NULL WHERE folder_id=?1",
            [id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM conversation_folders WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn presets(&self) -> Result<Vec<PromptPreset>, String> {
        let conn = self.conn.lock();
        let mut stmt=conn.prepare("SELECT id,name,description,params,created_at,updated_at FROM prompt_presets ORDER BY updated_at DESC").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                let value: String = r.get(3)?;
                let params = serde_json::from_str(&value).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(PromptPreset {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    description: r.get(2)?,
                    params,
                    created_at: r.get(4)?,
                    updated_at: r.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn save_preset(&self, mut preset: PromptPreset) -> Result<PromptPreset, String> {
        if preset.id.is_empty() {
            preset.id = uuid::Uuid::new_v4().to_string();
        }
        let now = chrono::Utc::now().to_rfc3339();
        if preset.created_at.is_empty() {
            preset.created_at = now.clone();
        }
        preset.updated_at = now;
        let value = serde_json::to_string(&preset.params).map_err(|e| e.to_string())?;
        self.conn.lock().execute("INSERT INTO prompt_presets(id,name,description,params,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET name=excluded.name,description=excluded.description,params=excluded.params,updated_at=excluded.updated_at",params![preset.id,preset.name,preset.description,value,preset.created_at,preset.updated_at]).map_err(|e|e.to_string())?;
        Ok(preset)
    }

    pub fn delete_preset(&self, id: &str) -> Result<(), String> {
        self.conn
            .lock()
            .execute("DELETE FROM prompt_presets WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn assistants(&self) -> Result<Vec<AssistantProfile>, String> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT id,name,description,icon,model_id,params,created_at,updated_at FROM assistant_profiles ORDER BY updated_at DESC").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                let raw: String = r.get(5)?;
                let params = serde_json::from_str(&raw).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        5,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(AssistantProfile {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    description: r.get(2)?,
                    icon: r.get(3)?,
                    model_id: r.get(4)?,
                    params,
                    created_at: r.get(6)?,
                    updated_at: r.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn save_assistant(
        &self,
        mut assistant: AssistantProfile,
    ) -> Result<AssistantProfile, String> {
        if assistant.name.trim().is_empty() {
            return Err("助手名称不能为空".into());
        }
        let now = chrono::Utc::now().to_rfc3339();
        if assistant.id.is_empty() {
            assistant.id = uuid::Uuid::new_v4().to_string();
            assistant.created_at = now.clone();
        }
        assistant.updated_at = now;
        let params = serde_json::to_string(&assistant.params).map_err(|e| e.to_string())?;
        self.conn.lock().execute("INSERT INTO assistant_profiles(id,name,description,icon,model_id,params,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(id) DO UPDATE SET name=excluded.name,description=excluded.description,icon=excluded.icon,model_id=excluded.model_id,params=excluded.params,updated_at=excluded.updated_at",params![assistant.id,assistant.name,assistant.description,assistant.icon,assistant.model_id,params,assistant.created_at,assistant.updated_at]).map_err(|e|e.to_string())?;
        Ok(assistant)
    }

    pub fn delete_assistant(&self, id: &str) -> Result<(), String> {
        self.conn
            .lock()
            .execute("DELETE FROM assistant_profiles WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn add_knowledge_document(
        &self,
        document: &KnowledgeDocument,
        content: &str,
    ) -> Result<(), String> {
        self.conn.lock().execute("INSERT INTO knowledge_documents(id,name,file_path,file_type,file_size,content,created_at,project_id,index_status,index_error,chunk_count,indexed_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12) ON CONFLICT(file_path) DO UPDATE SET name=excluded.name,file_type=excluded.file_type,file_size=excluded.file_size,content=excluded.content,project_id=COALESCE(excluded.project_id,knowledge_documents.project_id),index_status=excluded.index_status,index_error=excluded.index_error,chunk_count=excluded.chunk_count,indexed_at=excluded.indexed_at",params![document.id,document.name,document.file_path,document.file_type,document.file_size as i64,content,document.created_at,document.project_id,document.index_status,document.index_error,document.chunk_count,document.indexed_at]).map_err(|e|e.to_string())?;
        Ok(())
    }

    pub fn knowledge_documents(&self) -> Result<Vec<KnowledgeDocument>, String> {
        let conn = self.conn.lock();
        let mut stmt=conn.prepare("SELECT id,name,file_path,file_type,file_size,length(content),created_at,project_id,index_status,index_error,chunk_count,indexed_at FROM knowledge_documents ORDER BY created_at DESC").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(KnowledgeDocument {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    file_path: r.get(2)?,
                    file_type: r.get(3)?,
                    file_size: r.get::<_, i64>(4)? as u64,
                    character_count: r.get::<_, i64>(5)? as u64,
                    created_at: r.get(6)?,
                    project_id: r.get(7)?,
                    index_status: r.get(8)?,
                    index_error: r.get(9)?,
                    chunk_count: r.get::<_, i64>(10)? as u32,
                    indexed_at: r.get(11)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn knowledge_contents(
        &self,
        ids: &[String],
    ) -> Result<Vec<(KnowledgeDocument, String)>, String> {
        let conn = self.conn.lock();
        let mut output = Vec::new();
        for id in ids {
            if let Some(value)=conn.query_row("SELECT id,name,file_path,file_type,file_size,content,created_at,project_id,index_status,index_error,chunk_count,indexed_at FROM knowledge_documents WHERE id=?1",[id],|r|{let content:String=r.get(5)?;Ok((KnowledgeDocument{id:r.get(0)?,name:r.get(1)?,file_path:r.get(2)?,file_type:r.get(3)?,file_size:r.get::<_,i64>(4)? as u64,character_count:content.chars().count() as u64,created_at:r.get(6)?,project_id:r.get(7)?,index_status:r.get(8)?,index_error:r.get(9)?,chunk_count:r.get::<_,i64>(10)? as u32,indexed_at:r.get(11)?},content))}).optional().map_err(|e|e.to_string())? { output.push(value); }
        }
        Ok(output)
    }

    pub fn replace_knowledge_chunks(
        &self,
        document_id: &str,
        chunks: &[crate::rag::IndexedChunk],
    ) -> Result<(), String> {
        let mut conn = self.conn.lock();
        let transaction = conn.transaction().map_err(|e| e.to_string())?;
        transaction
            .execute(
                "DELETE FROM knowledge_chunks WHERE document_id=?1",
                [document_id],
            )
            .map_err(|e| e.to_string())?;
        for chunk in chunks {
            let terms = serde_json::to_string(&chunk.terms).map_err(|e| e.to_string())?;
            let embedding = serde_json::to_string(&chunk.embedding).map_err(|e| e.to_string())?;
            transaction.execute("INSERT INTO knowledge_chunks(id,document_id,chunk_index,page,content,terms,embedding) VALUES(?1,?2,?3,?4,?5,?6,?7)", params![chunk.id,chunk.document_id,chunk.chunk_index as i64,chunk.page,chunk.content,terms,embedding]).map_err(|e| e.to_string())?;
        }
        transaction.execute("UPDATE knowledge_documents SET index_status='ready',index_error=NULL,chunk_count=?1,indexed_at=?2 WHERE id=?3", params![chunks.len() as i64,chrono::Utc::now().to_rfc3339(),document_id]).map_err(|e| e.to_string())?;
        transaction.commit().map_err(|e| e.to_string())
    }

    pub fn knowledge_chunks(
        &self,
        document_ids: &[String],
    ) -> Result<Vec<crate::rag::IndexedChunk>, String> {
        let conn = self.conn.lock();
        let mut output = Vec::new();
        for document_id in document_ids {
            let mut stmt=conn.prepare("SELECT c.id,c.document_id,d.name,d.file_path,c.chunk_index,c.page,c.content,c.terms,c.embedding FROM knowledge_chunks c JOIN knowledge_documents d ON d.id=c.document_id WHERE c.document_id=?1 ORDER BY c.chunk_index").map_err(|e|e.to_string())?;
            let rows = stmt
                .query_map([document_id], |r| {
                    Ok(crate::rag::IndexedChunk {
                        id: r.get(0)?,
                        document_id: r.get(1)?,
                        document_name: r.get(2)?,
                        source_path: r.get(3)?,
                        chunk_index: r.get::<_, i64>(4)? as usize,
                        page: r.get(5)?,
                        content: r.get(6)?,
                        terms: serde_json::from_str(&r.get::<_, String>(7)?).unwrap_or_default(),
                        embedding: serde_json::from_str(&r.get::<_, String>(8)?)
                            .unwrap_or_default(),
                    })
                })
                .map_err(|e| e.to_string())?;
            output.extend(rows.filter_map(Result::ok));
        }
        Ok(output)
    }

    pub fn mark_knowledge_index_error(&self, id: &str, error: &str) -> Result<(), String> {
        self.conn
            .lock()
            .execute(
                "UPDATE knowledge_documents SET index_status='failed',index_error=?1 WHERE id=?2",
                params![error, id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_knowledge_document(&self, id: &str) -> Result<(), String> {
        self.conn
            .lock()
            .execute("DELETE FROM knowledge_documents WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn mcp_servers(&self) -> Result<Vec<McpServerConfig>, String> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT payload FROM mcp_servers ORDER BY updated_at DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        Ok(rows
            .filter_map(Result::ok)
            .filter_map(|value| serde_json::from_str(&value).ok())
            .collect())
    }

    pub fn save_mcp_server(&self, mut server: McpServerConfig) -> Result<McpServerConfig, String> {
        if server.name.trim().is_empty() || server.command.trim().is_empty() {
            return Err("MCP 名称和启动命令不能为空".into());
        }
        let now = chrono::Utc::now().to_rfc3339();
        if server.id.is_empty() {
            server.id = uuid::Uuid::new_v4().to_string();
            server.created_at = now.clone();
        }
        server.updated_at = now.clone();
        let payload = serde_json::to_string(&server).map_err(|e| e.to_string())?;
        self.conn.lock().execute("INSERT INTO mcp_servers(id,payload,updated_at) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload,updated_at=excluded.updated_at",params![server.id,payload,now]).map_err(|e|e.to_string())?;
        Ok(server)
    }

    pub fn mcp_server(&self, id: &str) -> Result<Option<McpServerConfig>, String> {
        let value: Option<String> = self
            .conn
            .lock()
            .query_row("SELECT payload FROM mcp_servers WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(value.and_then(|item| serde_json::from_str(&item).ok()))
    }
    pub fn delete_mcp_server(&self, id: &str) -> Result<(), String> {
        self.conn
            .lock()
            .execute("DELETE FROM mcp_servers WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn projects(&self) -> Result<Vec<Project>, String> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT payload FROM projects ORDER BY updated_at DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        Ok(rows
            .filter_map(Result::ok)
            .filter_map(|value| serde_json::from_str(&value).ok())
            .collect())
    }

    pub fn save_project(&self, mut project: Project) -> Result<Project, String> {
        if project.name.trim().is_empty() {
            return Err("项目名称不能为空".into());
        }
        let now = chrono::Utc::now().to_rfc3339();
        if project.id.is_empty() {
            project.id = uuid::Uuid::new_v4().to_string();
            project.created_at = now.clone();
        }
        project.updated_at = now.clone();
        let payload = serde_json::to_string(&project).map_err(|e| e.to_string())?;
        self.conn.lock().execute("INSERT INTO projects(id,payload,updated_at) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload,updated_at=excluded.updated_at",params![project.id,payload,now]).map_err(|e|e.to_string())?;
        Ok(project)
    }

    pub fn delete_project(&self, id: &str) -> Result<(), String> {
        self.conn
            .lock()
            .execute("DELETE FROM projects WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn project_artifacts(&self, project_id: &str) -> Result<Vec<ProjectArtifact>, String> {
        let conn = self.conn.lock();
        let mut stmt=conn.prepare("SELECT id,project_id,name,relative_path,language,content,version,created_at,updated_at FROM project_artifacts WHERE project_id=?1 ORDER BY relative_path").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map([project_id], |r| {
                Ok(ProjectArtifact {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    name: r.get(2)?,
                    relative_path: r.get(3)?,
                    language: r.get(4)?,
                    content: r.get(5)?,
                    version: r.get::<_, i64>(6)? as u32,
                    created_at: r.get(7)?,
                    updated_at: r.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn save_project_artifact(
        &self,
        mut artifact: ProjectArtifact,
    ) -> Result<ProjectArtifact, String> {
        if artifact.relative_path.trim().is_empty()
            || Path::new(&artifact.relative_path).is_absolute()
            || artifact
                .relative_path
                .split(['/', '\\'])
                .any(|part| part == ".." || part.is_empty())
        {
            return Err("文件必须使用项目内的安全相对路径".into());
        }
        if artifact.content.len() > 16 * 1024 * 1024 {
            return Err("单个项目文件不能超过 16 MB".into());
        }
        let now = chrono::Utc::now().to_rfc3339();
        let mut conn = self.conn.lock();
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        if artifact.id.is_empty() {
            artifact.id = uuid::Uuid::new_v4().to_string();
            artifact.version = 1;
            artifact.created_at = now.clone();
        } else if let Some((version, content)) = tx
            .query_row(
                "SELECT version,content FROM project_artifacts WHERE id=?1",
                [&artifact.id],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?
        {
            tx.execute("INSERT INTO artifact_revisions(artifact_id,version,content,created_at) VALUES(?1,?2,?3,?4)",params![artifact.id,version,content,now]).map_err(|e|e.to_string())?;
            artifact.version = version as u32 + 1;
        }
        artifact.name = Path::new(&artifact.relative_path)
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("file")
            .to_string();
        artifact.updated_at = now;
        tx.execute("INSERT INTO project_artifacts(id,project_id,name,relative_path,language,content,version,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(id) DO UPDATE SET name=excluded.name,relative_path=excluded.relative_path,language=excluded.language,content=excluded.content,version=excluded.version,updated_at=excluded.updated_at",params![artifact.id,artifact.project_id,artifact.name,artifact.relative_path,artifact.language,artifact.content,artifact.version,artifact.created_at,artifact.updated_at]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(artifact)
    }

    pub fn delete_project_artifact(&self, id: &str) -> Result<(), String> {
        self.conn
            .lock()
            .execute("DELETE FROM project_artifacts WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn add_tool_call_log(&self, log: &ToolCallLog) -> Result<(), String> {
        let payload = serde_json::to_string(log).map_err(|e| e.to_string())?;
        self.conn
            .lock()
            .execute(
                "INSERT INTO tool_call_logs(id,payload,created_at) VALUES(?1,?2,?3)",
                params![log.id, payload, log.created_at],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn seed_default_presets(&self) -> Result<(), String> {
        let count: i64 = self
            .conn
            .lock()
            .query_row("SELECT COUNT(*) FROM prompt_presets", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        if count > 0 {
            return Ok(());
        }
        let defaults = [
            ("精准问答", "降低随机性，适合事实说明和严谨分析。", 0.2, "你是一位严谨、准确的中文助手。先给出结论，再说明依据；不确定时明确说明。"),
            ("头脑风暴", "提高创造性，适合快速产生不同方案。", 0.95, "你是一位善于发散思考的中文助手。针对问题提出多个差异明显的方案，并说明各自优缺点。"),
            ("代码助手", "适合编程、调试和代码解释。", 0.3, "你是一位资深软件工程师。优先给出可运行、可验证的解决方案，指出风险和测试方法。"),
            ("翻译助手", "保留语义、语气和原文格式。", 0.3, "你是一位专业翻译。准确保留原文含义、语气、术语和格式；有歧义时给出简短说明。"),
        ];
        for (name, description, temperature, system_prompt) in defaults {
            self.save_preset(PromptPreset {
                id: String::new(),
                name: name.into(),
                description: description.into(),
                params: ChatParams {
                    temperature,
                    context_size: 4096,
                    max_tokens: 2048,
                    enable_thinking: false,
                    system_prompt: system_prompt.into(),
                    top_p: 0.95,
                    top_k: 40,
                    min_p: 0.0,
                    repeat_penalty: 1.1,
                    context_policy: "auto".into(),
                    response_mode: "text".into(),
                    json_schema: String::new(),
                    grammar: String::new(),
                    validation_retries: 1,
                },
                created_at: String::new(),
                updated_at: String::new(),
            })?;
        }
        Ok(())
    }
    pub fn delete_conversation(&self, id: &str) -> Result<(), String> {
        self.conn
            .lock()
            .execute("DELETE FROM conversations WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| e.to_string())?;
    let columns = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    if !columns.iter().any(|name| name == column) {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn json_vec(value: String) -> Vec<String> {
    serde_json::from_str(&value).unwrap_or_default()
}
fn json_option<T: serde::de::DeserializeOwned>(value: Option<String>) -> Option<T> {
    value.and_then(|item| serde_json::from_str(&item).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversation_productivity_round_trip() {
        let directory = tempfile::tempdir().expect("temp dir");
        let database = Database::open(&directory.path().join("app.db")).expect("database");
        assert!(database.presets().expect("presets").len() >= 4);

        let folder = database.create_folder("工作", None).expect("folder");
        let mut conversation = database
            .create_conversation("测试对话".into(), None)
            .expect("conversation");
        conversation.folder_id = Some(folder.id.clone());
        conversation.pinned = true;
        conversation.tags = vec!["测试".into(), "本地".into()];
        conversation.params = Some(ChatParams {
            temperature: 0.4,
            context_size: 8192,
            max_tokens: 1024,
            enable_thinking: false,
            system_prompt: "测试助手".into(),
            top_p: 0.9,
            top_k: 30,
            min_p: 0.05,
            repeat_penalty: 1.05,
            context_policy: "auto".into(),
            response_mode: "text".into(),
            json_schema: String::new(),
            grammar: String::new(),
            validation_retries: 1,
        });
        database
            .update_conversation(&conversation)
            .expect("update conversation");
        let first = database
            .add_message(&conversation.id, "user", "搜索关键词 alpha")
            .expect("first message");
        database
            .add_message(&conversation.id, "assistant", "第一条回答")
            .expect("answer");

        let found = database
            .search_conversations("alpha", false)
            .expect("search");
        assert_eq!(found.len(), 1);
        assert!(found[0].pinned);
        assert_eq!(found[0].folder_id.as_deref(), Some(folder.id.as_str()));

        let branch = database
            .duplicate_conversation(&conversation.id, Some(&first.id))
            .expect("branch");
        assert_eq!(branch.parent_id.as_deref(), Some(conversation.id.as_str()));
        assert_eq!(branch.messages.as_ref().map(Vec::len), Some(1));

        database
            .delete_message_and_after(&conversation.id, &first.id, false)
            .expect("truncate");
        let truncated = database
            .conversation(&conversation.id)
            .expect("read")
            .expect("exists");
        assert_eq!(truncated.messages.as_ref().map(Vec::len), Some(1));
    }

    #[test]
    fn assistants_knowledge_attachments_and_mcp_round_trip() {
        let directory = tempfile::tempdir().expect("temp dir");
        let database = Database::open(&directory.path().join("app.db")).expect("database");
        let params = ChatParams {
            temperature: 0.6,
            context_size: 16384,
            max_tokens: 3072,
            enable_thinking: false,
            system_prompt: "只根据本地资料回答".into(),
            top_p: 0.9,
            top_k: 40,
            min_p: 0.05,
            repeat_penalty: 1.1,
            context_policy: "auto".into(),
            response_mode: "text".into(),
            json_schema: String::new(),
            grammar: String::new(),
            validation_retries: 1,
        };
        let assistant = database
            .save_assistant(AssistantProfile {
                id: String::new(),
                name: "资料助手".into(),
                description: "测试".into(),
                icon: "📚".into(),
                model_id: None,
                params: params.clone(),
                created_at: String::new(),
                updated_at: String::new(),
            })
            .expect("assistant");
        assert!(!assistant.id.is_empty());
        assert_eq!(
            database.assistants().expect("assistants")[0]
                .params
                .context_size,
            16384
        );

        let document = KnowledgeDocument {
            id: uuid::Uuid::new_v4().to_string(),
            name: "说明.md".into(),
            file_path: directory
                .path()
                .join("说明.md")
                .to_string_lossy()
                .into_owned(),
            file_type: "md".into(),
            file_size: 12,
            character_count: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
            project_id: None,
            index_status: "pending".into(),
            index_error: None,
            chunk_count: 0,
            indexed_at: None,
        };
        database
            .add_knowledge_document(&document, "本地资料内容")
            .expect("knowledge");
        let contents = database
            .knowledge_contents(std::slice::from_ref(&document.id))
            .expect("contents");
        assert_eq!(contents[0].1, "本地资料内容");

        let conversation = database
            .create_conversation("附件测试".into(), None)
            .expect("conversation");
        let attachment = ChatAttachment {
            id: uuid::Uuid::new_v4().to_string(),
            name: "图片.png".into(),
            file_path: directory
                .path()
                .join("图片.png")
                .to_string_lossy()
                .into_owned(),
            mime_type: "image/png".into(),
            file_size: 42,
        };
        database
            .add_message_with_attachments(
                &conversation.id,
                "user",
                "看图",
                std::slice::from_ref(&attachment),
            )
            .expect("message");
        let saved = database
            .conversation(&conversation.id)
            .expect("read")
            .expect("exists");
        assert_eq!(
            saved.messages.expect("messages")[0].attachments[0].mime_type,
            "image/png"
        );

        let server = database
            .save_mcp_server(McpServerConfig {
                id: String::new(),
                name: "测试 MCP".into(),
                command: "server.exe".into(),
                args: vec!["--stdio".into()],
                working_directory: None,
                enabled: true,
                created_at: String::new(),
                updated_at: String::new(),
            })
            .expect("mcp");
        assert_eq!(
            database
                .mcp_server(&server.id)
                .expect("mcp read")
                .expect("mcp exists")
                .args,
            vec!["--stdio"]
        );
    }

    #[test]
    fn projects_and_versioned_artifacts_round_trip_safely() {
        let directory = tempfile::tempdir().expect("temp dir");
        let database = Database::open(&directory.path().join("app.db")).expect("database");
        let project = database
            .save_project(Project {
                id: String::new(),
                name: "网站项目".into(),
                description: "测试项目工作区".into(),
                instructions: "只使用本地资料".into(),
                model_id: None,
                params: ChatParams {
                    temperature: 0.4,
                    context_size: 8192,
                    max_tokens: 2048,
                    enable_thinking: false,
                    system_prompt: String::new(),
                    top_p: 0.9,
                    top_k: 40,
                    min_p: 0.0,
                    repeat_penalty: 1.1,
                    context_policy: "auto".into(),
                    response_mode: "text".into(),
                    json_schema: String::new(),
                    grammar: String::new(),
                    validation_retries: 1,
                },
                knowledge_document_ids: Vec::new(),
                mcp_server_ids: Vec::new(),
                tool_policy: "ask".into(),
                created_at: String::new(),
                updated_at: String::new(),
            })
            .expect("project");
        assert_eq!(database.projects().expect("projects")[0].name, "网站项目");

        let first = database
            .save_project_artifact(ProjectArtifact {
                id: String::new(),
                project_id: project.id.clone(),
                name: String::new(),
                relative_path: "src/index.html".into(),
                language: "html".into(),
                content: "v1".into(),
                version: 0,
                created_at: String::new(),
                updated_at: String::new(),
            })
            .expect("first artifact");
        let second = database
            .save_project_artifact(ProjectArtifact {
                content: "v2".into(),
                ..first
            })
            .expect("second artifact");
        assert_eq!(second.version, 2);
        assert_eq!(
            database.project_artifacts(&project.id).expect("artifacts")[0].content,
            "v2"
        );

        let unsafe_artifact = ProjectArtifact {
            id: String::new(),
            project_id: project.id,
            name: String::new(),
            relative_path: "../escape.txt".into(),
            language: "text".into(),
            content: "blocked".into(),
            version: 0,
            created_at: String::new(),
            updated_at: String::new(),
        };
        assert!(database.save_project_artifact(unsafe_artifact).is_err());
    }
}

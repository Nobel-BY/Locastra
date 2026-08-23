import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppSettings, AppUpdateInfo, AssistantProfile, ChatAttachment, ChatMessage, ChatParams, ChatToolCall, ChatToolDefinition, Conversation, ConversationFolder, DownloadRequest, DownloadTask,
  GenerationStats, HardwareReport, InstalledModel, KnowledgeDocument, KnowledgeSnippet, McpServerConfig, McpToolInfo, ModelBenchmark, ModelFile, ModelSearchResult, ModelSearchSource, ModelSource, Project, ProjectArtifact, PromptPreset, RuntimeComponent, RuntimeState, WebSearchResult
} from "../types";

export const isDesktop = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const now = new Date().toISOString();
const mockHardware: HardwareReport = {
  os: "Windows 11", arch: "x86_64", cpuName: "演示模式 · AMD Ryzen 7 7840HS",
  physicalCores: 8, logicalCores: 16, instructionSets: ["AVX", "AVX2", "FMA"],
  totalMemoryBytes: 32 * 1024 ** 3, availableMemoryBytes: 19.6 * 1024 ** 3,
  gpus: [{ name: "AMD Radeon 780M", vendor: "AMD", dedicatedMemoryBytes: 8 * 1024 ** 3 }],
  freeDiskBytes: 186 * 1024 ** 3, modelDirectory: "D:\\AIModels", acceleration: "vulkan"
};

const mockResults: ModelSearchResult[] = [
  { id: "modelscope:Qwen/Qwen3-4B-GGUF", repoId: "Qwen/Qwen3-4B-GGUF", name: "Qwen3 4B 中文通用", author: "Qwen", source: "modelscope", license: "Apache-2.0", parameterCount: 4_000_000_000, downloads: 3490510, likes: 1800, tags: ["GGUF", "中文", "推理"], compatible: true, updatedAt: now, description: "适合大多数电脑的中文通用对话模型，兼顾速度和效果。" },
  { id: "modelscope:unsloth/Qwen3-8B-GGUF", repoId: "unsloth/Qwen3-8B-GGUF", name: "Qwen3 8B 高质量", author: "unsloth", source: "modelscope", license: "Apache-2.0", parameterCount: 8_000_000_000, downloads: 892000, likes: 946, tags: ["GGUF", "中文", "代码"], compatible: true, updatedAt: now, description: "更强的写作、知识与代码能力，建议 12GB 以上可用内存。" },
  { id: "huggingface:bartowski/DeepSeek-R1-Distill-Qwen-7B-GGUF", repoId: "bartowski/DeepSeek-R1-Distill-Qwen-7B-GGUF", name: "DeepSeek R1 Distill 7B", author: "bartowski", source: "huggingface", license: "MIT", parameterCount: 7_000_000_000, downloads: 2280000, likes: 2100, tags: ["GGUF", "推理", "数学"], compatible: true, updatedAt: now, description: "擅长数学、逻辑和逐步推理的蒸馏模型。" }
];

const mockFiles: ModelFile[] = [
  { path: "model-Q4_K_M.gguf", name: "model-Q4_K_M.gguf", sizeBytes: 2.65 * 1024 ** 3, parts: [{ path: "model-Q4_K_M.gguf", name: "model-Q4_K_M.gguf", sizeBytes: 2.65 * 1024 ** 3 }], quantization: "Q4_K_M", isGguf: true, isMmproj: false, recommended: true, compatibility: "smooth", compatibilityReason: "预计占用约 3.4 GB，可流畅运行" },
  { path: "model-Q5_K_M.gguf", name: "model-Q5_K_M.gguf", sizeBytes: 3.15 * 1024 ** 3, parts: [{ path: "model-Q5_K_M.gguf", name: "model-Q5_K_M.gguf", sizeBytes: 3.15 * 1024 ** 3 }], quantization: "Q5_K_M", isGguf: true, isMmproj: false, recommended: false, compatibility: "smooth", compatibilityReason: "质量更好，速度略慢" },
  { path: "model-Q8_0.gguf", name: "model-Q8_0.gguf", sizeBytes: 4.35 * 1024 ** 3, parts: [{ path: "model-Q8_0.gguf", name: "model-Q8_0.gguf", sizeBytes: 4.35 * 1024 ** 3 }], quantization: "Q8_0", isGguf: true, isMmproj: false, recommended: false, compatibility: "runnable", compatibilityReason: "内存占用较高" }
];

const defaultSettings: AppSettings = {
  modelDirectory: mockHardware.modelDirectory, preferredSource: "modelscope", customMirror: "",
  downloadBypassProxy: true, downloadConcurrency: 4, onboardingComplete: localStorage.getItem("localdeploy-onboarded") === "1",
  chat: { temperature: 0.7, contextSize: 4096, maxTokens: 2048, enableThinking: false, systemPrompt: "你是一位可靠、友善的中文助手。", topP: 0.95, topK: 40, minP: 0, repeatPenalty: 1.1, contextPolicy: "auto", responseMode: "text", jsonSchema: "", grammar: "", validationRetries: 1 },
  theme: "dark", fontSize: 14, compactMode: false, minimizeToTray: false, autoUnloadMinutes: 60, runtimeDirectory: "", previousRuntimeDirectory: "", developerService: { enabled: false, port: 12345, apiToken: "", requestLogging: false }
};

let mockSettings = { ...defaultSettings };
let mockDownloads: DownloadTask[] = [];
let mockModels: InstalledModel[] = [];
let mockRuntime: RuntimeState = { status: "stopped" };
let mockConversations: Conversation[] = [];
let mockFolders: ConversationFolder[] = [];
let mockPresets: PromptPreset[] = [];
let mockAssistants: AssistantProfile[] = [];
let mockKnowledge: KnowledgeDocument[] = [];
let mockMcpServers: McpServerConfig[] = [];
let mockProjects: Project[] = [];
let mockArtifacts: ProjectArtifact[] = [];

async function call<T>(command: string, args?: Record<string, unknown>, fallback?: () => T | Promise<T>): Promise<T> {
  if (isDesktop()) return invoke<T>(command, args);
  if (!fallback) throw new Error(`浏览器演示模式不支持 ${command}`);
  await new Promise((resolve) => setTimeout(resolve, 160));
  return fallback();
}

export const api = {
  hardware: () => call("get_hardware_report", undefined, () => mockHardware),
  settings: () => call("get_settings", undefined, () => mockSettings),
  saveSettings: (settings: AppSettings) => call("save_settings", { settings }, () => {
    mockSettings = settings;
    localStorage.setItem("localdeploy-onboarded", settings.onboardingComplete ? "1" : "0");
    return settings;
  }),
  searchModels: (query: string, source: ModelSearchSource) => call<ModelSearchResult[]>("search_models", { query, source }, () =>
    mockResults.filter((item) => (source === "all" || item.source === source) && (!query || `${item.name} ${item.repoId}`.toLowerCase().includes(query.toLowerCase())))
  ),
  modelFiles: (repoId: string, source: ModelSource) => call<ModelFile[]>("get_model_files", { repoId, source }, () => mockFiles),
  startDownload: (request: DownloadRequest) => call<DownloadTask>("start_download", { request }, () => {
    const task: DownloadTask = { id: crypto.randomUUID(), repoId: request.repoId, fileName: request.file.name, destination: `${mockSettings.modelDirectory}\\${request.file.name}`, source: request.source, status: "downloading", downloadedBytes: request.file.sizeBytes * 0.38, totalBytes: request.file.sizeBytes, bytesPerSecond: 12.6 * 1024 ** 2, createdAt: now };
    mockDownloads = [task, ...mockDownloads];
    return task;
  }),
  downloads: () => call<DownloadTask[]>("list_downloads", undefined, () => mockDownloads),
  pauseDownload: (id: string) => call("pause_download", { id }, () => { mockDownloads = mockDownloads.map((t) => t.id === id ? { ...t, status: "paused", bytesPerSecond: 0 } : t); }),
  resumeDownload: (id: string) => call("resume_download", { id }, () => { mockDownloads = mockDownloads.map((t) => t.id === id ? { ...t, status: "downloading", bytesPerSecond: 12.6 * 1024 ** 2 } : t); }),
  cancelDownload: (id: string) => call("cancel_download", { id }, () => { mockDownloads = mockDownloads.map((t) => t.id === id ? { ...t, status: "cancelled", bytesPerSecond: 0 } : t); }),
  installedModels: () => call<InstalledModel[]>("list_installed_models", undefined, () => mockModels),
  updateModel: (model: InstalledModel) => call<InstalledModel>("update_installed_model", { model }, () => {
    mockModels = mockModels.map((item) => item.id === model.id ? model : item);
    return model;
  }),
  revealModel: (id: string) => call<void>("reveal_model", { id }, () => undefined),
  relocateModel: (id: string, path: string) => call<InstalledModel>("relocate_model", { id, path }, () => {
    const model = mockModels.find((item) => item.id === id)!;
    const updated = { ...model, filePath: path, valid: true };
    mockModels = mockModels.map((item) => item.id === id ? updated : item);
    return updated;
  }),
  duplicateModels: () => call<InstalledModel[][]>("find_duplicate_models", undefined, () => {
    const groups = new Map<string, InstalledModel[]>();
    for (const model of mockModels) {
      const key = `${model.fileSize}:${model.displayName.toLowerCase()}`;
      groups.set(key, [...(groups.get(key) ?? []), model]);
    }
    return [...groups.values()].filter((items) => items.length > 1);
  }),
  scanModelDirectory: (path: string) => call<InstalledModel[]>("scan_model_directory", { path }, () => []),
  moveModel: (id: string, destinationDirectory: string) => call<InstalledModel>("move_model", { id, destinationDirectory }, () => {
    const model = mockModels.find((item) => item.id === id)!;
    const updated = { ...model, filePath: `${destinationDirectory}\\${model.filePath.split(/[\\/]/).pop()}` };
    mockModels = mockModels.map((item) => item.id === id ? updated : item);
    return updated;
  }),
  benchmarkModel: (id: string) => call<ModelBenchmark>("benchmark_model", { id }, () => ({ modelId: id, loadTimeMs: 1280, promptTokensPerSecond: 185, generationTokensPerSecond: 42.6, output: "浏览器演示基准" })),
  installCudaRuntime: (cudaVersion = "12.4") => call<AppSettings>("install_cuda_runtime", { cudaVersion }, () => ({ ...mockSettings, runtimeDirectory: `D:\\LocalDeploy\\runtimes\\cuda-${cudaVersion}` })),
  importModel: (path: string) => call<InstalledModel>("import_model", { path }, () => {
    const model: InstalledModel = { id: crypto.randomUUID(), displayName: path.split(/[\\/]/).pop() ?? "本地模型", filePath: path, fileSize: 4.2 * 1024 ** 3, quantization: "Q4_K_M", source: "local", installedAt: now, valid: true, favorite: false, note: "", useCount: 0, config: { contextSize: 4096, gpuLayers: 999, threads: 0, flashAttention: true, kvCacheType: "auto", chatTemplate: "", draftTokens: 5, oversizedMode: false } };
    mockModels = [model, ...mockModels]; return model;
  }),
  deleteModel: (id: string, deleteFile = false) => call("delete_model", { id, deleteFile }, () => { mockModels = mockModels.filter((m) => m.id !== id); }),
  runtime: () => call<RuntimeState>("get_runtime_state", undefined, () => mockRuntime),
  runtimeComponents: () => call<RuntimeComponent[]>("list_runtime_components_command", undefined, () => []),
  activateRuntime: (path: string) => call<AppSettings>("activate_runtime_component", { path }, () => ({ ...mockSettings, previousRuntimeDirectory: mockSettings.runtimeDirectory, runtimeDirectory: path })),
  rollbackRuntime: () => call<AppSettings>("rollback_runtime_component", undefined, () => ({ ...mockSettings, runtimeDirectory: mockSettings.previousRuntimeDirectory, previousRuntimeDirectory: mockSettings.runtimeDirectory })),
  loadModel: (id: string, contextSize = 4096) => call<RuntimeState>("load_model", { id, contextSize }, () => {
    const model = mockModels.find((m) => m.id === id); mockRuntime = { status: "running", modelId: id, modelName: model?.displayName ?? "演示模型", port: 8080 }; return mockRuntime;
  }),
  unloadModel: () => call<RuntimeState>("unload_model", undefined, () => mockRuntime = { status: "stopped" }),
  conversations: () => call<Conversation[]>("list_conversations", undefined, () => mockConversations),
  searchConversations: (query: string, includeArchived = false) => call<Conversation[]>("search_conversations", { query, includeArchived }, () => mockConversations.filter((conversation) => (includeArchived || !conversation.archived) && `${conversation.title} ${(conversation.messages ?? []).map((message) => message.content).join(" ")}`.toLowerCase().includes(query.toLowerCase()))),
  createConversation: (title: string, modelId?: string) => call<Conversation>("create_conversation", { title, modelId }, () => {
    const conv: Conversation = { id: crypto.randomUUID(), title, modelId, createdAt: now, updatedAt: now, messages: [], pinned: false, archived: false, tags: [], knowledgeDocumentIds: [] }; mockConversations = [conv, ...mockConversations]; return conv;
  }),
  conversation: (id: string) => call<Conversation>("get_conversation", { id }, () => mockConversations.find((c) => c.id === id)!),
  deleteConversation: (id: string) => call("delete_conversation", { id }, () => { mockConversations = mockConversations.filter((c) => c.id !== id); }),
  updateConversation: (conversation: Conversation) => call<Conversation>("update_conversation", { conversation }, () => { mockConversations = mockConversations.map((item) => item.id === conversation.id ? conversation : item); return conversation; }),
  duplicateConversation: (id: string, throughMessageId?: string) => call<Conversation>("duplicate_conversation", { id, throughMessageId }, () => {
    const source = mockConversations.find((item) => item.id === id)!;
    const match = throughMessageId ? (source.messages ?? []).findIndex((message) => message.id === throughMessageId) : -1;
    const messages = match >= 0 ? (source.messages ?? []).slice(0, match + 1) : (source.messages ?? []);
    const conv: Conversation = { ...source, id: crypto.randomUUID(), title: `${source.title}（副本）`, parentId: source.id, pinned: false, createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(), messages: messages.map((message) => ({ ...message, id: crypto.randomUUID() })) };
    mockConversations = [conv, ...mockConversations]; return conv;
  }),
  addMessage: (conversationId: string, role: ChatMessage["role"], content: string, attachments: ChatAttachment[] = []) => call<ChatMessage>("add_message", { conversationId, role, content, attachments }, () => {
    const msg: ChatMessage = { id: crypto.randomUUID(), conversationId, role, content, attachments, createdAt: new Date().toISOString() };
    mockConversations = mockConversations.map((c) => c.id === conversationId ? { ...c, messages: [...(c.messages ?? []), msg], updatedAt: msg.createdAt } : c); return msg;
  }),
  createImageAttachment: (path: string) => call<ChatAttachment>("create_image_attachment", { path }, () => ({ id: crypto.randomUUID(), name: path.split(/[\\/]/).pop() ?? "图片", filePath: path, mimeType: "image/png", fileSize: 1024 })),
  updateMessage: (id: string, conversationId: string, content: string) => call<Conversation>("update_message", { id, conversationId, content }, () => {
    const conv = mockConversations.find((item) => item.id === conversationId)!;
    conv.messages = (conv.messages ?? []).map((message) => message.id === id ? { ...message, content } : message); return conv;
  }),
  setMessageStats: (id: string, stats: GenerationStats) => call("set_message_stats", { id, stats }, () => undefined),
  truncateMessages: (conversationId: string, messageId: string, includeMessage: boolean) => call<Conversation>("truncate_messages", { conversationId, messageId, includeMessage }, () => {
    const conv = mockConversations.find((item) => item.id === conversationId)!; const index = (conv.messages ?? []).findIndex((message) => message.id === messageId);
    conv.messages = index < 0 ? conv.messages : (conv.messages ?? []).slice(0, includeMessage ? index : index + 1); return conv;
  }),
  folders: () => call<ConversationFolder[]>("list_conversation_folders", undefined, () => mockFolders),
  createFolder: (name: string, parentId?: string) => call<ConversationFolder>("create_conversation_folder", { name, parentId }, () => { const folder = { id: crypto.randomUUID(), name, parentId, createdAt: new Date().toISOString() }; mockFolders.push(folder); return folder; }),
  deleteFolder: (id: string) => call("delete_conversation_folder", { id }, () => { mockFolders = mockFolders.filter((folder) => folder.id !== id); }),
  presets: () => call<PromptPreset[]>("list_prompt_presets", undefined, () => mockPresets),
  savePreset: (preset: PromptPreset) => call<PromptPreset>("save_prompt_preset", { preset }, () => { const value = { ...preset, id: preset.id || crypto.randomUUID(), createdAt: preset.createdAt || new Date().toISOString(), updatedAt: new Date().toISOString() }; mockPresets = [value, ...mockPresets.filter((item) => item.id !== value.id)]; return value; }),
  deletePreset: (id: string) => call("delete_prompt_preset", { id }, () => { mockPresets = mockPresets.filter((preset) => preset.id !== id); }),
  assistants: () => call<AssistantProfile[]>("list_assistants", undefined, () => mockAssistants),
  saveAssistant: (assistant: AssistantProfile) => call<AssistantProfile>("save_assistant", { assistant }, () => { const value = { ...assistant, id: assistant.id || crypto.randomUUID(), createdAt: assistant.createdAt || new Date().toISOString(), updatedAt: new Date().toISOString() }; mockAssistants = [value, ...mockAssistants.filter((item) => item.id !== value.id)]; return value; }),
  deleteAssistant: (id: string) => call("delete_assistant", { id }, () => { mockAssistants = mockAssistants.filter((assistant) => assistant.id !== id); }),
  knowledgeDocuments: () => call<KnowledgeDocument[]>("list_knowledge_documents", undefined, () => mockKnowledge),
  importKnowledgeDocument: (path: string, projectId?: string) => call<KnowledgeDocument>("import_knowledge_document", { path, projectId }, () => { const value: KnowledgeDocument = { id: crypto.randomUUID(), name: path.split(/[\\/]/).pop() ?? "资料.txt", filePath: path, fileType: path.split(".").pop() ?? "txt", fileSize: 1024, characterCount: 560, createdAt: new Date().toISOString(), projectId, indexStatus: "ready", chunkCount: 1, indexedAt: new Date().toISOString() }; mockKnowledge = [value, ...mockKnowledge]; return value; }),
  deleteKnowledgeDocument: (id: string) => call("delete_knowledge_document", { id }, () => { mockKnowledge = mockKnowledge.filter((document) => document.id !== id); }),
  retrieveKnowledge: (documentIds: string[], query: string, maxCharacters = 6000) => call<KnowledgeSnippet[]>("retrieve_knowledge", { documentIds, query, maxCharacters }, () => mockKnowledge.filter((item) => documentIds.includes(item.id)).map((item) => ({ chunkId: `${item.id}-0`, documentId: item.id, documentName: item.name, sourcePath: item.filePath, chunkIndex: 0, content: `浏览器演示资料：${query}`, score: 1, vectorScore: .8, bm25Score: .9 }))),
  rebuildKnowledgeIndex: (id: string) => call<KnowledgeDocument>("rebuild_knowledge_index", { id }, () => mockKnowledge.find((item) => item.id === id)!),
  projects: () => call<Project[]>("list_projects", undefined, () => mockProjects),
  saveProject: (project: Project) => call<Project>("save_project", { project }, () => { const value={...project,id:project.id||crypto.randomUUID(),createdAt:project.createdAt||new Date().toISOString(),updatedAt:new Date().toISOString()};mockProjects=[value,...mockProjects.filter(item=>item.id!==value.id)];return value; }),
  deleteProject: (id: string) => call<void>("delete_project", { id }, () => { mockProjects=mockProjects.filter(item=>item.id!==id); }),
  projectArtifacts: (projectId: string) => call<ProjectArtifact[]>("list_project_artifacts", { projectId }, () => mockArtifacts.filter(item=>item.projectId===projectId)),
  saveProjectArtifact: (artifact: ProjectArtifact) => call<ProjectArtifact>("save_project_artifact", { artifact }, () => { const existing=mockArtifacts.find(item=>item.id===artifact.id);const value={...artifact,id:artifact.id||crypto.randomUUID(),name:artifact.relativePath.split(/[\\/]/).pop()||"file",version:existing?existing.version+1:1,createdAt:artifact.createdAt||new Date().toISOString(),updatedAt:new Date().toISOString()};mockArtifacts=[value,...mockArtifacts.filter(item=>item.id!==value.id)];return value; }),
  deleteProjectArtifact: (id: string) => call<void>("delete_project_artifact", { id }, () => { mockArtifacts=mockArtifacts.filter(item=>item.id!==id); }),
  exportProjectArtifacts: (projectId: string, directory: string) => call<string>("export_project_artifacts", { projectId, directory }, () => directory),
  mcpServers: () => call<McpServerConfig[]>("list_mcp_servers", undefined, () => mockMcpServers),
  saveMcpServer: (server: McpServerConfig) => call<McpServerConfig>("save_mcp_server", { server }, () => { const value = { ...server, id: server.id || crypto.randomUUID(), createdAt: server.createdAt || new Date().toISOString(), updatedAt: new Date().toISOString() }; mockMcpServers = [value, ...mockMcpServers.filter((item) => item.id !== value.id)]; return value; }),
  deleteMcpServer: (id: string) => call("delete_mcp_server", { id }, () => { mockMcpServers = mockMcpServers.filter((server) => server.id !== id); }),
  discoverMcpTools: (id: string) => call<McpToolInfo[]>("discover_mcp_tools", { id }, () => [{ serverId: id, name: "demo_tool", description: "浏览器演示工具", inputSchema: { type: "object" } }]),
  callMcpTool: (id: string, name: string, args: unknown, projectId?: string, conversationId?: string) => call<unknown>("call_mcp_tool", { id, name, arguments: args, projectId, conversationId }, () => ({ content: [{ type: "text", text: `已调用 ${name}` }] })),
  exportPreset: (id: string, path: string) => call("export_prompt_preset", { id, path }, () => undefined),
  importPreset: (path: string) => call<PromptPreset>("import_prompt_preset", { path }, () => { throw new Error("浏览器演示模式不支持导入文件"); }),
  exportConversation: (id: string, format: "markdown" | "json", path: string) => call("export_conversation", { id, format, path }, () => undefined),
  saveGeneratedFile: (path: string, content: string) => call<void>("save_generated_file", { path, content }, () => undefined),
  revealPath: (path: string) => call<void>("reveal_path", { path }, () => undefined),
  webSearch: (query: string, maxResults = 5) => call<WebSearchResult[]>("web_search", { query, maxResults }, () => [
    { title: `浏览器演示结果：${query}`, url: "https://example.com", snippet: "桌面版会查询实时网页；浏览器演示模式不访问互联网。", engine: "bing" },
  ]),
  countTokens: (content: string) => call<number>("count_tokens", { content }, () => Math.ceil(content.length / 2.4)),
  summarizeContext: (transcript: string, previousSummary?: string) => call<string>("summarize_context", { transcript, previousSummary }, () => `${previousSummary ? `${previousSummary}\n` : ""}${transcript.slice(0, 800)}`),
  exportDiagnostics: (path: string) => call<void>("export_diagnostics", { path }, () => undefined),
  backupUserData: (path: string) => call<void>("backup_user_data", { path }, () => undefined),
  stageUserDataRestore: (path: string) => call<void>("stage_user_data_restore", { path }, () => undefined),
  checkForUpdate: () => call<AppUpdateInfo>("check_for_app_update", undefined, () => ({ available: false, currentVersion: "0.9.0", latestVersion: "0.9.0", notes: "浏览器演示模式不检查更新。" })),
  installUpdate: (update: AppUpdateInfo) => call<void>("install_app_update", { update }, () => undefined),
  rollbackUpdate: (path: string) => call<void>("rollback_app_update", { path }, () => Promise.reject(new Error("浏览器演示模式不支持版本回退"))),
  openExternalUrl: (url: string) => call<void>("open_external_url", { url }, () => { window.open(url, "_blank", "noopener,noreferrer"); }),
  importConversation: (path: string) => call<Conversation>("import_conversation", { path }, () => { throw new Error("浏览器演示模式不支持导入文件"); }),
  chat: (requestId: string, conversationId: string, messages: ChatMessage[], params: ChatParams, tools?: ChatToolDefinition[]) => call<void>("chat_stream", { requestId, conversationId, messages, params, tools }, () => { throw new Error("请在桌面应用中加载真实模型后聊天"); }),
  stopChat: (requestId: string) => call("stop_generation", { requestId }, () => undefined),
  speakText: (text: string) => call<void>("speak_text", { text }, () => undefined),
  stopSpeech: () => call<void>("stop_speech", undefined, () => undefined),
  dictateOnce: (language = "zh-CN") => call<string>("dictate_once", { language }, () => Promise.reject(new Error("浏览器演示模式不支持离线听写"))),
  onDownload: (handler: (task: DownloadTask) => void): Promise<UnlistenFn> => isDesktop() ? listen<DownloadTask>("download-progress", (e) => handler(e.payload)) : Promise.resolve(() => undefined),
  onRuntimeProgress: (handler: (payload: { modelId: string; stage: string; percent: number; message: string }) => void): Promise<UnlistenFn> => isDesktop() ? listen("runtime-progress", (event) => handler(event.payload as never)) : Promise.resolve(() => undefined),
  onRuntimeInstallProgress: (handler: (payload: { percent: number; message: string }) => void): Promise<UnlistenFn> => isDesktop() ? listen("runtime-install-progress", (event) => handler(event.payload as never)) : Promise.resolve(() => undefined),
  onChatProgress: (handler: (payload: { requestId: string; stage: "processing_prompt" | "generating"; elapsedMs: number }) => void): Promise<UnlistenFn> => isDesktop() ? listen("chat-progress", (event) => handler(event.payload as never)) : Promise.resolve(() => undefined),
  onChatToken: (handler: (payload: { requestId: string; token: string; reasoning?: string; done: boolean; error?: string; finishReason?: string; stats?: GenerationStats; toolCalls?: ChatToolCall[] }) => void): Promise<UnlistenFn> => isDesktop() ? listen("chat-token", (e) => handler(e.payload as never)) : Promise.resolve(() => undefined)
};

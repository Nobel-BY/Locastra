export type ModelSource = "modelscope" | "huggingface";
export type ModelSearchSource = ModelSource | "all";
export type CompatibilityRating = "smooth" | "runnable" | "not_recommended" | "unknown";
export type DownloadStatus = "queued" | "downloading" | "paused" | "verifying" | "completed" | "failed" | "cancelled";

export interface GpuInfo {
  name: string;
  vendor: string;
  dedicatedMemoryBytes: number;
  driverVersion?: string;
}

export interface HardwareReport {
  os: string;
  arch: string;
  cpuName: string;
  physicalCores: number;
  logicalCores: number;
  instructionSets: string[];
  totalMemoryBytes: number;
  availableMemoryBytes: number;
  gpus: GpuInfo[];
  freeDiskBytes: number;
  modelDirectory: string;
  acceleration: "vulkan" | "cpu";
}

export interface ModelSearchResult {
  id: string;
  repoId: string;
  name: string;
  author: string;
  source: ModelSource;
  description?: string;
  license?: string;
  parameterCount?: number;
  downloads?: number;
  likes?: number;
  tags: string[];
  compatible: boolean;
  incompatibleReason?: string;
  updatedAt?: string;
}

export interface ModelFile {
  path: string;
  name: string;
  sizeBytes: number;
  sha256?: string;
  quantization?: string;
  isGguf: boolean;
  isMmproj: boolean;
  shardGroup?: string;
  parts: ModelFilePart[];
  recommended: boolean;
  compatibility: CompatibilityRating;
  compatibilityReason: string;
}

export interface ModelFilePart {
  path: string;
  name: string;
  sizeBytes: number;
  sha256?: string;
}

export interface DownloadCandidate {
  source: ModelSource | "custom";
  url: string;
}

export interface DownloadRequest {
  repoId: string;
  file: ModelFile;
  source: ModelSource;
  destinationDirectory?: string;
}

export interface DownloadTask {
  id: string;
  repoId: string;
  fileName: string;
  destination: string;
  source: string;
  status: DownloadStatus;
  downloadedBytes: number;
  totalBytes: number;
  bytesPerSecond: number;
  error?: string;
  createdAt: string;
  parts?: ModelFilePart[];
}

export interface InstalledModel {
  id: string;
  displayName: string;
  repoId?: string;
  filePath: string;
  fileSize: number;
  quantization?: string;
  source: string;
  installedAt: string;
  valid: boolean;
  favorite: boolean;
  note: string;
  lastUsedAt?: string;
  useCount: number;
  config: ModelRuntimeConfig;
}

export interface ModelRuntimeConfig {
  contextSize: number;
  gpuLayers: number;
  threads: number;
  flashAttention: boolean;
  kvCacheType: "auto" | "f16" | "q8_0" | "q4_0" | string;
  chatTemplate: string;
  draftModelId?: string;
  draftTokens: number;
  mmprojPath?: string;
  oversizedMode: boolean;
}

export interface ModelBenchmark {
  modelId: string;
  loadTimeMs: number;
  promptTokensPerSecond?: number;
  generationTokensPerSecond?: number;
  output: string;
}

export interface RuntimeState {
  status: "stopped" | "starting" | "running" | "error";
  modelId?: string;
  modelName?: string;
  port?: number;
  error?: string;
}

export interface RuntimeComponent {
  id: string;
  name: string;
  version: string;
  backend: "cuda" | "vulkan" | "cpu" | "custom";
  path: string;
  source: "bundled" | "managed" | "custom";
  active: boolean;
  valid: boolean;
  capabilities: string[];
  diagnostic?: string;
}

export interface AppUpdateInfo {
  available: boolean;
  currentVersion: string;
  latestVersion: string;
  notes: string;
  publishedAt?: string;
  downloadUrl?: string;
  sha256?: string;
}

export interface ChatMessage {
  id: string;
  conversationId: string;
  role: "system" | "user" | "assistant";
  content: string;
  createdAt: string;
  stats?: GenerationStats;
  attachments?: ChatAttachment[];
}

export interface ChatAttachment {
  id: string;
  name: string;
  filePath: string;
  mimeType: string;
  fileSize: number;
}

export interface GenerationStats {
  inputTokens?: number;
  outputTokens?: number;
  promptTokensPerSecond?: number;
  tokensPerSecond?: number;
  timeToFirstTokenMs?: number;
  totalTimeMs?: number;
}

export interface WebSearchResult {
  title: string;
  url: string;
  snippet: string;
  engine: "bing" | "duckduckgo" | string;
}

export interface Conversation {
  id: string;
  title: string;
  modelId?: string;
  createdAt: string;
  updatedAt: string;
  messages?: ChatMessage[];
  folderId?: string;
  pinned: boolean;
  archived: boolean;
  tags: string[];
  parentId?: string;
  params?: ChatParams;
  knowledgeDocumentIds: string[];
  contextState?: ContextCompressionState;
  projectId?: string;
}

export type ContextPolicy = "auto" | "warn" | "off";

export interface ContextCompressionState {
  summary: string;
  summarizedMessageIds: string[];
  originalTokenCount: number;
  summaryTokenCount: number;
  updatedAt: string;
}

export interface AssistantProfile {
  id: string;
  name: string;
  description: string;
  icon: string;
  modelId?: string;
  params: ChatParams;
  createdAt: string;
  updatedAt: string;
}

export interface KnowledgeDocument {
  id: string;
  name: string;
  filePath: string;
  fileType: string;
  fileSize: number;
  characterCount: number;
  createdAt: string;
  projectId?: string;
  indexStatus: "pending" | "indexing" | "ready" | "failed";
  indexError?: string;
  chunkCount: number;
  indexedAt?: string;
}

export interface KnowledgeSnippet {
  chunkId: string;
  documentId: string;
  documentName: string;
  sourcePath: string;
  chunkIndex: number;
  page?: number;
  content: string;
  score: number;
  vectorScore: number;
  bm25Score: number;
}

export interface Project {
  id: string;
  name: string;
  description: string;
  instructions: string;
  modelId?: string;
  params: ChatParams;
  knowledgeDocumentIds: string[];
  mcpServerIds: string[];
  toolPolicy: "ask" | "conversation" | "readonly";
  createdAt: string;
  updatedAt: string;
}

export interface ProjectArtifact {
  id: string;
  projectId: string;
  name: string;
  relativePath: string;
  language: string;
  content: string;
  version: number;
  createdAt: string;
  updatedAt: string;
}

export interface McpServerConfig {
  id: string;
  name: string;
  command: string;
  args: string[];
  workingDirectory?: string;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface McpToolInfo {
  serverId: string;
  name: string;
  description: string;
  inputSchema: unknown;
}

export interface ChatToolDefinition {
  type: "function";
  function: {
    name: string;
    description: string;
    parameters: unknown;
  };
}

export interface ChatToolCall {
  id: string;
  name: string;
  arguments: string;
}

export interface ConversationFolder {
  id: string;
  name: string;
  parentId?: string;
  createdAt: string;
}

export interface PromptPreset {
  id: string;
  name: string;
  description: string;
  params: ChatParams;
  createdAt: string;
  updatedAt: string;
}

export interface ChatParams {
  temperature: number;
  contextSize: number;
  maxTokens: number;
  enableThinking: boolean;
  systemPrompt: string;
  topP: number;
  topK: number;
  minP: number;
  repeatPenalty: number;
  contextPolicy: ContextPolicy;
  responseMode: "text" | "json" | "schema" | "grammar";
  jsonSchema: string;
  grammar: string;
  validationRetries: number;
}

export interface AppSettings {
  modelDirectory: string;
  preferredSource: ModelSource;
  customMirror: string;
  downloadBypassProxy: boolean;
  downloadConcurrency: number;
  onboardingComplete: boolean;
  chat: ChatParams;
  theme: "light" | "dark" | "system";
  fontSize: number;
  compactMode: boolean;
  minimizeToTray: boolean;
  autoUnloadMinutes: number;
  runtimeDirectory: string;
  previousRuntimeDirectory: string;
  developerService: { enabled: boolean; port: number; apiToken: string; requestLogging: boolean };
}

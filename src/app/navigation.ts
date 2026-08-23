export type Page =
  | "chat"
  | "models"
  | "downloads"
  | "library"
  | "workspace"
  | "compare"
  | "mcp"
  | "settings";

export const pageMeta: Record<Page, { label: string; code: string }> = {
  chat: { label: "对话", code: "SESSION" },
  models: { label: "发现模型", code: "MODEL HUB" },
  downloads: { label: "下载管理", code: "TRANSFER" },
  library: { label: "我的模型", code: "MODEL VAULT" },
  workspace: { label: "项目与资料", code: "WORKSPACE" },
  compare: { label: "模型对比", code: "BENCHMARK" },
  mcp: { label: "MCP 工具", code: "TOOLS" },
  settings: { label: "设置", code: "SYSTEM" },
};

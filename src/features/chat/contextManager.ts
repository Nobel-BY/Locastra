import type {
  ChatMessage,
  ContextCompressionState,
  ContextPolicy,
} from "../../types";

export const CONTEXT_COMPRESSION_MARKER = "[Locastra 自动上下文摘要]";

export interface ContextPlan {
  action: "none" | "warn" | "compress";
  threshold: number;
  reserve: number;
  messagesToSummarize: ChatMessage[];
  messagesToKeep: ChatMessage[];
}

export function outputReserve(contextSize: number, maxTokens: number) {
  const requested = maxTokens < 0 ? Math.floor(contextSize * 0.2) : maxTokens;
  return Math.min(Math.max(requested, 1024), Math.floor(contextSize * 0.35));
}

export function planContext(
  messages: ChatMessage[],
  tokenCount: number,
  contextSize: number,
  maxTokens: number,
  policy: ContextPolicy,
  previous?: ContextCompressionState,
): ContextPlan {
  const reserve = outputReserve(contextSize, maxTokens) + 256;
  const threshold = Math.max(1024, Math.floor((contextSize - reserve) * 0.88));
  const alreadySummarized = new Set(previous?.summarizedMessageIds ?? []);
  const candidates = messages.filter(
    (message) => message.role !== "system" && !alreadySummarized.has(message.id),
  );
  const keepCount = Math.min(8, Math.max(4, Math.ceil(candidates.length * 0.3)));
  const messagesToKeep = candidates.slice(-keepCount);
  const keepIds = new Set(messagesToKeep.map((message) => message.id));
  const messagesToSummarize = candidates.filter(
    (message) => !keepIds.has(message.id),
  );

  if (tokenCount < threshold || messagesToSummarize.length < 2 || policy === "off") {
    return { action: "none", threshold, reserve, messagesToSummarize: [], messagesToKeep: candidates };
  }
  return {
    action: policy === "warn" ? "warn" : "compress",
    threshold,
    reserve,
    messagesToSummarize,
    messagesToKeep,
  };
}

export function transcriptForSummary(messages: ChatMessage[]) {
  return messages
    .map((message) => `${message.role === "user" ? "用户" : "助手"}：${message.content}`)
    .join("\n\n");
}

export function requestMessagesWithSummary(
  conversationId: string,
  messages: ChatMessage[],
  state?: ContextCompressionState,
) {
  if (!state?.summary.trim()) return messages;
  const summarized = new Set(state.summarizedMessageIds);
  const remaining = messages.filter(
    (message) => message.role === "system" || !summarized.has(message.id),
  );
  return [
    {
      id: `context-summary-${state.updatedAt}`,
      conversationId,
      role: "system" as const,
      content: `${CONTEXT_COMPRESSION_MARKER}\n以下摘要由本机模型从较早对话生成。把它当作对话背景；若与最近原文冲突，以最近原文为准。\n\n${state.summary}`,
      createdAt: state.updatedAt,
    },
    ...remaining,
  ];
}


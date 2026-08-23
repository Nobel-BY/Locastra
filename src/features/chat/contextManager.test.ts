import { describe, expect, it } from "vitest";
import type { ChatMessage, ContextCompressionState } from "../../types";
import { outputReserve, planContext, requestMessagesWithSummary } from "./contextManager";

function messages(count: number): ChatMessage[] {
  return Array.from({ length: count }, (_, index) => ({
    id: `m${index}`,
    conversationId: "c1",
    role: index % 2 ? "assistant" : "user",
    content: `message ${index}`,
    createdAt: new Date(index * 1000).toISOString(),
  }));
}

describe("automatic context management", () => {
  it("reserves output capacity and compresses only older turns", () => {
    const items = messages(12);
    const plan = planContext(items, 7600, 8192, 1024, "auto");
    expect(plan.action).toBe("compress");
    expect(plan.messagesToSummarize.length).toBeGreaterThan(0);
    expect(plan.messagesToKeep.at(-1)?.id).toBe("m11");
    expect(outputReserve(8192, -1)).toBeGreaterThanOrEqual(1024);
  });

  it("supports warning and disabled policies", () => {
    const items = messages(12);
    expect(planContext(items, 7600, 8192, 1024, "warn").action).toBe("warn");
    expect(planContext(items, 7600, 8192, 1024, "off").action).toBe("none");
  });

  it("replaces summarized messages with an editable summary", () => {
    const items = messages(6);
    const state: ContextCompressionState = {
      summary: "用户正在设计一个桌面应用。",
      summarizedMessageIds: ["m0", "m1", "m2"],
      originalTokenCount: 3000,
      summaryTokenCount: 28,
      updatedAt: "2026-08-17T00:00:00Z",
    };
    const request = requestMessagesWithSummary("c1", items, state);
    expect(request[0].role).toBe("system");
    expect(request.some((message) => message.id === "m0")).toBe(false);
    expect(request.at(-1)?.id).toBe("m5");
  });
});

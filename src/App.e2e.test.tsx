// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it } from "vitest";
import App from "./App";

function button(label: string, exact = false) {
  return [...document.querySelectorAll("button")].find((element) =>
    exact
      ? element.textContent?.trim() === label
      : element.textContent?.includes(label),
  ) as HTMLButtonElement | undefined;
}

async function settle(ms = 240) {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, ms));
  });
}

describe("Locastra desktop shell regression", () => {
  let root: Root;
  const testEnvironment = globalThis as typeof globalThis & {
    IS_REACT_ACT_ENVIRONMENT: boolean;
  };

  beforeAll(() => {
    testEnvironment.IS_REACT_ACT_ENVIRONMENT = true;
    Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
      configurable: true,
      value: () => undefined,
    });
  });

  afterAll(() => {
    testEnvironment.IS_REACT_ACT_ENVIRONMENT = false;
  });

  beforeEach(async () => {
    window.localStorage.clear();
    document.body.innerHTML = '<div id="root"></div>';
    root = createRoot(document.getElementById("root")!);
    await act(async () => root.render(<App />));
    await settle();
  });

  afterEach(async () => {
    await act(async () => root.unmount());
  });

  it("can leave chat and visit every primary page without a white screen", async () => {
    for (const label of [
      "对话",
      "发现模型",
      "下载管理",
      "我的模型",
      "项目与资料",
      "模型对比",
      "MCP 工具",
      "设置",
    ]) {
      const target = button(label, true);
      expect(target, `missing navigation button ${label}`).toBeTruthy();
      await act(async () => target!.click());
      await settle(20);
      expect(document.querySelector(".app-error")).toBeNull();
      expect(document.querySelector(".main-panel")).not.toBeNull();
    }
  });

  it("releases automatic following when the user scrolls upward", async () => {
    const chatButton = button("对话", true);
    expect(chatButton).toBeTruthy();
    await act(async () => chatButton!.click());
    await settle(20);
    const area = document.querySelector(".messages-area") as HTMLDivElement;
    expect(area).not.toBeNull();
    Object.defineProperties(area, {
      scrollHeight: { configurable: true, value: 1800 },
      clientHeight: { configurable: true, value: 600 },
      scrollTop: { configurable: true, writable: true, value: 1100 },
    });
    await act(async () => {
      area.dispatchEvent(new WheelEvent("wheel", { bubbles: true, deltaY: -120 }));
    });
    await settle(10);
    expect(button("返回最新")).toBeTruthy();
  });

  it("opens the global command center and navigates without using the sidebar", async () => {
    await act(async () => {
      window.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, ctrlKey: true, key: "k" }));
    });
    await settle(30);
    const palette = document.querySelector(".command-palette");
    expect(palette).not.toBeNull();
    expect((palette!.querySelector("input") as HTMLInputElement).placeholder).toContain("模型");
    const settingsCommand = [...palette!.querySelectorAll("button")].find(
      (element) => element.querySelector("strong")?.textContent === "设置",
    ) as HTMLButtonElement | undefined;
    expect(settingsCommand).toBeTruthy();
    await act(async () => settingsCommand!.click());
    await settle(30);
    expect(document.querySelector(".command-palette")).toBeNull();
    expect(document.querySelector('.settings[aria-current="page"]')).not.toBeNull();
  });

  it("offers storage-aware bulk model management", async () => {
    const library = button("我的模型", true);
    expect(library).toBeTruthy();
    await act(async () => library!.click());
    await settle(30);
    if (!document.querySelector(".model-storage-summary")) {
      const importModel = button("导入模型", true);
      expect(importModel).toBeTruthy();
      await act(async () => importModel!.click());
      await settle(380);
    }
    expect(document.querySelector(".model-storage-summary")).not.toBeNull();
    const manage = button("批量管理", true);
    expect(manage).toBeTruthy();
    await act(async () => manage!.click());
    await settle(20);
    expect(document.querySelector(".model-bulk-bar")).not.toBeNull();
    expect(document.querySelector('.model-select-check input[type="checkbox"]')).not.toBeNull();
  });

  it("shows installed models directly in the chat model selector", async () => {
    if (!document.querySelector(".model-storage-summary")) {
      const library = button("我的模型", true);
      await act(async () => library!.click());
      await settle(30);
      if (button("导入模型", true)) {
        await act(async () => button("导入模型", true)!.click());
        await settle(380);
      }
    }
    const chat = button("对话", true);
    expect(chat).toBeTruthy();
    await act(async () => chat!.click());
    await settle(30);
    const selector = document.querySelector(".chat-model-selector") as HTMLDetailsElement;
    expect(selector).not.toBeNull();
    await act(async () => selector.querySelector("summary")!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    await settle(20);
    expect(selector.open).toBe(true);
    expect(selector.querySelectorAll(".chat-model-menu > button").length).toBeGreaterThan(0);
  });

  it("shows the 0.9.0 release and privacy status in settings", async () => {
    const settings = button("设置", true);
    expect(settings).toBeTruthy();
    await act(async () => settings!.click());
    await settle(30);
    const status = document.querySelector(".release-status-card");
    expect(status).not.toBeNull();
    expect(status!.textContent).toContain("Locastra 0.9.0");
    expect(status!.textContent).toContain("聊天仅存本机");
  });
});

import { describe, expect, it } from "vitest";
import { compatibilityMeta, formatBytes, formatNumber } from "./format";

describe("format helpers", () => {
  it("formats model sizes for Chinese UI", () => {
    expect(formatBytes(4 * 1024 ** 3)).toBe("4.0 GB");
    expect(formatBytes(0)).toBe("未知");
  });

  it("formats popularity compactly", () => {
    expect(formatNumber(12_300)).not.toBe("—");
    expect(formatNumber()).toBe("—");
  });

  it("maps compatibility to a stable presentation", () => {
    expect(compatibilityMeta("smooth")).toEqual({ label: "流畅运行", className: "good" });
    expect(compatibilityMeta("not_recommended").className).toBe("bad");
  });
});

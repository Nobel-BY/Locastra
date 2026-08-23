import type { CompatibilityRating } from "../types";

export function formatBytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return "未知";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const power = Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1);
  const amount = value / 1024 ** power;
  return `${amount >= 10 || power === 0 ? amount.toFixed(0) : amount.toFixed(1)} ${units[power]}`;
}

export function formatNumber(value?: number): string {
  if (!value) return "—";
  return new Intl.NumberFormat("zh-CN", { notation: "compact", maximumFractionDigits: 1 }).format(value);
}

export function compatibilityMeta(rating: CompatibilityRating) {
  switch (rating) {
    case "smooth": return { label: "流畅运行", className: "good" };
    case "runnable": return { label: "可以运行", className: "warn" };
    case "not_recommended": return { label: "不建议", className: "bad" };
    default: return { label: "待评估", className: "neutral" };
  }
}

export function timeAgo(value?: string): string {
  if (!value) return "";
  const delta = Date.now() - new Date(value).getTime();
  const day = Math.floor(delta / 86_400_000);
  if (day <= 0) return "今天更新";
  if (day < 30) return `${day} 天前更新`;
  return new Date(value).toLocaleDateString("zh-CN");
}

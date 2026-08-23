import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";
import "./model-hub.css";
import "./brand.css";
import "./light.css";
import "./polish.css";

// Restore the last persisted theme before React mounts to avoid a color flash.
document.documentElement.dataset.theme =
  window.localStorage.getItem("locastra-theme") === "light" ? "light" : "dark";

function installDesktopGuards() {
  if (!("__TAURI_INTERNALS__" in window)) return;

  const blockContextMenu = (event: MouseEvent) => event.preventDefault();
  const blockBrowserShortcuts = (event: KeyboardEvent) => {
    const key = event.key.toLowerCase();
    const browserCommand =
      key === "f5" ||
      key === "f12" ||
      (event.ctrlKey && ["r", "p", "s", "u"].includes(key)) ||
      (event.ctrlKey && event.shiftKey && ["i", "j", "c"].includes(key));
    if (!browserCommand) return;
    event.preventDefault();
    event.stopImmediatePropagation();
  };

  window.addEventListener("contextmenu", blockContextMenu, { capture: true });
  window.addEventListener("keydown", blockBrowserShortcuts, { capture: true });
}

installDesktopGuards();

class AppErrorBoundary extends React.Component<React.PropsWithChildren, { error: Error | null }> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error) {
    console.error("Locastra 界面异常", error);
  }

  render() {
    if (this.state.error) {
      return <div className="app-error"><h1>界面遇到问题</h1><p>{this.state.error.message}</p><button onClick={() => window.location.reload()}>重新加载应用</button></div>;
    }
    return this.props.children;
  }
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode><AppErrorBoundary><App /></AppErrorBoundary></React.StrictMode>
);

import { useEffect, useState } from "react";
import { api } from "../../lib/api";
import type { RuntimeState } from "../../types";

/**
 * Owns the engine heartbeat independently from the application shell. Keeping
 * runtime state here prevents a page transition from creating extra polling
 * loops or losing the last known engine state.
 */
export function useRuntimeMonitor(intervalMs = 5000) {
  const [runtime, setRuntime] = useState<RuntimeState>({ status: "stopped" });

  useEffect(() => {
    let disposed = false;
    const refresh = () => {
      api
        .runtime()
        .then((next) => {
          if (!disposed) setRuntime(next);
        })
        .catch(() => undefined);
    };
    const timer = window.setInterval(refresh, intervalMs);
    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, [intervalMs]);

  return [runtime, setRuntime] as const;
}

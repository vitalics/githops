import { useEffect, useRef, useState } from "react";
import type { AppState, WsResult } from "../types";

type Pending = {
  resolve: (r: WsResult) => void;
  reject: (e: Error) => void;
  timeout: ReturnType<typeof setTimeout>;
};

export function useGithopsWs() {
  const [appState, setAppState] = useState<AppState | null>(null);
  const [connected, setConnected] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const pendingRef = useRef<Map<number, Pending>>(new Map());
  const nextIdRef = useRef(1);

  function send(method: string, params: Record<string, unknown> = {}): Promise<WsResult> {
    return new Promise((resolve, reject) => {
      const ws = wsRef.current;
      if (!ws || ws.readyState !== WebSocket.OPEN) {
        return reject(new Error("WebSocket not connected"));
      }
      const id = nextIdRef.current++;
      const timeout = setTimeout(() => {
        pendingRef.current.delete(id);
        reject(new Error("Request timeout"));
      }, 15_000);
      pendingRef.current.set(id, { resolve, reject, timeout });
      ws.send(JSON.stringify({ id, method, params }));
    });
  }

  useEffect(() => {
    let cancelled = false;

    function connect() {
      if (cancelled) return;
      const proto = location.protocol === "https:" ? "wss:" : "ws:";
      const ws = new WebSocket(`${proto}//${location.host}/ws`);
      wsRef.current = ws;

      ws.onmessage = (e) => {
        try {
          const msg = JSON.parse(e.data as string) as {
            id?: number;
            method?: string;
            result?: WsResult;
            error?: { message: string };
            params?: AppState;
          };
          if (msg.id !== undefined) {
            const p = pendingRef.current.get(msg.id);
            if (p) {
              clearTimeout(p.timeout);
              pendingRef.current.delete(msg.id);
              if (msg.error) p.reject(new Error(msg.error.message));
              else p.resolve(msg.result ?? {});
            }
          } else if (msg.method === "state" && msg.params) {
            setAppState(msg.params);
          }
        } catch (_) {}
      };

      ws.onopen = () => setConnected(true);

      ws.onclose = () => {
        wsRef.current = null;
        setConnected(false);
        pendingRef.current.forEach(({ reject: rej, timeout }) => {
          clearTimeout(timeout);
          rej(new Error("WebSocket closed"));
        });
        pendingRef.current.clear();
        if (!cancelled) setTimeout(connect, 3_000);
      };
    }

    connect();
    return () => {
      cancelled = true;
      wsRef.current?.close();
    };
  }, []);

  return { appState, connected, send };
}

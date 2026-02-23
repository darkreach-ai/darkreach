"use client";

import { useEffect, useRef, useState, useCallback } from "react";
import { WS_URL } from "@/lib/api";
import { formLabel } from "@/lib/prime-feed";

interface Toast {
  id: number;
  form: string;
  expression: string;
  digits: number;
  dismissing: boolean;
}

interface WsNotification {
  kind: string;
  title: string;
  details: string[];
  id: number;
}

const TOAST_DURATION = 8000;
const MAX_BACKOFF = 30000;

export function DiscoveryToast() {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const wsRef = useRef<WebSocket | null>(null);
  const backoffRef = useRef(1000);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const seenIds = useRef(new Set<number>());
  const initialized = useRef(false);
  const toastCounter = useRef(0);

  const dismiss = useCallback((id: number) => {
    setToasts((prev) =>
      prev.map((t) => (t.id === id ? { ...t, dismissing: true } : t))
    );
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 400);
  }, []);

  const addToast = useCallback(
    (notif: WsNotification) => {
      // Parse expression and digits from notification details
      const expression = notif.details[0] || notif.title;
      const digitsMatch = notif.details.find((d) => /\d+\s*digit/.test(d));
      const digits = digitsMatch
        ? parseInt(digitsMatch.replace(/\D/g, ""), 10)
        : 0;
      const formMatch = notif.title.match(/^\[(\w+)]/);
      const form = formMatch ? formMatch[1].toLowerCase() : "kbn";

      const id = ++toastCounter.current;
      setToasts((prev) => [...prev.slice(-2), { id, form, expression, digits, dismissing: false }]);

      setTimeout(() => dismiss(id), TOAST_DURATION);
    },
    [dismiss]
  );

  const connect = useCallback(() => {
    if (typeof window === "undefined") return;
    if (document.visibilityState === "hidden") return;

    try {
      const ws = new WebSocket(WS_URL);
      wsRef.current = ws;

      ws.onopen = () => {
        backoffRef.current = 1000;
      };

      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data);

          // Handle notification messages
          if (msg.type === "notification" || msg.notification) {
            const notif: WsNotification = msg.notification || msg;

            // First batch: mark as seen without toasting
            if (!initialized.current) {
              seenIds.current.add(notif.id);
              initialized.current = true;
              return;
            }

            if (notif.kind === "prime" && !seenIds.current.has(notif.id)) {
              seenIds.current.add(notif.id);
              // Prevent unbounded growth
              if (seenIds.current.size > 200) {
                const arr = Array.from(seenIds.current);
                seenIds.current = new Set(arr.slice(-100));
              }
              addToast(notif);
            }
          }

          // Handle notifications array in status messages
          if (Array.isArray(msg.notifications)) {
            if (!initialized.current) {
              for (const n of msg.notifications) {
                seenIds.current.add(n.id);
              }
              initialized.current = true;
              return;
            }

            for (const notif of msg.notifications) {
              if (notif.kind === "prime" && !seenIds.current.has(notif.id)) {
                seenIds.current.add(notif.id);
                if (seenIds.current.size > 200) {
                  const arr = Array.from(seenIds.current);
                  seenIds.current = new Set(arr.slice(-100));
                }
                addToast(notif);
              }
            }
          }
        } catch {
          // Ignore unparseable messages
        }
      };

      ws.onclose = () => {
        wsRef.current = null;
        // Reconnect with exponential backoff
        const delay = backoffRef.current;
        backoffRef.current = Math.min(backoffRef.current * 2, MAX_BACKOFF);
        reconnectTimer.current = setTimeout(connect, delay);
      };

      ws.onerror = () => {
        ws.close();
      };
    } catch {
      // WebSocket constructor can throw if URL is invalid
    }
  }, [addToast]);

  useEffect(() => {
    connect();

    const handleVisibility = () => {
      if (document.visibilityState === "visible" && !wsRef.current) {
        connect();
      } else if (document.visibilityState === "hidden") {
        wsRef.current?.close();
        wsRef.current = null;
        if (reconnectTimer.current) {
          clearTimeout(reconnectTimer.current);
        }
      }
    };

    document.addEventListener("visibilitychange", handleVisibility);

    return () => {
      document.removeEventListener("visibilitychange", handleVisibility);
      if (reconnectTimer.current) clearTimeout(reconnectTimer.current);
      wsRef.current?.close();
      wsRef.current = null;
    };
  }, [connect]);

  if (toasts.length === 0) return null;

  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 pointer-events-none">
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={`discovery-toast pointer-events-auto ${toast.dismissing ? "discovery-toast-out" : ""}`}
          role="status"
          aria-live="polite"
        >
          <button
            onClick={() => dismiss(toast.id)}
            className="absolute top-2 right-2 text-muted-foreground/60 hover:text-foreground text-xs leading-none"
            aria-label="Dismiss"
          >
            &times;
          </button>
          <div className="flex items-center gap-1.5 mb-1">
            <span className="inline-block w-1.5 h-1.5 rounded-full bg-accent-green pulse-green" />
            <span className="text-[10px] font-medium text-accent-green uppercase tracking-wider">
              New Prime
            </span>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-xs font-medium text-accent-purple">
              {formLabel(toast.form)}
            </span>
            <span className="font-mono text-xs text-foreground truncate max-w-[200px]">
              {toast.expression}
            </span>
          </div>
          {toast.digits > 0 && (
            <p className="text-[10px] text-muted-foreground mt-1 tabular-nums">
              {toast.digits.toLocaleString()} digits
            </p>
          )}
        </div>
      ))}
    </div>
  );
}

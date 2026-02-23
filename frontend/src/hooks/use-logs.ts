import { useCallback, useEffect, useRef, useState } from "react";
import { API_BASE } from "@/lib/format";

export interface LogRow {
  id: number;
  ts: string;
  level: string;
  source: string;
  component: string;
  message: string;
  worker_id?: string | null;
  search_job_id?: number | null;
  search_id?: string | null;
  context?: Record<string, unknown> | null;
}

export interface LogStatsBucket {
  ts: string;
  error: number;
  warn: number;
  info: number;
  debug: number;
}

export interface UseLogsOptions {
  from: string;
  to: string;
  level?: string;
  component?: string;
  workerID?: string;
  q?: string;
  limit?: number;
}

export function useLogs(opts: UseLogsOptions) {
  const [logs, setLogs] = useState<LogRow[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let active = true;
    async function fetchLogs() {
      const params = new URLSearchParams({
        from: opts.from,
        to: opts.to,
        limit: (opts.limit ?? 200).toString(),
      });
      if (opts.level && opts.level !== "all") params.set("level", opts.level);
      if (opts.component?.trim()) params.set("component", opts.component.trim());
      if (opts.workerID?.trim()) params.set("worker_id", opts.workerID.trim());
      if (opts.q?.trim()) params.set("q", opts.q.trim());

      const endpoint = opts.q?.trim()
        ? `${API_BASE}/api/observability/logs/search`
        : `${API_BASE}/api/observability/logs`;

      setLoading(true);
      try {
        const res = await fetch(`${endpoint}?${params}`);
        if (!res.ok) throw new Error("fetch failed");
        const json = await res.json();
        const data = (json.data ?? json) as { logs: LogRow[] };
        if (active) setLogs(data.logs ?? []);
      } catch {
        if (active) setLogs([]);
      } finally {
        if (active) setLoading(false);
      }
    }
    fetchLogs();
    return () => {
      active = false;
    };
  }, [opts.from, opts.to, opts.level, opts.component, opts.workerID, opts.q, opts.limit]);

  return { logs, loading };
}

export function useLogStats(from: string, to: string, bucket: string) {
  const [buckets, setBuckets] = useState<LogStatsBucket[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let active = true;
    async function fetchStats() {
      const params = new URLSearchParams({ from, to, bucket });
      setLoading(true);
      try {
        const res = await fetch(
          `${API_BASE}/api/observability/logs/stats?${params}`
        );
        if (!res.ok) throw new Error("fetch failed");
        const json = await res.json();
        if (active) setBuckets(json.buckets ?? []);
      } catch {
        if (active) setBuckets([]);
      } finally {
        if (active) setLoading(false);
      }
    }
    fetchStats();
    return () => {
      active = false;
    };
  }, [from, to, bucket]);

  return { buckets, loading };
}

/** SSE hook for live log streaming. Returns a growing list of recent log entries. */
export function useLogStream(
  enabled: boolean,
  level?: string,
  component?: string
) {
  const [streamLogs, setStreamLogs] = useState<LogRow[]>([]);
  const eventSourceRef = useRef<EventSource | null>(null);

  const clear = useCallback(() => setStreamLogs([]), []);

  useEffect(() => {
    if (!enabled) {
      eventSourceRef.current?.close();
      eventSourceRef.current = null;
      return;
    }

    const params = new URLSearchParams();
    if (level && level !== "all") params.set("level", level);
    if (component?.trim()) params.set("component", component.trim());

    const url = `${API_BASE}/api/observability/logs/stream?${params}`;
    const es = new EventSource(url);
    eventSourceRef.current = es;

    es.onmessage = (event) => {
      try {
        const log = JSON.parse(event.data) as LogRow;
        setStreamLogs((prev) => {
          const next = [log, ...prev];
          return next.length > 500 ? next.slice(0, 500) : next;
        });
      } catch {
        // ignore parse errors
      }
    };

    es.onerror = () => {
      // EventSource auto-reconnects
    };

    return () => {
      es.close();
      eventSourceRef.current = null;
    };
  }, [enabled, level, component]);

  return { streamLogs, clear };
}

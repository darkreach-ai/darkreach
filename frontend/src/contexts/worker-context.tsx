"use client";

/**
 * @module worker-context
 *
 * React context for browser-based compute contribution. Manages a Web Worker
 * that runs prime searches (WASM-accelerated with JS BigInt fallback) in a
 * background thread. The work loop claims blocks from the coordinator via
 * JWT-authed REST, dispatches them to the Web Worker, and submits results back.
 *
 * Provides `useContribute()` hook with status, stats, start/stop controls,
 * and an activity log.
 */

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useAuth } from "@/contexts/auth-context";
import { API_BASE } from "@/lib/format";

export type ContributeStatus =
  | "idle"
  | "claiming"
  | "running"
  | "submitting"
  | "paused"
  | "error";

export interface ContributeStats {
  tested: number;
  found: number;
  blocksCompleted: number;
  speed: number;
  sessionStart: number | null;
  currentBlockId: number | null;
  searchType: string | null;
  error: string | null;
  mode: "wasm" | "js" | null;
  blockProgress: number | null;
  /** Feedback from the most recent result submission. */
  lastSubmission: SubmissionFeedback | null;
}

/** Enriched response from POST /api/v1/contribute/result. */
export interface SubmissionFeedback {
  status: string;
  hashVerified: boolean;
  creditsEarned: number;
  trustLevel: number;
  badgesEarned: number;
  warnings: string[];
}

export interface ActivityEntry {
  id: number;
  time: number;
  type: "claimed" | "found" | "completed" | "error";
  message: string;
}

export interface ContributeData {
  status: ContributeStatus;
  stats: ContributeStats;
  log: ActivityEntry[];
  start: () => void;
  stop: () => void;
}

const SESSION_STORAGE_KEY = "darkreach-contribute-session";

const initialStats: ContributeStats = {
  tested: 0,
  found: 0,
  blocksCompleted: 0,
  speed: 0,
  sessionStart: null,
  currentBlockId: null,
  searchType: null,
  error: null,
  mode: null,
  blockProgress: null,
  lastSubmission: null,
};

/** Load persisted session stats from sessionStorage. */
function loadSessionStats(): Partial<ContributeStats> | null {
  try {
    const raw = sessionStorage.getItem(SESSION_STORAGE_KEY);
    if (!raw) return null;
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

/** Save session stats to sessionStorage. */
function saveSessionStats(stats: ContributeStats) {
  try {
    sessionStorage.setItem(
      SESSION_STORAGE_KEY,
      JSON.stringify({
        tested: stats.tested,
        found: stats.found,
        blocksCompleted: stats.blocksCompleted,
      })
    );
  } catch {
    // sessionStorage may be unavailable
  }
}

const ContributeContext = createContext<ContributeData | null>(null);

/** Get a fresh JWT access token for API calls. */
async function getToken(): Promise<string | null> {
  // Dynamic import to avoid SSR issues
  const { supabase } = await import("@/lib/supabase");
  const {
    data: { session },
  } = await supabase.auth.getSession();
  return session?.access_token ?? null;
}

export function ContributeProvider({ children }: { children: ReactNode }) {
  const { session } = useAuth();
  const [status, setStatus] = useState<ContributeStatus>("idle");
  const [stats, setStats] = useState<ContributeStats>(initialStats);
  const [log, setLog] = useState<ActivityEntry[]>([]);

  const workerRef = useRef<Worker | null>(null);
  const heartbeatRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const runningRef = useRef(false);
  const logIdRef = useRef(0);
  const blockRangeRef = useRef<{ start: number; end: number } | null>(null);
  const statusBeforePauseRef = useRef<ContributeStatus>("running");
  const statsRef = useRef<ContributeStats>(initialStats);
  const blockSizeHintRef = useRef<number>(1000);
  const blockStartTimeRef = useRef<number>(0);

  // Keep statsRef in sync for use in heartbeat callback
  useEffect(() => {
    statsRef.current = stats;
  }, [stats]);

  const addLog = useCallback(
    (type: ActivityEntry["type"], message: string) => {
      setLog((prev) => {
        const entry: ActivityEntry = {
          id: logIdRef.current++,
          time: Date.now(),
          type,
          message,
        };
        const next = [entry, ...prev];
        return next.length > 50 ? next.slice(0, 50) : next;
      });
    },
    []
  );

  /** Claim a work block from the coordinator. */
  const claimBlock = useCallback(
    async (
      mode: "wasm" | "js" | null
    ): Promise<Record<string, unknown> | null> => {
      const token = await getToken();
      if (!token) return null;

      const engineParam = mode === "wasm" ? "?engine=wasm" : "";
      const res = await fetch(
        `${API_BASE}/api/v1/contribute/work${engineParam}`,
        {
          headers: { Authorization: `Bearer ${token}` },
        }
      );

      if (res.status === 204) return null; // No blocks available
      if (!res.ok) throw new Error(`Work claim failed: ${res.status}`);
      return res.json();
    },
    []
  );

  /** Submit results for a completed block. Returns enriched feedback. */
  const submitResult = useCallback(
    async (
      blockId: number,
      tested: number,
      found: number,
      primes: Array<{
        expression: string;
        form: string;
        digits: number;
        proof_method: string;
      }>,
      resultHash?: string | null,
      durationMs?: number | null
    ): Promise<SubmissionFeedback | null> => {
      const token = await getToken();
      if (!token) return null;

      const res = await fetch(`${API_BASE}/api/v1/contribute/result`, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${token}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          block_id: blockId,
          tested,
          found,
          primes,
          ...(resultHash ? { result_hash: resultHash } : {}),
          ...(durationMs ? { duration_ms: durationMs } : {}),
        }),
      });

      if (!res.ok) return null;

      try {
        const data = await res.json();
        return {
          status: data.status ?? "ok",
          hashVerified: data.hash_verified ?? false,
          creditsEarned: data.credits_earned ?? 0,
          trustLevel: data.trust_level ?? 1,
          badgesEarned: data.badges_earned ?? 0,
          warnings: data.warnings ?? [],
        };
      } catch {
        return null;
      }
    },
    []
  );

  /** Send a heartbeat with speed data to keep the browser node alive. */
  const sendHeartbeat = useCallback(async () => {
    const token = await getToken();
    if (!token) return;

    try {
      const res = await fetch(`${API_BASE}/api/v1/contribute/heartbeat`, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${token}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          speed: statsRef.current.speed || undefined,
          search_type: statsRef.current.searchType || undefined,
        }),
      });
      if (res.ok) {
        const data = await res.json();
        if (data.block_size_hint) {
          blockSizeHintRef.current = data.block_size_hint;
        }
      }
    } catch {
      // Heartbeat failures are non-fatal
    }
  }, []);

  /** Main work loop: claim → run → submit → repeat. */
  const runWorkLoop = useCallback(async () => {
    if (!runningRef.current) return;

    // Create the Web Worker
    const worker = new Worker("/prime-worker.js");
    workerRef.current = worker;

    // Collected primes for current block
    let blockPrimes: Array<{
      expression: string;
      form: string;
      digits: number;
      proof_method: string;
    }> = [];
    let blockTested = 0;
    let blockFound = 0;
    let currentBlockId: number | null = null;
    let engineMode: "wasm" | "js" | null = null;

    const processNextBlock = async () => {
      if (!runningRef.current) return;

      // Claim
      setStatus("claiming");
      try {
        const block = await claimBlock(engineMode);
        if (!block) {
          // No blocks available — wait 30s and retry
          addLog("error", "No work blocks available, retrying in 30s...");
          if (runningRef.current) {
            setTimeout(processNextBlock, 30_000);
          }
          return;
        }

        currentBlockId = block.block_id as number;
        blockPrimes = [];
        blockTested = 0;
        blockFound = 0;
        blockRangeRef.current = {
          start: block.block_start as number,
          end: block.block_end as number,
        };

        setStats((prev) => ({
          ...prev,
          currentBlockId: currentBlockId,
          searchType: (block.search_type as string) || null,
          error: null,
          blockProgress: 0,
        }));
        addLog(
          "claimed",
          `Block #${currentBlockId} (${block.search_type}): ${block.block_start}..${block.block_end}`
        );
        setStatus("running");

        // Record block start time for duration tracking
        blockStartTimeRef.current = Date.now();

        // Dispatch to Web Worker
        worker.postMessage({
          type: "start",
          block: {
            block_id: currentBlockId,
            search_type: block.search_type,
            params: block.params,
            block_start: block.block_start,
            block_end: block.block_end,
          },
          config: { batchSize: 100 },
        });
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setStatus("error");
        setStats((prev) => ({ ...prev, error: message }));
        addLog("error", message);
        // Retry after delay
        if (runningRef.current) {
          setTimeout(processNextBlock, 10_000);
        }
      }
    };

    // Listen for worker messages
    worker.onmessage = async (e: MessageEvent) => {
      const msg = e.data;

      switch (msg.type) {
        case "init":
          engineMode = msg.mode;
          setStats((prev) => ({ ...prev, mode: msg.mode }));
          addLog(
            "claimed",
            `Engine initialized: ${msg.mode === "wasm" ? "WASM" : "JS BigInt"}`
          );
          break;

        case "progress": {
          // Compute block progress
          let blockProgress: number | null = null;
          if (blockRangeRef.current) {
            const { start, end } = blockRangeRef.current;
            const range = end - start;
            if (range > 0) {
              blockProgress = Math.min(
                1,
                (msg.current - start) / range
              );
            }
          }

          setStats((prev) => {
            const updated = {
              ...prev,
              tested: prev.tested - blockTested + msg.tested,
              found: prev.found - blockFound + msg.found,
              speed: msg.speed,
              blockProgress,
            };
            saveSessionStats(updated);
            return updated;
          });
          blockTested = msg.tested;
          blockFound = msg.found;
          break;
        }

        case "prime":
          blockPrimes.push({
            expression: msg.expression,
            form: msg.form,
            digits: msg.digits,
            proof_method: msg.proof_method,
          });
          blockFound++;
          setStats((prev) => {
            const updated = { ...prev, found: prev.found + 1 };
            saveSessionStats(updated);
            return updated;
          });
          addLog("found", `Prime: ${msg.expression} (${msg.digits} digits)`);
          break;

        case "done": {
          blockTested = msg.tested;
          blockFound = msg.found;

          // Compute block duration from start time
          const durationMs = blockStartTimeRef.current > 0
            ? Date.now() - blockStartTimeRef.current
            : null;

          setStats((prev) => ({
            ...prev,
            tested: prev.tested - blockTested + msg.tested,
            blockProgress: null,
          }));

          // Submit result
          setStatus("submitting");
          try {
            if (currentBlockId != null) {
              const feedback = await submitResult(
                currentBlockId,
                blockTested,
                blockFound,
                blockPrimes,
                msg.result_hash,
                durationMs
              );
              setStats((prev) => {
                const updated = {
                  ...prev,
                  blocksCompleted: prev.blocksCompleted + 1,
                  currentBlockId: null,
                  lastSubmission: feedback,
                };
                saveSessionStats(updated);
                return updated;
              });

              // Build feedback log message
              let feedbackMsg = `Block #${currentBlockId}: ${blockTested} tested, ${blockFound} found`;
              if (feedback) {
                feedbackMsg += ` | +${feedback.creditsEarned} credits`;
                if (feedback.hashVerified) feedbackMsg += " | hash verified";
                if (feedback.badgesEarned > 0)
                  feedbackMsg += ` | ${feedback.badgesEarned} new badge(s)!`;
              }
              addLog("completed", feedbackMsg);

              // Show warnings from server
              if (feedback?.warnings?.length) {
                for (const w of feedback.warnings) {
                  addLog("error", `Warning: ${w}`);
                }
              }
            }
          } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            addLog("error", `Submit failed: ${message}`);
          }

          blockRangeRef.current = null;

          if (msg.reason === "stopped" || !runningRef.current) {
            setStatus("idle");
            return;
          }

          // Claim next block
          processNextBlock();
          break;
        }

        case "error":
          setStats((prev) => ({ ...prev, error: msg.message }));
          addLog("error", msg.message);
          // Try next block
          if (runningRef.current) {
            setTimeout(processNextBlock, 5_000);
          }
          break;
      }
    };

    worker.onerror = (err) => {
      addLog("error", `Worker error: ${err.message}`);
      setStatus("error");
      setStats((prev) => ({ ...prev, error: err.message }));
    };

    // Start the heartbeat interval (30s)
    heartbeatRef.current = setInterval(sendHeartbeat, 30_000);
    sendHeartbeat(); // Immediate first heartbeat

    // Begin claiming (wait briefly for WASM init message)
    setTimeout(processNextBlock, 200);
  }, [claimBlock, submitResult, sendHeartbeat, addLog]);

  const start = useCallback(() => {
    if (runningRef.current || !session?.access_token) return;
    runningRef.current = true;

    // Restore persisted session stats (tested/found/blocksCompleted)
    const saved = loadSessionStats();
    setStats({
      ...initialStats,
      sessionStart: Date.now(),
      tested: saved?.tested ?? 0,
      found: saved?.found ?? 0,
      blocksCompleted: saved?.blocksCompleted ?? 0,
    });
    setLog([]);
    runWorkLoop();
  }, [session?.access_token, runWorkLoop]);

  const stop = useCallback(() => {
    runningRef.current = false;
    if (workerRef.current) {
      workerRef.current.postMessage({ type: "stop" });
    }
    if (heartbeatRef.current) {
      clearInterval(heartbeatRef.current);
      heartbeatRef.current = null;
    }
    setStatus("idle");
  }, []);

  // Tab visibility: pause/resume worker when tab is hidden/visible
  useEffect(() => {
    const handler = () => {
      if (!runningRef.current || !workerRef.current) return;

      if (document.hidden) {
        statusBeforePauseRef.current = "running";
        workerRef.current.postMessage({ type: "pause" });
        setStatus("paused");
      } else {
        workerRef.current.postMessage({ type: "resume" });
        setStatus(statusBeforePauseRef.current);
      }
    };
    document.addEventListener("visibilitychange", handler);
    return () => document.removeEventListener("visibilitychange", handler);
  }, []);

  // Clean up on unmount
  useEffect(() => {
    return () => {
      runningRef.current = false;
      if (workerRef.current) {
        workerRef.current.terminate();
        workerRef.current = null;
      }
      if (heartbeatRef.current) {
        clearInterval(heartbeatRef.current);
        heartbeatRef.current = null;
      }
    };
  }, []);

  // beforeunload handler for clean shutdown
  useEffect(() => {
    const handler = () => {
      if (runningRef.current) {
        runningRef.current = false;
        workerRef.current?.postMessage({ type: "stop" });
      }
    };
    window.addEventListener("beforeunload", handler);
    return () => window.removeEventListener("beforeunload", handler);
  }, []);

  return (
    <ContributeContext.Provider value={{ status, stats, log, start, stop }}>
      {children}
    </ContributeContext.Provider>
  );
}

export function useContribute(): ContributeData {
  const ctx = useContext(ContributeContext);
  if (!ctx) {
    throw new Error("useContribute must be used within a ContributeProvider");
  }
  return ctx;
}

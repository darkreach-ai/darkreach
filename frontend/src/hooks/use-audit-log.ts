"use client";

/**
 * @module use-audit-log
 *
 * React hook for fetching paginated and filtered audit log entries
 * from the admin API. Requires admin authentication via Supabase JWT.
 *
 * @see {@link src/dashboard/routes_audit.rs} -- Rust-side audit log endpoint
 */

import { useCallback, useEffect, useState } from "react";
import { adminFetch } from "@/lib/api";

/** A single audit log entry returned by the API. */
export interface AuditLogEntry {
  id: number;
  user_id: string;
  user_email: string | null;
  action: string;
  resource: string | null;
  method: string;
  status_code: number | null;
  ip_address: string | null;
  user_agent: string | null;
  payload: Record<string, unknown> | null;
  created_at: string;
}

interface UseAuditLogOptions {
  /** Page number (1-based). Defaults to 1. */
  page?: number;
  /** Entries per page. Defaults to 50. */
  limit?: number;
  /** Filter by action type (exact match). */
  action?: string;
  /** Filter by user ID (exact match). */
  userId?: string;
}

interface UseAuditLogResult {
  entries: AuditLogEntry[];
  total: number;
  isLoading: boolean;
  error: string | null;
  refetch: () => Promise<void>;
}

/**
 * Hook to fetch paginated audit log entries with optional filters.
 *
 * Uses `adminFetch` to attach the Supabase JWT for admin authentication.
 * Returns entries sorted by most recent first.
 */
export function useAuditLog(options: UseAuditLogOptions = {}): UseAuditLogResult {
  const { page = 1, limit = 50, action, userId } = options;

  const [entries, setEntries] = useState<AuditLogEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchAuditLog = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const params = new URLSearchParams({
        limit: String(limit),
        offset: String((page - 1) * limit),
      });
      if (action) params.set("action", action);
      if (userId) params.set("user_id", userId);

      const resp = await adminFetch(`/api/audit?${params}`);
      if (!resp.ok) {
        const body = await resp.json().catch(() => ({}));
        throw new Error(
          (body as Record<string, string>).error || `HTTP ${resp.status}`
        );
      }

      const json = await resp.json();
      const data = json.data ?? json;
      setEntries(data.entries ?? []);
      setTotal(data.total ?? 0);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to fetch audit log";
      setError(message);
      setEntries([]);
      setTotal(0);
    } finally {
      setIsLoading(false);
    }
  }, [page, limit, action, userId]);

  useEffect(() => {
    void fetchAuditLog();
  }, [fetchAuditLog]);

  return { entries, total, isLoading, error, refetch: fetchAuditLog };
}

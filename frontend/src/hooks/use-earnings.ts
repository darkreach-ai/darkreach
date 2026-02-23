"use client";

/**
 * @module use-earnings
 *
 * React hook that fetches an operator's credit history and monthly earnings
 * from the `/api/v1/operators/me/credits` and `/api/v1/operators/me/earnings`
 * endpoints. Requires JWT auth (Supabase session).
 *
 * Returns paginated credit rows and 12-month earnings aggregation for
 * the earnings dashboard page.
 */

import { useCallback, useEffect, useState } from "react";
import { API_BASE } from "@/lib/format";
import { useAuth } from "@/contexts/auth-context";

export interface CreditRow {
  id: number;
  block_id: number | null;
  credit: number;
  reason: string | null;
  granted_at: string;
}

export interface MonthlyEarning {
  month: string;
  total_credits: number | null;
  block_count: number | null;
}

export function useEarnings(limit = 50, offset = 0) {
  const { session } = useAuth();
  const [credits, setCredits] = useState<CreditRow[]>([]);
  const [earnings, setEarnings] = useState<MonthlyEarning[]>([]);
  const [loading, setLoading] = useState(true);

  const headers: Record<string, string> = {};
  if (session?.access_token) {
    headers["Authorization"] = `Bearer ${session.access_token}`;
  }

  const fetchCredits = useCallback(async () => {
    if (!session?.access_token) return;
    try {
      const res = await fetch(
        `${API_BASE}/api/v1/operators/me/credits?limit=${limit}&offset=${offset}`,
        { headers: { Authorization: `Bearer ${session.access_token}` } }
      );
      if (res.ok) {
        setCredits(await res.json());
      }
    } catch {
      // Network error — keep previous state
    }
  }, [session?.access_token, limit, offset]);

  const fetchEarnings = useCallback(async () => {
    if (!session?.access_token) return;
    try {
      const res = await fetch(
        `${API_BASE}/api/v1/operators/me/earnings`,
        { headers: { Authorization: `Bearer ${session.access_token}` } }
      );
      if (res.ok) {
        setEarnings(await res.json());
      }
    } catch {
      // Network error — keep previous state
    }
  }, [session?.access_token]);

  useEffect(() => {
    setLoading(true);
    Promise.all([fetchCredits(), fetchEarnings()]).finally(() =>
      setLoading(false)
    );
    const interval = setInterval(() => {
      fetchCredits();
      fetchEarnings();
    }, 30_000);
    return () => clearInterval(interval);
  }, [fetchCredits, fetchEarnings]);

  return { credits, earnings, loading, refetch: fetchCredits };
}

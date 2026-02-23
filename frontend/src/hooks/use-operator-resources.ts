"use client";

/**
 * @module use-operator-resources
 *
 * React hook that fetches operator stats and trust level from the
 * `/api/v1/operators/stats` endpoint (API key auth) and supplements
 * with JWT-authed credit data. Provides a combined summary of the
 * operator's account for the earnings dashboard.
 */

import { useCallback, useEffect, useState } from "react";
import { API_BASE } from "@/lib/format";
import { useAuth } from "@/contexts/auth-context";

export interface OperatorStats {
  username: string;
  credit: number;
  primes_found: number;
  trust_level: number | null;
  rank: number | null;
}

export function useOperatorStats() {
  const { session } = useAuth();
  const [stats, setStats] = useState<OperatorStats | null>(null);
  const [loading, setLoading] = useState(true);

  const fetchStats = useCallback(async () => {
    if (!session?.access_token) return;
    try {
      // Use the JWT-authed operator stats endpoint
      const res = await fetch(`${API_BASE}/api/v1/operators/stats`, {
        headers: { Authorization: `Bearer ${session.access_token}` },
      });
      if (res.ok) {
        setStats(await res.json());
      }
    } catch {
      // Network error — keep previous state
    }
  }, [session?.access_token]);

  useEffect(() => {
    setLoading(true);
    fetchStats().finally(() => setLoading(false));
    const interval = setInterval(fetchStats, 30_000);
    return () => clearInterval(interval);
  }, [fetchStats]);

  return { stats, loading, refetch: fetchStats };
}

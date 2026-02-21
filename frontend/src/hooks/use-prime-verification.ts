"use client";

/**
 * @module use-prime-verification
 *
 * Hooks for the distributed prime verification queue.
 *
 * - `usePrimeVerificationStats()` — polls queue depth and completion rate
 * - `usePrimeVerifications(primeId)` — fetches per-prime verification history
 */

import { useEffect, useState, useCallback, useRef } from "react";

const API_BASE = process.env.NEXT_PUBLIC_API_URL || "";

export interface PrimeVerificationStats {
  pending: number;
  claimed: number;
  verified: number;
  failed: number;
  total_primes: number;
  quorum_met: number;
}

export interface PrimeVerificationResult {
  id: number;
  prime_id: number;
  status: string;
  claimed_by: string | null;
  claimed_at: string | null;
  completed_at: string | null;
  verification_tier: number | null;
  verification_method: string | null;
  result_detail: unknown | null;
  error_reason: string | null;
}

/** Poll verification queue stats every 30s. */
export function usePrimeVerificationStats() {
  const [stats, setStats] = useState<PrimeVerificationStats | null>(null);
  const [loading, setLoading] = useState(true);
  const intervalRef = useRef<ReturnType<typeof setInterval>>(undefined);

  const fetchStats = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/api/prime-verification/stats`);
      if (res.ok) {
        const data = await res.json();
        if (data.ok && data.stats) {
          setStats(data.stats);
        }
      }
    } catch {
      // network error — leave existing state
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchStats();
    intervalRef.current = setInterval(fetchStats, 30_000);
    return () => clearInterval(intervalRef.current);
  }, [fetchStats]);

  return { stats, loading, refetch: fetchStats };
}

/** Fetch verification history for a specific prime. */
export function usePrimeVerifications(primeId: number | null) {
  const [results, setResults] = useState<PrimeVerificationResult[]>([]);
  const [loading, setLoading] = useState(false);

  const fetchVerifications = useCallback(async (id: number) => {
    setLoading(true);
    try {
      const res = await fetch(`${API_BASE}/api/primes/${id}/verifications`);
      if (res.ok) {
        const data = await res.json();
        if (data.ok && data.results) {
          setResults(data.results);
        }
      }
    } catch {
      // network error
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (primeId != null) {
      fetchVerifications(primeId);
    } else {
      setResults([]);
    }
  }, [primeId, fetchVerifications]);

  return { results, loading };
}

/** Fetch tag distribution for charts. */
export function useTagDistribution() {
  const [tags, setTags] = useState<{ tag: string; count: number }[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchTags = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/api/stats/tags`);
      if (res.ok) {
        const data = await res.json();
        setTags(Array.isArray(data) ? data : []);
      }
    } catch {
      // network error
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchTags();
  }, [fetchTags]);

  return { tags, loading, refetch: fetchTags };
}

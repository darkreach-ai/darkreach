/**
 * @module use-contribute-profile
 *
 * Fetches operator profile data for the contribute page: earned badges,
 * recent credits, trust level, and next-badge progress. Combines three
 * parallel API calls and recomputes derived state every 60 seconds while
 * the contribute page is active.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { API_BASE } from "@/lib/format";

export interface OperatorBadge {
  badge_id: string;
  earned_at: string;
}

export interface BadgeDefinition {
  id: string;
  name: string;
  description: string;
  icon: string;
  requirement_type: string;
  requirement_value: number;
}

export interface CreditRow {
  id: number;
  block_id: number | null;
  credit: number;
  reason: string | null;
  granted_at: string;
}

export interface TrustInfo {
  trust_level: number;
  consecutive_valid: number;
  total_valid: number;
}

export interface ContributeProfile {
  badges: OperatorBadge[];
  badgeDefinitions: BadgeDefinition[];
  recentCredits: CreditRow[];
  totalCredits: number;
  trust: TrustInfo;
  nextBadge: {
    definition: BadgeDefinition;
    progress: number;
  } | null;
  loading: boolean;
  error: string | null;
}

async function getToken(): Promise<string | null> {
  const { supabase } = await import("@/lib/supabase");
  const {
    data: { session },
  } = await supabase.auth.getSession();
  return session?.access_token ?? null;
}

async function fetchJson<T>(url: string, token: string): Promise<T> {
  const res = await fetch(url, {
    headers: { Authorization: `Bearer ${token}` },
  });
  if (!res.ok) throw new Error(`${url}: ${res.status}`);
  return res.json();
}

const REFRESH_INTERVAL = 60_000;

export function useContributeProfile(): ContributeProfile {
  const [badges, setBadges] = useState<OperatorBadge[]>([]);
  const [badgeDefinitions, setBadgeDefinitions] = useState<BadgeDefinition[]>(
    []
  );
  const [recentCredits, setRecentCredits] = useState<CreditRow[]>([]);
  const [trust, setTrust] = useState<TrustInfo>({
    trust_level: 1,
    consecutive_valid: 0,
    total_valid: 0,
  });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchAll = useCallback(async () => {
    const token = await getToken();
    if (!token) {
      setLoading(false);
      return;
    }

    try {
      const [badgesData, creditsData, defsData, trustData] = await Promise.all([
        fetchJson<OperatorBadge[]>(
          `${API_BASE}/api/v1/operators/me/badges`,
          token
        ),
        fetchJson<CreditRow[]>(
          `${API_BASE}/api/v1/operators/me/credits?limit=10`,
          token
        ),
        fetchJson<BadgeDefinition[]>(`${API_BASE}/api/v1/badges`, token),
        fetchJson<TrustInfo>(
          `${API_BASE}/api/v1/operators/me/trust`,
          token
        ),
      ]);

      setBadges(badgesData);
      setRecentCredits(creditsData);
      setBadgeDefinitions(defsData);
      setTrust(trustData);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchAll();
    intervalRef.current = setInterval(fetchAll, REFRESH_INTERVAL);
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [fetchAll]);

  // Compute derived values
  const totalCredits = recentCredits.reduce((sum, c) => sum + c.credit, 0);
  const earnedIds = new Set(badges.map((b) => b.badge_id));

  // Find the next unearned badge closest to completion
  let nextBadge: ContributeProfile["nextBadge"] = null;
  for (const def of badgeDefinitions) {
    if (earnedIds.has(def.id)) continue;

    let progress = 0;
    if (def.requirement_type === "blocks_completed") {
      progress = trust.total_valid / Math.max(1, def.requirement_value);
    } else if (def.requirement_type === "primes_found") {
      // Use total_valid as approximation since we don't have primes_found here
      progress = 0;
    } else if (def.requirement_type === "trust_level") {
      progress = trust.trust_level / Math.max(1, def.requirement_value);
    }

    progress = Math.min(1, progress);

    if (!nextBadge || progress > nextBadge.progress) {
      nextBadge = { definition: def, progress };
    }
  }

  return {
    badges,
    badgeDefinitions,
    recentCredits,
    totalCredits,
    trust,
    nextBadge,
    loading,
    error,
  };
}

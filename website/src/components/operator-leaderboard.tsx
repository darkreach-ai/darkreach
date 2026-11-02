"use client";

import { useEffect, useState, useCallback } from "react";
import { API_BASE } from "@/lib/api";

interface LeaderboardEntry {
  rank: number;
  username: string;
  team: string | null;
  credit: number;
  primes_found: number;
  node_count: number | null;
  trust_level: number | null;
}

const POLL_INTERVAL = 60_000;
const FETCH_TIMEOUT = 10_000;

function formatNumber(n: number): string {
  return n.toLocaleString();
}

function TrustBadge({ level }: { level: number | null }) {
  if (level == null || level <= 1) return null;
  const labels: Record<number, string> = {
    2: "Proven",
    3: "Trusted",
    4: "Core",
  };
  return (
    <span className="ml-2 text-xs px-1.5 py-0.5 rounded bg-accent-purple/10 text-accent-purple">
      {labels[level] ?? `L${level}`}
    </span>
  );
}

export function OperatorLeaderboard() {
  const [entries, setEntries] = useState<LeaderboardEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchLeaderboard = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/api/v1/leaderboard`, {
        signal: AbortSignal.timeout(FETCH_TIMEOUT),
      });
      if (!res.ok) {
        setError(`Failed to load leaderboard (HTTP ${res.status})`);
        return;
      }
      const data: LeaderboardEntry[] = await res.json();
      setEntries(data);
      setError(null);
    } catch (err) {
      if (err instanceof DOMException && err.name === "TimeoutError") {
        setError("Request timed out");
      } else if (err instanceof DOMException && err.name === "AbortError") {
        // Component unmounted, ignore
        return;
      } else {
        setError("Failed to load leaderboard");
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchLeaderboard();
    const interval = setInterval(fetchLeaderboard, POLL_INTERVAL);
    return () => {
      clearInterval(interval);
    };
  }, [fetchLeaderboard]);

  if (loading) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        Loading leaderboard...
      </div>
    );
  }

  if (error) {
    return (
      <div className="text-center py-12 text-red-500">
        {error}
      </div>
    );
  }

  if (entries.length === 0) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        No operators yet. Be the first!
      </div>
    );
  }

  return (
    <div className="overflow-x-auto rounded-lg border border-border">
      <table className="w-full text-sm">
        <thead>
          <tr className="bg-card text-muted-foreground text-left">
            <th className="px-4 py-3 font-medium w-16">Rank</th>
            <th className="px-4 py-3 font-medium">Operator</th>
            <th className="px-4 py-3 font-medium">Team</th>
            <th className="px-4 py-3 font-medium text-right">Credit</th>
            <th className="px-4 py-3 font-medium text-right">Primes</th>
            <th className="px-4 py-3 font-medium text-right">Nodes</th>
          </tr>
        </thead>
        <tbody>
          {entries.map((entry) => (
            <tr
              key={entry.rank}
              className="border-t border-border hover:bg-card/50 transition-colors"
            >
              <td className="px-4 py-3 tabular-nums text-muted-foreground">
                #{entry.rank}
              </td>
              <td className="px-4 py-3 font-mono text-foreground">
                {entry.username}
                <TrustBadge level={entry.trust_level} />
              </td>
              <td className="px-4 py-3 text-muted-foreground">
                {entry.team ?? "\u2014"}
              </td>
              <td className="px-4 py-3 tabular-nums text-foreground text-right">
                {formatNumber(entry.credit)}
              </td>
              <td className="px-4 py-3 tabular-nums text-accent-purple text-right">
                {formatNumber(entry.primes_found)}
              </td>
              <td className="px-4 py-3 tabular-nums text-muted-foreground text-right">
                {entry.node_count != null ? formatNumber(entry.node_count) : "\u2014"}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

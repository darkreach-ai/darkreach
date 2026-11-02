"use client";

import { useEffect, useState } from "react";
import { Section } from "@/components/ui/section";
import { Badge } from "@/components/ui/badge";
import { API_BASE } from "@/lib/api";
import { formLabel, formColor, timeAgo } from "@/lib/prime-feed";

/**
 * Matches the row shape returned by get_form_leaderboard() in PostgreSQL:
 * { form, count, largest_digits, largest_expression, latest_found_at, verified_count, verified_pct }
 */
interface FormRanking {
  form: string;
  count: number;
  largest_digits: number;
  largest_expression: string;
  latest_found_at: string;
  verified_count: number;
  verified_pct: number;
}

export function FormRankings() {
  const [rankings, setRankings] = useState<FormRanking[]>([]);
  const [live, setLive] = useState(false);

  useEffect(() => {
    let active = true;

    async function fetchRankings() {
      try {
        const res = await fetch(`${API_BASE}/api/stats/leaderboard`, {
          signal: AbortSignal.timeout(10000),
        });
        if (!res.ok || !active) return;

        // The endpoint returns a JSON array directly from get_form_leaderboard()
        const data = (await res.json()) as FormRanking[];
        if (active && Array.isArray(data) && data.length > 0) {
          setRankings(data);
          setLive(true);
        }
      } catch {
        // Keep empty state -- section won't render without data
      }
    }

    fetchRankings();
    return () => {
      active = false;
    };
  }, []);

  // Don't render the section if no data was fetched
  if (rankings.length === 0 && !live) {
    return null;
  }

  return (
    <Section>
      <div className="flex items-center gap-3 mb-4">
        <h2 className="text-2xl font-bold text-foreground">Form Rankings</h2>
        {live && (
          <Badge variant="green">
            <span className="inline-block w-1.5 h-1.5 rounded-full bg-accent-green pulse-green mr-1.5" />
            Live
          </Badge>
        )}
      </div>
      <p className="text-muted-foreground text-sm mb-6">
        Prime discoveries aggregated by search form, ranked by count.
      </p>

      <div className="overflow-x-auto rounded-lg border border-border">
        <table className="w-full text-sm">
          <thead>
            <tr className="bg-card text-muted-foreground text-left">
              <th className="px-4 py-3 font-medium w-12">#</th>
              <th className="px-4 py-3 font-medium">Form</th>
              <th className="px-4 py-3 font-medium text-right">Primes</th>
              <th className="px-4 py-3 font-medium text-right">Largest</th>
              <th className="px-4 py-3 font-medium">Record Expression</th>
              <th className="px-4 py-3 font-medium text-right">Verified</th>
              <th className="px-4 py-3 font-medium text-right">Latest</th>
            </tr>
          </thead>
          <tbody>
            {rankings.map((r, i) => (
              <tr
                key={r.form}
                className="border-t border-border hover:bg-card/50 transition-colors"
              >
                <td className="px-4 py-3 text-muted-foreground tabular-nums">
                  {i + 1}
                </td>
                <td className="px-4 py-3">
                  <span
                    className={`inline-flex px-2.5 py-1 rounded-md text-[11px] font-medium border ${formColor(r.form)}`}
                  >
                    {formLabel(r.form)}
                  </span>
                </td>
                <td className="px-4 py-3 tabular-nums text-accent-purple text-right font-medium">
                  {r.count.toLocaleString()}
                </td>
                <td className="px-4 py-3 tabular-nums text-foreground text-right">
                  {r.largest_digits.toLocaleString()}
                  <span className="text-muted-foreground/60 ml-0.5">d</span>
                </td>
                <td className="px-4 py-3 font-mono text-xs text-muted-foreground truncate max-w-[200px]">
                  {r.largest_expression}
                </td>
                <td className="px-4 py-3 tabular-nums text-right">
                  <span className="text-accent-green">{r.verified_pct}%</span>
                </td>
                <td className="px-4 py-3 text-xs text-muted-foreground text-right tabular-nums">
                  {r.latest_found_at ? timeAgo(r.latest_found_at) : "-"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Section>
  );
}

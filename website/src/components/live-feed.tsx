"use client";

import { useEffect, useState } from "react";
import { Section } from "./ui/section";
import { ExternalLink } from "lucide-react";
import {
  type PrimeEntry,
  FALLBACK_ENTRIES,
  timeAgo,
  formLabel,
  formColor,
} from "@/lib/prime-feed";
import { API_BASE } from "@/lib/api";

const LIVE_FEED_FALLBACK: PrimeEntry[] = [
  ...FALLBACK_ENTRIES,
  { id: 4, form: "palindromic", expression: "10^498 + R(497)^rev + 1", digits: 499, discovered_at: "2026-02-17T22:15:00Z" },
  { id: 5, form: "kbn", expression: "3 \u00b7 2^59887 + 1", digits: 18029, discovered_at: "2026-02-17T19:40:00Z" },
  { id: 6, form: "kbn", expression: "3 \u00b7 2^59851 - 1", digits: 18018, discovered_at: "2026-02-17T16:10:00Z" },
];

export function LiveFeed() {
  const [entries, setEntries] = useState<PrimeEntry[]>(LIVE_FEED_FALLBACK);
  const [live, setLive] = useState(false);

  useEffect(() => {
    let active = true;
    async function fetchRecent() {
      try {
        const res = await fetch(`${API_BASE}/api/primes?limit=6`);
        if (!res.ok) return;
        const data = (await res.json()) as { primes?: PrimeEntry[] };
        if (active && data.primes && data.primes.length > 0) {
          setEntries(data.primes.slice(0, 6));
          setLive(true);
        }
      } catch {
        // Keep fallback
      }
    }
    fetchRecent();
    const timer = setInterval(fetchRecent, 30000);
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, []);

  return (
    <Section>
      <div className="flex items-center justify-between mb-8">
        <div>
          <div className="flex items-center gap-3 mb-1">
            <h2 className="text-3xl sm:text-4xl font-bold text-foreground">Discoveries</h2>
            {live && (
              <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-medium bg-accent-green/10 text-accent-green border border-accent-green/20">
                <span className="inline-block w-1.5 h-1.5 rounded-full bg-accent-green pulse-green" />
                Live
              </span>
            )}
          </div>
          <p className="text-muted-foreground text-sm sm:text-base">
            Latest primes found by the network, updated in real time.
          </p>
        </div>
        <a
          href="https://app.darkreach.ai/browse"
          className="hidden sm:inline-flex items-center gap-1.5 text-sm text-primary hover:underline flex-shrink-0"
        >
          View all
          <ExternalLink size={13} />
        </a>
      </div>

      {/* Latest discovery — featured */}
      {entries.length > 0 && (
        <div className="mb-3 p-5 rounded-xl border border-accent-purple/20 bg-accent-purple/[0.03]">
          <div className="flex items-center gap-2 mb-3">
            <span className="text-[11px] font-medium text-accent-purple/70 uppercase tracking-wider">Latest</span>
            <span className="text-xs text-muted-foreground/50">{timeAgo(entries[0].discovered_at)}</span>
          </div>
          <div className="flex items-center gap-3 flex-wrap">
            <span className={`inline-flex px-2.5 py-1 rounded-md text-[11px] font-medium border flex-shrink-0 ${formColor(entries[0].form)}`}>
              {formLabel(entries[0].form)}
            </span>
            <span className="font-mono text-base sm:text-lg text-foreground truncate flex-1 min-w-0">
              {entries[0].expression}
            </span>
            <span className="text-sm font-mono text-muted-foreground flex-shrink-0 tabular-nums">
              {entries[0].digits.toLocaleString()}
              <span className="text-muted-foreground/60 ml-0.5">digits</span>
            </span>
          </div>
        </div>
      )}

      {/* Remaining discoveries */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
        {entries.slice(1).map((entry) => (
          <div
            key={entry.id}
            className="flex items-center gap-3 px-4 py-3 rounded-lg border border-border/60 hover:border-border hover:bg-card/40 transition-colors"
          >
            <span className={`inline-flex px-2.5 py-1 rounded-md text-[11px] font-medium border flex-shrink-0 ${formColor(entry.form)}`}>
              {formLabel(entry.form)}
            </span>
            <span className="font-mono text-sm text-foreground truncate flex-1 min-w-0">
              {entry.expression}
            </span>
            <span className="text-sm font-mono text-muted-foreground flex-shrink-0 tabular-nums">
              {entry.digits.toLocaleString()}
              <span className="text-muted-foreground/60 ml-0.5">d</span>
            </span>
            <span className="text-xs text-muted-foreground/60 flex-shrink-0 w-14 text-right tabular-nums">
              {timeAgo(entry.discovered_at)}
            </span>
          </div>
        ))}
      </div>

      <div className="mt-4 text-center sm:hidden">
        <a
          href="https://app.darkreach.ai/browse"
          className="text-sm text-primary hover:underline"
        >
          View all discoveries →
        </a>
      </div>
    </Section>
  );
}

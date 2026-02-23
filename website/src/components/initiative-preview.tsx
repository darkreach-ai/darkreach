"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { Section } from "./ui/section";
import { ScrollAnimate } from "./scroll-animate";
import {
  type PrimeEntry,
  FALLBACK_ENTRIES,
  timeAgo,
  formLabel,
  formColor,
} from "@/lib/prime-feed";
import { API_BASE } from "@/lib/api";

export function InitiativePreview() {
  const [entries, setEntries] = useState<PrimeEntry[]>(FALLBACK_ENTRIES);
  const [live, setLive] = useState(false);

  useEffect(() => {
    let active = true;
    async function fetchRecent() {
      try {
        const res = await fetch(`${API_BASE}/api/primes?limit=3`);
        if (!res.ok) return;
        const data = (await res.json()) as { primes?: PrimeEntry[] };
        if (active && data.primes && data.primes.length > 0) {
          setEntries(data.primes.slice(0, 3));
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
      <ScrollAnimate>
        <div className="max-w-2xl mx-auto">
          <div className="flex items-center gap-3 mb-3">
            <p className="text-sm font-medium text-accent-purple uppercase tracking-wider">
              Our first initiative
            </p>
            {live && (
              <span className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-medium bg-accent-green/10 text-accent-green border border-accent-green/20">
                <span className="inline-block w-1.5 h-1.5 rounded-full bg-accent-green pulse-green" />
                Live
              </span>
            )}
          </div>
          <h2 className="text-3xl sm:text-4xl font-bold text-foreground mb-4">
            Prime Number Discovery
          </h2>
          <p className="text-muted-foreground leading-relaxed mb-8">
            Searching for record-breaking primes across 12 mathematical forms, with
            deterministic proofs and AI-driven strategy. Every discovery is verified,
            catalogued, and published.
          </p>

          {/* Mini live feed */}
          <div className="space-y-2 mb-8">
            {entries.map((entry) => (
              <div
                key={entry.id}
                className="flex items-center gap-3 px-4 py-3 rounded-lg border border-border/60 bg-card/50"
              >
                <span
                  className={`inline-flex px-2.5 py-1 rounded-md text-[11px] font-medium border flex-shrink-0 ${formColor(entry.form)}`}
                >
                  {formLabel(entry.form)}
                </span>
                <span className="font-mono text-sm text-foreground truncate flex-1 min-w-0">
                  {entry.expression}
                </span>
                <span className="text-xs text-muted-foreground flex-shrink-0 tabular-nums">
                  {entry.digits.toLocaleString()}d
                </span>
                <span className="text-xs text-muted-foreground/60 flex-shrink-0">
                  {timeAgo(entry.discovered_at)}
                </span>
              </div>
            ))}
          </div>

          <Link
            href="/research"
            className="text-sm font-medium text-accent-purple hover:underline"
          >
            Learn about our research →
          </Link>
        </div>
      </ScrollAnimate>
    </Section>
  );
}

"use client";

/**
 * @module trust-progress
 *
 * Visual progress bar showing operator trust level advancement.
 * Trust levels: 1 (newcomer) → 2 (proven) → 3 (trusted) → 4 (core).
 * Thresholds based on consecutive valid block completions.
 */

import { cn } from "@/lib/utils";

interface TrustProgressProps {
  trustLevel: number;
}

const TRUST_LEVELS = [
  { level: 1, label: "Newcomer", threshold: 0, color: "bg-zinc-500" },
  { level: 2, label: "Proven", threshold: 10, color: "bg-blue-500" },
  { level: 3, label: "Trusted", threshold: 100, color: "bg-indigo-500" },
  { level: 4, label: "Core", threshold: 500, color: "bg-amber-500" },
];

export function TrustProgress({ trustLevel }: TrustProgressProps) {
  const currentIdx = TRUST_LEVELS.findIndex((t) => t.level === trustLevel);
  const current = TRUST_LEVELS[Math.max(0, currentIdx)] ?? TRUST_LEVELS[0];

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">
          Trust Level: <span className="text-indigo-400">{current.label}</span>
        </span>
        <span className="text-xs text-muted-foreground">
          Level {trustLevel} / 4
        </span>
      </div>
      <div className="flex gap-1">
        {TRUST_LEVELS.map((t) => (
          <div
            key={t.level}
            className={cn(
              "h-2 flex-1 rounded-full transition-colors",
              trustLevel >= t.level ? t.color : "bg-zinc-800"
            )}
          />
        ))}
      </div>
      <div className="flex justify-between text-[10px] text-muted-foreground">
        {TRUST_LEVELS.map((t) => (
          <span key={t.level}>{t.label}</span>
        ))}
      </div>
    </div>
  );
}

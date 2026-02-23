"use client";

/**
 * @module form-showcase-card
 *
 * Card component for the marketplace grid, showing a search form's
 * active job count, block progress, and completion rate.
 */

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { formLabels, numberWithCommas } from "@/lib/format";
import type { ActiveFormStat } from "@/hooks/use-marketplace";

interface FormShowcaseCardProps {
  stat: ActiveFormStat;
  creditRate?: number;
}

export function FormShowcaseCard({ stat, creditRate }: FormShowcaseCardProps) {
  const label = formLabels[stat.form] ?? stat.form;
  const total = stat.total_blocks ?? 0;
  const completed = stat.completed_blocks ?? 0;
  const pct = total > 0 ? Math.round((completed / total) * 100) : 0;

  return (
    <Card className="bg-zinc-900/50 border-zinc-800">
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-semibold">{label}</CardTitle>
          <Badge variant="secondary" className="text-[10px]">
            {stat.job_count ?? 0} {(stat.job_count ?? 0) === 1 ? "job" : "jobs"}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="flex justify-between text-xs text-muted-foreground">
          <span>{numberWithCommas(completed)} / {numberWithCommas(total)} blocks</span>
          <span>{pct}%</span>
        </div>
        <div className="h-1.5 rounded-full bg-zinc-800 overflow-hidden">
          <div
            className="h-full rounded-full bg-indigo-500 transition-all"
            style={{ width: `${pct}%` }}
          />
        </div>
        {creditRate != null && (
          <div className="text-xs text-muted-foreground">
            <span className="text-emerald-400 font-mono">{creditRate}</span>{" "}
            credits/core-hour
          </div>
        )}
      </CardContent>
    </Card>
  );
}

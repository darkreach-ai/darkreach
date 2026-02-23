"use client";

/**
 * @module earnings/page
 *
 * Earnings dashboard page for operators. Shows monthly earnings chart,
 * summary stats (total credits, blocks completed, trust level), trust
 * progress bar, and paginated credit transaction history.
 *
 * Data fetched from `/api/v1/operators/me/credits` and
 * `/api/v1/operators/me/earnings` (JWT auth required).
 */

import { Coins, Blocks, Shield, TrendingUp } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ViewHeader } from "@/components/view-header";
import { EarningsChart } from "@/components/operators/earnings-chart";
import { EarningsHistoryTable } from "@/components/operators/earnings-history-table";
import { TrustProgress } from "@/components/operators/trust-progress";
import { useEarnings } from "@/hooks/use-earnings";
import { useOperatorStats } from "@/hooks/use-operator-resources";
import { formatCredits, numberWithCommas } from "@/lib/format";

export default function EarningsPage() {
  const { credits, earnings, loading: earningsLoading } = useEarnings(200);
  const { stats, loading: statsLoading } = useOperatorStats();

  const loading = earningsLoading || statsLoading;
  const totalCredits = stats?.credit ?? 0;
  const trustLevel = stats?.trust_level ?? 1;
  const rank = stats?.rank;

  // Sum block count from the last 12 months of earnings
  const totalBlocks = earnings.reduce(
    (sum, e) => sum + (e.block_count ?? 0),
    0
  );

  return (
    <div className="space-y-6">
      <ViewHeader
        title="Earnings"
        subtitle="Track your credit earnings and trust level progression"
      />

      {/* Monthly earnings chart */}
      <Card className="bg-zinc-900/50 border-zinc-800">
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <TrendingUp className="h-4 w-4 text-indigo-400" />
            Monthly Earnings
          </CardTitle>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="h-[200px] animate-pulse bg-zinc-800/50 rounded" />
          ) : (
            <EarningsChart earnings={earnings} />
          )}
        </CardContent>
      </Card>

      {/* Stats row */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
        <Card className="bg-zinc-900/50 border-zinc-800">
          <CardContent className="pt-4 pb-4">
            <div className="flex items-center gap-3">
              <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-emerald-500/10">
                <Coins className="h-4.5 w-4.5 text-emerald-400" />
              </div>
              <div>
                <p className="text-xs text-muted-foreground">Total Credits</p>
                <p className="text-lg font-semibold font-mono tabular-nums">
                  {loading ? "—" : formatCredits(totalCredits)}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
        <Card className="bg-zinc-900/50 border-zinc-800">
          <CardContent className="pt-4 pb-4">
            <div className="flex items-center gap-3">
              <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-indigo-500/10">
                <Blocks className="h-4.5 w-4.5 text-indigo-400" />
              </div>
              <div>
                <p className="text-xs text-muted-foreground">
                  Blocks Completed (12mo)
                </p>
                <p className="text-lg font-semibold font-mono tabular-nums">
                  {loading ? "—" : numberWithCommas(totalBlocks)}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
        <Card className="bg-zinc-900/50 border-zinc-800">
          <CardContent className="pt-4 pb-4">
            <div className="flex items-center gap-3">
              <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-amber-500/10">
                <Shield className="h-4.5 w-4.5 text-amber-400" />
              </div>
              <div>
                <p className="text-xs text-muted-foreground">Rank</p>
                <p className="text-lg font-semibold font-mono tabular-nums">
                  {loading || rank == null ? "—" : `#${rank}`}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Trust progress */}
      <Card className="bg-zinc-900/50 border-zinc-800">
        <CardContent className="pt-4 pb-4">
          {loading ? (
            <div className="h-12 animate-pulse bg-zinc-800/50 rounded" />
          ) : (
            <TrustProgress trustLevel={trustLevel} />
          )}
        </CardContent>
      </Card>

      {/* Credit history table */}
      <Card className="bg-zinc-900/50 border-zinc-800">
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <Coins className="h-4 w-4 text-indigo-400" />
            Credit History
          </CardTitle>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="h-32 animate-pulse bg-zinc-800/50 rounded" />
          ) : (
            <EarningsHistoryTable credits={credits} />
          )}
        </CardContent>
      </Card>
    </div>
  );
}

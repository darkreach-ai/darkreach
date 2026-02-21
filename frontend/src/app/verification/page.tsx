"use client";

/**
 * @module verification/page
 *
 * Distributed prime verification queue dashboard. Shows:
 * - Queue summary cards: pending, claimed, verified, failed
 * - Quorum progress: primes in queue vs quorum met
 * - Completion rate: verified / total percentage
 * - Auto-refreshes via 30s polling
 */

import { CheckCircle2, Clock, AlertTriangle, Shield, Users, TrendingUp } from "lucide-react";
import { usePrimeVerificationStats } from "@/hooks/use-prime-verification";
import { Card, CardContent } from "@/components/ui/card";
import { ViewHeader } from "@/components/view-header";
import { Skeleton } from "@/components/ui/skeleton";
import { numberWithCommas } from "@/lib/format";

function StatCard({
  label,
  value,
  icon: Icon,
  color,
  loading,
}: {
  label: string;
  value: number;
  icon: React.ComponentType<{ className?: string }>;
  color: string;
  loading: boolean;
}) {
  return (
    <Card className="py-4">
      <CardContent className="p-0 px-4">
        <div className="flex items-center gap-3">
          <div className={`rounded-lg p-2 ${color}`}>
            <Icon className="size-4" />
          </div>
          <div>
            <div className="text-xs text-muted-foreground">{label}</div>
            {loading ? (
              <Skeleton className="h-6 w-16 mt-0.5" />
            ) : (
              <div className="text-xl font-semibold tabular-nums">
                {numberWithCommas(value)}
              </div>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

export default function VerificationPage() {
  const { stats, loading } = usePrimeVerificationStats();

  const totalTasks = (stats?.pending ?? 0) + (stats?.claimed ?? 0) + (stats?.verified ?? 0) + (stats?.failed ?? 0);
  const completionRate = totalTasks > 0
    ? ((stats?.verified ?? 0) / totalTasks * 100)
    : 0;
  const quorumRate = (stats?.total_primes ?? 0) > 0
    ? ((stats?.quorum_met ?? 0) / (stats?.total_primes ?? 1) * 100)
    : 0;

  return (
    <>
      <ViewHeader
        title="Verification"
        subtitle="Distributed prime verification queue status"
        className="mb-6"
      />

      {/* Summary cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
        <StatCard
          label="Pending"
          value={stats?.pending ?? 0}
          icon={Clock}
          color="bg-yellow-500/10 text-yellow-600 dark:text-yellow-400"
          loading={loading}
        />
        <StatCard
          label="Claimed"
          value={stats?.claimed ?? 0}
          icon={Users}
          color="bg-blue-500/10 text-blue-600 dark:text-blue-400"
          loading={loading}
        />
        <StatCard
          label="Verified"
          value={stats?.verified ?? 0}
          icon={CheckCircle2}
          color="bg-green-500/10 text-green-600 dark:text-green-400"
          loading={loading}
        />
        <StatCard
          label="Failed"
          value={stats?.failed ?? 0}
          icon={AlertTriangle}
          color="bg-red-500/10 text-red-600 dark:text-red-400"
          loading={loading}
        />
      </div>

      {/* Progress cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        <Card className="py-4">
          <CardContent className="p-0 px-4 space-y-2">
            <div className="flex items-center gap-2">
              <Shield className="size-4 text-muted-foreground" />
              <span className="text-xs font-medium text-muted-foreground">Quorum Progress</span>
            </div>
            {loading ? (
              <Skeleton className="h-6 w-32" />
            ) : (
              <>
                <div className="text-lg font-semibold tabular-nums">
                  {numberWithCommas(stats?.quorum_met ?? 0)}{" "}
                  <span className="text-sm font-normal text-muted-foreground">
                    / {numberWithCommas(stats?.total_primes ?? 0)} primes
                  </span>
                </div>
                <div className="h-2 bg-muted rounded-full overflow-hidden">
                  <div
                    className="h-full bg-emerald-500 rounded-full transition-all duration-500"
                    style={{ width: `${Math.min(100, quorumRate)}%` }}
                  />
                </div>
                <div className="text-xs text-muted-foreground tabular-nums">
                  {quorumRate.toFixed(1)}% quorum met
                </div>
              </>
            )}
          </CardContent>
        </Card>

        <Card className="py-4">
          <CardContent className="p-0 px-4 space-y-2">
            <div className="flex items-center gap-2">
              <TrendingUp className="size-4 text-muted-foreground" />
              <span className="text-xs font-medium text-muted-foreground">Completion Rate</span>
            </div>
            {loading ? (
              <Skeleton className="h-6 w-32" />
            ) : (
              <>
                <div className="text-lg font-semibold tabular-nums">
                  {completionRate.toFixed(1)}%
                </div>
                <div className="h-2 bg-muted rounded-full overflow-hidden">
                  <div
                    className="h-full bg-indigo-500 rounded-full transition-all duration-500"
                    style={{ width: `${Math.min(100, completionRate)}%` }}
                  />
                </div>
                <div className="text-xs text-muted-foreground tabular-nums">
                  {numberWithCommas(stats?.verified ?? 0)} verified of {numberWithCommas(totalTasks)} tasks
                </div>
              </>
            )}
          </CardContent>
        </Card>

        <Card className="py-4">
          <CardContent className="p-0 px-4 space-y-2">
            <div className="flex items-center gap-2">
              <Users className="size-4 text-muted-foreground" />
              <span className="text-xs font-medium text-muted-foreground">Queue Depth</span>
            </div>
            {loading ? (
              <Skeleton className="h-6 w-32" />
            ) : (
              <>
                <div className="text-lg font-semibold tabular-nums">
                  {numberWithCommas(totalTasks)}
                  <span className="text-sm font-normal text-muted-foreground ml-1">tasks</span>
                </div>
                <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Pending</span>
                    <span className="tabular-nums">{numberWithCommas(stats?.pending ?? 0)}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Claimed</span>
                    <span className="tabular-nums">{numberWithCommas(stats?.claimed ?? 0)}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Verified</span>
                    <span className="tabular-nums">{numberWithCommas(stats?.verified ?? 0)}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Failed</span>
                    <span className="tabular-nums">{numberWithCommas(stats?.failed ?? 0)}</span>
                  </div>
                </div>
              </>
            )}
          </CardContent>
        </Card>
      </div>

      <div className="text-xs text-muted-foreground text-center">
        Auto-refreshes every 30 seconds
      </div>
    </>
  );
}

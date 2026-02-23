"use client";

/**
 * @module contribute/page
 *
 * Browser-based compute contribution page. Operators can run prime searches
 * directly in their browser tab using a WASM-accelerated Web Worker (with
 * JS BigInt fallback). The page shows session stats (time, candidates tested,
 * primes found, speed, blocks/hour), a start/stop control panel with WASM/JS
 * engine badge and block progress bar, and a scrollable activity log.
 *
 * Data flows through the {@link ContributeProvider} context which manages
 * the Web Worker lifecycle and REST API communication.
 */

import { useEffect, useState } from "react";
import {
  Activity,
  AlertTriangle,
  Award,
  CheckCircle,
  Clock,
  Coins,
  Cpu,
  Hash,
  Play,
  Search,
  Shield,
  Square,
  Sparkles,
  Zap,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { ViewHeader } from "@/components/view-header";
import {
  ContributeProvider,
  useContribute,
  type ContributeStatus,
  type ActivityEntry,
  type SubmissionFeedback,
} from "@/contexts/worker-context";
import { useContributeProfile } from "@/hooks/use-contribute-profile";
import { numberWithCommas } from "@/lib/format";

function formatElapsed(ms: number): string {
  const totalSec = Math.floor(ms / 1000);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  if (h > 0) return `${h}h ${m}m ${s}s`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

function statusColor(status: ContributeStatus): string {
  switch (status) {
    case "idle":
      return "bg-zinc-500/15 text-zinc-400 border-zinc-500/20";
    case "claiming":
      return "bg-amber-500/15 text-amber-400 border-amber-500/20";
    case "running":
      return "bg-emerald-500/15 text-emerald-400 border-emerald-500/20";
    case "submitting":
      return "bg-indigo-500/15 text-indigo-400 border-indigo-500/20";
    case "paused":
      return "bg-zinc-500/15 text-zinc-400 border-zinc-500/20";
    case "error":
      return "bg-red-500/15 text-red-400 border-red-500/20";
  }
}

function statusLabel(status: ContributeStatus): string {
  switch (status) {
    case "idle":
      return "Idle";
    case "claiming":
      return "Claiming Block";
    case "running":
      return "Computing";
    case "submitting":
      return "Submitting";
    case "paused":
      return "Paused";
    case "error":
      return "Error";
  }
}

function logEntryColor(type: ActivityEntry["type"]): string {
  switch (type) {
    case "claimed":
      return "text-indigo-400";
    case "found":
      return "text-emerald-400";
    case "completed":
      return "text-zinc-300";
    case "error":
      return "text-red-400";
  }
}

function logEntryIcon(type: ActivityEntry["type"]): string {
  switch (type) {
    case "claimed":
      return ">>>";
    case "found":
      return "***";
    case "completed":
      return "===";
    case "error":
      return "!!!";
  }
}

function trustLabel(level: number): string {
  switch (level) {
    case 0:
      return "Untrusted";
    case 1:
      return "New";
    case 2:
      return "Proven";
    case 3:
      return "Trusted";
    case 4:
      return "Core";
    default:
      return `Level ${level}`;
  }
}

function trustColor(level: number): string {
  switch (level) {
    case 0:
      return "bg-red-500/15 text-red-400 border-red-500/20";
    case 1:
      return "bg-zinc-500/15 text-zinc-400 border-zinc-500/20";
    case 2:
      return "bg-cyan-500/15 text-cyan-400 border-cyan-500/20";
    case 3:
      return "bg-emerald-500/15 text-emerald-400 border-emerald-500/20";
    case 4:
      return "bg-amber-500/15 text-amber-400 border-amber-500/20";
    default:
      return "bg-zinc-500/15 text-zinc-400 border-zinc-500/20";
  }
}

function ContributeContent() {
  const { status, stats, log, start, stop } = useContribute();
  const profile = useContributeProfile();
  const [elapsed, setElapsed] = useState(0);

  // Session timer
  useEffect(() => {
    if (!stats.sessionStart) {
      setElapsed(0);
      return;
    }
    setElapsed(Date.now() - stats.sessionStart);
    const interval = setInterval(() => {
      setElapsed(Date.now() - (stats.sessionStart ?? Date.now()));
    }, 1000);
    return () => clearInterval(interval);
  }, [stats.sessionStart]);

  const isRunning = status !== "idle";

  // Blocks per hour
  const blocksPerHour =
    elapsed > 0 && stats.blocksCompleted > 0
      ? (stats.blocksCompleted / (elapsed / 3_600_000)).toFixed(1)
      : "--";

  return (
    <div className="space-y-6">
      <ViewHeader
        title="Contribute"
        subtitle="Run prime searches in your browser"
      />

      {/* Stats cards row */}
      <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-4">
        <Card className="bg-zinc-900/50 border-zinc-800">
          <CardContent className="pt-4 pb-4">
            <div className="flex items-center gap-3">
              <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-indigo-500/10">
                <Clock className="h-4.5 w-4.5 text-indigo-400" />
              </div>
              <div>
                <p className="text-xs text-muted-foreground">Session Time</p>
                <p className="text-lg font-semibold font-mono tabular-nums">
                  {stats.sessionStart ? formatElapsed(elapsed) : "--:--"}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-zinc-900/50 border-zinc-800">
          <CardContent className="pt-4 pb-4">
            <div className="flex items-center gap-3">
              <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-cyan-500/10">
                <Hash className="h-4.5 w-4.5 text-cyan-400" />
              </div>
              <div>
                <p className="text-xs text-muted-foreground">
                  Candidates Tested
                </p>
                <p className="text-lg font-semibold font-mono tabular-nums">
                  {numberWithCommas(stats.tested)}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-zinc-900/50 border-zinc-800">
          <CardContent className="pt-4 pb-4">
            <div className="flex items-center gap-3">
              <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-emerald-500/10">
                <Sparkles className="h-4.5 w-4.5 text-emerald-400" />
              </div>
              <div>
                <p className="text-xs text-muted-foreground">Primes Found</p>
                <p className="text-lg font-semibold font-mono tabular-nums">
                  {numberWithCommas(stats.found)}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-zinc-900/50 border-zinc-800">
          <CardContent className="pt-4 pb-4">
            <div className="flex items-center gap-3">
              <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-amber-500/10">
                <Zap className="h-4.5 w-4.5 text-amber-400" />
              </div>
              <div>
                <p className="text-xs text-muted-foreground">Speed</p>
                <p className="text-lg font-semibold font-mono tabular-nums">
                  {stats.speed > 0
                    ? `${numberWithCommas(stats.speed)}/s`
                    : "--"}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-zinc-900/50 border-zinc-800">
          <CardContent className="pt-4 pb-4">
            <div className="flex items-center gap-3">
              <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-violet-500/10">
                <Activity className="h-4.5 w-4.5 text-violet-400" />
              </div>
              <div>
                <p className="text-xs text-muted-foreground">Blocks/Hour</p>
                <p className="text-lg font-semibold font-mono tabular-nums">
                  {blocksPerHour}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-zinc-900/50 border-zinc-800">
          <CardContent className="pt-4 pb-4">
            <div className="flex items-center gap-3">
              <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-yellow-500/10">
                <Coins className="h-4.5 w-4.5 text-yellow-400" />
              </div>
              <div>
                <p className="text-xs text-muted-foreground">Credits Earned</p>
                <p className="text-lg font-semibold font-mono tabular-nums">
                  {profile.loading
                    ? "--"
                    : numberWithCommas(profile.totalCredits)}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Control panel */}
      <Card className="bg-zinc-900/50 border-zinc-800">
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <Cpu className="h-4 w-4 text-indigo-400" />
            Control Panel
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <Badge variant="outline" className={statusColor(status)}>
                {statusLabel(status)}
              </Badge>
              {stats.mode && (
                <Badge
                  variant="outline"
                  className={
                    stats.mode === "wasm"
                      ? "bg-violet-500/15 text-violet-400 border-violet-500/20"
                      : "bg-zinc-500/15 text-zinc-400 border-zinc-500/20"
                  }
                >
                  {stats.mode === "wasm" ? "WASM" : "JS"}
                </Badge>
              )}
              {!profile.loading && (
                <Badge
                  variant="outline"
                  className={trustColor(profile.trust.trust_level)}
                >
                  <Shield className="h-3 w-3 mr-1" />
                  Trust: {trustLabel(profile.trust.trust_level)}
                </Badge>
              )}
              {stats.currentBlockId != null && (
                <span className="text-sm text-muted-foreground">
                  Block #{stats.currentBlockId}
                  {stats.searchType && (
                    <span className="text-indigo-400 ml-1">
                      ({stats.searchType})
                    </span>
                  )}
                </span>
              )}
              {stats.blocksCompleted > 0 && (
                <span className="text-sm text-muted-foreground">
                  {stats.blocksCompleted} block
                  {stats.blocksCompleted !== 1 ? "s" : ""} completed
                </span>
              )}
            </div>

            <Button
              onClick={isRunning ? stop : start}
              variant={isRunning ? "destructive" : "default"}
              size="sm"
              className={
                isRunning
                  ? ""
                  : "bg-indigo-600 hover:bg-indigo-700 text-white"
              }
            >
              {isRunning ? (
                <>
                  <Square className="h-4 w-4 mr-1.5" />
                  Stop
                </>
              ) : (
                <>
                  <Play className="h-4 w-4 mr-1.5" />
                  Start Contributing
                </>
              )}
            </Button>
          </div>

          {/* Block progress bar */}
          {stats.blockProgress != null && stats.blockProgress > 0 && (
            <div className="w-full h-1 bg-zinc-800 rounded-full overflow-hidden">
              <div
                className="h-full bg-indigo-500 rounded-full transition-all duration-300"
                style={{ width: `${Math.round(stats.blockProgress * 100)}%` }}
              />
            </div>
          )}

          {stats.error && (
            <p className="text-sm text-red-400">{stats.error}</p>
          )}

          <p className="text-xs text-muted-foreground">
            Your browser will claim small work blocks from the coordinator and
            test candidates using{" "}
            {stats.mode === "wasm"
              ? "compiled Rust WASM"
              : "trial division + Miller-Rabin"}
            . Credits are earned at 50% of native worker rates. Leave this tab
            open to keep contributing.
          </p>
        </CardContent>
      </Card>

      {/* Last submission feedback */}
      {stats.lastSubmission && (
        <Card className="bg-zinc-900/50 border-zinc-800">
          <CardContent className="pt-4 pb-4">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                {stats.lastSubmission.hashVerified ? (
                  <CheckCircle className="h-4 w-4 text-emerald-400" />
                ) : stats.lastSubmission.warnings.length > 0 ? (
                  <AlertTriangle className="h-4 w-4 text-amber-400" />
                ) : (
                  <CheckCircle className="h-4 w-4 text-zinc-400" />
                )}
                <span className="text-sm text-zinc-300">
                  Last submission
                </span>
                <Badge
                  variant="outline"
                  className={
                    stats.lastSubmission.hashVerified
                      ? "bg-emerald-500/15 text-emerald-400 border-emerald-500/20"
                      : "bg-zinc-500/15 text-zinc-400 border-zinc-500/20"
                  }
                >
                  {stats.lastSubmission.hashVerified
                    ? "Hash verified"
                    : "No hash"}
                </Badge>
              </div>
              <div className="flex items-center gap-4 text-sm">
                <span className="text-yellow-400 font-mono">
                  +{numberWithCommas(stats.lastSubmission.creditsEarned)} credits
                </span>
                <Badge
                  variant="outline"
                  className={trustColor(stats.lastSubmission.trustLevel)}
                >
                  Trust: {trustLabel(stats.lastSubmission.trustLevel)}
                </Badge>
                {stats.lastSubmission.badgesEarned > 0 && (
                  <Badge
                    variant="outline"
                    className="bg-amber-500/15 text-amber-400 border-amber-500/20"
                  >
                    {stats.lastSubmission.badgesEarned} new badge
                    {stats.lastSubmission.badgesEarned !== 1 ? "s" : ""}!
                  </Badge>
                )}
              </div>
            </div>
            {stats.lastSubmission.warnings.length > 0 && (
              <div className="mt-2 space-y-1">
                {stats.lastSubmission.warnings.map((w, i) => (
                  <p key={i} className="text-xs text-amber-400">
                    {w}
                  </p>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {/* Badges section */}
      {!profile.loading && (profile.badges.length > 0 || profile.nextBadge) && (
        <Card className="bg-zinc-900/50 border-zinc-800">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <Award className="h-4 w-4 text-amber-400" />
              Badges
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {/* Earned badges */}
            {profile.badges.length > 0 && (
              <TooltipProvider>
                <div className="flex flex-wrap gap-2">
                  {profile.badges.map((badge) => {
                    const def = profile.badgeDefinitions.find(
                      (d) => d.id === badge.badge_id
                    );
                    return (
                      <Tooltip key={badge.badge_id}>
                        <TooltipTrigger>
                          <Badge
                            variant="outline"
                            className="bg-amber-500/15 text-amber-400 border-amber-500/20"
                          >
                            {def?.icon || "🏅"} {def?.name || badge.badge_id}
                          </Badge>
                        </TooltipTrigger>
                        <TooltipContent>
                          <p>{def?.description || "Badge earned"}</p>
                          <p className="text-xs text-muted-foreground">
                            Earned{" "}
                            {new Date(badge.earned_at).toLocaleDateString()}
                          </p>
                        </TooltipContent>
                      </Tooltip>
                    );
                  })}
                </div>
              </TooltipProvider>
            )}

            {/* Next badge progress */}
            {profile.nextBadge && (
              <div className="space-y-1.5">
                <div className="flex items-center justify-between text-xs">
                  <span className="text-muted-foreground">
                    Next: {profile.nextBadge.definition.icon || "🏅"}{" "}
                    {profile.nextBadge.definition.name}
                  </span>
                  <span className="text-muted-foreground font-mono">
                    {Math.round(profile.nextBadge.progress * 100)}%
                  </span>
                </div>
                <div className="w-full h-1.5 bg-zinc-800 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-amber-500 rounded-full transition-all duration-500"
                    style={{
                      width: `${Math.round(profile.nextBadge.progress * 100)}%`,
                    }}
                  />
                </div>
                <p className="text-xs text-muted-foreground">
                  {profile.nextBadge.definition.description}
                </p>
              </div>
            )}

            {profile.badges.length === 0 && !profile.nextBadge && (
              <p className="text-sm text-muted-foreground py-2 text-center">
                Complete blocks to earn badges.
              </p>
            )}
          </CardContent>
        </Card>
      )}

      {/* Activity log */}
      <Card className="bg-zinc-900/50 border-zinc-800">
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <Search className="h-4 w-4 text-indigo-400" />
            Activity Log
          </CardTitle>
        </CardHeader>
        <CardContent>
          {log.length === 0 ? (
            <p className="text-sm text-muted-foreground py-4 text-center">
              Start contributing to see activity here.
            </p>
          ) : (
            <ScrollArea className="h-[300px]">
              <div className="space-y-1 font-mono text-xs">
                {log.map((entry) => (
                  <div key={entry.id} className="flex gap-2">
                    <span className="text-muted-foreground shrink-0">
                      {new Date(entry.time).toLocaleTimeString()}
                    </span>
                    <span className={`shrink-0 ${logEntryColor(entry.type)}`}>
                      {logEntryIcon(entry.type)}
                    </span>
                    <span className="text-zinc-300 break-all">
                      {entry.message}
                    </span>
                  </div>
                ))}
              </div>
            </ScrollArea>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

export default function ContributePage() {
  return (
    <ContributeProvider>
      <ContributeContent />
    </ContributeProvider>
  );
}

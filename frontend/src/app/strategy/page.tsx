"use client";

/**
 * @module strategy/page
 *
 * Admin-only page for the AI strategy engine. Shows engine status, form
 * scoring table, decision timeline, and configuration panel.
 */

import { Suspense, useState, useMemo, useCallback } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ViewHeader } from "@/components/view-header";
import { EmptyState } from "@/components/empty-state";
import {
  useStrategyStatus,
  useStrategyScores,
  useStrategyDecisions,
  useStrategyConfig,
  updateStrategyConfig,
  triggerStrategyTick,
  overrideDecision,
} from "@/hooks/use-strategy";
import type { FormScore, StrategyDecision } from "@/hooks/use-strategy";
import { useWs } from "@/contexts/websocket-context";
import type { AiEngineDecision, ScoringWeights, WsData } from "@/hooks/use-websocket";
import {
  Brain,
  Play,
  Pause,
  Clock,
  DollarSign,
  Activity,
  ChevronDown,
  ChevronRight,
  RefreshCw,
  Cpu,
  Target,
  CheckCircle2,
  XCircle,
  CircleDot,
} from "lucide-react";

export default function StrategyPage() {
  return (
    <Suspense>
      <StrategyPageInner />
    </Suspense>
  );
}

function StrategyPageInner() {
  const { status, refetch: refetchStatus } = useStrategyStatus();
  const { scores } = useStrategyScores();
  const { decisions } = useStrategyDecisions();
  const { config, refetch: refetchConfig } = useStrategyConfig();
  const { aiEngine } = useWs();
  const [triggering, setTriggering] = useState(false);

  const handleToggle = useCallback(
    async (enabled: boolean) => {
      try {
        await updateStrategyConfig({ enabled });
        toast.success(enabled ? "Strategy engine enabled" : "Strategy engine disabled");
        refetchStatus();
        refetchConfig();
      } catch (e) {
        toast.error(
          e instanceof Error ? e.message : "Failed to toggle engine"
        );
      }
    },
    [refetchStatus, refetchConfig]
  );

  const handleTriggerTick = useCallback(async () => {
    setTriggering(true);
    try {
      const result = await triggerStrategyTick();
      toast.success(`Tick complete: ${result.decisions.length} decisions`);
      refetchStatus();
    } catch (e) {
      toast.error(
        e instanceof Error ? e.message : "Failed to trigger tick"
      );
    } finally {
      setTriggering(false);
    }
  }, [refetchStatus]);

  const actionableDecisions = useMemo(
    () => decisions.filter((d) => d.decision_type !== "no_action"),
    [decisions]
  );

  return (
    <>
      <ViewHeader
        title="Strategy Engine"
        subtitle="Autonomous search form selection and project creation"
        metadata={
          <div className="flex gap-4 text-sm text-muted-foreground">
            <span className="flex items-center gap-1">
              <Brain className="h-4 w-4" />
              {status?.enabled ? "Active" : "Disabled"}
            </span>
            <span className="flex items-center gap-1">
              <Activity className="h-4 w-4" />
              {actionableDecisions.length} actions
            </span>
            <span className="flex items-center gap-1">
              <DollarSign className="h-4 w-4" />$
              {(status?.monthly_spend_usd ?? 0).toFixed(2)} /{" "}
              ${(status?.monthly_budget_usd ?? 0).toFixed(0)}
            </span>
          </div>
        }
        actions={
          <div className="flex gap-2">
            <Button
              size="sm"
              variant="outline"
              onClick={handleTriggerTick}
              disabled={triggering}
            >
              <RefreshCw
                className={`h-4 w-4 mr-1 ${triggering ? "animate-spin" : ""}`}
              />
              Force Tick
            </Button>
          </div>
        }
      />

      <Tabs defaultValue="overview">
        <TabsList>
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="scores">
            Scoring ({scores.length})
          </TabsTrigger>
          <TabsTrigger value="decisions">
            Decisions ({decisions.length})
          </TabsTrigger>
          <TabsTrigger value="ai-engine">
            AI Engine {aiEngine?.tick_count != null && `(${aiEngine.tick_count})`}
          </TabsTrigger>
          <TabsTrigger value="config">Config</TabsTrigger>
        </TabsList>

        {/* ── Overview ──────────────────────────────────── */}
        <TabsContent value="overview" className="space-y-4 mt-4">
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground">
                  Status
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="flex items-center justify-between">
                  <span className="text-2xl font-bold">
                    {status?.enabled ? "On" : "Off"}
                  </span>
                  <Switch
                    checked={status?.enabled ?? false}
                    onCheckedChange={handleToggle}
                  />
                </div>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground">
                  Last Tick
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="flex items-center gap-2">
                  <Clock className="h-4 w-4 text-muted-foreground" />
                  <span className="text-sm">
                    {status?.last_tick
                      ? new Date(status.last_tick).toLocaleString()
                      : "Never"}
                  </span>
                </div>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground">
                  Monthly Spend
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="flex items-center gap-2">
                  <DollarSign className="h-4 w-4 text-muted-foreground" />
                  <span className="text-2xl font-bold">
                    ${(status?.monthly_spend_usd ?? 0).toFixed(2)}
                  </span>
                  <span className="text-sm text-muted-foreground">
                    / ${(status?.monthly_budget_usd ?? 0).toFixed(0)}
                  </span>
                </div>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground">
                  Active Projects
                </CardTitle>
              </CardHeader>
              <CardContent>
                <span className="text-2xl font-bold">
                  — / {status?.max_concurrent_projects ?? 3}
                </span>
              </CardContent>
            </Card>
          </div>

          {/* Top 5 scores preview */}
          {scores.length > 0 && (
            <Card>
              <CardHeader>
                <CardTitle className="text-sm font-medium">
                  Top-Ranked Forms
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="space-y-2">
                  {scores.slice(0, 5).map((s) => (
                    <ScoreBar key={s.form} score={s} />
                  ))}
                </div>
              </CardContent>
            </Card>
          )}
        </TabsContent>

        {/* ── Scoring ──────────────────────────────────── */}
        <TabsContent value="scores" className="space-y-3 mt-4">
          {scores.length === 0 ? (
            <EmptyState message="No scores computed yet. Enable the engine and trigger a tick." />
          ) : (
            <Card>
              <CardContent className="pt-4">
                <div className="overflow-x-auto">
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="border-b text-left text-muted-foreground">
                        <th className="pb-2 pr-4">Rank</th>
                        <th className="pb-2 pr-4">Form</th>
                        <th className="pb-2 pr-4 text-right">Total</th>
                        <th className="pb-2 pr-4 text-right">Record Gap</th>
                        <th className="pb-2 pr-4 text-right">Yield</th>
                        <th className="pb-2 pr-4 text-right">Cost Eff.</th>
                        <th className="pb-2 pr-4 text-right">Coverage</th>
                        <th className="pb-2 text-right">Fleet Fit</th>
                      </tr>
                    </thead>
                    <tbody>
                      {scores.map((s, i) => (
                        <tr
                          key={s.form}
                          className="border-b last:border-0 hover:bg-muted/50"
                        >
                          <td className="py-2 pr-4 text-muted-foreground">
                            {i + 1}
                          </td>
                          <td className="py-2 pr-4 font-medium">{s.form}</td>
                          <td className="py-2 pr-4 text-right font-mono">
                            {s.total.toFixed(3)}
                          </td>
                          <td className="py-2 pr-4 text-right font-mono text-muted-foreground">
                            {s.record_gap.toFixed(2)}
                          </td>
                          <td className="py-2 pr-4 text-right font-mono text-muted-foreground">
                            {s.yield_rate.toFixed(2)}
                          </td>
                          <td className="py-2 pr-4 text-right font-mono text-muted-foreground">
                            {s.cost_efficiency.toFixed(2)}
                          </td>
                          <td className="py-2 pr-4 text-right font-mono text-muted-foreground">
                            {s.coverage_gap.toFixed(2)}
                          </td>
                          <td className="py-2 text-right font-mono text-muted-foreground">
                            {s.fleet_fit.toFixed(2)}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </CardContent>
            </Card>
          )}
        </TabsContent>

        {/* ── Decisions ─────────────────────────────────── */}
        <TabsContent value="decisions" className="space-y-3 mt-4">
          {decisions.length === 0 ? (
            <EmptyState message="No decisions yet. Enable the engine to start." />
          ) : (
            decisions.map((d) => <DecisionCard key={d.id} decision={d} />)
          )}
        </TabsContent>

        {/* ── AI Engine ─────────────────────────────────── */}
        <TabsContent value="ai-engine" className="space-y-4 mt-4">
          <AiEngineTab aiEngine={aiEngine} />
        </TabsContent>

        {/* ── Config ────────────────────────────────────── */}
        <TabsContent value="config" className="space-y-4 mt-4">
          {config && <ConfigPanel config={config} onUpdate={refetchConfig} />}
        </TabsContent>
      </Tabs>
    </>
  );
}

// ── Sub-components ──────────────────────────────────────────────

function ScoreBar({ score }: { score: FormScore }) {
  const pct = Math.min(score.total * 100, 100);
  return (
    <div className="flex items-center gap-3">
      <span className="w-28 text-sm font-medium truncate">{score.form}</span>
      <div className="flex-1 h-2 rounded bg-muted overflow-hidden">
        <div
          className="h-full rounded bg-indigo-500 transition-all"
          style={{ width: `${pct}%` }}
        />
      </div>
      <span className="w-12 text-right text-sm font-mono text-muted-foreground">
        {score.total.toFixed(2)}
      </span>
    </div>
  );
}

function DecisionCard({ decision }: { decision: StrategyDecision }) {
  const [expanded, setExpanded] = useState(false);
  const [overriding, setOverriding] = useState(false);

  const typeColor: Record<string, string> = {
    create_project: "bg-green-500/10 text-green-700 dark:text-green-400",
    pause_job: "bg-yellow-500/10 text-yellow-700 dark:text-yellow-400",
    verify_result: "bg-blue-500/10 text-blue-700 dark:text-blue-400",
    no_action: "bg-gray-500/10 text-gray-500",
    create_job: "bg-green-500/10 text-green-700 dark:text-green-400",
  };

  const typeIcon: Record<string, React.ReactNode> = {
    create_project: <Play className="h-3 w-3" />,
    pause_job: <Pause className="h-3 w-3" />,
    no_action: <Clock className="h-3 w-3" />,
  };

  const handleOverride = async () => {
    setOverriding(true);
    try {
      await overrideDecision(decision.id, "overridden", "Admin override");
      toast.success("Decision overridden");
    } catch (e) {
      toast.error(
        e instanceof Error ? e.message : "Failed to override"
      );
    } finally {
      setOverriding(false);
    }
  };

  return (
    <Card>
      <CardContent className="py-4">
        <div className="flex items-start justify-between gap-4">
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <Badge
                variant="secondary"
                className={typeColor[decision.decision_type] || ""}
              >
                {typeIcon[decision.decision_type]}
                <span className="ml-1">{decision.decision_type.replace(/_/g, " ")}</span>
              </Badge>
              {decision.form && (
                <Badge variant="outline">{decision.form}</Badge>
              )}
              {decision.action_taken === "overridden" && (
                <Badge variant="destructive" className="text-xs">
                  Overridden
                </Badge>
              )}
            </div>
            <p className="text-sm font-medium">{decision.summary}</p>
            <p className="text-xs text-muted-foreground mt-1">
              {new Date(decision.created_at).toLocaleString()}
              {decision.estimated_cost_usd != null &&
                ` · Est. $${decision.estimated_cost_usd.toFixed(2)}`}
            </p>
          </div>
          <div className="flex items-center gap-1">
            {decision.decision_type !== "no_action" &&
              decision.action_taken !== "overridden" && (
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={handleOverride}
                  disabled={overriding}
                >
                  Override
                </Button>
              )}
            <Button
              size="sm"
              variant="ghost"
              onClick={() => setExpanded(!expanded)}
            >
              {expanded ? (
                <ChevronDown className="h-4 w-4" />
              ) : (
                <ChevronRight className="h-4 w-4" />
              )}
            </Button>
          </div>
        </div>
        {expanded && (
          <div className="mt-3 pt-3 border-t text-sm text-muted-foreground whitespace-pre-wrap">
            {decision.reasoning}
            {decision.override_reason && (
              <p className="mt-2 text-destructive">
                Override: {decision.override_reason}
              </p>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function ConfigPanel({
  config,
  onUpdate,
}: {
  config: NonNullable<ReturnType<typeof useStrategyConfig>["config"]>;
  onUpdate: () => void;
}) {
  const [saving, setSaving] = useState(false);
  const [maxProjects, setMaxProjects] = useState(
    config.max_concurrent_projects
  );
  const [monthlyBudget, setMonthlyBudget] = useState(
    config.max_monthly_budget_usd
  );
  const [projectBudget, setProjectBudget] = useState(
    config.max_per_project_budget_usd
  );
  const [minIdle, setMinIdle] = useState(config.min_idle_workers_to_create);
  const [proximity, setProximity] = useState(
    config.record_proximity_threshold
  );
  const [tickInterval, setTickInterval] = useState(config.tick_interval_secs);

  const handleSave = async () => {
    setSaving(true);
    try {
      await updateStrategyConfig({
        max_concurrent_projects: maxProjects,
        max_monthly_budget_usd: monthlyBudget,
        max_per_project_budget_usd: projectBudget,
        min_idle_workers_to_create: minIdle,
        record_proximity_threshold: proximity,
        tick_interval_secs: tickInterval,
      });
      toast.success("Configuration saved");
      onUpdate();
    } catch (e) {
      toast.error(
        e instanceof Error ? e.message : "Failed to save"
      );
    } finally {
      setSaving(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-sm font-medium">Engine Configuration</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="grid gap-4 md:grid-cols-2">
          <div>
            <label className="text-sm font-medium text-muted-foreground">
              Max Concurrent Projects
            </label>
            <input
              type="number"
              className="mt-1 block w-full rounded-md border bg-background px-3 py-2 text-sm"
              value={maxProjects}
              onChange={(e) => setMaxProjects(Number(e.target.value))}
              min={1}
              max={10}
            />
          </div>
          <div>
            <label className="text-sm font-medium text-muted-foreground">
              Monthly Budget (USD)
            </label>
            <input
              type="number"
              className="mt-1 block w-full rounded-md border bg-background px-3 py-2 text-sm"
              value={monthlyBudget}
              onChange={(e) => setMonthlyBudget(Number(e.target.value))}
              min={0}
              step={10}
            />
          </div>
          <div>
            <label className="text-sm font-medium text-muted-foreground">
              Per-Project Budget (USD)
            </label>
            <input
              type="number"
              className="mt-1 block w-full rounded-md border bg-background px-3 py-2 text-sm"
              value={projectBudget}
              onChange={(e) => setProjectBudget(Number(e.target.value))}
              min={0}
              step={5}
            />
          </div>
          <div>
            <label className="text-sm font-medium text-muted-foreground">
              Min Idle Workers to Create
            </label>
            <input
              type="number"
              className="mt-1 block w-full rounded-md border bg-background px-3 py-2 text-sm"
              value={minIdle}
              onChange={(e) => setMinIdle(Number(e.target.value))}
              min={0}
              max={100}
            />
          </div>
          <div>
            <label className="text-sm font-medium text-muted-foreground">
              Record Proximity Threshold
            </label>
            <input
              type="number"
              className="mt-1 block w-full rounded-md border bg-background px-3 py-2 text-sm"
              value={proximity}
              onChange={(e) => setProximity(Number(e.target.value))}
              min={0}
              max={1}
              step={0.01}
            />
          </div>
          <div>
            <label className="text-sm font-medium text-muted-foreground">
              Tick Interval (seconds)
            </label>
            <input
              type="number"
              className="mt-1 block w-full rounded-md border bg-background px-3 py-2 text-sm"
              value={tickInterval}
              onChange={(e) => setTickInterval(Number(e.target.value))}
              min={60}
              step={60}
            />
          </div>
        </div>
        <div className="mt-4 flex justify-between items-center">
          <div className="text-xs text-muted-foreground">
            Preferred: {config.preferred_forms.join(", ") || "none"} ·
            Excluded: {config.excluded_forms.join(", ") || "none"}
          </div>
          <Button onClick={handleSave} disabled={saving} size="sm">
            {saving ? "Saving..." : "Save"}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

// ── AI Engine Tab ────────────────────────────────────────────────

function AiEngineTab({ aiEngine }: { aiEngine: WsData["aiEngine"] }) {
  if (!aiEngine) {
    return <EmptyState message="No AI engine data yet. The engine may not have run a tick." />;
  }

  return (
    <div className="space-y-4">
      {/* Status cards */}
      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Tick Count
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2">
              <Cpu className="h-4 w-4 text-muted-foreground" />
              <span className="text-2xl font-bold">
                {aiEngine.tick_count ?? 0}
              </span>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Cost Model Version
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2">
              <Target className="h-4 w-4 text-muted-foreground" />
              <span className="text-2xl font-bold">
                v{aiEngine.cost_model_version ?? 0}
              </span>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Recent Decisions
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2">
              <Activity className="h-4 w-4 text-muted-foreground" />
              <span className="text-2xl font-bold">
                {aiEngine.recent_decisions?.length ?? 0}
              </span>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Scoring weights */}
      {aiEngine.scoring_weights && (
        <WeightsCard weights={aiEngine.scoring_weights} />
      )}

      {/* Recent decisions */}
      {aiEngine.recent_decisions?.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="text-sm font-medium">
              Recent AI Decisions
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {aiEngine.recent_decisions.map((d) => (
              <AiDecisionCard key={d.id} decision={d} />
            ))}
          </CardContent>
        </Card>
      )}
    </div>
  );
}

/** Horizontal bar visualization of the 7-component scoring weights. */
function WeightsCard({ weights }: { weights: ScoringWeights }) {
  const entries: { key: string; label: string; value: number }[] = [
    { key: "record_gap", label: "Record Gap", value: weights.record_gap },
    { key: "yield_rate", label: "Yield Rate", value: weights.yield_rate },
    { key: "cost_efficiency", label: "Cost Efficiency", value: weights.cost_efficiency },
    { key: "opportunity_density", label: "Opportunity", value: weights.opportunity_density },
    { key: "fleet_fit", label: "Fleet Fit", value: weights.fleet_fit },
    { key: "momentum", label: "Momentum", value: weights.momentum },
    { key: "competition", label: "Competition", value: weights.competition },
  ];

  const maxWeight = Math.max(...entries.map((e) => e.value), 0.01);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-sm font-medium">
          Scoring Weights (learned via EWA)
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {entries.map((e) => (
            <div key={e.key} className="flex items-center gap-3">
              <span className="w-28 text-sm text-muted-foreground truncate">
                {e.label}
              </span>
              <div className="flex-1 h-3 rounded bg-muted overflow-hidden">
                <div
                  className="h-full rounded bg-indigo-500 transition-all"
                  style={{ width: `${(e.value / maxWeight) * 100}%` }}
                />
              </div>
              <span className="w-14 text-right text-sm font-mono">
                {(e.value * 100).toFixed(1)}%
              </span>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

/** Individual AI engine decision card with outcome tracking. */
function AiDecisionCard({ decision }: { decision: AiEngineDecision }) {
  const [expanded, setExpanded] = useState(false);

  const typeColor: Record<string, string> = {
    create_project: "bg-green-500/10 text-green-700 dark:text-green-400",
    stall_penalty: "bg-yellow-500/10 text-yellow-700 dark:text-yellow-400",
    request_agent_intel: "bg-blue-500/10 text-blue-700 dark:text-blue-400",
    rebalance_fleet: "bg-purple-500/10 text-purple-700 dark:text-purple-400",
    no_action: "bg-gray-500/10 text-gray-500",
  };

  const outcomeIcon = decision.outcome
    ? decision.outcome.verdict === "success"
      ? <CheckCircle2 className="h-3.5 w-3.5 text-green-500" />
      : <XCircle className="h-3.5 w-3.5 text-red-500" />
    : <CircleDot className="h-3.5 w-3.5 text-muted-foreground" />;

  return (
    <div className="border rounded-lg p-3">
      <div className="flex items-start justify-between gap-4">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <Badge
              variant="secondary"
              className={typeColor[decision.decision_type] || ""}
            >
              {decision.decision_type.replace(/_/g, " ")}
            </Badge>
            {decision.form && (
              <Badge variant="outline">{decision.form}</Badge>
            )}
            {decision.confidence != null && (
              <span className="text-xs text-muted-foreground">
                {(decision.confidence * 100).toFixed(0)}% conf
              </span>
            )}
            <span className="ml-auto flex items-center gap-1 text-xs text-muted-foreground">
              {outcomeIcon}
              {decision.outcome
                ? (decision.outcome.verdict as string)
                : "pending"}
            </span>
          </div>
          <p className="text-sm">{decision.action}</p>
          <p className="text-xs text-muted-foreground mt-1">
            tick #{decision.tick_id} ·{" "}
            {new Date(decision.created_at).toLocaleString()}
          </p>
        </div>
        <Button
          size="sm"
          variant="ghost"
          onClick={() => setExpanded(!expanded)}
        >
          {expanded ? (
            <ChevronDown className="h-4 w-4" />
          ) : (
            <ChevronRight className="h-4 w-4" />
          )}
        </Button>
      </div>
      {expanded && (
        <div className="mt-3 pt-3 border-t space-y-2">
          <p className="text-sm text-muted-foreground whitespace-pre-wrap">
            {decision.reasoning}
          </p>
          {decision.outcome && (
            <div className="text-xs bg-muted/50 rounded p-2 font-mono">
              {JSON.stringify(decision.outcome, null, 2)}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

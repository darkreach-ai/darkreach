"use client";

/**
 * @module network/page
 *
 * Compute nodes monitoring page. Shows machines grouped as collapsible
 * sections, each containing a table of worker nodes with real-time health,
 * throughput, and search progress.
 *
 * Data comes from the WebSocket (fleet heartbeats). Service servers
 * (coordinator, DB, agents) are filtered out — this page is purely
 * about compute capacity.
 */

import { useMemo, useState } from "react";
import { toast } from "sonner";
import { ChevronDown, ChevronRight } from "lucide-react";
import { useWs } from "@/contexts/websocket-context";
import { WorkerDetailDialog } from "@/components/worker-detail-dialog";
import { StatCard } from "@/components/stat-card";
import { EmptyState } from "@/components/empty-state";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { API_BASE, numberWithCommas } from "@/lib/format";
import { ViewHeader } from "@/components/view-header";
import type { ServerInfo, WorkerStatus } from "@/hooks/use-websocket";

type WorkerHealth = "healthy" | "stale" | "offline";

function workerHealth(worker: WorkerStatus): WorkerHealth {
  if (worker.last_heartbeat_secs_ago < 30) return "healthy";
  if (worker.last_heartbeat_secs_ago < 60) return "stale";
  return "offline";
}

function healthDotClass(health: WorkerHealth): string {
  if (health === "healthy") return "bg-green-500";
  if (health === "stale") return "bg-yellow-500";
  return "bg-red-500";
}

function healthPillClass(health: WorkerHealth): string {
  if (health === "healthy") return "border-emerald-500/40 bg-emerald-500/10 text-emerald-400";
  if (health === "stale") return "border-amber-500/40 bg-amber-500/10 text-amber-300";
  return "border-red-500/40 bg-red-500/10 text-red-300";
}

/** Derive machine status from its worker nodes */
function machineStatus(server: ServerInfo, workers: WorkerStatus[]): "online" | "degraded" | "offline" {
  const hostWorkers = workers.filter((w) => server.worker_ids.includes(w.worker_id));
  if (hostWorkers.length === 0) return "offline";
  const allHealthy = hostWorkers.every((w) => workerHealth(w) === "healthy");
  if (allHealthy) return "online";
  const anyHealthy = hostWorkers.some((w) => workerHealth(w) === "healthy");
  return anyHealthy ? "degraded" : "offline";
}

function statusDotClass(status: "online" | "degraded" | "offline"): string {
  if (status === "online") return "bg-green-500";
  if (status === "degraded") return "bg-yellow-500";
  return "bg-red-500";
}

export default function NetworkPage() {
  const { fleet } = useWs();
  const [selectedWorker, setSelectedWorker] = useState<WorkerStatus | null>(null);
  const [workerDetailOpen, setWorkerDetailOpen] = useState(false);
  const [stoppingWorkerId, setStoppingWorkerId] = useState<string | null>(null);
  const [searchFilter, setSearchFilter] = useState("");
  const [healthFilter, setHealthFilter] = useState<"all" | WorkerHealth>("all");
  const [typeFilter, setTypeFilter] = useState<"all" | string>("all");

  const workers = useMemo(() => fleet?.workers ?? [], [fleet]);

  // Compute servers from backend data, falling back to client-side grouping
  const machines = useMemo((): ServerInfo[] => {
    let list: ServerInfo[];
    if (fleet?.servers && fleet.servers.length > 0) {
      list = fleet.servers;
    } else {
      // Fallback: group workers by hostname
      const hostMap = new Map<string, WorkerStatus[]>();
      for (const w of workers) {
        const arr = hostMap.get(w.hostname) ?? [];
        arr.push(w);
        hostMap.set(w.hostname, arr);
      }
      list = Array.from(hostMap.entries()).map(([hostname, hw]) => ({
        hostname,
        role: "compute" as const,
        metrics: hw[0]?.metrics ?? null,
        worker_count: hw.length,
        cores: hw.reduce((s, w) => s + w.cores, 0),
        worker_ids: hw.map((w) => w.worker_id),
        total_tested: hw.reduce((s, w) => s + w.tested, 0),
        total_found: hw.reduce((s, w) => s + w.found, 0),
        uptime_secs: Math.max(...hw.map((w) => w.uptime_secs), 0),
      }));
    }
    // Only compute machines
    return list.filter((s) => s.role === "compute");
  }, [fleet, workers]);

  // Summary stats
  const totalNodes = useMemo(() => machines.reduce((s, m) => s + m.worker_count, 0), [machines]);
  const totalCores = useMemo(() => machines.reduce((s, m) => s + m.cores, 0), [machines]);
  const totalRate = useMemo(() => {
    return workers
      .filter((w) => machines.some((m) => m.worker_ids.includes(w.worker_id)))
      .reduce((acc, w) => {
        if (w.uptime_secs <= 0) return acc;
        return acc + w.tested / w.uptime_secs;
      }, 0);
  }, [workers, machines]);

  // Unique search types for filter dropdown
  const searchTypes = useMemo(() => {
    const types = new Set<string>();
    for (const w of workers) {
      if (w.search_type) types.add(w.search_type);
    }
    return Array.from(types).sort();
  }, [workers]);

  // Filter workers for display
  const filteredWorkerIds = useMemo(() => {
    const query = searchFilter.trim().toLowerCase();
    const set = new Set<string>();
    for (const w of workers) {
      if (healthFilter !== "all" && workerHealth(w) !== healthFilter) continue;
      if (typeFilter !== "all" && w.search_type !== typeFilter) continue;
      if (query && !(
        w.worker_id.toLowerCase().includes(query) ||
        w.hostname.toLowerCase().includes(query) ||
        w.current.toLowerCase().includes(query)
      )) continue;
      set.add(w.worker_id);
    }
    return set;
  }, [workers, healthFilter, typeFilter, searchFilter]);

  // Track which machines are expanded (all by default)
  const [expandedMachines, setExpandedMachines] = useState<Set<string> | null>(null);

  // Initialize expanded set with all machine hostnames on first render with data
  const expanded = useMemo(() => {
    if (expandedMachines !== null) return expandedMachines;
    return new Set(machines.map((m) => m.hostname));
  }, [expandedMachines, machines]);

  function toggleMachine(hostname: string) {
    setExpandedMachines((prev) => {
      const current = prev ?? new Set(machines.map((m) => m.hostname));
      const next = new Set(current);
      if (next.has(hostname)) {
        next.delete(hostname);
      } else {
        next.add(hostname);
      }
      return next;
    });
  }

  async function stopNode(workerId: string) {
    setStoppingWorkerId(workerId);
    try {
      const res = await fetch(`${API_BASE}/api/fleet/workers/${encodeURIComponent(workerId)}/stop`, {
        method: "POST",
      });
      if (!res.ok) {
        const data = await res.json().catch(() => ({}));
        throw new Error(data.error || `HTTP ${res.status}`);
      }
      toast.success(`Stop command sent to ${workerId}`);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to stop node";
      toast.error(message);
    } finally {
      setStoppingWorkerId(null);
    }
  }

  return (
    <>
      <ViewHeader
        title="Network"
        subtitle="Compute nodes powering the distributed search network."
        className="mb-5"
      />

      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-5">
        <StatCard label="Machines" value={numberWithCommas(machines.length)} />
        <StatCard label="Nodes" value={numberWithCommas(totalNodes)} />
        <StatCard label="Cores" value={numberWithCommas(totalCores)} />
        <StatCard label="Throughput" value={`${totalRate.toFixed(1)}/s`} />
      </div>

      {/* Filters */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-2 mb-5">
        <Input
          placeholder="Filter by node, machine, or candidate..."
          value={searchFilter}
          onChange={(e) => setSearchFilter(e.target.value)}
        />
        <Select
          value={healthFilter}
          onValueChange={(v) => setHealthFilter(v as "all" | WorkerHealth)}
        >
          <SelectTrigger>
            <SelectValue placeholder="Health" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All health</SelectItem>
            <SelectItem value="healthy">Healthy</SelectItem>
            <SelectItem value="stale">Stale</SelectItem>
            <SelectItem value="offline">Offline</SelectItem>
          </SelectContent>
        </Select>
        <Select
          value={typeFilter}
          onValueChange={(v) => setTypeFilter(v)}
        >
          <SelectTrigger>
            <SelectValue placeholder="Type" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All types</SelectItem>
            {searchTypes.map((t) => (
              <SelectItem key={t} value={t}>{t.toUpperCase()}</SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {/* Machines */}
      {machines.length === 0 ? (
        <EmptyState message="No compute machines online." />
      ) : (
        <div className="space-y-3">
          {machines.map((machine) => {
            const status = machineStatus(machine, workers);
            const isOpen = expanded.has(machine.hostname);
            const machineWorkers = workers
              .filter((w) => machine.worker_ids.includes(w.worker_id) && filteredWorkerIds.has(w.worker_id))
              .sort((a, b) => a.worker_id.localeCompare(b.worker_id));
            const machineRate = machine.uptime_secs > 0
              ? (machine.total_tested / machine.uptime_secs).toFixed(1)
              : "0.0";

            return (
              <Collapsible
                key={machine.hostname}
                open={isOpen}
                onOpenChange={() => toggleMachine(machine.hostname)}
              >
                <div className="border rounded-md">
                  <CollapsibleTrigger asChild>
                    <button
                      className="w-full flex flex-wrap items-center gap-x-3 gap-y-1 px-4 py-3 text-left hover:bg-muted/30 transition-colors cursor-pointer"
                    >
                      {isOpen ? (
                        <ChevronDown className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                      ) : (
                        <ChevronRight className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                      )}
                      <div className={`w-2.5 h-2.5 rounded-full flex-shrink-0 ${statusDotClass(status)}`} />
                      <span className="font-medium">{machine.hostname}</span>
                      <span className="text-xs text-muted-foreground font-mono">
                        {machine.worker_count} node{machine.worker_count !== 1 ? "s" : ""}
                      </span>
                      <span className="text-xs text-muted-foreground font-mono">
                        {machine.cores} core{machine.cores !== 1 ? "s" : ""}
                      </span>
                      {machine.metrics && (
                        <>
                          <span className="text-xs text-muted-foreground font-mono">
                            CPU {Math.round(machine.metrics.cpu_usage_percent)}%
                          </span>
                          <span className="text-xs text-muted-foreground font-mono">
                            Mem {Math.round(machine.metrics.memory_usage_percent)}%
                          </span>
                        </>
                      )}
                      <span className="text-xs text-muted-foreground font-mono">
                        {machineRate}/s
                      </span>
                    </button>
                  </CollapsibleTrigger>

                  <CollapsibleContent>
                    {machineWorkers.length > 0 ? (
                      <div className="border-t">
                        <Table>
                          <TableHeader>
                            <TableRow>
                              <TableHead className="text-xs font-medium text-muted-foreground">Node</TableHead>
                              <TableHead className="text-xs font-medium text-muted-foreground">Type</TableHead>
                              <TableHead className="text-xs font-medium text-muted-foreground">Health</TableHead>
                              <TableHead className="text-xs font-medium text-muted-foreground text-right">Tested</TableHead>
                              <TableHead className="text-xs font-medium text-muted-foreground text-right">Found</TableHead>
                              <TableHead className="text-xs font-medium text-muted-foreground text-right">Rate</TableHead>
                              <TableHead className="text-xs font-medium text-muted-foreground">Current</TableHead>
                              <TableHead className="text-xs font-medium text-muted-foreground text-right">Actions</TableHead>
                            </TableRow>
                          </TableHeader>
                          <TableBody>
                            {machineWorkers.map((worker) => {
                              const health = workerHealth(worker);
                              const rate = worker.uptime_secs > 0
                                ? (worker.tested / worker.uptime_secs).toFixed(1)
                                : "0.0";
                              return (
                                <TableRow key={worker.worker_id} className="hover:bg-muted/30">
                                  <TableCell className="font-mono text-xs">{worker.worker_id}</TableCell>
                                  <TableCell>
                                    <Badge variant="outline" className="font-mono text-xs">
                                      {worker.search_type}
                                    </Badge>
                                  </TableCell>
                                  <TableCell>
                                    <span className="flex items-center gap-1.5">
                                      <span className={`w-2 h-2 rounded-full inline-block ${healthDotClass(health)}`} />
                                      <span className={`text-xs px-1.5 py-0.5 rounded-full border ${healthPillClass(health)}`}>
                                        {health}
                                      </span>
                                    </span>
                                  </TableCell>
                                  <TableCell className="text-right font-mono text-xs">
                                    {numberWithCommas(worker.tested)}
                                  </TableCell>
                                  <TableCell className="text-right font-mono text-xs">
                                    {numberWithCommas(worker.found)}
                                  </TableCell>
                                  <TableCell className="text-right font-mono text-xs">
                                    {rate}/s
                                  </TableCell>
                                  <TableCell className="font-mono text-xs max-w-[200px] truncate" title={worker.current}>
                                    {worker.current}
                                  </TableCell>
                                  <TableCell className="text-right">
                                    <div className="flex items-center justify-end gap-1">
                                      <Button
                                        size="sm"
                                        variant="outline"
                                        className="h-6 text-xs"
                                        onClick={(e) => {
                                          e.stopPropagation();
                                          setSelectedWorker(worker);
                                          setWorkerDetailOpen(true);
                                        }}
                                      >
                                        Inspect
                                      </Button>
                                      {health === "healthy" && (
                                        <Button
                                          size="sm"
                                          variant="outline"
                                          className="h-6 text-xs text-red-600 hover:text-red-700"
                                          disabled={stoppingWorkerId === worker.worker_id}
                                          onClick={(e) => {
                                            e.stopPropagation();
                                            void stopNode(worker.worker_id);
                                          }}
                                        >
                                          {stoppingWorkerId === worker.worker_id ? "Stopping..." : "Stop"}
                                        </Button>
                                      )}
                                    </div>
                                  </TableCell>
                                </TableRow>
                              );
                            })}
                          </TableBody>
                        </Table>
                      </div>
                    ) : machine.worker_count > 0 ? (
                      <div className="border-t px-4 py-3 text-xs text-muted-foreground">
                        No nodes match current filters.
                      </div>
                    ) : null}
                  </CollapsibleContent>
                </div>
              </Collapsible>
            );
          })}
        </div>
      )}

      <WorkerDetailDialog
        worker={selectedWorker}
        open={workerDetailOpen}
        onOpenChange={(open) => {
          if (!open) {
            setWorkerDetailOpen(false);
            setSelectedWorker(null);
          }
        }}
      />
    </>
  );
}

/**
 * @module node-detail-dialog
 *
 * Modal dialog showing detailed information for a single network node.
 * Displays hostname, health status dot (green/yellow/red based on heartbeat
 * age), throughput rate, hardware metrics bars, search parameters, and
 * checkpoint data.
 *
 * This is the node-oriented version of WorkerDetailDialog, following the
 * naming migration from worker -> node for operator-facing components.
 *
 * @see {@link ../hooks/use-websocket} NodeStatus, HardwareMetrics types
 */

import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { MetricsBar } from "@/components/metrics-bar";
import { JsonBlock } from "@/components/json-block";
import { numberWithCommas, formatUptime } from "@/lib/format";
import type { NodeStatus } from "@/hooks/use-websocket";

function parseJson(value: string): Record<string, unknown> | null {
  try {
    const parsed = JSON.parse(value) as unknown;
    if (!parsed || typeof parsed !== "object") return null;
    return parsed as Record<string, unknown>;
  } catch {
    return null;
  }
}

interface NodeDetailDialogProps {
  node: NodeStatus | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function NodeDetailDialog({ node, open, onOpenChange }: NodeDetailDialogProps) {
  if (!node) {
    return (
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>Node</DialogTitle>
          </DialogHeader>
        </DialogContent>
      </Dialog>
    );
  }

  const throughput = node.uptime_secs > 0
    ? (node.tested / node.uptime_secs).toFixed(1)
    : "0.0";
  const params = parseJson(node.search_params);
  const checkpoint = node.checkpoint ? parseJson(node.checkpoint) : null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <div
              className={`w-2.5 h-2.5 rounded-full flex-shrink-0 ${
                node.last_heartbeat_secs_ago < 30
                  ? "bg-green-500"
                  : node.last_heartbeat_secs_ago < 60
                    ? "bg-yellow-500"
                    : "bg-red-500"
              }`}
            />
            {node.hostname}
          </DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1">Node ID</div>
              <span className="font-mono text-xs">{node.worker_id}</span>
            </div>
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1">Search Type</div>
              <Badge variant="outline">{node.search_type}</Badge>
            </div>
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1">Cores</div>
              <span className="font-semibold">{node.cores}</span>
            </div>
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1">Uptime</div>
              <span>{formatUptime(node.uptime_secs)}</span>
            </div>
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1">Tested</div>
              <span className="font-semibold">{numberWithCommas(node.tested)}</span>
            </div>
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1">Found</div>
              <span className="font-semibold">{node.found}</span>
            </div>
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1">Throughput</div>
              <span>{throughput} candidates/sec</span>
            </div>
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1">Heartbeat</div>
              <span>
                {node.last_heartbeat_secs_ago < 5
                  ? "just now"
                  : `${node.last_heartbeat_secs_ago}s ago`}
              </span>
            </div>
          </div>
          {node.metrics && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-2">Hardware</div>
              <div className="space-y-2">
                <MetricsBar label="CPU" percent={node.metrics.cpu_usage_percent} />
                <MetricsBar
                  label="Memory"
                  percent={node.metrics.memory_usage_percent}
                  detail={`${node.metrics.memory_used_gb} / ${node.metrics.memory_total_gb} GB`}
                />
                <MetricsBar
                  label="Disk"
                  percent={node.metrics.disk_usage_percent}
                  detail={`${node.metrics.disk_used_gb} / ${node.metrics.disk_total_gb} GB`}
                />
                <div className="text-xs text-muted-foreground">
                  Load: {node.metrics.load_avg_1m} / {node.metrics.load_avg_5m} / {node.metrics.load_avg_15m}
                </div>
              </div>
            </div>
          )}
          {node.current && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1">Current candidate</div>
              <div className="font-mono text-xs break-all bg-muted rounded-md p-2">
                {node.current}
              </div>
            </div>
          )}
          {params && <JsonBlock label="Search parameters" data={params} />}
          {checkpoint && <JsonBlock label="Checkpoint" data={checkpoint} />}
        </div>
      </DialogContent>
    </Dialog>
  );
}

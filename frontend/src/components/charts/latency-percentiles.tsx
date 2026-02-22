"use client";

/**
 * @module charts/latency-percentiles
 *
 * Time-series chart showing p50, p95, and p99 latency lines over time.
 * Uses Recharts LineChart with a shared tooltip for all three percentiles.
 */

import {
  LineChart,
  Line,
  ResponsiveContainer,
  XAxis,
  YAxis,
  Tooltip,
  Legend,
  ReferenceLine,
} from "recharts";
import { formatTime } from "@/lib/format";

export interface LatencyPoint {
  ts: string;
  p50: number;
  p95: number;
  p99: number;
  count: number;
}

interface LatencyPercentilesProps {
  data: LatencyPoint[];
  /** Unit label for tooltip (default: "ms") */
  unit?: string;
  /** SLO threshold line value (optional) */
  sloTarget?: number;
  /** Chart height (default: 240) */
  height?: number;
}

export function LatencyPercentiles({
  data,
  unit = "ms",
  sloTarget,
  height = 240,
}: LatencyPercentilesProps) {
  if (data.length === 0) {
    return (
      <div
        className="flex items-center justify-center text-sm text-muted-foreground"
        style={{ height }}
      >
        No latency data available
      </div>
    );
  }

  return (
    <ResponsiveContainer width="100%" height={height}>
      <LineChart data={data}>
        <XAxis
          dataKey="ts"
          tickFormatter={(v) => new Date(v).toLocaleTimeString()}
          fontSize={11}
        />
        <YAxis
          fontSize={11}
          tickFormatter={(v) => `${Math.round(v)}${unit}`}
        />
        <Tooltip
          labelFormatter={(v) => formatTime(v)}
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          formatter={((v: number, name: string) => [
            `${(v ?? 0).toFixed(1)}${unit}`,
            name,
          ]) as any}
          contentStyle={{
            backgroundColor: "var(--card)",
            border: "1px solid var(--border)",
            borderRadius: "6px",
            fontSize: "12px",
          }}
        />
        <Legend
          wrapperStyle={{ fontSize: "11px" }}
        />
        {sloTarget !== undefined && (
          <ReferenceLine
            y={sloTarget}
            stroke="#f87171"
            strokeDasharray="4 4"
            label={{
              value: `SLO ${sloTarget}${unit}`,
              position: "right",
              fill: "#f87171",
              fontSize: 10,
            }}
          />
        )}
        <Line
          type="monotone"
          dataKey="p50"
          name="p50"
          stroke="#34d399"
          strokeWidth={2}
          dot={false} isAnimationActive={false}
        />
        <Line
          type="monotone"
          dataKey="p95"
          name="p95"
          stroke="#fbbf24"
          strokeWidth={2}
          dot={false} isAnimationActive={false}
        />
        <Line
          type="monotone"
          dataKey="p99"
          name="p99"
          stroke="#f87171"
          strokeWidth={1.5}
          dot={false} isAnimationActive={false}
          strokeDasharray="4 2"
        />
      </LineChart>
    </ResponsiveContainer>
  );
}

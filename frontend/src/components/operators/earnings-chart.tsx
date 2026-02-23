"use client";

/**
 * @module earnings-chart
 *
 * Bar chart showing monthly credit earnings over the last 12 months.
 * Uses Recharts BarChart with the same styling conventions as other
 * dashboard charts (dark theme, indigo accent).
 */

import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import type { MonthlyEarning } from "@/hooks/use-earnings";
import { formatCredits } from "@/lib/format";

interface EarningsChartProps {
  earnings: MonthlyEarning[];
}

function formatMonth(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleDateString(undefined, { month: "short", year: "2-digit" });
}

export function EarningsChart({ earnings }: EarningsChartProps) {
  if (earnings.length === 0) {
    return (
      <div className="flex items-center justify-center h-[200px] text-sm text-muted-foreground">
        No earnings data yet
      </div>
    );
  }

  const chartData = earnings.map((e) => ({
    month: formatMonth(e.month),
    credits: e.total_credits ?? 0,
    blocks: e.block_count ?? 0,
  }));

  return (
    <ResponsiveContainer width="100%" height={200}>
      <BarChart data={chartData}>
        <XAxis
          dataKey="month"
          tick={{ fontSize: 11 }}
          interval="preserveStartEnd"
        />
        <YAxis
          tick={{ fontSize: 11 }}
          width={55}
          tickFormatter={(v: number) => formatCredits(v)}
        />
        <Tooltip
          contentStyle={{
            fontSize: 12,
            background: "var(--popover)",
            border: "1px solid var(--border)",
            borderRadius: 6,
          }}
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          formatter={(value: any, name: any) => {
            const v = Number(value);
            if (name === "credits")
              return [formatCredits(v), "Credits"];
            return [v.toLocaleString(), "Blocks"];
          }}
        />
        <Bar
          dataKey="credits"
          fill="#6366f1"
          radius={[3, 3, 0, 0]}
          isAnimationActive={false}
        />
      </BarChart>
    </ResponsiveContainer>
  );
}

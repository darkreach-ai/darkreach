"use client";

/**
 * @module tag-distribution
 *
 * Horizontal bar chart showing the frequency of tags across all primes.
 * Color-coded by tag category to match TagChip colors.
 */

import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from "recharts";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { tagCategoryColor } from "@/components/tag-chip";

interface TagEntry {
  tag: string;
  count: number;
}

interface Props {
  data: TagEntry[];
}

export function TagDistribution({ data }: Props) {
  if (data.length === 0) return null;

  // Take top 20 tags for readability
  const chartData = data.slice(0, 20);

  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-xs font-medium text-muted-foreground">
          Tag distribution
        </CardTitle>
      </CardHeader>
      <CardContent>
        <ResponsiveContainer width="100%" height={Math.max(200, chartData.length * 28)}>
          <BarChart data={chartData} layout="vertical" margin={{ left: 0 }}>
            <XAxis type="number" tick={{ fontSize: 11 }} />
            <YAxis
              type="category"
              dataKey="tag"
              tick={{ fontSize: 11 }}
              width={120}
            />
            <Tooltip
              contentStyle={{
                fontSize: 12,
                background: "var(--popover)",
                border: "1px solid var(--border)",
                borderRadius: 6,
              }}
            />
            <Bar dataKey="count" radius={[0, 4, 4, 0]}>
              {chartData.map((entry) => (
                <Cell key={entry.tag} fill={tagCategoryColor(entry.tag)} />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </CardContent>
    </Card>
  );
}

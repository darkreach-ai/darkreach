"use client";

import { useEffect, useState } from "react";
import { API_BASE } from "@/lib/api";
import { Users, Server, Cpu } from "lucide-react";

interface OperatorStatsData {
  totalOperators: number;
  activeNodes: number;
  totalCores: number;
}

const FALLBACK: OperatorStatsData = {
  totalOperators: 12,
  activeNodes: 4,
  totalCores: 32,
};

/**
 * Fetches live operator statistics from the API and displays them
 * in a three-column stat grid. Falls back to static placeholder
 * values when the API is unreachable.
 */
export function OperatorStats() {
  const [stats, setStats] = useState<OperatorStatsData>(FALLBACK);
  const [live, setLive] = useState(false);

  useEffect(() => {
    let cancelled = false;

    async function fetchOperatorStats() {
      try {
        const res = await fetch(`${API_BASE}/api/operators/stats`);
        if (cancelled) return;

        if (res.ok) {
          const data = await res.json();
          setStats({
            totalOperators: data.total_operators ?? data.totalOperators ?? FALLBACK.totalOperators,
            activeNodes: data.active_nodes ?? data.activeNodes ?? FALLBACK.activeNodes,
            totalCores: data.total_cores ?? data.totalCores ?? FALLBACK.totalCores,
          });
          setLive(true);
        }
      } catch {
        // Keep fallback values when API is unreachable
      }
    }

    fetchOperatorStats();
    const interval = setInterval(fetchOperatorStats, 30_000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  const items = [
    {
      icon: Users,
      label: "Total Operators",
      value: stats.totalOperators.toLocaleString(),
      accent: "from-indigo-500 to-violet-500",
    },
    {
      icon: Server,
      label: "Active Nodes",
      value: stats.activeNodes.toLocaleString(),
      accent: "from-emerald-500 to-teal-500",
    },
    {
      icon: Cpu,
      label: "Total Cores",
      value: stats.totalCores.toLocaleString(),
      accent: "from-amber-500 to-orange-500",
    },
  ];

  return (
    <div className="rounded-2xl border border-border bg-card p-8">
      <div className="flex items-center gap-2 mb-6">
        <div
          className={`w-2 h-2 rounded-full ${live ? "bg-accent-green pulse-green" : "bg-muted-foreground"}`}
        />
        <span className="text-xs text-muted-foreground font-medium">
          {live ? "Live" : "Cached"}
        </span>
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-6">
        {items.map((item) => (
          <div key={item.label} className="text-center">
            <div className="flex justify-center mb-3">
              <div
                className={`w-10 h-10 rounded-lg bg-gradient-to-br ${item.accent} flex items-center justify-center text-white`}
              >
                <item.icon size={20} />
              </div>
            </div>
            <p
              className="text-3xl sm:text-4xl font-bold text-foreground mb-1"
              style={{ fontVariantNumeric: "tabular-nums" }}
            >
              {item.value}
            </p>
            <p className="text-sm text-muted-foreground">{item.label}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

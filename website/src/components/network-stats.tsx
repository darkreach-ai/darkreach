"use client";

import { useEffect, useState } from "react";
import { API_BASE } from "@/lib/api";

interface Stats {
  nodes: number;
  cores: number;
  uptime: number;
}

const FALLBACK: Stats = { nodes: 4, cores: 32, uptime: 99.9 };

export function NetworkStats() {
  const [stats, setStats] = useState<Stats>(FALLBACK);
  const [live, setLive] = useState(false);

  useEffect(() => {
    let cancelled = false;

    async function fetchStats() {
      try {
        const networkRes = await fetch(`${API_BASE}/api/network`);
        if (cancelled) return;

        if (networkRes.ok) {
          const data = await networkRes.json();
          const workers = data.workers ?? [];
          const activeNodes = workers.filter(
            (w: { status?: string }) => w.status === "active" || w.status === "running"
          ).length;
          const totalCores = workers.reduce(
            (sum: number, w: { cores?: number }) => sum + (w.cores ?? 0),
            0
          );
          setStats({
            nodes: activeNodes || workers.length || FALLBACK.nodes,
            cores: totalCores || FALLBACK.cores,
            uptime: FALLBACK.uptime,
          });
          setLive(true);
        }
      } catch {
        // Keep fallback values
      }
    }

    fetchStats();
    const interval = setInterval(fetchStats, 30_000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  const items = [
    { label: "Active Nodes", value: stats.nodes.toLocaleString() },
    { label: "Total Cores", value: stats.cores.toLocaleString() },
    { label: "Network Uptime", value: `${stats.uptime}%` },
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
      <div className="grid grid-cols-3 gap-6">
        {items.map((item) => (
          <div key={item.label} className="text-center">
            <p className="text-3xl sm:text-4xl font-bold text-foreground mb-1" style={{ fontVariantNumeric: "tabular-nums" }}>
              {item.value}
            </p>
            <p className="text-sm text-muted-foreground">{item.label}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

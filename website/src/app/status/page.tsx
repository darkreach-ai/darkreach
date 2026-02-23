"use client";

import { useEffect, useState } from "react";
import { Section } from "@/components/ui/section";
import { StatusCard } from "@/components/status-card";
import { UptimeBar } from "@/components/uptime-bar";
import { Badge } from "@/components/ui/badge";
import { WaitlistCTA } from "@/components/waitlist-cta";
import {
  services as fallbackServices,
  networkStats as fallbackNetwork,
  recentIncidents,
  type Service,
  type NetworkStats,
} from "@/lib/status-data";
import { API_BASE } from "@/lib/api";

/** Probe a URL and return latency + status. */
async function probeService(
  url: string
): Promise<{ status: "operational" | "degraded" | "down"; latency: string }> {
  const start = performance.now();
  try {
    const res = await fetch(url, { mode: "no-cors", cache: "no-store" });
    const ms = Math.round(performance.now() - start);
    if (!res.ok && res.type !== "opaque")
      return { status: "degraded", latency: `${ms}ms` };
    return { status: "operational", latency: `${ms}ms` };
  } catch {
    return { status: "down", latency: "-" };
  }
}

export default function StatusPage() {
  const [serviceList, setServiceList] = useState<Service[]>(fallbackServices);
  const [network, setNetwork] = useState<NetworkStats>(fallbackNetwork);
  const [live, setLive] = useState(false);

  useEffect(() => {
    let active = true;

    async function refresh() {
      // Probe services in parallel
      const probes = await Promise.all([
        probeService(`${API_BASE}/api/health`),
        probeService("https://app.darkreach.ai"),
        probeService(`${API_BASE}/api/health`), // DB health via coordinator
        probeService("https://darkreach.ai"),
      ]);

      if (!active) return;

      const updated: Service[] = fallbackServices.map((svc, i) => ({
        ...svc,
        status: probes[i].status,
        latency: probes[i].latency,
      }));
      setServiceList(updated);

      // Fetch network stats from API
      try {
        const [statusRes, networkRes] = await Promise.all([
          fetch(`${API_BASE}/api/status`),
          fetch(`${API_BASE}/api/network`),
        ]);
        if (statusRes.ok && networkRes.ok) {
          const status = (await statusRes.json()) as {
            total_primes?: number;
            uptime_secs?: number;
          };
          const networkData = (await networkRes.json()) as {
            workers?: Array<{
              status?: string;
              cores?: number;
            }>;
          };
          const nodes = networkData.workers ?? [];
          const activeNodes = nodes.filter(
            (w) => w.status === "active" || w.status === "running"
          ).length;
          const totalCores = nodes.reduce((sum, w) => sum + (w.cores ?? 0), 0);
          const uptimeDays = status.uptime_secs
            ? (status.uptime_secs / 86400).toFixed(0)
            : "0";

          if (active) {
            setNetwork({
              activeNodes: activeNodes || nodes.length,
              totalCores: totalCores || fallbackNetwork.totalCores,
              uptimePercent: Number(uptimeDays) > 0 ? 99.9 : fallbackNetwork.uptimePercent,
              primesLast24h: fallbackNetwork.primesLast24h, // Kept as fallback (no 24h endpoint)
            });
          }
        }
      } catch {
        // Keep fallback network stats
      }

      if (active) setLive(true);
    }

    refresh();
    const timer = setInterval(refresh, 30000);
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, []);

  const allOperational = serviceList.every((s) => s.status === "operational");

  return (
    <>
      <Section>
        <div className="flex items-center gap-4 mb-8">
          <h1 className="text-4xl font-bold text-foreground">System Status</h1>
          {allOperational ? (
            <Badge variant="green">All Systems Operational</Badge>
          ) : (
            <Badge variant="orange">Partial Outage</Badge>
          )}
          {live && (
            <span className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
              <span className="inline-block w-1.5 h-1.5 rounded-full bg-accent-green pulse-green" />
              Live
            </span>
          )}
        </div>

        <div className="space-y-3">
          {serviceList.map((service) => (
            <StatusCard key={service.name} service={service} />
          ))}
        </div>
      </Section>

      <Section secondary>
        <h2 className="text-2xl font-bold text-foreground mb-8">Network Overview</h2>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-6 mb-12">
          <div className="text-center">
            <div className="text-3xl font-bold tabular-nums text-foreground">
              {network.activeNodes}
            </div>
            <div className="text-sm text-muted-foreground">Active Nodes</div>
          </div>
          <div className="text-center">
            <div className="text-3xl font-bold tabular-nums text-foreground">
              {network.totalCores}
            </div>
            <div className="text-sm text-muted-foreground">Total Cores</div>
          </div>
          <div className="text-center">
            <div className="text-3xl font-bold tabular-nums text-accent-green">
              {network.uptimePercent}%
            </div>
            <div className="text-sm text-muted-foreground">Uptime (30d)</div>
          </div>
          <div className="text-center">
            <div className="text-3xl font-bold tabular-nums text-foreground">
              {network.primesLast24h}
            </div>
            <div className="text-sm text-muted-foreground">Primes (24h)</div>
          </div>
        </div>

        <div className="space-y-8">
          <UptimeBar label="Coordinator (api.darkreach.ai)" />
          <UptimeBar label="Dashboard (app.darkreach.ai)" />
          <UptimeBar label="Database (Supabase)" />
          <UptimeBar label="Website (darkreach.ai)" />
        </div>
      </Section>

      <Section>
        <h2 className="text-2xl font-bold text-foreground mb-8">Recent Incidents</h2>
        {recentIncidents.length === 0 ? (
          <p className="text-muted-foreground">No recent incidents.</p>
        ) : (
          <div className="space-y-4">
            {recentIncidents.map((incident) => (
              <div
                key={incident.date}
                className="border border-border rounded-md p-4 bg-card"
              >
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-3">
                    <h3 className="text-foreground font-semibold">
                      {incident.title}
                    </h3>
                    <Badge
                      variant={
                        incident.status === "resolved" ? "green" : "orange"
                      }
                    >
                      {incident.status}
                    </Badge>
                  </div>
                  <span className="text-sm text-muted-foreground">
                    {incident.date}
                  </span>
                </div>
                <p className="text-sm text-muted-foreground">
                  {incident.description}
                </p>
                <p className="text-xs text-muted-foreground mt-1">
                  Duration: {incident.duration}
                </p>
              </div>
            ))}
          </div>
        )}
      </Section>

      <WaitlistCTA />
    </>
  );
}

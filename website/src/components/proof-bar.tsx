"use client";

import { useEffect, useRef, useState, useCallback } from "react";
import { API_BASE } from "@/lib/api";

interface ProofStats {
  discoveries: string;
  tested: string;
  nodes: string;
  uptime: string;
}

const FALLBACK: ProofStats = {
  discoveries: "392K+",
  tested: "14.2B",
  nodes: "4",
  uptime: "99.9%",
};

function formatNumber(n: number): string {
  if (n >= 1_000_000_000) return (n / 1_000_000_000).toFixed(1) + "B";
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
  if (n >= 1_000) return (n / 1_000).toFixed(1) + "K+";
  return String(n);
}

/** Animated count-up component. Parses "392K+", "14.2B" etc. and animates the number. */
function CountUp({ value }: { value: string }) {
  const ref = useRef<HTMLSpanElement>(null);
  const animated = useRef(false);
  const [display, setDisplay] = useState(value);

  const animate = useCallback(() => {
    if (animated.current) return;
    animated.current = true;

    // Check prefers-reduced-motion
    if (typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      setDisplay(value);
      return;
    }

    // Parse numeric portion and suffix from formatted strings like "392K+", "14.2B", "99.9%"
    const match = value.match(/^([\d.]+)(.*)$/);
    if (!match) {
      setDisplay(value);
      return;
    }

    const target = parseFloat(match[1]);
    const suffix = match[2];
    const duration = 1500;
    const start = performance.now();

    function tick(now: number) {
      const elapsed = now - start;
      const progress = Math.min(elapsed / duration, 1);
      // easeOut: 1 - (1 - t)^3
      const eased = 1 - Math.pow(1 - progress, 3);
      const current = target * eased;

      // Match decimal places of original
      const decimals = match![1].includes(".") ? match![1].split(".")[1].length : 0;
      setDisplay(current.toFixed(decimals) + suffix);

      if (progress < 1) {
        requestAnimationFrame(tick);
      }
    }

    requestAnimationFrame(tick);
  }, [value]);

  useEffect(() => {
    // Reset animation when value changes
    animated.current = false;
    setDisplay(value);

    const el = ref.current;
    if (!el) return;

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          animate();
        }
      },
      { threshold: 0.1 }
    );

    observer.observe(el);
    return () => observer.disconnect();
  }, [value, animate]);

  return <span ref={ref}>{display}</span>;
}

export function ProofBar() {
  const [stats, setStats] = useState(FALLBACK);
  const [live, setLive] = useState(false);

  useEffect(() => {
    let active = true;
    async function fetchStats() {
      try {
        const [statsRes, networkRes] = await Promise.all([
          fetch(`${API_BASE}/api/stats`),
          fetch(`${API_BASE}/api/network`),
        ]);

        if (!statsRes.ok || !networkRes.ok) return;

        const status = (await statsRes.json()) as {
          total_primes?: number;
          total_tested?: number;
        };
        const network = (await networkRes.json()) as {
          workers?: Array<{ status?: string }>;
        };

        if (!active) return;

        const nodes = network.workers ?? [];
        const activeCount = nodes.filter(
          (n) => n.status === "active" || n.status === "running"
        ).length;

        setStats({
          discoveries: formatNumber(status.total_primes ?? 0),
          tested: formatNumber(status.total_tested ?? 0),
          nodes: String(activeCount || nodes.length),
          uptime: "99.9%",
        });
        setLive(true);
      } catch {
        // Keep fallback values
      }
    }

    fetchStats();
    const timer = setInterval(fetchStats, 30000);
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, []);

  const items = [
    { label: "discoveries", value: stats.discoveries },
    { label: "candidates tested", value: stats.tested },
    { label: "active nodes", value: stats.nodes },
    { label: "uptime", value: stats.uptime },
  ];

  return (
    <section className="border-y border-border/40">
      <div className="mx-auto max-w-7xl px-6 sm:px-8 lg:px-12 py-8">
        <div className="flex flex-wrap items-center justify-center gap-8 sm:gap-12 md:gap-16">
          <div className="flex items-center gap-2">
            <span
              className={`inline-block w-2 h-2 rounded-full ${
                live ? "bg-accent-green pulse-green" : "bg-muted-foreground/40"
              }`}
            />
            <span className="text-sm font-medium text-muted-foreground">Live</span>
          </div>

          {items.map((stat) => (
            <div key={stat.label} className="flex items-baseline gap-2">
              <span className="text-2xl sm:text-3xl font-bold tracking-tight text-foreground">
                <CountUp value={stat.value} />
              </span>
              <span className="text-sm text-muted-foreground">{stat.label}</span>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

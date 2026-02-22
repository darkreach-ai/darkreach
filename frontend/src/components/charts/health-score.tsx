"use client";

/**
 * @module charts/health-score
 *
 * Radial gauge displaying a composite system health score (0-100).
 * Uses an SVG arc that fills based on the score, with color-coded status.
 */

interface HealthScoreProps {
  /** Score from 0 to 100 */
  score: number;
  /** Status label (e.g., "healthy", "degraded", "critical") */
  status: string;
  /** Gauge size in pixels (default: 140) */
  size?: number;
  /** Additional CSS classes */
  className?: string;
}

function scoreColor(score: number): string {
  if (score >= 80) return "#34d399"; // green
  if (score >= 50) return "#fbbf24"; // amber
  return "#f87171"; // red
}

export function HealthScore({
  score,
  status,
  size = 140,
  className,
}: HealthScoreProps) {
  const radius = (size - 16) / 2;
  const circumference = 2 * Math.PI * radius;
  const clamped = Math.max(0, Math.min(100, score));
  const offset = circumference - (clamped / 100) * circumference;
  const color = scoreColor(clamped);
  const center = size / 2;

  return (
    <div className={`relative flex flex-col items-center ${className ?? ""}`}>
      <svg width={size} height={size} className="-rotate-90">
        {/* Background track */}
        <circle
          cx={center}
          cy={center}
          r={radius}
          fill="none"
          stroke="currentColor"
          strokeWidth={6}
          className="text-border/40"
        />
        {/* Score arc */}
        <circle
          cx={center}
          cy={center}
          r={radius}
          fill="none"
          stroke={color}
          strokeWidth={6}
          strokeLinecap="round"
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          style={{ transition: "stroke-dashoffset 0.6s ease" }}
        />
      </svg>
      {/* Score number overlaid at center */}
      <div
        className="absolute flex flex-col items-center justify-center"
        style={{ width: size, height: size }}
      >
        <div className="flex items-baseline gap-0.5">
          <span
            className="text-2xl font-bold tabular-nums leading-none"
            style={{ color }}
          >
            {clamped}
          </span>
          <span className="text-[10px] font-medium text-muted-foreground/50">/100</span>
        </div>
        <span className="text-[10px] text-muted-foreground capitalize leading-none mt-0.5">
          {status}
        </span>
      </div>
    </div>
  );
}

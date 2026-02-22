"use client";

/**
 * @module charts/sparkline
 *
 * Compact inline line chart for embedding in stat cards and table rows.
 * Renders an SVG polyline with optional gradient fill — no axes, no labels.
 */

interface SparklineProps {
  /** Array of numeric values to plot */
  data: number[];
  /** SVG width in pixels (default: 80) */
  width?: number;
  /** SVG height in pixels (default: 32) */
  height?: number;
  /** Stroke color (default: currentColor) */
  color?: string;
  /** Show gradient fill below the line (default: true) */
  fill?: boolean;
  /** Stroke width (default: 1.5) */
  strokeWidth?: number;
  /** Additional CSS classes */
  className?: string;
}

export function Sparkline({
  data,
  width = 80,
  height = 32,
  color = "currentColor",
  fill = true,
  strokeWidth = 1.5,
  className,
}: SparklineProps) {
  if (data.length < 2) {
    return <svg width={width} height={height} className={className} />;
  }

  const pad = strokeWidth;
  const plotW = width - pad * 2;
  const plotH = height - pad * 2;

  const min = Math.min(...data);
  const max = Math.max(...data);
  const range = max - min || 1;

  const points = data.map((v, i) => {
    const x = pad + (i / (data.length - 1)) * plotW;
    const y = pad + plotH - ((v - min) / range) * plotH;
    return `${x},${y}`;
  });

  const gradientId = `sparkline-grad-${Math.random().toString(36).slice(2, 8)}`;

  const fillPath = fill
    ? `M${pad},${pad + plotH} ${points.join(" ")} L${pad + plotW},${pad + plotH} Z`
    : undefined;

  return (
    <svg
      width={width}
      height={height}
      className={className}
      viewBox={`0 0 ${width} ${height}`}
    >
      {fill && (
        <defs>
          <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={color} stopOpacity={0.3} />
            <stop offset="100%" stopColor={color} stopOpacity={0.02} />
          </linearGradient>
        </defs>
      )}
      {fillPath && (
        <path d={fillPath} fill={`url(#${gradientId})`} />
      )}
      <polyline
        points={points.join(" ")}
        fill="none"
        stroke={color}
        strokeWidth={strokeWidth}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

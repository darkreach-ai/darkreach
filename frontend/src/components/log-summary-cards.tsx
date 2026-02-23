import { Card, CardContent } from "@/components/ui/card";

interface LogSummaryCardsProps {
  error: number;
  warn: number;
  info: number;
  debug: number;
  hours: number;
}

interface LevelCardProps {
  label: string;
  count: number;
  rate: string;
  color: string;
  bgColor: string;
}

function LevelCard({ label, count, rate, color, bgColor }: LevelCardProps) {
  return (
    <Card className={bgColor}>
      <CardContent className="p-3">
        <div className={`text-2xl font-bold tabular-nums ${color}`}>
          {count.toLocaleString()}
        </div>
        <div className={`text-xs font-medium uppercase ${color}`}>{label}</div>
        <div className="text-[11px] text-muted-foreground">{rate}/hr</div>
      </CardContent>
    </Card>
  );
}

export function LogSummaryCards({
  error,
  warn,
  info,
  debug,
  hours,
}: LogSummaryCardsProps) {
  const fmt = (n: number) =>
    hours > 0 ? (n / hours).toFixed(1) : "0";

  return (
    <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
      <LevelCard
        label="Error"
        count={error}
        rate={fmt(error)}
        color="text-red-500"
        bgColor="border-red-500/20"
      />
      <LevelCard
        label="Warn"
        count={warn}
        rate={fmt(warn)}
        color="text-amber-500"
        bgColor="border-amber-500/20"
      />
      <LevelCard
        label="Info"
        count={info}
        rate={fmt(info)}
        color="text-blue-500"
        bgColor="border-blue-500/20"
      />
      <LevelCard
        label="Debug"
        count={debug}
        rate={fmt(debug)}
        color="text-zinc-400"
        bgColor="border-zinc-500/20"
      />
    </div>
  );
}

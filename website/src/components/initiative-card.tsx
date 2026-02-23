import { type LucideIcon, ArrowRight } from "lucide-react";
import Link from "next/link";
import { cn } from "@/lib/cn";

interface InitiativeCardProps {
  title: string;
  description: string;
  status: "active" | "coming-soon";
  stats?: Record<string, string>;
  href?: string;
  icon: LucideIcon;
}

export function InitiativeCard({
  title,
  description,
  status,
  stats,
  href,
  icon: Icon,
}: InitiativeCardProps) {
  const isActive = status === "active";

  const content = (
    <div
      className={cn(
        "rounded-2xl border border-border bg-card p-8 transition-colors",
        isActive && "hover:border-muted-foreground/30",
        !isActive && "opacity-60"
      )}
    >
      <div className="flex items-start justify-between mb-4">
        <div
          className={cn(
            "w-12 h-12 rounded-xl flex items-center justify-center",
            isActive ? "bg-accent-purple/10 text-accent-purple" : "bg-muted text-muted-foreground"
          )}
        >
          <Icon size={24} />
        </div>
        {!isActive && (
          <span className="text-xs font-medium px-2.5 py-1 rounded-full bg-muted text-muted-foreground border border-border">
            Coming Soon
          </span>
        )}
      </div>

      <h3 className="text-xl font-semibold text-foreground mb-2">{title}</h3>
      <p className="text-sm text-muted-foreground leading-relaxed mb-4">{description}</p>

      {isActive && stats && (
        <div className="flex gap-6 mb-4">
          {Object.entries(stats).map(([label, value]) => (
            <div key={label}>
              <p className="text-lg font-bold font-mono text-foreground">{value}</p>
              <p className="text-xs text-muted-foreground">{label}</p>
            </div>
          ))}
        </div>
      )}

      {isActive && href && (
        <span className="inline-flex items-center gap-1 text-sm text-accent-purple font-medium">
          Learn more <ArrowRight size={14} />
        </span>
      )}
    </div>
  );

  if (isActive && href) {
    return <Link href={href}>{content}</Link>;
  }

  return content;
}

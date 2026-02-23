import { cn } from "@/lib/cn";
import type { HTMLAttributes } from "react";

interface CardProps extends HTMLAttributes<HTMLDivElement> {
  hover?: boolean;
}

export function Card({ hover = false, className, children, ...props }: CardProps) {
  return (
    <div
      className={cn(
        "rounded-lg border border-border bg-card p-5",
        hover && "transition-colors hover:border-muted-foreground/30",
        className
      )}
      {...props}
    >
      {children}
    </div>
  );
}

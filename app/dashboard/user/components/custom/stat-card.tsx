import type { LucideIcon } from "lucide-react";
import { TrendingUp, TrendingDown, Minus } from "lucide-react";
import { cn } from "@/lib/utils";

type StatCardProps = {
  label: string;
  value: React.ReactNode;
  /** Optional classes for the value span — downsize long string values (e.g. dates). */
  valueClassName?: string;
  sub?: React.ReactNode;
  /** Percent change vs the comparison period — renders a trend chip under the label. */
  delta?: number;
  /** Caption for the delta chip ("vs yesterday"). */
  deltaLabel?: string;
  icon?: LucideIcon | React.ComponentType<{ className?: string }>;
};

/**
 * Shared stat tile: oversized value beside a divider, label + optional
 * sub-caption / trend chip to its right. Standardizes stat grids across
 * dashboard pages (one chrome, one type scale) — plain feature-page counters
 * and analytics metrics both use it.
 */
export function StatCard({ label, value, valueClassName, sub, delta, deltaLabel, icon: Icon }: StatCardProps) {
  const trend = delta != null && delta !== 0 ? (delta > 0 ? "up" : "down") : "neutral";

  return (
    <div className="flex items-center gap-4 rounded-xl border border-border px-5 py-4">
      <span className={cn("shrink-0 text-4xl font-bold tracking-tight tabular-nums", valueClassName)}>{value}</span>
      <div className="min-w-0 border-l border-border pl-4">
        <p className="flex items-center gap-1.5 text-sm font-medium">
          {Icon && <Icon className="size-3.5 shrink-0 text-muted-foreground" />}
          <span className="truncate">{label}</span>
        </p>
        {sub != null && <p className="mt-0.5 text-xs text-muted-foreground">{sub}</p>}
        {delta != null && (
          <p className="mt-0.5 flex items-center gap-1 text-xs">
            {trend === "up" && <TrendingUp className="size-3 text-success" />}
            {trend === "down" && <TrendingDown className="size-3 text-destructive" />}
            {trend === "neutral" && <Minus className="size-3 text-muted-foreground" />}
            <span
              className={cn(
                trend === "up" && "text-success",
                trend === "down" && "text-destructive",
                trend === "neutral" && "text-muted-foreground",
              )}
            >
              {Math.abs(delta)}%
            </span>
            {deltaLabel && <span className="text-muted-foreground">{deltaLabel}</span>}
          </p>
        )}
      </div>
    </div>
  );
}

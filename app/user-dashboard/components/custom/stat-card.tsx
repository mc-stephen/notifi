import type { LucideIcon } from "lucide-react";

type StatCardProps = {
  label: string;
  value: React.ReactNode;
  sub?: React.ReactNode;
  icon?: LucideIcon;
};

/**
 * Shared stat tile: oversized value beside a divider, label + optional
 * sub-caption stacked to its right. Standardizes stat grids across
 * dashboard pages (one chrome, one type scale).
 */
export function StatCard({ label, value, sub, icon: Icon }: StatCardProps) {
  return (
    <div className="flex items-center gap-4 rounded-xl border border-border px-5 py-4">
      <span className="shrink-0 text-4xl font-bold tracking-tight tabular-nums">{value}</span>
      <div className="min-w-0 border-l border-border pl-4">
        <p className="flex items-center gap-1.5 text-sm font-medium">
          {Icon && <Icon className="size-3.5 shrink-0 text-muted-foreground" />}
          <span className="truncate">{label}</span>
        </p>
        {sub != null && <p className="mt-0.5 text-xs text-muted-foreground">{sub}</p>}
      </div>
    </div>
  );
}

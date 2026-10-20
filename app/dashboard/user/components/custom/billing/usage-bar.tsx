import { usagePercent } from "./billing-display";

type UsageBarProps = {
  label: string;
  used: number;
  limit: number | null;
  unit?: string;
};

/**
 * Label row (description left, percent right) + progress bar.
 * Null cap renders an "Unlimited" line with no bar.
 */
export function UsageBar({ label, used, limit, unit = "" }: UsageBarProps) {
  const pct = usagePercent(used, limit);
  const detail =
    limit === null
      ? `${used.toLocaleString()}${unit ? ` ${unit}` : ""} · Unlimited`
      : `${used.toLocaleString()} of ${limit.toLocaleString()}${unit ? ` ${unit}` : ""} remaining`;

  return (
    <div>
      <div className="flex items-baseline justify-between gap-3 text-sm">
        <span className="text-muted-foreground">{detail}</span>
        {limit !== null && (
          <span className="shrink-0 font-medium tabular-nums">{pct}%</span>
        )}
      </div>
      {limit !== null && (
        <div className="mt-1.5 h-2 overflow-hidden rounded-full bg-muted">
          <div
            className="h-full rounded-full bg-primary transition-[width]"
            style={{ width: `${pct}%` }}
          />
        </div>
      )}
      <span className="sr-only">{label}</span>
    </div>
  );
}

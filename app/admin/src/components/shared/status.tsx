import { cn } from "@/lib/utils";

interface StatusDotProps {
  status: "operational" | "degraded" | "down" | "unknown" | "healthy" | "active" | "inactive";
  className?: string;
}

const statusColors = {
  operational: "bg-success",
  healthy: "bg-success",
  active: "bg-success",
  degraded: "bg-warning",
  inactive: "bg-muted-foreground",
  down: "bg-destructive",
  unknown: "bg-muted-foreground",
};

const pingColors = {
  operational: "bg-success",
  healthy: "bg-success",
  active: "bg-success",
  degraded: "bg-warning",
  inactive: "",
  down: "bg-destructive",
  unknown: "",
};

export function StatusDot({ status, className }: StatusDotProps) {
  const hasPing = ["operational", "healthy", "active", "degraded", "down"].includes(status);

  return (
    <span className={cn("relative flex size-2 shrink-0", className)}>
      {hasPing && (
        <span className={cn("absolute inline-flex size-full animate-ping rounded-full opacity-75", pingColors[status])} />
      )}
      <span className={cn("relative inline-flex size-2 rounded-full", statusColors[status])} />
    </span>
  );
}

interface StatusBadgeProps {
  status: string;
  className?: string;
}

export function StatusBadge({ status, className }: StatusBadgeProps) {
  const colorMap: Record<string, string> = {
    active: "bg-success/10 text-success border-success/20",
    operational: "bg-success/10 text-success border-success/20",
    healthy: "bg-success/10 text-success border-success/20",
    delivered: "bg-success/10 text-success border-success/20",
    sent: "bg-info/10 text-info border-info/20",
    open: "bg-info/10 text-info border-info/20",
    pending: "bg-warning/10 text-warning border-warning/20",
    waiting: "bg-warning/10 text-warning border-warning/20",
    degraded: "bg-warning/10 text-warning border-warning/20",
    restricted: "bg-warning/10 text-warning border-warning/20",
    resolved: "bg-success/10 text-success border-success/20",
    closed: "bg-muted text-muted-foreground border-border",
    cancelled: "bg-muted text-muted-foreground border-border",
    failed: "bg-destructive/10 text-destructive border-destructive/20",
    suspended: "bg-destructive/10 text-destructive border-destructive/20",
    banned: "bg-destructive/10 text-destructive border-destructive/20",
    down: "bg-destructive/10 text-destructive border-destructive/20",
    unknown: "bg-muted text-muted-foreground border-border",
    queued: "bg-muted text-muted-foreground border-border",
    succeeded: "bg-success/10 text-success border-success/20",
    paid: "bg-success/10 text-success border-success/20",
    past_due: "bg-warning/10 text-warning border-warning/20",
    refunded: "bg-info/10 text-info border-info/20",
    success: "bg-success/10 text-success border-success/20",
    failure: "bg-destructive/10 text-destructive border-destructive/20",
  };

  return (
    <span className={cn("inline-flex items-center rounded-md border px-2 py-0.5 text-xs font-medium capitalize", colorMap[status] || "bg-muted text-muted-foreground border-border", className)}>
      {status.replace("_", " ")}
    </span>
  );
}

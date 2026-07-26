import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { NotificationStatus } from "@/lib/types";
import { STATUS_LABELS, STATUS_COLORS } from "@/lib/constants";

type StatusBadgeProps = {
  status: NotificationStatus;
  className?: string;
};

export function StatusBadge({ status, className }: StatusBadgeProps) {
  return (
    <Badge
      variant="outline"
      className={cn("gap-1.5 font-medium", STATUS_COLORS[status], className)}
    >
      <span className="relative flex size-1.5">
        {status === "queued" || status === "processing" || status === "retrying" ? (
          <span className="absolute inline-flex size-full animate-ping rounded-full opacity-75 bg-current" />
        ) : null}
        <span className="relative inline-flex size-1.5 rounded-full bg-current" />
      </span>
      {STATUS_LABELS[status]}
    </Badge>
  );
}

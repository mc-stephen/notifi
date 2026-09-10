import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { NotificationPriority } from "@/lib/types";
import { PRIORITY_LABELS, PRIORITY_COLORS } from "@/lib/constants";

type PriorityBadgeProps = {
  priority: NotificationPriority;
  className?: string;
};

export function PriorityBadge({ priority, className }: PriorityBadgeProps) {
  if (priority === "normal") {
    return null;
  }

  return (
    <Badge
      variant="outline"
      className={cn("font-medium", PRIORITY_COLORS[priority], className)}
    >
      {PRIORITY_LABELS[priority]}
    </Badge>
  );
}

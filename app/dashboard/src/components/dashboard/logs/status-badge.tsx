"use client";

import { cn } from "@/lib/utils";
import type { NotificationStatus } from "@/lib/types";
import { STATUS_LABELS } from "@/lib/constants";

const statusStyles: Record<NotificationStatus, string> = {
  queued: "bg-amber-500/10 text-amber-500 border-amber-500/20",
  sent: "bg-blue-500/10 text-blue-500 border-blue-500/20",
  delivered: "bg-emerald-500/10 text-emerald-500 border-emerald-500/20",
  failed: "bg-red-500/10 text-red-500 border-red-500/20",
};

interface StatusBadgeProps {
  status: NotificationStatus;
}

export function StatusBadge({ status }: StatusBadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-md border px-2 py-0.5 text-xs font-medium",
        statusStyles[status],
      )}
    >
      <span
        className={cn(
          "h-1.5 w-1.5 rounded-full",
          status === "queued" && "bg-amber-500",
          status === "sent" && "bg-blue-500",
          status === "delivered" && "bg-emerald-500",
          status === "failed" && "bg-red-500",
        )}
      />
      {STATUS_LABELS[status]}
    </span>
  );
}

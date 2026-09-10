import { cn } from "@/lib/utils";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";
import type { HealthStatus } from "@/lib/types";
import { HEALTH_LABELS, HEALTH_COLORS } from "@/lib/constants";

type HealthIndicatorProps = {
  status: HealthStatus;
  label?: string;
  className?: string;
  showLabel?: boolean;
};

export function HealthIndicator({ status, label, className, showLabel = false }: HealthIndicatorProps) {
  const displayLabel = label ?? HEALTH_LABELS[status];

  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <span className={cn("inline-flex items-center gap-2", className)}>
            <span className="relative flex size-2">
              {status === "healthy" ? (
                <span className="absolute inline-flex size-full animate-ping rounded-full opacity-75 bg-success" />
              ) : null}
              <span className={cn("relative inline-flex size-2 rounded-full", HEALTH_COLORS[status])} />
            </span>
            {showLabel && <span className="text-sm text-muted-foreground">{displayLabel}</span>}
          </span>
        }
      />
      <TooltipContent>{displayLabel}</TooltipContent>
    </Tooltip>
  );
}

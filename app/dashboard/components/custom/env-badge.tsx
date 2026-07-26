import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { Environment } from "@/lib/types";
import { ENVIRONMENT_LABELS, ENVIRONMENT_COLORS } from "@/lib/constants";

type EnvBadgeProps = {
  environment: Environment;
  className?: string;
};

export function EnvBadge({ environment, className }: EnvBadgeProps) {
  return (
    <Badge
      variant="outline"
      className={cn("gap-1.5 font-mono text-[10px] font-semibold uppercase tracking-wider", ENVIRONMENT_COLORS[environment], className)}
    >
      {ENVIRONMENT_LABELS[environment]}
    </Badge>
  );
}

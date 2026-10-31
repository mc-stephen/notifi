import { Check, Minus } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

type ActiveBadgeProps = {
  active: boolean;
  className?: string;
};

/** Project-state chip: icon + On/Off for the project-level active switch. */
export function ActiveBadge({ active, className }: ActiveBadgeProps) {
  const Icon = active ? Check : Minus;
  return (
    <Badge
      variant="outline"
      className={cn(
        "gap-1 font-mono text-[10px] font-semibold uppercase tracking-wider",
        active
          ? "bg-success/15 text-success border-success/20"
          : "bg-muted text-muted-foreground border-border",
        className,
      )}
    >
      <Icon className="size-2.5" strokeWidth={2.5} />
      {active ? "On" : "Off"}
    </Badge>
  );
}

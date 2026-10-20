import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

type ActiveBadgeProps = {
  active: boolean;
  className?: string;
};

/** Project-state chip: dot + label for the project-level active switch. */
export function ActiveBadge({ active, className }: ActiveBadgeProps) {
  return (
    <Badge
      variant="outline"
      className={cn(
        "font-mono text-[10px] font-semibold uppercase tracking-wider",
        active
          ? "bg-success/15 text-success border-success/20"
          : "bg-muted text-muted-foreground border-border",
        className,
      )}
    >
      {active ? "Active" : "Inactive"}
    </Badge>
  );
}

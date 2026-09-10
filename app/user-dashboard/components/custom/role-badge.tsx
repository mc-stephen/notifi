import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { Role } from "@/lib/types";
import { ROLE_LABELS, ROLE_COLORS } from "@/lib/constants";

type RoleBadgeProps = {
  role: Role;
  className?: string;
};

export function RoleBadge({ role, className }: RoleBadgeProps) {
  return (
    <Badge variant="outline" className={cn("font-medium", ROLE_COLORS[role], className)}>
      {ROLE_LABELS[role]}
    </Badge>
  );
}

import { Card, CardContent } from "@/components/ui/card";
import type { LucideIcon } from "lucide-react";

type StatCardProps = {
  label: string;
  value: React.ReactNode;
  sub?: React.ReactNode;
  icon?: LucideIcon;
};

/**
 * Shared stat tile: label row (optional icon) + big value + optional
 * sub-caption. Standardizes the hand-rolled `Card > CardContent pt-5`
 * stat grids across dashboard pages (one padding, one type scale).
 */
export function StatCard({ label, value, sub, icon: Icon }: StatCardProps) {
  return (
    <Card>
      <CardContent className="pt-5">
        <div className="flex items-center justify-between">
          <span className="text-sm text-muted-foreground">{label}</span>
          {Icon && <Icon className="size-4 text-muted-foreground" />}
        </div>
        <div className="text-2xl font-bold tracking-tight mt-1">{value}</div>
        {sub != null && <p className="text-xs text-muted-foreground mt-1">{sub}</p>}
      </CardContent>
    </Card>
  );
}

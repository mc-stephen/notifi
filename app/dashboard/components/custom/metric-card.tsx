import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { TrendingUp, TrendingDown, Minus } from "lucide-react";

type MetricCardProps = {
  title: string;
  value: string | number;
  change?: number;
  changeLabel?: string;
  icon?: React.ComponentType<{ className?: string }>;
  className?: string;
};

export function MetricCard({ title, value, change, changeLabel, icon: Icon, className }: MetricCardProps) {
  const trend = change != null && change !== 0 ? (change > 0 ? "up" : "down") : "neutral";

  return (
    <Card className={cn("relative overflow-hidden", className)}>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground">{title}</CardTitle>
        {Icon && <Icon className="size-4 text-muted-foreground" />}
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-bold tracking-tight">{value}</div>
        {change != null && (
          <div className="mt-1 flex items-center gap-1 text-xs">
            {trend === "up" && <TrendingUp className="size-3 text-success" />}
            {trend === "down" && <TrendingDown className="size-3 text-destructive" />}
            {trend === "neutral" && <Minus className="size-3 text-muted-foreground" />}
            <span
              className={cn(
                trend === "up" && "text-success",
                trend === "down" && "text-destructive",
                trend === "neutral" && "text-muted-foreground",
              )}
            >
              {Math.abs(change)}%
            </span>
            {changeLabel && <span className="text-muted-foreground">{changeLabel}</span>}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

import { Card } from "@/components/ui/card";
import type { MetricData } from "@/lib/types";
import { ArrowUpRight, ArrowDownRight, Minus } from "lucide-react";

interface MetricCardProps {
  data: MetricData;
}

export function MetricCard({ data }: MetricCardProps) {
  const change = data.change ?? 0;
  const isPositive = change > 0;
  const isNegative = change < 0;
  const isNeutral = change === 0;

  return (
    <Card className="p-4">
      <div className="space-y-2">
        <p className="text-sm text-muted-foreground font-medium">{data.title}</p>
        <div className="flex items-baseline gap-2">
          <p className="text-2xl font-bold text-foreground tracking-tight">{data.value}</p>
          {!isNeutral && (
            <span className={`flex items-center gap-0.5 text-xs font-medium ${isPositive ? "text-success" : "text-destructive"}`}>
              {isPositive ? <ArrowUpRight className="size-3" /> : <ArrowDownRight className="size-3" />}
              {Math.abs(change)}%
            </span>
          )}
          {isNeutral && (
            <span className="flex items-center gap-0.5 text-xs font-medium text-muted-foreground">
              <Minus className="size-3" />
            </span>
          )}
        </div>
        {data.changeLabel && (
          <p className="text-xs text-muted-foreground">{data.changeLabel}</p>
        )}
      </div>
    </Card>
  );
}

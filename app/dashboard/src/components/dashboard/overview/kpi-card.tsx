"use client";

import { TrendingUp, TrendingDown, Send, Clock, CheckCircle2, PlugZap } from "lucide-react";
import { cn } from "@/lib/utils";
import { Card } from "@/components/ui/card";
import type { KpiMetric } from "@/lib/types";

const iconMap: Record<string, React.ElementType> = {
  Send,
  Clock,
  CheckCircle2,
  PlugZap,
};

const bgMap: Record<string, string> = {
  Send: "bg-blue-500/10 text-blue-400",
  Clock: "bg-amber-500/10 text-amber-400",
  CheckCircle2: "bg-emerald-500/10 text-emerald-400",
  PlugZap: "bg-purple-500/10 text-purple-400",
};

interface KpiCardProps {
  metric: KpiMetric;
}

export function KpiCard({ metric }: KpiCardProps) {
  const Icon = iconMap[metric.icon];
  const isUp = metric.change >= 0;

  return (
    <Card className="card-lift min-w-0 overflow-hidden rounded-xl border-border/40">
      <div className={cn("h-1 w-full bg-gradient-to-r", gradientMap[metric.icon] || "from-primary/40 to-primary")} />
      <div className="p-5">
        <div className="flex items-center justify-between">
          <span className="text-xs font-medium text-muted-foreground tracking-wide uppercase">
            {metric.label}
          </span>
          {Icon && (
            <div className={cn("flex h-8 w-8 items-center justify-center rounded-lg", bgMap[metric.icon])}>
              <Icon className="h-4 w-4" />
            </div>
          )}
        </div>
        <div className="mt-3 flex items-baseline gap-3">
          <span className="text-3xl font-semibold tracking-tight">
            {metric.isPercentage
              ? `${metric.value.toFixed(1)}%`
              : metric.label === "Avg Latency"
                ? `${metric.value.toFixed(0)}ms`
                : metric.value.toLocaleString()}
          </span>
          <span
            className={cn(
              "inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium",
              isUp
                ? "bg-emerald-500/10 text-emerald-400"
                : "bg-red-500/10 text-red-400",
            )}
          >
            {isUp ? (
              <TrendingUp className="h-3 w-3" />
            ) : (
              <TrendingDown className="h-3 w-3" />
            )}
            {Math.abs(metric.change).toFixed(1)}%
          </span>
        </div>
        <p className="mt-2 text-[11px] text-muted-foreground">
          vs. previous period
        </p>
      </div>
    </Card>
  );
}

const gradientMap: Record<string, string> = {
  Send: "from-blue-500/40 to-blue-500",
  Clock: "from-amber-500/40 to-amber-500",
  CheckCircle2: "from-emerald-500/40 to-emerald-500",
  PlugZap: "from-purple-500/40 to-purple-500",
};

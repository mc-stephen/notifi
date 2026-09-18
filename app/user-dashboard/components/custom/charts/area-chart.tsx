"use client";

import {
  AreaChart as RechartsAreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip as RechartsTooltip,
  ResponsiveContainer,
} from "recharts";
import { cn } from "@/lib/utils";
import type { ChartDataPoint } from "@/lib/types";
import { useDebouncedSize } from "@/hooks/use-debounced-size";

type AreaChartProps = {
  data: ChartDataPoint[];
  className?: string;
  height?: number | "fill";
  color?: string;
  showGrid?: boolean;
};

export function AreaChart({ data, className, height = 300, color = "var(--chart-1)", showGrid = true }: AreaChartProps) {
  // Explicit pixel dimensions disable ResponsiveContainer's internal
  // per-tick observer (see useDebouncedSize).
  const { ref, width, height: measuredHeight } = useDebouncedSize<HTMLDivElement>();
  const resolvedHeight = height === "fill" ? measuredHeight : height;
  const ready = width > 0 && resolvedHeight > 0;

  return (
    <div ref={ref} className={cn("w-full", height === "fill" && "h-full", className)}>
      {ready ? (
      <ResponsiveContainer width={width} height={resolvedHeight}>
        <RechartsAreaChart data={data} margin={{ top: 5, right: 5, left: -20, bottom: 0 }}>
          {showGrid && (
            <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" vertical={false} />
          )}
          <XAxis
            dataKey="label"
            tick={{ fontSize: 11, fill: "var(--muted-foreground)" }}
            axisLine={false}
            tickLine={false}
          />
          <YAxis
            tick={{ fontSize: 11, fill: "var(--muted-foreground)" }}
            axisLine={false}
            tickLine={false}
          />
          <RechartsTooltip
            contentStyle={{
              backgroundColor: "var(--card)",
              border: "1px solid var(--border)",
              borderRadius: "8px",
              fontSize: "12px",
              color: "var(--card-foreground)",
            }}
            labelStyle={{ color: "var(--muted-foreground)" }}
          />
          <Area
            type="monotone"
            dataKey="value"
            stroke={color}
            fill={color}
            fillOpacity={0.15}
            strokeWidth={2}
            dot={false}
            activeDot={{ r: 4, strokeWidth: 0 }}
          />
        </RechartsAreaChart>
      </ResponsiveContainer>
      ) : (
        <div
          style={height === "fill" ? undefined : { height }}
          className={height === "fill" ? "h-full min-h-[200px]" : undefined}
          aria-hidden="true"
        />
      )}
    </div>
  );
}

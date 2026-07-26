"use client";

import {
  PieChart,
  Pie,
  Cell,
  Tooltip as RechartsTooltip,
  ResponsiveContainer,
  Legend,
} from "recharts";
import { cn } from "@/lib/utils";

type DonutChartProps = {
  data: { name: string; value: number; fill?: string }[];
  className?: string;
  height?: number;
  innerRadius?: number;
  showLegend?: boolean;
};

export function DonutChart({ data, className, height = 300, innerRadius = 60, showLegend = true }: DonutChartProps) {
  const total = data.reduce((sum, d) => sum + d.value, 0);

  return (
    <div className={cn("w-full", className)}>
      <ResponsiveContainer width="100%" height={height}>
        <PieChart>
          <Pie
            data={data}
            cx="50%"
            cy="50%"
            innerRadius={innerRadius}
            outerRadius={innerRadius + 30}
            paddingAngle={3}
            dataKey="value"
            strokeWidth={0}
          >
            {data.map((entry, index) => (
              <Cell key={index} fill={entry.fill ?? "var(--chart-1)"} />
            ))}
          </Pie>
          <RechartsTooltip
            contentStyle={{
              backgroundColor: "var(--card)",
              border: "1px solid var(--border)",
              borderRadius: "8px",
              fontSize: "12px",
              color: "var(--card-foreground)",
            }}
            formatter={(value, name) => [`${Number(value).toLocaleString()} (${((Number(value) / total) * 100).toFixed(1)}%)`, name]}
          />
          {showLegend && (
            <Legend
              verticalAlign="bottom"
              height={36}
              iconType="circle"
              iconSize={8}
              formatter={(value) => <span className="text-xs text-muted-foreground">{value}</span>}
            />
          )}
        </PieChart>
      </ResponsiveContainer>
    </div>
  );
}

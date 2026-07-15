"use client";

import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip } from "recharts";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { CHANNEL_LABELS } from "@/lib/constants";
import type { Channel } from "@/lib/types";

const COLORS = [
  "var(--color-chart-1)",
  "var(--color-chart-2)",
  "var(--color-chart-3)",
  "var(--color-chart-4)",
  "var(--color-chart-5)",
];

interface ChannelBreakdownProps {
  data: { channel: Channel; volume: number; color: string }[];
  loading?: boolean;
}

export function ChannelBreakdown({ data, loading }: ChannelBreakdownProps) {
  const total = data.reduce((s, d) => s + d.volume, 0);

  if (loading) {
    return (
      <Card className="rounded-xl overflow-hidden">
        <CardHeader>
          <Skeleton className="h-5 w-36" />
        </CardHeader>
        <CardContent>
          <Skeleton className="h-[220px] w-full rounded-lg" />
        </CardContent>
      </Card>
    );
  }

  const chartData = data.map((d) => ({
    name: CHANNEL_LABELS[d.channel],
    value: d.volume,
    channel: d.channel,
  }));

  return (
    <Card className="rounded-xl overflow-hidden">
      <CardHeader className="pb-3">
        <CardTitle className="text-base font-semibold">Channel Breakdown</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex items-center gap-6">
          <div className="h-[180px] w-[180px] shrink-0">
            <ResponsiveContainer width="100%" height="100%">
              <PieChart>
                <Pie
                  data={chartData}
                  cx="50%"
                  cy="50%"
                  innerRadius={55}
                  outerRadius={80}
                  paddingAngle={3}
                  dataKey="value"
                  nameKey="name"
                  strokeWidth={0}
                >
                  {chartData.map((_, i) => (
                    <Cell key={i} fill={COLORS[i % COLORS.length]} />
                  ))}
                </Pie>
                <Tooltip
                  content={({ active, payload }) =>
                    active && payload?.length ? (
                      <div className="rounded-xl border border-glass-border bg-glass backdrop-blur-[var(--blur-glass)] px-3 py-2 text-sm shadow-xl">
                        <p className="font-medium">{payload[0].name}</p>
                        <p className="text-muted-foreground">{payload[0].value?.toLocaleString()} notifications</p>
                      </div>
                    ) : null
                  }
                />
              </PieChart>
            </ResponsiveContainer>
          </div>
          <div className="flex flex-col gap-2">
            {chartData.map((d, i) => (
              <div key={d.name} className="flex items-center gap-2.5">
                <div
                  className="h-2.5 w-2.5 shrink-0 rounded-full"
                  style={{ backgroundColor: COLORS[i % COLORS.length] }}
                />
                <span className="text-sm text-muted-foreground flex-1 capitalize">{d.name}</span>
                <span className="text-sm font-medium">{((d.value / total) * 100).toFixed(0)}%</span>
              </div>
            ))}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

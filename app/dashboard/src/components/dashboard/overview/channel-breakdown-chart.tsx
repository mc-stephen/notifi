"use client";

import {
  PieChart,
  Pie,
  Cell,
  ResponsiveContainer,
  Tooltip,
} from "recharts";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import type { ChannelBreakdown } from "@/lib/types";
import { CHANNEL_LABELS } from "@/lib/constants";

interface ChannelBreakdownChartProps {
  data: ChannelBreakdown[];
  loading?: boolean;
}

function CustomTooltip({ active, payload }: any) {
  if (!active || !payload?.length) return null;
  const entry = payload[0].payload;
  return (
    <div className="rounded-lg border border-border bg-popover p-3 shadow-lg">
      <p className="text-sm font-medium">{CHANNEL_LABELS[entry.channel] || entry.channel}</p>
      <p className="text-xs text-muted-foreground">
        {entry.volume.toLocaleString()} ({((entry.volume / 284521) * 100).toFixed(1)}%)
      </p>
    </div>
  );
}

export function ChannelBreakdownChart({ data, loading }: ChannelBreakdownChartProps) {
  if (loading) {
    return (
      <Card>
        <CardHeader>
          <Skeleton className="h-5 w-36" />
        </CardHeader>
        <CardContent>
          <Skeleton className="h-[200px] w-full rounded-full" />
        </CardContent>
      </Card>
    );
  }

  const total = data.reduce((acc, d) => acc + d.volume, 0);

  return (
    <Card>
      <CardHeader>
        <CardTitle>Channel Breakdown</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="h-[200px]">
          <ResponsiveContainer width="100%" height="100%">
            <PieChart>
              <Pie
                data={data}
                cx="50%"
                cy="50%"
                innerRadius={55}
                outerRadius={85}
                paddingAngle={3}
                dataKey="volume"
                animationDuration={800}
              >
                {data.map((entry) => (
                  <Cell key={entry.channel} fill={entry.color} />
                ))}
              </Pie>
              <Tooltip content={<CustomTooltip />} />
            </PieChart>
          </ResponsiveContainer>
        </div>
        <div className="mt-4 flex flex-wrap gap-3">
          {data.map((entry) => (
            <div key={entry.channel} className="flex items-center gap-1.5">
              <div
                className="h-2 w-2 rounded-full"
                style={{ backgroundColor: entry.color }}
              />
              <span className="text-xs text-muted-foreground">
                {CHANNEL_LABELS[entry.channel]} — {((entry.volume / total) * 100).toFixed(0)}%
              </span>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

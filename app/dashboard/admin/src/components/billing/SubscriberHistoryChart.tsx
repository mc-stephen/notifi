// .==================================
// Subscriber history chart (admin /billing -> Subscribers tab).
// Monthly stacked per-plan AreaChart over live points from
// GET /admin/billing/subscribers/history (replayed from
// project births + billing audit events).
// Series keys come from the data so custom plans render too.
// .==================================

import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { SubscriberHistoryPoint } from "@/lib/types";

type Props = {
  points: SubscriberHistoryPoint[];
};

const KNOWN_COLORS: Record<string, string> = {
  free: "#94a3b8",
  starter: "#38bdf8",
  pro: "#6366f1",
  enterprise: "#22c55e",
};

const FALLBACK_COLORS = ["#f59e0b", "#ec4899", "#14b8a6", "#a78bfa", "#f472b6", "#84cc16"];

// Series in stable order (known plans first), each with a color,
// so custom plans render instead of being dropped.
export function seriesColors(points: SubscriberHistoryPoint[]): { key: string; color: string }[] {
  const seen: string[] = [];
  for (const p of points) {
    for (const key of Object.keys(p.byPlan)) {
      if (!seen.includes(key)) seen.push(key);
    }
  }
  const known = Object.keys(KNOWN_COLORS).filter((k) => seen.includes(k));
  const extra = seen.filter((k) => !(k in KNOWN_COLORS)).sort();
  return [...known, ...extra].map((key, i) => ({
    key,
    color: KNOWN_COLORS[key] ?? FALLBACK_COLORS[i % FALLBACK_COLORS.length],
  }));
}

function monthLabel(iso: string, first: boolean) {
  const d = new Date(`${iso}T00:00:00Z`);
  const month = d.toLocaleDateString(undefined, { month: "short" });
  // Year only where the 12-month window crosses it.
  if (first || d.getUTCMonth() === 0) {
    return `${month} ’${String(d.getUTCFullYear()).slice(2)}`;
  }
  return month;
}

function fullLabel(iso: string) {
  const d = new Date(`${iso}T00:00:00Z`);
  return d.toLocaleDateString(undefined, { month: "long", year: "numeric" });
}

export default function SubscriberHistoryChart({ points }: Props) {
  if (points.length === 0) {
    return (
      <div className="px-4 py-12 text-center">
        <p className="text-sm font-medium text-foreground">No subscriber history yet</p>
        <p className="text-xs text-muted-foreground mt-1">History will appear once subscriptions are recorded.</p>
      </div>
    );
  }

  const series = seriesColors(points);
  const data = points.map((p, i) => {
    const row: Record<string, string | number> = {
      label: monthLabel(p.date, i === 0),
      full: fullLabel(p.date),
    };
    for (const { key } of series) {
      row[key] = p.byPlan[key] ?? 0;
    }
    return row;
  });

  return (
    <div className="h-[300px] w-full">
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={data} margin={{ top: 8, right: 8, bottom: 0, left: -12 }}>
          <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
          <XAxis dataKey="label" tickLine={false} axisLine={false} tick={{ fontSize: 11 }} interval={0} />
          <YAxis tickLine={false} axisLine={false} tick={{ fontSize: 11 }} allowDecimals={false} />
          <Tooltip
            labelFormatter={(_, payload) => payload?.[0]?.payload?.full ?? ""}
            contentStyle={{
              borderRadius: 8,
              fontSize: 12,
              border: "1px solid hsl(var(--border))",
              background: "hsl(var(--card))",
            }}
          />
          {series.map(({ key, color }) => (
            <Area
              key={key}
              type="monotone"
              dataKey={key}
              stackId="subscribers"
              stroke={color}
              fill={color}
              fillOpacity={0.22}
              strokeWidth={1.5}
              name={key}
            />
          ))}
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}

// .==================================
// Live subscriber-history panel (admin /billing -> Subscribers).
// Fetches trailing monthly snapshots from
// GET /admin/billing/subscribers/history (replayed from
// project births + billing audit events — no mock).
// Also refreshes the hero delta next to the live total.
// .==================================

import { useCallback, useEffect, useState } from "react";
import { ApiError, billing } from "@/lib/api";
import type { SubscriberHistoryPoint } from "@/lib/types";
import SubscriberHistoryChart, { seriesColors } from "./SubscriberHistoryChart";

function heroDelta(points: SubscriberHistoryPoint[]): { text: string; tone: string } {
  if (points.length < 2) return { text: "across all plans", tone: "text-muted-foreground" };
  const prev = points[points.length - 2].total;
  const last = points[points.length - 1].total;
  if (prev <= 0) return { text: "across all plans", tone: "text-muted-foreground" };
  const pct = Math.round(((last - prev) / prev) * 1000) / 10;
  const sign = pct > 0 ? "+" : "";
  const tone = pct > 0 ? "text-success" : pct < 0 ? "text-destructive" : "text-muted-foreground";
  return { text: `${sign}${pct}% vs last month`, tone };
}

function paintHeroDelta(points: SubscriberHistoryPoint[]) {
  // The hero total is painted by the page script; the delta belongs
  // to this history and is painted here.
  const el = document.getElementById("subscriber-delta");
  if (!el) return;
  const { text, tone } = heroDelta(points);
  el.textContent = text;
  el.className = `text-xs mt-1 ${tone}`;
}

export default function SubscriberHistoryPanel() {
  const [points, setPoints] = useState<SubscriberHistoryPoint[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setError(null);
    try {
      const { points } = await billing.subscriberHistory(12);
      setPoints(points);
      paintHeroDelta(points);
    } catch (err) {
      setError(
        err instanceof ApiError && err.status === 401
          ? "Your session has expired. Please sign out and sign in again."
          : "Could not load subscriber history. Check that the API is running and try again.",
      );
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  if (error) {
    return (
      <div className="px-4 py-12 text-center space-y-3">
        <p className="text-sm text-destructive">{error}</p>
        <button
          type="button"
          onClick={load}
          className="inline-flex items-center justify-center rounded-lg border border-input bg-background px-4 py-2 text-sm font-medium hover:bg-muted transition-colors"
        >
          Try again
        </button>
      </div>
    );
  }

  if (!points) {
    return (
      <div className="h-[300px] w-full rounded-lg bg-muted/40 animate-pulse" aria-label="Loading subscriber history" />
    );
  }

  return (
    <div className="space-y-4">
      <SubscriberHistoryChart points={points} />
      <div className="flex flex-wrap gap-4">
        {seriesColors(points).map(({ key, color }) => (
          <div key={key} className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <span className="size-2.5 rounded-full" style={{ background: color }}></span>
            <span className="capitalize">{key}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

import type { MetricCard, ChartDataPoint } from "@/lib/types";
import { createRng } from "@/lib/random";
import {
  Bell,
  CheckCircle2,
  XCircle,
  Clock,
  TrendingUp,
  MousePointerClick,
  Eye,
  Timer,
} from "lucide-react";

export function useMetrics(): MetricCard[] {
  return [
    {
      title: "Sent Today",
      value: "12,847",
      change: 12.5,
      changeLabel: "vs yesterday",
      icon: Bell,
    },
    {
      title: "Delivered",
      value: "12,503",
      change: 97.3,
      changeLabel: "delivery rate",
      icon: CheckCircle2,
    },
    {
      title: "Failures",
      value: "42",
      change: -8.2,
      changeLabel: "vs yesterday",
      icon: XCircle,
    },
    {
      title: "Queued",
      value: "312",
      change: 0,
      changeLabel: "in queue",
      icon: Clock,
    },
    {
      title: "Delivery Rate",
      value: "97.3%",
      change: 0.4,
      changeLabel: "vs last week",
      icon: TrendingUp,
    },
    {
      title: "Open Rate",
      value: "68.1%",
      change: 2.1,
      changeLabel: "vs last week",
      icon: Eye,
    },
    {
      title: "Click Rate",
      value: "24.7%",
      change: -1.3,
      changeLabel: "vs last week",
      icon: MousePointerClick,
    },
    {
      title: "Avg Latency",
      value: "142ms",
      change: -12,
      changeLabel: "vs last week",
      icon: Timer,
    },
  ];
}

const TIMELINE_REFERENCE_HOUR = 12;

function buildTimelineData(): ChartDataPoint[] {
  const rng = createRng(789);
  return Array.from({ length: 24 }, (_, i) => {
    const hour = (TIMELINE_REFERENCE_HOUR - (23 - i) + 24) % 24;
    return {
      date: `2026-01-15T${String(hour).padStart(2, "0")}:00:00.000Z`,
      value: Math.floor(300 + rng() * 700),
      label: `${String(hour).padStart(2, "0")}:00`,
    };
  });
}

const TIMELINE_DATA = buildTimelineData();

export function useNotificationTimeline(): ChartDataPoint[] {
  return TIMELINE_DATA;
}

export function useChannelDistribution() {
  return [
    { name: "Email", value: 4520, fill: "var(--chart-1)" },
    { name: "SMS", value: 2310, fill: "var(--chart-2)" },
    { name: "Push", value: 3200, fill: "var(--chart-3)" },
    { name: "Webhook", value: 1800, fill: "var(--chart-4)" },
    { name: "Slack", value: 1017, fill: "var(--chart-5)" },
  ];
}

export function useCountryDistribution() {
  return [
    { name: "United States", value: 4520, code: "US" },
    { name: "United Kingdom", value: 1830, code: "GB" },
    { name: "Germany", value: 1420, code: "DE" },
    { name: "France", value: 980, code: "FR" },
    { name: "Japan", value: 870, code: "JP" },
    { name: "Canada", value: 750, code: "CA" },
    { name: "Australia", value: 620, code: "AU" },
    { name: "Brazil", value: 510, code: "BR" },
    { name: "India", value: 480, code: "IN" },
    { name: "Other", value: 867, code: "XX" },
  ];
}

export function usePlatformDistribution() {
  return [
    { name: "iOS", value: 3800 },
    { name: "Android", value: 3200 },
    { name: "Web", value: 2900 },
    { name: "Desktop", value: 1800 },
    { name: "Other", value: 1147 },
  ];
}

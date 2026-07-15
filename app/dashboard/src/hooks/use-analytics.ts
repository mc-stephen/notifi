"use client";

import { useState, useEffect } from "react";
import type {
  KpiMetric,
  TimeSeriesPoint,
  ChannelBreakdown,
  Provider,
} from "@/lib/types";

type TimeRange = "24h" | "7d" | "30d";

function generateTimeSeries(range: TimeRange): TimeSeriesPoint[] {
  const now = Date.now();
  const points: TimeSeriesPoint[] = [];
  const intervals =
    range === "24h" ? 24 : range === "7d" ? 7 * 24 : 30 * 24;
  const intervalMs = (range === "24h" ? 3600000 : 3600000) as number;

  for (let i = intervals; i >= 0; i--) {
    const ts = new Date(now - i * intervalMs).toISOString();
    const base = Math.sin(i / 5) * 500 + 1000;
    const noise = Math.random() * 400 - 200;
    const ingested = Math.round(Math.max(0, base + noise));
    const delivered = Math.round(ingested * (0.85 + Math.random() * 0.12));
    points.push({ timestamp: ts, ingested, delivered });
  }
  return points;
}

const mockKpis: KpiMetric[] = [
  {
    label: "Total Outbound",
    value: 284_521,
    previousValue: 245_100,
    change: 16.1,
    isPercentage: true,
    icon: "Send",
  },
  {
    label: "Avg Latency",
    value: 142,
    previousValue: 178,
    change: -20.2,
    isPercentage: false,
    icon: "Clock",
  },
  {
    label: "Success Rate",
    value: 97.4,
    previousValue: 95.8,
    change: 1.6,
    isPercentage: true,
    icon: "CheckCircle2",
  },
  {
    label: "Active Channels",
    value: 5,
    previousValue: 4,
    change: 25,
    isPercentage: true,
    icon: "PlugZap",
  },
];

const channelBreakdown: ChannelBreakdown[] = [
  { channel: "email", volume: 120_000, color: "#3b82f6" },
  { channel: "fcm", volume: 85_000, color: "#22c55e" },
  { channel: "apns", volume: 45_000, color: "#a855f7" },
  { channel: "sms", volume: 22_000, color: "#f59e0b" },
  { channel: "webpush", volume: 12_521, color: "#06b6d4" },
];

const providers: Provider[] = [
  { id: "sendgrid", name: "SendGrid", channel: "email", status: "healthy", icon: "Mail" },
  { id: "fcm", name: "Firebase Cloud", channel: "fcm", status: "healthy", icon: "Flame" },
  { id: "apns", name: "APNs", channel: "apns", status: "degraded", icon: "Apple" },
  { id: "twilio", name: "Twilio", channel: "sms", status: "healthy", icon: "MessageCircle" },
  { id: "postmark", name: "Postmark", channel: "email", status: "healthy", icon: "Mail" },
  { id: "webpush", name: "WebPush", channel: "webpush", status: "outage", icon: "Globe" },
];

export function useAnalytics() {
  const [timeRange, setTimeRange] = useState<TimeRange>("24h");
  const [loading, setLoading] = useState(true);
  const [data, setData] = useState<{
    kpis: KpiMetric[];
    timeSeries: TimeSeriesPoint[];
    channelBreakdown: ChannelBreakdown[];
    providers: Provider[];
  }>({
    kpis: [],
    timeSeries: [],
    channelBreakdown: [],
    providers: [],
  });

  useEffect(() => {
    setLoading(true);
    const timer = setTimeout(() => {
      setData({
        kpis: mockKpis,
        timeSeries: generateTimeSeries(timeRange),
        channelBreakdown,
        providers,
      });
      setLoading(false);
    }, 600);
    return () => clearTimeout(timer);
  }, [timeRange]);

  return { ...data, timeRange, setTimeRange, loading };
}

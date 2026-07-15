"use client";

import { useAnalytics } from "@/hooks/use-analytics";
import { KpiCard } from "./kpi-card";
import { AnalyticsChart } from "./analytics-chart";
import { ChannelBreakdown } from "./channel-breakdown";
import { ProviderStatusBoard } from "./provider-status";

export function OverviewPage() {
  const { kpis, timeSeries, channelBreakdown, providers, loading } = useAnalytics();

  return (
    <div className="mx-auto w-full max-w-7xl space-y-6 p-5">
      {/* Page header */}
      <div>
        <h1 className="text-xl font-semibold tracking-tight">Dashboard</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Overview of your notification performance and provider health.
        </p>
      </div>

      {/* KPI cards */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {kpis.map((metric) => (
          <KpiCard key={metric.label} metric={metric} />
        ))}
      </div>

      {/* Charts row */}
      <div className="grid gap-6 lg:grid-cols-3">
        <div className="lg:col-span-2">
          <AnalyticsChart data={timeSeries} loading={loading} />
        </div>
        <ChannelBreakdown data={channelBreakdown} loading={loading} />
      </div>

      {/* Provider status */}
      <ProviderStatusBoard providers={providers} loading={loading} />
    </div>
  );
}

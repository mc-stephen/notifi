"use client";

import { useAnalytics } from "@/hooks/use-analytics";
import { PageHeader } from "@/components/dashboard/shared/page-header";
import { KpiGrid } from "@/components/dashboard/overview/kpi-grid";
import { TimeRangeToggle } from "@/components/dashboard/overview/time-range-toggle";
import { AnalyticsChart } from "@/components/dashboard/overview/analytics-chart";
import { ChannelBreakdownChart } from "@/components/dashboard/overview/channel-breakdown-chart";
import { ProviderStatusBoard } from "@/components/dashboard/overview/provider-status-board";
import { OverviewSkeleton } from "@/components/dashboard/overview/overview-skeleton";

export default function OverviewPage() {
  const {
    kpis,
    timeSeries,
    channelBreakdown,
    providers,
    timeRange,
    setTimeRange,
    loading,
  } = useAnalytics();

  if (loading) {
    return <OverviewSkeleton />;
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title="Overview"
        description="High-level metrics for your notification platform."
        action={<TimeRangeToggle value={timeRange} onChange={setTimeRange} />}
      />

      <KpiGrid metrics={kpis} />

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
        <div className="lg:col-span-2">
          <AnalyticsChart data={timeSeries} />
        </div>
        <ChannelBreakdownChart data={channelBreakdown} />
      </div>

      <ProviderStatusBoard providers={providers} />
    </div>
  );
}

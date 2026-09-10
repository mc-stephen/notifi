"use client";

import { useState } from "react";
import { useMetrics, useNotificationTimeline, useChannelDistribution, useCountryDistribution, usePlatformDistribution } from "@/hooks";
import { PageHeader } from "@/components/custom/page-header";
import { AreaChart } from "@/components/custom/charts/area-chart";
import { BarChart } from "@/components/custom/charts/bar-chart";
import { DonutChart } from "@/components/custom/charts/donut-chart";
import { MetricCard } from "@/components/custom/metric-card";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from "@/components/ui/select";
import {
  Download,
  TrendingUp,
  BarChart3,
  Users,
  Globe,
} from "lucide-react";

export default function AnalyticsPage() {
  const metrics = useMetrics();
  const timeline = useNotificationTimeline();
  const channelDistribution = useChannelDistribution();
  const countryDistribution = useCountryDistribution();
  const platformDistribution = usePlatformDistribution();
  const [timeRange, setTimeRange] = useState("7d");

  const deliveryData = timeline.map((d) => ({
    ...d,
    label: d.label ?? d.date,
  }));

  return (
    <div className="space-y-6">
      <PageHeader
        title="Analytics"
        description="Delivery performance and insights"
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Analytics" }]}
        actions={
          <div className="flex items-center gap-2">
            <Select value={timeRange} onValueChange={(v) => { if (v) setTimeRange(v); }}>
              <SelectTrigger className="w-[140px] h-8">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="24h">Last 24 hours</SelectItem>
                <SelectItem value="7d">Last 7 days</SelectItem>
                <SelectItem value="30d">Last 30 days</SelectItem>
                <SelectItem value="90d">Last 90 days</SelectItem>
              </SelectContent>
            </Select>
            <Button size="sm" variant="outline" className="gap-1.5">
              <Download className="size-3.5" /> Export
            </Button>
          </div>
        }
      />

      {/* KPI Grid */}
      <div className="grid gap-4 md:grid-cols-4">
        {metrics.slice(0, 4).map((metric) => (
          <MetricCard key={metric.title} {...metric} />
        ))}
      </div>

      {/* Secondary Metrics */}
      <div className="grid gap-4 md:grid-cols-4">
        {metrics.slice(4).map((metric) => (
          <MetricCard key={metric.title} {...metric} />
        ))}
      </div>

      {/* Charts */}
      <Tabs defaultValue="delivery">
        <TabsList>
          <TabsTrigger value="delivery">Delivery Trends</TabsTrigger>
          <TabsTrigger value="channels">Channels</TabsTrigger>
          <TabsTrigger value="geography">Geography</TabsTrigger>
          <TabsTrigger value="platforms">Platforms</TabsTrigger>
        </TabsList>

        <TabsContent value="delivery" className="mt-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <TrendingUp className="size-4" /> Delivery Volume (24h)
              </CardTitle>
            </CardHeader>
            <CardContent>
              <AreaChart
                data={deliveryData}
                height={300}
              />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="channels" className="mt-4">
          <div className="grid gap-6 lg:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <BarChart3 className="size-4" /> Channel Distribution
                </CardTitle>
              </CardHeader>
              <CardContent>
                <DonutChart
                  data={channelDistribution}
                  height={300}
                />
              </CardContent>
            </Card>
            <Card>
              <CardHeader>
                <CardTitle>Channel Performance</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  {channelDistribution.map((ch) => (
                    <div key={ch.name} className="space-y-2">
                      <div className="flex items-center justify-between text-sm">
                        <span>{ch.name}</span>
                        <span className="text-muted-foreground">{ch.value.toLocaleString()}</span>
                      </div>
                      <div className="h-2 bg-muted rounded-full overflow-hidden">
                        <div
                          className="h-full bg-primary rounded-full"
                          style={{ width: `${(ch.value / Math.max(...channelDistribution.map((c) => c.value))) * 100}%` }}
                        />
                      </div>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        <TabsContent value="geography" className="mt-4">
          <div className="grid gap-6 lg:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Globe className="size-4" /> Top Countries
                </CardTitle>
              </CardHeader>
              <CardContent>
                <BarChart
                  data={countryDistribution.slice(0, 8)}
                  height={300}
                />
              </CardContent>
            </Card>
            <Card>
              <CardHeader>
                <CardTitle>Country Breakdown</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="space-y-3">
                  {countryDistribution.slice(0, 8).map((country) => {
                    const total = countryDistribution.reduce((acc, c) => acc + c.value, 0);
                    const pct = ((country.value / total) * 100).toFixed(1);
                    return (
                      <div key={country.code} className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                          <span className="text-sm">{country.name}</span>
                          <Badge variant="secondary" className="text-[10px]">{country.code}</Badge>
                        </div>
                        <div className="flex items-center gap-3">
                          <div className="w-20 h-1.5 bg-muted rounded-full overflow-hidden">
                            <div className="h-full bg-primary rounded-full" style={{ width: `${pct}%` }} />
                          </div>
                          <span className="text-xs text-muted-foreground w-12 text-right">{pct}%</span>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        <TabsContent value="platforms" className="mt-4">
          <div className="grid gap-6 lg:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Users className="size-4" /> Platform Distribution
                </CardTitle>
              </CardHeader>
              <CardContent>
                <BarChart
                  data={platformDistribution}
                  height={300}
                />
              </CardContent>
            </Card>
            <Card>
              <CardHeader>
                <CardTitle>Platform Breakdown</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="space-y-3">
                  {platformDistribution.map((p) => {
                    const total = platformDistribution.reduce((acc, pl) => acc + pl.value, 0);
                    const pct = ((p.value / total) * 100).toFixed(1);
                    return (
                      <div key={p.name} className="flex items-center justify-between">
                        <span className="text-sm">{p.name}</span>
                        <div className="flex items-center gap-3">
                          <div className="w-20 h-1.5 bg-muted rounded-full overflow-hidden">
                            <div className="h-full bg-primary rounded-full" style={{ width: `${pct}%` }} />
                          </div>
                          <span className="text-xs text-muted-foreground w-12 text-right">{pct}%</span>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}

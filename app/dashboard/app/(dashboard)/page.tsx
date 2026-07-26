"use client";

import { useMetrics, useNotificationTimeline, useChannelDistribution, useCountryDistribution, usePlatformDistribution, useNotifications } from "@/hooks";
import { MetricCard } from "@/components/custom/metric-card";
import { StatusBadge } from "@/components/custom/status-badge";
import { ChannelBadge } from "@/components/custom/channel-badge";
import { HealthIndicator } from "@/components/custom/health-indicator";
import { Card, CardContent, CardHeader, CardTitle, CardAction } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { AreaChart } from "@/components/custom/charts/area-chart";
import { DonutChart } from "@/components/custom/charts/donut-chart";
import { BarChart } from "@/components/custom/charts/bar-chart";
import {
  ArrowRight,
  Plus,
  FileText,
  UserPlus,
  KeyRound,
  Plug,
  Activity,
  XCircle,
  Webhook,
} from "lucide-react";
import Link from "next/link";

function MetricCards() {
  const metrics = useMetrics();

  return (
    <div className="grid gap-4 grid-cols-2 md:grid-cols-4">
      {metrics.map((m) => (
        <MetricCard key={m.title} {...m} />
      ))}
    </div>
  );
}

function NotificationTimeline() {
  const data = useNotificationTimeline();

  return (
    <Card>
      <CardHeader>
        <CardTitle>Notification Timeline</CardTitle>
        <CardAction>
          <Button variant="ghost" size="sm" render={<Link href="/analytics" />}>
              View analytics <ArrowRight className="size-3.5 ml-1" />
          </Button>
        </CardAction>
      </CardHeader>
      <CardContent>
        <AreaChart data={data} height={250} />
      </CardContent>
    </Card>
  );
}

function ChannelDistribution() {
  const data = useChannelDistribution();

  return (
    <Card>
      <CardHeader>
        <CardTitle>Channel Distribution</CardTitle>
      </CardHeader>
      <CardContent>
        <DonutChart data={data} height={260} innerRadius={55} />
      </CardContent>
    </Card>
  );
}

function CountryDistribution() {
  const data = useCountryDistribution();

  return (
    <Card>
      <CardHeader>
        <CardTitle>Top Countries</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-3">
          {data.slice(0, 8).map((country) => {
            const maxVal = data[0].value;
            const pct = (country.value / maxVal) * 100;
            return (
              <div key={country.code} className="flex items-center gap-3">
                <span className="text-xs font-mono text-muted-foreground w-7">{country.code}</span>
                <div className="flex-1 h-1.5 bg-muted rounded-full overflow-hidden">
                  <div
                    className="h-full bg-primary/60 rounded-full"
                    style={{ width: `${pct}%` }}
                  />
                </div>
                <span className="text-xs font-medium tabular-nums w-14 text-right">{country.value.toLocaleString()}</span>
              </div>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}

function PlatformDistribution() {
  const data = usePlatformDistribution();

  return (
    <Card>
      <CardHeader>
        <CardTitle>Platforms</CardTitle>
      </CardHeader>
      <CardContent>
        <BarChart data={data} height={200} showGrid={false} />
      </CardContent>
    </Card>
  );
}

function RecentNotifications() {
  const { items } = useNotifications(undefined, undefined, 1, 8);

  return (
    <Card>
      <CardHeader>
        <CardTitle>Recent Notifications</CardTitle>
        <CardAction>
          <Button variant="ghost" size="sm" render={<Link href="/notifications" />}>
              View all <ArrowRight className="size-3.5 ml-1" />
          </Button>
        </CardAction>
      </CardHeader>
      <CardContent>
        <div className="space-y-1">
          {items.map((n) => (
            <div key={n.id} className="flex items-center gap-3 rounded-lg px-2 py-1.5 hover:bg-muted/50 transition-colors">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-mono text-muted-foreground">{n.id}</span>
                  <ChannelBadge channel={n.channel} showIcon={false} className="h-4 text-[10px] px-1.5" />
                </div>
                <p className="text-xs text-muted-foreground truncate mt-0.5">{n.subject ?? n.body}</p>
              </div>
              <StatusBadge status={n.status} className="h-5 text-[10px] px-1.5" />
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

function RecentFailures() {
  const { items } = useNotifications({ status: ["failed"] }, undefined, 1, 5);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <XCircle className="size-4 text-destructive" />
          Recent Failures
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {items.map((n) => (
            <div key={n.id} className="rounded-lg border border-destructive/20 bg-destructive/5 p-3">
              <div className="flex items-center justify-between">
                <span className="text-sm font-mono">{n.id}</span>
                <ChannelBadge channel={n.channel} showIcon={false} className="h-4 text-[10px] px-1.5" />
              </div>
              <p className="text-xs text-destructive mt-1">{n.failureReason}</p>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

function QuickActions() {
  const actions = [
    { label: "Create Notification", icon: Plus, href: "/notifications/new" },
    { label: "Create Template", icon: FileText, href: "/templates/new" },
    { label: "Invite Member", icon: UserPlus, href: "/team" },
    { label: "Generate API Key", icon: KeyRound, href: "/api-keys" },
    { label: "Add Provider", icon: Plug, href: "/providers" },
  ];

  return (
    <Card>
      <CardHeader>
        <CardTitle>Quick Actions</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-2 gap-2">
          {actions.map((a) => (
            <Button key={a.href} variant="outline" size="sm" className="justify-start gap-2" render={<Link href={a.href} />}>
                <a.icon className="size-3.5" />
                {a.label}
            </Button>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

function HealthStatus() {
  const providers = [
    { name: "SendGrid (Email)", status: "healthy" as const },
    { name: "Twilio (SMS)", status: "healthy" as const },
    { name: "Firebase (Push)", status: "degraded" as const },
    { name: "SNS (Push)", status: "healthy" as const },
    { name: "Webhook Worker", status: "healthy" as const },
  ];

  const systemStatus = [
    { name: "API Gateway", status: "healthy" as const },
    { name: "Queue", status: "healthy" as const },
    { name: "Workers", status: "healthy" as const },
    { name: "Database", status: "healthy" as const },
    { name: "Cache", status: "healthy" as const },
  ];

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Activity className="size-4" />
          System Health
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          <div>
            <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-2">Providers</h4>
            <div className="space-y-1.5">
              {providers.map((p) => (
                <div key={p.name} className="flex items-center justify-between">
                  <span className="text-sm">{p.name}</span>
                  <HealthIndicator status={p.status} showLabel />
                </div>
              ))}
            </div>
          </div>
          <Separator />
          <div>
            <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-2">System</h4>
            <div className="space-y-1.5">
              {systemStatus.map((s) => (
                <div key={s.name} className="flex items-center justify-between">
                  <span className="text-sm">{s.name}</span>
                  <HealthIndicator status={s.status} showLabel />
                </div>
              ))}
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function RecentWebhooks() {
  const webhooks = [
    { id: "wh_1", url: "api.example.com/webhooks/notif", event: "notification.delivered", status: "success", time: "2m ago" },
    { id: "wh_2", url: "hooks.slack.com/T0XXX", event: "notification.failed", status: "success", time: "5m ago" },
    { id: "wh_3", url: "api.example.com/webhooks/billing", event: "notification.sent", status: "failed", time: "8m ago" },
    { id: "wh_4", url: "zapier.com/hooks/catch/123", event: "notification.opened", status: "success", time: "12m ago" },
  ];

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Webhook className="size-4" />
          Recent Webhooks
        </CardTitle>
        <CardAction>
          <Button variant="ghost" size="sm" render={<Link href="/webhooks" />}>
              View all <ArrowRight className="size-3.5 ml-1" />
          </Button>
        </CardAction>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {webhooks.map((w) => (
            <div key={w.id} className="flex items-center gap-3 rounded-lg px-2 py-1.5 hover:bg-muted/50 transition-colors">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-mono truncate">{w.url}</span>
                  {w.status === "success" ? (
                    <span className="size-1.5 rounded-full bg-success shrink-0" />
                  ) : (
                    <span className="size-1.5 rounded-full bg-destructive shrink-0" />
                  )}
                </div>
                <div className="flex items-center gap-2 mt-0.5">
                  <span className="text-xs text-muted-foreground">{w.event}</span>
                  <span className="text-xs text-muted-foreground">·</span>
                  <span className="text-xs text-muted-foreground">{w.time}</span>
                </div>
              </div>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

export default function DashboardPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">Dashboard</h1>
        <p className="text-sm text-muted-foreground">Overview of your notification system.</p>
      </div>

      <MetricCards />

      <div className="grid gap-6 lg:grid-cols-3">
        <div className="lg:col-span-2">
          <NotificationTimeline />
        </div>
        <div>
          <HealthStatus />
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        <div>
          <ChannelDistribution />
        </div>
        <div>
          <CountryDistribution />
        </div>
        <div>
          <PlatformDistribution />
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        <div className="lg:col-span-2">
          <RecentNotifications />
        </div>
        <div className="space-y-6">
          <QuickActions />
          <RecentFailures />
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        <RecentWebhooks />
      </div>
    </div>
  );
}

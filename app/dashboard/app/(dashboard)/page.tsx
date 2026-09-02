"use client";

import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { DataTable } from "@/components/custom/data-table";
import { MetricCard } from "@/components/custom/metric-card";
import { StatusBadge } from "@/components/custom/status-badge";
import { ChannelBadge } from "@/components/custom/channel-badge";
import { AreaChart } from "@/components/custom/charts/area-chart";
import { PriorityBadge } from "@/components/custom/priority-badge";
import { HealthIndicator } from "@/components/custom/health-indicator";
import { useMetrics, useNotificationTimeline, useNotifications } from "@/hooks";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardAction,
} from "@/components/ui/card";

import Link from "next/link";
import { format } from "date-fns";
import type { Notification } from "@/lib/types";
import { ArrowRight, Activity } from "lucide-react";
import { type ColumnDef } from "@tanstack/react-table";

//===================================
//
//===================================
function MetricCards() {
  const metrics = useMetrics();

  return (
    <div className="grid gap-4 grid-cols-2 md:grid-cols-6">
      {metrics.map((m) => (
        <MetricCard key={m.title} {...m} />
      ))}
    </div>
  );
}

//===================================
//
//===================================
function NotificationTimeline() {
  const data = useNotificationTimeline();

  return (
    <Card className="h-full">
      <CardHeader>
        <CardTitle>Notification Timeline</CardTitle>
        <CardAction>
          <Button variant="ghost" size="sm" render={<Link href="/analytics" />}>
            View analytics <ArrowRight className="size-3.5 ml-1" />
          </Button>
        </CardAction>
      </CardHeader>
      <CardContent className="flex-1">
        <AreaChart data={data} height="fill" />
      </CardContent>
    </Card>
  );
}

//===================================
//
//===================================
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
    <Card className="h-full">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Activity className="size-4" />
          System Health
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          <div>
            <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-2">
              Providers
            </h4>
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
            <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-2">
              System
            </h4>
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

//===================================
//
//===================================
function RecentNotificationsTable() {
  const { items } = useNotifications(undefined, undefined, 1, 50);

  const columns: ColumnDef<Notification, unknown>[] = [
    {
      accessorKey: "id",
      header: "ID",
      cell: ({ row }) => (
        <span className="font-mono text-xs">{row.original.id}</span>
      ),
    },
    {
      accessorKey: "channel",
      header: "Channel",
      cell: ({ row }) => (
        <ChannelBadge channel={row.original.channel} showIcon />
      ),
    },
    {
      accessorKey: "subject",
      header: "Subject",
      cell: ({ row }) => (
        <span className="truncate max-w-[240px] text-sm">
          {row.original.subject ?? row.original.body}
        </span>
      ),
    },
    {
      accessorKey: "status",
      header: "Status",
      cell: ({ row }) => <StatusBadge status={row.original.status} />,
    },
    {
      accessorKey: "priority",
      header: "Priority",
      cell: ({ row }) => <PriorityBadge priority={row.original.priority} />,
    },
    {
      accessorKey: "recipientId",
      header: "Recipient",
      cell: ({ row }) => (
        <span className="font-mono text-xs">{row.original.recipientId}</span>
      ),
    },
    {
      accessorKey: "createdAt",
      header: "Created",
      cell: ({ row }) => (
        <span className="text-xs text-muted-foreground whitespace-nowrap">
          {format(new Date(row.original.createdAt), "MMM d, HH:mm")}
        </span>
      ),
    },
  ];

  return (
    <Card>
      <CardHeader>
        <CardTitle>Recent Notifications</CardTitle>
        <CardAction>
          <Button
            variant="ghost"
            size="sm"
            render={<Link href="/notifications" />}
          >
            View all <ArrowRight className="size-3.5 ml-1" />
          </Button>
        </CardAction>
      </CardHeader>
      <CardContent>
        <DataTable
          columns={columns}
          data={items}
          searchKey="subject"
          searchPlaceholder="Search by subject..."
          pageSize={10}
        />
      </CardContent>
    </Card>
  );
}

//===================================
//
//===================================
export default function DashboardPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">Dashboard</h1>
        <p className="text-sm text-muted-foreground">
          Overview of your notification system.
        </p>
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

      <RecentNotificationsTable />
    </div>
  );
}

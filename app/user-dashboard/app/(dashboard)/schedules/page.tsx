"use client";

import { useState } from "react";
import { type ColumnDef } from "@tanstack/react-table";
import { PageHeader } from "@/components/custom/page-header";
import { StatCard } from "@/components/custom/stat-card";
import { DataTable } from "@/components/custom/data-table";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { format } from "date-fns";
import {
  Clock,
  Plus,
  Play,
  Pause,
  Calendar,
  Timer,
  RotateCcw,
} from "lucide-react";

type ScheduleEntry = {
  id: string;
  name: string;
  notificationId: string;
  cron: string;
  cronHuman: string;
  timezone: string;
  active: boolean;
  nextRunAt?: string;
  lastRunAt?: string;
  channel: string;
  recipientCount: number;
  createdAt: string;
};

const MOCK_SCHEDULES: ScheduleEntry[] = [
  { id: "sch_001", name: "Daily Welcome Email", notificationId: "ntf_0001", cron: "0 9 * * *", cronHuman: "Every day at 9:00 AM", timezone: "America/New_York", active: true, nextRunAt: "2025-06-26T09:00:00Z", lastRunAt: "2025-06-25T09:00:00Z", channel: "email", recipientCount: 45, createdAt: "2025-01-15T00:00:00Z" },
  { id: "sch_002", name: "Weekly Newsletter", notificationId: "ntf_0006", cron: "0 10 * * 1", cronHuman: "Every Monday at 10:00 AM", timezone: "America/New_York", active: true, nextRunAt: "2025-06-30T10:00:00Z", lastRunAt: "2025-06-23T10:00:00Z", channel: "email", recipientCount: 12000, createdAt: "2025-02-01T00:00:00Z" },
  { id: "sch_003", name: "Monthly Usage Report", notificationId: "ntf_0007", cron: "0 8 1 * *", cronHuman: "1st of every month at 8:00 AM", timezone: "Europe/London", active: true, nextRunAt: "2025-07-01T08:00:00Z", lastRunAt: "2025-06-01T08:00:00Z", channel: "email", recipientCount: 500, createdAt: "2025-03-10T00:00:00Z" },
  { id: "sch_004", name: "Hourly System Health Check", notificationId: "ntf_0010", cron: "0 * * * *", cronHuman: "Every hour", timezone: "UTC", active: false, nextRunAt: undefined, lastRunAt: "2025-06-25T08:00:00Z", channel: "webhook", recipientCount: 1, createdAt: "2025-04-01T00:00:00Z" },
  { id: "sch_005", name: "Daily Push Digest", notificationId: "ntf_0011", cron: "0 18 * * *", cronHuman: "Every day at 6:00 PM", timezone: "Asia/Tokyo", active: true, nextRunAt: "2025-06-26T18:00:00Z", lastRunAt: "2025-06-25T18:00:00Z", channel: "push-android", recipientCount: 8500, createdAt: "2025-05-01T00:00:00Z" },
];

export default function SchedulesPage() {
  const [createDialog, setCreateDialog] = useState(false);

  const activeCount = MOCK_SCHEDULES.filter((s) => s.active).length;

  const columns: ColumnDef<ScheduleEntry>[] = [
    {
      accessorKey: "name",
      header: "Schedule",
      cell: ({ row }) => (
        <div className="min-w-0">
          <div className="text-sm font-medium">{row.original.name}</div>
          <div className="text-xs text-muted-foreground">{row.original.timezone}</div>
        </div>
      ),
    },
    {
      accessorKey: "cron",
      header: "Cron",
      cell: ({ row }) => (
        <code className="text-xs font-mono bg-muted px-1.5 py-0.5 rounded">{row.original.cron}</code>
      ),
    },
    {
      accessorKey: "cronHuman",
      header: "Frequency",
      cell: ({ row }) => <span className="text-xs">{row.original.cronHuman}</span>,
    },
    {
      accessorKey: "channel",
      header: "Channel",
      cell: ({ row }) => (
        <Badge variant="secondary" className="text-xs capitalize">
          {row.original.channel.replace(/-/g, " ")}
        </Badge>
      ),
    },
    {
      accessorKey: "recipientCount",
      header: "Recipients",
      cell: ({ row }) => (
        <span className="text-sm">{row.original.recipientCount.toLocaleString()}</span>
      ),
    },
    {
      accessorKey: "nextRunAt",
      header: "Next Run",
      cell: ({ row }) => (
        <span className="text-xs text-muted-foreground">
          {row.original.nextRunAt ? format(new Date(row.original.nextRunAt), "MMM d, HH:mm") : "—"}
        </span>
      ),
    },
    {
      accessorKey: "lastRunAt",
      header: "Last Run",
      cell: ({ row }) => (
        <span className="text-xs text-muted-foreground">
          {row.original.lastRunAt ? format(new Date(row.original.lastRunAt), "MMM d, HH:mm") : "—"}
        </span>
      ),
    },
    {
      accessorKey: "active",
      header: "Status",
      cell: ({ row }) => (
        <Badge variant={row.original.active ? "default" : "secondary"} className={row.original.active ? "bg-success/15 text-success border-success/20" : ""}>
          {row.original.active ? "Active" : "Paused"}
        </Badge>
      ),
    },
  ];

  return (
    <div className="space-y-6">
      <PageHeader
        title="Schedules"
        description={`${activeCount} active schedules`}
        actions={
          <Button size="sm" className="gap-1.5" onClick={() => setCreateDialog(true)}>
            <Plus className="size-3.5" /> New schedule
          </Button>
        }
      />

      <div className="grid gap-4 md:grid-cols-3">
        <StatCard label="Active Schedules" value={activeCount} icon={Clock} />
        <StatCard
          label="Next Run"
          icon={Calendar}
          value={
            MOCK_SCHEDULES.filter((s) => s.active && s.nextRunAt).length > 0
              ? format(
                  new Date(
                    Math.min(
                      ...MOCK_SCHEDULES.filter((s) => s.active && s.nextRunAt).map((s) => new Date(s.nextRunAt!).getTime())
                    )
                  ),
                  "MMM d"
                )
              : "—"
          }
        />
        <StatCard
          label="Total Recipients"
          icon={Timer}
          value={MOCK_SCHEDULES.reduce((acc, s) => acc + s.recipientCount, 0).toLocaleString()}
        />
      </div>

      <DataTable
        columns={columns}
        data={MOCK_SCHEDULES}
        searchPlaceholder="Search schedules..."
        emptyMessage="No schedules found."
        tableClassName="bg-card"
        rowActions={(schedule) => (
          <div className="flex items-center gap-1">
            <Button variant="ghost" size="icon-xs" title={schedule.active ? "Pause" : "Resume"}>
              {schedule.active ? <Pause className="size-3" /> : <Play className="size-3" />}
            </Button>
            <Button variant="ghost" size="icon-xs" title="Run now">
              <RotateCcw className="size-3" />
            </Button>
          </div>
        )}
      />

      <Dialog open={createDialog} onOpenChange={setCreateDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create schedule</DialogTitle>
            <DialogDescription>Set up a recurring notification schedule.</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">Schedule name</label>
              <Input placeholder="e.g. Daily Digest" />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Cron expression</label>
              <Input placeholder="0 9 * * *" />
              <p className="text-xs text-muted-foreground">Minute Hour Day Month Weekday</p>
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Timezone</label>
              <Select defaultValue="UTC">
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="UTC">UTC</SelectItem>
                  <SelectItem value="America/New_York">America/New_York</SelectItem>
                  <SelectItem value="Europe/London">Europe/London</SelectItem>
                  <SelectItem value="Asia/Tokyo">Asia/Tokyo</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Notification template</label>
              <Select defaultValue="tpl_1">
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="tpl_1">Welcome Email</SelectItem>
                  <SelectItem value="tpl_6">Weekly Digest</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setCreateDialog(false)}>Cancel</Button>
            <Button onClick={() => setCreateDialog(false)}>Create schedule</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

"use client";

import { useState, useMemo } from "react";
import { PageHeader } from "@/components/custom/page-header";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
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
  const [search, setSearch] = useState("");
  const [createDialog, setCreateDialog] = useState(false);

  const filtered = useMemo(() => {
    if (!search) return MOCK_SCHEDULES;
    const q = search.toLowerCase();
    return MOCK_SCHEDULES.filter((s) => s.name.toLowerCase().includes(q) || s.cronHuman.toLowerCase().includes(q));
  }, [search]);

  const activeCount = MOCK_SCHEDULES.filter((s) => s.active).length;

  return (
    <div className="space-y-6">
      <PageHeader
        title="Schedules"
        description={`${activeCount} active schedules`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Schedules" }]}
        actions={
          <Button size="sm" className="gap-1.5" onClick={() => setCreateDialog(true)}>
            <Plus className="size-3.5" /> New schedule
          </Button>
        }
      />

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Active Schedules</span>
              <Clock className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{activeCount}</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Next Run</span>
              <Calendar className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">
              {MOCK_SCHEDULES.filter((s) => s.active && s.nextRunAt).length > 0
                ? format(
                    new Date(
                      Math.min(
                        ...MOCK_SCHEDULES.filter((s) => s.active && s.nextRunAt).map((s) => new Date(s.nextRunAt!).getTime())
                      )
                    ),
                    "MMM d"
                  )
                : "—"}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Total Recipients</span>
              <Timer className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">
              {MOCK_SCHEDULES.reduce((acc, s) => acc + s.recipientCount, 0).toLocaleString()}
            </div>
          </CardContent>
        </Card>
      </div>

      <div className="relative max-w-sm">
        <Input
          placeholder="Search schedules..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="h-8 text-sm"
        />
      </div>

      <Card>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Schedule</TableHead>
                <TableHead>Cron</TableHead>
                <TableHead>Frequency</TableHead>
                <TableHead>Channel</TableHead>
                <TableHead>Recipients</TableHead>
                <TableHead>Next Run</TableHead>
                <TableHead>Last Run</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="w-[100px]"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filtered.map((schedule) => (
                <TableRow key={schedule.id}>
                  <TableCell>
                    <div className="text-sm font-medium">{schedule.name}</div>
                    <div className="text-xs text-muted-foreground">{schedule.timezone}</div>
                  </TableCell>
                  <TableCell>
                    <code className="text-xs font-mono bg-muted px-1.5 py-0.5 rounded">{schedule.cron}</code>
                  </TableCell>
                  <TableCell className="text-xs">{schedule.cronHuman}</TableCell>
                  <TableCell>
                    <Badge variant="secondary" className="text-xs capitalize">{schedule.channel.replace(/-/g, " ")}</Badge>
                  </TableCell>
                  <TableCell className="text-sm">{schedule.recipientCount.toLocaleString()}</TableCell>
                  <TableCell>
                    <span className="text-xs text-muted-foreground">
                      {schedule.nextRunAt ? format(new Date(schedule.nextRunAt), "MMM d, HH:mm") : "—"}
                    </span>
                  </TableCell>
                  <TableCell>
                    <span className="text-xs text-muted-foreground">
                      {schedule.lastRunAt ? format(new Date(schedule.lastRunAt), "MMM d, HH:mm") : "—"}
                    </span>
                  </TableCell>
                  <TableCell>
                    <Badge variant={schedule.active ? "default" : "secondary"} className={schedule.active ? "bg-success/15 text-success border-success/20" : ""}>
                      {schedule.active ? "Active" : "Paused"}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1">
                      <Button variant="ghost" size="icon-xs" title={schedule.active ? "Pause" : "Resume"}>
                        {schedule.active ? <Pause className="size-3" /> : <Play className="size-3" />}
                      </Button>
                      <Button variant="ghost" size="icon-xs" title="Run now">
                        <RotateCcw className="size-3" />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

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
              <select className="w-full rounded-md border bg-transparent px-3 py-2 text-sm">
                <option value="UTC">UTC</option>
                <option value="America/New_York">America/New_York</option>
                <option value="Europe/London">Europe/London</option>
                <option value="Asia/Tokyo">Asia/Tokyo</option>
              </select>
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Notification template</label>
              <select className="w-full rounded-md border bg-transparent px-3 py-2 text-sm">
                <option value="tpl_1">Welcome Email</option>
                <option value="tpl_6">Weekly Digest</option>
              </select>
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

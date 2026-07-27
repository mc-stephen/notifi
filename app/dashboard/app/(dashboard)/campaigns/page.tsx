"use client";

import { useState, useMemo } from "react";
import { PageHeader } from "@/components/custom/page-header";
import { ChannelBadge } from "@/components/custom/channel-badge";
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
  Megaphone,
  Plus,
  Search,
  Users,
  CheckCircle2,
  TrendingUp,
} from "lucide-react";
import type { NotificationChannel } from "@/lib/types";

type CampaignEntry = {
  id: string;
  name: string;
  templateId: string;
  status: "draft" | "scheduled" | "running" | "paused" | "completed" | "cancelled";
  channel: NotificationChannel;
  recipientCount: number;
  sentCount: number;
  deliveredCount: number;
  failedCount: number;
  scheduledAt?: string;
  startedAt?: string;
  completedAt?: string;
  createdAt: string;
};

const STATUS_COLORS: Record<string, string> = {
  draft: "bg-muted text-muted-foreground",
  scheduled: "bg-info/15 text-info",
  running: "bg-success/15 text-success",
  paused: "bg-warning/15 text-warning",
  completed: "bg-success/15 text-success",
  cancelled: "bg-muted text-muted-foreground",
};

const MOCK_CAMPAIGNS: CampaignEntry[] = [
  { id: "cmp_001", name: "Q2 Product Launch", templateId: "tpl_1", status: "completed", channel: "email", recipientCount: 15000, sentCount: 15000, deliveredCount: 14720, failedCount: 280, startedAt: "2025-06-01T09:00:00Z", completedAt: "2025-06-01T10:30:00Z", createdAt: "2025-05-28T14:00:00Z" },
  { id: "cmp_002", name: "Summer Sale Promo", templateId: "tpl_3", status: "running", channel: "email", recipientCount: 8500, sentCount: 6200, deliveredCount: 6100, failedCount: 100, startedAt: "2025-06-25T08:00:00Z", createdAt: "2025-06-20T11:00:00Z" },
  { id: "cmp_003", name: "Security Alert - Password Reset", templateId: "tpl_2", status: "completed", channel: "sms", recipientCount: 3200, sentCount: 3200, deliveredCount: 3180, failedCount: 20, startedAt: "2025-06-20T14:00:00Z", completedAt: "2025-06-20T14:45:00Z", createdAt: "2025-06-19T16:00:00Z" },
  { id: "cmp_004", name: "App Update Push", templateId: "tpl_5", status: "scheduled", channel: "push-android", recipientCount: 25000, sentCount: 0, deliveredCount: 0, failedCount: 0, scheduledAt: "2025-07-01T10:00:00Z", createdAt: "2025-06-24T09:00:00Z" },
  { id: "cmp_005", name: "Weekly Newsletter", templateId: "tpl_6", status: "paused", channel: "email", recipientCount: 12000, sentCount: 4500, deliveredCount: 4450, failedCount: 50, startedAt: "2025-06-24T09:00:00Z", createdAt: "2025-06-23T15:00:00Z" },
  { id: "cmp_006", name: "Onboarding Flow - Day 3", templateId: "tpl_1", status: "draft", channel: "email", recipientCount: 0, sentCount: 0, deliveredCount: 0, failedCount: 0, createdAt: "2025-06-25T10:00:00Z" },
  { id: "cmp_007", name: "Event Reminder - Webinar", templateId: "tpl_4", status: "scheduled", channel: "push-ios", recipientCount: 5000, sentCount: 0, deliveredCount: 0, failedCount: 0, scheduledAt: "2025-07-05T08:00:00Z", createdAt: "2025-06-25T11:00:00Z" },
];

export default function CampaignsPage() {
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState<string | null>(null);
  const [createDialog, setCreateDialog] = useState(false);

  const filtered = useMemo(() => {
    let result = [...MOCK_CAMPAIGNS];
    if (search) {
      const q = search.toLowerCase();
      result = result.filter((c) => c.name.toLowerCase().includes(q) || c.id.toLowerCase().includes(q));
    }
    if (statusFilter) {
      result = result.filter((c) => c.status === statusFilter);
    }
    return result;
  }, [search, statusFilter]);

  const totalRecipients = MOCK_CAMPAIGNS.reduce((acc, c) => acc + c.recipientCount, 0);
  const totalSent = MOCK_CAMPAIGNS.reduce((acc, c) => acc + c.sentCount, 0);
  const totalDelivered = MOCK_CAMPAIGNS.reduce((acc, c) => acc + c.deliveredCount, 0);
  const deliveryRate = totalSent > 0 ? ((totalDelivered / totalSent) * 100).toFixed(1) : "0";

  return (
    <div className="space-y-6">
      <PageHeader
        title="Campaigns"
        description={`${MOCK_CAMPAIGNS.length} campaigns total`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Campaigns" }]}
        actions={
          <Button size="sm" className="gap-1.5" onClick={() => setCreateDialog(true)}>
            <Plus className="size-3.5" /> New campaign
          </Button>
        }
      />

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Total Campaigns</span>
              <Megaphone className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{MOCK_CAMPAIGNS.length}</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Recipients Reached</span>
              <Users className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{totalRecipients.toLocaleString()}</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Notifications Sent</span>
              <CheckCircle2 className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{totalSent.toLocaleString()}</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Delivery Rate</span>
              <TrendingUp className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{deliveryRate}%</div>
          </CardContent>
        </Card>
      </div>

      <div className="flex items-center gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
          <Input
            placeholder="Search campaigns..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-8 h-8 text-sm"
          />
        </div>
        <div className="flex items-center gap-2 flex-wrap">
          {(["draft", "scheduled", "running", "paused", "completed", "cancelled"] as const).map((s) => (
            <button
              key={s}
              onClick={() => setStatusFilter(statusFilter === s ? null : s)}
              className={`flex items-center gap-1.5 rounded-md border px-2.5 py-1 text-xs transition-colors capitalize ${
                statusFilter === s ? "border-primary bg-primary/5" : "hover:bg-muted/50"
              }`}
            >
              <div className={`size-1.5 rounded-full ${STATUS_COLORS[s].split(" ")[0]}`} />
              {s}
            </button>
          ))}
          {statusFilter && (
            <Button variant="ghost" size="sm" onClick={() => setStatusFilter(null)} className="text-xs">
              Clear
            </Button>
          )}
        </div>
      </div>

      <Card>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Campaign</TableHead>
                <TableHead>Channel</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Recipients</TableHead>
                <TableHead>Sent</TableHead>
                <TableHead>Delivered</TableHead>
                <TableHead>Failed</TableHead>
                <TableHead>Scheduled</TableHead>
                <TableHead>Created</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filtered.map((campaign) => (
                <TableRow key={campaign.id} className="cursor-pointer hover:bg-muted/30">
                  <TableCell>
                    <div>
                      <div className="text-sm font-medium">{campaign.name}</div>
                      <div className="text-xs text-muted-foreground font-mono">{campaign.id}</div>
                    </div>
                  </TableCell>
                  <TableCell>
                    <ChannelBadge channel={campaign.channel} showIcon={false} className="h-4 text-[10px] px-1.5" />
                  </TableCell>
                  <TableCell>
                    <Badge variant="secondary" className={`text-xs capitalize ${STATUS_COLORS[campaign.status]}`}>
                      {campaign.status}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-sm">{campaign.recipientCount.toLocaleString()}</TableCell>
                  <TableCell className="text-sm">{campaign.sentCount.toLocaleString()}</TableCell>
                  <TableCell className="text-sm text-success">{campaign.deliveredCount.toLocaleString()}</TableCell>
                  <TableCell className="text-sm text-destructive">{campaign.failedCount.toLocaleString()}</TableCell>
                  <TableCell>
                    <span className="text-xs text-muted-foreground">
                      {campaign.scheduledAt ? format(new Date(campaign.scheduledAt), "MMM d, HH:mm") : "—"}
                    </span>
                  </TableCell>
                  <TableCell>
                    <span className="text-xs text-muted-foreground">
                      {format(new Date(campaign.createdAt), "MMM d, yyyy")}
                    </span>
                  </TableCell>
                </TableRow>
              ))}
              {filtered.length === 0 && (
                <TableRow>
                  <TableCell colSpan={9} className="text-center py-8 text-sm text-muted-foreground">
                    No campaigns found.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Dialog open={createDialog} onOpenChange={setCreateDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create campaign</DialogTitle>
            <DialogDescription>Set up a new notification campaign for your recipients.</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">Campaign name</label>
              <Input placeholder="e.g. Summer Product Launch" />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Channel</label>
              <select className="w-full rounded-md border bg-transparent px-3 py-2 text-sm">
                <option value="email">Email</option>
                <option value="sms">SMS</option>
                <option value="push-android">Android Push</option>
                <option value="push-ios">Apple Push</option>
                <option value="web-push">Web Push</option>
              </select>
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Template</label>
              <select className="w-full rounded-md border bg-transparent px-3 py-2 text-sm">
                <option value="tpl_1">Welcome Email</option>
                <option value="tpl_3">Order Confirmation</option>
                <option value="tpl_6">Weekly Digest</option>
              </select>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setCreateDialog(false)}>Cancel</Button>
            <Button onClick={() => setCreateDialog(false)}>Create campaign</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

"use client";

import { useState } from "react";
import { PageHeader } from "@/components/custom/page-header";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
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
  Webhook,
  Plus,
  RotateCcw,
  Trash2,
} from "lucide-react";

type WebhookEntry = {
  id: string;
  url: string;
  events: string[];
  enabled: boolean;
  lastTriggeredAt?: string;
  successRate: number;
  createdAt: string;
};

const MOCK_WEBHOOKS: WebhookEntry[] = [
  { id: "wh_1", url: "https://api.example.com/webhooks/notifications", events: ["notification.delivered", "notification.failed"], enabled: true, lastTriggeredAt: "2025-06-25T10:30:00Z", successRate: 99.2, createdAt: "2025-01-15T00:00:00Z" },
  { id: "wh_2", url: "https://hooks.slack.com/services/T00/B00/xxx", events: ["notification.failed"], enabled: true, lastTriggeredAt: "2025-06-25T09:15:00Z", successRate: 100, createdAt: "2025-02-01T00:00:00Z" },
  { id: "wh_3", url: "https://api.stripe.com/webhooks", events: ["campaign.started", "campaign.completed"], enabled: false, successRate: 85.5, createdAt: "2025-03-10T00:00:00Z" },
];

export default function WebhooksPage() {
  const [webhooks] = useState(MOCK_WEBHOOKS);
  const [deleteDialog, setDeleteDialog] = useState<WebhookEntry | null>(null);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Webhooks"
        description={`${webhooks.length} configured webhooks`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Webhooks" }]}
        actions={
          <Button size="sm" className="gap-1.5">
            <Plus className="size-3.5" /> Add webhook
          </Button>
        }
      />

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardContent className="pt-5">
            <div className="text-2xl font-bold">{webhooks.filter((w) => w.enabled).length}</div>
            <p className="text-xs text-muted-foreground mt-1">Active webhooks</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="text-2xl font-bold">
              {webhooks.filter((w) => w.enabled).length > 0
                ? Math.round(webhooks.filter((w) => w.enabled).reduce((acc, w) => acc + w.successRate, 0) / webhooks.filter((w) => w.enabled).length)
                : 0}%
            </div>
            <p className="text-xs text-muted-foreground mt-1">Avg success rate</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="text-2xl font-bold">
              {webhooks.filter((w) => w.lastTriggeredAt).length > 0
                ? format(new Date(Math.max(...webhooks.filter((w) => w.lastTriggeredAt).map((w) => new Date(w.lastTriggeredAt!).getTime()))), "MMM d, HH:mm")
                : "—"}
            </div>
            <p className="text-xs text-muted-foreground mt-1">Last triggered</p>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>URL</TableHead>
                <TableHead>Events</TableHead>
                <TableHead>Success Rate</TableHead>
                <TableHead>Last Triggered</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="w-[100px]"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {webhooks.map((webhook) => (
                <TableRow key={webhook.id}>
                  <TableCell>
                    <div className="flex items-center gap-2">
                      <Webhook className="size-3.5 text-muted-foreground" />
                      <span className="font-mono text-xs max-w-[250px] truncate">{webhook.url}</span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="flex flex-wrap gap-1">
                      {webhook.events.slice(0, 2).map((e) => (
                        <Badge key={e} variant="secondary" className="text-[10px]">{e}</Badge>
                      ))}
                      {webhook.events.length > 2 && (
                        <Badge variant="secondary" className="text-[10px]">+{webhook.events.length - 2}</Badge>
                      )}
                    </div>
                  </TableCell>
                  <TableCell>
                    <span className={`text-sm ${webhook.successRate >= 95 ? "text-success" : webhook.successRate >= 80 ? "text-warning" : "text-destructive"}`}>
                      {webhook.successRate}%
                    </span>
                  </TableCell>
                  <TableCell>
                    <span className="text-xs text-muted-foreground">
                      {webhook.lastTriggeredAt ? format(new Date(webhook.lastTriggeredAt), "MMM d, HH:mm") : "Never"}
                    </span>
                  </TableCell>
                  <TableCell>
                    <Badge variant={webhook.enabled ? "default" : "secondary"} className={webhook.enabled ? "bg-success/15 text-success border-success/20" : ""}>
                      {webhook.enabled ? "Active" : "Disabled"}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1">
                      <Button variant="ghost" size="icon-xs">
                        <RotateCcw className="size-3" />
                      </Button>
                      <Button variant="ghost" size="icon-xs" className="text-destructive" onClick={() => setDeleteDialog(webhook)}>
                        <Trash2 className="size-3" />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Dialog open={!!deleteDialog} onOpenChange={() => setDeleteDialog(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete webhook</DialogTitle>
            <DialogDescription>
              Permanently delete this webhook? This cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteDialog(null)}>Cancel</Button>
            <Button variant="destructive" onClick={() => setDeleteDialog(null)}>Delete</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

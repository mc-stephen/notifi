"use client";

import { useMemo, useState, type ReactNode } from "react";
import { type ColumnDef } from "@tanstack/react-table";
import { DataTable } from "@/components/custom/data-table";
import { DetailPanel } from "@/components/custom/detail-panel";
import { PageHeader } from "@/components/custom/page-header";
import { StatCard } from "@/components/custom/stat-card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Checkbox } from "@/components/ui/checkbox";
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
  Plus,
  Trash2,
  Copy,
  Check,
  RotateCcw,
} from "lucide-react";
import { WEBHOOK_EVENTS } from "@/lib/constants";
import { AvailableWebhookEvents } from "@/components/custom/webhooks/available-events";
import type { Webhook } from "@/lib/types";

const MOCK_WEBHOOKS: Webhook[] = [
  {
    id: "wh_1",
    projectId: "proj_1",
    url: "https://api.example.com/webhooks/notifications",
    events: ["notification.delivered", "notification.failed"],
    signingSecret: "whsec_a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4",
    enabled: true,
    lastTriggeredAt: "2025-06-25T10:30:00Z",
    successRate: 99.2,
    createdAt: "2025-01-15T00:00:00Z",
  },
  {
    id: "wh_2",
    projectId: "proj_1",
    url: "https://hooks.slack.com/services/T00/B00/xxx",
    events: ["notification.failed"],
    signingSecret: "whsec_f6e5d4c3b2a1f6e5d4c3b2a1f6e5d4c3",
    enabled: true,
    lastTriggeredAt: "2025-06-25T09:15:00Z",
    successRate: 100,
    createdAt: "2025-02-01T00:00:00Z",
  },
  {
    id: "wh_3",
    projectId: "proj_1",
    url: "https://api.stripe.com/webhooks",
    events: ["campaign.started", "campaign.completed"],
    signingSecret: "whsec_9a8b7c6d5e4f9a8b7c6d5e4f9a8b7c6d",
    enabled: false,
    successRate: 85.5,
    createdAt: "2025-03-10T00:00:00Z",
  },
];

function generateSecret() {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return `whsec_${Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("")}`;
}
function successColor(rate: number) {
  if (rate >= 95) return "text-success";
  if (rate >= 80) return "text-warning";
  return "text-destructive";
}
function DetailRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-3">
      <dt className="shrink-0 pt-px text-xs text-muted-foreground">{label}</dt>
      <dd className="min-w-0 text-right">{children}</dd>
    </div>
  );
}
// ---------------------------------------------------------------------------
// Right-side detail panel
// ---------------------------------------------------------------------------

function WebhookDetailPanel({
  webhook,
  onToggle,
  onDelete,
  onRotateSecret,
  onClose,
}: {
  webhook: Webhook;
  onToggle: (enabled: boolean) => void;
  onDelete: () => void;
  onRotateSecret: () => void;
  onClose: () => void;
}) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(webhook.signingSecret);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  };

  return (
    <DetailPanel
      onClose={onClose}
      header={
        <div className="space-y-1.5">
          <div className="flex items-center gap-2">
            <span
              className={`size-2 shrink-0 rounded-full ${webhook.enabled ? "bg-success" : "bg-muted-foreground/40"}`}
            />
            <h3 className="truncate font-semibold text-foreground">
              {webhook.url}
            </h3>
          </div>
          <code className="block truncate font-mono text-xs text-muted-foreground">
            {webhook.id}
          </code>
        </div>
      }
    >

      {/* Scrollable body */}
      <div className="flex-1 min-h-0 overflow-y-auto">
        {/* Overview */}
        <div className="px-5 pt-4 pb-3">
          <h4 className="mb-2.5 text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
            Overview
          </h4>
          <dl className="space-y-2">
            <DetailRow label="URL">
              <span className="block break-all font-mono text-xs text-foreground">
                {webhook.url}
              </span>
            </DetailRow>
            <DetailRow label="Events">
              <div className="flex flex-wrap justify-end gap-1">
                {webhook.events.map((e) => (
                  <Badge key={e} variant="outline" className="text-[10px]">
                    {e}
                  </Badge>
                ))}
              </div>
            </DetailRow>
            <DetailRow label="Status">
              <Badge
                variant={webhook.enabled ? "default" : "secondary"}
                className={`text-[10px] font-medium ${webhook.enabled ? "bg-success/15 text-success border-success/20" : ""}`}
              >
                {webhook.enabled ? "Active" : "Disabled"}
              </Badge>
            </DetailRow>
            <DetailRow label="Success rate">
              <span className={`text-xs font-medium ${webhook.lastTriggeredAt ? successColor(webhook.successRate) : "text-muted-foreground"}`}>
                {webhook.lastTriggeredAt ? `${webhook.successRate}%` : "—"}
              </span>
            </DetailRow>
            <DetailRow label="Last triggered">
              <span className="text-xs text-foreground">
                {webhook.lastTriggeredAt
                  ? format(new Date(webhook.lastTriggeredAt), "MMM d, HH:mm")
                  : "Never"}
              </span>
            </DetailRow>
            <DetailRow label="Created">
              <span className="text-xs text-foreground">
                {format(new Date(webhook.createdAt), "MMM d, yyyy")}
              </span>
            </DetailRow>
          </dl>
        </div>

        <div className="mx-5 h-px bg-border" />

        {/* Signing secret */}
        <div className="px-5 py-4">
          <h4 className="mb-2.5 text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
            Signing secret
          </h4>
          <p className="mb-2.5 text-xs text-muted-foreground">
            Included as an HMAC signature header with every delivery.
          </p>
          <div className="flex items-center gap-1.5">
            <code className="min-w-0 flex-1 truncate rounded-md bg-muted/50 px-2.5 py-1.5 font-mono text-xs">
              {`${webhook.signingSecret.slice(0, 7)}••••••••••••`}
            </code>
            <Button variant="ghost" size="icon-xs" onClick={handleCopy} aria-label="Copy signing secret">
              {copied ? <Check className="size-3 text-success" /> : <Copy className="size-3" />}
            </Button>
            <Button variant="ghost" size="icon-xs" onClick={onRotateSecret} aria-label="Rotate signing secret">
              <RotateCcw className="size-3" />
            </Button>
          </div>
        </div>
      </div>

      {/* Sticky footer */}
      <div className="flex items-center justify-between gap-3 border-t bg-muted/30 px-5 py-3">
        <div className="flex items-center gap-2">
          <Switch checked={webhook.enabled} onCheckedChange={onToggle} />
          <span className="text-xs text-muted-foreground">Enabled</span>
        </div>
        <Button size="sm" variant="destructive" className="gap-1.5" onClick={onDelete}>
          <Trash2 className="size-3.5" /> Delete
        </Button>
      </div>
    </DetailPanel>
  );
}
// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function WebhooksPage() {
  const [webhooks, setWebhooks] = useState(MOCK_WEBHOOKS);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Webhook | null>(null);

  const [newUrl, setNewUrl] = useState("");
  const [newEvents, setNewEvents] = useState<string[]>(["notification.failed"]);
  const [urlError, setUrlError] = useState<string | null>(null);

  const selected = useMemo(
    () => webhooks.find((w) => w.id === selectedId) ?? null,
    [webhooks, selectedId],
  );

  const stats = useMemo(() => {
    const triggered = webhooks.filter((w) => w.lastTriggeredAt);
    const avgSuccess =
      triggered.length > 0
        ? Math.round(triggered.reduce((acc, w) => acc + w.successRate, 0) / triggered.length)
        : null;
    const lastTriggered =
      triggered.length > 0
        ? triggered.reduce((a, b) =>
            new Date(a.lastTriggeredAt!) > new Date(b.lastTriggeredAt!) ? a : b,
          ).lastTriggeredAt!
        : null;
    return {
      total: webhooks.length,
      active: webhooks.filter((w) => w.enabled).length,
      avgSuccess,
      lastTriggered,
    };
  }, [webhooks]);

  const toggleEvent = (key: string) => {
    setNewEvents((prev) =>
      prev.includes(key) ? prev.filter((e) => e !== key) : [...prev, key],
    );
  };

  const handleCreate = () => {
    const url = newUrl.trim();
    if (!/^https:\/\/.+\..+/.test(url)) {
      setUrlError("Enter a valid HTTPS URL.");
      return;
    }
    if (newEvents.length === 0) return;
    const webhook: Webhook = {
      id: `wh_${Date.now().toString(36)}`,
      projectId: "proj_1",
      url,
      events: newEvents,
      signingSecret: generateSecret(),
      enabled: true,
      successRate: 100,
      createdAt: new Date().toISOString(),
    };
    setWebhooks((prev) => [webhook, ...prev]);
    setSelectedId(webhook.id);
    setCreateOpen(false);
    setNewUrl("");
    setNewEvents(["notification.failed"]);
    setUrlError(null);
  };

  const handleDelete = () => {
    if (!deleteTarget) return;
    setWebhooks((prev) => prev.filter((w) => w.id !== deleteTarget.id));
    setSelectedId((prev) => (prev === deleteTarget.id ? null : prev));
    setDeleteTarget(null);
  };

  const columns: ColumnDef<Webhook, unknown>[] = [
    {
      accessorKey: "url",
      header: "Endpoint",
      cell: ({ row }) => (
        <div className="min-w-0">
          <p className="max-w-[240px] truncate font-mono text-xs" title={row.original.url}>{row.original.url}</p>
          <p className="mt-0.5 text-[11px] text-muted-foreground">
            Created {format(new Date(row.original.createdAt), "MMM d, yyyy")}
          </p>
        </div>
      ),
    },
    {
      accessorKey: "events",
      header: "Events",
      cell: ({ row }) => (
        <div className="flex flex-wrap gap-1">
          {row.original.events.slice(0, 2).map((e) => (
            <Badge key={e} variant="secondary" className="text-[10px]">{e}</Badge>
          ))}
          {row.original.events.length > 2 && (
            <Badge variant="secondary" className="text-[10px]">
              +{row.original.events.length - 2}
            </Badge>
          )}
        </div>
      ),
    },
    {
      accessorKey: "successRate",
      header: "Success rate",
      cell: ({ row }) => (
        <span className={`text-sm ${row.original.lastTriggeredAt ? successColor(row.original.successRate) : "text-muted-foreground"}`}>
          {row.original.lastTriggeredAt ? `${row.original.successRate}%` : "—"}
        </span>
      ),
    },
    {
      accessorKey: "lastTriggeredAt",
      header: "Last triggered",
      cell: ({ row }) => (
        <span className="whitespace-nowrap text-xs text-muted-foreground">
          {row.original.lastTriggeredAt
            ? format(new Date(row.original.lastTriggeredAt), "MMM d, HH:mm")
            : "Never"}
        </span>
      ),
    },
    {
      accessorKey: "enabled",
      header: "Status",
      cell: ({ row }) => (
        <Badge
          variant={row.original.enabled ? "default" : "secondary"}
          className={row.original.enabled ? "bg-success/15 text-success border-success/20" : ""}
        >
          {row.original.enabled ? "Active" : "Disabled"}
        </Badge>
      ),
    },
  ];

  return (
    <div className="space-y-6">
      <PageHeader
        title="Webhooks"
        description={`${webhooks.length} endpoint${webhooks.length === 1 ? "" : "s"}`}
        actions={
          <Button size="sm" className="gap-1.5" onClick={() => setCreateOpen(true)}>
            <Plus className="size-3.5" /> Add endpoint
          </Button>
        }
      />

      <div className="grid gap-4 sm:grid-cols-2 md:grid-cols-4">
        <StatCard label="Total endpoints" value={stats.total} />
        <StatCard label="Active" value={stats.active} />
        <StatCard
          label="Avg success rate"
          value={stats.avgSuccess !== null ? `${stats.avgSuccess}%` : "—"}
        />
        <StatCard
          label="Last triggered"
          value={
            stats.lastTriggered
              ? format(new Date(stats.lastTriggered), "MMM d, HH:mm")
              : "—"
          }
        />
      </div>

      <div className="flex min-h-0 flex-1 items-stretch">
        <div className="min-w-0 flex-1">
          <DataTable
            columns={columns}
            data={webhooks}
            searchKey="url"
            searchPlaceholder="Search by URL..."
            pageSize={10}
            tableClassName="bg-card"
            emptyMessage="No endpoints yet. Add one to start receiving event deliveries."
            onRowClick={(row) =>
              setSelectedId((prev) => (prev === row.id ? null : row.id))
            }
          />
        </div>

        {selected && (
          <WebhookDetailPanel
            key={selected.id}
            webhook={selected}
            onToggle={(enabled) =>
              setWebhooks((prev) =>
                prev.map((w) => (w.id === selected.id ? { ...w, enabled } : w)),
              )
            }
            onDelete={() => setDeleteTarget(selected)}
            onRotateSecret={() =>
              setWebhooks((prev) =>
                prev.map((w) =>
                  w.id === selected.id ? { ...w, signingSecret: generateSecret() } : w,
                ),
              )
            }
            onClose={() => setSelectedId(null)}
          />
        )}
      </div>

      <AvailableWebhookEvents />

      {/* Create dialog */}
      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add endpoint</DialogTitle>
            <DialogDescription>
              Events are delivered as signed POST requests to this URL.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-1.5">
              <Label>URL</Label>
              <Input
                value={newUrl}
                onChange={(e) => setNewUrl(e.target.value)}
                placeholder="https://api.example.com/webhooks/notifi"
                className="font-mono text-sm"
              />
              {urlError && <p className="text-xs text-destructive">{urlError}</p>}
            </div>
            <div className="space-y-1.5">
              <Label>Events</Label>
              <div className="space-y-3 rounded-md border p-3 max-h-64 overflow-y-auto">
                {WEBHOOK_EVENTS.map((group) => (
                  <div key={group.label}>
                    <p className="mb-1.5 text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
                      {group.label}
                    </p>
                    <div className="space-y-1.5">
                      {group.events.map((event) => (
                        <label key={event.key} className="flex cursor-pointer items-center gap-2 text-sm">
                          <Checkbox
                            checked={newEvents.includes(event.key)}
                            onCheckedChange={() => toggleEvent(event.key)}
                          />
                          <code className="font-mono text-xs">{event.key}</code>
                        </label>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
              {newEvents.length === 0 && (
                <p className="text-xs text-destructive">Select at least one event.</p>
              )}
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setCreateOpen(false)}>Cancel</Button>
            <Button onClick={handleCreate} disabled={newEvents.length === 0}>
              Add endpoint
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete dialog */}
      <Dialog open={!!deleteTarget} onOpenChange={() => setDeleteTarget(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete endpoint</DialogTitle>
            <DialogDescription>
              Permanently delete deliveries to{" "}
              <code className="font-mono text-xs">{deleteTarget?.url}</code>?
              This cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteTarget(null)}>Cancel</Button>
            <Button variant="destructive" onClick={handleDelete}>Delete</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

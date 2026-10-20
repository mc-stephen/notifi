"use client";

import { useMemo, useState } from "react";
import { useRouter } from "next/navigation";
import { type ColumnDef } from "@tanstack/react-table";
import { PageHeader } from "@/components/custom/page-header";
import { StatCard } from "@/components/custom/stat-card";
import { DataTable } from "@/components/custom/data-table";
import { DetailPanel } from "@/components/custom/detail-panel";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  AlertTriangle,
  Archive,
  Download,
  Loader2,
  Pause,
  Play,
  Plus,
  Trash2,
  Undo2,
  Webhook as WebhookIcon,
} from "lucide-react";
import { toast } from "sonner";
import { useProjectStore } from "@/store/project-store";
import {
  createWebhook,
  deleteWebhook,
  patchWebhook,
  useWebhooks,
  type ServerWebhook,
} from "@/hooks/use-webhooks";
import { getFlow } from "@/hooks/use-flows";
import { downloadJson, fileSlug } from "@/lib/utils";
import { EventsEditor } from "@/components/custom/webhooks/events-editor";
import { WebhookDetailPanel } from "@/components/custom/webhooks/webhook-detail-panel";

export default function WebhooksPage() {
  const router = useRouter();
  const projectId = useProjectStore((s) => s.currentProject?.id);
  const { webhooks, loading, error, refresh } = useWebhooks();

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [createName, setCreateName] = useState("");
  const [createDescription, setCreateDescription] = useState("");
  const [createEvents, setCreateEvents] = useState<string[]>([]);
  const [submitting, setSubmitting] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<ServerWebhook | null>(null);

  const selected = webhooks.find((w) => w.id === selectedId) ?? null;

  const active = useMemo(
    () => webhooks.filter((w) => w.archivedAt == null),
    [webhooks],
  );
  const archived = useMemo(
    () => webhooks.filter((w) => w.archivedAt != null),
    [webhooks],
  );

  const stats = useMemo(() => {
    const live = active.filter((w) => w.active);
    const events = new Set<string>();
    for (const w of live) {
      for (const e of w.events) events.add(e);
    }
    return {
      live: live.length,
      total: webhooks.length,
      events: events.size,
      archived: archived.length,
    };
  }, [active, archived, webhooks]);

  const openCreate = () => {
    setCreateName("");
    setCreateDescription("");
    setCreateEvents([]);
    setCreateOpen(true);
  };

  const handleCreate = async () => {
    if (!projectId) return;
    setSubmitting(true);
    try {
      await createWebhook(projectId, {
        name: createName,
        description: createDescription || undefined,
        events: createEvents,
      });
      setCreateOpen(false);
      await refresh();
      toast.success("Webhook created — its flow is ready in the builder.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't create webhook.");
    } finally {
      setSubmitting(false);
    }
  };

  const toggleActive = async (webhook: ServerWebhook) => {
    if (!projectId) return;
    try {
      await patchWebhook(projectId, webhook.id, { active: !webhook.active });
      await refresh();
      toast.success(webhook.active ? "Webhook turned off." : "Webhook turned on.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't update webhook.");
    }
  };

  const toggleArchived = async (webhook: ServerWebhook, restore: boolean) => {
    if (!projectId) return;
    try {
      await patchWebhook(projectId, webhook.id, { archived: restore });
      await refresh();
      toast.success(restore ? "Webhook restored." : "Webhook archived.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't update webhook.");
    }
  };

  /** Exports the webhook's flow before a terminal delete. */
  const downloadFlowData = async (webhook: ServerWebhook) => {
    if (!projectId) return;
    try {
      const flow = await getFlow(projectId, webhook.flowId);
      downloadJson(`notifi-flow-${fileSlug(webhook.flowName || webhook.name)}.json`, {
        kind: "notifi-flow-export",
        exportedAt: new Date().toISOString(),
        webhook: {
          name: webhook.name,
          description: webhook.description,
          events: webhook.events,
        },
        flow: {
          name: flow.name,
          definition: flow.definition,
          version: flow.version,
        },
      });
      toast.success("Flow data downloaded.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't export flow data.");
    }
  };

  const handleDelete = async () => {
    if (!projectId || !deleteTarget) return;
    try {
      await deleteWebhook(projectId, deleteTarget.id);
      if (selectedId === deleteTarget.id) setSelectedId(null);
      setDeleteTarget(null);
      await refresh();
      toast.success("Webhook deleted. Its flow went with it.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't delete webhook.");
    }
  };

  const columns: ColumnDef<ServerWebhook>[] = [
    {
      accessorKey: "name",
      header: "Webhook",
      cell: ({ row }) => (
        <div className="min-w-0">
          <div className="truncate text-sm font-medium">{row.original.name}</div>
          <div className="truncate text-xs text-muted-foreground">
            {row.original.description || "—"}
          </div>
        </div>
      ),
    },
    {
      accessorKey: "flowName",
      header: "Flow",
      cell: ({ row }) => (
        <button
          type="button"
          className="truncate text-sm hover:underline"
          onClick={(e) => {
            e.stopPropagation();
            router.push(`/flows/${row.original.flowId}`);
          }}
        >
          {row.original.flowName || "—"}
        </button>
      ),
    },
    {
      accessorKey: "events",
      header: "Events",
      cell: ({ row }) => {
        const events = row.original.events;
        if (events.length === 0) return <span className="text-xs text-muted-foreground">Any</span>;
        return (
          <div className="flex max-w-64 flex-wrap items-center gap-1">
            {events.slice(0, 2).map((e) => (
              <span
                key={e}
                className="rounded-md border border-border bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground"
              >
                {e}
              </span>
            ))}
            {events.length > 2 && (
              <span className="text-[10px] text-muted-foreground">
                +{events.length - 2}
              </span>
            )}
          </div>
        );
      },
    },
    {
      accessorKey: "active",
      header: "Status",
      cell: ({ row }) => {
        const w = row.original;
        if (w.archivedAt != null) {
          return <Badge className="border-warning/20 bg-warning/15 text-warning">Archived</Badge>;
        }
        return (
          <Badge
            variant="secondary"
            className={
              w.active ? "border-success/20 bg-success/15 text-success" : ""
            }
          >
            {w.active ? "Active" : "Off"}
          </Badge>
        );
      },
    },
  ];

  return (
    <div className="space-y-6">
      <PageHeader
        title="Webhooks"
        description={`${stats.live} active webhook${stats.live === 1 ? "" : "s"}`}
        actions={
          <Button size="sm" className="gap-1.5" onClick={openCreate} disabled={!projectId}>
            <Plus className="size-3.5" /> New webhook
          </Button>
        }
      />

      {!error && (webhooks.length > 0 || loading) && (
        <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
          <StatCard
            label="Active webhooks"
            value={loading ? "–" : stats.live}
            icon={WebhookIcon}
          />
          <StatCard label="Events tracked" value={loading ? "–" : stats.events} sub="Distinct trigger names" icon={Play} />
          <StatCard
            label="Owned flows"
            value={loading ? "–" : stats.total}
            sub="One flow per webhook"
            icon={Archive}
          />
          <StatCard
            label="Archived"
            value={loading ? "–" : stats.archived}
            sub="Parked, not firing"
            icon={Pause}
          />
        </div>
      )}

      <div className="flex items-start">
        <div className="min-w-0 flex-1">
          {loading ? (
            <div className="overflow-hidden rounded-lg border bg-card">
              <div className="flex items-center justify-center py-16 text-muted-foreground">
                <Loader2 className="mr-2 size-5 animate-spin" /> Loading webhooks…
              </div>
            </div>
          ) : error ? (
            <div className="overflow-hidden rounded-lg border bg-card">
              <div className="flex flex-col items-center justify-center gap-3 py-16 text-center">
                <AlertTriangle className="size-8 text-warning" />
                <p className="text-sm text-muted-foreground">{error}</p>
                <Button size="sm" variant="outline" onClick={() => void refresh()}>
                  Retry
                </Button>
              </div>
            </div>
          ) : (
            <DataTable
              columns={columns}
              data={active}
              searchPlaceholder="Search webhooks..."
              emptyMessage="No webhooks yet. Create one to fire a flow on an inbound call."
              onRowClick={(row) => setSelectedId(row.id)}
              rowActions={(webhook) => (
                <div className="flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    title={webhook.active ? "Turn off" : "Turn on"}
                    onClick={(e) => {
                      e.stopPropagation();
                      void toggleActive(webhook);
                    }}
                  >
                    {webhook.active ? (
                      <Pause className="size-3" />
                    ) : (
                      <Play className="size-3" />
                    )}
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    title="Delete"
                    onClick={(e) => {
                      e.stopPropagation();
                      setDeleteTarget(webhook);
                    }}
                  >
                    <Trash2 className="size-3" />
                  </Button>
                </div>
              )}
            />
          )}
        </div>

        {selected && projectId && (
          <WebhookDetailPanel
            key={selected.id}
            webhook={selected}
            projectId={projectId}
            onClose={() => setSelectedId(null)}
            onChanged={(next) => {
              setSelectedId(next.id);
              void refresh();
            }}
            onDeleteRequest={setDeleteTarget}
          />
        )}
      </div>

      {!error && !loading && archived.length > 0 && (
        <section className="space-y-2">
          <div className="flex items-center gap-2 px-0.5">
            <Archive className="size-3.5 text-muted-foreground" />
            <h2 className="text-sm font-medium text-muted-foreground">Archived</h2>
            <span className="text-xs text-muted-foreground">
              {archived.length} parked · kept, not firing
            </span>
          </div>
          <DataTable
            columns={columns}
            data={archived}
            searchPlaceholder="Search archived..."
            emptyMessage="Nothing archived."
            onRowClick={(row) => setSelectedId(row.id)}
            rowActions={(webhook) => (
              <div className="flex items-center gap-1">
                <Button
                  variant="ghost"
                  size="icon-xs"
                  title="Restore"
                  onClick={(e) => {
                    e.stopPropagation();
                    void toggleArchived(webhook, true);
                  }}
                >
                  <Undo2 className="size-3" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon-xs"
                  title="Delete"
                  onClick={(e) => {
                    e.stopPropagation();
                    setDeleteTarget(webhook);
                  }}
                >
                  <Trash2 className="size-3" />
                </Button>
              </div>
            )}
          />
        </section>
      )}

      {createOpen && (
        <DetailPanel
          onClose={() => setCreateOpen(false)}
          header={
            <div>
              <h3 className="truncate font-semibold text-foreground">New webhook</h3>
              <p className="mt-0.5 truncate text-xs text-muted-foreground">
                Creates and owns one flow — build it right after.
              </p>
            </div>
          }
        >
          <div className="min-h-0 flex-1 overflow-y-auto">
            <div className="space-y-4 px-5 py-4">
              <div className="space-y-1.5">
                <Label className="text-sm font-medium">Webhook name</Label>
                <Input
                  value={createName}
                  onChange={(e) => setCreateName(e.target.value)}
                  placeholder="e.g. Order intake"
                  className="h-9 text-sm"
                />
              </div>
              <div className="space-y-1.5">
                <Label className="text-sm font-medium">Description</Label>
                <Input
                  value={createDescription}
                  onChange={(e) => setCreateDescription(e.target.value)}
                  placeholder="What fires this webhook?"
                  className="h-9 text-sm"
                />
              </div>
              <div className="space-y-1.5">
                <Label className="text-sm font-medium">Events</Label>
                <EventsEditor events={createEvents} onChange={setCreateEvents} />
              </div>
            </div>
          </div>
          <div className="flex gap-2 border-t border-border p-4">
            <Button
              variant="outline"
              size="sm"
              className="flex-1"
              onClick={() => setCreateOpen(false)}
            >
              Cancel
            </Button>
            <Button
              size="sm"
              className="flex-1"
              onClick={() => void handleCreate()}
              disabled={submitting || !createName.trim()}
            >
              {submitting ? "Creating…" : "Create webhook"}
            </Button>
          </div>
        </DetailPanel>
      )}

      <Dialog open={!!deleteTarget} onOpenChange={() => setDeleteTarget(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete webhook</DialogTitle>
            <DialogDescription>
              Permanently delete {deleteTarget?.name ?? "this webhook"} <em>and
              its flow</em>? The trigger dies with it — the inbound URL stops
              working immediately, and this cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <p className="rounded-lg border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning">
            Callers still POSTing to this webhook will start failing. If you
            may want it back, archive instead — archived webhooks keep their
            flow and URL, they just stop firing.
          </p>
          <DialogFooter className="items-center">
            <Button
              variant="ghost"
              size="sm"
              className="gap-1.5"
              disabled={!deleteTarget}
              onClick={() => {
                if (deleteTarget) void downloadFlowData(deleteTarget);
              }}
            >
              <Download className="size-3.5" /> Download flow data
            </Button>
            <div className="flex-1" />
            {deleteTarget?.archivedAt == null && (
              <Button
                variant="outline"
                onClick={() => {
                  void toggleArchived(deleteTarget!, true);
                  setDeleteTarget(null);
                }}
              >
                <Archive className="mr-1.5 size-3.5" /> Archive instead
              </Button>
            )}
            <Button variant="outline" onClick={() => setDeleteTarget(null)}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={() => void handleDelete()}>
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

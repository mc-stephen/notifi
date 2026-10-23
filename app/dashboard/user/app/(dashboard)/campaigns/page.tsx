"use client";

import { useMemo, useState } from "react";
import { type ColumnDef } from "@tanstack/react-table";
import { PageHeader } from "@/components/custom/page-header";
import { StatCard } from "@/components/custom/stat-card";
import { ChannelBadge } from "@/components/custom/channel-badge";
import { DataTable } from "@/components/custom/data-table";
import { DetailPanel } from "@/components/custom/detail-panel";
import { Button } from "@/components/ui/button";
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
  AlertTriangle,
  CheckCircle2,
  Download,
  Loader2,
  Lock,
  Megaphone,
  Pause,
  Play,
  Plus,
  Rocket,
  Trash2,
  Users,
} from "lucide-react";
import { toast } from "sonner";
import { useProjectStore } from "@/store/project-store";
import { downloadJson, fileSlug } from "@/lib/utils";
import type { NotificationChannel } from "@/lib/types";
import {
  CampaignFormFields,
  blankCampaignForm,
  campaignFormError,
  formToCreateInput,
  labelForChannel,
  type CampaignFormValue,
} from "@/components/custom/campaigns/campaign-form";
import { CampaignDetailPanel } from "@/components/custom/campaigns/campaign-detail-panel";
import {
  campaignAction,
  createCampaign,
  deleteCampaign,
  useCampaigns,
  type CampaignStatus,
  type ServerCampaignWithStats,
} from "@/hooks/use-campaigns";
import { useProviderConfigs } from "@/hooks/use-provider-configs";

/** Channels ChannelBadge renders (its icon/label maps are union-typed). */
const BADGE_CHANNELS = new Set<string>([
  "email",
  "sms",
  "push-android",
  "push-ios",
  "web-push",
  "linux",
  "macos",
  "rcs",
  "whatsapp",
  "slack",
  "discord",
  "teams",
  "telegram",
  "webhook",
]);

function fmtTime(iso: string | null): string {
  if (!iso) return "—";
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? "—" : format(d, "MMM d, HH:mm");
}

const STATUS_BADGE: Record<CampaignStatus, string> = {
  draft: "border-border text-muted-foreground",
  scheduled: "border-info/20 bg-info/15 text-info",
  running: "border-success/20 bg-success/15 text-success",
  paused: "border-warning/20 bg-warning/15 text-warning",
  completed: "border-success/20 bg-success/15 text-success",
  cancelled: "border-border text-muted-foreground",
  failed: "border-destructive/20 bg-destructive/15 text-destructive",
};

/** Campaigns deliver flow-less: Run now is a draft-only action, and
    only the worker itself fires scheduled campaigns. */
const RUNNABLE: CampaignStatus[] = ["draft"];
const MOVABLE: CampaignStatus[] = ["running", "paused"];

export default function CampaignsPage() {
  const projectId = useProjectStore((s) => s.currentProject?.id);
  const { campaigns, loading, error, refresh } = useCampaigns();
  const { configs } = useProviderConfigs();

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [composeOpen, setComposeOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] =
    useState<ServerCampaignWithStats | null>(null);
  const [form, setForm] = useState<CampaignFormValue>(blankCampaignForm);
  const [submitting, setSubmitting] = useState(false);
  const [statusFilter, setStatusFilter] = useState<"all" | CampaignStatus>(
    "all",
  );

  const selected = campaigns.find((c) => c.id === selectedId) ?? null;

  const filtered = useMemo(
    () =>
      statusFilter === "all"
        ? campaigns
        : campaigns.filter((c) => c.status === statusFilter),
    [campaigns, statusFilter],
  );

  const stats = useMemo(() => {
    const recipients = campaigns.reduce((acc, c) => acc + c.totals.total, 0);
    const delivered = campaigns.reduce((acc, c) => acc + c.totals.delivered, 0);
    const failed = campaigns.reduce((acc, c) => acc + c.totals.failed, 0);
    const pending = campaigns.reduce((acc, c) => acc + c.totals.pending, 0);
    const sent = delivered + failed;
    return {
      campaigns: campaigns.length,
      recipients,
      delivered,
      failed,
      pending,
      sent,
      inFlight: campaigns.filter(
        (c) => c.status === "running" || c.status === "paused",
      ).length,
      rate: sent > 0 ? Math.round((delivered / sent) * 100) : null,
    };
  }, [campaigns]);

  const updateRow = (next: ServerCampaignWithStats) => {
    setSelectedId(next.id);
    void refresh();
  };

  // Render-safe form state (no time comparisons): surfaces the blocking
  // problem inline above the composer footer.
  const scheduleConfigured = configs.some(
    (c) => c.channel_id === form.channel && c.enabled,
  );
  const formProblem = campaignFormError(form, scheduleConfigured);

  const quickAction = async (
    campaign: ServerCampaignWithStats,
    action: "run-now" | "pause" | "resume",
    message: string,
  ) => {
    if (!projectId) return;
    try {
      await campaignAction(projectId, campaign.id, action);
      await refresh();
      toast.success(message);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't update campaign.");
    }
  };

  const handleCreate = async () => {
    if (!projectId) return;
    const problem = campaignFormError(
      form,
      configs.some((c) => c.channel_id === form.channel && c.enabled),
    );
    if (problem) {
      toast.error(problem);
      return;
    }
    setSubmitting(true);
    try {
      const created = await createCampaign(
        projectId,
        formToCreateInput(form, configs),
      );
      setComposeOpen(false);
      setForm(blankCampaignForm());
      await refresh();
      // Open the new campaign so the composer hands over to its detail.
      setSelectedId(created.id);
      toast.success(
        form.timing === "scheduled"
          ? "Campaign scheduled — the worker fires it once the time passes."
          : "Campaign drafted — send it whenever it's ready.",
      );
    } catch (e) {
      toast.error(
        e instanceof Error ? e.message : "Couldn't create campaign.",
      );
    } finally {
      setSubmitting(false);
    }
  };

  /** Report = the campaign snapshot + derived delivery totals. */
  const downloadReport = (campaign: ServerCampaignWithStats) => {
    downloadJson(
      `notifi-campaign-report-${fileSlug(campaign.name)}.json`,
      {
        kind: "notifi-campaign-report",
        exportedAt: new Date().toISOString(),
        campaign: {
          id: campaign.id,
          name: campaign.name,
          description: campaign.description,
          channel: campaign.channel,
          templateId: campaign.templateId,
          title: campaign.title,
          body: campaign.body,
          audienceMode: campaign.audienceMode,
          selectedUserIds: campaign.selectedUserIds,
          status: campaign.status,
          sendAt: campaign.sendAt,
          startedAt: campaign.startedAt,
          completedAt: campaign.completedAt,
          lastError: campaign.lastError,
          createdAt: campaign.createdAt,
        },
        totals: campaign.totals,
        estimatedAudience: campaign.estimatedAudience,
      },
    );
    toast.success("Delivery report downloaded.");
  };

  const handleDelete = async () => {
    if (!projectId || !deleteTarget) return;
    try {
      await deleteCampaign(projectId, deleteTarget.id);
      if (selectedId === deleteTarget.id) setSelectedId(null);
      setDeleteTarget(null);
      await refresh();
      toast.success("Campaign deleted. Delivery history stays.");
    } catch (e) {
      toast.error(
        e instanceof Error ? e.message : "Couldn't delete campaign.",
      );
    }
  };

  const columns: ColumnDef<ServerCampaignWithStats>[] = [
    {
      accessorKey: "name",
      header: "Campaign",
      cell: ({ row }) => (
        <div className="min-w-0">
          <div className="truncate text-sm font-medium">
            {row.original.name}
          </div>
          <div className="truncate text-xs text-muted-foreground">
            {row.original.description || row.original.title || "—"}
          </div>
        </div>
      ),
    },
    {
      accessorKey: "channel",
      header: "Channel",
      cell: ({ row }) => {
        const channel = row.original.channel;
        return BADGE_CHANNELS.has(channel) ? (
          <ChannelBadge channel={channel as NotificationChannel} />
        ) : (
          <Badge variant="secondary" className="font-medium">
            {labelForChannel(channel)}
          </Badge>
        );
      },
    },
    {
      accessorKey: "status",
      header: "Status",
      cell: ({ row }) => (
        <Badge
          variant="secondary"
          className={`text-xs capitalize ${STATUS_BADGE[row.original.status] ?? ""}`}
        >
          {row.original.status}
        </Badge>
      ),
    },
    {
      id: "audience",
      header: "Audience",
      cell: ({ row }) => {
        const c = row.original;
        const count =
          c.estimatedAudience != null
            ? c.estimatedAudience.toLocaleString()
            : c.totals.total.toLocaleString();
        // Expanded rows know exactly who was contacted; the estimate is
        // "~N" live for all-mode, exact for a compose-time selection.
        const marker =
          c.estimatedAudience != null && c.audienceMode === "all" ? "~" : "";
        return (
          <span className="text-xs whitespace-nowrap tabular-nums text-muted-foreground">
            {marker}
            {count}
            {c.estimatedAudience != null && c.audienceMode === "custom"
              ? " selected"
              : ""}
          </span>
        );
      },
    },
    {
      id: "delivered",
      header: "Delivered",
      cell: ({ row }) => (
        <span className="text-xs whitespace-nowrap tabular-nums">
          {row.original.totals.delivered.toLocaleString()}
          {row.original.totals.failed > 0 && (
            <span className="text-destructive">
              {" "}
              · {row.original.totals.failed.toLocaleString()} failed
            </span>
          )}
          {row.original.totals.pending > 0 && (
            <span className="text-muted-foreground">
              {" "}
              · {row.original.totals.pending.toLocaleString()} pending
            </span>
          )}
        </span>
      ),
    },
    {
      accessorKey: "sendAt",
      header: "Send time",
      cell: ({ row }) => {
        const c = row.original;
        const stamp = c.startedAt ?? c.sendAt;
        return (
          <span className="text-xs whitespace-nowrap text-muted-foreground">
            {stamp ? fmtTime(stamp) : "—"}
          </span>
        );
      },
    },
  ];

  return (
    <div className="space-y-6">
      <PageHeader
        title="Campaigns"
        description={
          stats.inFlight > 0
            ? `${stats.inFlight} in flight · the worker drains pending sends in batches`
            : "One-shot messages to your recipients — delivered by your connected providers"
        }
        actions={
          <Button
            size="sm"
            className="gap-1.5"
            onClick={() => {
              setForm(blankCampaignForm());
              setComposeOpen(true);
            }}
            disabled={!projectId}
          >
            <Plus className="size-3.5" /> New campaign
          </Button>
        }
      />

      {!error && (campaigns.length > 0 || loading) && (
        <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
          <StatCard
            label="Recipients reached"
            value={loading ? "–" : stats.recipients.toLocaleString()}
            icon={Users}
          />
          <StatCard
            label="Delivered"
            value={loading ? "–" : stats.delivered.toLocaleString()}
            sub={
              stats.pending > 0
                ? `${stats.pending.toLocaleString()} pending in the queue`
                : undefined
            }
            icon={CheckCircle2}
          />
          <StatCard
            label="Delivery rate"
            value={loading ? "–" : stats.rate === null ? "—" : `${stats.rate}%`}
            sub={stats.failed > 0 ? `${stats.failed} failed` : undefined}
            icon={Rocket}
          />
          <StatCard
            label="Campaigns"
            value={loading ? "–" : stats.campaigns}
            icon={Megaphone}
          />
        </div>
      )}

      <div className="flex items-start">
        <div className="min-w-0 flex-1">
          <div className="mb-3 flex items-center gap-1.5">
            {(["all", "draft", "scheduled", "running", "paused", "completed", "failed", "cancelled"] as const).map(
              (status) => {
                const count =
                  status === "all"
                    ? campaigns.length
                    : campaigns.filter((c) => c.status === status).length;
                if (status !== "all" && count === 0) return null;
                return (
                  <Button
                    key={status}
                    size="sm"
                    variant={statusFilter === status ? "secondary" : "ghost"}
                    className="h-7 gap-1.5 px-2.5 text-xs capitalize"
                    onClick={() => setStatusFilter(status)}
                  >
                    {status === "all" ? "All" : status}
                    <span className="tabular-nums text-muted-foreground">
                      {count}
                    </span>
                  </Button>
                );
              },
            )}
          </div>

          {loading ? (
            <div className="overflow-hidden rounded-lg border bg-card">
              <div className="flex items-center justify-center py-16 text-muted-foreground">
                <Loader2 className="mr-2 size-5 animate-spin" /> Loading
                campaigns…
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
              data={filtered}
              searchPlaceholder="Search campaigns..."
              emptyMessage="No campaigns yet. Compose one to message your recipients."
              onRowClick={(row) => setSelectedId(row.id)}
              rowActions={(campaign) => (
                <div className="flex items-center gap-1">
                  {RUNNABLE.includes(campaign.status) && (
                    <Button
                      variant="ghost"
                      size="icon-xs"
                      title="Run now"
                      onClick={(e) => {
                        e.stopPropagation();
                        void quickAction(
                          campaign,
                          "run-now",
                          "Campaign fired — the worker is draining the rest.",
                        );
                      }}
                    >
                      <Rocket className="size-3" />
                    </Button>
                  )}
                  {MOVABLE.includes(campaign.status) && (
                    <Button
                      variant="ghost"
                      size="icon-xs"
                      title={campaign.status === "running" ? "Pause" : "Resume"}
                      onClick={(e) => {
                        e.stopPropagation();
                        void (campaign.status === "running"
                          ? quickAction(campaign, "pause", "Campaign paused.")
                          : quickAction(
                              campaign,
                              "resume",
                              "Campaign resumed.",
                            ));
                      }}
                    >
                      {campaign.status === "running" ? (
                        <Pause className="size-3" />
                      ) : (
                        <Play className="size-3" />
                      )}
                    </Button>
                  )}
                  {!MOVABLE.includes(campaign.status) && (
                    <Button
                      variant="ghost"
                      size="icon-xs"
                      title="Delete"
                      onClick={(e) => {
                        e.stopPropagation();
                        setDeleteTarget(campaign);
                      }}
                    >
                      <Trash2 className="size-3" />
                    </Button>
                  )}
                </div>
              )}
            />
          )}
        </div>

        {selected && projectId && (
          <CampaignDetailPanel
            key={selected.id}
            campaign={selected}
            projectId={projectId}
            onClose={() => setSelectedId(null)}
            onChanged={updateRow}
            onDeleteRequest={setDeleteTarget}
          />
        )}

        {composeOpen && projectId && (
          <DetailPanel
            onClose={() => setComposeOpen(false)}
            header={
              <div>
                <h3 className="truncate font-semibold text-foreground">
                  New campaign
                </h3>
                <p className="mt-0.5 truncate text-xs text-muted-foreground">
                  Compose from a template — its render is snapshotted; delivery
                  goes straight through your connected providers.
                </p>
              </div>
            }
            className="w-[min(30rem,94%)]"
          >
            <div className="min-h-0 flex-1 overflow-y-auto">
              <div className="px-5 py-4">
                <CampaignFormFields value={form} onChange={setForm} />
              </div>
            </div>
            <div className="space-y-2 border-t border-border p-4">
              {formProblem && (
                <p className="flex items-start gap-1.5 text-xs text-destructive">
                  <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
                  {formProblem}
                </p>
              )}
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  className="flex-1"
                  onClick={() => setComposeOpen(false)}
                  disabled={submitting}
                >
                  Cancel
                </Button>
                <Button
                  size="sm"
                  className="flex-1"
                  onClick={() => void handleCreate()}
                  disabled={submitting || formProblem !== null}
                >
                  {submitting
                    ? "Creating…"
                    : form.timing === "scheduled"
                      ? "Schedule campaign"
                      : "Create campaign"}
                </Button>
              </div>
            </div>
          </DetailPanel>
        )}
      </div>

      <Dialog open={!!deleteTarget} onOpenChange={() => setDeleteTarget(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete campaign</DialogTitle>
            <DialogDescription>
              Permanently delete {deleteTarget?.name ?? "this campaign"}? This
              cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <p className="rounded-lg border border-border px-3 py-2 text-xs text-muted-foreground">
            Already-delivered sends keep their delivery history — only the
            campaign and its still-pending sends go away.
          </p>
          <DialogFooter className="items-center">
            <Button
              variant="ghost"
              size="sm"
              className="gap-1.5"
              disabled={!deleteTarget}
              onClick={() => {
                if (deleteTarget) downloadReport(deleteTarget);
              }}
            >
              <Download className="size-3.5" /> Download delivery report
            </Button>
            <div className="flex-1" />
            {(deleteTarget?.status === "running" ||
              deleteTarget?.status === "paused") && (
              <span className="flex items-center gap-1.5 text-xs text-warning">
                <Lock className="size-3.5" /> Pause or cancel first
              </span>
            )}
            <Button
              variant="outline"
              onClick={() => setDeleteTarget(null)}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              disabled={
                !deleteTarget ||
                deleteTarget.status === "running" ||
                deleteTarget.status === "paused"
              }
              onClick={() => void handleDelete()}
            >
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

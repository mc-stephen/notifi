"use client";

import { useState } from "react";
import { format, formatDistanceToNow } from "date-fns";
import {
  AlertTriangle,
  Copy,
  Pause,
  Pencil,
  Play,
  Rocket,
  Square,
  Trash2,
} from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { DetailPanel } from "@/components/custom/detail-panel";
import { useProviderConfigs } from "@/hooks/use-provider-configs";
import {
  CAMPAIGN_CHANNELS,
  CampaignFormFields,
  campaignToForm,
  campaignFormError,
  formToPatchInput,
  labelForChannel,
  contactKeyFor,
  channelConfigured,
  type CampaignFormValue,
} from "./campaign-form";
import {
  campaignAction,
  createCampaign,
  patchCampaign,
  type ServerCampaignWithStats,
} from "@/hooks/use-campaigns";

function fmtTime(iso: string | null): string {
  if (!iso) return "—";
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? "—" : format(d, "MMM d, HH:mm");
}

const STATUS_BADGE: Record<string, string> = {
  draft: "border-border",
  scheduled: "border-info/20 bg-info/15 text-info",
  running: "border-success/20 bg-success/15 text-success",
  paused: "border-warning/20 bg-warning/15 text-warning",
  completed: "border-success/20 bg-success/15 text-success",
  cancelled: "border-border text-muted-foreground",
  failed: "border-destructive/20 bg-destructive/15 text-destructive",
};

/**
 * Right-side campaign panel: derived totals, composed message, lifecycle
 * actions, and a compact editor for draft/scheduled campaigns.
 */
export function CampaignDetailPanel({
  campaign,
  projectId,
  onClose,
  onChanged,
  onDeleteRequest,
}: {
  campaign: ServerCampaignWithStats;
  projectId: string;
  onClose: () => void;
  /** Refresh the page list after a mutation (receives the updated row). */
  onChanged: (next: ServerCampaignWithStats) => void;
  onDeleteRequest: (campaign: ServerCampaignWithStats) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [form, setForm] = useState<CampaignFormValue>(() =>
    campaignToForm(campaign),
  );
  const [saving, setSaving] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const { configs: providerConfigs } = useProviderConfigs();

  const editable = campaign.status === "draft" || campaign.status === "scheduled";

  const close = () => onClose();

  const toMessage = (e: unknown, fallback: string) =>
    e instanceof Error ? e.message : fallback;

  const act = async (
    label: string,
    action: "run-now" | "pause" | "resume" | "cancel",
    done: (message: string) => string,
  ) => {
    setBusy(action);
    try {
      const next = await campaignAction(projectId, campaign.id, action);
      toast.success(done(next.status));
      onChanged(next);
    } catch (e) {
      toast.error(toMessage(e, `Couldn't ${label}.`));
    } finally {
      setBusy(null);
    }
  };

  const handleSave = async () => {
    const problem = campaignFormError(
      form,
      channelConfigured(providerConfigs, form.channel),
    );
    if (problem) {
      toast.error(problem);
      return;
    }
    setSaving(true);
    try {
      const updated = await patchCampaign(
        projectId,
        campaign.id,
        formToPatchInput(form, campaign, providerConfigs),
      );
      toast.success("Campaign updated.");
      onChanged(updated);
      setEditing(false);
    } catch (e) {
      toast.error(toMessage(e, "Couldn't update campaign."));
    } finally {
      setSaving(false);
    }
  };

  /** Duplicate = identical draft copy; edit its content before resending. */
  const handleDuplicate = async () => {
    setBusy("duplicate");
    try {
      const copy = await createCampaign(projectId, {
        name: `${campaign.name} (copy)`,
        description: campaign.description || undefined,
        // Template compose re-renders through the same template + channel;
        // the provider pick rides along.
        templateId: campaign.templateId ?? "",
        channel: campaign.channel,
        providerId: campaign.providerId || undefined,
        audienceMode: campaign.audienceMode,
        selectedUserIds:
          campaign.audienceMode === "custom"
            ? campaign.selectedUserIds
            : undefined,
      });
      toast.success("Campaign duplicated as a draft.");
      onChanged(copy);
    } catch (e) {
      toast.error(toMessage(e, "Couldn't duplicate campaign."));
    } finally {
      setBusy(null);
    }
  };

  const totals = campaign.totals;
  const finished =
    campaign.status === "completed" ||
    campaign.status === "cancelled" ||
    campaign.status === "failed";

  // Render-safe form state (no time comparisons) for the inline notice.
  const formProblem = campaignFormError(
    form,
    channelConfigured(providerConfigs, form.channel),
  );

  return (
    <DetailPanel
      onClose={close}
      header={
        <div>
          <div className="flex items-center gap-2">
            <h3 className="min-w-0 truncate font-semibold text-foreground">
              {campaign.name}
            </h3>
            <Badge
              variant="secondary"
              className={`text-xs capitalize ${STATUS_BADGE[campaign.status] ?? ""}`}
            >
              {campaign.status}
            </Badge>
          </div>
          <p className="mt-0.5 truncate text-xs text-muted-foreground">
            {campaign.lastError
              ? campaign.lastError
              : campaign.description
                ? campaign.description
                : campaign.totals.pending > 0
                  ? `${campaign.totals.pending} remaining → the worker sends these in batches`
                  : `${labelForChannel(campaign.channel)} campaign`}
          </p>
        </div>
      }
    >
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="space-y-4 px-5 py-4">
          {/* Actions — lifecycle-appropriate buttons. */}
          {!editing && (
            <div className="flex flex-wrap gap-2">
              {/* Run now is draft-only: scheduled campaigns are the
                  worker's job, not a manual action. */}
              {campaign.status === "draft" && (
                <Button
                  size="sm"
                  className="gap-1.5"
                  disabled={busy !== null}
                  onClick={() => void act("run campaign", "run-now", () => "Campaign fired — the worker is draining the rest.")}
                >
                  {busy === "run-now" ? (
                    <Rocket className="size-3.5 animate-pulse" />
                  ) : (
                    <Rocket className="size-3.5" />
                  )}
                  Run now
                </Button>
              )}
              {campaign.status === "running" && (
                <>
                  <Button
                    size="sm"
                    variant="outline"
                    className="gap-1.5"
                    disabled={busy !== null}
                    onClick={() => void act("pause campaign", "pause", () => "Campaign paused — the pending sends wait.")}
                  >
                    <Pause className="size-3.5" /> Pause
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    className="gap-1.5 text-destructive hover:text-destructive"
                    disabled={busy !== null}
                    onClick={() => void act("cancel campaign", "cancel", () => "Campaign cancelled — pending sends dropped.")}
                  >
                    <Square className="size-3.5" /> Cancel send
                  </Button>
                </>
              )}
              {campaign.status === "paused" && (
                <>
                  <Button
                    size="sm"
                    className="gap-1.5"
                    disabled={busy !== null}
                    onClick={() => void act("resume campaign", "resume", () => "Campaign resumed — back in the worker queue.")}
                  >
                    <Play className="size-3.5" /> Resume
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    className="gap-1.5 text-destructive hover:text-destructive"
                    disabled={busy !== null}
                    onClick={() => void act("cancel campaign", "cancel", () => "Campaign cancelled — pending sends dropped.")}
                  >
                    <Square className="size-3.5" /> Cancel send
                  </Button>
                </>
              )}
              {finished && (
                <Button
                  size="sm"
                  variant="outline"
                  className="gap-1.5"
                  disabled={busy !== null}
                  onClick={() => void handleDuplicate()}
                >
                  <Copy className="size-3.5" /> Duplicate
                </Button>
              )}
              {editable && (
                <Button
                  size="sm"
                  variant="outline"
                  className="gap-1.5"
                  onClick={() => {
                    setForm(campaignToForm(campaign));
                    setEditing(true);
                  }}
                >
                  <Pencil className="size-3.5" /> Edit
                </Button>
              )}
            </div>
          )}

          {/* Derived stats — nothing is stored twice on the server. */}
          {!editing && (
            <div className="grid grid-cols-2 gap-2">
              <Stat label="Recipients" value={totals.total} />
              <Stat
                label={
                  campaign.audienceMode === "custom"
                    ? "Will send to (selected)"
                    : "Will send to (live)"
                }
                value={campaign.estimatedAudience ?? null}
              />
              <Stat label="Delivered" value={totals.delivered} tone="success" />
              <Stat label="Failed" value={totals.failed} tone="destructive" />
              <Stat label="Pending" value={totals.pending} />
            </div>
          )}

          {campaign.lastError && campaign.status === "failed" && (
            <p className="flex items-start gap-2 rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive">
              <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
              {campaign.lastError}
            </p>
          )}

          {!editing && (
            <>
              <Separator />
              <dl className="space-y-2 text-sm">
                <Row label="Channel" value={labelForChannel(campaign.channel)} />
                <Row
                  label="Provider"
                  value={
                    campaign.providerId
                      ? campaign.providerId
                      : "First connected (auto)"
                  }
                />
                {campaign.templateId && (
                  <Row label="Template" value={campaign.templateId} mono />
                )}
                {campaign.sendAt && (
                  <Row
                    label="Scheduled for"
                    value={`${fmtTime(campaign.sendAt)} (${formatDistanceToNow(
                      new Date(campaign.sendAt),
                      {
                        addSuffix: true,
                      },
                    )})`}
                  />
                )}
                {campaign.startedAt && (
                  <Row label="Started" value={fmtTime(campaign.startedAt)} />
                )}
                {campaign.completedAt && (
                  <Row label="Finished" value={fmtTime(campaign.completedAt)} />
                )}
                <Row label="Created" value={fmtTime(campaign.createdAt)} />
              </dl>

              <Separator />

              <div>
                <p className="mb-1 text-xs font-medium text-foreground">
                  Message
                </p>
                <div className="rounded-lg border border-border px-3 py-2">
                  <p className="text-sm font-medium">{campaign.title || "—"}</p>
                  <p className="mt-1 line-clamp-6 text-xs whitespace-pre-wrap text-muted-foreground">
                    {campaign.body || "—"}
                  </p>
                </div>
              </div>

              <div>
                <p className="mb-1 text-xs font-medium text-foreground">
                  Audience
                </p>
                {campaign.audienceMode === "custom" ? (
                  <div className="rounded-lg border border-border px-3 py-2">
                    <p className="text-sm font-medium">
                      {campaign.selectedUserIds.length} selected recipient
                      {campaign.selectedUserIds.length === 1 ? "" : "s"}
                    </p>
                    <p className="mt-1 line-clamp-3 font-mono text-xs break-all text-muted-foreground">
                      {campaign.selectedUserIds.join(", ") || "—"}
                    </p>
                  </div>
                ) : (
                  <p className="rounded-lg border border-border px-3 py-2 text-xs text-muted-foreground">
                    Everyone with a{" "}
                    <span className="text-foreground">
                      {CAMPAIGN_CHANNELS[campaign.channel]?.contact ??
                        contactKeyFor(campaign.channel)}
                    </span>{" "}
                    on file — resolved live at send time.
                  </p>
                )}
              </div>
            </>
          )}

          {editing && (
            <div className="space-y-4 pt-1">
              <CampaignFormFields value={form} onChange={setForm} />
              <div className="space-y-2">
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
                    onClick={() => {
                      setForm(campaignToForm(campaign));
                      setEditing(false);
                    }}
                    disabled={saving}
                  >
                    Cancel
                  </Button>
                  <Button
                    size="sm"
                    className="flex-1"
                    onClick={() => void handleSave()}
                    disabled={saving || formProblem !== null}
                  >
                    {saving ? "Saving…" : "Save changes"}
                  </Button>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>

      <div className="border-t-border border-t p-4">
        <Button
          variant="outline"
          size="sm"
          className="w-full gap-1.5 text-destructive hover:text-destructive"
          onClick={() => onDeleteRequest(campaign)}
          disabled={campaign.status === "running" || campaign.status === "paused"}
        >
          <Trash2 className="size-3.5" /> Delete campaign
        </Button>
        <p className="mt-2 text-center text-xs text-muted-foreground">
          Sends keep their delivery history; delivery goes straight through
          the connected provider for this channel.
        </p>
      </div>
    </DetailPanel>
  );
}

function Stat({
  label,
  value,
  tone,
}: {
  label: string;
  value: number | null;
  tone?: "success" | "destructive";
}) {
  return (
    <div className="rounded-lg border border-border px-3 py-2">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p
        className={`mt-0.5 text-lg font-semibold tabular-nums ${
          tone === "success"
            ? "text-success"
            : tone === "destructive"
              ? "text-destructive"
              : ""
        }`}
      >
        {value === null ? "—" : value.toLocaleString()}
      </p>
    </div>
  );
}

function Row({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="flex items-baseline justify-between gap-3">
      <dt className="shrink-0 text-xs text-muted-foreground">{label}</dt>
      <dd
        className={`min-w-0 truncate text-right text-xs text-foreground ${
          mono ? "font-mono" : ""
        }`}
      >
        {value}
      </dd>
    </div>
  );
}

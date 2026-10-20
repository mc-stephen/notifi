"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import {
  Archive,
  ArrowUpRight,
  Check,
  Copy,
  Pause,
  Play,
  Trash2,
  Undo2,
} from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { DetailPanel } from "@/components/custom/detail-panel";
import { EventsEditor } from "./events-editor";
import { getFlow, asFlowDoc } from "@/hooks/use-flows";
import {
  patchWebhook,
  type ServerWebhook,
} from "@/hooks/use-webhooks";
import { env } from "@/lib/env";

function Field({ label, children, hint }: { label: string; children: React.ReactNode; hint?: string }) {
  return (
    <div className="space-y-1.5">
      <Label className="text-sm font-medium">{label}</Label>
      {children}
      {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
    </div>
  );
}

/**
 * Right-side webhook editor: trigger URL + events (the owner owns the
 * trigger here), webhook identity fields, and an Open-builder button.
 */
export function WebhookDetailPanel({
  webhook,
  projectId,
  onClose,
  onChanged,
  onDeleteRequest,
}: {
  webhook: ServerWebhook;
  projectId: string;
  onClose: () => void;
  /** Refresh the page list after a mutation (receives the updated row). */
  onChanged: (next: ServerWebhook) => void;
  onDeleteRequest: (webhook: ServerWebhook) => void;
}) {
  const router = useRouter();
  const [name, setName] = useState(webhook.name);
  const [description, setDescription] = useState(webhook.description);
  const [events, setEvents] = useState<string[]>(webhook.events);
  const [saving, setSaving] = useState(false);
  const [toggling, setToggling] = useState(false);
  const [archiving, setArchiving] = useState(false);
  /** The owned flow's trigger token — drives the inbound URL display. */
  const [token, setToken] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const archived = webhook.archivedAt != null;

  useEffect(() => {
    let ignore = false;
    getFlow(projectId, webhook.flowId)
      .then((flow) => {
        if (ignore) return;
        const doc = asFlowDoc(flow.definition);
        const trigger = doc?.nodes.find((n) => n.type === "trigger");
        const data = trigger?.data as { token?: unknown } | undefined;
        setToken(
          data && typeof data.token === "string" ? data.token : null,
        );
      })
      .catch(() => setToken(null));
    return () => {
      ignore = true;
    };
  }, [projectId, webhook.flowId]);

  const url = token ? `${env.apiBase}/v1/hooks/${token}` : null;

  const copyUrl = async () => {
    if (!url) return;
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Clipboard unavailable — the URL stays visible for manual copy.
    }
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const updated = await patchWebhook(projectId, webhook.id, {
        name: name.trim(),
        description,
        events,
      });
      toast.success("Webhook updated.");
      onChanged(updated);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't update webhook.");
    } finally {
      setSaving(false);
    }
  };

  const handleToggleActive = async () => {
    setToggling(true);
    try {
      const updated = await patchWebhook(projectId, webhook.id, {
        active: !webhook.active,
      });
      toast.success(webhook.active ? "Webhook turned off." : "Webhook turned on.");
      onChanged(updated);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't update webhook.");
    } finally {
      setToggling(false);
    }
  };

  const handleToggleArchived = async () => {
    setArchiving(true);
    try {
      const updated = await patchWebhook(projectId, webhook.id, {
        archived: !archived,
      });
      toast.success(archived ? "Webhook restored." : "Webhook archived.");
      onChanged(updated);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't update webhook.");
    } finally {
      setArchiving(false);
    }
  };

  return (
    <DetailPanel onClose={onClose} header={
        <div>
          <div className="flex items-center gap-2">
            <h3 className="truncate font-semibold text-foreground">{webhook.name}</h3>
            {archived ? (
              <Badge className="border-warning/20 bg-warning/15 text-[10px] font-medium text-warning">
                Archived
              </Badge>
            ) : (
              <Badge
                variant="secondary"
                className={
                  webhook.active
                    ? "border-success/20 bg-success/15 text-[10px] font-medium text-success"
                    : "text-[10px] font-medium"
                }
              >
                {webhook.active ? "Active" : "Off"}
              </Badge>
            )}
          </div>
          <p className="mt-0.5 truncate text-xs text-muted-foreground">
            {webhook.events.length} event{webhook.events.length === 1 ? "" : "s"} · {webhook.flowName || "flow"}
          </p>
        </div>
      }
    >
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="space-y-4 px-5 py-4">
          <dl className="grid grid-cols-1 gap-x-4 gap-y-3 rounded-lg border border-border bg-card p-3 text-sm">
            <div className="col-span-2">
              <dt className="text-xs text-muted-foreground">Inbound URL</dt>
              <dd className="mt-0.5 font-medium">
                {url ? (
                  <span className="flex items-center gap-1.5">
                    <code className="min-w-0 flex-1 truncate rounded bg-muted px-1.5 py-0.5 font-mono text-[11px]">
                      {url}
                    </code>
                    <Button
                      variant="outline"
                      size="icon-sm"
                      aria-label="Copy inbound URL"
                      className="shrink-0"
                      onClick={() => void copyUrl()}
                    >
                      {copied ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
                    </Button>
                  </span>
                ) : (
                  <span className="font-normal text-muted-foreground">
                    Loading trigger…
                  </span>
                )}
              </dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">Created</dt>
              <dd className="mt-0.5 font-medium">
                {new Date(webhook.createdAt).toLocaleDateString()}
              </dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">Updated</dt>
              <dd className="mt-0.5 font-medium">
                {new Date(webhook.updatedAt).toLocaleDateString()}
              </dd>
            </div>
          </dl>

          <Button
            variant="outline"
            size="sm"
            className="w-full gap-1.5"
            onClick={() => router.push(`/flows/${webhook.flowId}`)}
          >
            Open flow builder <ArrowUpRight className="size-3.5" />
          </Button>

          <Separator />

          <Field label="Webhook name">
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Order intake"
              className="h-9 text-sm"
            />
            <p className="text-xs text-muted-foreground">
              Renames the owned flow to match — one webhook, one flow.
            </p>
          </Field>

          <Field label="Description">
            <Input
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="What fires this webhook?"
              className="h-9 text-sm"
            />
          </Field>

          <Field
            label="Events"
            hint="The trigger rewrites to match these — its token survives, so the inbound URL never changes."
          >
            <EventsEditor events={events} onChange={setEvents} />
          </Field>

          <Button size="sm" className="w-full" onClick={() => void handleSave()} disabled={saving}>
            {saving ? "Saving…" : "Save changes"}
          </Button>
        </div>
      </div>

      <div className="flex gap-2 border-t border-border p-4">
        <Button
          variant="outline"
          size="sm"
          className="flex-1 gap-1.5"
          onClick={() => void handleToggleActive()}
          disabled={toggling || archived}
          title={archived ? "Restore the webhook first" : undefined}
        >
          {webhook.active ? (
            <>
              <Pause className="size-3.5" /> Turn off
            </>
          ) : (
            <>
              <Play className="size-3.5" /> Turn on
            </>
          )}
        </Button>
        <Button
          variant="outline"
          size="sm"
          className="flex-1 gap-1.5"
          onClick={() => void handleToggleArchived()}
          disabled={archiving}
        >
          {archived ? (
            <>
              <Undo2 className="size-3.5" /> Restore
            </>
          ) : (
            <>
              <Archive className="size-3.5" /> Archive
            </>
          )}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          aria-label="Delete webhook"
          className="text-destructive hover:text-destructive"
          onClick={() => onDeleteRequest(webhook)}
        >
          <Trash2 className="size-3.5" />
        </Button>
      </div>
    </DetailPanel>
  );
}

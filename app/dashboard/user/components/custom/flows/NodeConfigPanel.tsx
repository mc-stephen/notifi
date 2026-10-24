"use client";

import { useState } from "react";
import { Check, CalendarClock, Copy, ExternalLink, Webhook, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { DetailPanel } from "@/components/custom/detail-panel";
import { ProviderIcon } from "@/components/custom/providers/provider-icon";
import { useProviderRegistry } from "@/hooks/use-provider-registry";
import { useTemplates } from "@/hooks/use-templates";
import {
  AI_MODELS,
  CONDITION_FIELDS,
  CONDITION_OPERATORS,
  HTTP_METHODS,
  TRIGGER_EVENTS,
  triggerKindOf,
  type AiTransformData,
  type ConditionData,
  type FlowNode,
  type HttpRequestData,
  type ProviderData,
  type StopData,
  type TemplateData,
  type TriggerData,
} from "./flow-types";
import { env } from "@/lib/env";
import { WEBHOOK_EVENTS } from "@/lib/constants";
import { cn } from "@/lib/utils";

type NodeConfigPanelProps = {
  node: FlowNode;
  onChange: (data: FlowNode["data"]) => void;
  onClose: () => void;
  /** Owner flows: the trigger is owned data (schedule/webhook page). */
  lockedTrigger?: boolean;
};

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1.5">
      <Label className="text-sm font-medium">{label}</Label>
      {children}
    </div>
  );
}

/** Read-only event chip list for owner-managed triggers. */
function EventChips({ events }: { events: string[] }) {
  return (
    <div className="flex flex-wrap gap-1.5">
      {events.map((e) => (
        <span
          key={e}
          className="rounded-md border border-border bg-muted px-2 py-0.5 font-mono text-[11px] text-muted-foreground"
        >
          {e}
        </span>
      ))}
    </div>
  );
}

function TriggerForm({
  data,
  onChange,
}: {
  data: TriggerData;
  onChange: (d: TriggerData) => void;
}) {
  const kind = triggerKindOf(data);
  const [copied, setCopied] = useState(false);

  const copyUrl = async (url: string) => {
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Clipboard unavailable — the URL stays visible for manual copy.
    }
  };

  const webhookUrl = data.token ? `${env.apiBase}/v1/hooks/${data.token}` : "";

  // Owner data: the schedule page sets the timing; the builder only
  // writes steps. Fixed card, nothing editable.
  if (kind === "schedule") {
    return (
      <div className="space-y-4">
        <div className="rounded-lg border border-primary/30 bg-primary/5 px-3 py-3">
          <p className="flex items-center gap-2 text-sm font-medium">
            <CalendarClock className="size-4 text-primary" /> Schedule trigger
          </p>
          <p className="mt-1 text-xs text-muted-foreground">
            This flow belongs to a schedule and fires automatically on its
            timing. The trigger is owned data — change the schedule itself
            on the Schedules page.
          </p>
        </div>
        {(data.events?.length ?? 0) > 0 && (
          <Field label="Listens for">
            <EventChips events={data.events ?? []} />
          </Field>
        )}
      </div>
    );
  }

  // Owner identity: the webhook page mints the token and picks the
  // events; the builder only inspects. Webhook triggers are never
  // placed by hand — a hand-placed one carries no owner token and can
  // never fire. Fallback fields below are the flow's own config.
  if (kind === "webhook") {
    return (
      <div className="space-y-4">
        <div className="rounded-lg border border-primary/30 bg-primary/5 px-3 py-3">
          <p className="flex items-center gap-2 text-sm font-medium">
            <Webhook className="size-4 text-primary" /> Webhook trigger
          </p>
          <p className="mt-1 text-xs text-muted-foreground">
            This flow belongs to a webhook and fires on its inbound HTTP
            call. The trigger itself is owned data — manage the webhook on
            the Webhooks page.
          </p>
        </div>
        <Field label="Inbound URL">
          <div className="flex items-center gap-1.5">
            <Input
              readOnly
              value={webhookUrl}
              aria-label="Inbound webhook URL"
              className="h-9 font-mono text-xs"
            />
            <Button
              variant="outline"
              size="icon-sm"
              aria-label="Copy webhook URL"
              className="shrink-0"
              onClick={() => void copyUrl(webhookUrl)}
              disabled={!webhookUrl}
            >
              {copied ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
            </Button>
          </div>
          {!webhookUrl && (
            <p className="text-xs text-warning">
              No token — this trigger was placed by hand and can never
              fire. Delete it and start the flow from the Webhooks page.
            </p>
          )}
        </Field>
        {(data.events?.length ?? 0) > 0 && (
          <Field label="Listens for">
            <EventChips events={data.events ?? []} />
            <p className="text-xs text-muted-foreground">
              Edited on the Webhooks page — the trigger follows along.
            </p>
          </Field>
        )}
        <TriggerFallbackFields data={data} onChange={onChange} />
      </div>
    );
  }

  const scopedGroup = data.preset
    ? WEBHOOK_EVENTS.find((g) => g.label === data.preset)
    : undefined;
  // Scoped parts list their own group only (deduplicated). Legacy nodes
  // keep the full catalog. notification.created always rides with the
  // Notifications part — it is a notification event.
  const scopedEvents = scopedGroup
    ? [
        ...(scopedGroup.label === "Notifications" ? ["notification.created"] : []),
        ...scopedGroup.events.map((e) => e.key),
      ].filter((e, i, arr) => arr.indexOf(e) === i)
    : null;

  const eventGroups = scopedGroup
    ? [{ label: scopedGroup.label, events: scopedEvents! }]
    : [
        { label: "Flow triggers", events: [...TRIGGER_EVENTS] },
        ...WEBHOOK_EVENTS.map((g) => ({
          label: g.label,
          events: g.events.map((e) => e.key),
        })),
      ];

  return (
    <div className="space-y-4">
      <Field label="Event">
        <Select value={data.event} onValueChange={(v) => v && onChange({ ...data, event: v })}>
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {eventGroups.map((group) => (
              <SelectGroup key={group.label}>
                <SelectLabel>{group.label}</SelectLabel>
                {group.events.map((e) => (
                  <SelectItem key={e} value={e}>
                    {e}
                  </SelectItem>
                ))}
              </SelectGroup>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          Only notification.created fires ad-hoc sends; other events arm
          future listeners.
        </p>
      </Field>
    </div>
  );
}

function TriggerFallbackFields({
  data,
  onChange,
}: {
  data: TriggerData;
  onChange: (d: TriggerData) => void;
}) {
  const { templates, loading } = useTemplates();
  const { registry } = useProviderRegistry();
  const channels = registry?.channels ?? [];
  const recipientsText = (data.recipients ?? []).join("\n");
  return (
    <>
      <Field label="Fallback template (optional)">
        <Select
          value={data.templateId || undefined}
          onValueChange={(v) => onChange({ ...data, templateId: v || undefined })}
        >
          <SelectTrigger>
            <SelectValue placeholder={loading ? "Loading templates…" : "Select a template"} />
          </SelectTrigger>
          <SelectContent>
            {templates.map((t) => (
              <SelectItem key={t.id} value={t.id}>
                {t.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          Renders the message when the call omits title and body.
        </p>
      </Field>
      <Field label="Fallback channel (optional)">
        <Select
          value={data.channel || undefined}
          onValueChange={(v) => onChange({ ...data, channel: v || undefined })}
        >
          <SelectTrigger>
            <SelectValue placeholder="Select channel" />
          </SelectTrigger>
          <SelectContent>
            {channels.map((c) => (
              <SelectItem key={c.channel_id} value={c.channel_id}>
                {c.channel_name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      <Field label="Fallback recipients (optional)">
        <Textarea
          value={recipientsText}
          onChange={(e) =>
            onChange({
              ...data,
              recipients: e.target.value
                .split("\n")
                .map((r) => r.trim())
                .filter(Boolean),
            })
          }
          placeholder={"user-1\nuser-2"}
          className="min-h-[64px] font-mono text-xs"
        />
        <p className="text-xs text-muted-foreground">
          One per line. Used when the call omits recipients.
        </p>
      </Field>
    </>
  );
}

function ProviderForm({ data, onChange }: { data: ProviderData; onChange: (d: ProviderData) => void }) {
  const { registry, loading } = useProviderRegistry();
  const channels = registry?.channels ?? [];
  // Palette parts fix the channel; legacy nodes pick freely.
  const scopedChannel = data.preset
    ? channels.find((c) => c.channel_id === data.preset)
    : undefined;
  const channel = scopedChannel ?? channels.find((c) => c.channel_id === data.channelId);
  const providers = channel?.providers ?? [];

  return (
    <>
      <Field label="Channel">
        {scopedChannel ? (
          <div className="flex h-9 items-center rounded-md border border-border bg-muted px-3 text-sm font-medium">
            {scopedChannel.channel_name}
          </div>
        ) : (
          <Select
            value={data.channelId || undefined}
            onValueChange={(v) => {
              if (!v) return;
              const next = channels.find((c) => c.channel_id === v);
              const keep = next?.providers.some((p) => p.provider_id === data.providerId);
              onChange({ ...data, channelId: v, providerId: keep ? data.providerId : "" });
            }}
          >
            <SelectTrigger>
              <SelectValue placeholder={loading ? "Loading channels…" : "Select channel"} />
            </SelectTrigger>
            <SelectContent>
              {channels.map((c) => (
                <SelectItem key={c.channel_id} value={c.channel_id}>
                  {c.channel_name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
      </Field>
      <Field label="Provider">
        {!channel ? (
          <p className="rounded-md border border-dashed border-border px-3 py-4 text-center text-xs text-muted-foreground">
            {loading ? "Loading providers…" : "Pick a channel first"}
          </p>
        ) : providers.length === 0 ? (
          <p className="rounded-md border border-dashed border-border px-3 py-4 text-center text-xs text-muted-foreground">
            No providers for this channel.
          </p>
        ) : (
          <div role="radiogroup" aria-label="Provider" className="space-y-1.5">
            {providers.map((p) => {
              const selected = data.providerId === p.provider_id;
              const meta = [
                p.scope === "global" ? "Global" : "Regional",
                ...(p.platforms ?? []),
                ...(p.primary_regions ?? []),
              ].join(" · ");
              return (
                <div key={p.provider_id} className="flex items-center gap-1">
                  <button
                    type="button"
                    role="radio"
                    aria-checked={selected}
                    onClick={() => onChange({ ...data, providerId: p.provider_id })}
                    className={cn(
                      "flex min-w-0 flex-1 items-center gap-2.5 rounded-lg border px-2.5 py-2 text-left transition-colors",
                      selected
                        ? "border-primary/50 bg-primary/5"
                        : "border-border hover:bg-muted",
                    )}
                  >
                    <ProviderIcon name={p.name} iconUrl={p.icon_url} size="sm" />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-sm font-medium">{p.name}</span>
                      {meta && (
                        <span className="block truncate text-[11px] text-muted-foreground">{meta}</span>
                      )}
                    </span>
                    {selected && <Check className="size-3.5 shrink-0 text-primary" />}
                  </button>
                  {p.docs_url && (
                    <a
                      href={p.docs_url}
                      target="_blank"
                      rel="noopener noreferrer"
                      aria-label={`${p.name} documentation`}
                      title={`${p.name} documentation`}
                      className="shrink-0 rounded-md p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground"
                    >
                      <ExternalLink className="size-3.5" />
                    </a>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </Field>
    </>
  );
}

function HttpRequestForm({ data, onChange }: { data: HttpRequestData; onChange: (d: HttpRequestData) => void }) {
  const setHeader = (index: number, patch: Partial<{ key: string; value: string }>) => {
    onChange({
      ...data,
      headers: data.headers.map((h, i) => (i === index ? { ...h, ...patch } : h)),
    });
  };
  const removeHeader = (index: number) => {
    onChange({ ...data, headers: data.headers.filter((_, i) => i !== index) });
  };
  return (
    <>
      <Field label="URL">
        <Input
          value={data.url || ""}
          onChange={(e) => onChange({ ...data, url: e.target.value })}
          placeholder="https://api.example.com/hook"
          inputMode="url"
          className="h-9 font-mono text-xs"
        />
      </Field>
      <Field label="Method">
        <Select value={data.method} onValueChange={(v) => v && onChange({ ...data, method: v })}>
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {HTTP_METHODS.map((m) => (
              <SelectItem key={m} value={m}>
                {m}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      <Field label="Headers">
        <div className="space-y-1.5">
          {data.headers.map((h, i) => (
            <div key={i} className="flex items-center gap-1.5">
              <Input
                value={h.key}
                onChange={(e) => setHeader(i, { key: e.target.value })}
                placeholder="Header"
                aria-label={`Header ${i + 1} name`}
                className="h-8 font-mono text-xs"
              />
              <Input
                value={h.value}
                onChange={(e) => setHeader(i, { value: e.target.value })}
                placeholder="Value"
                aria-label={`Header ${i + 1} value`}
                className="h-8 font-mono text-xs"
              />
              <Button
                variant="ghost"
                size="icon-xs"
                aria-label={`Remove header ${i + 1}`}
                onClick={() => removeHeader(i)}
              >
                <X className="size-3.5" />
              </Button>
            </div>
          ))}
          <Button
            variant="outline"
            size="sm"
            className="w-full"
            onClick={() => onChange({ ...data, headers: [...data.headers, { key: "", value: "" }] })}
          >
            Add header
          </Button>
        </div>
      </Field>
      <Field label="Body">
        <Textarea
          value={data.body || ""}
          onChange={(e) => onChange({ ...data, body: e.target.value })}
          placeholder='{"event": "user.signup"}'
          className="min-h-[80px] font-mono text-xs"
        />
        <p className="text-xs text-muted-foreground">
          Sent as-is. Failures stay in server logs — the send still completes.
        </p>
      </Field>
    </>
  );
}

function TemplateForm({ data, onChange }: { data: TemplateData; onChange: (d: TemplateData) => void }) {
  const { templates, loading } = useTemplates();
  const picked = templates.find((t) => t.id === data.templateId);
  return (
    <>
      <Field label="Template">
        <Select
          value={data.templateId || undefined}
          onValueChange={(v) => {
            if (!v) return;
            const chosen = templates.find((t) => t.id === v);
            onChange({ ...data, templateId: v, templateName: chosen?.name ?? data.templateName });
          }}
        >
          <SelectTrigger>
            <SelectValue placeholder={loading ? "Loading templates…" : picked?.name ?? "Select a template"} />
          </SelectTrigger>
          <SelectContent>
            {templates.length === 0 && !loading ? (
              <div className="px-2 py-1.5 text-xs text-muted-foreground">
                No templates yet — create one on the Templates page first.
              </div>
            ) : (
              templates.map((t) => (
                <SelectItem key={t.id} value={t.id}>
                  {t.name}
                </SelectItem>
              ))
            )}
          </SelectContent>
        </Select>
      </Field>
      <p className="text-xs text-muted-foreground">
        The template&apos;s content renders fresh at every fire — rename it on
        the Templates page and the next run picks that up. The send chain
        below chooses what delivers it.
      </p>
    </>
  );
}

function ConditionForm({ data, onChange }: { data: ConditionData; onChange: (d: ConditionData) => void }) {
  return (
    <>
      <Field label="Field">
        <Select value={data.field || undefined} onValueChange={(v) => v && onChange({ ...data, field: v })}>
          <SelectTrigger>
            <SelectValue placeholder="Select field" />
          </SelectTrigger>
          <SelectContent>
            {CONDITION_FIELDS.map((f) => (
              <SelectItem key={f} value={f}>
                {f}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      <Field label="Operator">
        <Select value={data.operator} onValueChange={(v) => v && onChange({ ...data, operator: v })}>
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {CONDITION_OPERATORS.map((o) => (
              <SelectItem key={o} value={o}>
                {o}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      <Field label="Value">
        <Input
          value={data.value || ""}
          onChange={(e) => onChange({ ...data, value: e.target.value })}
          placeholder="e.g. female"
          className="h-9 text-sm"
        />
      </Field>
    </>
  );
}

function AiTransformForm({ data, onChange }: { data: AiTransformData; onChange: (d: AiTransformData) => void }) {
  return (
    <>
      <Field label="Model">
        <Select value={data.model} onValueChange={(v) => v && onChange({ ...data, model: v })}>
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {AI_MODELS.map((m) => (
              <SelectItem key={m} value={m}>
                {m}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      <Field label="Instruction">
        <Textarea
          value={data.instruction || ""}
          onChange={(e) => onChange({ ...data, instruction: e.target.value })}
          placeholder="e.g. Rewrite this SMS as a branded email following the wrapper style…"
          className="min-h-[96px] text-sm"
        />
      </Field>
      <div className="flex items-center justify-between gap-3 rounded-lg border px-3 py-2.5">
        <div>
          <p className="text-sm font-medium">Brand wrapper</p>
          <p className="text-xs text-muted-foreground">Apply the project email style</p>
        </div>
        <Switch
          checked={data.useBrandWrapper}
          onCheckedChange={(v) => onChange({ ...data, useBrandWrapper: v === true })}
        />
      </div>
    </>
  );
}

function StopForm({ data, onChange }: { data: StopData; onChange: (d: StopData) => void }) {
  return (
    <Field label="Reason (optional)">
      <Input
        value={data.reason || ""}
        onChange={(e) => onChange({ ...data, reason: e.target.value })}
        placeholder="e.g. channel failed, halt branch"
        className="h-9 text-sm"
      />
    </Field>
  );
}

const TITLES: Record<string, string> = {
  trigger: "Trigger",
  provider: "Send via provider",
  template: "Template",
  "http-request": "HTTP Request",
  condition: "Condition",
  "ai-transform": "AI transform",
  stop: "Stop",
};

export function NodeConfigPanel({ node, onChange, onClose, lockedTrigger }: NodeConfigPanelProps) {
  return (
    <DetailPanel
      onClose={onClose}
      header={
        <div>
          <h3 className="truncate font-semibold text-foreground">
            {TITLES[node.type ?? ""] ?? "Node"}
          </h3>
          <p className="mt-0.5 truncate text-xs text-muted-foreground">
            {node.type === "trigger" && lockedTrigger
              ? "Owned — read-only here"
              : "Configure this step"}
          </p>
        </div>
      }
    >
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="space-y-4 px-5 py-4">
          {node.type === "trigger" && (
            <TriggerForm
              data={node.data as TriggerData}
              onChange={onChange}
            />
          )}
          {node.type === "provider" && (
            <ProviderForm data={node.data as ProviderData} onChange={onChange} />
          )}
          {node.type === "template" && (
            <TemplateForm data={node.data as TemplateData} onChange={onChange} />
          )}
          {node.type === "http-request" && (
            <HttpRequestForm data={node.data as HttpRequestData} onChange={onChange} />
          )}
          {node.type === "condition" && (
            <ConditionForm data={node.data as ConditionData} onChange={onChange} />
          )}
          {node.type === "ai-transform" && (
            <AiTransformForm data={node.data as AiTransformData} onChange={onChange} />
          )}
          {node.type === "stop" && (
            <StopForm data={node.data as StopData} onChange={onChange} />
          )}
        </div>
      </div>
    </DetailPanel>
  );
}

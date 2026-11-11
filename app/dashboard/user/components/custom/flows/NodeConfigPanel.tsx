"use client";

import { useState } from "react";
import { ArrowRightLeft, Check, CalendarClock, Copy, ExternalLink, Inbox, Plus, Webhook, X } from "lucide-react";
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
import { useInboundProbe } from "@/hooks/use-inbound-probe";
import {
  incomingShapeOf,
  outgoingShapeOf,
  probeShapeOf,
  type DataShape,
} from "./flow-data";
import { getVariables } from "@/lib/template-content";
import {
  AI_MODELS,
  CONDITION_FIELDS,
  CONDITION_OPERATORS,
  HTTP_METHODS,
  TRIGGER_EVENTS,
  mintWebhookSecret,
  triggerKindOf,
  validIpRule,
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
  /** The node feeding this one (first incoming edge) — its output is
   * this node's incoming data. */
  upstream?: FlowNode | null;
  /** Flow identity for the webhook door's last-ping probe. */
  projectId?: string;
  flowId?: string;
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

  // Webhook triggers are builder-born: the palette mints the token at
  // drop time (full-entropy, the server's spec) and the definition is
  // the source of truth — the token index syncs on every save and the
  // same /v1/hooks door fires it. The fallback fields are gone: the
  // POST body drives the message. Legacy tokens stay deletable.
  if (kind === "webhook") {
    return (
      <div className="space-y-4">
        <div className="rounded-lg border border-primary/30 bg-primary/5 px-3 py-3">
          <p className="flex items-center gap-2 text-sm font-medium">
            <Webhook className="size-4 text-primary" /> Webhook trigger
          </p>
          <p className="mt-1 text-xs text-muted-foreground">
            Fires whenever something POSTs to its inbound URL. The POST body
            drives the message: channel + recipients, then title/body or a
            caller templateId. Secrets and IP rules keep the door picky.
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
              No token on this trigger — delete it and place a fresh
              webhook trigger (or start the flow from the Webhooks page).
            </p>
          )}
        </Field>
        {(data.events?.length ?? 0) > 0 && (
          <Field label="Listens for">
            <EventChips events={data.events ?? []} />
          </Field>
        )}
        <WebhookDoorFields data={data} onChange={onChange} />
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

/** Webhook door auth: inbound secret keys (auto-generated `whsk_` keys,
 * any listed key passes the `x-notifi-secret` header) + an IP
 * allow-list (plain IP or CIDR entries). Both empty keeps the door
 * open — the server's pre-auth default. Owner-born triggers render
 * read-only. */
function WebhookDoorFields({
  data,
  onChange,
  locked,
}: {
  data: TriggerData;
  onChange: (d: TriggerData) => void;
  locked?: boolean;
}) {
  const secrets = data.secrets ?? [];
  const rules = data.allowedIps ?? [];
  const [ruleDraft, setRuleDraft] = useState("");
  const [ruleError, setRuleError] = useState<string | null>(null);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  const copySecret = async (key: string) => {
    try {
      await navigator.clipboard.writeText(key);
      setCopiedKey(key);
      setTimeout(() => setCopiedKey(null), 1500);
    } catch {
      // Clipboard unavailable — the key is visible for manual copy.
    }
  };

  const createSecret = () => {
    // Full-entropy keys only — typed secrets can be weak.
    onChange({ ...data, secrets: [...secrets, mintWebhookSecret()] });
  };

  const addRule = () => {
    const rule = ruleDraft.trim();
    if (!rule) return;
    if (!validIpRule(rule)) {
      setRuleError("Use an IP like 10.0.0.1 or a block like 10.0.0.0/8.");
      return;
    }
    if (rules.includes(rule)) {
      setRuleDraft("");
      setRuleError(null);
      return;
    }
    setRuleError(null);
    setRuleDraft("");
    onChange({ ...data, allowedIps: [...rules, rule] });
  };

  return (
    <>
      <Field label="Secret keys">
        <div className="space-y-1.5">
          {secrets.map((key) => (
            <div key={key} className="flex items-center gap-1.5">
              <Input readOnly value={key} aria-label="Inbound secret key" className="h-8 font-mono text-xs" />
              <Button
                variant="outline"
                size="icon-sm"
                aria-label={`Copy secret ${key}`}
                className="shrink-0"
                onClick={() => void copySecret(key)}
              >
                {copiedKey === key ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
              </Button>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={`Delete secret ${key}`}
                className="shrink-0 text-muted-foreground hover:text-destructive"
                disabled={locked}
                onClick={() => onChange({ ...data, secrets: secrets.filter((s) => s !== key) })}
              >
                <X className="size-3.5" />
              </Button>
            </div>
          ))}
          {secrets.length === 0 && (
            <p className="text-xs text-muted-foreground">
              No keys — the door is open. Create one to reject callers without a valid key.
            </p>
          )}
          <Button variant="outline" size="sm" className="mt-0.5" disabled={locked} onClick={createSecret}>
            <Plus className="size-3.5" /> Create key
          </Button>
          <p className="text-xs text-muted-foreground">
            {locked
              ? "This trigger is owned data — keys read-only here."
              : "Pass a key in the x-notifi-secret header; a wrong one is rejected (rotation: any listed key works)."}
          </p>
        </div>
      </Field>
      <Field label="IP allow-list">
        <div className="space-y-1.5">
          {rules.map((rule) => (
            <div key={rule} className="flex items-center gap-1.5">
              <Input readOnly value={rule} aria-label="Allowed address" className="h-8 font-mono text-xs" />
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={`Remove ${rule}`}
                className="shrink-0 text-muted-foreground hover:text-destructive"
                disabled={locked}
                onClick={() => onChange({ ...data, allowedIps: rules.filter((r) => r !== rule) })}
              >
                <X className="size-3.5" />
              </Button>
            </div>
          ))}
          {rules.length === 0 && (
            <p className="text-xs text-muted-foreground">Empty — any address may call.</p>
          )}
          {!locked && (
            <>
              <div className="flex items-center gap-1.5">
                <Input
                  value={ruleDraft}
                  aria-label="New allowed address"
                  placeholder="10.0.0.1 or 10.0.0.0/8"
                  className="h-8 font-mono text-xs"
                  onChange={(e) => setRuleDraft(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      addRule();
                    }
                  }}
                />
                <Button variant="outline" size="icon-sm" aria-label="Add address" className="shrink-0" onClick={addRule}>
                  <Plus className="size-3.5" />
                </Button>
              </div>
              {ruleError && <p className="text-xs text-destructive">{ruleError}</p>}
            </>
          )}
          {locked && rules.length === 0 && <p className="text-xs text-muted-foreground">No rules.</p>}
        </div>
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
  const declared = picked ? getVariables(picked.content) : [];

  // Switching templates prunes presets that no longer map to a declared
  // ref; same-named refs keep their value.
  const pickTemplate = (v: string) => {
    const chosen = templates.find((t) => t.id === v);
    const keep = new Set(chosen ? getVariables(chosen.content).map((x) => x.key) : []);
    const values = Object.fromEntries(
      Object.entries(data.values ?? {}).filter(([key]) => keep.has(key)),
    );
    onChange({ ...data, templateId: v, templateName: chosen?.name ?? data.templateName, values });
  };

  const setVar = (key: string, value: string) => {
    const next = { ...data.values };
    if (value.trim() === "") {
      delete next[key];
    } else {
      next[key] = value.trim();
    }
    onChange({ ...data, values: next });
  };

  return (
    <>
      <Field label="Template">
        <Select
          value={data.templateId || undefined}
          onValueChange={(v) => {
            if (!v) return;
            pickTemplate(v);
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
      {/* Declared {{refs}} need a preset here — the declared-to-use rule
          makes the editor refuse undeclared ones, and {{recipient.*}}
          resolves from the recipient on its own. */}
      {declared.length > 0 && picked && (
        <Field label="Dynamic data">
          <DeclaredValues
            declared={declared}
            values={data.values}
            onSet={setVar}
          />
        </Field>
      )}
      <p className="text-xs text-muted-foreground">
        The template&apos;s content renders fresh at every fire — rename it on
        the Templates page and the next run picks that up. The send chain
        below chooses what delivers it.
      </p>
    </>
  );
}

/** One input per declared ref — shared by the template node and the
 * webhook trigger's fallback panel. Blank input deletes the preset;
 * the description renders as the helper text. */
function DeclaredValues({
  declared,
  values,
  onSet,
}: {
  declared: ReturnType<typeof getVariables>;
  values: Record<string, string> | undefined;
  onSet: (key: string, value: string) => void;
}) {
  return (
    <>
      {declared.map((v) => (
        <div key={v.key} className="space-y-1">
          <Input
            value={values?.[v.key] ?? ""}
            onChange={(e) => onSet(v.key, e.target.value)}
            placeholder={`Value for {{${v.key}}}`}
            aria-label={`Value for ${v.key}`}
          />
          {v.description && (
            <p className="text-xs text-muted-foreground">
              {`{{${v.key}}}`} — {v.description}
            </p>
          )}
        </div>
      ))}
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

type PanelTab = "config" | "in" | "out";

const TABS: { id: PanelTab; label: string }[] = [
  { id: "config", label: "Config" },
  { id: "in", label: "Incoming" },
  { id: "out", label: "Outgoing" },
];

/** One key/value row of a data shape. */
function ShapeRow({ row }: { row: DataShape["rows"][number] }) {
  return (
    <div className="flex items-baseline justify-between gap-3 py-1.5">
      <span className="shrink-0 font-mono text-xs text-muted-foreground dark:text-zinc-400">
        {row.key}
      </span>
      <span className="min-w-0 flex-1 truncate text-right font-mono text-xs text-foreground dark:text-zinc-200">
        {row.value}
      </span>
    </div>
  );
}

function DataTabBody({ shape }: { shape: DataShape }) {
  return (
    <div className="space-y-3">
      {!shape.real && (
        <p className="rounded-lg bg-muted px-3 py-2 text-[11px] text-muted-foreground dark:bg-white/5 dark:text-zinc-400">
          Sample — derived from the graph, not a real run.
        </p>
      )}
      <div className="divide-y rounded-lg border border-border dark:border-white/10">
        {shape.rows.map((row) => (
          <div key={row.key} className="px-3">
            <ShapeRow row={row} />
            {row.note && (
              <p className="pb-1.5 text-[11px] leading-snug text-muted-foreground dark:text-zinc-500">
                {row.note}
              </p>
            )}
          </div>
        ))}
        {shape.rows.length === 0 && (
          <p className="px-3 py-3 text-xs text-muted-foreground">No data.</p>
        )}
      </div>
      <pre className="max-h-56 overflow-auto rounded-lg bg-muted p-3 font-mono text-[11px] leading-relaxed text-foreground dark:bg-white/5 dark:text-zinc-200">
        {shape.json}
      </pre>
    </div>
  );
}

function IncomingTab({
  node,
  upstream,
  projectId,
  flowId,
  active,
}: {
  node: FlowNode;
  upstream: FlowNode | null;
  projectId?: string;
  flowId?: string;
  active: boolean;
}) {
  const trigger = node.type === "trigger" ? (node.data as TriggerData) : null;
  const isWebhookDoor = trigger !== null && triggerKindOf(trigger) === "webhook";
  const probe = useInboundProbe(
    active && isWebhookDoor && Boolean(projectId && flowId),
    projectId,
    flowId,
    trigger?.token,
  );

  if (isWebhookDoor) {
    const heading = (
      <p className="mb-2 flex items-center gap-1.5 text-xs font-medium text-muted-foreground dark:text-zinc-400">
        <Webhook className="size-3.5" /> Last ping at the door
      </p>
    );
    if (probe.loading) {
      return (
        <div>
          {heading}
          <div className="h-32 animate-pulse rounded-lg bg-muted dark:bg-white/5" />
        </div>
      );
    }
    if (probe.probe) {
      return (
        <div>
          {heading}
          <DataTabBody shape={probeShapeOf(probe.probe)} />
        </div>
      );
    }
    return (
      <div>
        {heading}
        <p className="mb-3 rounded-lg bg-muted px-3 py-2 text-[11px] text-muted-foreground dark:bg-white/5 dark:text-zinc-400">
          No pings yet — POST to this flow&apos;s URL (Config tab). Below is the
          shape of what arrives when one lands.
        </p>
        <DataTabBody shape={incomingShapeOf(node, null)!} />
      </div>
    );
  }

  if (!upstream) {
    return (
      <div className="py-10 text-center">
        <Inbox className="mx-auto size-6 text-muted-foreground dark:text-zinc-500" />
        <p className="mt-2 text-sm text-muted-foreground dark:text-zinc-400">
          Connect an upstream node to see what arrives here.
        </p>
      </div>
    );
  }

  const shape = incomingShapeOf(node, upstream);
  return (
    <div>
      <p className="mb-2 flex items-center gap-1.5 text-xs font-medium text-muted-foreground dark:text-zinc-400">
        <ArrowRightLeft className="size-3.5" /> From {upstream.data && "templateName" in upstream.data && upstream.data.templateName ? upstream.data.templateName : upstream.type}
      </p>
      {shape && <DataTabBody shape={shape} />}
    </div>
  );
}

export function NodeConfigPanel({
  node,
  onChange,
  onClose,
  lockedTrigger,
  upstream,
  projectId,
  flowId,
}: NodeConfigPanelProps) {
  const [tab, setTab] = useState<PanelTab>("config");
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
      {/* Tab bar — Config, Incoming (what arrives), Outgoing (what this
          node passes downstream). */}
      <div className="border-b px-5 py-2.5 dark:border-white/10">
        <div className="flex gap-1 rounded-lg bg-muted p-1 dark:bg-white/5">
          {TABS.map((t) => (
            <button
              key={t.id}
              type="button"
              onClick={() => setTab(t.id)}
              aria-pressed={tab === t.id}
              className={cn(
                "flex-1 cursor-pointer rounded-md px-2 py-1 text-xs font-medium transition-colors",
                tab === t.id
                  ? "bg-popover text-foreground shadow-sm dark:bg-white/10 dark:text-zinc-100"
                  : "text-muted-foreground hover:text-foreground dark:text-zinc-500 dark:hover:text-zinc-300",
              )}
            >
              {t.label}
            </button>
          ))}
        </div>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto">
        {tab === "config" && (
          <div className="space-y-4 px-5 py-4">
            {node.type === "trigger" && (
              <TriggerForm data={node.data as TriggerData} onChange={onChange} />
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
        )}
        {tab === "in" && (
          <div className="px-5 py-4">
            <IncomingTab
              node={node}
              upstream={upstream ?? null}
              projectId={projectId}
              flowId={flowId}
              active={tab === "in"}
            />
          </div>
        )}
        {tab === "out" && (
          <div className="px-5 py-4">
            <p className="mb-2 flex items-center gap-1.5 text-xs font-medium text-muted-foreground dark:text-zinc-400">
              <ArrowRightLeft className="size-3.5" /> Passed downstream
            </p>
            <DataTabBody shape={outgoingShapeOf(node)} />
          </div>
        )}
      </div>
    </DetailPanel>
  );
}

"use client";

import { useMemo, useState } from "react";
import Link from "next/link";
import { AlertTriangle, Check, Plus, Search, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import { useTemplates } from "@/hooks/use-templates";
import { useRecipients } from "@/hooks/use-recipients";
import {
  useProviderConfigs,
  type ProviderConfig,
} from "@/hooks/use-provider-configs";
import { STRUCTURED_KEYS } from "@/components/custom/recipient-contacts-editor";
import type { Recipient } from "@/lib/types";
import type { AudienceMode, ServerCampaign } from "@/hooks/use-campaigns";

/**
 * Channels a campaign can target — template render keys that can ever
 * match a connectable channel in the Providers registry (email, sms,
 * push). Campaigns deliver directly through a connected provider for
 * their channel, so compose only offers versions where one *could* be
 * connected; labels fall back to the raw key for older/legacy rows.
 */
export const CAMPAIGN_CHANNELS: Record<
  string,
  { label: string; contact: string }
> = {
  email: { label: "Email", contact: "email address" },
  sms: { label: "SMS", contact: "phone number" },
  push: { label: "Push", contact: "push token" },
};

export function labelForChannel(channel: string): string {
  return CAMPAIGN_CHANNELS[channel]?.label ?? channel;
}

/** The contacts key a campaign channel resolves to (mirrors the server). */
export function contactKeyFor(channel: string): string {
  return channel === "sms" || channel === "whatsapp" ? "phone" : channel;
}

/** True when some enabled provider config covers the channel. */
export function channelConfigured(
  configs: ProviderConfig[],
  channel: string,
): boolean {
  if (!channel) return false;
  return configs.some((c) => c.channel_id === channel && c.enabled);
}

/**
 * Connected provider ids for a channel, in the store's order (the same
 * order the server's delivery chain uses as fallback sequence).
 */
export function connectedProvidersFor(
  configs: ProviderConfig[],
  channel: string,
): string[] {
  if (!channel) return [];
  return configs
    .filter((c) => c.channel_id === channel && c.enabled)
    .map((c) => c.provider_id);
}

/**
 * The provider the picker effectively points at: the explicit pick or,
 * when untouched, the first connected provider for the channel —
 * whatever the picker shows is what gets sent.
 */
export function resolveCampaignProvider(
  form: Pick<CampaignFormValue, "provider" | "channel">,
  configs: ProviderConfig[],
): string {
  if (form.provider.trim()) return form.provider.trim();
  return connectedProvidersFor(configs, form.channel)[0] ?? "";
}

/**
 * Audience filter over loaded recipients — ANDed with the search box,
 * purely client-side (selection stays an explicit snapshot, so the
 * server is untouched). Reads only what the recipients list returns.
 */
export type RecipientFilter =
  /** Has a (non-empty) contact for the campaign's channel. */
  | { kind: "contact" }
  /** Created within the yyyy-mm-dd range (either bound optional). */
  | { kind: "created"; from: string; to: string }
  /** Custom contacts field equals a given value. */
  | { kind: "field"; field: string; value: string };

function matchesFilter(
  recipient: Recipient,
  filter: RecipientFilter,
  channelContactKey: string | null,
): boolean {
  switch (filter.kind) {
    case "contact":
      return (
        channelContactKey !== null &&
        String(recipient.contacts?.[channelContactKey] ?? "").trim() !== ""
      );
    case "created": {
      const day = recipient.createdAt.slice(0, 10);
      // String compare works: both sides are yyyy-mm-dd.
      return (
        (!filter.from || day >= filter.from) &&
        (!filter.to || day <= filter.to)
      );
    }
    case "field":
      return (
        filter.field !== "" &&
        String(recipient.contacts?.[filter.field] ?? "")
          .trim()
          .toLowerCase() === filter.value.trim().toLowerCase()
      );
  }
}

/**
 * Channels a template has usable render content for (client-side
 * approximation of the server's rule — the server still validates the
 * exact render). Drives the compose channel picker: only what the
 * template covers is offered.
 */
export function coveredChannelsOf(content: unknown): string[] {
  if (!content || typeof content !== "object") return [];
  return Object.keys(content).filter((key) => key in CAMPAIGN_CHANNELS);
}

/** Editable form state shared by the create panel and the detail editor. */
export type CampaignFormValue = {
  name: string;
  description: string;
  /** Campaigns compose from templates — required. */
  templateId: string;
  /** The template's channel version — required (no default render). */
  channel: string;
  /**
   * Explicit primary-provider pick ("" = untouched). Resolving it
   * through `resolveCampaignProvider` fills "" with the first connected
   * provider for the channel — whatever the picker shows is what gets
   * sent. Reset to "" whenever the channel or template changes.
   */
  provider: string;
  audienceMode: AudienceMode;
  /** Custom-audience selection (recipient `userId`s). */
  selected: string[];
  /** Save as draft (run manually) or schedule the fire. */
  timing: "draft" | "scheduled";
  /** `datetime-local` string (browser-local) when scheduled. */
  sendAt: string;
};

export function blankCampaignForm(): CampaignFormValue {
  return {
    name: "",
    description: "",
    templateId: "",
    channel: "",
    provider: "",
    audienceMode: "all",
    selected: [],
    timing: "draft",
    sendAt: "",
  };
}

/** Rebuilds form state from a stored campaign (template stays attached). */
export function campaignToForm(c: ServerCampaign): CampaignFormValue {
  return {
    name: c.name,
    description: c.description,
    templateId: c.templateId ?? "",
    channel: c.channel,
    provider: c.providerId ?? "",
    audienceMode: c.audienceMode,
    selected: c.selectedUserIds,
    timing: c.status === "scheduled" ? "scheduled" : "draft",
    sendAt: c.sendAt ? toLocalInput(c.sendAt) : "",
  };
}

/** ISO string → `datetime-local` input value (browser-local). */
export function toLocalInput(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(
    d.getHours(),
  )}:${pad(d.getMinutes())}`;
}

/** `datetime-local` input value → ISO string (null when blank/invalid). */
export function fromLocalISO(local: string): string | null {
  if (!local.trim()) return null;
  const d = new Date(local);
  return Number.isNaN(d.getTime()) ? null : d.toISOString();
}

/** Form state → create input (sendAt only when scheduling). Resolves
    the untouched provider pick to the first connected provider for the
    channel so what the picker shows is what gets sent. */
export function formToCreateInput(
  form: CampaignFormValue,
  configs: ProviderConfig[] = [],
) {
  return {
    name: form.name.trim(),
    description: form.description.trim() || undefined,
    templateId: form.templateId,
    channel: form.channel || undefined,
    providerId: resolveCampaignProvider(form, configs) || undefined,
    audienceMode: form.audienceMode,
    selectedUserIds: form.audienceMode === "custom" ? form.selected : undefined,
    sendAt:
      form.timing === "scheduled" ? fromLocalISO(form.sendAt) : undefined,
  };
}

/**
 * Form state → patch input, diffed against the stored campaign so
 * unchanged fields stay absent (no accidental re-renders/unschedules).
 */
export function formToPatchInput(
  form: CampaignFormValue,
  orig: ServerCampaign,
  configs: ProviderConfig[] = [],
) {
  const patch: {
    name?: string;
    description?: string;
    templateId?: string;
    channel?: string;
    providerId?: string;
    audienceMode?: AudienceMode;
    selectedUserIds?: string[];
    sendAt?: string | null;
  } = {};
  const name = form.name.trim();
  if (name !== orig.name) patch.name = name;
  const description = form.description.trim();
  if (description !== orig.description) patch.description = description;
  if (form.templateId && form.templateId !== orig.templateId) {
    patch.templateId = form.templateId;
  }
  // The channel is always explicit; only a re-pick (different from the
  // stored one) is sent. A template patch without a re-pick lets the
  // server fall back to the stored channel — it 400s if the new template
  // does not cover it. The resolved provider pick (explicit, or the
  // first connected one when untouched) follows the same rule.
  if (form.channel && form.channel !== orig.channel) {
    patch.channel = form.channel;
  }
  const providerId = resolveCampaignProvider(form, configs);
  if (providerId !== (orig.providerId ?? "")) {
    patch.providerId = providerId;
  }
  const nextSelected = form.audienceMode === "custom" ? form.selected : [];
  if (
    form.audienceMode !== orig.audienceMode ||
    nextSelected.join("\u0000") !== orig.selectedUserIds.join("\u0000")
  ) {
    patch.audienceMode = form.audienceMode;
    patch.selectedUserIds = nextSelected;
  }
  if (form.timing === "draft" && orig.sendAt) {
    patch.sendAt = null; // unschedule → back to draft
  } else if (form.timing === "scheduled") {
    const next = fromLocalISO(form.sendAt);
    if (next && next !== orig.sendAt) patch.sendAt = next;
  }
  return patch;
}

/**
 * First blocking problem in the form, or null when ready to submit.
 * The template's channel version is required for every campaign (drafts
 * included — delivery resolves per-row against the connected provider
 * for exactly that channel). `scheduledChannelConfigured` only matters
 * when scheduling: a channel that no enabled provider covers blocks the
 * schedule (drafts always save). Not called during render for anything
 * time-based — the schedule-time freshness check runs on submit only.
 */
export function campaignFormError(
  form: CampaignFormValue,
  scheduledChannelConfigured: boolean,
): string | null {
  if (!form.name.trim()) return "Give the campaign a name.";
  if (!form.templateId) return "Pick a template to compose from.";
  if (!form.channel) return "Pick which version of the template to send.";
  if (form.timing === "scheduled") {
    if (!scheduledChannelConfigured) {
      return "The selected channel has no connected provider — connect one on the Providers page, or save as a draft.";
    }
    if (!form.sendAt.trim()) return "Pick the scheduled date and time.";
    if (fromLocalISO(form.sendAt) === null) {
      return "The scheduled time is invalid.";
    }
  }
  if (form.audienceMode === "custom" && form.selected.length === 0) {
    return "Choose at least one recipient for a custom audience.";
  }
  return null;
}

function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="min-w-0 space-y-1.5">
      <label className="text-xs font-medium text-foreground">{label}</label>
      {children}
      {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
    </div>
  );
}

function SegmentButton({
  active,
  title,
  onClick,
  children,
}: {
  active: boolean;
  title: string;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-pressed={active}
      title={title}
      onClick={onClick}
      className={cn(
        "flex-1 rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
        active
          ? "bg-background text-foreground shadow-sm"
          : "text-muted-foreground hover:text-foreground",
      )}
    >
      {children}
    </button>
  );
}

/** One editable audience-filter row (AND-combined with the others). */
function FilterRow({
  filter,
  recipients,
  customFieldKeys,
  contactLabel,
  onChange,
  onRemove,
}: {
  filter: RecipientFilter;
  /** Loaded recipients — for the distinct values of a custom field. */
  recipients: Recipient[];
  customFieldKeys: string[];
  /** Channel contact label ("email", "phone number", ...) or null. */
  contactLabel: string | null;
  onChange: (next: RecipientFilter) => void;
  onRemove: () => void;
}) {
  const fieldValues = useMemo(
    () =>
      filter.kind === "field" && filter.field
        ? [
            ...new Set(
              recipients
                .map((r) => String(r.contacts?.[filter.field] ?? "").trim())
                .filter((v) => v !== ""),
            ),
          ].sort()
        : [],
    [recipients, filter],
  );

  return (
    <div className="flex items-center gap-2 rounded-md bg-muted px-2 py-1.5 text-xs">
      {filter.kind === "contact" && (
        <span className="min-w-0 flex-1 truncate text-foreground">
          Has <span className="font-medium">{contactLabel}</span> contact
        </span>
      )}
      {filter.kind === "created" && (
        <>
          <span className="shrink-0 text-muted-foreground">Created from</span>
          <Input
            type="date"
            value={filter.from}
            onChange={(e) =>
              onChange({ ...filter, from: e.target.value })
            }
            aria-label="Created from date"
            className="h-7 w-32 text-xs"
          />
          <span className="shrink-0 text-muted-foreground">to</span>
          <Input
            type="date"
            value={filter.to}
            onChange={(e) => onChange({ ...filter, to: e.target.value })}
            aria-label="Created to date"
            className="h-7 w-32 text-xs"
          />
        </>
      )}
      {filter.kind === "field" && (
        <>
          <span className="shrink-0 text-muted-foreground">Field is</span>
          <Select
            value={filter.field}
            onValueChange={(v) =>
              v !== null && onChange({ ...filter, field: v, value: "" })
            }
          >
            <SelectTrigger
              aria-label="Filter field"
              className="h-7 w-36 text-xs"
            >
              <SelectValue placeholder="Custom field…" />
            </SelectTrigger>
            <SelectContent>
              {customFieldKeys.map((key) => (
                <SelectItem key={key} value={key}>
                  {key}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </>
      )}
      {filter.kind === "field" && filter.field !== "" && (
        <Select
          value={filter.value}
          onValueChange={(v) => v !== null && onChange({ ...filter, value: v })}
        >
          <SelectTrigger
            aria-label="Filter value"
            className="h-7 min-w-0 flex-1 text-xs"
          >
            <SelectValue placeholder="Value…" />
          </SelectTrigger>
          <SelectContent>
            {fieldValues.map((v) => (
              <SelectItem key={v} value={v}>
                {v}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      )}
      <button
        type="button"
        aria-label="Remove filter"
        onClick={onRemove}
        className="shrink-0 rounded p-1 text-muted-foreground transition-colors hover:bg-background hover:text-foreground"
      >
        <X className="size-3" />
      </button>
    </div>
  );
}

/**
 * Campaign composition fields. Campaigns compose from templates by
 * design: the template snapshot freezes the copy at compose time, the
 * channel picks which of the template's covered renders to send, and
 * timing is draft-now or scheduled-for-later (never auto-send).
 */
export function CampaignFormFields({
  value,
  onChange,
}: {
  value: CampaignFormValue;
  onChange: (next: CampaignFormValue) => void;
}) {
  const { templates, loading: templatesLoading } = useTemplates();
  const { recipients } = useRecipients();
  const { configs } = useProviderConfigs();

  const set = (patch: Partial<CampaignFormValue>) =>
    onChange({ ...value, ...patch });

  // All-mode reach preview: recipients holding the channel's contact key.
  // Also drives the "has contact" audience filter (and the per-recipient
  // contact hints), so it resolves first.
  const contactKey =
    value.channel !== "" ? contactKeyFor(value.channel) : null;
  const reachableCount = contactKey
    ? recipients.filter(
        (r) => String(r.contacts?.[contactKey] ?? "").trim() !== "",
      ).length
    : null;

  const scheduled = value.timing === "scheduled";
  const channelConfiguredNow = channelConfigured(configs, value.channel);

  const [templateQuery, setTemplateQuery] = useState("");
  const [recipientQuery, setRecipientQuery] = useState("");
  const [filters, setFilters] = useState<RecipientFilter[]>([]);

  const selectedTemplate = useMemo(
    () => templates.find((t) => t.id === value.templateId) ?? null,
    [templates, value.templateId],
  );
  const covered = useMemo(
    () => coveredChannelsOf(selectedTemplate?.content),
    [selectedTemplate],
  );

  const visibleTemplates = templates.filter((t) =>
    t.name.toLowerCase().includes(templateQuery.trim().toLowerCase()),
  );

  // Search (name/key), then the active filters (ANDed).
  const q = recipientQuery.trim().toLowerCase();
  const searchedRecipients = recipients.filter(
    (r) => !q || r.name.toLowerCase().includes(q) || r.userId.toLowerCase().includes(q),
  );
  const matchingRecipients = searchedRecipients.filter((r) =>
    filters.every((f) => matchesFilter(r, f, contactKey)),
  );
  const matchingIds = new Set(matchingRecipients.map((r) => r.userId));
  const matchingSelected = matchingRecipients.filter((r) =>
    value.selected.includes(r.userId),
  ).length;

  // Custom contacts field keys across the project (what filters can read).
  const customFieldKeys = useMemo(() => {
    const keys = new Set<string>();
    for (const r of recipients) {
      for (const key of Object.keys(r.contacts ?? {})) {
        if (!STRUCTURED_KEYS.has(key)) keys.add(key);
      }
    }
    return [...keys].sort();
  }, [recipients]);

  return (
    <div className="space-y-4">
      <Field label="Campaign name">
        <Input
          value={value.name}
          onChange={(e) => set({ name: e.target.value })}
          placeholder="e.g. April release notes"
          className="h-9 text-sm"
        />
      </Field>

      <Field
        label="Description"
        hint="Internal note — what is this campaign about?"
      >
        <Textarea
          value={value.description}
          onChange={(e) => set({ description: e.target.value })}
          placeholder="What is this campaign about?"
          rows={2}
          className="text-sm"
        />
      </Field>

      <Field
        label="Template"
        hint="The chosen render is copied into the campaign — later template edits never change it."
      >
        <div className="space-y-2">
          <div className="relative">
            <Search className="absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={templateQuery}
              onChange={(e) => setTemplateQuery(e.target.value)}
              placeholder="Search templates..."
              aria-label="Search templates"
              className="h-8 pl-8 text-sm"
            />
          </div>
          <div
            role="radiogroup"
            aria-label="Choose a template"
            className="max-h-44 space-y-1 overflow-y-auto rounded-lg border border-border p-1"
          >
            {templatesLoading ? (
              <p className="px-2 py-4 text-center text-xs text-muted-foreground">
                Loading templates…
              </p>
            ) : visibleTemplates.length === 0 ? (
              <p className="px-2 py-4 text-center text-xs text-muted-foreground">
                {templates.length === 0
                  ? "No templates yet — create one to get started."
                  : "No templates match."}
              </p>
            ) : (
              visibleTemplates.map((t) => {
                const selected = value.templateId === t.id;
                const coverage = coveredChannelsOf(t.content);
                return (
                  <button
                    key={t.id}
                    type="button"
                    role="radio"
                    aria-checked={selected}
                    onClick={() =>
                      set({
                        templateId: t.id,
                        // Reset the channel pick — the template decides
                        // which channels exist (the provider pick rides
                        // the channel, so it resets too).
                        channel: "",
                        provider: "",
                      })
                    }
                    className={cn(
                      "flex w-full items-center gap-2 rounded-md px-2.5 py-2 text-left text-sm transition-colors hover:bg-muted",
                      selected && "bg-muted",
                    )}
                  >
                    <span
                      className={cn(
                        "flex size-4 shrink-0 items-center justify-center rounded-full border",
                        selected
                          ? "border-primary bg-primary text-primary-foreground"
                          : "border-border text-transparent",
                      )}
                    >
                      <Check className="size-3" />
                    </span>
                    <span className="min-w-0 flex-1 truncate font-medium">
                      {t.name}
                    </span>
                    {coverage.length > 0 && (
                      <span className="flex shrink-0 items-center gap-1">
                        {coverage.slice(0, 2).map((key) => (
                          <span
                            key={key}
                            className="rounded border border-border bg-background px-1.5 py-0.5 text-[10px] text-muted-foreground"
                          >
                            {labelForChannel(key)}
                          </span>
                        ))}
                        {coverage.length > 2 && (
                          <span className="text-[10px] text-muted-foreground">
                            +{coverage.length - 2}
                          </span>
                        )}
                      </span>
                    )}
                  </button>
                );
              })
            )}
          </div>
        </div>
      </Field>

      <div className="grid min-w-0 gap-3 sm:grid-cols-2">
        <Field
          label="Channel"
          hint={`Sends as ${
            value.channel ? labelForChannel(value.channel) : "the version you pick"
          } — the chosen render is snapshotted at create. One channel per campaign.`}
        >
          <Select
            value={value.channel}
            onValueChange={(v) => v !== null && set({ channel: v, provider: "" })}
            disabled={covered.length === 0}
          >
            <SelectTrigger className="h-9 w-full min-w-0 text-sm">
              <SelectValue
                placeholder={
                  selectedTemplate
                    ? covered.length === 0
                      ? "No deliverable channels"
                      : "Pick the template's version"
                    : "Pick a template first"
                }
              />
            </SelectTrigger>
            <SelectContent>
              {covered.map((key) => (
                <SelectItem key={key} value={key}>
                  {labelForChannel(key)}
                  {channelConfigured(configs, key) ? "" : " · not connected"}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>

        <Field
          label="Provider"
          hint="Tried first — the channel's other connected providers fall back in order."
        >
          <Select
            value={resolveCampaignProvider(value, configs)}
            onValueChange={(v) => v !== null && set({ provider: v })}
            disabled={
              !value.channel ||
              connectedProvidersFor(configs, value.channel).length === 0
            }
          >
            <SelectTrigger className="h-9 w-full min-w-0 text-sm">
              <SelectValue
                placeholder={
                  !value.channel
                    ? "Pick a channel first"
                    : "No connected provider"
                }
              />
            </SelectTrigger>
            <SelectContent>
              {connectedProvidersFor(configs, value.channel).map((pid) => (
                <SelectItem key={pid} value={pid}>
                  {pid}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>
      </div>

      <Field
        label="Audience"
        hint={
          value.audienceMode === "all"
            ? "Recipients resolve live at send time — new signups are picked up."
            : "The selection is frozen at compose time; deleted recipients drop out."
        }
      >
        <div className="flex items-center gap-2 rounded-lg bg-muted p-1">
          <SegmentButton
            active={value.audienceMode === "all"}
            title="Send to everyone with a contact for the channel"
            onClick={() => set({ audienceMode: "all" })}
          >
            All recipients
          </SegmentButton>
          <SegmentButton
            active={value.audienceMode === "custom"}
            title="Send only to the recipients selected here"
            onClick={() => set({ audienceMode: "custom" })}
          >
            Choose recipients
          </SegmentButton>
        </div>

        {value.audienceMode === "all" ? (
          <p className="text-xs text-muted-foreground">
            {contactKey
              ? `Will send to everyone with a ${CAMPAIGN_CHANNELS[value.channel]?.contact ?? `${contactKey} contact`} on file`
              : "Everyone reachable on the channel you pick."}
            {reachableCount !== null
              ? ` — ${reachableCount} recipient${reachableCount === 1 ? "" : "s"} right now.`
              : ""}
          </p>
        ) : (
          <div className="space-y-2">
            <div className="relative">
              <Search className="absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={recipientQuery}
                onChange={(e) => setRecipientQuery(e.target.value)}
                placeholder="Search recipients by name or key..."
                aria-label="Search recipients"
                className="h-8 pl-8 text-sm"
              />
            </div>

            {filters.length > 0 && (
              <div className="space-y-1.5">
                {filters.map((f, idx) => (
                  <FilterRow
                    key={idx}
                    filter={f}
                    recipients={recipients}
                    customFieldKeys={customFieldKeys}
                    contactLabel={
                      contactKey
                        ? (CAMPAIGN_CHANNELS[value.channel]?.contact ??
                          `${contactKey} contact`)
                        : null
                    }
                    onChange={(next) =>
                      setFilters(
                        filters.map((old, i) => (i === idx ? next : old)),
                      )
                    }
                    onRemove={() =>
                      setFilters(filters.filter((_, i) => i !== idx))
                    }
                  />
                ))}
              </div>
            )}

            <div className="flex flex-wrap items-center gap-1.5">
              <DropdownMenu>
                <DropdownMenuTrigger
                  render={
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      className="h-7 gap-1 px-2 text-xs"
                    />
                  }
                >
                  <Plus className="size-3" /> Add filter
                </DropdownMenuTrigger>
                <DropdownMenuContent
                  align="start"
                  className="w-64"
                >
                  <DropdownMenuItem
                    disabled={contactKey === null}
                    onClick={() => setFilters([...filters, { kind: "contact" }])}
                  >
                    Has a{" "}
                    {contactKey
                      ? (CAMPAIGN_CHANNELS[value.channel]?.contact ??
                        `${contactKey} contact`)
                      : "contact (pick a channel first)"}
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    onClick={() =>
                      setFilters([
                        ...filters,
                        { kind: "created", from: "", to: "" },
                      ])
                    }
                  >
                    Created within a date range
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    disabled={customFieldKeys.length === 0}
                    onClick={() =>
                      setFilters([
                        ...filters,
                        { kind: "field", field: "", value: "" },
                      ])
                    }
                  >
                    Custom field is equal to…
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
              {filters.length > 0 && (
                <>
                  <span className="ml-auto text-xs tabular-nums text-muted-foreground">
                    {matchingRecipients.length} match
                    {matchingRecipients.length === 1 ? "" : "es"}
                  </span>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="h-7 px-2 text-xs"
                    disabled={matchingRecipients.length === 0}
                    onClick={() =>
                      set({
                        selected: [
                          ...new Set([
                            ...value.selected,
                            ...matchingRecipients.map((r) => r.userId),
                          ]),
                        ],
                      })
                    }
                  >
                    Select all matches
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="h-7 px-2 text-xs"
                    disabled={matchingSelected === 0}
                    onClick={() =>
                      set({
                        selected: value.selected.filter(
                          (id) => !matchingIds.has(id),
                        ),
                      })
                    }
                  >
                    Deselect matches
                  </Button>
                </>
              )}
            </div>

            <div
              role="group"
              aria-label="Choose recipients"
              className="max-h-44 space-y-1 overflow-y-auto rounded-lg border border-border p-1"
            >
              {recipients.length === 0 ? (
                <p className="px-2 py-4 text-center text-xs text-muted-foreground">
                  No recipients yet — add some on the Recipients page.
                </p>
              ) : matchingRecipients.length === 0 ? (
                <p className="px-2 py-4 text-center text-xs text-muted-foreground">
                  {filters.length > 0
                    ? "No recipients match the filters — loosen one or clear them."
                    : "No recipients match."}
                </p>
              ) : (
                matchingRecipients.map((r) => {
                  const selected = value.selected.includes(r.userId);
                  const contactValue =
                    contactKey !== null
                      ? String(r.contacts?.[contactKey] ?? "").trim()
                      : "";
                  const toggle = () =>
                    set({
                      selected: selected
                        ? value.selected.filter((id) => id !== r.userId)
                        : [...value.selected, r.userId],
                    });
                  return (
                    <button
                      key={r.id}
                      type="button"
                      role="checkbox"
                      aria-checked={selected}
                      onClick={toggle}
                      className="flex w-full items-center gap-2 rounded-md px-2.5 py-2 text-left text-sm transition-colors hover:bg-muted"
                    >
                      <span
                        className={cn(
                          "flex size-4 shrink-0 items-center justify-center rounded border",
                          selected
                            ? "border-primary bg-primary text-primary-foreground"
                            : "border-border text-transparent",
                        )}
                      >
                        <Check className="size-3" />
                      </span>
                      <span className="min-w-0 flex-1 truncate font-medium">
                        {r.name}
                      </span>
                      <span className="shrink-0 text-xs text-muted-foreground">
                        {contactKey
                          ? contactValue !== ""
                            ? contactValue
                            : `${CAMPAIGN_CHANNELS[value.channel]?.label ?? value.channel} contact missing`
                          : r.userId}
                      </span>
                    </button>
                  );
                })
              )}
            </div>
            <p className="text-xs text-muted-foreground">
              {value.selected.length} of {recipients.length} selected
              {contactKey
                ? ` — sends resolve each recipient's live contact at fire time.`
                : ""}
            </p>
          </div>
        )}
      </Field>

      <Field
        label="Timing"
        hint={
          scheduled
            ? "Fires automatically when the scheduled time passes — the channel needs a connected provider."
            : "Saved as a draft; send it whenever you're ready with Run now."
        }
      >
        <div className="flex items-center gap-2 rounded-lg bg-muted p-1">
          <SegmentButton
            active={!scheduled}
            title="Save as a draft"
            onClick={() => set({ timing: "draft" })}
          >
            Save as draft
          </SegmentButton>
          <SegmentButton
            active={scheduled}
            title="Schedule the send for a later time"
            onClick={() => set({ timing: "scheduled" })}
          >
            Schedule for later
          </SegmentButton>
        </div>
        {scheduled && (
          <Input
            type="datetime-local"
            value={value.sendAt}
            onChange={(e) => set({ sendAt: e.target.value })}
            className="h-9 w-full text-sm"
            aria-label="Schedule date and time"
          />
        )}
        {scheduled && value.channel && !channelConfiguredNow && (
          <div className="flex items-start gap-2 rounded-md border border-amber-500/40 bg-amber-500/10 p-2.5 text-xs text-amber-700 dark:text-amber-300">
            <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
            <p>
              {labelForChannel(value.channel)} has no connected provider yet.
              Connect one on the{" "}
              <Link href="/providers" className="underline">
                Providers page
              </Link>{" "}
              to schedule this campaign, or save it as a draft for now.
            </p>
          </div>
        )}
      </Field>
    </div>
  );
}

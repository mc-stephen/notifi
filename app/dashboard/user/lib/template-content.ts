import type { TemplateContent, TemplateVariable } from "./types";
import {
  Mail,
  MessageSquare,
  Smartphone,
  Inbox,
  Webhook,
  Hash,
  Phone,
  Send,
  CircleDot,
  type LucideIcon,
} from "lucide-react";

/// A single editable field within a template's per-channel content.
export type TemplateField = {
  key: string;
  label: string;
  multiline?: boolean;
  placeholder?: string;
};

export type TemplateChannelModel = {
  value: string;
  label: string;
  icon: LucideIcon;
  fields: TemplateField[];
};

/// The subset of backend-recognised channels we expose for authoring
/// templates in the dashboard.
export const TEMPLATE_CHANNELS: TemplateChannelModel[] = [
  {
    value: "email",
    label: "Email",
    icon: Mail,
    fields: [
      { key: "subject", label: "Subject", placeholder: "Email subject line" },
      { key: "html", label: "HTML body", multiline: true, placeholder: "<p>Hello {{first_name}}!</p>" },
      { key: "text", label: "Plain text body", multiline: true, placeholder: "Hello {{first_name}}!" },
    ],
  },
  {
    value: "sms",
    label: "SMS",
    icon: MessageSquare,
    fields: [
      { key: "sms", label: "SMS text", multiline: true, placeholder: "Your code is {{code}}" },
    ],
  },
  {
    value: "push",
    label: "Push",
    icon: Smartphone,
    fields: [
      { key: "title", label: "Title", placeholder: "New notification" },
      { key: "body", label: "Body", multiline: true, placeholder: "{{sender_name}} sent you a message" },
    ],
  },
  {
    value: "in_app",
    label: "In-app",
    icon: Inbox,
    fields: [
      { key: "title", label: "Title", placeholder: "New message from {{sender_name}}" },
      { key: "body", label: "Body", multiline: true, placeholder: "Message body…" },
    ],
  },
  {
    value: "webhook",
    label: "Webhook",
    icon: Webhook,
    fields: [
      { key: "payload", label: "JSON payload", multiline: true, placeholder: '{"event":"{{event_type}}"}' },
    ],
  },
  {
    value: "slack",
    label: "Slack",
    icon: Hash,
    fields: [
      { key: "text", label: "Message text", multiline: true, placeholder: "Hello <@{{user_id}}>" },
    ],
  },
  {
    value: "whatsapp",
    label: "WhatsApp",
    icon: Phone,
    fields: [
      { key: "text", label: "Message text", multiline: true, placeholder: "Hello {{first_name}}" },
    ],
  },
  {
    value: "telegram",
    label: "Telegram",
    icon: Send,
    fields: [
      { key: "text", label: "Message text", multiline: true, placeholder: "Hello {{first_name}}" },
    ],
  },
];

export function templateChannelModel(channel: string): TemplateChannelModel {
  // The pinned fallback tab: content used by any configured channel that
  // doesn't carry its own text.
  if (channel === DEFAULT_CONTENT_KEY) {
    return DEFAULT_TAB_MODEL;
  }
  return (
    TEMPLATE_CHANNELS.find((c) => c.value === channel) ?? {
      value: channel,
      // Humanize config-style types ("push-android" → "Push Android").
      label: channel
        .split(/[-_]/)
        .map((part) => part[0].toUpperCase() + part.slice(1))
        .join(" "),
      icon: MessageSquare,
      fields: [{ key: "body", label: "Body", multiline: true }],
    }
  );
}

/// The content model: a map keyed by channel value, each value being that
/// channel's field object — e.g. `{ email: {subject,html,text}, sms: {text} }`.
/// A top-level `variables` key may also exist (dynamic data) and is not a
/// channel: the set of covered channels is `Object.keys(content)` filtered
/// to `TEMPLATE_CHANNELS`.
export function coveredChannels(content: TemplateContent): string[] {
  return Object.keys(content).filter((key) =>
    TEMPLATE_CHANNELS.some((c) => c.value === key),
  );
}

/// The field object for a channel (empty when not present).
function channelObject(content: TemplateContent, channel: string): Record<string, unknown> {
  const value = content[channel];
  return typeof value === "object" && value !== null
    ? (value as Record<string, unknown>)
    : {};
}

/// Read a string field out of a channel's slice ("" when absent).
export function getChannelField(
  content: TemplateContent,
  channel: string,
  key: string,
): string {
  const value = channelObject(content, channel)[key];
  return typeof value === "string" ? value : "";
}

/// Write a string field into a channel's slice, adding the channel when
/// missing and dropping empty fields.
export function setChannelField(
  content: TemplateContent,
  channel: string,
  key: string,
  value: string,
): TemplateContent {
  const current = { ...channelObject(content, channel) };
  if (value) {
    current[key] = value;
  } else {
    delete current[key];
  }
  return { ...content, [channel]: current };
}

/* ------------------------------------------------------------------ */
/* Dynamic data — the template's declared {{placeholder}} vocabulary.   */
/* Lives in the reserved top-level `variables` key of the content blob. */
/* ------------------------------------------------------------------ */

/// Reserved content key declaring the template's {{refs}}. `variables` can
/// never collide with a channel: coveredChannels() only matches known ones.
export const VARIABLES_KEY = "variables";

/// Reserved content key holding the pinned fallback text — used by any
/// configured channel without its own content. Not a channel, so
/// coveredChannels() likewise skips it.
export const DEFAULT_CONTENT_KEY = "default";

export const DEFAULT_TAB_MODEL: TemplateChannelModel = {
  value: DEFAULT_CONTENT_KEY,
  label: "Default",
  icon: CircleDot,
  fields: [
    {
      key: "body",
      label: "Text",
      multiline: true,
      placeholder: "Fallback text for configured channels without their own",
    },
  ],
};

export function getVariables(content: TemplateContent): TemplateVariable[] {
  const raw = content[VARIABLES_KEY];
  if (!Array.isArray(raw)) return [];
  return raw.filter(
    (v): v is TemplateVariable =>
      typeof v === "object" && v !== null && typeof (v as TemplateVariable).key === "string",
  );
}

export function setVariables(
  content: TemplateContent,
  variables: TemplateVariable[],
): TemplateContent {
  const next = { ...content };
  if (variables.length === 0) {
    delete next[VARIABLES_KEY];
  } else {
    next[VARIABLES_KEY] = variables;
  }
  return next;
}

/// Every `{{ref}}` identifier used in a text, in appearance order.
/// Matches `{{first_name}}`, `{{ user.id }}` — letters, digits, `_ . -`.
export function extractRefs(text: string): string[] {
  const regex = /\{\{\s*([a-zA-Z0-9_.-]+)\s*\}\}/g;
  const refs: string[] = [];
  for (const match of text.matchAll(regex)) {
    if (!refs.includes(match[1])) refs.push(match[1]);
  }
  return refs;
}

/// Reserved `{{recipient.<key>}}` namespace: refs resolved from the
/// recipient record at send time (name, user_id, any contact key) — immune
/// to Dynamic data declarations by design. Anything bare stays declared.
export const RECIPIENT_REF_PREFIX = "recipient";

/// Any `{{ref}}` used anywhere in the content that is not declared in
/// `:variables` (dyn. data). Each entry carries where it was found, so the
/// editor can say exactly which field to fix. Scans covered channels plus
/// the reserved fallback slice.
export function undeclaredRefs(content: TemplateContent): {
  channel: string;
  fieldKey: string;
  ref: string;
}[] {
  const declared = new Set(getVariables(content).map((v) => v.key));
  const slices = [...coveredChannels(content)];
  if (DEFAULT_CONTENT_KEY in content) slices.push(DEFAULT_CONTENT_KEY);
  const missing: { channel: string; fieldKey: string; ref: string }[] = [];
  for (const channel of slices) {
    for (const fieldKey of Object.keys(content[channel] as object)) {
      const value = (content[channel] as Record<string, unknown>)[fieldKey];
      if (typeof value !== "string") continue;
      for (const ref of extractRefs(value)) {
        if (ref.startsWith(RECIPIENT_REF_PREFIX + ".")) continue;
        if (!declared.has(ref)) missing.push({ channel, fieldKey, ref });
      }
    }
  }
  return missing;
}

/// Content is the source of truth: a key survives only when it carries at
/// least one non-empty field value — a channel slice that has none either
/// never existed or renders the Default fallback. Keeps the reserved
/// `variables` key untouched.
export function pruneEmptyChannels(content: TemplateContent): TemplateContent {
  const next: TemplateContent = {};
  for (const [key, slice] of Object.entries(content)) {
    if (key === VARIABLES_KEY) {
      next[key] = slice;
      continue;
    }
    const values =
      typeof slice === "object" && slice !== null
        ? Object.values(slice as Record<string, unknown>)
        : [];
    if (values.some((v) => typeof v === "string" && v.length > 0)) {
      next[key] = slice;
    }
  }
  return next;
}

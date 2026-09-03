import type { TemplateContent } from "./types";
import {
  Mail,
  MessageSquare,
  Smartphone,
  Inbox,
  Webhook,
  Hash,
  Phone,
  Send,
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
  return (
    TEMPLATE_CHANNELS.find((c) => c.value === channel) ?? {
      value: channel,
      label: channel,
      icon: MessageSquare,
      fields: [{ key: "body", label: "Body", multiline: true }],
    }
  );
}

/// The content model: a map keyed by channel value, each value being that
/// channel's field object — e.g. `{ email: {subject,html,text}, sms: {text} }`.
/// The set of covered channels for a template is `Object.keys(content)`.
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

/// Build a fresh per-channel slice with empty string fields so the editor
/// shows them. Used when creating or newly adding a channel.
export function emptyChannelFields(channel: string, includeEmpty: boolean): TemplateContent {
  const model = templateChannelModel(channel);
  const obj: Record<string, unknown> = {};
  for (const field of model.fields) {
    obj[field.key] = includeEmpty ? "" : undefined;
  }
  const cleaned: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(obj)) {
    if (v !== undefined) cleaned[k] = v;
  }
  return cleaned;
}

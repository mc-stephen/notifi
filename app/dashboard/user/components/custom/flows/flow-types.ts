import type { Edge, Node } from "@xyflow/react";

export type FlowNodeKind =
  | "trigger"
  | "provider"
  | "template"
  | "http-request"
  | "condition"
  | "ai-transform"
  | "stop";

export type TriggerKind = "event" | "webhook" | "schedule";

export type TriggerData = {
  /** Missing (legacy docs) reads as "event". */
  kind?: TriggerKind;
  /** Palette group that created the node (e.g. "Campaigns"); scopes the panel. */
  preset?: string;
  /** Event triggers only. */
  event?: string;
  /** Webhook triggers only: unguessable inbound token. */
  token?: string;
  /** Webhook/schedule triggers: the owner's event filter (read-only). */
  events?: string[];
  /** Webhook triggers only: operator note. */
  note?: string;
  /** Webhook triggers only: fallback message template. Legacy — the
   * panel no longer edits it; the POST body drives content (caller
   * fields + caller templateId). */
  templateId?: string;
  /** Webhook triggers only: presets for the fallback template's
   * declared refs (the backend renders them at fire time;
   * `{{recipient.*}}` never needs one — it resolves from the
   * recipient). Rides the node, same doctrine as `TemplateData`. */
  values?: Record<string, string>;
  /** Webhook triggers only: fallback channel. */
  channel?: string;
  /** Webhook triggers only: fallback recipients. */
  recipients?: string[];
  /** Webhook triggers only: accepted inbound secrets (`whsk_` keys).
   * Any listed key passes in the `x-notifi-secret` header. Empty =
   * open door. */
  secrets?: string[];
  /** Webhook triggers only: caller IP allow-list (plain IP or CIDR).
   * Empty = any address. */
  allowedIps?: string[];
};

/** Normalizes trigger data: legacy `{event}` docs read as event-kind. */
export function triggerKindOf(data: TriggerData): TriggerKind {
  return data.kind ?? "event";
}

/** A full-entropy inbound token matching the server's spec (exactly 32
 * hex chars, `valid_trigger_token` in `inbound/entities`). The builder
 * mints its own so hand-placed webhook triggers can fire through the
 * same `/v1/hooks` door as owner-page ones — the token index is synced
 * from the definition on every save either way. */
export function mintWebhookToken(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

/** An inbound secret key (`whsk_` + 128-bit hex) for the builder panel —
 * the caller echoes it back in the `x-notifi-secret` header on every
 * fire call. */
export function mintWebhookSecret(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
  return `whsk_${hex}`;
}

/** Shape-checks an IP allow-list entry: a plain IPv4/IPv6 address or a
 * CIDR block (`10.0.0.0/8`). Exact validity is the server's `ipnet`
 * parser's truth (`entities::ip_allowed` — a malformed rule never
 * widens the door); client-side this keeps obvious typos out. */
export function validIpRule(rule: string): boolean {
  const v = rule.trim();
  if (!v) return false;
  if (v.includes("/")) {
    const [addr, mask] = v.split("/");
    if (!/^\d{1,3}$/.test(mask)) return false;
    const bits = Number(mask);
    const maxBits = addr.includes(":") ? 128 : 32;
    return bits <= maxBits && validIpRule(addr);
  }
  if (v.includes(":")) {
    // Loose IPv6: hex digits and colons only.
    return /^[0-9a-fA-F:]+$/.test(v);
  }
  return /^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}$/.test(
    v,
  );
}

export type ProviderData = {
  channelId: string;
  providerId: string;
  priority: number;
  /** Palette channel that created the node; scopes the panel. */
  preset?: string;
};

export type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE";

export const HTTP_METHODS: HttpMethod[] = ["GET", "POST", "PUT", "PATCH", "DELETE"];

export type HttpRequestData = {
  url: string;
  method: HttpMethod;
  headers: { key: string; value: string }[];
  body: string;
};

export type ConditionData = {
  field: string;
  operator: string;
  value: string;
};

export type AiTransformData = {
  model: string;
  instruction: string;
  useBrandWrapper: boolean;
};

export type StopData = {
  reason: string;
};

/** Render a saved template's content — message content lives in the flow. */
export type TemplateData = {
  templateId: string;
  /** Name snapshot for the node + panel labels. */
  templateName?: string;
  /** Node-level presets for the template's declared refs
   * (`content.variables`, see template-content.ts). Values ride the node,
   * not the template, so the same template can take different values in
   * two nodes. `{{recipient.*}}` refs never appear here — they resolve
   * from the recipient at delivery. */
  values?: Record<string, string>;
};

export type FlowNodeData =
  | TriggerData
  | ProviderData
  | TemplateData
  | HttpRequestData
  | ConditionData
  | AiTransformData
  | StopData;

export type FlowNode = Node<FlowNodeData, FlowNodeKind>;

/** Port ids. Provider/condition nodes fan out; everything else is linear. */
export const HANDLE = {
  in: "in",
  out: "out",
  success: "success",
  failure: "failure",
  yes: "yes",
  no: "no",
} as const;

export const TRIGGER_EVENTS = [
  "notification.created",
  "notification.failed",
] as const;

export const CONDITION_FIELDS = [
  "recipient.gender",
  "recipient.country",
  "message.channel",
  "message.priority",
  "delivery.attempt",
  // Last HTTP Request step outcome ("success" | "failed"). Evaluated by
  // the M-flow traversal engine; until then it documents intent.
  "http.status",
] as const;

export const CONDITION_OPERATORS = [
  "equals",
  "not-equals",
  "contains",
] as const;

export const AI_MODELS = ["auto", "gpt-4o-mini", "gpt-4o"] as const;

export type FlowDoc = {
  version: 1;
  nodes: FlowNode[];
  edges: Edge[];
};

export type FlowIssueSeverity = "error" | "warning";

export type FlowIssue = {
  severity: FlowIssueSeverity;
  nodeId?: string;
  message: string;
};

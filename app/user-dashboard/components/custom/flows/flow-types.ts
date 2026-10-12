import type { Edge, Node } from "@xyflow/react";

export type FlowNodeKind =
  | "trigger"
  | "provider"
  | "http-request"
  | "condition"
  | "ai-transform"
  | "stop";

export type TriggerKind = "event" | "webhook";

export type TriggerData = {
  /** Missing (legacy docs) reads as "event". */
  kind?: TriggerKind;
  /** Palette group that created the node (e.g. "Campaigns"); scopes the panel. */
  preset?: string;
  /** Event triggers only. */
  event?: string;
  /** Webhook triggers only: unguessable inbound token. */
  token?: string;
  /** Webhook triggers only: operator note. */
  note?: string;
  /** Webhook triggers only: fallback message template. */
  templateId?: string;
  /** Webhook triggers only: fallback channel. */
  channel?: string;
  /** Webhook triggers only: fallback recipients. */
  recipients?: string[];
};

/** Normalizes trigger data: legacy `{event}` docs read as event-kind. */
export function triggerKindOf(data: TriggerData): TriggerKind {
  return data.kind ?? "event";
}

/** Unguessable token for inbound webhook triggers (128 bits, hex). */
export function newTriggerToken(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
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

export type FlowNodeData =
  | TriggerData
  | ProviderData
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

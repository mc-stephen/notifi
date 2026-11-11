/** Node data shapes for the builder panel's Incoming / Outgoing tabs.
 *
 * No run engine exists yet, so shapes are derived from the graph —
 * type-level truth, tagged "sample". One real data source exists today:
 * the webhook trigger's Incoming tab shows the door's actual last ping
 * (`GET /app/flows/{id}/inbound/last`) when the server has one. The
 * day per-node run traces land, these same shapes get real values. */

import type { FlowNode, TriggerData } from "./flow-types";
import { triggerKindOf } from "./flow-types";

export type ShapeRow = {
  key: string;
  value: string;
  note?: string;
};

export type DataShape = {
  /** Pretty-printed payload for the JSON block. */
  json: string;
  /** Flat rows for the key/value table (top-level fields). */
  rows: ShapeRow[];
  /** True only when the values came from a real ping/run. */
  real: boolean;
};

function shapeOf(
  sample: Record<string, unknown>,
  notes: Record<string, string> = {},
  real = false,
): DataShape {
  const rows: ShapeRow[] = Object.entries(sample)
    .filter(([, v]) => v !== undefined)
    .map(([key, v]) => ({
      key,
      value: typeof v === "string" ? v : JSON.stringify(v) ?? "null",
      note: notes[key],
    }));
  return { json: JSON.stringify(sample, null, 2), rows, real };
}

const STR = (v: unknown) => typeof v === "string" && v.trim() !== "" ? v : undefined;

/** What this node passes downstream — the shape its connected
 * successors receive as their incoming data. */
export function outgoingShapeOf(node: FlowNode): DataShape {
  const data = node.data as Record<string, unknown>;
  switch (node.type) {
    case "trigger": {
      const t = data as unknown as TriggerData;
      if (triggerKindOf(t) === "webhook") {
        return shapeOf(
          {
            event: "webhook",
            note: STR(t.note),
            values: t.values ?? {},
            channel: STR(t.channel),
            recipients: t.recipients,
            templateId: STR(t.templateId),
          },
          {
            event: "the caller's fire becomes this door's event",
            values: "presets for the fallback template's declared refs",
            channel: "fallback — the POST body wins",
            recipients: "fallback — the POST body wins",
            templateId: "fallback template (legacy — the body drives content)",
          },
        );
      }
      if (triggerKindOf(t) === "schedule") {
        return shapeOf(
          { event: t.events?.[0] ?? "schedule.tick", scheduledAt: "<ISO time>" },
          { event: "the schedule's event filter", scheduledAt: "each tick stamps the run" },
        );
      }
      return shapeOf(
        { event: t.event ?? "notification.created" },
        { event: "the platform event that starts the run" },
      );
    }
    case "provider":
      return shapeOf(
        {
          channel: data.channelId,
          provider: data.providerId,
          priority: data.priority,
        },
        { channel: "delivers through this channel's provider chain" },
      );
    case "template":
      return shapeOf(
        {
          channel: "<template channel>",
          title: "<rendered subject/title>",
          body: "<rendered body>",
          ...Object.fromEntries(
            Object.entries(
              (data as unknown as { values?: Record<string, string> }).values ?? {},
            ).map(([k, v]) => [`values.${k}`, v]),
          ),
        },
        {
          channel: "the template's channel",
          title: "rendered from the template",
          body: "rendered from the template",
        },
      );
    case "http-request":
      return shapeOf(
        { status: "<response code>", headers: {}, body: "<response body>" },
        { status: "what the called URL answered" },
      );
    case "condition":
      return shapeOf(
        { field: data.field, operator: data.operator, value: data.value, branch: "true | false" },
        { branch: "which way the check went" },
      );
    case "ai-transform":
      return shapeOf(
        { model: data.model, text: "<rewritten content>" },
        { text: "the model's rewrite" },
      );
    case "stop":
      return shapeOf({ stopped: true, reason: STR(data.reason) });
    default:
      return shapeOf({});
  }
}

/** What arrives at this node. Triggers describe their door's ping (or
 * platform event); every other node reads its first upstream node's
 * outgoing shape — that node's output is this node's incoming data. */
export function incomingShapeOf(node: FlowNode, upstream: FlowNode | null): DataShape | null {
  if (node.type === "trigger") {
    const t = node.data as unknown as TriggerData;
    if (triggerKindOf(t) === "webhook") {
      // Sample until a real ping exists — the panel swaps in the door's
      // last-ping probe when the server has one.
      return shapeOf(
        {
          clientIp: "203.0.113.9",
          contentType: "application/json",
          userAgent: "your-service/1.0",
          forwardedFor: "203.0.113.9",
          body: {
            channel: "email",
            recipients: ["user@example.com"],
            title: "Order shipped",
            body: "On its way",
          },
        },
        {
          clientIp: "the address that POSTed to the door",
          contentType: "content-type header",
          userAgent: "user-agent header",
          forwardedFor: "x-forwarded-for header",
          body: "the JSON the caller sent",
        },
      );
    }
    if (triggerKindOf(t) === "schedule") {
      return shapeOf({ scheduledAt: "<ISO time>" }, { scheduledAt: "the tick that started the run" });
    }
    return shapeOf(
      { event: t.event ?? "notification.created" },
      { event: "the platform event payload" },
    );
  }
  if (!upstream) return null;
  return outgoingShapeOf(upstream);
}

/** The door's real last ping, as the probe endpoint returns it — the
 * Incoming tab's actual-data shape. */
export type InboundProbeDto = {
  token: string;
  clientIp?: string | null;
  contentType?: string | null;
  userAgent?: string | null;
  forwardedFor?: string | null;
  body?: string | null;
  firedAt: string;
};

export function probeShapeOf(probe: InboundProbeDto): DataShape {
  return shapeOf(
    {
      clientIp: probe.clientIp,
      contentType: probe.contentType,
      userAgent: probe.userAgent,
      forwardedFor: probe.forwardedFor,
      body: probe.body ?? undefined,
      firedAt: probe.firedAt,
    },
    {
      clientIp: "the address that POSTed to the door",
      contentType: "content-type header",
      userAgent: "user-agent header",
      forwardedFor: "x-forwarded-for header",
      body: "the JSON the caller sent",
      firedAt: "when the door was hit",
    },
    true,
  );
}

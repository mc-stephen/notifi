import type { Edge } from "@xyflow/react";
import { HANDLE, type ConditionData, type FlowIssue, type FlowNode, type ProviderData, type TriggerData } from "./flow-types";

function isProviderData(d: unknown): d is ProviderData {
  return typeof d === "object" && d !== null && "providerId" in d && "priority" in d;
}

function isConditionData(d: unknown): d is ConditionData {
  return typeof d === "object" && d !== null && "field" in d && "operator" in d;
}

function isHttpRequestData(d: unknown): d is { url?: unknown; method?: unknown } {
  return typeof d === "object" && d !== null && "url" in d && "method" in d;
}

function isHttpUrl(url: unknown): boolean {
  if (typeof url !== "string" || !url.trim()) return false;
  try {
    const parsed = new URL(url);
    return parsed.protocol === "http:" || parsed.protocol === "https:";
  } catch {
    return false;
  }
}

/**
 * Pure graph validation for v1 flows. Errors block test runs; warnings
 * are advisory. No I/O, no React — unit-testable in isolation.
 */
export function validateFlow(nodes: FlowNode[], edges: Edge[]): FlowIssue[] {
  const issues: FlowIssue[] = [];
  const byId = new Map(nodes.map((n) => [n.id, n]));
  const outHandles = new Map<string, Set<string>>();
  for (const e of edges) {
    if (!byId.has(e.source) || !byId.has(e.target)) {
      issues.push({ severity: "error", message: "An edge points to a missing node." });
      continue;
    }
    if (e.source === e.target) {
      issues.push({ severity: "error", nodeId: e.source, message: "A node cannot connect to itself." });
      continue;
    }
    const set = outHandles.get(e.source) ?? new Set<string>();
    set.add(e.sourceHandle ?? HANDLE.out);
    outHandles.set(e.source, set);
  }

  const triggers = nodes.filter((n) => n.type === "trigger");
  if (triggers.length === 0) {
    issues.push({ severity: "error", message: "Add a trigger — every flow starts with exactly one." });
  } else if (triggers.length > 1) {
    for (const t of triggers) {
      issues.push({ severity: "error", nodeId: t.id, message: "Only one trigger per flow." });
    }
  }

  // Reachability from the (first) trigger.
  const reachable = new Set<string>();
  const start = triggers[0]?.id;
  if (start) {
    const queue = [start];
    reachable.add(start);
    const outgoing = new Map<string, string[]>();
    for (const e of edges) {
      const list = outgoing.get(e.source) ?? [];
      list.push(e.target);
      outgoing.set(e.source, list);
    }
    while (queue.length > 0) {
      const id = queue.pop()!;
      for (const next of outgoing.get(id) ?? []) {
        if (!reachable.has(next)) {
          reachable.add(next);
          queue.push(next);
        }
      }
    }
    for (const n of nodes) {
      if (!reachable.has(n.id)) {
        issues.push({ severity: "error", nodeId: n.id, message: "Node is not reachable from the trigger." });
      }
    }
  }

  for (const n of nodes) {
    const handles = outHandles.get(n.id) ?? new Set<string>();
    const need = (h: string, label: string) => {
      if (!handles.has(h)) {
        issues.push({ severity: "error", nodeId: n.id, message: `${label} needs an outgoing connection.` });
      }
    };
    switch (n.type) {
      case "trigger": {
        const data = n.data as TriggerData;
        const kind = data.kind ?? "event";
        if (kind === "event" && !data.event) {
          issues.push({ severity: "error", nodeId: n.id, message: "Pick an event for the trigger." });
        }
        if (kind === "webhook" && !data.token) {
          issues.push({ severity: "error", nodeId: n.id, message: "No token on this webhook trigger — start the flow from the Webhooks page instead of placing the trigger by hand." });
        }
        // "schedule" triggers are owner-side data — timing lives on the
        // schedule, nothing to check locally.
        need(HANDLE.out, "Trigger output");
        break;
      }
      case "provider": {
        if (!isProviderData(n.data) || !n.data.providerId) {
          issues.push({ severity: "error", nodeId: n.id, message: "Pick a provider to send with." });
        }
        if (!isProviderData(n.data) || !Number.isInteger(n.data.priority) || n.data.priority < 1) {
          issues.push({ severity: "error", nodeId: n.id, message: "Priority must be a positive whole number." });
        }
        need(HANDLE.success, "Success output");
        break;
      }
      case "template": {
        const data = n.data as { templateId?: unknown };
        if (typeof data.templateId !== "string" || data.templateId === "") {
          issues.push({ severity: "error", nodeId: n.id, message: "Pick a template for this node." });
        }
        need(HANDLE.out, "Template output");
        break;
      }
      case "http-request": {
        if (!isHttpRequestData(n.data) || !isHttpUrl(n.data.url)) {
          issues.push({ severity: "error", nodeId: n.id, message: "HTTP Request needs a valid http(s) URL." });
        }
        break;
      }
      case "condition": {
        if (!isConditionData(n.data) || !n.data.field || !n.data.value) {
          issues.push({ severity: "error", nodeId: n.id, message: "Condition needs a field, operator, and value." });
        }
        need(HANDLE.yes, "Yes branch");
        need(HANDLE.no, "No branch");
        break;
      }
      case "ai-transform": {
        const instruction = (n.data as { instruction?: string }).instruction ?? "";
        if (!instruction.trim()) {
          issues.push({ severity: "error", nodeId: n.id, message: "Describe what the AI should do." });
        }
        need(HANDLE.out, "AI output");
        break;
      }
      case "stop":
        break;
      default:
        issues.push({ severity: "error", nodeId: n.id, message: "Unknown node type." });
    }
  }

  return issues;
}

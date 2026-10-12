"use client";

import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";
import { useProjectStore } from "@/store/project-store";
import type { FlowDoc } from "@/components/custom/flows/flow-types";

export type ServerFlow = {
  id: string;
  projectId: string;
  name: string;
  definition: unknown;
  version: number;
  createdAt: string;
  updatedAt: string;
};

/** A flow qualifies for sends with an event trigger for the event. */
export function handlesSendEvent(definition: unknown): boolean {
  if (typeof definition !== "object" || definition === null) return false;
  const nodes = (definition as { nodes?: unknown }).nodes;
  if (!Array.isArray(nodes)) return false;
  return nodes.some((n) => {
    if (typeof n !== "object" || n === null) return false;
    if ((n as { type?: unknown }).type !== "trigger") return false;
    const data = (n as { data?: unknown }).data;
    if (typeof data !== "object" || data === null) return false;
    const kind = (data as { kind?: unknown }).kind;
    if (kind !== undefined && kind !== "event") return false;
    return (
      ((data as { event?: unknown }).event as string | undefined) ===
      "notification.created"
    );
  });
}

/** Narrow an unknown server definition to a FlowDoc, or null. */
export function asFlowDoc(value: unknown): FlowDoc | null {
  if (typeof value !== "object" || value === null) return null;
  const doc = value as { version?: unknown; nodes?: unknown; edges?: unknown };
  if (typeof doc.version !== "number" || !Array.isArray(doc.nodes) || !Array.isArray(doc.edges)) {
    return null;
  }
  return doc as FlowDoc;
}

type LegacyMeta = { id: string; name: string };

function readLegacyIndex(projectId: string): LegacyMeta[] {
  try {
    const raw = window.localStorage.getItem(`notifi:flows:${projectId}`);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter((m) => m && typeof (m as { id?: unknown }).id === "string")
      .map((m) => ({
        id: (m as { id: string }).id,
        name:
          typeof (m as { name?: unknown }).name === "string" &&
          ((m as { name: string }).name as string).trim()
            ? ((m as { name: string }).name as string)
            : "Imported flow",
      }));
  } catch {
    return [];
  }
}

function readLegacyDoc(projectId: string, flowId: string): FlowDoc | null {
  try {
    const raw = window.localStorage.getItem(`notifi:flow:${projectId}:${flowId}`);
    if (!raw) return null;
    return asFlowDoc(JSON.parse(raw));
  } catch {
    return null;
  }
}

function clearLegacy(projectId: string) {
  try {
    for (const m of readLegacyIndex(projectId)) {
      window.localStorage.removeItem(`notifi:flow:${projectId}:${m.id}`);
    }
    window.localStorage.removeItem(`notifi:flows:${projectId}`);
  } catch {
    // Ignore — drafts simply stay local and invisible.
  }
}

/**
 * One-time adoption of pre-server localStorage drafts into the server
 * store. Idempotent (clears legacy keys); returns true when anything was
 * imported. Runs on list + builder mounts so direct-URL entry works too.
 */
export async function adoptLegacyDrafts(projectId: string): Promise<boolean> {
  const drafts = readLegacyIndex(projectId);
  if (drafts.length === 0) {
    clearLegacy(projectId);
    return false;
  }
  let imported = false;
  for (const d of drafts) {
    const doc = readLegacyDoc(projectId, d.id);
    if (!doc) continue;
    try {
      await createFlow(projectId, d.name, doc);
      imported = true;
    } catch {
      // Keep going — one bad draft must not block the rest.
    }
  }
  clearLegacy(projectId);
  return imported;
}

function toMessage(e: unknown, fallback: string) {
  return e instanceof Error ? e.message : fallback;
}

export async function createFlow(
  projectId: string,
  name: string,
  definition?: FlowDoc,
): Promise<ServerFlow> {
  const res = await api<{ flow: ServerFlow }>("/app/flows", {
    method: "POST",
    body: JSON.stringify({ projectId, name, ...(definition ? { definition } : {}) }),
  });
  return res.flow;
}

export async function putFlow(
  projectId: string,
  id: string,
  name: string,
  definition: FlowDoc,
): Promise<ServerFlow> {
  const res = await api<{ flow: ServerFlow }>(`/app/flows/${id}`, {
    method: "PUT",
    body: JSON.stringify({ projectId, name, definition }),
  });
  return res.flow;
}

export async function deleteFlow(projectId: string, id: string): Promise<boolean> {
  const res = await api<{ deleted: boolean }>(
    `/app/flows/${id}?project_id=${encodeURIComponent(projectId)}`,
    { method: "DELETE" },
  );
  return res.deleted;
}

/**
 * Project flows, server-side source of truth. Replaces the old
 * localStorage index (removed): every flow that matters for sending
 * lives here.
 *
 * Reads go through a module-level per-project cache so navigating
 * between the list and the builder never flashes a loader on warm
 * data: first paint renders from cache, then revalidates silently.
 */
const flowsCache = new Map<string, ServerFlow[]>();

async function fetchFlows(projectId: string): Promise<ServerFlow[]> {
  const res = await api<{ flows: ServerFlow[] }>(
    `/app/flows?project_id=${encodeURIComponent(projectId)}`,
  );
  const flows = res.flows ?? [];
  flowsCache.set(projectId, flows);
  return flows;
}

export function useFlows() {
  const projectId = useProjectStore((s) => s.currentProject?.id);
  const [flows, setFlows] = useState<ServerFlow[]>(() =>
    projectId ? (flowsCache.get(projectId) ?? []) : [],
  );
  const [loading, setLoading] = useState(
    () => !!projectId && !flowsCache.has(projectId),
  );
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!projectId) {
      setFlows([]);
      setLoading(false);
      return;
    }
    const warm = flowsCache.has(projectId);
    if (!warm) setLoading(true);
    setError(null);
    try {
      setFlows(await fetchFlows(projectId));
    } catch (e) {
      if (!warm) {
        setFlows([]);
        setError(toMessage(e, "Failed to load flows"));
      }
      // Warm refresh failures keep stale rows on screen silently.
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    let ignore = false;
    (async () => {
      if (!projectId) {
        if (!ignore) {
          setFlows([]);
          setLoading(false);
          setError(null);
        }
        return;
      }
      const cached = flowsCache.get(projectId);
      if (cached) {
        // Instant paint; revalidate underneath.
        if (!ignore) {
          setFlows(cached);
          setLoading(false);
        }
        try {
          const fresh = await fetchFlows(projectId);
          if (!ignore) setFlows(fresh);
        } catch {
          // Stale rows stay; a cold load would have errored instead.
        }
        return;
      }
      if (!ignore) setLoading(true);
      setError(null);
      try {
        const fresh = await fetchFlows(projectId);
        if (!ignore) setFlows(fresh);
      } catch (e) {
        if (!ignore) {
          setFlows([]);
          setError(toMessage(e, "Failed to load flows"));
        }
      } finally {
        if (!ignore) setLoading(false);
      }
    })();
    return () => {
      ignore = true;
    };
  }, [projectId]);

  return { flows, loading, error, refresh };
}

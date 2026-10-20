"use client";

import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";
import { useProjectStore } from "@/store/project-store";

export type ServerWebhook = {
  id: string;
  projectId: string;
  name: string;
  description: string;
  /** The webhook's own flow — created (and deleted) with the webhook. */
  flowId: string;
  flowName: string;
  /** Event names the trigger listens for. */
  events: string[];
  /** Off = the flow never fires even though everything still exists. */
  active: boolean;
  /** Non-null = parked: kept but no longer firing. */
  archivedAt: string | null;
  createdAt: string;
  updatedAt: string;
};

export type WebhookCreateInput = {
  name: string;
  description?: string;
  events?: string[];
};

export type WebhookPatchInput = {
  name?: string;
  description?: string;
  events?: string[];
  active?: boolean;
  /** true parks the webhook; false restores it. */
  archived?: boolean;
};

function toMessage(e: unknown, fallback: string) {
  return e instanceof Error ? e.message : fallback;
}

export async function createWebhook(
  projectId: string,
  input: WebhookCreateInput,
): Promise<ServerWebhook> {
  const res = await api<{ webhook: ServerWebhook }>("/app/webhooks", {
    method: "POST",
    body: JSON.stringify({ projectId, ...input }),
  });
  return res.webhook;
}

export async function patchWebhook(
  projectId: string,
  id: string,
  patch: WebhookPatchInput,
): Promise<ServerWebhook> {
  const res = await api<{ webhook: ServerWebhook }>(`/app/webhooks/${id}`, {
    method: "PATCH",
    body: JSON.stringify({ projectId, ...patch }),
  });
  return res.webhook;
}

export async function deleteWebhook(projectId: string, id: string): Promise<boolean> {
  const res = await api<{ deleted: boolean }>(
    `/app/webhooks/${id}?project_id=${encodeURIComponent(projectId)}`,
    { method: "DELETE" },
  );
  return res.deleted;
}

/**
 * Project webhooks, server-side source of truth. Each row owns one flow
 * that never appears on the Flows page — the builder opens it from here.
 */
export function useWebhooks() {
  const projectId = useProjectStore((s) => s.currentProject?.id);
  const [webhooks, setWebhooks] = useState<ServerWebhook[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!projectId) {
      setWebhooks([]);
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const res = await api<{ webhooks: ServerWebhook[] }>(
        `/app/webhooks?project_id=${encodeURIComponent(projectId)}`,
      );
      setWebhooks(res.webhooks ?? []);
    } catch (e) {
      setWebhooks([]);
      setError(toMessage(e, "Failed to load webhooks"));
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    let ignore = false;
    (async () => {
      if (!projectId) {
        if (!ignore) {
          setWebhooks([]);
          setLoading(false);
        }
        return;
      }
      try {
        const res = await api<{ webhooks: ServerWebhook[] }>(
          `/app/webhooks?project_id=${encodeURIComponent(projectId)}`,
        );
        if (!ignore) setWebhooks(res.webhooks ?? []);
      } catch (e) {
        if (!ignore) {
          setWebhooks([]);
          setError(toMessage(e, "Failed to load webhooks"));
        }
      } finally {
        if (!ignore) setLoading(false);
      }
    })();
    return () => {
      ignore = true;
    };
  }, [projectId]);

  return { webhooks, loading, error, refresh };
}

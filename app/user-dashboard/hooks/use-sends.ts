"use client";

import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import { useProjectStore } from "@/store/project-store";
import type { DeliverySend } from "@/lib/types";

function toMessage(e: unknown, fallback: string) {
  return e instanceof Error ? e.message : fallback;
}

/**
 * Newest-first product sends for the deliveries view. Read-only in v1
 * (no retry/cancel/delete endpoints yet — those actions stay out of the
 * table rather than shipping dead).
 */
export function useSends() {
  const projectId = useProjectStore((s) => s.currentProject?.id);
  const [sends, setSends] = useState<DeliverySend[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let ignore = false;
    (async () => {
      setLoading(true);
      setError(null);
      if (!projectId) {
        if (!ignore) {
          setSends([]);
          setLoading(false);
        }
        return;
      }
      try {
        const res = await api<{ sends: DeliverySend[] }>(
          `/app/sends?project_id=${encodeURIComponent(projectId)}&limit=100`,
        );
        if (ignore) return;
        setSends(res.sends ?? []);
        setLoading(false);
      } catch (e) {
        if (ignore) return;
        setSends([]);
        setError(toMessage(e, "Failed to load deliveries"));
        setLoading(false);
      }
    })();
    return () => {
      ignore = true;
    };
  }, [projectId]);

  return { sends, loading, error };
}

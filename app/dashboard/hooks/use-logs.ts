"use client";

import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";

export type AuditLog = {
  id: string;
  userId?: string | null;
  actorName?: string | null;
  eventType: string;
  message: string;
  projectId?: string | null;
  metadata?: Record<string, unknown> | null;
  occurredAt: string;
};

export function useLogs() {
  const [logs, setLogs] = useState<AuditLog[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let ignore = false;
    (async () => {
      try {
        const { logs } = await api<{ logs: AuditLog[] }>("/v1/logs");
        if (ignore) return;
        setLogs(logs);
        setError(null);
      } catch (e) {
        if (ignore) return;
        setLogs([]);
        setError(e instanceof Error ? e.message : "Failed to load logs");
      } finally {
        if (!ignore) setLoading(false);
      }
    })();
    return () => {
      ignore = true;
    };
  }, []);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const { logs } = await api<{ logs: AuditLog[] }>("/v1/logs");
      setLogs(logs);
      setError(null);
    } catch (e) {
      setLogs([]);
      setError(e instanceof Error ? e.message : "Failed to load logs");
    } finally {
      setLoading(false);
    }
  }, []);

  return { logs, loading, error, refresh };
}

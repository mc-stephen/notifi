"use client";

import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";
import { useProjectStore } from "@/store/project-store";

export type ServerSchedule = {
  id: string;
  projectId: string;
  name: string;
  /** The schedule's own flow — created (and deleted) with the schedule. */
  flowId: string;
  flowName: string;
  kind: "once" | "recurring" | "weekly";
  runAt: string | null;
  intervalSecs: number | null;
  days: number[];
  runTime: string | null;
  timezone: string;
  templateId: string | null;
  templateName: string | null;
  channel: string;
  title: string;
  body: string;
  recipients: string[];
  active: boolean;
  /** Non-null = parked: kept but no longer counted or fired. */
  archivedAt: string | null;
  nextRunAt: string | null;
  lastRunAt: string | null;
  lastStatus: string | null;
  lastError: string | null;
  runCount: number;
  timing: string;
  createdAt: string;
  updatedAt: string;
};

export type ScheduleCreateInput = {
  name: string;
  kind: "once" | "recurring" | "weekly";
  runAt?: string | null;
  intervalSecs?: number | null;
  days?: number[];
  runTime?: string | null;
  timezone?: string;
  templateId?: string | null;
  channel?: string;
  title?: string;
  body?: string;
  recipients: string[];
};

export type SchedulePatchInput = {
  name?: string;
  kind?: "once" | "recurring" | "weekly";
  /** null clears the field (kind switches); absent keeps it. */
  runAt?: string | null;
  intervalSecs?: number | null;
  days?: number[];
  runTime?: string | null;
  timezone?: string;
  templateId?: string | null;
  channel?: string;
  title?: string;
  body?: string;
  recipients?: string[];
  active?: boolean;
  /** true parks the schedule; false restores it. */
  archived?: boolean;
};

function toMessage(e: unknown, fallback: string) {
  return e instanceof Error ? e.message : fallback;
}

export async function createSchedule(
  projectId: string,
  input: ScheduleCreateInput,
): Promise<ServerSchedule> {
  const res = await api<{ schedule: ServerSchedule }>("/app/schedules", {
    method: "POST",
    body: JSON.stringify({ projectId, ...input }),
  });
  return res.schedule;
}

export async function patchSchedule(
  projectId: string,
  id: string,
  patch: SchedulePatchInput,
): Promise<ServerSchedule> {
  const res = await api<{ schedule: ServerSchedule }>(`/app/schedules/${id}`, {
    method: "PATCH",
    body: JSON.stringify({ projectId, ...patch }),
  });
  return res.schedule;
}

export async function deleteSchedule(projectId: string, id: string): Promise<boolean> {
  const res = await api<{ deleted: boolean }>(
    `/app/schedules/${id}?project_id=${encodeURIComponent(projectId)}`,
    { method: "DELETE" },
  );
  return res.deleted;
}

export async function runScheduleNow(
  projectId: string,
  id: string,
): Promise<ServerSchedule> {
  const res = await api<{ schedule: ServerSchedule }>(
    `/app/schedules/${id}/run-now`,
    {
      method: "POST",
      body: JSON.stringify({ projectId }),
    },
  );
  return res.schedule;
}

/**
 * Project flow schedules, server-side source of truth. Replaces the old
 * mock list: every schedule runs an assigned flow on its timing.
 */
export function useSchedules() {
  const projectId = useProjectStore((s) => s.currentProject?.id);
  const [schedules, setSchedules] = useState<ServerSchedule[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!projectId) {
      setSchedules([]);
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const res = await api<{ schedules: ServerSchedule[] }>(
        `/app/schedules?project_id=${encodeURIComponent(projectId)}`,
      );
      setSchedules(res.schedules ?? []);
    } catch (e) {
      setSchedules([]);
      setError(toMessage(e, "Failed to load schedules"));
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    let ignore = false;
    (async () => {
      if (!projectId) {
        if (!ignore) {
          setSchedules([]);
          setLoading(false);
        }
        return;
      }
      try {
        const res = await api<{ schedules: ServerSchedule[] }>(
          `/app/schedules?project_id=${encodeURIComponent(projectId)}`,
        );
        if (!ignore) setSchedules(res.schedules ?? []);
      } catch (e) {
        if (!ignore) {
          setSchedules([]);
          setError(toMessage(e, "Failed to load schedules"));
        }
      } finally {
        if (!ignore) setLoading(false);
      }
    })();
    return () => {
      ignore = true;
    };
  }, [projectId]);

  return { schedules, loading, error, refresh };
}

"use client";

import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";
import { useProjectStore } from "@/store/project-store";

export type TeamMemberRecord = {
  userId: string;
  name: string;
  email: string;
  role: string;
  has2fa: boolean;
  lastActiveAt: string | null;
};

type TeamState = {
  members: TeamMemberRecord[];
  loading: boolean;
  error: string | null;
};

function toMessage(e: unknown, fallback: string) {
  return e instanceof Error ? e.message : fallback;
}

/** Live team roster for the current project, with 2FA standing. */
export function useTeamMembers() {
  const projectId = useProjectStore((s) => s.currentProject?.id);

  const [state, setState] = useState<TeamState>({
    members: [],
    loading: true,
    error: null,
  });

  const load = useCallback(async () => {
    if (!projectId) {
      setState({ members: [], loading: false, error: null });
      return;
    }
    setState((prev) => ({ ...prev, loading: true, error: null }));
    try {
      const res = await api<{ members: TeamMemberRecord[] }>(
        `/app/projects/${encodeURIComponent(projectId)}/members`
      );
      setState({ members: res.members, loading: false, error: null });
    } catch (e) {
      setState((prev) => ({ ...prev, loading: false, error: toMessage(e, "Failed to load team") }));
    }
  }, [projectId]);

  useEffect(() => {
    let ignore = false;
    (async () => {
      if (!projectId) {
        setState({ members: [], loading: false, error: null });
        return;
      }
      setState((prev) => ({ ...prev, loading: true, error: null }));
      try {
        const res = await api<{ members: TeamMemberRecord[] }>(
          `/app/projects/${encodeURIComponent(projectId)}/members`
        );
        if (ignore) return;
        setState({ members: res.members, loading: false, error: null });
      } catch (e) {
        if (ignore) return;
        setState((prev) => ({
          ...prev,
          loading: false,
          error: toMessage(e, "Failed to load team"),
        }));
      }
    })();
    return () => {
      ignore = true;
    };
  }, [projectId]);

  return {
    members: state.members,
    loading: state.loading,
    error: state.error,
    refresh: load,
  };
}

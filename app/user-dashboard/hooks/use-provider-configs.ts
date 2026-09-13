"use client";

import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";
import { useProjectStore } from "@/store/project-store";

export type ProviderConfig = {
  id: string;
  project_id: string;
  channel_id: string;
  provider_id: string;
  config: Record<string, unknown>;
  smtp_fallback?: Record<string, unknown> | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
};

export type ProviderConfigInput = {
  channel_id: string;
  provider_id: string;
  config: Record<string, unknown>;
  smtp_fallback?: Record<string, unknown> | null;
  enabled: boolean;
};

type ListState = {
  configs: ProviderConfig[];
  loading: boolean;
  error: string | null;
};

/// Provider configs connected to the current project.
/// Joined with the registry (names, icons, fields) at render time.
export function useProviderConfigs() {
  const projectId = useProjectStore((s) => s.currentProject?.id);
  const [state, setState] = useState<ListState>({
    configs: [],
    loading: true,
    error: null,
  });

  const refresh = useCallback(async () => {
    if (!projectId) return;
    setState((prev) => ({ ...prev, loading: true }));
    try {
      const res = await api<ProviderConfig[]>(
        `/app/projects/${projectId}/channel-configs`,
      );
      setState({ configs: res, loading: false, error: null });
    } catch (e) {
      setState({
        configs: [],
        loading: false,
        error: e instanceof Error ? e.message : "Failed to load providers",
      });
    }
  }, [projectId]);

  useEffect(() => {
    if (!projectId) return;

    let ignore = false;
    (async () => {
      try {
        const res = await api<ProviderConfig[]>(
          `/app/projects/${projectId}/channel-configs`,
        );
        if (ignore) return;
        setState({ configs: res, loading: false, error: null });
      } catch (e) {
        if (ignore) return;
        setState({
          configs: [],
          loading: false,
          error: e instanceof Error ? e.message : "Failed to load providers",
        });
      }
    })();
    return () => {
      ignore = true;
    };
  }, [projectId]);

  return {
    configs: state.configs,
    loading: state.loading && !!projectId,
    error: state.error,
    refresh,
  };
}

/// Create/test/update/remove actions scoped to the current project.
export function useProviderConfigActions() {
  const projectId = useProjectStore((s) => s.currentProject?.id);

  const test = useCallback(
    async (
      channel_id: string,
      provider_id: string,
      config: Record<string, unknown>,
    ): Promise<{ success: boolean; message: string }> => {
      if (!projectId) throw new Error("No project selected");
      return api<{ success: boolean; message: string }>(
        `/app/projects/${projectId}/channel-configs/test`,
        {
          method: "POST",
          body: JSON.stringify({ channel_id, provider_id, config }),
        },
      );
    },
    [projectId],
  );

  const create = useCallback(
    async (input: ProviderConfigInput): Promise<ProviderConfig> => {
      if (!projectId) throw new Error("No project selected");
      return api<ProviderConfig>(`/app/projects/${projectId}/channel-configs`, {
        method: "POST",
        body: JSON.stringify(input),
      });
    },
    [projectId],
  );

  const update = useCallback(
    async (
      configId: string,
      input: ProviderConfigInput,
    ): Promise<ProviderConfig> => {
      if (!projectId) throw new Error("No project selected");
      return api<ProviderConfig>(
        `/app/projects/${projectId}/channel-configs/${configId}`,
        { method: "PATCH", body: JSON.stringify(input) },
      );
    },
    [projectId],
  );

  const remove = useCallback(
    async (configId: string): Promise<void> => {
      if (!projectId) throw new Error("No project selected");
      await api<void>(
        `/app/projects/${projectId}/channel-configs/${configId}`,
        { method: "DELETE" },
      );
    },
    [projectId],
  );

  return { test, create, update, remove };
}

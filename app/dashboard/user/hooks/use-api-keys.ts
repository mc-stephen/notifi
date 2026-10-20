"use client";

import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";
import { useProjectStore } from "@/store/project-store";
import type { ApiKey, IpAllowEntry } from "@/lib/types";

export type CreateApiKeyInput = {
  name: string;
  permissions?: string[];
  expiresAt?: string | null;
};

export type UpdateApiKeyInput = {
  name?: string;
  enabled?: boolean;
};

type ListState = {
  keys: ApiKey[];
  loading: boolean;
  error: string | null;
};

/// All API keys for the current project, plus loading/error/refresh state.
/// The raw secret is never in this list — it is returned once on create.
export function useApiKeys() {
  const projectId = useProjectStore((s) => s.currentProject?.id);
  const [state, setState] = useState<ListState>({
    keys: [],
    loading: true,
    error: null,
  });

  useEffect(() => {
    if (!projectId) return;

    let ignore = false;
    (async () => {
      try {
        const res = await api<{ keys: ApiKey[] }>(
          `/app/projects/${projectId}/api-keys`,
        );
        if (ignore) return;
        setState({ keys: res.keys, loading: false, error: null });
      } catch (e) {
        if (ignore) return;
        setState({
          keys: [],
          loading: false,
          error: e instanceof Error ? e.message : "Failed to load API keys",
        });
      }
    })();
    return () => {
      ignore = true;
    };
  }, [projectId]);

  const refresh = useCallback(async () => {
    if (!projectId) return;
    setState((prev) => ({ ...prev, loading: true }));
    try {
      const res = await api<{ keys: ApiKey[] }>(
        `/app/projects/${projectId}/api-keys`,
      );
      setState({ keys: res.keys, loading: false, error: null });
    } catch (e) {
      setState({
        keys: [],
        loading: false,
        error: e instanceof Error ? e.message : "Failed to load API keys",
      });
    }
  }, [projectId]);

  return {
    keys: state.keys,
    loading: state.loading && !!projectId,
    error: state.error,
    refresh,
  };
}

/// Create/rename/enable/revoke actions scoped to the current project.
export function useApiKeyActions() {
  const projectId = useProjectStore((s) => s.currentProject?.id);

  const create = useCallback(
    async (input: CreateApiKeyInput): Promise<{ key: ApiKey; secret: string }> => {
      if (!projectId) throw new Error("No project selected");
      const res = await api<{ key: ApiKey; secret: string }>(
        `/app/projects/${projectId}/api-keys`,
        { method: "POST", body: JSON.stringify(input) },
      );
      return res;
    },
    [projectId],
  );

  const update = useCallback(
    async (keyId: string, input: UpdateApiKeyInput): Promise<ApiKey> => {
      if (!projectId) throw new Error("No project selected");
      const res = await api<{ key: ApiKey }>(
        `/app/projects/${projectId}/api-keys/${keyId}`,
        { method: "PATCH", body: JSON.stringify(input) },
      );
      return res.key;
    },
    [projectId],
  );

  const revoke = useCallback(
    async (keyId: string): Promise<ApiKey> => {
      if (!projectId) throw new Error("No project selected");
      const res = await api<{ key: ApiKey }>(
        `/app/projects/${projectId}/api-keys/${keyId}/revoke`,
        { method: "POST", body: JSON.stringify({}) },
      );
      return res.key;
    },
    [projectId],
  );

  return { create, update, revoke };
}

type IpState = {
  entries: IpAllowEntry[];
  loading: boolean;
  error: string | null;
};

/// Per-key IP allowlist. Empty means unrestricted — the first entry restricts.
export function useIpAllowlist(keyId: string | null) {
  const projectId = useProjectStore((s) => s.currentProject?.id);
  const [state, setState] = useState<IpState>({
    entries: [],
    loading: true,
    error: null,
  });

  const load = useCallback(async () => {
    if (!projectId || !keyId) return;
    setState((prev) => ({ ...prev, loading: true }));
    try {
      const res = await api<{ ips: IpAllowEntry[] }>(
        `/app/projects/${projectId}/api-keys/${keyId}/ips`,
      );
      setState({ entries: res.ips, loading: false, error: null });
    } catch (e) {
      setState({
        entries: [],
        loading: false,
        error: e instanceof Error ? e.message : "Failed to load IP allowlist",
      });
    }
  }, [projectId, keyId]);

  useEffect(() => {
    if (!projectId || !keyId) return;
    let ignore = false;
    (async () => {
      try {
        const res = await api<{ ips: IpAllowEntry[] }>(
          `/app/projects/${projectId}/api-keys/${keyId}/ips`,
        );
        if (ignore) return;
        setState({ entries: res.ips, loading: false, error: null });
      } catch (e) {
        if (ignore) return;
        setState({
          entries: [],
          loading: false,
          error: e instanceof Error ? e.message : "Failed to load IP allowlist",
        });
      }
    })();
    return () => {
      ignore = true;
    };
  }, [projectId, keyId]);

  const add = useCallback(
    async (cidr: string, label?: string): Promise<IpAllowEntry> => {
      if (!projectId || !keyId) throw new Error("No key selected");
      const res = await api<{ ip: IpAllowEntry }>(
        `/app/projects/${projectId}/api-keys/${keyId}/ips`,
        { method: "POST", body: JSON.stringify({ cidr, label }) },
      );
      await load();
      return res.ip;
    },
    [projectId, keyId, load],
  );

  const remove = useCallback(
    async (entryId: string): Promise<void> => {
      if (!projectId || !keyId) throw new Error("No key selected");
      await api(`/app/projects/${projectId}/api-keys/${keyId}/ips/${entryId}`, {
        method: "DELETE",
      });
      await load();
    },
    [projectId, keyId, load],
  );

  return {
    entries: state.entries,
    loading: state.loading && !!projectId && !!keyId,
    error: state.error,
    refresh: load,
    add,
    remove,
  };
}

"use client";

import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";
import { useProjectStore } from "@/store/project-store";
import type { Recipient } from "@/lib/types";

/// Partial input for creating a recipient.
export type CreateRecipientInput = {
  userId: string;
  name: string;
  contacts?: Record<string, unknown>;
};

/// Partial input for updating a recipient (name and/or contacts).
export type UpdateRecipientInput = {
  name?: string;
  contacts?: Record<string, unknown>;
};

type ListState = {
  recipients: Recipient[];
  loading: boolean;
  error: string | null;
};

/// All recipients for the current project, plus loading/error/refresh state.
export function useRecipients() {
  const projectId = useProjectStore((s) => s.currentProject?.id);
  const [state, setState] = useState<ListState>({
    recipients: [],
    loading: true,
    error: null,
  });

  useEffect(() => {
    // No project selected (e.g. still loading) — leave state untouched; the
    // derived `loading` below reports a clean idle state until one exists.
    if (!projectId) return;

    let ignore = false;
    (async () => {
      try {
        const res = await api<{ recipients: Recipient[] }>(
          `/v1/projects/${projectId}/recipients`,
        );
        if (ignore) return;
        setState({ recipients: res.recipients, loading: false, error: null });
      } catch (e) {
        if (ignore) return;
        setState({
          recipients: [],
          loading: false,
          error: e instanceof Error ? e.message : "Failed to load recipients",
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
      const res = await api<{ recipients: Recipient[] }>(
        `/v1/projects/${projectId}/recipients`,
      );
      setState({ recipients: res.recipients, loading: false, error: null });
    } catch (e) {
      setState({
        recipients: [],
        loading: false,
        error: e instanceof Error ? e.message : "Failed to load recipients",
      });
    }
  }, [projectId]);

  return {
    recipients: state.recipients,
    loading: state.loading && !!projectId,
    error: state.error,
    refresh,
  };
}

/// Create/delete actions scoped to the current project.
export function useRecipientActions() {
  const projectId = useProjectStore((s) => s.currentProject?.id);

  const create = useCallback(
    async (input: CreateRecipientInput): Promise<Recipient> => {
      if (!projectId) throw new Error("No project selected");
      const res = await api<{ recipient: Recipient }>(
        `/v1/projects/${projectId}/recipients`,
        { method: "POST", body: JSON.stringify(input) },
      );
      return res.recipient;
    },
    [projectId],
  );

  const remove = useCallback(
    async (recipientId: string): Promise<void> => {
      if (!projectId) throw new Error("No project selected");
      await api(`/v1/projects/${projectId}/recipients/${recipientId}`, {
        method: "DELETE",
      });
    },
    [projectId],
  );

  const update = useCallback(
    async (recipientId: string, input: UpdateRecipientInput): Promise<Recipient> => {
      if (!projectId) throw new Error("No project selected");
      const res = await api<{ recipient: Recipient }>(
        `/v1/projects/${projectId}/recipients/${recipientId}`,
        { method: "PATCH", body: JSON.stringify(input) },
      );
      return res.recipient;
    },
    [projectId],
  );

  return { create, remove, update };
}

type SingleState = {
  recipient: Recipient | null;
  loading: boolean;
  error: string | null;
};

/// A single recipient by id in the current project.
export function useRecipient(id: string) {
  const projectId = useProjectStore((s) => s.currentProject?.id);
  const [state, setState] = useState<SingleState>({
    recipient: null,
    loading: true,
    error: null,
  });

  const load = useCallback(async () => {
    if (!projectId) return;
    try {
      const res = await api<{ recipient: Recipient }>(
        `/v1/projects/${projectId}/recipients/${id}`,
      );
      setState({ recipient: res.recipient, loading: false, error: null });
    } catch (e) {
      setState({
        recipient: null,
        loading: false,
        error: e instanceof Error ? e.message : "Failed to load recipient",
      });
    }
  }, [id, projectId]);

  useEffect(() => {
    let ignore = false;
    (async () => {
      if (!projectId) return;
      try {
        const res = await api<{ recipient: Recipient }>(
          `/v1/projects/${projectId}/recipients/${id}`,
        );
        if (ignore) return;
        setState({ recipient: res.recipient, loading: false, error: null });
      } catch (e) {
        if (ignore) return;
        setState({
          recipient: null,
          loading: false,
          error: e instanceof Error ? e.message : "Failed to load recipient",
        });
      }
    })();
    return () => {
      ignore = true;
    };
  }, [id, projectId]);

  return {
    recipient: state.recipient,
    loading: state.loading && !!projectId,
    error: state.error,
    refresh: load,
  };
}

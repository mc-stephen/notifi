"use client";

import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";
import { useProjectStore } from "@/store/project-store";
import type { SupportTicket, TicketMessage } from "@/lib/types";

type ListState = {
  tickets: SupportTicket[];
  loading: boolean;
  error: string | null;
};

export function useSupportTickets() {
  const projectId = useProjectStore((s) => s.currentProject?.id);

  const [state, setState] = useState<ListState>({
    tickets: [],
    loading: true,
    error: null,
  });

  useEffect(() => {
    let ignore = false;
    (async () => {
      try {
        const url = projectId
          ? `/v1/support/tickets?project_id=${encodeURIComponent(projectId)}`
          : "/v1/support/tickets";
        const res = await api<{ tickets: SupportTicket[] }>(url);
        if (ignore) return;
        setState({ tickets: res.tickets, loading: false, error: null });
      } catch (e) {
        if (ignore) return;
        setState({
          tickets: [],
          loading: false,
          error: e instanceof Error ? e.message : "Failed to load tickets",
        });
      }
    })();
    return () => {
      ignore = true;
    };
  }, [projectId]);

  const refresh = useCallback(async () => {
    setState((prev) => ({ ...prev, loading: true }));
    try {
      const url = projectId
        ? `/v1/support/tickets?project_id=${encodeURIComponent(projectId)}`
        : "/v1/support/tickets";
      const res = await api<{ tickets: SupportTicket[] }>(url);
      setState({ tickets: res.tickets, loading: false, error: null });
    } catch (e) {
      setState({
        tickets: [],
        loading: false,
        error: e instanceof Error ? e.message : "Failed to load tickets",
      });
    }
  }, [projectId]);

  return {
    tickets: state.tickets,
    loading: state.loading,
    error: state.error,
    refresh,
  };
}

export type CreateTicketInput = {
  projectId?: string | null;
  subject: string;
  category: string;
  priority: string;
  description: string;
};

export function useSubmitTicket() {
  const projectId = useProjectStore((s) => s.currentProject?.id);

  const submit = useCallback(
    async (input: CreateTicketInput): Promise<SupportTicket> => {
      const res = await api<{ ticket: SupportTicket }>(
        "/v1/support/tickets",
        {
          method: "POST",
          body: JSON.stringify({
            projectId: input.projectId ?? projectId ?? null,
            subject: input.subject,
            category: input.category,
            priority: input.priority,
            description: input.description,
          }),
        },
      );
      return res.ticket;
    },
    [projectId],
  );

  return { submit };
}

type ThreadState = {
  messages: TicketMessage[];
  loading: boolean;
  error: string | null;
};

export function useTicketThread(ticketId: string | null) {
  const [state, setState] = useState<ThreadState>({
    messages: [],
    loading: false,
    error: null,
  });

  useEffect(() => {
    if (!ticketId) return;
    let ignore = false;
    (async () => {
      setState({ messages: [], loading: true, error: null });
      try {
        const res = await api<{ messages: TicketMessage[] }>(
          `/v1/support/tickets/${ticketId}/messages`,
        );
        if (ignore) return;
        setState({ messages: res.messages, loading: false, error: null });
      } catch (e) {
        if (ignore) return;
        setState({
          messages: [],
          loading: false,
          error: e instanceof Error ? e.message : "Failed to load messages",
        });
      }
    })();
    return () => {
      ignore = true;
    };
  }, [ticketId]);

  const refresh = useCallback(async () => {
    if (!ticketId) return;
    setState((prev) => ({ ...prev, loading: true }));
    try {
      const res = await api<{ messages: TicketMessage[] }>(
        `/v1/support/tickets/${ticketId}/messages`,
      );
      setState({ messages: res.messages, loading: false, error: null });
    } catch (e) {
      setState({
        messages: [],
        loading: false,
        error: e instanceof Error ? e.message : "Failed to load messages",
      });
    }
  }, [ticketId]);

  return {
    messages: state.messages,
    loading: state.loading,
    error: state.error,
    refresh,
  };
}

export function useSendReply() {
  const sendReply = useCallback(
    async (ticketId: string, body: string): Promise<TicketMessage> => {
      const res = await api<{ message: TicketMessage }>(
        `/v1/support/tickets/${ticketId}/messages`,
        {
          method: "POST",
          body: JSON.stringify({ body }),
        },
      );
      return res.message;
    },
    [],
  );

  return { sendReply };
}

"use client";

import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";
import type { InAppNotification } from "@/lib/types";

type FetchState = {
  notifications: InAppNotification[];
  hasMore: boolean;
  unreadCount: number;
  loading: boolean;
  error: string | null;
};

export function useInAppNotifications() {
  const [state, setState] = useState<FetchState>({
    notifications: [],
    hasMore: false,
    unreadCount: 0,
    loading: true,
    error: null,
  });

  const fetchNotifications = useCallback(async (unreadOnly = false) => {
    setState((s) => ({ ...s, loading: true, error: null }));
    try {
      const params = new URLSearchParams();
      if (unreadOnly) params.set("unreadOnly", "true");
      params.set("limit", "50");
      // api() targets the Rust backend (env.apiBase) — a raw fetch
      // would hit this Next.js server instead and 404.
      const data = await api<{ notifications: InAppNotification[]; hasMore: boolean }>(
        `/app/notifications?${params}`
      );
      setState({
        notifications: data.notifications ?? [],
        hasMore: data.hasMore ?? false,
        unreadCount: 0,
        loading: false,
        error: null,
      });
    } catch (e) {
      setState((s) => ({
        ...s,
        loading: false,
        error: e instanceof Error ? e.message : "Failed to load",
      }));
    }
  }, []);

  const fetchUnreadCount = useCallback(async () => {
    try {
      const data = await api<{ count: number }>("/app/notifications/count");
      setState((s) => ({ ...s, unreadCount: data.count ?? 0 }));
    } catch {
      // silent — badge just stays stale
    }
  }, []);

  useEffect(() => {
    fetchNotifications();
    fetchUnreadCount();
  }, [fetchNotifications, fetchUnreadCount]);

  const markRead = useCallback(
    async (id: string, read: boolean) => {
      setState((s) => ({
        ...s,
        notifications: s.notifications.map((n) =>
          n.id === id ? { ...n, read, readAt: read ? new Date().toISOString() : null } : n,
        ),
        unreadCount: read
          ? Math.max(0, s.unreadCount - 1)
          : s.unreadCount + 1,
      }));
      try {
        await api(`/app/notifications/${id}/read`, {
          method: "PATCH",
          body: JSON.stringify({ read }),
        });
      } catch {
        // revert on failure
        setState((s) => ({
          ...s,
          notifications: s.notifications.map((n) =>
            n.id === id ? { ...n, read: !read, readAt: !read ? new Date().toISOString() : null } : n,
          ),
          unreadCount: !read
            ? Math.max(0, s.unreadCount - 1)
            : s.unreadCount + 1,
        }));
      }
    },
    [],
  );

  const markAllRead = useCallback(async () => {
    setState((s) => ({
      ...s,
      notifications: s.notifications.map((n) => ({ ...n, read: true, readAt: new Date().toISOString() })),
      unreadCount: 0,
    }));
    try {
      await api("/app/notifications/read-all", { method: "PATCH" });
    } catch {
      // silent — optimistic update already applied
    }
  }, []);

  const deleteNotification = useCallback(
    async (id: string) => {
      const prev = state.notifications;
      setState((s) => ({
        ...s,
        notifications: s.notifications.filter((n) => n.id !== id),
      }));
      try {
        await api(`/app/notifications/${id}`, { method: "DELETE" });
      } catch {
        setState((s) => ({ ...s, notifications: prev }));
      }
    },
    [state.notifications],
  );

  return {
    ...state,
    markRead,
    markAllRead,
    deleteNotification,
    refresh: fetchNotifications,
  };
}

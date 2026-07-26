import { useMemo } from "react";
import type { Notification, NotificationChannel, NotificationStatus, NotificationPriority } from "@/lib/types";

const CHANNELS: NotificationChannel[] = ["email", "sms", "push-android", "push-ios", "web-push", "webhook"];
const STATUSES: NotificationStatus[] = ["queued", "processing", "sent", "delivered", "opened", "clicked", "failed", "cancelled", "retrying"];
const PRIORITIES: NotificationPriority[] = ["low", "normal", "high", "urgent"];

function generateNotifications(count: number): Notification[] {
  return Array.from({ length: count }, (_, i) => {
    const status = STATUSES[Math.floor(Math.random() * STATUSES.length)];
    const channel = CHANNELS[Math.floor(Math.random() * CHANNELS.length)];
    const priority = PRIORITIES[Math.floor(Math.random() * PRIORITIES.length)];
    const createdAt = new Date();
    createdAt.setMinutes(createdAt.getMinutes() - Math.floor(Math.random() * 1440));

    return {
      id: `ntf_${String(i + 1).padStart(4, "0")}`,
      projectId: "proj_1",
      templateId: Math.random() > 0.3 ? `tpl_${Math.floor(Math.random() * 10) + 1}` : undefined,
      recipientId: `rcp_${String(Math.floor(Math.random() * 500) + 1).padStart(4, "0")}`,
      channel,
      status,
      priority,
      subject: channel === "email" ? `Notification ${i + 1}` : undefined,
      body: `This is notification body ${i + 1}. It contains important information for the recipient.`,
      retryCount: status === "retrying" ? Math.floor(Math.random() * 3) + 1 : 0,
      providerId: Math.random() > 0.2 ? `prv_${Math.floor(Math.random() * 5) + 1}` : undefined,
      sentAt: ["sent", "delivered", "opened", "clicked", "failed"].includes(status) ? createdAt.toISOString() : undefined,
      deliveredAt: ["delivered", "opened", "clicked"].includes(status) ? createdAt.toISOString() : undefined,
      failedAt: status === "failed" ? createdAt.toISOString() : undefined,
      failureReason: status === "failed" ? "Provider rate limit exceeded" : undefined,
      createdAt: createdAt.toISOString(),
    };
  });
}

const ALL_NOTIFICATIONS = generateNotifications(200);

export type NotificationFilters = {
  search?: string;
  status?: NotificationStatus[];
  channel?: NotificationChannel[];
  priority?: NotificationPriority[];
  dateFrom?: string;
  dateTo?: string;
};

export type NotificationSort = {
  field: keyof Notification;
  direction: "asc" | "desc";
};

export function useNotifications(
  filters?: NotificationFilters,
  sort?: NotificationSort,
  page: number = 1,
  pageSize: number = 20,
) {
  return useMemo(() => {
    let filtered = [...ALL_NOTIFICATIONS];

    if (filters?.search) {
      const q = filters.search.toLowerCase();
      filtered = filtered.filter(
        (n) =>
          n.id.toLowerCase().includes(q) ||
          n.body.toLowerCase().includes(q) ||
          n.subject?.toLowerCase().includes(q),
      );
    }

    if (filters?.status?.length) {
      filtered = filtered.filter((n) => filters.status!.includes(n.status));
    }

    if (filters?.channel?.length) {
      filtered = filtered.filter((n) => filters.channel!.includes(n.channel));
    }

    if (filters?.priority?.length) {
      filtered = filtered.filter((n) => filters.priority!.includes(n.priority));
    }

    if (sort) {
      filtered.sort((a, b) => {
        const aVal = a[sort.field];
        const bVal = b[sort.field];
        const cmp = String(aVal ?? "").localeCompare(String(bVal ?? ""));
        return sort.direction === "asc" ? cmp : -cmp;
      });
    } else {
      filtered.sort((a, b) => b.createdAt.localeCompare(a.createdAt));
    }

    const total = filtered.length;
    const start = (page - 1) * pageSize;
    const items = filtered.slice(start, start + pageSize);

    return { items, total, page, pageSize, totalPages: Math.ceil(total / pageSize) };
  }, [filters, sort, page, pageSize]);
}

export function useNotification(id: string) {
  return useMemo(() => ALL_NOTIFICATIONS.find((n) => n.id === id) ?? null, [id]);
}

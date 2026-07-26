import { useMemo } from "react";
import type { Recipient, Device } from "@/lib/types";

function generateDevices(recipientId: string, count: number): Device[] {
  const platforms: Device["platform"][] = ["android", "ios", "ipados", "macos", "linux", "windows", "browser"];
  const providers: Record<Device["platform"], string> = {
    android: "FCM",
    ios: "APNs",
    ipados: "APNs",
    macos: "APNs",
    linux: "Web Push",
    windows: "WNS",
    browser: "Web Push",
  };

  return Array.from({ length: count }, (_, i) => {
    const platform = platforms[i % platforms.length];
    const status: Device["status"] = i === 0 ? "active" : Math.random() > 0.3 ? "active" : Math.random() > 0.5 ? "inactive" : "expired";
    const createdAt = new Date();
    createdAt.setDate(createdAt.getDate() - Math.floor(Math.random() * 180));

    return {
      id: `dev_${recipientId.slice(4)}_${String(i + 1).padStart(2, "0")}`,
      recipientId,
      platform,
      token: `tok_${Math.random().toString(36).slice(2, 14)}`,
      provider: providers[platform],
      appVersion: `${Math.floor(Math.random() * 3) + 1}.${Math.floor(Math.random() * 10)}.${Math.floor(Math.random() * 20)}`,
      platformVersion: `${Math.floor(Math.random() * 5) + 14}.0`,
      status,
      lastActiveAt: status === "active" ? new Date().toISOString() : undefined,
      expiresAt: status === "expired" ? new Date(Date.now() - 86400000).toISOString() : undefined,
    };
  });
}

function generateRecipients(count: number): Recipient[] {
  const firstNames = ["James", "Mary", "John", "Patricia", "Robert", "Jennifer", "Michael", "Linda", "David", "Elizabeth", "William", "Barbara", "Richard", "Susan", "Joseph"];
  const lastNames = ["Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis", "Rodriguez", "Martinez", "Hernandez", "Lopez", "Gonzalez", "Wilson", "Anderson"];
  const domains = ["gmail.com", "yahoo.com", "outlook.com", "hey.com", "fastmail.com"];
  const tags = ["vip", "trial", "premium", "enterprise", "active", "inactive", "churned"];
  const segments = ["onboarding", "engaged", "dormant", "power-user", "at-risk"];

  return Array.from({ length: count }, (_, i) => {
    const first = firstNames[i % firstNames.length];
    const last = lastNames[i % lastNames.length];
    const createdAt = new Date();
    createdAt.setDate(createdAt.getDate() - Math.floor(Math.random() * 365));

    const deviceCount = Math.floor(Math.random() * 3) + 1;

    return {
      id: `rcp_${String(i + 1).padStart(4, "0")}`,
      projectId: "proj_1",
      email: `${first.toLowerCase()}.${last.toLowerCase()}${i}@${domains[i % domains.length]}`,
      phone: Math.random() > 0.3 ? `+1${String(Math.floor(Math.random() * 9000000000) + 1000000000)}` : undefined,
      name: `${first} ${last}`,
      attributes: { company: `Company ${i % 50}`, plan: ["free", "pro", "enterprise"][i % 3] },
      tags: tags.filter(() => Math.random() > 0.6),
      segments: segments.filter(() => Math.random() > 0.7),
      language: ["en", "es", "fr", "de", "ja"][i % 5],
      timezone: ["America/New_York", "America/Chicago", "Europe/London", "Europe/Berlin", "Asia/Tokyo"][i % 5],
      createdAt: createdAt.toISOString(),
      lastActiveAt: Math.random() > 0.2 ? new Date().toISOString() : undefined,
      devices: generateDevices(`rcp_${String(i + 1).padStart(4, "0")}`, deviceCount),
    };
  });
}

const ALL_RECIPIENTS = generateRecipients(500);

export type RecipientFilters = {
  search?: string;
  tags?: string[];
  segments?: string[];
};

export function useRecipients(
  filters?: RecipientFilters,
  page: number = 1,
  pageSize: number = 20,
) {
  return useMemo(() => {
    let filtered = [...ALL_RECIPIENTS];

    if (filters?.search) {
      const q = filters.search.toLowerCase();
      filtered = filtered.filter(
        (r) =>
          r.name.toLowerCase().includes(q) ||
          r.email?.toLowerCase().includes(q) ||
          r.id.toLowerCase().includes(q),
      );
    }

    if (filters?.tags?.length) {
      filtered = filtered.filter((r) => filters.tags!.some((t) => r.tags?.includes(t)));
    }

    if (filters?.segments?.length) {
      filtered = filtered.filter((r) => filters.segments!.some((s) => r.segments?.includes(s)));
    }

    filtered.sort((a, b) => b.createdAt.localeCompare(a.createdAt));

    const total = filtered.length;
    const start = (page - 1) * pageSize;
    const items = filtered.slice(start, start + pageSize);

    return { items, total, page, pageSize, totalPages: Math.ceil(total / pageSize) };
  }, [filters, page, pageSize]);
}

export function useRecipient(id: string) {
  return useMemo(() => ALL_RECIPIENTS.find((r) => r.id === id) ?? null, [id]);
}

export function useRecipientNotifications(recipientId: string) {
  return useMemo(() => {
    // Generate deterministic notifications for this recipient
    const seed = recipientId.split("").reduce((acc, c) => acc + c.charCodeAt(0), 0);
    const count = 5 + (seed % 10);

    return Array.from({ length: count }, (_, i) => {
      const status = (["delivered", "opened", "clicked", "failed"] as const)[i % 4];
      const channel = (["email", "sms", "push-ios", "web-push"] as const)[i % 4];
      const createdAt = new Date();
      createdAt.setMinutes(createdAt.getMinutes() - i * 30 - ((seed + i) % 30));

      return {
        id: `ntf_${recipientId.slice(4)}_${String(i + 1).padStart(2, "0")}`,
        channel,
        status,
        subject: channel === "email" ? `Update ${i + 1}` : undefined,
        body: `Notification for recipient ${recipientId}, message ${i + 1}.`,
        createdAt: createdAt.toISOString(),
        sentAt: ["delivered", "opened", "clicked", "failed"].includes(status) ? createdAt.toISOString() : undefined,
      };
    });
  }, [recipientId]);
}

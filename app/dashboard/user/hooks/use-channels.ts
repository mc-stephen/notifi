import { useMemo } from "react";
import type { Channel, MessageProvider, NotificationChannel } from "@/lib/types";

const PROVIDERS: MessageProvider[] = [
  { id: "prv_1", projectId: "proj_1", name: "SendGrid", type: "email", connected: true, health: "healthy", latencyMs: 120, successRate: 99.2, region: "us-east-1", config: { from: "noreply@example.com" }, quotaUsed: 45230, quotaLimit: 100000, createdAt: "2025-01-15T00:00:00Z" },
  { id: "prv_2", projectId: "proj_1", name: "Amazon SES", type: "email", connected: true, health: "healthy", latencyMs: 95, successRate: 98.8, region: "us-west-2", config: { from: "noreply@example.com" }, quotaUsed: 12300, quotaLimit: 50000, createdAt: "2025-02-01T00:00:00Z" },
  { id: "prv_3", projectId: "proj_1", name: "Resend", type: "email", connected: false, health: "unknown", latencyMs: 0, successRate: 0, region: "us-east-1", config: {}, quotaUsed: 0, quotaLimit: 0, createdAt: "2025-03-01T00:00:00Z" },
  { id: "prv_4", projectId: "proj_1", name: "Postmark", type: "email", connected: false, health: "unknown", latencyMs: 0, successRate: 0, region: "us-east-1", config: {}, quotaUsed: 0, quotaLimit: 0, createdAt: "2025-03-15T00:00:00Z" },
  { id: "prv_5", projectId: "proj_1", name: "Mailgun", type: "email", connected: false, health: "unknown", latencyMs: 0, successRate: 0, region: "us-west-1", config: {}, quotaUsed: 0, quotaLimit: 0, createdAt: "2025-04-01T00:00:00Z" },
  { id: "prv_6", projectId: "proj_1", name: "Twilio", type: "sms", connected: true, health: "healthy", latencyMs: 340, successRate: 97.5, region: "us-east-1", config: {}, quotaUsed: 8900, quotaLimit: 25000, createdAt: "2025-01-20T00:00:00Z" },
  { id: "prv_7", projectId: "proj_1", name: "Vonage", type: "sms", connected: false, health: "unknown", latencyMs: 0, successRate: 0, region: "us-east-1", config: {}, quotaUsed: 0, quotaLimit: 0, createdAt: "2025-02-15T00:00:00Z" },
  { id: "prv_8", projectId: "proj_1", name: "Firebase Cloud Messaging", type: "push-android", connected: true, health: "healthy", latencyMs: 45, successRate: 99.8, region: "global", config: {}, quotaUsed: 120000, quotaLimit: 500000, createdAt: "2025-01-10T00:00:00Z" },
  { id: "prv_9", projectId: "proj_1", name: "Apple Push Notification Service", type: "push-ios", connected: true, health: "degraded", latencyMs: 180, successRate: 96.2, region: "global", config: {}, quotaUsed: 45000, quotaLimit: 200000, createdAt: "2025-01-10T00:00:00Z" },
  { id: "prv_10", projectId: "proj_1", name: "Web Push (VAPID)", type: "web-push", connected: true, health: "healthy", latencyMs: 60, successRate: 99.1, region: "global", config: {}, quotaUsed: 32000, quotaLimit: 100000, createdAt: "2025-03-01T00:00:00Z" },
  { id: "prv_11", projectId: "proj_1", name: "Slack", type: "slack", connected: true, health: "healthy", latencyMs: 180, successRate: 99.5, region: "global", config: {}, quotaUsed: 1200, quotaLimit: 10000, createdAt: "2025-02-01T00:00:00Z" },
  { id: "prv_12", projectId: "proj_1", name: "Discord", type: "discord", connected: false, health: "unknown", latencyMs: 0, successRate: 0, region: "global", config: {}, quotaUsed: 0, quotaLimit: 0, createdAt: "2025-04-01T00:00:00Z" },
  { id: "prv_13", projectId: "proj_1", name: "Telegram Bot API", type: "telegram", connected: false, health: "unknown", latencyMs: 0, successRate: 0, region: "global", config: {}, quotaUsed: 0, quotaLimit: 0, createdAt: "2025-04-15T00:00:00Z" },
  { id: "prv_14", projectId: "proj_1", name: "Custom Webhook", type: "webhook", connected: true, health: "healthy", latencyMs: 210, successRate: 98.5, region: "us-east-1", config: { url: "https://api.example.com/webhook" }, quotaUsed: 5600, quotaLimit: 50000, createdAt: "2025-04-01T00:00:00Z" },
];

const CHANNELS: Channel[] = [
  { id: "ch_1", projectId: "proj_1", type: "email", enabled: true, providerIds: ["prv_1", "prv_2"] },
  { id: "ch_2", projectId: "proj_1", type: "sms", enabled: true, providerIds: ["prv_6"] },
  { id: "ch_3", projectId: "proj_1", type: "push-android", enabled: true, providerIds: ["prv_8"] },
  { id: "ch_4", projectId: "proj_1", type: "push-ios", enabled: true, providerIds: ["prv_9"] },
  { id: "ch_5", projectId: "proj_1", type: "web-push", enabled: true, providerIds: ["prv_10"] },
  { id: "ch_6", projectId: "proj_1", type: "slack", enabled: true, providerIds: ["prv_11"] },
  { id: "ch_7", projectId: "proj_1", type: "discord", enabled: false, providerIds: [] },
  { id: "ch_8", projectId: "proj_1", type: "webhook", enabled: true, providerIds: ["prv_14"] },
];

export function useChannels() {
  return useMemo(() => CHANNELS, []);
}

export function useProviders() {
  return useMemo(() => PROVIDERS, []);
}

export function useProvidersByType(type: NotificationChannel) {
  return useMemo(() => PROVIDERS.filter((p) => p.type === type), [type]);
}

export function useChannelProviders(channelId: string) {
  return useMemo(() => {
    const channel = CHANNELS.find((c) => c.id === channelId);
    if (!channel) return [];
    return channel.providerIds
      .map((id) => PROVIDERS.find((p) => p.id === id))
      .filter(Boolean) as MessageProvider[];
  }, [channelId]);
}

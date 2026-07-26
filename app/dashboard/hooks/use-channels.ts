import { useMemo } from "react";
import type { ChannelConfig, Provider } from "@/lib/types";

const CHANNELS: ChannelConfig[] = [
  { id: "ch_1", projectId: "proj_1", type: "email", enabled: true, config: { provider: "sendgrid", from: "noreply@example.com" } },
  { id: "ch_2", projectId: "proj_1", type: "sms", enabled: true, config: { provider: "twilio", from: "+15551234567" } },
  { id: "ch_3", projectId: "proj_1", type: "push-android", enabled: true, config: { provider: "fcm", projectId: "my-app" } },
  { id: "ch_4", projectId: "proj_1", type: "push-ios", enabled: true, config: { provider: "apns", bundleId: "com.example.app" } },
  { id: "ch_5", projectId: "proj_1", type: "web-push", enabled: true, config: { provider: "web-push", vapidPublicKey: "BK..." } },
  { id: "ch_6", projectId: "proj_1", type: "slack", enabled: false, config: { webhookUrl: "https://hooks.slack.com/..." } },
  { id: "ch_7", projectId: "proj_1", type: "discord", enabled: false, config: { botToken: "..." } },
  { id: "ch_8", projectId: "proj_1", type: "webhook", enabled: true, config: { url: "https://api.example.com/webhook", method: "POST" } },
];

const PROVIDERS: Provider[] = [
  { id: "prv_1", channelId: "ch_1", type: "email", name: "SendGrid", enabled: true, health: "healthy", latencyMs: 120, successRate: 99.2, priority: 1, fallbackId: "prv_2", region: "us-east-1", quotaUsed: 45230, quotaLimit: 100000, createdAt: "2025-01-15T00:00:00Z" },
  { id: "prv_2", channelId: "ch_1", type: "email", name: "Amazon SES", enabled: true, health: "healthy", latencyMs: 95, successRate: 98.8, priority: 2, region: "us-west-2", quotaUsed: 12300, quotaLimit: 50000, createdAt: "2025-02-01T00:00:00Z" },
  { id: "prv_3", channelId: "ch_2", type: "sms", name: "Twilio", enabled: true, health: "healthy", latencyMs: 340, successRate: 97.5, priority: 1, region: "us-east-1", quotaUsed: 8900, quotaLimit: 25000, createdAt: "2025-01-20T00:00:00Z" },
  { id: "prv_4", channelId: "ch_3", type: "push-android", name: "Firebase Cloud Messaging", enabled: true, health: "healthy", latencyMs: 45, successRate: 99.8, priority: 1, region: "global", quotaUsed: 120000, quotaLimit: 500000, createdAt: "2025-01-10T00:00:00Z" },
  { id: "prv_5", channelId: "ch_4", type: "push-ios", name: "Apple Push Notification Service", enabled: true, health: "degraded", latencyMs: 180, successRate: 96.2, priority: 1, region: "global", quotaUsed: 45000, quotaLimit: 200000, createdAt: "2025-01-10T00:00:00Z" },
  { id: "prv_6", channelId: "ch_5", type: "web-push", name: "Web Push (VAPID)", enabled: true, health: "healthy", latencyMs: 60, successRate: 99.1, priority: 1, region: "global", quotaUsed: 32000, quotaLimit: 100000, createdAt: "2025-03-01T00:00:00Z" },
  { id: "prv_7", channelId: "ch_8", type: "webhook", name: "Custom Webhook", enabled: true, health: "healthy", latencyMs: 210, successRate: 98.5, priority: 1, region: "us-east-1", quotaUsed: 5600, quotaLimit: 50000, createdAt: "2025-04-01T00:00:00Z" },
];

export function useChannels() {
  return useMemo(() => CHANNELS, []);
}

export function useProviders() {
  return useMemo(() => PROVIDERS, []);
}

export function useProvidersByChannel(channelId: string) {
  return useMemo(() => PROVIDERS.filter((p) => p.channelId === channelId), [channelId]);
}

export function useProviderHealth() {
  return useMemo(() => {
    return PROVIDERS.map((p) => ({
      id: p.id,
      name: p.name,
      type: p.type,
      health: p.health,
      latencyMs: p.latencyMs,
      successRate: p.successRate,
      quotaUsed: p.quotaUsed,
      quotaLimit: p.quotaLimit,
    }));
  }, []);
}

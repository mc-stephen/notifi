import type { Tenant } from "./types";

export const MOCK_TENANTS: Tenant[] = [
  { id: "org_1", name: "Acme Corp", slug: "acme" },
  { id: "org_2", name: "Globex Inc", slug: "globex" },
  { id: "org_3", name: "Initech", slug: "initech" },
];

export const NAV_ITEMS = [
  { label: "Overview", href: "/overview", icon: "LayoutDashboard" },
  { label: "Logs & Activity", href: "/logs", icon: "ScrollText" },
  { label: "Workflows", href: "/workflows", icon: "Workflow" },
  { label: "Subscribers", href: "/subscribers", icon: "Users" },
  { label: "Channels", href: "/channels", icon: "PlugZap" },
  { label: "API Keys", href: "/settings/api-keys", icon: "Key" },
  { label: "Settings", href: "/settings", icon: "Settings" },
] as const;

export const CHANNEL_LABELS: Record<string, string> = {
  email: "Email",
  fcm: "FCM",
  apns: "APNs",
  sms: "SMS",
  webpush: "WebPush",
};

export const STATUS_LABELS: Record<string, string> = {
  queued: "Queued",
  sent: "Sent",
  delivered: "Delivered",
  failed: "Failed",
};

import { useMemo } from "react";
import type { Template } from "@/lib/types";

const MOCK_TEMPLATES: Template[] = [
  {
    id: "tpl_1",
    projectId: "proj_1",
    name: "Welcome Email",
    folder: "Onboarding",
    category: "Transactional",
    channel: "email",
    subject: "Welcome to {{app_name}}, {{first_name}}!",
    body: "<h1>Welcome, {{first_name}}!</h1><p>We're excited to have you on board.</p>",
    variables: [
      { name: "app_name", type: "string", required: true },
      { name: "first_name", type: "string", required: true },
    ],
    version: 3,
    isDraft: false,
    locale: "en",
    createdAt: "2025-01-20T00:00:00Z",
    updatedAt: "2025-06-15T00:00:00Z",
  },
  {
    id: "tpl_2",
    projectId: "proj_1",
    name: "Password Reset",
    folder: "Security",
    category: "Transactional",
    channel: "email",
    subject: "Reset your password",
    body: "<p>Click the link to reset your password: {{reset_link}}</p>",
    variables: [
      { name: "reset_link", type: "string", required: true },
      { name: "expires_in", type: "string", required: false, defaultValue: "30 minutes" },
    ],
    version: 1,
    isDraft: false,
    locale: "en",
    createdAt: "2025-02-01T00:00:00Z",
    updatedAt: "2025-02-01T00:00:00Z",
  },
  {
    id: "tpl_3",
    projectId: "proj_1",
    name: "Order Confirmation",
    folder: "Commerce",
    category: "Transactional",
    channel: "email",
    subject: "Order #{{order_id}} confirmed",
    body: "<p>Your order has been confirmed. Total: {{total}}</p>",
    variables: [
      { name: "order_id", type: "string", required: true },
      { name: "total", type: "string", required: true },
      { name: "items", type: "json", required: true },
    ],
    version: 2,
    isDraft: false,
    locale: "en",
    createdAt: "2025-03-10T00:00:00Z",
    updatedAt: "2025-05-20T00:00:00Z",
  },
  {
    id: "tpl_4",
    projectId: "proj_1",
    name: "Shipping Update",
    folder: "Commerce",
    category: "Transactional",
    channel: "sms",
    body: "Your order {{order_id}} has shipped! Track: {{tracking_url}}",
    variables: [
      { name: "order_id", type: "string", required: true },
      { name: "tracking_url", type: "string", required: true },
    ],
    version: 1,
    isDraft: false,
    locale: "en",
    createdAt: "2025-03-15T00:00:00Z",
    updatedAt: "2025-03-15T00:00:00Z",
  },
  {
    id: "tpl_5",
    projectId: "proj_1",
    name: "Push: New Message",
    folder: "Messaging",
    category: "Engagement",
    channel: "push-android",
    body: "{{sender_name}}: {{preview}}",
    variables: [
      { name: "sender_name", type: "string", required: true },
      { name: "preview", type: "string", required: true },
    ],
    version: 1,
    isDraft: false,
    createdAt: "2025-04-01T00:00:00Z",
    updatedAt: "2025-04-01T00:00:00Z",
  },
  {
    id: "tpl_6",
    projectId: "proj_1",
    name: "Weekly Digest",
    folder: "Marketing",
    category: "Marketing",
    channel: "email",
    subject: "Your weekly summary",
    body: "<h1>Weekly Digest</h1><p>Here's what happened this week.</p>",
    variables: [
      { name: "week_summary", type: "string", required: true },
      { name: "highlights", type: "json", required: true },
    ],
    version: 5,
    isDraft: true,
    locale: "en",
    createdAt: "2025-02-15T00:00:00Z",
    updatedAt: "2025-06-20T00:00:00Z",
  },
  {
    id: "tpl_7",
    projectId: "proj_1",
    name: "Webhook: Event Alert",
    folder: "System",
    category: "System",
    channel: "webhook",
    body: '{"event": "{{event_type}}", "message": "{{message}}", "severity": "{{severity}}"}',
    variables: [
      { name: "event_type", type: "string", required: true },
      { name: "message", type: "string", required: true },
      { name: "severity", type: "string", required: true, defaultValue: "info" },
    ],
    version: 2,
    isDraft: false,
    createdAt: "2025-05-01T00:00:00Z",
    updatedAt: "2025-06-10T00:00:00Z",
  },
];

export type TemplateFilters = {
  search?: string;
  folder?: string;
  channel?: string;
  isDraft?: boolean;
};

export function useTemplates(filters?: TemplateFilters) {
  return useMemo(() => {
    let filtered = [...MOCK_TEMPLATES];

    if (filters?.search) {
      const q = filters.search.toLowerCase();
      filtered = filtered.filter(
        (t) =>
          t.name.toLowerCase().includes(q) ||
          t.subject?.toLowerCase().includes(q),
      );
    }

    if (filters?.folder) {
      filtered = filtered.filter((t) => t.folder === filters.folder);
    }

    if (filters?.channel) {
      filtered = filtered.filter((t) => t.channel === filters.channel);
    }

    if (filters?.isDraft !== undefined) {
      filtered = filtered.filter((t) => t.isDraft === filters.isDraft);
    }

    return filtered;
  }, [filters]);
}

export function useTemplate(id: string) {
  return useMemo(() => MOCK_TEMPLATES.find((t) => t.id === id) ?? null, [id]);
}

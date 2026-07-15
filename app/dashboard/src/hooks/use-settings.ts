"use client";

import { useState } from "react";
import type { ApiKey, Webhook } from "@/lib/types";

const mockKeys: ApiKey[] = [
  {
    id: "key_1",
    prefix: "live_",
    maskedKey: "live_xxxxxxxxxxxxx",
    createdAt: new Date(Date.now() - 86400000 * 30).toISOString(),
    environment: "production",
    active: true,
  },
  {
    id: "key_2",
    prefix: "test_",
    maskedKey: "test_xxxxxxxxxxxxx",
    createdAt: new Date(Date.now() - 86400000 * 15).toISOString(),
    environment: "development",
    active: true,
  },
];

const mockWebhooks: Webhook[] = [
  {
    id: "wh_1",
    url: "https://api.acme.com/notifications/hook",
    events: ["message.delivered", "message.failed"],
    active: true,
    createdAt: new Date(Date.now() - 86400000 * 7).toISOString(),
  },
];

const availableEvents = [
  "message.queued",
  "message.sent",
  "message.delivered",
  "message.failed",
  "message.opened",
  "message.clicked",
] as const;

export function useSettings() {
  const [keys, setKeys] = useState<ApiKey[]>(mockKeys);
  const [webhooks, setWebhooks] = useState<Webhook[]>(mockWebhooks);
  const [showKeys, setShowKeys] = useState<Record<string, boolean>>({});

  const toggleKeyVisibility = (id: string) => {
    setShowKeys((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  const rollKey = (id: string) => {
    setKeys((prev) =>
      prev.map((k) =>
        k.id === id
          ? {
              ...k,
              maskedKey: `${k.prefix}${Math.random().toString(36).substring(2)}`,
              createdAt: new Date().toISOString(),
            }
          : k,
      ),
    );
  };

  const addWebhook = (url: string, events: string[]) => {
    const wh: Webhook = {
      id: `wh_${Math.random().toString(36).substring(2, 10)}`,
      url,
      events,
      active: true,
      createdAt: new Date().toISOString(),
    };
    setWebhooks((prev) => [...prev, wh]);
  };

  const removeWebhook = (id: string) => {
    setWebhooks((prev) => prev.filter((w) => w.id !== id));
  };

  return {
    keys,
    webhooks,
    showKeys,
    availableEvents,
    toggleKeyVisibility,
    rollKey,
    addWebhook,
    removeWebhook,
  };
}

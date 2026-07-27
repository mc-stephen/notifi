import { useMemo } from "react";
import type { Event } from "@/lib/types";
import { createRng } from "@/lib/random";

const EVENT_TYPES: Event["type"][] = [
  "queued",
  "worker_assigned",
  "provider_selected",
  "sent",
  "delivered",
  "opened",
  "clicked",
  "failed",
  "retried",
  "cancelled",
];

const PROVIDERS = ["SendGrid", "Twilio", "Firebase", "SNS", "Postmark", "Resend"];
const REFERENCE_DATE = new Date("2026-07-27T12:00:00.000Z");

function generateEvents(count: number): Event[] {
  const rng = createRng(123);
  return Array.from({ length: count }, (_, i) => {
    const timestamp = new Date(REFERENCE_DATE);
    timestamp.setMinutes(timestamp.getMinutes() - i * 2 - Math.floor(rng() * 5));

    return {
      id: `evt_${String(i + 1).padStart(5, "0")}`,
      notificationId: `ntf_${String(Math.floor(rng() * 100) + 1).padStart(4, "0")}`,
      type: EVENT_TYPES[Math.floor(rng() * EVENT_TYPES.length)],
      timestamp: timestamp.toISOString(),
      provider: PROVIDERS[Math.floor(rng() * PROVIDERS.length)],
      requestId: `req_${rng().toString(36).slice(2, 10)}`,
      correlationId: `cor_${rng().toString(36).slice(2, 10)}`,
      metadata: { source: "api", region: "us-east-1" },
    };
  });
}

const ALL_EVENTS = generateEvents(100);

export function useEvents(page: number = 1, pageSize: number = 20) {
  return useMemo(() => {
    const sorted = [...ALL_EVENTS].sort((a, b) => b.timestamp.localeCompare(a.timestamp));
    const total = sorted.length;
    const start = (page - 1) * pageSize;
    const items = sorted.slice(start, start + pageSize);
    return { items, total, page, pageSize, totalPages: Math.ceil(total / pageSize) };
  }, [page, pageSize]);
}

"use client";

import { useState, useEffect } from "react";
import type { LogEntry } from "@/lib/types";

const channels: LogEntry["channel"][] = ["email", "fcm", "apns", "sms", "webpush"];
const statuses: LogEntry["status"][] = ["queued", "sent", "delivered", "failed"];

function generateLogs(count: number): LogEntry[] {
  return Array.from({ length: count }, (_, i) => {
    const status = statuses[Math.floor(Math.random() * (i < 5 ? 1 : statuses.length))];
    const channel = channels[Math.floor(Math.random() * channels.length)];
    const now = Date.now();
    const steps = [
      { label: "Received", offsetMs: 0, status: "success" as const },
      { label: "Throttling Checked", offsetMs: 2, status: "success" as const },
      { label: "Pushed to Redis", offsetMs: 5, status: "success" as const },
      {
        label: "Delivered to Provider",
        offsetMs: 142 + Math.floor(Math.random() * 200),
        status: status === "failed" ? ("error" as const) : ("success" as const),
      },
    ];

    return {
      id: `log_${i + 1}`,
      trackingId: `ntf_${Math.random().toString(36).substring(2, 10)}`,
      subscriberId: `sub_${Math.random().toString(36).substring(2, 8)}`,
      channel,
      status,
      timestamp: new Date(now - i * 120000 + Math.random() * 60000).toISOString(),
      metadata: {
        notification: { title: "Welcome notification", body: "Hello from Notifi!" },
        subscriber: { email: "user@example.com", timezone: "UTC" },
        provider: { name: channel === "email" ? "SendGrid" : "FCM", latency_ms: 145 },
      },
      executionSteps: steps,
      error: status === "failed" ? "Provider returned 429: Too Many Requests" : undefined,
    };
  });
}

const allLogs = generateLogs(150);

export function useLogs() {
  const [loading, setLoading] = useState(true);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [page, setPage] = useState(0);
  const [selectedLog, setSelectedLog] = useState<LogEntry | null>(null);
  const pageSize = 25;
  const totalPages = Math.ceil(allLogs.length / pageSize);

  useEffect(() => {
    setLoading(true);
    const timer = setTimeout(() => {
      const start = page * pageSize;
      setLogs(allLogs.slice(start, start + pageSize));
      setLoading(false);
    }, 400);
    return () => clearTimeout(timer);
  }, [page]);

  return {
    logs,
    loading,
    page,
    totalPages,
    setPage,
    selectedLog,
    setSelectedLog,
    pageSize,
  };
}

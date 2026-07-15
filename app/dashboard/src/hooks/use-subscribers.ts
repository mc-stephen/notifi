"use client";

import { useState, useEffect, useMemo } from "react";
import type { Subscriber, DeviceToken, Channel } from "@/lib/types";

const channels: Channel[] = ["email", "fcm", "apns", "sms", "webpush"];

function generateTokens(): DeviceToken[] {
  const count = Math.floor(Math.random() * 4) + 1;
  return Array.from({ length: count }, (_, i) => ({
    id: `tok_${Math.random().toString(36).substring(2, 10)}`,
    channel: channels[i % channels.length],
    token: `${channels[i % channels.length]}_${Math.random().toString(36).substring(2, 18)}`,
    lastUsed: new Date(Date.now() - Math.random() * 86400000 * 7).toISOString(),
    active: Math.random() > 0.2,
  }));
}

const allSubscribers: Subscriber[] = Array.from({ length: 50 }, (_, i) => ({
  id: `sub_${Math.random().toString(36).substring(2, 10)}`,
  email: `user${i + 1}@example.com`,
  phone: i % 3 === 0 ? `+1555${String(1000 + i).padStart(4, "0")}` : undefined,
  name: `User ${i + 1}`,
  createdAt: new Date(Date.now() - Math.random() * 86400000 * 90).toISOString(),
  tokens: generateTokens(),
}));

export function useSubscribers() {
  const [loading, setLoading] = useState(true);
  const [subscribers, setSubscribers] = useState<Subscriber[]>([]);
  const [search, setSearch] = useState("");

  useEffect(() => {
    setLoading(true);
    const timer = setTimeout(() => {
      setSubscribers(allSubscribers);
      setLoading(false);
    }, 500);
    return () => clearTimeout(timer);
  }, []);

  const filtered = useMemo(() => {
    if (!search) return subscribers;
    const q = search.toLowerCase();
    return subscribers.filter(
      (s) =>
        s.id.toLowerCase().includes(q) ||
        s.email?.toLowerCase().includes(q) ||
        s.phone?.includes(q) ||
        s.name?.toLowerCase().includes(q),
    );
  }, [search, subscribers]);

  const getSubscriberById = (id: string) =>
    subscribers.find((s) => s.id === id) || null;

  return {
    subscribers: filtered,
    loading,
    search,
    setSearch,
    getSubscriberById,
  };
}

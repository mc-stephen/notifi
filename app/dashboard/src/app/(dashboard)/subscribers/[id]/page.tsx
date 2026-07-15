"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { useSubscribers } from "@/hooks/use-subscribers";
import { SubscriberProfile } from "@/components/dashboard/subscribers/subscriber-profile";
import type { Subscriber } from "@/lib/types";

export default function SubscriberProfilePage() {
  const params = useParams();
  const { getSubscriberById } = useSubscribers();
  const [subscriber, setSubscriber] = useState<Subscriber | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    const timer = setTimeout(() => {
      setSubscriber(getSubscriberById(params.id as string));
      setLoading(false);
    }, 400);
    return () => clearTimeout(timer);
  }, [params.id, getSubscriberById]);

  return <SubscriberProfile subscriber={subscriber} loading={loading} />;
}
